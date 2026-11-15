// SPDX-License-Identifier: LGPL-3.0-or-later
//! Needle-grown steep-edge filter (Recipe B growth path).
//!
//! A chirped H/L quarter-wave stack gives a long-pass edge; the target rewards
//! low reflectance below 500 nm and high reflectance above 600 nm. The seed is
//! `--pairs` H/L pairs (default 5), and the operator runs for up to
//! `--insertions` improving needle steps (default 30). This is a growth probe,
//! not a claimed optimum.

use std::collections::BTreeMap;

use navette::smatrix::angle::Radians;
use navette::smatrix::synthesis::design_config::{MaterialDef, TabData};
use navette::smatrix::synthesis::merit::{
    ConstraintKind, CurveId, MeritKey, MeritSpec, MeritTarget, SimTransform,
};
use navette::smatrix::synthesis::pipeline::SpectralInputs;
use navette::smatrix::synthesis::structure::{DesignStack, LayerSpec};
use navette_ga::bridge::{EvaluatorOptions, FitnessEvaluator, MemeticOperator, StackTemplate};
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

fn number(args: &[String], flag: &str, default: usize) -> usize {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let pairs = number(&args, "--pairs", 5);
    let max_insertions = number(&args, "--insertions", 30);
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
    let decoder = Decoder::without_constraints(pool).unwrap();
    let ambient = LayerSpec::constant("air", 1.0, 0.0, 0.0, wavelengths.len());
    let substrate = LayerSpec::constant("substrate", 1.52, 0.0, 0.0, wavelengths.len());
    let mut h = LayerSpec::constant("H", 2.30, 0.0, 100.0, wavelengths.len());
    h.label = Some("H".into());
    let mut l = LayerSpec::constant("L", 1.45, 0.0, 100.0, wavelengths.len());
    l.label = Some("L".into());
    let template = StackTemplate::new(ambient.clone(), substrate.clone(), vec![h, l]).unwrap();
    let spec = target(&[(400.0, 500.0, 0.0), (600.0, 900.0, 1.0)], &wavelengths);
    let mut evaluator = FitnessEvaluator::new(
        wavelengths.clone(),
        &[0.0],
        spec.clone(),
        EvaluatorOptions {
            clamp_min_nm: 10.0,
            clamp_max_nm: 300.0,
            thin_layer_policy: navette::smatrix::synthesis::config::ThinLayerPolicy::ClampUpAlways,
            lm: navette::lsq::lm::LmConfig {
                max_iterations: 3_000,
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
    for index in 0..pairs {
        let centre = 450.0 + index as f64 * (500.0 / pairs as f64);
        let mut h = LayerSpec::constant("H", 2.30, 0.0, centre / (4.0 * 2.30), wavelengths.len());
        h.label = Some("H".into());
        let mut l = LayerSpec::constant("L", 1.45, 0.0, centre / (4.0 * 1.45), wavelengths.len());
        l.label = Some("L".into());
        films.push(h);
        films.push(l);
    }
    let mut stack = DesignStack::with_films(ambient, substrate, films).unwrap();
    let seed_merit = evaluator.evaluate(&stack).unwrap();
    evaluator.refine(&mut stack).unwrap();
    println!(
        "GROWTH pairs={pairs} seed_merit={seed_merit:.6} after_lm={:.6} layers={}",
        evaluator.evaluate(&stack).unwrap(),
        stack.films().len()
    );

    let report = operator
        .apply(&mut evaluator, &mut stack, max_insertions)
        .unwrap();
    println!(
        "GROWTH operator insertions={} scans={} merit={:.6} layers={}",
        report.steps.len(),
        report.scans,
        report.final_merit,
        stack.films().len()
    );
    let (final_merit, lm) = evaluator.refine_report(&mut stack).unwrap();
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
    let production = legal.is_ok()
        && stack.films().len() >= 40
        && min_nm >= 10.0
        && avg(&pass) < 0.05
        && avg(&stop) > 0.90;
    println!(
        "GROWTH finish_merit={final_merit:.6} layers={} min_nm={min_nm:.3} \
         pass_avg_R={:.6} stop_avg_R={:.6} lm_converged={} production={}",
        stack.films().len(),
        avg(&pass),
        avg(&stop),
        lm.as_ref()
            .is_some_and(|report| report.termination.is_converged()),
        if production { "pass" } else { "fail" }
    );
    for w in [450.0, 500.0, 550.0, 600.0, 700.0] {
        let idx = wavelengths.iter().position(|x| *x == w).unwrap();
        println!("GROWTH R({w})={:.6}", curve[idx]);
    }
    if let Err(error) = legal {
        println!("GROWTH a4_error={error}");
    }
}
