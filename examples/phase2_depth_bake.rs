// SPDX-License-Identifier: LGPL-3.0-or-later
//! Phase 2 depth bake on the C00-class AR fixture.
//!
//! Runs the same H/L, 400-900 nm antireflection target at several fixed layer
//! caps and records the best legal design, its measured merit, and a
//! production-style screen (A4 legality, d_min, layer cap). The points are
//! also inserted into the standard MOME archive so the depth/thickness
//! occupancy is visible.
//!
//! This is not the steep-edge Recipe B filter and it is not an equal-compute
//! campaign. It is the engineering bake that answers whether the fixed-width
//! path still produces legal, manufacturable screens at 40 layers.

use std::collections::BTreeMap;

use navette::smatrix::angle::Radians;
use navette::smatrix::synthesis::design_config::{MaterialDef, TabData};
use navette::smatrix::synthesis::merit::{
    ConstraintKind, CurveId, MeritKey, MeritSpec, MeritTarget, SimTransform,
};
use navette::smatrix::synthesis::structure::LayerSpec;
use navette_ga::bridge::{EvaluatorOptions, FitnessEvaluator, StackTemplate};
use navette_ga::decoder::Decoder;
use navette_ga::engines::{run_recipe_a, RecipeAConfig};
use navette_ga::materials::{decoder_pool, load_library, stack_template, ThicknessBounds};
use navette_ga::qd::{ArchiveConfig, MomeArchive};

const D_MIN_NM: f64 = 10.0;

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
        thickness_bounds: Some(ThicknessBounds::new(D_MIN_NM, 300.0).unwrap()),
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

fn production_filter(
    decoder: &Decoder,
    stack: &navette::smatrix::synthesis::structure::DesignStack,
    max_layers: usize,
) -> Result<(), String> {
    decoder.validate_stack(stack)?;
    if stack.films().len() > max_layers {
        return Err(format!(
            "{} films exceeds the {max_layers}-layer cap",
            stack.films().len()
        ));
    }
    if let Some((index, film)) = stack
        .films()
        .iter()
        .enumerate()
        .find(|(_, film)| film.d_nm < D_MIN_NM)
    {
        return Err(format!(
            "film {index} is {:.3} nm, below the {D_MIN_NM} nm production floor",
            film.d_nm
        ));
    }
    Ok(())
}

fn main() {
    let wavelengths: Vec<f64> = (400..=900).step_by(25).map(f64::from).collect();
    let library = vec![
        table("H", 2.30, &wavelengths),
        table("L", 1.45, &wavelengths),
    ];
    let loaded = load_library(&library, &wavelengths).unwrap();
    let pool = decoder_pool(&loaded).unwrap();
    let decoder = Decoder::without_constraints(pool)
        .unwrap()
        .with_max_layers(40);
    let ambient = LayerSpec::constant("air", 1.0, 0.0, 0.0, wavelengths.len());
    let substrate = LayerSpec::constant("substrate", 1.52, 0.0, 0.0, wavelengths.len());
    let template: StackTemplate = stack_template(ambient, substrate, &loaded).unwrap();
    let target = spec(&wavelengths);
    let evaluator = FitnessEvaluator::new(
        wavelengths.clone(),
        &[0.0],
        target,
        EvaluatorOptions {
            clamp_min_nm: D_MIN_NM,
            clamp_max_nm: 300.0,
            ..EvaluatorOptions::default()
        },
    )
    .unwrap();

    let mut archive = MomeArchive::new(ArchiveConfig {
        depth_bins: 40,
        thickness_bins: 20,
        max_layers: 40.0,
        max_thickness_nm: 12_000.0,
        max_front: 16,
    })
    .unwrap();

    println!("PHASE2_DEPTH_BAKE");
    for max_layers in [4usize, 8, 12, 16, 24, 32, 40] {
        let cfg = RecipeAConfig {
            population_size: 32,
            generations: 40,
            max_layers,
            mutation_sigma: 0.2,
            seed: Some(20261116u64.wrapping_add(max_layers as u64)),
        };
        let result = run_recipe_a(&decoder, &template, &evaluator, &cfg).unwrap();
        let sim = evaluator.simulate(&result.stack).unwrap();
        let reflectance = sim.curve(CurveId::Rs).unwrap();
        let avg_r = reflectance.iter().sum::<f64>() / reflectance.len() as f64;
        let min_nm = result
            .stack
            .films()
            .iter()
            .map(|film| film.d_nm)
            .reduce(f64::min)
            .unwrap_or(0.0);
        let filter = production_filter(&decoder, &result.stack, max_layers);
        archive
            .insert_stack(result.merit, result.stack.clone())
            .unwrap();
        println!(
            "depth={max_layers} merit={:.9} avg_R={avg_r:.9} layers={} min_nm={min_nm:.6} filter={}",
            result.merit,
            result.decoded.len(),
            if filter.is_ok() { "pass" } else { "fail" }
        );
        if let Err(error) = filter {
            println!("depth={max_layers} filter_error={error}");
        }
    }

    println!("archive_cells={}", archive.len());
    println!("archive_coverage={:.6}", archive.coverage());
    let mut front: Vec<_> = archive.front_points().collect();
    front.sort_by(|a, b| {
        a.layer_count
            .cmp(&b.layer_count)
            .then_with(|| a.merit.total_cmp(&b.merit))
    });
    for point in front {
        println!(
            "front layers={} merit={:.9} thickness_nm={:.6}",
            point.layer_count, point.merit, point.total_thickness_nm
        );
    }
}
