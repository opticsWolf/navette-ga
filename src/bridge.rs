// SPDX-License-Identifier: LGPL-3.0-or-later
//! G1/G4 — fitness bridge and needle-memetic operator.
//!
//! The bridge turns a decoded genome into a navette `DesignStack` (plus any
//! environments), evaluates the joint merit, and returns the scalar fitness
//! with the per-target breakdown used by the QD descriptors. A full generation
//! is batched so stack setup is amortised.
//!
//! The memetic operator runs the needle pipeline over a genome: program-level
//! (`run_needle`) first, then the staged A1 API (`scan/insert/polish/cleanup`)
//! once it lands. Lamarckian write-back is the default; the Baldwinian variant
//! is kept for the V21 ablation.

use std::sync::Arc;

use navette::lsq::lm::LmConfig;
use navette::smatrix::angle::Radians;
use navette::smatrix::synthesis::config::ThinLayerPolicy;
use navette::smatrix::synthesis::context::DesignContext;
use navette::smatrix::synthesis::evaluator::SmatrixContext;
use navette::smatrix::synthesis::merit::{MeritSpec, SimCurves};
use navette::smatrix::synthesis::structure::{ClampReport, DesignStack, LayerSpec};

use crate::decoder::DecodedStack;

/// A material pool plus the fixed ambient/substrate half-spaces.
///
/// `materials[i]` is the optical template for decoder material index `i`.
/// The template's thickness is ignored; every assembled film takes its
/// thickness from the decoded layer. `optimize` and `needle` are template
/// defaults for unpinned films; pinned films always have both switched off.
#[derive(Clone, Debug)]
pub struct StackTemplate {
    pub ambient: LayerSpec,
    pub substrate: LayerSpec,
    pub materials: Vec<LayerSpec>,
}

impl StackTemplate {
    pub fn new(
        mut ambient: LayerSpec,
        mut substrate: LayerSpec,
        materials: Vec<LayerSpec>,
    ) -> Result<Self, String> {
        if materials.is_empty() {
            return Err("stack template needs at least one material".into());
        }
        let num_wavs = ambient.nk.len();
        if num_wavs == 0 {
            return Err("ambient nk is empty".into());
        }
        if substrate.nk.len() != num_wavs {
            return Err(format!(
                "substrate nk length {} != ambient nk length {}",
                substrate.nk.len(),
                num_wavs
            ));
        }
        for (index, material) in materials.iter().enumerate() {
            if material.nk.len() != num_wavs {
                return Err(format!(
                    "material {index} ({:?}) nk length {} != ambient nk length {}",
                    material.material,
                    material.nk.len(),
                    num_wavs
                ));
            }
        }
        ambient.optimize = false;
        ambient.needle = false;
        substrate.optimize = false;
        substrate.needle = false;
        Ok(Self {
            ambient,
            substrate,
            materials,
        })
    }

    /// Build a navette stack from decoded layers.
    pub fn assemble(&self, decoded: &DecodedStack) -> Result<DesignStack, String> {
        let mut films = Vec::with_capacity(decoded.len());
        for (index, layer) in decoded.layers().iter().enumerate() {
            let mut film = self
                .materials
                .get(layer.material)
                .ok_or_else(|| {
                    format!(
                        "decoded layer {index}: material index {} is outside the template",
                        layer.material
                    )
                })?
                .clone();
            film.d_nm = layer.thickness_nm;
            film.optimize = film.optimize && !layer.pinned;
            film.needle = film.needle && !layer.pinned;
            if film.label.is_none() {
                film.label = Some(Arc::from(film.material.as_ref()));
            }
            films.push(film);
        }
        DesignStack::with_films(self.ambient.clone(), self.substrate.clone(), films)
    }
}

/// Context knobs for the fitness evaluator.
#[derive(Clone, Debug)]
pub struct EvaluatorOptions {
    pub clamp_min_nm: f64,
    pub clamp_max_nm: f64,
    pub lm: LmConfig,
    pub thin_layer_policy: ThinLayerPolicy,
}

impl Default for EvaluatorOptions {
    fn default() -> Self {
        Self {
            clamp_min_nm: 2.0,
            clamp_max_nm: 1000.0,
            lm: LmConfig::default(),
            thin_layer_policy: ThinLayerPolicy::Remove,
        }
    }
}

/// A fixed spectral/angular problem plus the navette synthesis context.
///
/// One evaluator is reused for a whole GA run. `evaluate` is pure (no stack
/// mutation); `refine` is the LM finish and mutates the stack in place.
#[derive(Clone)]
pub struct FitnessEvaluator {
    ctx: SmatrixContext,
}

impl FitnessEvaluator {
    pub fn new(
        wavelengths: Vec<f64>,
        angles_deg: &[f64],
        spec: MeritSpec,
        options: EvaluatorOptions,
    ) -> Result<Self, String> {
        if wavelengths.is_empty() {
            return Err("wavelength grid is empty".into());
        }
        for (index, wavelength) in wavelengths.iter().enumerate() {
            if !wavelength.is_finite() || *wavelength <= 0.0 {
                return Err(format!(
                    "wavelength[{index}] = {wavelength} is not finite and positive"
                ));
            }
            if index > 0 && *wavelength <= wavelengths[index - 1] {
                return Err(format!(
                    "wavelength grid must be strictly increasing at index {index}: \
                     {} then {wavelength}",
                    wavelengths[index - 1]
                ));
            }
        }
        if angles_deg.is_empty() {
            return Err("angle grid is empty".into());
        }
        let angles: Vec<Radians> = angles_deg
            .iter()
            .map(|&angle| Radians::from_degrees(angle))
            .collect::<Result<_, _>>()?;

        if !options.clamp_min_nm.is_finite()
            || !options.clamp_max_nm.is_finite()
            || options.clamp_min_nm < 0.0
            || options.clamp_max_nm < options.clamp_min_nm
        {
            return Err(format!(
                "invalid clamp bounds [{}, {}] nm",
                options.clamp_min_nm, options.clamp_max_nm
            ));
        }

        // Refuse unsupported demands before any solve, not after a silent
        // missing-curve penalty.
        SmatrixContext::check_spec(&spec)?;

        let sin_theta = angles.iter().map(|angle| angle.sin()).collect();
        Ok(Self {
            ctx: SmatrixContext {
                wavls: wavelengths,
                sin_theta,
                spec,
                clamp_min_nm: options.clamp_min_nm,
                clamp_max_nm: options.clamp_max_nm,
                thin_layer_policy: options.thin_layer_policy,
                lm: options.lm,
                clamp_accumulator: ClampReport::default(),
                envs: None,
            },
        })
    }

    /// Scalar joint merit of one stack.
    pub fn evaluate(&self, stack: &DesignStack) -> Result<f64, String> {
        self.ctx.evaluate_merit(stack)
    }

    /// Evaluate a generation in input order through navette's A3 batch door.
    ///
    /// A3 lives on navette's `dev_ga` branch and is compiled in by the local
    /// path dependency. The call fans the population out through Rayon while
    /// each solve keeps its own grid parallelism. A published navette release
    /// containing A3 is required before this crate can be published; see the
    /// release-gate note on the navette dependency in `Cargo.toml`.
    pub fn evaluate_batch(&self, stacks: &[DesignStack]) -> Result<Vec<f64>, String> {
        self.ctx.evaluate_many(stacks)
    }

    /// Bounded LM thickness finish, in place. Returns the post-finish merit.
    pub fn refine(&mut self, stack: &mut DesignStack) -> Result<f64, String> {
        self.ctx.refine_thicknesses(stack)
    }

    pub fn simulate(&self, stack: &DesignStack) -> Result<SimCurves, String> {
        self.ctx.simulate(stack)
    }

    pub fn context(&self) -> &SmatrixContext {
        &self.ctx
    }

    pub fn context_mut(&mut self) -> &mut SmatrixContext {
        &mut self.ctx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use navette::smatrix::synthesis::merit::{
        ConstraintKind, CurveId, MeritKey, MeritTarget, SimTransform,
    };

    use crate::decoder::{
        AdjacencyGrammar, Decoder, Genome, LayerGene, Material, MaterialPool, Pins,
    };

    fn constant(name: &str, n: f64, d_nm: f64, num_wavs: usize) -> LayerSpec {
        LayerSpec::constant(name, n, 0.0, d_nm, num_wavs)
    }

    fn fixed_stack(films: Vec<LayerSpec>, num_wavs: usize) -> DesignStack {
        let mut ambient = constant("air", 1.0, 0.0, num_wavs);
        ambient.optimize = false;
        ambient.needle = false;
        let mut substrate = constant("substrate", 1.52, 0.0, num_wavs);
        substrate.optimize = false;
        substrate.needle = false;
        DesignStack::with_films(ambient, substrate, films).unwrap()
    }

    fn zero_reflectance_spec(angle_deg: f64, wavelength: f64) -> MeritSpec {
        let mut spec = MeritSpec::new();
        let key = spec.add_key(MeritKey {
            angle: Radians::from_degrees(angle_deg).unwrap(),
            curve: CurveId::Rs,
        });
        spec.add_target(MeritTarget {
            key_idx: key as u32,
            env_idx: 0,
            wavelengths: vec![wavelength].into(),
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

    fn evaluator(spec: MeritSpec) -> FitnessEvaluator {
        FitnessEvaluator::new(vec![550.0], &[0.0], spec, EvaluatorOptions::default()).unwrap()
    }

    fn reflectance(stack: &DesignStack) -> f64 {
        let ev = evaluator(zero_reflectance_spec(0.0, 550.0));
        let sim = ev.simulate(stack).unwrap();
        sim.curve(CurveId::Rs).unwrap()[0]
    }

    #[test]
    fn quarter_wave_layer_is_an_ideal_ar_at_design_wavelength() {
        let n_substrate = 1.52_f64;
        let n_film = n_substrate.sqrt();
        let d_nm = 550.0 / (4.0 * n_film);
        let stack = fixed_stack(vec![constant("film", n_film, d_nm, 1)], 1);
        let ev = evaluator(zero_reflectance_spec(0.0, 550.0));

        assert!(reflectance(&stack) < 1e-24);
        assert!(ev.evaluate(&stack).unwrap() < 1e-24);
    }

    #[test]
    fn two_layer_v_coat_is_zero_at_design_wavelength() {
        // Two quarter-wave layers satisfy n1^2 * n_substrate / n2^2 = 1.
        let n_substrate = 1.52_f64;
        let n1 = 1.38;
        let n2 = n1 * n_substrate.sqrt();
        let d1 = 550.0 / (4.0 * n1);
        let d2 = 550.0 / (4.0 * n2);
        let stack = fixed_stack(
            vec![constant("n1", n1, d1, 1), constant("n2", n2, d2, 1)],
            1,
        );

        assert!(reflectance(&stack) < 1e-24);
    }

    #[test]
    fn template_and_decoder_assemble_the_same_design() {
        let n_substrate = 1.52_f64;
        let n_film = n_substrate.sqrt();
        let d_nm = 550.0 / (4.0 * n_film);

        let pool = MaterialPool::new(vec![Material::new("film", d_nm, d_nm).unwrap()]).unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let genome = Genome::from(vec![LayerGene::new(0.2, 0.9, 0.9)]);
        let decoded = decoder.decode(&genome).unwrap();
        assert_eq!(decoded.len(), 1);

        let template = StackTemplate::new(
            constant("air", 1.0, 0.0, 1),
            constant("substrate", n_substrate, 0.0, 1),
            vec![constant("film", n_film, 0.0, 1)],
        )
        .unwrap();
        let stack = template.assemble(&decoded).unwrap();

        assert!((stack.films()[0].d_nm - d_nm).abs() < 1e-12);
        assert!(reflectance(&stack) < 1e-24);
    }

    #[test]
    fn pinned_layers_are_not_optimized_or_needle_hosts() {
        let pool = MaterialPool::new(vec![
            Material::new("L", 10.0, 200.0).unwrap(),
            Material::new("H", 20.0, 300.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::new(
            pool,
            Pins::with_edges(Some(0), Some(1)),
            AdjacencyGrammar::all_allowed(2),
        )
        .unwrap();
        let genome = Genome::from(vec![
            LayerGene::new(0.0, 0.5, 0.9),
            LayerGene::new(0.9, 0.5, 0.9),
        ]);
        let decoded = decoder.decode(&genome).unwrap();

        let template = StackTemplate::new(
            constant("air", 1.0, 0.0, 1),
            constant("substrate", 1.52, 0.0, 1),
            vec![constant("L", 1.38, 0.0, 1), constant("H", 2.2, 0.0, 1)],
        )
        .unwrap();
        let stack = template.assemble(&decoded).unwrap();
        assert!(!stack.films()[0].optimize);
        assert!(!stack.films()[0].needle);
        assert!(!stack.films()[1].optimize);
        assert!(!stack.films()[1].needle);
    }

    #[test]
    fn lm_finish_improves_a_detuned_quarter_wave() {
        let n_substrate = 1.52_f64;
        let n_film = n_substrate.sqrt();
        let d_nm = 550.0 / (4.0 * n_film);
        let mut stack = fixed_stack(vec![constant("film", n_film, 0.8 * d_nm, 1)], 1);
        let mut ev = evaluator(zero_reflectance_spec(0.0, 550.0));

        let before = ev.evaluate(&stack).unwrap();
        let after = ev.refine(&mut stack).unwrap();
        let d_after = stack.films()[0].d_nm;

        assert!(after < before, "merit {before} -> {after}");
        assert!(
            (d_after - d_nm).abs() < 2.0,
            "LM finished at {d_after} nm, design {d_nm} nm"
        );
    }

    #[test]
    fn batch_preserves_input_order() {
        let ev = evaluator(zero_reflectance_spec(0.0, 550.0));
        let stacks = [
            fixed_stack(vec![constant("a", 1.2, 100.0, 1)], 1),
            fixed_stack(vec![constant("b", 1.4, 120.0, 1)], 1),
        ];
        let batch = ev.evaluate_batch(&stacks).unwrap();
        let serial: Vec<f64> = stacks.iter().map(|st| ev.evaluate(st).unwrap()).collect();
        assert_eq!(batch, serial);
    }

    #[test]
    fn invalid_inputs_are_refused_before_a_solve() {
        let spec = zero_reflectance_spec(0.0, 550.0);
        assert!(FitnessEvaluator::new(
            Vec::new(),
            &[0.0],
            spec.clone(),
            EvaluatorOptions::default()
        )
        .is_err());
        assert!(
            FitnessEvaluator::new(vec![550.0], &[], spec.clone(), EvaluatorOptions::default())
                .is_err()
        );
        assert!(FitnessEvaluator::new(
            vec![550.0, 500.0],
            &[0.0],
            spec,
            EvaluatorOptions::default()
        )
        .is_err());
    }

    #[test]
    fn template_refuses_a_wrong_nk_grid() {
        let result = StackTemplate::new(
            constant("air", 1.0, 0.0, 2),
            constant("substrate", 1.52, 0.0, 2),
            vec![constant("film", 1.4, 100.0, 1)],
        );
        assert!(result.is_err());
    }
}
