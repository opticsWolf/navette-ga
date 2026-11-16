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

use crate::bridge::{FitnessEvaluator, MemeticOperator, StackTemplate};
use crate::decoder::{Decoder, Genome, LayerGene};
use crate::engines::{
    finish_recipe_a, run_h0_control, run_h0_control_seeded, run_recipe_a, run_recipe_a_illuminated,
    run_recipe_a_illuminated_seeded, run_recipe_a_memetic, run_recipe_a_memetic_seeded,
    run_recipe_a_seeded, run_recipe_a_surrogate, run_recipe_a_surrogate_seeded,
    run_recipe_a_tribes, run_recipe_a_tribes_seeded, run_recipe_a_vl_proposed,
    run_recipe_a_vl_seeded, H0Config, HeterogeneousTribeConfig, IslandRecipeConfig, IslandStrategy,
    MemeticSchedule, ProposalConfig, RecipeAConfig, RecipeAResult, SurrogateConfig, TerminalReport,
    TribeConstraint, VlRecipeAConfig,
};
use crate::qd::ArchiveConfig;
use crate::study::{holm_bonferroni, mann_whitney_u, RankSumReport};

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

/// Configuration for one H0–H7 comparison-arm dispatch.
///
/// The arm owns the driver choice; this object supplies the shared problem
/// inputs and the arm-specific policy. H2/H3 require an operator, H7 requires
/// a non-empty seed bank, and every other arm is runnable with the defaults.
#[derive(Clone, Debug)]
pub struct ComparisonOptions {
    pub recipe: RecipeAConfig,
    pub h0: H0Config,
    pub islands: IslandRecipeConfig,
    pub vl: VlRecipeAConfig,
    pub memetic: MemeticSchedule,
    pub surrogate: SurrogateConfig,
    pub archive: ArchiveConfig,
    pub operator: Option<MemeticOperator>,
    /// Initial fixed-width genomes for H0–H6. Each is padded with inactive
    /// slots up to the arm's width; empty keeps random initialization. H0
    /// uses the first genome as one seeded start and the remaining starts at
    /// random; H7 uses `seeds` instead.
    pub initial_genomes: Vec<Genome>,
    pub seeds: Vec<Genome>,
    pub seed_fraction: f64,
    /// Optional Gaussian proposal expansion for H7. `None` uses the raw seed
    /// bank (the mined-seed fallback).
    pub proposal: Option<ProposalConfig>,
    /// Run the H2-style terminal LM/needle finish on H3–H6 as well.
    /// H2 always finishes and H7 uses `finish_h7`, so this is the terminal
    /// ablation switch for the other fixed-width arms.
    pub finish_h3_h6: bool,
    /// Run the H2-style terminal LM/needle finish on H7 as well.
    pub finish_h7: bool,
    /// Insertion cap passed to the H2/H7 terminal finish.
    pub terminal_insertions: usize,
}

impl Default for ComparisonOptions {
    fn default() -> Self {
        Self {
            recipe: RecipeAConfig::default(),
            h0: H0Config::default(),
            islands: IslandRecipeConfig::default(),
            vl: VlRecipeAConfig::default(),
            memetic: MemeticSchedule::default(),
            surrogate: SurrogateConfig::default(),
            archive: ArchiveConfig::default(),
            operator: None,
            initial_genomes: Vec::new(),
            seeds: Vec::new(),
            seed_fraction: 1.0,
            proposal: None,
            finish_h3_h6: false,
            finish_h7: false,
            terminal_insertions: 20,
        }
    }
}

/// One completed comparison arm.
#[derive(Clone, Debug)]
pub struct ComparisonRun {
    pub arm: ComparisonArm,
    pub result: RecipeAResult,
    /// Solver-evaluation counter after the arm, including LM evals.
    pub evaluations: usize,
    /// Terminal finish report when the arm ran one. H2 always reports it;
    /// H3–H6 report it when `finish_h3_h6` is set; H7 reports it when
    /// `finish_h7` is set.
    pub terminal: Option<TerminalReport>,
}

/// Dispatch one E09 H0–H7 arm through the engine that implements it.
///
/// H2 is the phased GA -> needle -> LM finish; H3 is the mid-loop memetic
/// schedule; H4 is the ring with alternating island mutation regimes; H5 is
/// the illuminated MOME run; H6 is the linear surrogate screen; H7 is the
/// variable-length seeded proposal arm. Arm-specific missing inputs are
/// reported as errors instead of being silently replaced by another arm.
pub fn run_comparison_arm(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &mut FitnessEvaluator,
    arm: ComparisonArm,
    options: &ComparisonOptions,
) -> Result<ComparisonRun, String> {
    evaluator.reset_evaluations();
    let fixed_terminal = options.finish_h3_h6
        && matches!(
            arm,
            ComparisonArm::H3 | ComparisonArm::H4 | ComparisonArm::H5 | ComparisonArm::H6
        );
    if (arm == ComparisonArm::H2
        || fixed_terminal
        || (arm == ComparisonArm::H7 && options.finish_h7))
        && options.operator.is_none()
    {
        return Err(format!(
            "{} terminal finish needs a memetic operator",
            arm.label()
        ));
    }

    let result = match arm {
        ComparisonArm::H0 => {
            if options.initial_genomes.is_empty() {
                run_h0_control(
                    decoder,
                    template,
                    evaluator,
                    &options.h0,
                    options.operator.as_ref(),
                )?
            } else {
                run_h0_control_seeded(
                    decoder,
                    template,
                    evaluator,
                    &options.h0,
                    options.operator.as_ref(),
                    &options.initial_genomes,
                )?
            }
        }
        ComparisonArm::H1 => {
            if options.initial_genomes.is_empty() {
                run_recipe_a(decoder, template, evaluator, &options.recipe)?
            } else {
                run_recipe_a_seeded(
                    decoder,
                    template,
                    evaluator,
                    &options.recipe,
                    &options.initial_genomes,
                )?
            }
        }
        ComparisonArm::H2 => {
            if options.initial_genomes.is_empty() {
                run_recipe_a(decoder, template, evaluator, &options.recipe)?
            } else {
                run_recipe_a_seeded(
                    decoder,
                    template,
                    evaluator,
                    &options.recipe,
                    &options.initial_genomes,
                )?
            }
        }
        ComparisonArm::H3 => {
            let operator = options
                .operator
                .as_ref()
                .ok_or_else(|| "H3 needs a memetic operator".to_string())?;
            if options.initial_genomes.is_empty() {
                run_recipe_a_memetic(
                    decoder,
                    template,
                    evaluator,
                    &options.recipe,
                    operator,
                    &options.memetic,
                )?
                .best
            } else {
                run_recipe_a_memetic_seeded(
                    decoder,
                    template,
                    evaluator,
                    &options.recipe,
                    operator,
                    &options.memetic,
                    &options.initial_genomes,
                )?
                .best
            }
        }
        ComparisonArm::H4 => {
            let count = options.islands.islands;
            let strategies = (0..count)
                .map(|index| IslandStrategy {
                    mutation_sigma: if index.is_multiple_of(2) { 0.10 } else { 0.20 },
                    ..IslandStrategy::default()
                })
                .collect::<Vec<_>>();
            let constraints = vec![TribeConstraint::default(); count];
            let tribes = HeterogeneousTribeConfig {
                islands: options.islands.clone(),
                strategies,
                constraints,
            };
            if options.initial_genomes.is_empty() {
                run_recipe_a_tribes(decoder, template, evaluator, &options.recipe, &tribes)?.best
            } else {
                run_recipe_a_tribes_seeded(
                    decoder,
                    template,
                    evaluator,
                    &options.recipe,
                    &tribes,
                    &options.initial_genomes,
                )?
                .best
            }
        }
        ComparisonArm::H5 => {
            if options.initial_genomes.is_empty() {
                run_recipe_a_illuminated(
                    decoder,
                    template,
                    evaluator,
                    &options.recipe,
                    options.archive,
                )?
                .best
            } else {
                run_recipe_a_illuminated_seeded(
                    decoder,
                    template,
                    evaluator,
                    &options.recipe,
                    options.archive,
                    &options.initial_genomes,
                )?
                .best
            }
        }
        ComparisonArm::H6 => {
            if options.initial_genomes.is_empty() {
                run_recipe_a_surrogate(
                    decoder,
                    template,
                    evaluator,
                    &options.recipe,
                    options.surrogate,
                )?
                .best
            } else {
                run_recipe_a_surrogate_seeded(
                    decoder,
                    template,
                    evaluator,
                    &options.recipe,
                    options.surrogate,
                    &options.initial_genomes,
                )?
                .best
            }
        }
        ComparisonArm::H7 => {
            if options.seeds.is_empty() {
                return Err("H7 needs a non-empty mined-seed bank".into());
            }
            if let Some(proposal) = options.proposal {
                run_recipe_a_vl_proposed(
                    decoder,
                    template,
                    evaluator,
                    &options.vl,
                    &options.seeds,
                    options.seed_fraction,
                    proposal,
                )?
            } else {
                run_recipe_a_vl_seeded(
                    decoder,
                    template,
                    evaluator,
                    &options.vl,
                    &options.seeds,
                    options.seed_fraction,
                )?
            }
        }
    };

    let (result, terminal) = if arm == ComparisonArm::H2
        || fixed_terminal
        || (arm == ComparisonArm::H7 && options.finish_h7)
    {
        let operator = options
            .operator
            .as_ref()
            .ok_or_else(|| format!("{} terminal finish needs a memetic operator", arm.label()))?;
        let (finished, report) = finish_recipe_a(
            decoder,
            template,
            evaluator,
            result,
            operator,
            options.terminal_insertions,
        )?;
        (finished, Some(report))
    } else {
        (result, None)
    };
    let evaluations = evaluator.evaluations();
    Ok(ComparisonRun {
        arm,
        result,
        evaluations,
        terminal,
    })
}

/// All fixed-seed runs for one comparison arm.
#[derive(Clone, Debug)]
pub struct CampaignArm {
    pub arm: ComparisonArm,
    pub runs: Vec<ComparisonRun>,
}

impl CampaignArm {
    pub fn best_merit(&self) -> Option<f64> {
        self.runs
            .iter()
            .map(|run| run.result.merit)
            .min_by(f64::total_cmp)
    }

    pub fn median_merit(&self) -> Option<f64> {
        let mut values = self
            .runs
            .iter()
            .map(|run| run.result.merit)
            .collect::<Vec<_>>();
        values.sort_by(f64::total_cmp);
        values.get(values.len() / 2).copied()
    }

    pub fn median_evaluations(&self) -> Option<f64> {
        let mut values = self
            .runs
            .iter()
            .map(|run| run.evaluations as f64)
            .collect::<Vec<_>>();
        values.sort_by(f64::total_cmp);
        values.get(values.len() / 2).copied()
    }
}

/// One pairwise rank-sum comparison inside a campaign family.
#[derive(Clone, Debug)]
pub struct ArmComparison {
    pub first: ComparisonArm,
    pub second: ComparisonArm,
    pub report: RankSumReport,
    /// Holm–Bonferroni adjustment over the whole family of pairs.
    pub adjusted_p: f64,
}

/// Pairwise Mann–Whitney comparisons with a Holm adjustment over all pairs.
///
/// The family is every unordered pair of arms in `campaign`, so `m =
/// k*(k-1)/2`. The underlying rank-sum test uses the exact permutation
/// distribution only for small tie-free samples and the tie-corrected normal
/// approximation otherwise; the returned report says which was used.
pub fn family_rank_sum(campaign: &[CampaignArm]) -> Result<Vec<ArmComparison>, String> {
    if campaign.len() < 2 {
        return Err("family rank-sum needs at least two arms".into());
    }
    let mut pairs = Vec::new();
    for first in 0..campaign.len() {
        for second in (first + 1)..campaign.len() {
            let a = campaign[first]
                .runs
                .iter()
                .map(|run| run.result.merit)
                .collect::<Vec<_>>();
            let b = campaign[second]
                .runs
                .iter()
                .map(|run| run.result.merit)
                .collect::<Vec<_>>();
            pairs.push((
                campaign[first].arm,
                campaign[second].arm,
                mann_whitney_u(&a, &b)?,
            ));
        }
    }
    let p_values = pairs
        .iter()
        .map(|(_, _, report)| report.p_value)
        .collect::<Vec<_>>();
    let adjusted = holm_bonferroni(&p_values)?;
    Ok(pairs
        .into_iter()
        .zip(adjusted)
        .map(|((first, second, report), adjusted_p)| ArmComparison {
            first,
            second,
            report,
            adjusted_p,
        })
        .collect())
}

/// Fixed-target expected-running-time summary for one arm.
///
/// `ert` is the censored estimate `sum(evaluations over all runs) / successes`:
/// a run that does not reach the target still contributes its full observed
/// evaluation count. This is an evaluation-count proxy, not wall time, and it
/// is only comparable when the arms use similar per-evaluation costs.
#[derive(Clone, Debug)]
pub struct ErtArm {
    pub arm: ComparisonArm,
    pub trials: usize,
    pub successes: usize,
    pub total_evaluations: usize,
    pub ert: Option<f64>,
}

impl ErtArm {
    pub fn success_rate(&self) -> f64 {
        if self.trials == 0 {
            0.0
        } else {
            self.successes as f64 / self.trials as f64
        }
    }
}

/// Fixed-target ERT table for a campaign.
#[derive(Clone, Debug)]
pub struct ErtReport {
    pub target_merit: f64,
    pub arms: Vec<ErtArm>,
}

/// Compute fixed-target ERT from a campaign.
///
/// A run succeeds when its final merit is `<= target_merit` (minimization).
/// Arms with no success report `None`, not an infinite or fabricated value.
pub fn expected_running_time(
    campaign: &[CampaignArm],
    target_merit: f64,
) -> Result<ErtReport, String> {
    if !target_merit.is_finite() {
        return Err("ERT target merit must be finite".into());
    }
    let arms = campaign
        .iter()
        .map(|entry| {
            let successes = entry
                .runs
                .iter()
                .filter(|run| run.result.merit <= target_merit)
                .count();
            let total_evaluations = entry.runs.iter().map(|run| run.evaluations).sum();
            let ert = (successes > 0).then_some(total_evaluations as f64 / successes as f64);
            ErtArm {
                arm: entry.arm,
                trials: entry.runs.len(),
                successes,
                total_evaluations,
                ert,
            }
        })
        .collect();
    Ok(ErtReport { target_merit, arms })
}

/// Run a fixed set of arms over a fixed set of seeds.
///
/// The seed is written into every arm configuration that has a seed field
/// (`recipe`, `h0`, and `vl`); H6 and H7 therefore inherit a reproducible
/// initializer from the same seed. This is the campaign loop, not an ERT
/// report: a caller still needs a target merit and a success rule to compute
/// expected running time.
pub fn run_comparison_campaign(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &mut FitnessEvaluator,
    arms: &[ComparisonArm],
    seeds: &[u64],
    options: &ComparisonOptions,
) -> Result<Vec<CampaignArm>, String> {
    if arms.is_empty() {
        return Err("comparison campaign needs at least one arm".into());
    }
    if seeds.is_empty() {
        return Err("comparison campaign needs at least one seed".into());
    }
    let mut results = Vec::with_capacity(arms.len());
    for &arm in arms {
        let mut runs = Vec::with_capacity(seeds.len());
        for &seed in seeds {
            let mut seeded = options.clone();
            seeded.recipe.seed = Some(seed);
            seeded.h0.seed = Some(seed);
            seeded.vl.seed = Some(seed);
            runs.push(run_comparison_arm(
                decoder, template, evaluator, arm, &seeded,
            )?);
        }
        results.push(CampaignArm { arm, runs });
    }
    Ok(results)
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
        // Keep the evaluator's LM clamp inside the decoder's material bounds;
        // otherwise a terminal LM can return a thickness the A4 check rejects.
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            spec(),
            EvaluatorOptions {
                clamp_min_nm: 10.0,
                clamp_max_nm: 200.0,
                ..EvaluatorOptions::default()
            },
        )
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

    fn comparison_options(problem: &ReflectanceProblem) -> ComparisonOptions {
        let wavelengths = vec![550.0];
        let spectral = navette::smatrix::synthesis::pipeline::SpectralInputs::from_spec(
            &spec(),
            &[Radians::ZERO],
            &wavelengths,
        )
        .unwrap();
        let operator = MemeticOperator::new(
            spectral,
            problem.template.needle_pool().unwrap(),
            navette::smatrix::synthesis::cycle::NeedleCycleConfig {
                max_needles: 2,
                scan_step_nm: 10.0,
                boundary_needles: false,
                ..Default::default()
            },
        );
        ComparisonOptions {
            recipe: RecipeAConfig {
                population_size: 4,
                generations: 2,
                max_layers: 2,
                mutation_sigma: 0.2,
                seed: Some(7),
            },
            h0: H0Config {
                starts: 2,
                max_layers: 2,
                mutation_sigma: 0.2,
                needle_steps: 1,
                seed: Some(11),
            },
            islands: IslandRecipeConfig {
                islands: 2,
                migration_interval: 1,
                migration_count: 1,
            },
            vl: VlRecipeAConfig {
                population_size: 4,
                generations: 2,
                min_layers: 1,
                max_layers: 2,
                mutation_sigma: 0.2,
                length_penalty: None,
                seed: Some(13),
            },
            memetic: MemeticSchedule {
                interval: 1,
                top: 1,
                max_insertions: 1,
            },
            surrogate: SurrogateConfig {
                warmup: 2,
                sample_every: 2,
                refit_every: 2,
            },
            archive: crate::qd::ArchiveConfig {
                depth_bins: 2,
                thickness_bins: 2,
                max_layers: 2.0,
                max_thickness_nm: 1_000.0,
                max_front: 2,
            },
            operator: Some(operator),
            initial_genomes: Vec::new(),
            seeds: vec![Genome::from(vec![LayerGene::new(0.25, 0.5, 1.0)])],
            seed_fraction: 0.5,
            proposal: Some(ProposalConfig {
                noise_sigma: 0.1,
                proposals_per_seed: 1,
            }),
            finish_h3_h6: false,
            finish_h7: false,
            terminal_insertions: 4,
        }
    }

    #[test]
    fn every_comparison_arm_dispatches_to_its_engine() {
        for arm in ComparisonArm::ALL {
            let problem = problem();
            let mut evaluator = problem.evaluator.clone();
            let options = comparison_options(&problem);
            let run = run_comparison_arm(
                &problem.decoder,
                &problem.template,
                &mut evaluator,
                arm,
                &options,
            )
            .unwrap_or_else(|error| panic!("{} dispatch failed: {error}", arm.label()));
            assert_eq!(run.arm, arm);
            assert!(run.result.merit.is_finite(), "{} merit", arm.label());
            assert!(run.evaluations > 0, "{} evaluations", arm.label());
        }
    }

    #[test]
    fn seeded_fixed_width_arms_accept_a_two_layer_start() {
        let seed = Genome::from(vec![LayerGene::new(0.25, 0.5, 1.0)]);
        for arm in [
            ComparisonArm::H0,
            ComparisonArm::H3,
            ComparisonArm::H4,
            ComparisonArm::H5,
            ComparisonArm::H6,
        ] {
            let problem = problem();
            let mut evaluator = problem.evaluator.clone();
            let mut options = comparison_options(&problem);
            options.initial_genomes = vec![seed.clone()];
            let run = run_comparison_arm(
                &problem.decoder,
                &problem.template,
                &mut evaluator,
                arm,
                &options,
            )
            .unwrap_or_else(|error| panic!("{} seeded dispatch failed: {error}", arm.label()));
            assert_eq!(run.arm, arm);
            assert!(run.result.merit.is_finite(), "{} merit", arm.label());
        }
    }

    #[test]
    fn comparison_dispatch_reports_missing_arm_inputs() {
        let problem = problem();
        let mut evaluator = problem.evaluator.clone();
        let mut options = comparison_options(&problem);
        options.seeds.clear();
        let error = run_comparison_arm(
            &problem.decoder,
            &problem.template,
            &mut evaluator,
            ComparisonArm::H7,
            &options,
        )
        .unwrap_err();
        assert!(error.contains("H7"));

        options.operator = None;
        let error = run_comparison_arm(
            &problem.decoder,
            &problem.template,
            &mut evaluator,
            ComparisonArm::H2,
            &options,
        )
        .unwrap_err();
        assert!(error.contains("H2"));
    }

    #[test]
    fn comparison_campaign_runs_each_seed_for_each_arm() {
        let problem = problem();
        let mut evaluator = problem.evaluator.clone();
        let options = comparison_options(&problem);
        let arms = [ComparisonArm::H0, ComparisonArm::H1, ComparisonArm::H6];
        let campaign = run_comparison_campaign(
            &problem.decoder,
            &problem.template,
            &mut evaluator,
            &arms,
            &[101, 202],
            &options,
        )
        .unwrap();
        assert_eq!(campaign.len(), arms.len());
        for entry in &campaign {
            assert_eq!(entry.runs.len(), 2);
            assert!(entry.best_merit().unwrap().is_finite());
            assert!(entry.median_merit().unwrap().is_finite());
            assert!(entry.median_evaluations().unwrap() > 0.0);
        }
        assert!(run_comparison_campaign(
            &problem.decoder,
            &problem.template,
            &mut evaluator,
            &arms,
            &[],
            &options,
        )
        .is_err());
    }

    #[test]
    fn ert_report_counts_fixed_target_successes() {
        let problem = problem();
        let mut evaluator = problem.evaluator.clone();
        let options = comparison_options(&problem);
        let campaign = run_comparison_campaign(
            &problem.decoder,
            &problem.template,
            &mut evaluator,
            &[ComparisonArm::H0, ComparisonArm::H1],
            &[101, 202],
            &options,
        )
        .unwrap();

        let all = expected_running_time(&campaign, f64::MAX).unwrap();
        assert_eq!(all.target_merit, f64::MAX);
        for arm in &all.arms {
            assert_eq!(arm.trials, 2);
            assert_eq!(arm.successes, 2);
            assert_eq!(arm.success_rate(), 1.0);
            assert!(arm.ert.unwrap() > 0.0);
        }

        let none = expected_running_time(&campaign, -1.0).unwrap();
        for arm in &none.arms {
            assert_eq!(arm.successes, 0);
            assert!(arm.ert.is_none());
            assert_eq!(arm.success_rate(), 0.0);
        }

        assert!(expected_running_time(&campaign, f64::NAN).is_err());
    }

    #[test]
    fn h7_terminal_finish_uses_the_shared_operator() {
        let problem = problem();
        let mut evaluator = problem.evaluator.clone();
        let mut options = comparison_options(&problem);
        options.finish_h7 = true;
        let run = run_comparison_arm(
            &problem.decoder,
            &problem.template,
            &mut evaluator,
            ComparisonArm::H7,
            &options,
        )
        .unwrap();
        assert!(run.result.merit.is_finite());
        assert!(run.evaluations > 0);
        assert!(run.terminal.is_some());

        options.operator = None;
        let error = run_comparison_arm(
            &problem.decoder,
            &problem.template,
            &mut evaluator,
            ComparisonArm::H7,
            &options,
        )
        .unwrap_err();
        assert!(error.contains("terminal finish"));
    }

    #[test]
    fn fixed_width_arms_report_terminal_certificate_when_enabled() {
        for arm in [
            ComparisonArm::H3,
            ComparisonArm::H4,
            ComparisonArm::H5,
            ComparisonArm::H6,
        ] {
            let problem = problem();
            let mut evaluator = problem.evaluator.clone();
            let mut options = comparison_options(&problem);
            options.finish_h3_h6 = true;
            let run = run_comparison_arm(
                &problem.decoder,
                &problem.template,
                &mut evaluator,
                arm,
                &options,
            )
            .unwrap_or_else(|error| panic!("{} terminal dispatch failed: {error}", arm.label()));
            let report = run
                .terminal
                .unwrap_or_else(|| panic!("{} did not report a terminal finish", arm.label()));
            assert_eq!(report.actual_improving_after, 0, "{}", arm.label());
            assert!(report.final_merit.is_finite(), "{}", arm.label());
        }
    }

    #[test]
    fn family_rank_sum_adjusts_every_pair() {
        let problem = problem();
        let mut evaluator = problem.evaluator.clone();
        let options = comparison_options(&problem);
        let campaign = run_comparison_campaign(
            &problem.decoder,
            &problem.template,
            &mut evaluator,
            &[ComparisonArm::H0, ComparisonArm::H1, ComparisonArm::H6],
            &[101, 202],
            &options,
        )
        .unwrap();
        let family = family_rank_sum(&campaign).unwrap();
        assert_eq!(family.len(), 3);
        for pair in &family {
            assert_eq!(pair.report.n_a, 2);
            assert_eq!(pair.report.n_b, 2);
            assert!((0.0..=1.0).contains(&pair.adjusted_p));
            assert!(pair.adjusted_p >= pair.report.p_value);
        }
        assert!(family_rank_sum(&campaign[..1]).is_err());
    }
}
