// SPDX-License-Identifier: LGPL-3.0-or-later
//! Phase 1 C00-class antireflection run.
//!
//! This is the reproducible ledger artifact for the Phase 1 exit: a two-material
//! H/L antireflection search over 400–900 nm, a terminal needle/LM finish, and
//! the scan certificate printed with the result. It is intentionally small
//! enough to rerun as a check; the numbers in the ledger note come from this
//! exact command. Pass `--memetic` to run the same config through the
//! mid-loop H3 schedule and print its wall-time share, or `--h0-budget N` to
//! compare H0 and H1 at the same solver-evaluation count.

use std::collections::BTreeMap;

use navette::smatrix::angle::Radians;
use navette::smatrix::synthesis::design_config::{MaterialDef, TabData};
use navette::smatrix::synthesis::merit::{
    ConstraintKind, CurveId, MeritKey, MeritSpec, MeritTarget, SimTransform,
};
use navette::smatrix::synthesis::pipeline::SpectralInputs;
use navette::smatrix::synthesis::structure::LayerSpec;
use navette_ga::bridge::{EvaluatorOptions, FitnessEvaluator, MemeticOperator, StackTemplate};
use navette_ga::decoder::Decoder;
use navette_ga::engines::{
    compare_h0_and_ga_repeats, finish_recipe_a, run_recipe_a, run_recipe_a_memetic, H0Config,
    MemeticSchedule, RecipeAConfig, TerminalReport,
};
use navette_ga::materials::{decoder_pool, load_library, stack_template, ThicknessBounds};
use navette_ga::study::mann_whitney_u;

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

fn main() {
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
    let decoder = Decoder::without_constraints(pool)
        .unwrap()
        .with_max_layers(32);
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
    let cfg = RecipeAConfig {
        population_size: 64,
        generations: 100,
        max_layers: 32,
        mutation_sigma: 0.2,
        seed: Some(20261114),
    };
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

    // Default mode reproduces the Phase 1 artifact. `--memetic` runs the same
    // config with a mid-loop schedule so its wall-time share can be recorded.
    let args: Vec<String> = std::env::args().collect();
    let number = |flag: &str, default: usize| -> usize {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|index| args.get(index + 1))
            .and_then(|value| value.parse().ok())
            .unwrap_or(default)
    };
    if let Some(budget) = args
        .iter()
        .position(|arg| arg == "--h0-budget")
        .and_then(|index| args.get(index + 1))
        .and_then(|value| value.parse::<usize>().ok())
    {
        let h0_cfg = H0Config {
            starts: 1,
            max_layers: cfg.max_layers,
            mutation_sigma: cfg.mutation_sigma,
            needle_steps: 0,
            seed: cfg.seed,
        };
        let repeats = number("--h0-repeats", 1);
        let base_seed = cfg.seed.unwrap_or(20261114);
        let seeds: Vec<u64> = (0..repeats)
            .map(|index| base_seed.wrapping_add(index as u64))
            .collect();
        let comparison = compare_h0_and_ga_repeats(
            &decoder, &template, &evaluator, &h0_cfg, &cfg, &seeds, budget,
        )
        .unwrap();
        println!("PHASE2_H0_BUDGET");
        let mut h0_r: Vec<f64> = Vec::with_capacity(seeds.len());
        let mut ga_r: Vec<f64> = Vec::with_capacity(seeds.len());
        let mut h0_merit = Vec::with_capacity(seeds.len());
        let mut ga_merit = Vec::with_capacity(seeds.len());
        for (index, seed) in seeds.iter().enumerate() {
            let h0 = &comparison.h0[index];
            let ga = &comparison.ga[index];
            let h0_avg = {
                let sim = evaluator.simulate(&h0.result.stack).unwrap();
                let reflectance = sim.curve(CurveId::Rs).unwrap();
                reflectance.iter().sum::<f64>() / reflectance.len() as f64
            };
            let ga_avg = {
                let sim = evaluator.simulate(&ga.result.stack).unwrap();
                let reflectance = sim.curve(CurveId::Rs).unwrap();
                reflectance.iter().sum::<f64>() / reflectance.len() as f64
            };
            h0_r.push(h0_avg);
            ga_r.push(ga_avg);
            h0_merit.push(h0.result.merit);
            ga_merit.push(ga.result.merit);
            println!(
                "seed={seed} h0_merit={:.9} h0_avg_R={h0_avg:.9} h0_evaluations={} ga_merit={:.9} ga_avg_R={ga_avg:.9} ga_evaluations={}",
                h0.result.merit, h0.evaluations, ga.result.merit, ga.evaluations
            );
        }
        let median = |values: &mut [f64]| -> f64 {
            values.sort_by(f64::total_cmp);
            let mid = values.len() / 2;
            if values.len().is_multiple_of(2) {
                0.5 * (values[mid - 1] + values[mid])
            } else {
                values[mid]
            }
        };
        println!("budget={budget} repeats={}", seeds.len());
        println!(
            "h0_median_merit={:.9} h0_median_avg_R={:.9}",
            median(&mut h0_merit),
            median(&mut h0_r)
        );
        println!(
            "ga_median_merit={:.9} ga_median_avg_R={:.9}",
            median(&mut ga_merit),
            median(&mut ga_r)
        );
        let rank_sum = mann_whitney_u(&h0_merit, &ga_merit).unwrap();
        println!(
            "rank_sum_u={:.6} p_value={:.9} exact={}",
            rank_sum.u, rank_sum.p_value, rank_sum.exact
        );
        return;
    }

    let memetic = args.iter().any(|arg| arg == "--memetic");
    let (result, memetic_report) = if memetic {
        let defaults = MemeticSchedule::default();
        let schedule = MemeticSchedule {
            interval: number("--memetic-interval", defaults.interval),
            top: number("--memetic-top", defaults.top),
            max_insertions: number("--memetic-insertions", defaults.max_insertions),
        };
        println!(
            "memetic_schedule={},{},{}",
            schedule.interval, schedule.top, schedule.max_insertions
        );
        let run = run_recipe_a_memetic(
            &decoder,
            &template,
            &mut evaluator,
            &cfg,
            &operator,
            &schedule,
        )
        .unwrap();
        (run.best, Some(run.report))
    } else {
        (
            run_recipe_a(&decoder, &template, &evaluator, &cfg).unwrap(),
            None,
        )
    };
    let (finished, report): (_, TerminalReport) =
        finish_recipe_a(&decoder, &template, &mut evaluator, result, &operator, 20).unwrap();

    let sim = evaluator.simulate(&finished.stack).unwrap();
    let reflectance = sim.curve(CurveId::Rs).unwrap();
    let avg_r = reflectance.iter().sum::<f64>() / reflectance.len() as f64;
    let max_r = reflectance
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    println!("PHASE1_C00_AR");
    println!("merit={:.9}", finished.merit);
    println!("avg_R={avg_r:.9}");
    println!("max_R={max_r:.9}");
    println!("layers={}", finished.decoded.len());
    println!("insertions={}", report.insertions);
    println!("lm_converged={}", report.lm_converged);
    println!("min_thickness_nm={:.6}", report.min_thickness_nm);
    println!("scan_candidates_after={}", report.scan_candidates_after);
    println!("actual_improving_after={}", report.actual_improving_after);
    if let Some(share) = memetic_report {
        println!("memetic_passes={}", share.passes);
        println!("memetic_refined={}", share.refined);
        println!("memetic_improved={}", share.improved);
        println!("memetic_insertions={}", share.insertions);
        println!("memetic_scans={}", share.scans);
        println!("memetic_prediction_rejected={}", share.prediction_rejected);
        println!("memetic_write_back_rejected={}", share.write_back_rejected);
        println!("memetic_merit_rejected={}", share.merit_rejected);
        println!("memetic_ga_seconds={:.6}", share.ga_seconds);
        println!("memetic_seconds={:.6}", share.memetic_seconds);
        println!("memetic_share={:.6}", share.memetic_share());
    }
    println!(
        "stack={}",
        finished
            .stack
            .films()
            .iter()
            .map(|film| format!("{}:{:.3}", film.material, film.d_nm))
            .collect::<Vec<_>>()
            .join(",")
    );
}
