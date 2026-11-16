// SPDX-License-Identifier: LGPL-3.0-or-later
//! H0–H7 comparison campaign on the steep-edge filter.
//!
//! A chirped H/L quarter-wave stack gives a long-pass edge; the target rewards
//! low reflectance below 500 nm and high reflectance above 600 nm. Pass
//! `--fixture shortpass` with the same stack to reverse the bands and the
//! chirp. This is the starting point for the Recipe B filter run, not a
//! claimed optimum yet.

use std::collections::BTreeMap;

use navette::smatrix::angle::Radians;
use navette::smatrix::synthesis::design_config::{MaterialDef, TabData};
use navette::smatrix::synthesis::merit::{
    ConstraintKind, CurveId, MeritKey, MeritSpec, MeritTarget, SimTransform,
};
use navette::smatrix::synthesis::pipeline::SpectralInputs;
use navette::smatrix::synthesis::structure::{DesignStack, LayerSpec};
use navette_ga::bridge::{EvaluatorOptions, FitnessEvaluator, MemeticOperator};
use navette_ga::decoder::{Decoder, Genome, LayerGene};
use navette_ga::materials::{decoder_pool, load_library, stack_template, ThicknessBounds};

fn table(name: &str, n: f64, wavelengths: &[f64]) -> MaterialDef {
    MaterialDef {
        name: name.to_string(),
        code: None,
        model: "TableMaterial".to_string(),
        params: BTreeMap::new(),
        n_data: Some(TabData {
            wavelengths: wavelengths.to_vec(),
            values: vec![n; wavelengths.len()],
        }),
        k_data: None,
    }
}

fn target(profile: &[(f64, f64, f64)], wavelengths: &[f64]) -> MeritSpec {
    let mut spec = MeritSpec::new();
    let key = spec.add_key(MeritKey {
        angle: Radians::ZERO,
        curve: CurveId::Rs,
    });
    for &(lo, hi, value) in profile {
        let selected: Vec<usize> = wavelengths
            .iter()
            .enumerate()
            .filter(|(_, wavelength)| **wavelength >= lo && **wavelength <= hi)
            .map(|(index, _)| index)
            .collect();
        spec.add_target(MeritTarget {
            key_idx: key as u32,
            env_idx: 0,
            wavelengths: selected
                .iter()
                .map(|&i| wavelengths[i])
                .collect::<Vec<_>>()
                .into(),
            kind: ConstraintKind::Exact,
            transform: SimTransform::Linear,
            norm_factor: 1.0,
            normalized_targets: vec![value; selected.len()].into(),
            tolerances: vec![1e-3; selected.len()].into(),
            band: Vec::new().into(),
            phase: false,
            differential_passes: None,
            retardance: false,
            ratio: None,
            dispersion: None,
            stokes: None,
            point_weights: Vec::new().into(),
            integral: false,
            weight: 1.0,
            count_norm: None,
        })
        .unwrap();
    }
    spec
}

use navette_ga::bench::{
    expected_running_time, family_rank_sum, run_comparison_campaign, ComparisonArm,
    ComparisonOptions,
};
use navette_ga::engines::{
    H0Config, IslandRecipeConfig, MemeticSchedule, ProposalConfig, RecipeAConfig, VlRecipeAConfig,
};

fn number(args: &[String], flag: &str, default: usize) -> usize {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

fn genome_from_stack(decoder: &Decoder, stack: &DesignStack) -> Genome {
    let materials = decoder.pool().materials();
    Genome::from(
        stack
            .films()
            .iter()
            .map(|film| {
                let index = materials
                    .iter()
                    .position(|material| material.name() == film.material.as_ref())
                    .unwrap();
                let material = (index as f64 + 0.5) / materials.len() as f64;
                let lo = materials[index].min_thickness_nm();
                let hi = materials[index].max_thickness_nm();
                let thickness = ((film.d_nm - lo) / (hi - lo)).clamp(0.0, 1.0);
                LayerGene::new(material, thickness, 1.0)
            })
            .collect::<Vec<_>>(),
    )
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let seed_count = number(&args, "--seeds", 7).clamp(1, 20);
    let max_layers = number(&args, "--max-layers", 40).clamp(2, 60);
    let fixture = args
        .iter()
        .position(|arg| arg == "--fixture")
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
        .unwrap_or("longpass");
    let shortpass = match fixture {
        "longpass" => false,
        "shortpass" => true,
        other => panic!("unknown fixture {other:?}; use longpass or shortpass"),
    };
    let wavelengths: Vec<f64> = (400..=900).step_by(5).map(f64::from).collect();
    let library = vec![
        table("H", 2.30, &wavelengths),
        table("L", 1.45, &wavelengths),
    ];
    let bounds: BTreeMap<String, ThicknessBounds> = [
        ("H".to_string(), ThicknessBounds::new(10.0, 300.0).unwrap()),
        ("L".to_string(), ThicknessBounds::new(10.0, 300.0).unwrap()),
    ]
    .into_iter()
    .collect();
    let loaded = load_library(&library, &wavelengths, &bounds).unwrap();
    let pool = decoder_pool(&loaded).unwrap();
    let decoder = Decoder::without_constraints(pool)
        .unwrap()
        .with_max_layers(60);
    let ambient = LayerSpec::constant("air", 1.0, 0.0, 0.0, wavelengths.len());
    let substrate = LayerSpec::constant("substrate", 1.52, 0.0, 0.0, wavelengths.len());
    let template = stack_template(ambient.clone(), substrate.clone(), &loaded).unwrap();
    let profile = if shortpass {
        [(400.0, 500.0, 1.0), (600.0, 900.0, 0.0)]
    } else {
        [(400.0, 500.0, 0.0), (600.0, 900.0, 1.0)]
    };
    let spec = target(&profile, &wavelengths);
    let mut evaluator = FitnessEvaluator::new(
        wavelengths.clone(),
        &[0.0],
        spec.clone(),
        EvaluatorOptions {
            clamp_min_nm: 10.0,
            clamp_max_nm: 300.0,
            thin_layer_policy: navette::smatrix::synthesis::config::ThinLayerPolicy::ClampUpAlways,
            lm: navette::lsq::lm::LmConfig {
                max_iterations: 1_000,
                ..Default::default()
            },
        },
    )
    .unwrap();
    let spectral = SpectralInputs::from_spec(&spec, &[Radians::ZERO], &wavelengths).unwrap();
    let operator = MemeticOperator::from_template(
        spectral,
        &template,
        navette::smatrix::synthesis::cycle::NeedleCycleConfig {
            max_needles: 8,
            scan_step_nm: 10.0,
            boundary_needles: false,
            ..Default::default()
        },
    )
    .unwrap();

    let mut films = Vec::new();
    for index in 0..20 {
        let position = if shortpass { 19 - index } else { index };
        let centre = 450.0 + position as f64 * 10.0;
        let mut h = LayerSpec::constant("H", 2.30, 0.0, centre / (4.0 * 2.30), wavelengths.len());
        h.label = Some("H".into());
        let mut l = LayerSpec::constant("L", 1.45, 0.0, centre / (4.0 * 1.45), wavelengths.len());
        l.label = Some("L".into());
        films.push(h);
        films.push(l);
    }
    let seed_stack = DesignStack::with_films(ambient, substrate, films).unwrap();
    let seed = genome_from_stack(&decoder, &seed_stack);

    let options = ComparisonOptions {
        recipe: RecipeAConfig {
            population_size: 12,
            generations: 12,
            max_layers,
            mutation_sigma: 0.15,
            seed: None,
        },
        h0: H0Config {
            starts: 3,
            max_layers,
            mutation_sigma: 0.2,
            needle_steps: 1,
            seed: None,
        },
        islands: IslandRecipeConfig {
            islands: 2,
            migration_interval: 4,
            migration_count: 1,
        },
        vl: VlRecipeAConfig {
            population_size: 12,
            generations: 12,
            min_layers: 10,
            max_layers: 50,
            mutation_sigma: 0.15,
            length_penalty: None,
            seed: None,
        },
        memetic: MemeticSchedule {
            interval: 6,
            top: 2,
            max_insertions: 1,
        },
        operator: Some(operator),
        seeds: vec![seed],
        seed_fraction: 0.5,
        proposal: Some(ProposalConfig {
            noise_sigma: 0.10,
            proposals_per_seed: 4,
        }),
        finish_h7: true,
        terminal_insertions: 10,
        ..ComparisonOptions::default()
    };
    let seeds = (0..seed_count)
        .map(|offset| 20261121 + offset as u64)
        .collect::<Vec<_>>();
    let campaign = run_comparison_campaign(
        &decoder,
        &template,
        &mut evaluator,
        &[
            ComparisonArm::H0,
            ComparisonArm::H1,
            ComparisonArm::H2,
            ComparisonArm::H3,
            ComparisonArm::H4,
            ComparisonArm::H5,
            ComparisonArm::H6,
            ComparisonArm::H7,
        ],
        &seeds,
        &options,
    )
    .unwrap();
    for entry in &campaign {
        let best = entry
            .runs
            .iter()
            .min_by(|a, b| a.result.merit.total_cmp(&b.result.merit))
            .unwrap();
        let sim = evaluator.simulate(&best.result.stack).unwrap();
        let curve = sim.curve(CurveId::Rs).unwrap();
        let (pass_lo, pass_hi, stop_lo, stop_hi) = if shortpass {
            (600.0, 900.0, 400.0, 500.0)
        } else {
            (400.0, 500.0, 600.0, 900.0)
        };
        let pass: Vec<f64> = wavelengths
            .iter()
            .zip(curve.iter())
            .filter(|(w, _)| **w >= pass_lo && **w <= pass_hi)
            .map(|(_, r)| *r)
            .collect();
        let stop: Vec<f64> = wavelengths
            .iter()
            .zip(curve.iter())
            .filter(|(w, _)| **w >= stop_lo && **w <= stop_hi)
            .map(|(_, r)| *r)
            .collect();
        let avg = |x: &[f64]| x.iter().sum::<f64>() / x.len().max(1) as f64;
        let pass_avg = avg(&pass);
        let stop_avg = avg(&stop);
        let min_nm = best
            .result
            .stack
            .films()
            .iter()
            .map(|film| film.d_nm)
            .reduce(f64::min)
            .unwrap_or(0.0);
        let deep_production = best.result.decoded.len() >= 40
            && min_nm >= 10.0 - 1e-9
            && pass_avg < 0.02
            && stop_avg > 0.99;
        println!(
            "FILTER_CAMPAIGN fixture={} arm={} best_merit={:.6} median_merit={:.6} median_evals={:.1}              best_layers={} best_pass={:.6} best_stop={:.6} best_min_nm={:.6}              deep_production={}",
            fixture,
            entry.arm.label(),
            entry.best_merit().unwrap(),
            entry.median_merit().unwrap(),
            entry.median_evaluations().unwrap(),
            best.result.decoded.len(),
            pass_avg,
            stop_avg,
            min_nm,
            deep_production
        );
    }
    let ert = expected_running_time(&campaign, 30_000.0).unwrap();
    for arm in &ert.arms {
        println!(
            "FILTER_ERT arm={} successes={}/{} ert={}",
            arm.arm.label(),
            arm.successes,
            arm.trials,
            arm.ert
                .map_or_else(|| "none".to_string(), |v| format!("{v:.1}"))
        );
    }
    for pair in family_rank_sum(&campaign).unwrap() {
        println!(
            "FILTER_FAMILY {} vs {} p={:.9} holm={:.9}",
            pair.first.label(),
            pair.second.label(),
            pair.report.p_value,
            pair.adjusted_p
        );
    }
}
