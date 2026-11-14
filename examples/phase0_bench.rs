// SPDX-License-Identifier: LGPL-3.0-or-later
//! G9 micro-benchmark: the bridge on a 61-wavelength quarter-wave AR.
//!
//! Run with:
//! ```text
//! cargo run --release --example phase0_bench
//! ```
//!
//! The reference numbers are navette's published benches: one LM thickness
//! optimize is 2.1 ms, and one needle insertion's scan is 2.2-2.6 ms. This
//! harness measures the bridge's evaluate and LM-finish paths, not the needle
//! scan (A1 is still pending), so it is the Phase 0 baseline the later G9
//! check compares against.

use std::time::Instant;

use navette::smatrix::angle::Radians;
use navette::smatrix::synthesis::merit::{
    ConstraintKind, CurveId, MeritKey, MeritSpec, MeritTarget, SimTransform,
};
use navette::smatrix::synthesis::structure::{DesignStack, LayerSpec};

use navette_ga::bridge::{EvaluatorOptions, FitnessEvaluator};

const LAMBDA0_NM: f64 = 550.0;
const N_SUBSTRATE: f64 = 1.52;

fn wavelengths() -> Vec<f64> {
    (0..61).map(|i| 400.0 + 5.0 * i as f64).collect()
}

fn zero_reflectance_spec() -> MeritSpec {
    let mut spec = MeritSpec::new();
    let key = spec.add_key(MeritKey {
        angle: Radians::ZERO,
        curve: CurveId::Rs,
    });
    spec.add_target(MeritTarget {
        key_idx: key as u32,
        env_idx: 0,
        wavelengths: vec![LAMBDA0_NM].into(),
        kind: ConstraintKind::Exact,
        transform: SimTransform::Linear,
        norm_factor: 1.0,
        normalized_targets: vec![0.0].into(),
        tolerances: vec![1e-3].into(),
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

fn quarter_wave_stack(scale: f64) -> DesignStack {
    let n_film = N_SUBSTRATE.sqrt();
    let d_nm = scale * LAMBDA0_NM / (4.0 * n_film);
    let nw = wavelengths().len();
    let mut ambient = LayerSpec::constant("air", 1.0, 0.0, 0.0, nw);
    ambient.optimize = false;
    ambient.needle = false;
    let mut substrate = LayerSpec::constant("substrate", N_SUBSTRATE, 0.0, 0.0, nw);
    substrate.optimize = false;
    substrate.needle = false;
    let mut film = LayerSpec::constant("film", n_film, 0.0, d_nm, nw);
    film.optimize = true;
    film.needle = true;
    DesignStack::with_films(ambient, substrate, vec![film]).unwrap()
}

fn millis_per_call(mut f: impl FnMut(), warmup: usize, iterations: usize) -> f64 {
    for _ in 0..warmup {
        f();
    }
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    start.elapsed().as_secs_f64() * 1e3 / iterations as f64
}

fn main() {
    let ev = FitnessEvaluator::new(
        wavelengths(),
        &[0.0],
        zero_reflectance_spec(),
        EvaluatorOptions::default(),
    )
    .unwrap();

    let stack = quarter_wave_stack(1.0);
    let merit = ev.evaluate(&stack).unwrap();
    let sim_ms = millis_per_call(
        || {
            let _ = ev.simulate(&stack).unwrap();
        },
        3,
        20,
    );

    let mut refine_ev = ev.clone();
    let lm_ms = millis_per_call(
        || {
            let mut detuned = quarter_wave_stack(0.8);
            let _ = refine_ev.refine(&mut detuned).unwrap();
        },
        1,
        10,
    );

    println!(
        "Phase 0 G9 micro-bench ({} wavelengths, 1 angle)",
        wavelengths().len()
    );
    println!("  quarter-wave merit:       {merit:.3e}");
    println!("  simulate (61 lambda):     {sim_ms:.3} ms/call");
    println!("  LM finish (1 film):       {lm_ms:.3} ms/call");
    println!("  navette reference:        LM 2.1 ms; needle scan 2.2-2.6 ms");
}
