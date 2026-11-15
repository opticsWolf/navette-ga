// SPDX-License-Identifier: LGPL-3.0-or-later
//! H0–H7 comparison campaign on the C00 antireflection fixture.
//!
//! This runs the eight E09 arms through `bench::run_comparison_campaign` and
//! prints the fixed-target ERT and Holm-adjusted pairwise rank-sum family.
//! The default is ten seeds; `--seeds N` and `--target X` override the campaign
//! size and target. This is fixed-seed evidence, not the final campaign.

use std::collections::BTreeMap;

use navette::smatrix::angle::Radians;
use navette::smatrix::synthesis::design_config::{MaterialDef, TabData};
use navette::smatrix::synthesis::merit::{
    ConstraintKind, CurveId, MeritKey, MeritSpec, MeritTarget, SimTransform,
};
use navette::smatrix::synthesis::pipeline::SpectralInputs;
use navette::smatrix::synthesis::structure::LayerSpec;
use navette_ga::bridge::{EvaluatorOptions, FitnessEvaluator, MemeticOperator, StackTemplate};
use navette_ga::decoder::{Decoder, Genome, LayerGene};
use navette_ga::engines::{
    H0Config, IslandRecipeConfig, MemeticSchedule, ProposalConfig, RecipeAConfig, VlRecipeAConfig,
};
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

fn spec(wavelengths: &[f64]) -> MeritSpec {
    let mut spec = MeritSpec::new();
    let key = spec.add_key(MeritKey {
        angle: Radians::ZERO,
        curve: CurveId::Rs,
    });
    let zeros = vec![0.0; wavelengths.len()];
    let tolerances = vec![1e-3; wavelengths.len()];
    spec.add_target(MeritTarget {
        key_idx: key as u32,
        env_idx: 0,
        wavelengths: wavelengths.to_vec().into(),
        kind: ConstraintKind::Exact,
        transform: SimTransform::Linear,
        norm_factor: 1.0,
        normalized_targets: zeros.into(),
        tolerances: tolerances.into(),
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
    spec
}

use navette_ga::bench::{
    expected_running_time, family_rank_sum, run_comparison_campaign, ComparisonArm,
    ComparisonOptions,
};
fn option<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let seed_count = option(&args, "--seeds")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(10)
        .clamp(1, 50);
    let target_merit = option(&args, "--target")
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(30_000.0);
    let wavelengths: Vec<f64> = (400..=900).step_by(25).map(f64::from).collect();
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
    let decoder = Decoder::without_constraints(pool).unwrap();
    let ambient = LayerSpec::constant("air", 1.0, 0.0, 0.0, wavelengths.len());
    let substrate = LayerSpec::constant("substrate", 1.52, 0.0, 0.0, wavelengths.len());
    let template: StackTemplate = stack_template(ambient, substrate, &loaded).unwrap();
    let target = spec(&wavelengths);
    let mut evaluator = FitnessEvaluator::new(
        wavelengths.clone(),
        &[0.0],
        target.clone(),
        EvaluatorOptions {
            clamp_min_nm: 10.0,
            clamp_max_nm: 300.0,
            ..EvaluatorOptions::default()
        },
    )
    .unwrap();
    let spectral = SpectralInputs::from_spec(&target, &[Radians::ZERO], &wavelengths).unwrap();
    let operator = MemeticOperator::new(
        spectral,
        template.needle_pool().unwrap(),
        navette::smatrix::synthesis::cycle::NeedleCycleConfig {
            max_needles: 8,
            scan_step_nm: 10.0,
            boundary_needles: false,
            ..Default::default()
        },
    );
    let seed = Genome::from(vec![
        LayerGene::new(0.25, 0.20, 1.0),
        LayerGene::new(0.75, 0.30, 1.0),
        LayerGene::new(0.25, 0.20, 1.0),
    ]);
    let options = ComparisonOptions {
        recipe: RecipeAConfig {
            population_size: 24,
            generations: 30,
            max_layers: 16,
            mutation_sigma: 0.2,
            seed: None,
        },
        h0: H0Config {
            starts: 8,
            max_layers: 16,
            mutation_sigma: 0.2,
            needle_steps: 1,
            seed: None,
        },
        islands: IslandRecipeConfig {
            islands: 2,
            migration_interval: 5,
            migration_count: 1,
        },
        vl: VlRecipeAConfig {
            population_size: 24,
            generations: 30,
            min_layers: 3,
            max_layers: 16,
            mutation_sigma: 0.2,
            length_penalty: None,
            seed: None,
        },
        memetic: MemeticSchedule {
            interval: 10,
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
        terminal_insertions: 20,
        ..ComparisonOptions::default()
    };
    let seeds = (0..seed_count)
        .map(|index| 20261121_u64 + index as u64)
        .collect::<Vec<_>>();
    println!("CAMPAIGN seeds={} target={target_merit:.1}", seeds.len());
    let campaign = run_comparison_campaign(
        &decoder,
        &template,
        &mut evaluator,
        &ComparisonArm::ALL,
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
        let avg_r = curve.iter().sum::<f64>() / curve.len() as f64;
        println!(
            "CAMPAIGN arm={} best_merit={:.6} median_merit={:.6} median_evals={:.1} \
             best_avg_R={avg_r:.6} layers={}",
            entry.arm.label(),
            entry.best_merit().unwrap(),
            entry.median_merit().unwrap(),
            entry.median_evaluations().unwrap(),
            best.result.decoded.len()
        );
    }

    let ert = expected_running_time(&campaign, target_merit).unwrap();
    for arm in &ert.arms {
        println!(
            "ERT target={} arm={} trials={} successes={} rate={:.3} ert={}",
            ert.target_merit,
            arm.arm.label(),
            arm.trials,
            arm.successes,
            arm.success_rate(),
            arm.ert
                .map_or_else(|| "none".to_string(), |value| format!("{value:.1}"))
        );
    }

    let family = family_rank_sum(&campaign).unwrap();
    for pair in &family {
        println!(
            "FAMILY {} vs {} u={:.3} p={:.9} exact={} holm={:.9}",
            pair.first.label(),
            pair.second.label(),
            pair.report.u,
            pair.report.p_value,
            pair.report.exact,
            pair.adjusted_p
        );
    }
}
