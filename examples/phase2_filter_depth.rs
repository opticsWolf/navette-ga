// SPDX-License-Identifier: LGPL-3.0-or-later
//! Merit-versus-depth bake for the steep-edge filter.
//!
//! A chirped H/L quarter-wave stack gives a long-pass edge; the target rewards
//! low reflectance below 500 nm and high reflectance above 600 nm. This is the
//! starting point for the Recipe B filter run, not a claimed optimum yet.

use std::collections::BTreeMap;

use navette::smatrix::angle::Radians;
use navette::smatrix::synthesis::design_config::{MaterialDef, TabData};
use navette::smatrix::synthesis::merit::{
    ConstraintKind, CurveId, MeritKey, MeritSpec, MeritTarget, SimTransform,
};
use navette::smatrix::synthesis::structure::{DesignStack, LayerSpec};
use navette_ga::bridge::{EvaluatorOptions, FitnessEvaluator};
use navette_ga::decoder::Decoder;
use navette_ga::materials::{decoder_pool, load_library, ThicknessBounds};

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
        thickness_bounds: Some(ThicknessBounds::new(10.0, 300.0).unwrap()),
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

fn run(depth: usize, wavelengths: &[f64], decoder: &Decoder, evaluator: &mut FitnessEvaluator) {
    let ambient = LayerSpec::constant("air", 1.0, 0.0, 0.0, wavelengths.len());
    let substrate = LayerSpec::constant("substrate", 1.52, 0.0, 0.0, wavelengths.len());
    let pairs = depth / 2;
    let mut films = Vec::with_capacity(depth);
    for index in 0..pairs {
        let centre = 450.0 + index as f64 * (500.0 / pairs.max(1) as f64);
        let mut h = LayerSpec::constant("H", 2.30, 0.0, centre / (4.0 * 2.30), wavelengths.len());
        h.label = Some("H".into());
        h.optimize = true;
        h.needle = true;
        let mut l = LayerSpec::constant("L", 1.45, 0.0, centre / (4.0 * 1.45), wavelengths.len());
        l.label = Some("L".into());
        l.optimize = true;
        l.needle = true;
        films.push(h);
        films.push(l);
    }
    let mut stack = DesignStack::with_films(ambient, substrate, films).unwrap();
    let seed_merit = evaluator.evaluate(&stack).unwrap();
    let (finish_merit, lm) = evaluator.refine_report(&mut stack).unwrap();
    let sim = evaluator.simulate(&stack).unwrap();
    let curve = sim.curve(CurveId::Rs).unwrap();
    let pass: Vec<f64> = wavelengths
        .iter()
        .zip(curve.iter())
        .filter(|(wavelength, _)| **wavelength <= 500.0)
        .map(|(_, reflectance)| *reflectance)
        .collect();
    let stop: Vec<f64> = wavelengths
        .iter()
        .zip(curve.iter())
        .filter(|(wavelength, _)| **wavelength >= 600.0)
        .map(|(_, reflectance)| *reflectance)
        .collect();
    let avg = |values: &[f64]| values.iter().sum::<f64>() / values.len().max(1) as f64;
    let min_nm = stack
        .films()
        .iter()
        .map(|film| film.d_nm)
        .reduce(f64::min)
        .unwrap_or(0.0);
    let legal = decoder.validate_stack(&stack);
    let lm_converged = lm
        .as_ref()
        .is_some_and(|report| report.termination.is_converged());
    println!(
        "DEPTH depth={depth} seed_merit={seed_merit:.6} finish_merit={finish_merit:.6} \
         layers={} min_nm={min_nm:.3} pass_avg_R={:.6} stop_avg_R={:.6} \
         lm_converged={lm_converged} a4={}",
        stack.films().len(),
        avg(&pass),
        avg(&stop),
        if legal.is_ok() { "pass" } else { "fail" }
    );
    if let Err(error) = legal {
        println!("DEPTH depth={depth} a4_error={error}");
    }
}

fn main() {
    let wavelengths: Vec<f64> = (400..=900).step_by(5).map(f64::from).collect();
    let library = vec![
        table("H", 2.30, &wavelengths),
        table("L", 1.45, &wavelengths),
    ];
    let loaded = load_library(&library, &wavelengths).unwrap();
    let pool = decoder_pool(&loaded).unwrap();
    let decoder = Decoder::without_constraints(pool).unwrap();
    let spec = target(&[(400.0, 500.0, 0.0), (600.0, 900.0, 1.0)], &wavelengths);
    let mut evaluator = FitnessEvaluator::new(
        wavelengths.clone(),
        &[0.0],
        spec,
        EvaluatorOptions {
            clamp_min_nm: 10.0,
            clamp_max_nm: 300.0,
            thin_layer_policy: navette::smatrix::synthesis::config::ThinLayerPolicy::ClampUpAlways,
            lm: navette::lsq::lm::LmConfig {
                max_iterations: 5_000,
                ..Default::default()
            },
        },
    )
    .unwrap();

    for depth in [10, 20, 30, 40, 50] {
        run(depth, &wavelengths, &decoder, &mut evaluator);
    }
}
