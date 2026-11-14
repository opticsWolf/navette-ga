// SPDX-License-Identifier: LGPL-3.0-or-later
//! Phase 1 C00-class antireflection run.
//!
//! This is the reproducible ledger artifact for the Phase 1 exit: a two-material
//! H/L antireflection search over 400–900 nm, a terminal needle/LM finish, and
//! the scan certificate printed with the result. It is intentionally small
//! enough to rerun as a check; the numbers in the ledger note come from this
//! exact command.

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
use navette_ga::engines::{finish_recipe_a, run_recipe_a, RecipeAConfig, TerminalReport};
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
            clamp_min_nm: 5.0,
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
    let result = run_recipe_a(&decoder, &template, &evaluator, &cfg).unwrap();
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
