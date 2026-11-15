// SPDX-License-Identifier: LGPL-3.0-or-later
//! G8/G9 — proof harness and performance checks.
//!
//! The kurobako problem exposes a fixed Recipe A design space: one material
//! index and one normalized thickness per layer. A solver's parameters are
//! decoded through the same navette stack the GA uses, and the objective is the
//! scalar merit. The `ComparisonArm` registry names H0–H7 so a study runner can
//! register the comparison set without re-inventing labels.
//!
//! Enabled by the `bench` feature: kurobako pulls `rlua` 0.17.1, which does not
//! compile on rustc 1.100, so this module stays out of the default build.

use kurobako_core::domain::{var, VariableBuilder};
use kurobako_core::problem::{Evaluator, Problem, ProblemSpecBuilder};
use kurobako_core::trial::{Params, Values};
use kurobako_core::{Error, ErrorKind, Result};
use trackable::error::ErrorKindExt;

use crate::bridge::{FitnessEvaluator, StackTemplate};
use crate::decoder::{Decoder, Genome, LayerGene};

/// The eight comparison arms from E09 v2 §2.7 / §5.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonArm {
    /// Classical needle-plus-refinement multi-start.
    H0,
    /// Single-island Recipe A.
    H1,
    /// Ring islands.
    H2,
    /// Heterogeneous islands/tribes.
    H3,
    /// QD/MOME archive.
    H4,
    /// MOO split-off.
    H5,
    /// Screening past TMM.
    H6,
    /// Seeded / proposal-armed Recipe B.
    H7,
}

impl ComparisonArm {
    pub const ALL: [Self; 8] = [
        Self::H0,
        Self::H1,
        Self::H2,
        Self::H3,
        Self::H4,
        Self::H5,
        Self::H6,
        Self::H7,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::H0 => "H0",
            Self::H1 => "H1",
            Self::H2 => "H2",
            Self::H3 => "H3",
            Self::H4 => "H4",
            Self::H5 => "H5",
            Self::H6 => "H6",
            Self::H7 => "H7",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::H0 => "needle-plus-refinement multi-start control",
            Self::H1 => "GA-only single-island recipe",
            Self::H2 => "phased GA -> needle -> LM (S1)",
            Self::H3 => "needle-memetic refinement (S2)",
            Self::H4 => "heterogeneous islands/tribes (S3)",
            Self::H5 => "QD/MOME archive illumination (S4)",
            Self::H6 => "surrogate-assisted screening (S7)",
            Self::H7 => "diffusion/seeded proposal arm (S8)",
        }
    }

    /// The strategy item this arm exists to test, or `None` for the control.
    ///
    /// This mapping is the E09 v2 §2.7 / PAPER v4 §15 comparison set. Keep it
    /// next to [`Self::description`] so a code-first rename cannot silently
    /// re-label a published arm again.
    pub const fn strategy_item(self) -> Option<&'static str> {
        match self {
            Self::H0 => None,
            Self::H1 => Some("GA-only"),
            Self::H2 => Some("S1"),
            Self::H3 => Some("S2"),
            Self::H4 => Some("S3"),
            Self::H5 => Some("S4"),
            Self::H6 => Some("S7"),
            Self::H7 => Some("S8"),
        }
    }
}

/// Fixed parameterization used by the kurobako problem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BenchmarkConfig {
    pub max_layers: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self { max_layers: 12 }
    }
}

/// A Recipe A design space as a kurobako problem.
#[derive(Clone)]
pub struct ReflectanceProblem {
    decoder: Decoder,
    template: StackTemplate,
    evaluator: FitnessEvaluator,
    cfg: BenchmarkConfig,
}

impl ReflectanceProblem {
    pub fn new(
        decoder: Decoder,
        template: StackTemplate,
        evaluator: FitnessEvaluator,
        cfg: BenchmarkConfig,
    ) -> Result<Self> {
        if cfg.max_layers == 0 || decoder.pool().is_empty() {
            return Err(ErrorKind::InvalidInput
                .cause("benchmark needs max_layers >= 1 and a non-empty pool")
                .into());
        }
        Ok(Self {
            decoder,
            template,
            evaluator,
            cfg,
        })
    }

    pub fn specification(&self) -> Result<kurobako_core::problem::ProblemSpec> {
        let mut builder = ProblemSpecBuilder::new("navette-ga-reflectance")
            .attr("algorithm", "recipe-a")
            .attr("layers", &self.cfg.max_layers.to_string());
        let pool_len = self.decoder.pool().len();
        for layer in 0..self.cfg.max_layers {
            builder = builder
                .param(var(&format!("m{layer}")).discrete(0, pool_len as i64 - 1))
                .param(var(&format!("d{layer}")).continuous(0.0, 1.0));
        }
        builder
            .value(var("merit").continuous(0.0, 1.0e12))
            .steps(vec![1])
            .finish()
    }
}

impl Problem for ReflectanceProblem {
    type Evaluator = ReflectanceEvaluator;

    fn create_evaluator(&self, params: Params) -> Result<Self::Evaluator> {
        if params.get().len() != self.cfg.max_layers * 2 {
            return Err(ErrorKind::InvalidInput
                .cause("benchmark parameter count does not match the layer space")
                .into());
        }
        let pool_len = self.decoder.pool().len();
        let genome = Genome::from(
            params
                .get()
                .chunks(2)
                .map(|pair| {
                    let material = ((pair[0].round() as usize).min(pool_len - 1) as f64 + 0.5)
                        / pool_len as f64;
                    LayerGene::new(material, pair[1].clamp(0.0, 1.0), 1.0)
                })
                .collect::<Vec<_>>(),
        );
        let decoded = self
            .decoder
            .decode(&genome)
            .map_err(|error| ErrorKind::InvalidInput.cause(error.to_string()))?;
        let stack = self
            .template
            .assemble(&decoded)
            .map_err(|error| ErrorKind::InvalidInput.cause(error))?;
        Ok(ReflectanceEvaluator {
            evaluator: self.evaluator.clone(),
            stack: Some(stack),
        })
    }
}

/// One-shot evaluator; the problem has a single step.
pub struct ReflectanceEvaluator {
    evaluator: FitnessEvaluator,
    stack: Option<navette::smatrix::synthesis::structure::DesignStack>,
}

impl Evaluator for ReflectanceEvaluator {
    fn evaluate(&mut self, next_step: u64) -> Result<(u64, Values)> {
        if next_step != 1 {
            return Err(ErrorKind::InvalidInput
                .cause("navette benchmark only exposes step 1")
                .into());
        }
        let stack = self
            .stack
            .take()
            .ok_or_else(|| Error::from(ErrorKind::Bug))?;
        let merit = self
            .evaluator
            .evaluate(&stack)
            .map_err(|error| ErrorKind::InvalidInput.cause(error))?;
        if !merit.is_finite() {
            return Err(ErrorKind::UnevaluableParams
                .cause("navette merit is not finite")
                .into());
        }
        Ok((1, Values::new(vec![merit])))
    }
}

/// Register the comparison set in the fixed order H0–H7.
pub fn comparison_suite() -> [ComparisonArm; 8] {
    ComparisonArm::ALL
}

/// Parameter names are stable across runs; a study can pin a layer by name.
pub fn layer_params(layer: usize) -> (String, String) {
    (format!("m{layer}"), format!("d{layer}"))
}

/// A convenience for a caller that only has a material count.
pub fn material_domain(pool_len: usize) -> Result<VariableBuilder> {
    if pool_len == 0 {
        return Err(ErrorKind::InvalidInput
            .cause("material domain needs at least one material")
            .into());
    }
    Ok(var("material").discrete(0, pool_len as i64 - 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use navette::smatrix::angle::Radians;
    use navette::smatrix::synthesis::merit::{
        ConstraintKind, CurveId, MeritKey, MeritSpec, MeritTarget, SimTransform,
    };
    use navette::smatrix::synthesis::structure::LayerSpec;

    use crate::bridge::EvaluatorOptions;
    use crate::decoder::{Material, MaterialPool};

    fn constant(name: &str, n: f64) -> LayerSpec {
        LayerSpec::constant(name, n, 0.0, 60.0, 1)
    }

    fn spec() -> MeritSpec {
        let mut spec = MeritSpec::new();
        let key = spec.add_key(MeritKey {
            angle: Radians::ZERO,
            curve: CurveId::Rs,
        });
        spec.add_target(MeritTarget {
            key_idx: key as u32,
            env_idx: 0,
            wavelengths: vec![550.0].into(),
            kind: ConstraintKind::Exact,
            transform: SimTransform::Linear,
            norm_factor: 1.0,
            normalized_targets: vec![0.0].into(),
            tolerances: vec![0.01].into(),
            band: vec![].into(),
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

    fn problem() -> ReflectanceProblem {
        let pool = MaterialPool::new(vec![
            Material::new("H", 10.0, 200.0).unwrap(),
            Material::new("L", 10.0, 200.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0),
            constant("substrate", 1.52),
            vec![constant("H", 2.2), constant("L", 1.46)],
        )
        .unwrap();
        let evaluator =
            FitnessEvaluator::new(vec![550.0], &[0.0], spec(), EvaluatorOptions::default())
                .unwrap();
        ReflectanceProblem::new(
            decoder,
            template,
            evaluator,
            BenchmarkConfig { max_layers: 2 },
        )
        .unwrap()
    }

    #[test]
    fn comparison_set_is_h0_through_h7_in_order() {
        assert_eq!(
            comparison_suite().map(ComparisonArm::label),
            ["H0", "H1", "H2", "H3", "H4", "H5", "H6", "H7"]
        );
    }

    #[test]
    fn comparison_arms_follow_the_e09_comparison_set() {
        assert_eq!(comparison_suite()[0].strategy_item(), None);
        assert_eq!(
            comparison_suite()
                .iter()
                .skip(1)
                .map(|arm| arm.strategy_item().unwrap())
                .collect::<Vec<_>>(),
            ["GA-only", "S1", "S2", "S3", "S4", "S7", "S8"]
        );
        assert!(ComparisonArm::H2.description().contains("phased GA"));
        assert!(ComparisonArm::H3.description().contains("needle-memetic"));
        assert!(ComparisonArm::H5.description().contains("MOME"));
    }

    #[test]
    fn kurobako_problem_has_two_parameters_per_layer() {
        let specification = problem().specification().unwrap();
        assert_eq!(specification.params_domain.len(), 4);
        assert_eq!(specification.values_domain.len(), 1);
        assert_eq!(layer_params(0).0, "m0");
    }

    #[test]
    fn kurobako_evaluator_returns_one_finite_value() {
        let problem = problem();
        let mut evaluator = problem
            .create_evaluator(Params::new(vec![0.0, 0.5, 1.0, 0.5]))
            .unwrap();
        let (step, values) = evaluator.evaluate(1).unwrap();
        assert_eq!(step, 1);
        let values = values.into_vec();
        assert_eq!(values.len(), 1);
        assert!(values[0].is_finite());
    }
}
