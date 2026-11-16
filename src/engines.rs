// SPDX-License-Identifier: LGPL-3.0-or-later
//! S1–S3 — evolutionary engines and assembly backbone.
//!
//! Starts with the single-island Recipe A path on `genetic_algorithms` 3.0:
//! a fixed-width chromosome of three random keys per layer, decoded by
//! [`crate::decoder`] and evaluated by [`crate::bridge`]. Presence keys make
//! the decoded stack variable-length even though the engine's DNA is fixed
//! width. The Phase 2 island driver runs a ring with best-replace-worst-if-better
//! migration; true variable-length insertion/deletion mutation follows.
//!
//! The `genevo` 0.7 Simulation/Operator/Population traits are adopted when
//! the E09 v2 §1.1 copy gate opens. Both copies are Apache-2.0 /
//! MIT-Apache and are recorded in `CREDIT.md` at the moment a file moves in.

use std::borrow::Cow;
use std::sync::{Arc, Mutex};

use genetic_algorithms::chromosomes::ChromosomeLength;
use genetic_algorithms::configuration::ProblemSolving;
use genetic_algorithms::fitness::FitnessFnWrapper;
use genetic_algorithms::ga::Ga;
use genetic_algorithms::operations::{
    AlignmentStrategy, Crossover, GaussianParams, Mutation, Selection, Survivor,
};
use genetic_algorithms::population::Population;
use genetic_algorithms::rng::{make_rng, set_seed};
use genetic_algorithms::traits::{
    ChromosomeT, ConfigurationT, CrossoverConfig, GeneT, LinearChromosome, MutationConfig,
    OperatorCompat, RealValuedMutation, SelectionConfig, StoppingConfig, SurvivorConfig,
};
use rand::Rng;

use crate::bridge::{FitnessEvaluator, MemeticOperator, StackTemplate};
use crate::decoder::{AdjacencyGrammar, DecodedStack, Decoder, Genome, LayerGene};
use crate::qd::{ArchiveConfig, MomeArchive};

/// One gene: the three random keys of one layer.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct KeyGene {
    /// Gene identity required by `genetic_algorithms`.
    pub id: i32,
    pub material: f64,
    pub thickness: f64,
    pub presence: f64,
}

impl KeyGene {
    pub fn new(material: f64, thickness: f64, presence: f64) -> Self {
        Self {
            id: 0,
            material,
            thickness,
            presence,
        }
    }

    pub fn random(rng: &mut impl Rng) -> Self {
        Self::new(rng.random(), rng.random(), rng.random())
    }
}

impl GeneT for KeyGene {
    fn id(&self) -> i32 {
        self.id
    }

    fn set_id(&mut self, id: i32) -> &mut Self {
        self.id = id;
        self
    }
}

/// Flat chromosome over [`KeyGene`]s.
#[derive(Clone, Debug)]
pub struct LayerChromosome {
    dna: Vec<KeyGene>,
    fitness: f64,
    age: usize,
    fitness_fn: FitnessFnWrapper<KeyGene>,
}

impl Default for LayerChromosome {
    fn default() -> Self {
        Self {
            dna: Vec::new(),
            fitness: f64::NAN,
            age: 0,
            fitness_fn: FitnessFnWrapper::default(),
        }
    }
}

impl LayerChromosome {
    pub fn from_dna(dna: Vec<KeyGene>) -> Self {
        Self {
            dna,
            ..Self::default()
        }
    }

    pub fn to_genome(&self) -> Genome {
        Genome::from(
            self.dna
                .iter()
                .map(|gene| LayerGene::new(gene.material, gene.thickness, gene.presence))
                .collect::<Vec<_>>(),
        )
    }
}

impl ChromosomeT for LayerChromosome {
    type Gene = KeyGene;

    fn calculate_fitness(&mut self) {
        self.fitness = self.fitness_fn.call(&self.dna);
    }

    fn fitness(&self) -> f64 {
        self.fitness
    }

    fn set_fitness(&mut self, fitness: f64) -> &mut Self {
        self.fitness = fitness;
        self
    }

    fn set_age(&mut self, age: usize) -> &mut Self {
        self.age = age;
        self
    }

    fn age(&self) -> usize {
        self.age
    }
}

impl LinearChromosome for LayerChromosome {
    fn dna(&self) -> &[Self::Gene] {
        &self.dna
    }

    fn dna_mut(&mut self) -> &mut [Self::Gene] {
        &mut self.dna
    }

    fn set_dna<'a>(&mut self, dna: Cow<'a, [Self::Gene]>) -> &mut Self {
        self.dna = dna.into_owned();
        self
    }

    fn set_fitness_fn<F>(&mut self, fitness_fn: F) -> &mut Self
    where
        F: Fn(&[Self::Gene]) -> f64 + Send + Sync + 'static,
    {
        self.fitness_fn = FitnessFnWrapper::new(fitness_fn);
        self
    }
}

impl OperatorCompat for LayerChromosome {}

impl RealValuedMutation for LayerChromosome {}

impl genetic_algorithms::operations::mutation::ValueMutable for LayerChromosome {
    fn gaussian_mutate(&mut self, sigma: f64) {
        let mut rng = make_rng();
        for gene in &mut self.dna {
            gene.material = wrap01(gene.material + gaussian(&mut rng) * sigma);
            gene.thickness = wrap01(gene.thickness + gaussian(&mut rng) * sigma);
            gene.presence = wrap01(gene.presence + gaussian(&mut rng) * sigma);
        }
    }

    fn value_mutate(&mut self) {
        self.gaussian_mutate(0.1);
    }

    fn creep_mutate(&mut self, step: f64) {
        let mut rng = make_rng();
        for gene in &mut self.dna {
            let delta = rng.random_range(-step..=step);
            gene.material = wrap01(gene.material + delta);
            gene.thickness = wrap01(gene.thickness + delta);
            gene.presence = wrap01(gene.presence + delta);
        }
    }
}

fn wrap01(x: f64) -> f64 {
    if x.is_finite() {
        x.rem_euclid(1.0)
    } else {
        0.0
    }
}

/// Box-Muller standard normal, one sample.
fn gaussian(rng: &mut impl Rng) -> f64 {
    let u1: f64 = rng.random::<f64>().max(1e-12);
    let u2: f64 = rng.random();
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

/// Recipe A configuration: one island, fixed-width keys, presence-coded VL.
#[derive(Clone, Debug)]
pub struct RecipeAConfig {
    pub population_size: usize,
    /// Total generations including the initial population evaluation.
    pub generations: usize,
    /// DNA width; decoded layer count is at most this.
    pub max_layers: usize,
    pub mutation_sigma: f64,
    pub seed: Option<u64>,
}

impl Default for RecipeAConfig {
    fn default() -> Self {
        Self {
            population_size: 32,
            generations: 20,
            max_layers: 12,
            mutation_sigma: 0.15,
            seed: None,
        }
    }
}

/// Optional mid-loop needle schedule for Recipe A (G4/S2).
///
/// Every `interval` generations, the `top` best chromosomes are decoded,
/// assembled, passed through a [`MemeticOperator`], and written back into the
/// same fixed-width DNA when the refined stack is better and still satisfies
/// the decoder's A4 contract. The terminal finish remains a separate call; a
/// schedule does not replace the final scan certificate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemeticSchedule {
    /// Apply one memetic pass every this many generations.
    pub interval: usize,
    /// Number of best chromosomes refined in each pass.
    pub top: usize,
    /// Maximum needle insertions per chromosome per pass.
    pub max_insertions: usize,
}

impl Default for MemeticSchedule {
    fn default() -> Self {
        // C00-class measured sweet spot: a 20-generation cadence with two
        // chromosomes and one insertion each cost about 6 % wall time and
        // moved the average reflectance from 3.36 % to 1.17 %. The same
        // schedule at 10/8/2 cost 63 % and was worse on this seed; cadence
        // and top-m are the budget knobs, not "more operator" by default.
        Self {
            interval: 20,
            top: 2,
            max_insertions: 1,
        }
    }
}

/// Phase 2 island policy for Recipe A.
///
/// The loop runs every island independently, then a ring migration every
/// `migration_interval` generations: each island sends its best to its
/// successor and replaces the successor's worst only when the migrant is
/// strictly better. `migration_count` is capped at 5% of the population (at
/// least one), matching E09 v2 §2.8.1. No extra legality repair is needed on
/// this fixed-width chromosome: every island shares the decoder grammar and
/// pins, so a legal migrant stays legal in the destination.
#[derive(Clone, Debug)]
pub struct IslandRecipeConfig {
    pub islands: usize,
    pub migration_interval: usize,
    pub migration_count: usize,
}

impl Default for IslandRecipeConfig {
    fn default() -> Self {
        Self {
            islands: 4,
            migration_interval: 10,
            migration_count: 1,
        }
    }
}

/// H0 control arm: needle-plus-refinement multi-start.
///
/// This is the classical competitor E09 v2 §2.7 schedules first. Each start is
/// a random stack, a bounded LM refinement, an optional memetic needle step,
/// and a second refinement. The best start wins. It shares the decoder and
/// evaluator with Recipe A, so a control victory is measured on the same
/// physics and the same compute unit, not on a different problem.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct H0Config {
    pub starts: usize,
    pub max_layers: usize,
    pub mutation_sigma: f64,
    pub needle_steps: usize,
    pub seed: Option<u64>,
}

impl Default for H0Config {
    fn default() -> Self {
        Self {
            starts: 8,
            max_layers: 12,
            mutation_sigma: 0.2,
            needle_steps: 1,
            seed: None,
        }
    }
}

/// I1 experiment: cross-island multi-parent crossover.
///
/// This is deliberately separate from the default ring migration policy. At
/// each migration event, every destination receives one child assembled from
/// `parents` distinct source islands; the child replaces the destination's
/// worst only when its merit is strictly better. The experiment reports the
/// normal ring fields plus the number of accepted crossovers and a material-mix
/// divergence (0 = homogenised islands, 1 = disjoint mixes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct I1Config {
    pub islands: usize,
    pub migration_interval: usize,
    /// Number of distinct source islands per child. Must be at least 2 and
    /// strictly less than `islands` so the destination is not a parent.
    pub parents: usize,
}

impl Default for I1Config {
    fn default() -> Self {
        Self {
            islands: 4,
            migration_interval: 10,
            parents: 2,
        }
    }
}

/// True variable-length Recipe A configuration.
///
/// Unlike [`RecipeAConfig`], the chromosome is not a fixed-width presence-coded
/// array: the engine samples a length in `[min_layers, max_layers]` and the
/// mutation portfolio can insert or delete genes. Crossover uses the
/// variable-length `Pad` alignment. Decoder presence keys remain part of the
/// gene, so an inserted gene can still be inactive until its presence key rises.
#[derive(Clone, Debug)]
pub struct VlRecipeAConfig {
    pub population_size: usize,
    pub generations: usize,
    pub min_layers: usize,
    pub max_layers: usize,
    pub mutation_sigma: f64,
    /// Optional parsimony pressure: adjusted fitness adds
    /// `length_penalty * DNA length` for minimization.
    pub length_penalty: Option<f64>,
    pub seed: Option<u64>,
}

impl Default for VlRecipeAConfig {
    fn default() -> Self {
        Self {
            population_size: 32,
            generations: 20,
            min_layers: 1,
            max_layers: 12,
            mutation_sigma: 0.15,
            length_penalty: None,
            seed: None,
        }
    }
}

/// Linear parsimony schedule for [`run_recipe_a_vl_ramp`].
///
/// The length penalty starts at `start` on generation 0 and reaches `end` on
/// the final generation. Values are coefficients on DNA length, not thickness
/// penalties.
#[derive(Clone, Copy, Debug)]
pub struct ParsimonyRamp {
    pub start: f64,
    pub end: f64,
}

impl ParsimonyRamp {
    pub fn new(start: f64, end: f64) -> Result<Self, String> {
        if !start.is_finite() || !end.is_finite() || start < 0.0 || end < 0.0 {
            return Err("parsimony ramp coefficients must be finite and non-negative".into());
        }
        Ok(Self { start, end })
    }
}

/// Best design from one Recipe A run.
#[derive(Clone, Debug)]
pub struct RecipeAResult {
    pub best_genome: Genome,
    pub decoded: DecodedStack,
    pub stack: navette::smatrix::synthesis::structure::DesignStack,
    pub merit: f64,
    pub generations: usize,
}

/// Work performed by the optional mid-loop memetic schedule.
#[derive(Clone, Debug, Default)]
pub struct MemeticRunReport {
    /// Number of scheduled passes actually executed.
    pub passes: usize,
    /// Chromosomes handed to the operator.
    pub refined: usize,
    /// Chromosomes whose write-back was accepted.
    pub improved: usize,
    /// Accepted needle insertions across all passes.
    pub insertions: usize,
    /// Spectral scans run by the operator across all passes.
    pub scans: usize,
    /// Refinements whose operator merit did not beat the starting merit.
    pub prediction_rejected: usize,
    /// Refinements rejected by A4 validation or the fixed-width write-back.
    pub write_back_rejected: usize,
    /// Refinements whose canonical write-back re-scored no better.
    pub merit_rejected: usize,
    /// Wall time spent advancing the GA, in seconds.
    pub ga_seconds: f64,
    /// Wall time spent in scheduled memetic passes, in seconds.
    pub memetic_seconds: f64,
}

impl MemeticRunReport {
    /// Fraction of the run spent in the mid-loop memetic schedule.
    pub fn memetic_share(&self) -> f64 {
        let total = self.ga_seconds + self.memetic_seconds;
        if total > 0.0 {
            self.memetic_seconds / total
        } else {
            0.0
        }
    }
}

/// A Recipe A run plus its mid-loop memetic bookkeeping.
#[derive(Clone, Debug)]
pub struct MemeticRunResult {
    pub best: RecipeAResult,
    pub report: MemeticRunReport,
}

/// H6 screening schedule for the linear surrogate arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurrogateConfig {
    /// True evaluations before the model is allowed to predict.
    pub warmup: usize,
    /// Re-evaluate every this-many-th requested chromosome.
    pub sample_every: usize,
    /// Refit the linear model after this many accepted samples.
    pub refit_every: usize,
}

impl Default for SurrogateConfig {
    fn default() -> Self {
        Self {
            warmup: 16,
            sample_every: 4,
            refit_every: 8,
        }
    }
}

/// True and predicted work performed by the H6 surrogate arm.
#[derive(Clone, Debug, Default)]
pub struct SurrogateReport {
    pub true_evaluations: usize,
    pub predicted_evaluations: usize,
    pub samples: usize,
    pub model_fits: usize,
}

/// A Recipe A run whose fitness closure mixed true TMM evaluations with a
/// linear surrogate.
#[derive(Clone, Debug)]
pub struct SurrogateRunResult {
    pub best: RecipeAResult,
    pub report: SurrogateReport,
}

/// What a terminal memetic finish did, for the Phase 1 scan certificate.
#[derive(Clone, Debug)]
pub struct TerminalReport {
    pub initial_merit: f64,
    pub final_merit: f64,
    /// Number of needle insertions the memetic operator accepted.
    pub insertions: usize,
    /// Total scans: the operator's scans plus the final certificate scan.
    pub scans: usize,
    /// Candidates the final scan still returned.
    pub scan_candidates_after: usize,
    /// Of those, how many actually lower the merit when inserted and scored.
    /// Zero is the Phase 1 scan certificate.
    pub actual_improving_after: usize,
    /// The last LM run's termination was a convergence test (or there was
    /// nothing free to optimize).
    pub lm_converged: bool,
    /// Smallest film thickness after the finish, in nm.
    pub min_thickness_nm: f64,
}

/// One island's operator regime.
///
/// The ring policy is shared; this is how S3's "heterogeneous islands" enter
/// without changing the migration contract. A strategy currently varies the
/// Gaussian mutation scale and the parent-selection rule.
#[derive(Clone, Copy, Debug)]
pub struct IslandStrategy {
    pub mutation_sigma: f64,
    pub selection: Selection,
}

impl Default for IslandStrategy {
    fn default() -> Self {
        Self {
            mutation_sigma: 0.15,
            selection: Selection::Tournament,
        }
    }
}

/// Ring policy plus one strategy per island.
#[derive(Clone, Debug)]
pub struct HeterogeneousIslandConfig {
    pub islands: IslandRecipeConfig,
    pub strategies: Vec<IslandStrategy>,
}

/// One island's decoder regime.
///
/// The pool, pins, and chromosome width stay shared; this narrows presence
/// and adjacency legality for an island. `None` leaves the base decoder's
/// value in place. Migrants are re-evaluated by the destination's next
/// `Ga` advance, so a genome legal in one tribe is not credited with the
/// source tribe's fitness.
#[derive(Clone, Debug, Default)]
pub struct TribeConstraint {
    pub presence_threshold: Option<f64>,
    pub grammar: Option<AdjacencyGrammar>,
}

/// Ring policy plus per-island operator and decoder regimes.
#[derive(Clone, Debug)]
pub struct HeterogeneousTribeConfig {
    pub islands: IslandRecipeConfig,
    pub strategies: Vec<IslandStrategy>,
    pub constraints: Vec<TribeConstraint>,
}

/// Result of an illuminated Recipe A run.
#[derive(Clone, Debug)]
pub struct IlluminatedResult {
    pub best: RecipeAResult,
    pub archive: MomeArchive,
    pub generations: usize,
}

/// One arm of an equal-budget comparison.
#[derive(Clone, Debug)]
pub struct BudgetArm {
    pub result: RecipeAResult,
    /// Solver evaluations actually spent, including LM function evaluations.
    pub evaluations: usize,
}

/// H0 versus GAs at the same solver-work budget.
#[derive(Clone, Debug)]
pub struct BudgetComparison {
    pub budget: usize,
    pub h0: BudgetArm,
    pub ga: BudgetArm,
}

/// A fixed-seed repeat set for one equal-budget comparison.
#[derive(Clone, Debug)]
pub struct RepeatComparison {
    pub budget: usize,
    pub seeds: Vec<u64>,
    pub h0: Vec<BudgetArm>,
    pub ga: Vec<BudgetArm>,
}

/// Best design plus the policy bookkeeping from an island run.
#[derive(Clone, Debug)]
pub struct IslandRecipeResult {
    pub best: RecipeAResult,
    /// Final best merit on each island, in island order.
    pub island_bests: Vec<f64>,
    /// Number of migration events; each island receives once per event.
    pub migrations: usize,
    /// Effective migrants per island per event (after the 5% cap).
    pub migrants_per_event: usize,
    /// Cross-island multi-parent crossover children accepted by I1. Zero for
    /// the default migrate-then-mate-locally policy.
    pub crossovers: usize,
    /// Mean pairwise L1 distance between island material histograms, in
    /// `[0, 1]`; zero means every island has the same material mix.
    pub homogenisation: f64,
    /// Mid-loop memetic work, or all-zero when no schedule was supplied.
    pub memetic: MemeticRunReport,
}

fn validate_recipe_a(cfg: &RecipeAConfig) -> Result<(), String> {
    if cfg.population_size < 2 {
        return Err("Recipe A needs a population of at least 2".into());
    }
    if cfg.max_layers == 0 {
        return Err("Recipe A needs max_layers >= 1".into());
    }
    if cfg.generations == 0 {
        return Err("Recipe A needs at least one generation".into());
    }
    if !cfg.mutation_sigma.is_finite() || cfg.mutation_sigma <= 0.0 {
        return Err("Recipe A mutation_sigma must be finite and positive".into());
    }
    Ok(())
}

fn validate_memetic_schedule(
    cfg: &RecipeAConfig,
    schedule: &MemeticSchedule,
) -> Result<(), String> {
    if schedule.interval == 0 {
        return Err("memetic schedule interval must be >= 1".into());
    }
    if schedule.top == 0 || schedule.top > cfg.population_size {
        return Err(format!(
            "memetic schedule top must be in 1..={}; got {}",
            cfg.population_size, schedule.top
        ));
    }
    if schedule.max_insertions == 0 {
        return Err("memetic schedule max_insertions must be >= 1".into());
    }
    Ok(())
}

fn recipe_a_fitness(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
) -> impl Fn(&[KeyGene]) -> f64 + Send + Sync + Clone + 'static {
    let decoder = decoder.clone();
    let template = template.clone();
    let evaluator = evaluator.clone();
    move |dna: &[KeyGene]| -> f64 {
        let genome = Genome::from(
            dna.iter()
                .map(|gene| LayerGene::new(gene.material, gene.thickness, gene.presence))
                .collect::<Vec<_>>(),
        );
        let Ok(decoded) = decoder.decode(&genome) else {
            return f64::INFINITY;
        };
        let Ok(stack) = checked_assemble(&decoder, &template, &decoded) else {
            return f64::INFINITY;
        };
        evaluator.evaluate(&stack).unwrap_or(f64::INFINITY)
    }
}

/// Assemble a decoded stack and enforce the decoder's A4 contract.
///
/// The GA evaluates through this helper so a genotype that decodes but cannot
/// be represented by the decoder's bounds/grammar is selected against, not
/// merely rejected at the end of a run.
fn checked_assemble(
    decoder: &Decoder,
    template: &StackTemplate,
    decoded: &DecodedStack,
) -> Result<navette::smatrix::synthesis::structure::DesignStack, String> {
    let stack = template.assemble(decoded)?;
    decoder.validate_stack(&stack)?;
    Ok(stack)
}

/// Encode a physical stack back into a fixed-width genome, then decode and
/// assemble it again so the returned genome/stack pair is the canonical one.
///
/// This is the one Lamarckian write-back used by both the terminal finish and
/// the mid-loop schedule. It preserves material identity by pool name (the
/// base label before `#`), and it refuses a stack that grows past `width`.
fn write_back_stack(
    decoder: &Decoder,
    template: &StackTemplate,
    stack: &navette::smatrix::synthesis::structure::DesignStack,
    width: usize,
) -> Result<
    (
        Genome,
        DecodedStack,
        navette::smatrix::synthesis::structure::DesignStack,
    ),
    String,
> {
    let films = stack
        .films()
        .iter()
        .map(|film| {
            let pool_name = film
                .label
                .as_deref()
                .unwrap_or_else(|| film.material.split('#').next().unwrap_or(&film.material));
            let index = decoder
                .pool()
                .materials()
                .iter()
                .position(|material| material.name() == pool_name)
                .ok_or_else(|| {
                    format!(
                        "terminal stack film {:?} is not in the decoder pool",
                        film.material
                    )
                })?;
            Ok((index, film.d_nm))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let genome = decoder
        .encode_films(&films, width)
        .map_err(|error| error.to_string())?;
    let decoded = decoder.decode(&genome).map_err(|e| e.to_string())?;
    let canonical = checked_assemble(decoder, template, &decoded)?;
    Ok((genome, decoded, canonical))
}

fn genome_to_dna(genome: &Genome) -> Vec<KeyGene> {
    genome
        .genes()
        .iter()
        .map(|gene| KeyGene::new(gene.material, gene.thickness, gene.presence))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn build_recipe_a_ga(
    cfg: &RecipeAConfig,
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    population: Option<Population<LayerChromosome>>,
    generations: usize,
    seed: Option<u64>,
    strategy: Option<&IslandStrategy>,
) -> Result<Ga<LayerChromosome>, String> {
    build_recipe_a_ga_with_fitness(
        cfg,
        decoder,
        template,
        evaluator,
        population,
        generations,
        seed,
        strategy,
        recipe_a_fitness(decoder, template, evaluator),
    )
}

#[allow(clippy::too_many_arguments)]
fn build_recipe_a_ga_with_fitness<F>(
    cfg: &RecipeAConfig,
    _decoder: &Decoder,
    _template: &StackTemplate,
    _evaluator: &FitnessEvaluator,
    population: Option<Population<LayerChromosome>>,
    generations: usize,
    seed: Option<u64>,
    strategy: Option<&IslandStrategy>,
    fitness: F,
) -> Result<Ga<LayerChromosome>, String>
where
    F: Fn(&[KeyGene]) -> f64 + Send + Sync + Clone + 'static,
{
    let init_width = cfg.max_layers;
    let init_fn = move |genes: usize, _alleles: Option<&[KeyGene]>| -> Vec<KeyGene> {
        let mut rng = make_rng();
        let width = if genes == 0 { init_width } else { genes };
        (0..width).map(|_| KeyGene::random(&mut rng)).collect()
    };

    // A supplied population was created outside this builder, so its
    // chromosomes still hold `FitnessFnWrapper::default()` (the constant
    // zero function). `Population::fitness_calculation` only evaluates NaN
    // fitness, so without this rebinding an island generation would silently
    // keep every chromosome at fitness 0.
    let mut population = population;
    if let Some(population) = population.as_mut() {
        for chromosome in &mut population.chromosomes {
            chromosome.set_fitness_fn(fitness.clone());
        }
    }

    let mut builder = Ga::<LayerChromosome>::new()
        .with_chromosome_length(ChromosomeLength::Fixed(cfg.max_layers))
        .with_population_size(cfg.population_size)
        .with_initialization_fn(init_fn)
        .with_fitness_fn(fitness)
        .with_selection_method(strategy.map_or(Selection::Tournament, |s| s.selection))
        .with_crossover_method(Crossover::Uniform)
        .with_mutation_method(Mutation::Gaussian(GaussianParams {
            sigma: Some(strategy.map_or(cfg.mutation_sigma, |s| s.mutation_sigma)),
        }))
        .with_survivor_method(Survivor::Fitness)
        .with_problem_solving(ProblemSolving::Minimization)
        .with_threads(1)
        .with_max_generations(generations);
    if let Some(population) = population {
        builder = builder.with_population(population);
    }
    if let Some(seed) = seed {
        builder = builder.with_rng_seed(seed);
    }
    builder.build().map_err(|e| e.to_string())
}

fn initial_population_seeded(
    cfg: &RecipeAConfig,
    seed: Option<u64>,
    initial_genomes: &[Genome],
) -> Result<Population<LayerChromosome>, String> {
    if initial_genomes.is_empty() {
        return Ok(initial_population(cfg, seed));
    }
    for (index, genome) in initial_genomes.iter().enumerate() {
        if genome.is_empty() {
            return Err(format!("seeded Recipe A initial genome {index} is empty"));
        }
        if genome.len() > cfg.max_layers {
            return Err(format!(
                "seeded Recipe A initial genome {index} has {} layers, above the width {}",
                genome.len(),
                cfg.max_layers
            ));
        }
    }

    set_seed(seed);
    let mut rng = make_rng();
    let chromosomes = (0..cfg.population_size)
        .map(|index| {
            let base = &initial_genomes[index % initial_genomes.len()];
            let mut dna = genome_to_dna(base);
            if index > 0 {
                // Keep the seed's active/inactive pattern, but diversify the
                // material and thickness keys so the first generation is not
                // a population of identical chromosomes.
                for gene in &mut dna {
                    gene.material =
                        wrap01(gene.material + gaussian(&mut rng) * cfg.mutation_sigma * 0.25);
                    gene.thickness =
                        wrap01(gene.thickness + gaussian(&mut rng) * cfg.mutation_sigma * 0.25);
                }
            }
            while dna.len() < cfg.max_layers {
                let mut padding = KeyGene::random(&mut rng);
                padding.presence = 0.0;
                dna.push(padding);
            }
            LayerChromosome::from_dna(dna)
        })
        .collect();
    Ok(Population::new(chromosomes))
}

fn initial_population(cfg: &RecipeAConfig, seed: Option<u64>) -> Population<LayerChromosome> {
    set_seed(seed);
    let mut rng = make_rng();
    let chromosomes = (0..cfg.population_size)
        .map(|_| {
            let dna = (0..cfg.max_layers)
                .map(|_| KeyGene::random(&mut rng))
                .collect();
            LayerChromosome::from_dna(dna)
        })
        .collect();
    Population::new(chromosomes)
}

/// Ring migration: snapshot each island's best before writing, then replace
/// each destination's worst with its predecessor's best only when strictly
/// better. Returns the number of migration events.
fn migrate_islands(populations: &mut [Population<LayerChromosome>], count: usize) -> usize {
    if populations.len() < 2 || count == 0 {
        return 0;
    }
    let snapshots: Vec<Vec<LayerChromosome>> = populations
        .iter()
        .map(|population| {
            let mut ranked = population.chromosomes.clone();
            ranked.sort_by(|a, b| a.fitness().total_cmp(&b.fitness()));
            ranked.truncate(count);
            ranked
        })
        .collect();

    for dest in 0..populations.len() {
        let source = (dest + populations.len() - 1) % populations.len();
        let mut worst: Vec<usize> = (0..populations[dest].chromosomes.len()).collect();
        worst.sort_by(|&a, &b| {
            populations[dest].chromosomes[b]
                .fitness()
                .total_cmp(&populations[dest].chromosomes[a].fitness())
        });
        for (migrant, &index) in snapshots[source].iter().zip(worst.iter()) {
            if migrant.fitness() < populations[dest].chromosomes[index].fitness() {
                populations[dest].chromosomes[index] = migrant.clone();
            }
        }
    }
    populations.len()
}

/// One child from `parents` distinct source islands, round-robin by gene.
///
/// The child is evaluated through the same decoder/template/evaluator as the
/// GA, so an illegal recombination is discarded exactly like an illegal
/// mutation. The fitter-than-worst replacement is the experiment's only
/// selection step; no destination parent enters the child.
fn cross_island_child(
    source: &[&Population<LayerChromosome>],
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    mutation_sigma: f64,
) -> Option<LayerChromosome> {
    let width = source
        .iter()
        .filter_map(|population| population.chromosomes.first().map(|c| c.dna().len()))
        .max()?;
    if width == 0 {
        return None;
    }
    let mut dna = Vec::with_capacity(width);
    for position in 0..width {
        let population = source[position % source.len()];
        let chromosome = population
            .chromosomes
            .get(position % population.chromosomes.len())?;
        let gene = chromosome.dna().get(position % chromosome.dna().len())?;
        dna.push(*gene);
    }
    let mut rng = make_rng();
    for gene in &mut dna {
        gene.material = wrap01(gene.material + gaussian(&mut rng) * mutation_sigma);
        gene.thickness = wrap01(gene.thickness + gaussian(&mut rng) * mutation_sigma);
        gene.presence = wrap01(gene.presence + gaussian(&mut rng) * mutation_sigma);
    }
    let mut child = LayerChromosome::from_dna(dna);
    let genome = child.to_genome();
    let decoded = decoder.decode(&genome).ok()?;
    let stack = checked_assemble(decoder, template, &decoded).ok()?;
    let merit = evaluator.evaluate(&stack).ok()?;
    if !merit.is_finite() {
        return None;
    }
    child.set_fitness(merit);
    Some(child)
}

/// Replace each destination's worst with one cross-island child. Returns the
/// number of accepted children.
fn migrate_i1(
    populations: &mut [Population<LayerChromosome>],
    parents: usize,
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    mutation_sigma: f64,
) -> usize {
    if populations.len() < 3 || parents < 2 || parents >= populations.len() {
        return 0;
    }
    let mut accepted = 0;
    for dest in 0..populations.len() {
        let source_indices: Vec<usize> = (1..=parents)
            .map(|offset| (dest + offset) % populations.len())
            .collect();
        let sources: Vec<&Population<LayerChromosome>> = source_indices
            .iter()
            .map(|&index| &populations[index])
            .collect();
        let Some(child) =
            cross_island_child(&sources, decoder, template, evaluator, mutation_sigma)
        else {
            continue;
        };
        let Some(worst_index) = (0..populations[dest].chromosomes.len()).max_by(|&a, &b| {
            populations[dest].chromosomes[a]
                .fitness()
                .total_cmp(&populations[dest].chromosomes[b].fitness())
        }) else {
            continue;
        };
        if child.fitness() < populations[dest].chromosomes[worst_index].fitness() {
            populations[dest].chromosomes[worst_index] = child;
            accepted += 1;
        }
    }
    accepted
}

/// Mean pairwise L1 distance between island material histograms, in `[0, 1]`.
fn island_divergence(populations: &[Population<LayerChromosome>], pool_len: usize) -> f64 {
    if populations.len() < 2 || pool_len == 0 {
        return 0.0;
    }
    let histogram = |population: &Population<LayerChromosome>| -> Vec<f64> {
        let mut bins = vec![0.0; pool_len];
        let mut total = 0.0;
        for chromosome in &population.chromosomes {
            for gene in chromosome.dna() {
                if gene.presence < 0.5 {
                    continue;
                }
                let index = ((gene.material.rem_euclid(1.0) * pool_len as f64).floor() as usize)
                    .min(pool_len - 1);
                bins[index] += 1.0;
                total += 1.0;
            }
        }
        if total > 0.0 {
            for bin in &mut bins {
                *bin /= total;
            }
        }
        bins
    };
    let histograms: Vec<Vec<f64>> = populations.iter().map(histogram).collect();
    let mut sum = 0.0;
    let mut pairs = 0usize;
    for i in 0..histograms.len() {
        for j in (i + 1)..histograms.len() {
            let distance = histograms[i]
                .iter()
                .zip(&histograms[j])
                .map(|(a, b)| (a - b).abs())
                .sum::<f64>()
                * 0.5;
            sum += distance;
            pairs += 1;
        }
    }
    if pairs == 0 {
        0.0
    } else {
        sum / pairs as f64
    }
}

/// Run the H0 control: independent random starts, LM, and optional needle.
///
/// The returned [`RecipeAResult`] uses the same shape as a GA run, so the
/// comparison harness can report it without a second result type.
pub fn run_h0_control(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &mut FitnessEvaluator,
    cfg: &H0Config,
    operator: Option<&MemeticOperator>,
) -> Result<RecipeAResult, String> {
    run_h0_control_inner(decoder, template, evaluator, cfg, operator, None)
}

/// H0 with a solver-evaluation budget.
///
/// The loop stops before a start would begin once `budget` solver evaluations
/// have been spent. A start may overshoot by its own LM work; the comparison
/// reports the actual count. `cfg.starts` still bounds the loop, so pass a
/// value at least as large as `budget` for a budget-limited run.
pub fn run_h0_control_budget(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &mut FitnessEvaluator,
    cfg: &H0Config,
    operator: Option<&MemeticOperator>,
    budget: usize,
) -> Result<RecipeAResult, String> {
    if budget == 0 {
        return Err("H0 budget must be >= 1".into());
    }
    run_h0_control_inner(decoder, template, evaluator, cfg, operator, Some(budget))
}

fn run_h0_control_inner(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &mut FitnessEvaluator,
    cfg: &H0Config,
    operator: Option<&MemeticOperator>,
    budget: Option<usize>,
) -> Result<RecipeAResult, String> {
    if cfg.starts == 0 {
        return Err("H0 needs at least one start".into());
    }
    if cfg.max_layers == 0 {
        return Err("H0 needs max_layers >= 1".into());
    }
    if !cfg.mutation_sigma.is_finite() || cfg.mutation_sigma <= 0.0 {
        return Err("H0 mutation_sigma must be finite and positive".into());
    }
    template.check_grammar(decoder.pool(), decoder.grammar())?;
    set_seed(cfg.seed);
    let mut best: Option<(
        f64,
        Genome,
        DecodedStack,
        navette::smatrix::synthesis::structure::DesignStack,
    )> = None;
    for _ in 0..cfg.starts {
        if budget.is_some_and(|budget| evaluator.evaluations() >= budget) {
            break;
        }
        let mut rng = make_rng();
        let genome = Genome::from(
            (0..cfg.max_layers)
                .map(|_| {
                    let gene = KeyGene::random(&mut rng);
                    LayerGene::new(gene.material, gene.thickness, gene.presence)
                })
                .collect::<Vec<_>>(),
        );
        let decoded = match decoder.decode(&genome) {
            Ok(decoded) => decoded,
            Err(_) => continue,
        };
        let mut stack = match checked_assemble(decoder, template, &decoded) {
            Ok(stack) => stack,
            Err(_) => continue,
        };
        evaluator.refine(&mut stack)?;
        // A random start whose LM path leaves the decoder's A4 bounds is a
        // discarded start, not a reason to abort the whole control arm.
        if decoder.validate_stack(&stack).is_err() {
            continue;
        }
        if let Some(operator) = operator {
            operator.apply(evaluator, &mut stack, cfg.needle_steps)?;
            if decoder.validate_stack(&stack).is_err() {
                continue;
            }
            evaluator.refine(&mut stack)?;
            if decoder.validate_stack(&stack).is_err() {
                continue;
            }
        }
        let merit = evaluator.evaluate(&stack)?;
        if merit.is_finite() && best.as_ref().is_none_or(|(b, ..)| merit < *b) {
            best = Some((merit, genome, decoded, stack));
        }
    }
    let (merit, best_genome, decoded, stack) =
        best.ok_or_else(|| "H0 found no legal design".to_string())?;
    Ok(RecipeAResult {
        best_genome,
        decoded,
        stack,
        merit,
        generations: 1,
    })
}

/// Run Recipe A while illuminating a MOME archive with every legal member of
/// every generation.
///
/// This is the Phase 2 S4-lite path: the archive does not steer selection yet,
/// but it records the merit-vs-depth/thickness map from the same GA run. The
/// selection remains the ordinary Recipe A engine, so the map cannot be
/// mistaken for a quality-diversity search.
pub fn run_recipe_a_illuminated(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &RecipeAConfig,
    archive_cfg: ArchiveConfig,
) -> Result<IlluminatedResult, String> {
    validate_recipe_a(cfg)?;
    template.check_grammar(decoder.pool(), decoder.grammar())?;
    let mut archive = MomeArchive::new(archive_cfg)?;
    let mut population = initial_population(cfg, cfg.seed);

    for generation in 0..cfg.generations {
        let seed = cfg
            .seed
            .map(|base| base.wrapping_add((generation as u64).wrapping_mul(1009).wrapping_add(7)));
        let mut ga = build_recipe_a_ga(
            cfg,
            decoder,
            template,
            evaluator,
            Some(population),
            1,
            seed,
            None,
        )?;
        ga.run().map_err(|e| e.to_string())?;
        population = ga.population.clone();
        for chromosome in &population.chromosomes {
            let genome = chromosome.to_genome();
            let Ok(decoded) = decoder.decode(&genome) else {
                continue;
            };
            let Ok(stack) = checked_assemble(decoder, template, &decoded) else {
                continue;
            };
            let merit = chromosome.fitness();
            if merit.is_finite() {
                let _ = archive.insert_stack(merit, stack);
            }
        }
    }

    let best = population
        .chromosomes
        .iter()
        .min_by(|a, b| a.fitness().total_cmp(&b.fitness()))
        .cloned()
        .ok_or_else(|| "illuminated Recipe A produced an empty population".to_string())?;
    if !best.fitness().is_finite() {
        return Err("illuminated Recipe A found no legal design".into());
    }
    let best_genome = best.to_genome();
    let decoded = decoder.decode(&best_genome).map_err(|e| e.to_string())?;
    let stack = checked_assemble(decoder, template, &decoded)?;
    let merit = evaluator.evaluate(&stack)?;
    Ok(IlluminatedResult {
        best: RecipeAResult {
            best_genome,
            decoded,
            stack,
            merit,
            generations: cfg.generations,
        },
        archive,
        generations: cfg.generations,
    })
}

struct SurrogateState {
    dim: usize,
    config: SurrogateConfig,
    samples: Vec<(Vec<f64>, f64)>,
    coefficients: Option<Vec<f64>>,
    calls: usize,
    true_evaluations: usize,
    predicted_evaluations: usize,
    model_fits: usize,
}

impl SurrogateState {
    fn new(dim: usize, config: SurrogateConfig) -> Self {
        Self {
            dim,
            config,
            samples: Vec::new(),
            coefficients: None,
            calls: 0,
            true_evaluations: 0,
            predicted_evaluations: 0,
            model_fits: 0,
        }
    }

    fn next_is_true(&self) -> bool {
        self.true_evaluations < self.config.warmup
            || self.coefficients.is_none()
            || self.calls.is_multiple_of(self.config.sample_every)
    }

    fn predict(&self, features: &[f64]) -> f64 {
        if let Some(coefficients) = &self.coefficients {
            let value = coefficients
                .iter()
                .zip(features)
                .map(|(coefficient, feature)| coefficient * feature)
                .sum::<f64>();
            value.max(0.0)
        } else {
            let mean = self.samples.iter().map(|(_, merit)| *merit).sum::<f64>()
                / self.samples.len().max(1) as f64;
            mean.max(0.0)
        }
    }

    fn add_sample(&mut self, features: Vec<f64>, merit: f64) {
        if merit.is_finite() {
            self.samples.push((features, merit));
        }
        if self.samples.len() > self.dim
            && self
                .samples
                .len()
                .is_multiple_of(self.config.refit_every.max(1))
        {
            if let Some(coefficients) = fit_linear(&self.samples, self.dim) {
                self.coefficients = Some(coefficients);
                self.model_fits += 1;
            }
        }
    }
}

fn fit_linear(samples: &[(Vec<f64>, f64)], dim: usize) -> Option<Vec<f64>> {
    if samples.len() < dim + 1 {
        return None;
    }
    let mut augmented = vec![vec![0.0; dim + 1]; dim];
    for (features, merit) in samples {
        if features.len() != dim {
            continue;
        }
        for row in 0..dim {
            for column in 0..dim {
                augmented[row][column] += features[row] * features[column];
            }
            augmented[row][dim] += features[row] * merit;
        }
    }
    // Ridge term keeps the normal equations solvable when the population has
    // not yet spanned every feature direction.
    for (index, row) in augmented.iter_mut().enumerate() {
        row[index] += 1e-9;
    }

    for column in 0..dim {
        let pivot = (column..dim).max_by(|&a, &b| {
            augmented[a][column]
                .abs()
                .total_cmp(&augmented[b][column].abs())
        })?;
        if augmented[pivot][column].abs() < 1e-12 {
            return None;
        }
        augmented.swap(column, pivot);
        let scale = augmented[column][column];
        for value in &mut augmented[column][column..=dim] {
            *value /= scale;
        }
        let pivot_row = augmented[column].clone();
        for (row, values) in augmented.iter_mut().enumerate().take(dim) {
            if row == column {
                continue;
            }
            let factor = values[column];
            for (value, pivot) in values[column..=dim]
                .iter_mut()
                .zip(&pivot_row[column..=dim])
            {
                *value -= factor * pivot;
            }
        }
    }
    Some((0..dim).map(|row| augmented[row][dim]).collect())
}

fn surrogate_features(
    decoder: &Decoder,
    stack: &navette::smatrix::synthesis::structure::DesignStack,
    max_layers: usize,
) -> Vec<f64> {
    let films = stack.films();
    let total = films.iter().map(|film| film.d_nm).sum::<f64>();
    let min = films
        .iter()
        .map(|film| film.d_nm)
        .reduce(f64::min)
        .unwrap_or(0.0);
    let materials = &decoder.pool().materials();
    let mut counts = vec![0.0; materials.len()];
    for film in films {
        let base = film
            .label
            .as_deref()
            .unwrap_or_else(|| film.material.split('#').next().unwrap_or(&film.material));
        if let Some(index) = materials
            .iter()
            .position(|material| material.name() == base)
        {
            counts[index] += 1.0;
        }
    }
    let count = films.len().max(1) as f64;
    let mut features = vec![
        1.0,
        films.len() as f64 / max_layers.max(1) as f64,
        total / 1000.0,
        (total / count) / 300.0,
        min / 300.0,
    ];
    features.extend(counts.into_iter().map(|value| value / count));
    features
}

/// Run fixed-width Recipe A with a linear surrogate screening the fitness
/// calls (H6).
///
/// The first `warmup` candidates and every `sample_every`-th later candidate
/// are evaluated by navette; the rest receive the current linear prediction.
/// The returned best design is always re-evaluated by navette, so a model
/// hallucination cannot become the reported merit. This is a screening arm,
/// not a trained GP or Bayesian optimiser.
pub fn run_recipe_a_surrogate(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &RecipeAConfig,
    surrogate: SurrogateConfig,
) -> Result<SurrogateRunResult, String> {
    validate_recipe_a(cfg)?;
    if surrogate.warmup == 0 || surrogate.sample_every == 0 || surrogate.refit_every == 0 {
        return Err("surrogate warmup/sample/refit intervals must be >= 1".into());
    }
    template.check_grammar(decoder.pool(), decoder.grammar())?;

    let dim = 5 + decoder.pool().len();
    let max_layers = cfg.max_layers;
    let state = Arc::new(Mutex::new(SurrogateState::new(dim, surrogate)));
    let state_for_fitness = Arc::clone(&state);
    let decoder_for_fitness = decoder.clone();
    let template_for_fitness = template.clone();
    let evaluator_for_fitness = evaluator.clone();
    let fitness = move |dna: &[KeyGene]| -> f64 {
        let genome = Genome::from(
            dna.iter()
                .map(|gene| LayerGene::new(gene.material, gene.thickness, gene.presence))
                .collect::<Vec<_>>(),
        );
        let Ok(decoded) = decoder_for_fitness.decode(&genome) else {
            return f64::INFINITY;
        };
        let Ok(stack) = checked_assemble(&decoder_for_fitness, &template_for_fitness, &decoded)
        else {
            return f64::INFINITY;
        };
        let features = surrogate_features(&decoder_for_fitness, &stack, max_layers);
        let mut state = match state_for_fitness.lock() {
            Ok(state) => state,
            Err(_) => return f64::INFINITY,
        };
        state.calls += 1;
        if state.next_is_true() {
            state.true_evaluations += 1;
            let merit = evaluator_for_fitness
                .evaluate(&stack)
                .unwrap_or(f64::INFINITY);
            state.add_sample(features, merit);
            merit
        } else {
            state.predicted_evaluations += 1;
            state.predict(&features)
        }
    };

    let mut ga = build_recipe_a_ga_with_fitness(
        cfg,
        decoder,
        template,
        evaluator,
        None,
        cfg.generations,
        cfg.seed,
        None,
        fitness,
    )?;
    ga.run().map_err(|e| e.to_string())?;
    let best = ga
        .population
        .chromosomes
        .iter()
        .min_by(|a, b| a.fitness().total_cmp(&b.fitness()))
        .cloned()
        .ok_or_else(|| "surrogate Recipe A produced an empty population".to_string())?;
    let best_genome = best.to_genome();
    let decoded = decoder.decode(&best_genome).map_err(|e| e.to_string())?;
    let stack = checked_assemble(decoder, template, &decoded)?;
    let merit = evaluator.evaluate(&stack)?;
    if !merit.is_finite() {
        return Err("surrogate Recipe A's final design is not finite".into());
    }
    let report = {
        let state = state.lock().map_err(|_| "surrogate state poisoned")?;
        SurrogateReport {
            true_evaluations: state.true_evaluations + 1,
            predicted_evaluations: state.predicted_evaluations,
            samples: state.samples.len(),
            model_fits: state.model_fits,
        }
    };
    Ok(SurrogateRunResult {
        best: RecipeAResult {
            best_genome,
            decoded,
            stack,
            merit,
            generations: cfg.generations,
        },
        report,
    })
}

/// Run Recipe A: single-island GA over random keys → decoder → navette merit.
///
/// `decoder`, `template` and `evaluator` are cloned into the fitness closure,
/// so the same problem definition serves every chromosome. Decode or evaluate
/// failures score `f64::INFINITY`; the GA minimizes, so malformed genomes are
/// selected against rather than panicking.
pub fn run_recipe_a(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &RecipeAConfig,
) -> Result<RecipeAResult, String> {
    validate_recipe_a(cfg)?;
    template.check_grammar(decoder.pool(), decoder.grammar())?;

    let mut ga = build_recipe_a_ga(
        cfg,
        decoder,
        template,
        evaluator,
        None,
        cfg.generations,
        cfg.seed,
        None,
    )?;
    let best = ga.run().map_err(|e| e.to_string())?.best_chromosome.clone();
    let best_genome = best.to_genome();
    let decoded = decoder.decode(&best_genome).map_err(|e| e.to_string())?;
    let stack = checked_assemble(decoder, template, &decoded)?;
    let merit = evaluator.evaluate(&stack)?;

    Ok(RecipeAResult {
        best_genome,
        decoded,
        stack,
        merit,
        generations: cfg.generations,
    })
}

/// Run Recipe A from a fixed-width genome bank.
///
/// Each seed is padded into `cfg.max_layers` slots; the seed's active genes
/// stay at the front and the remaining slots carry `presence = 0`. This lets a
/// two-layer design start a 40-layer-width search without changing the
/// fixed-width GA contract. With an empty bank this is ordinary random Recipe A.
pub fn run_recipe_a_seeded(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &RecipeAConfig,
    initial_genomes: &[Genome],
) -> Result<RecipeAResult, String> {
    if initial_genomes.is_empty() {
        return run_recipe_a(decoder, template, evaluator, cfg);
    }
    validate_recipe_a(cfg)?;
    template.check_grammar(decoder.pool(), decoder.grammar())?;

    let population = initial_population_seeded(cfg, cfg.seed, initial_genomes)?;
    let mut ga = build_recipe_a_ga(
        cfg,
        decoder,
        template,
        evaluator,
        Some(population),
        cfg.generations,
        cfg.seed,
        None,
    )?;
    let best = ga.run().map_err(|e| e.to_string())?.best_chromosome.clone();
    let best_genome = best.to_genome();
    let decoded = decoder.decode(&best_genome).map_err(|e| e.to_string())?;
    let stack = checked_assemble(decoder, template, &decoded)?;
    let merit = evaluator.evaluate(&stack)?;

    Ok(RecipeAResult {
        best_genome,
        decoded,
        stack,
        merit,
        generations: cfg.generations,
    })
}

/// Run Recipe A until a solver-evaluation budget is reached.
///
/// One generation is the stopping grain: the loop checks the shared evaluator
/// counter before each generation, so the final count can overshoot by at most
/// one population evaluation. The returned `generations` field is the number
/// actually advanced, not the configured maximum.
pub fn run_recipe_a_budget(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &RecipeAConfig,
    budget: usize,
) -> Result<RecipeAResult, String> {
    validate_recipe_a(cfg)?;
    template.check_grammar(decoder.pool(), decoder.grammar())?;
    if budget == 0 {
        return Err("Recipe A budget must be >= 1".into());
    }

    let mut population = initial_population(cfg, cfg.seed);
    let mut generations = 0usize;
    for generation in 0..cfg.generations {
        if generation > 0 && evaluator.evaluations() >= budget {
            break;
        }
        let seed = cfg.seed.map(|base| {
            base.wrapping_add(((generation as u64).wrapping_mul(1009)).wrapping_add(7))
        });
        let mut ga = build_recipe_a_ga(
            cfg,
            decoder,
            template,
            evaluator,
            Some(population),
            1,
            seed,
            None,
        )?;
        ga.run().map_err(|e| e.to_string())?;
        population = ga.population.clone();
        generations += 1;
    }

    let best = population
        .chromosomes
        .iter()
        .min_by(|a, b| a.fitness().total_cmp(&b.fitness()))
        .cloned()
        .ok_or_else(|| "Recipe A produced an empty population".to_string())?;
    if !best.fitness().is_finite() {
        return Err("Recipe A found no legal design".into());
    }
    let best_genome = best.to_genome();
    let decoded = decoder.decode(&best_genome).map_err(|e| e.to_string())?;
    let stack = checked_assemble(decoder, template, &decoded)?;
    let merit = evaluator.evaluate(&stack)?;
    Ok(RecipeAResult {
        best_genome,
        decoded,
        stack,
        merit,
        generations,
    })
}

/// Run Recipe A on a ring of islands with best-replace-worst-if-better
/// migration.
///
/// The engine is `genetic_algorithms`' single-population `Ga`, advanced one
/// generation per island. That keeps the E09 v2 §2.8.1 rule — a migrant may
/// never make a destination worse — explicit instead of relying on the
/// dependency's unconditional `BestReplaceWorst` policy. A seeded run is
/// reproducible because every island and generation gets its own derived
/// RNG seed.
#[allow(clippy::too_many_arguments)]
fn run_recipe_a_islands_with_strategies(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &RecipeAConfig,
    islands: &IslandRecipeConfig,
    strategies: Option<&[IslandStrategy]>,
    constraints: Option<&[TribeConstraint]>,
    i1: Option<&I1Config>,
    memetic: Option<(&MemeticOperator, &MemeticSchedule)>,
) -> Result<IslandRecipeResult, String> {
    validate_recipe_a(cfg)?;
    if let Some((_, schedule)) = memetic {
        validate_memetic_schedule(cfg, schedule)?;
    }
    if let Some(strategies) = strategies {
        validate_island_strategies(strategies, islands.islands)?;
    }
    if let Some(constraints) = constraints {
        validate_tribe_constraints(constraints, islands.islands, decoder.pool().len())?;
    }
    if islands.islands < 2 {
        return Err("island Recipe A needs at least two islands".into());
    }
    if islands.migration_interval == 0 {
        return Err("island Recipe A migration_interval must be >= 1".into());
    }
    if islands.migration_count == 0 {
        return Err("island Recipe A migration_count must be >= 1".into());
    }
    if let Some(i1) = i1 {
        if i1.islands != islands.islands {
            return Err(format!(
                "I1: {} islands in the policy but {} in the experiment",
                islands.islands, i1.islands
            ));
        }
        if i1.islands < 3 || i1.parents < 2 || i1.parents >= i1.islands {
            return Err("I1 needs at least 3 islands and 2 <= parents < islands".into());
        }
    }
    template.check_grammar(decoder.pool(), decoder.grammar())?;

    let cap = ((cfg.population_size as f64) * 0.05).floor().max(1.0) as usize;
    let migrants_per_event = islands.migration_count.min(cap);
    let mut populations: Vec<Population<LayerChromosome>> = (0..islands.islands)
        .map(|island| {
            let seed = cfg
                .seed
                .map(|base| base.wrapping_add((island as u64).wrapping_mul(1009)));
            initial_population(cfg, seed)
        })
        .collect();
    let mut migrations = 0;
    let mut crossovers = 0;
    let mut memetic_report = MemeticRunReport::default();
    let mut memetic_evaluator = memetic.map(|_| evaluator.clone());

    for generation in 0..cfg.generations {
        for (island, population) in populations.iter_mut().enumerate() {
            let seed = cfg.seed.map(|base| {
                base.wrapping_add(
                    ((generation * islands.islands + island) as u64)
                        .wrapping_mul(1009)
                        .wrapping_add(7),
                )
            });
            let strategy = strategies.map(|all| &all[island]);
            let island_decoder = match constraints {
                Some(all) => tribe_decoder(decoder, &all[island])?,
                None => decoder.clone(),
            };
            let ga_started = std::time::Instant::now();
            let mut ga = build_recipe_a_ga(
                cfg,
                &island_decoder,
                template,
                evaluator,
                Some(population.clone()),
                1,
                seed,
                strategy,
            )?;
            ga.run().map_err(|e| e.to_string())?;
            *population = ga.population.clone();
            memetic_report.ga_seconds += ga_started.elapsed().as_secs_f64();
            if let (Some((operator, schedule)), Some(ev)) = (memetic, memetic_evaluator.as_mut()) {
                if (generation + 1) % schedule.interval == 0 {
                    let memetic_started = std::time::Instant::now();
                    let pass = memetic_pass(
                        &island_decoder,
                        template,
                        ev,
                        operator,
                        population,
                        schedule.top,
                        schedule.max_insertions,
                        cfg.max_layers,
                    )?;
                    memetic_report.memetic_seconds += memetic_started.elapsed().as_secs_f64();
                    memetic_report.passes += 1;
                    memetic_report.refined += pass.refined;
                    memetic_report.improved += pass.improved;
                    memetic_report.insertions += pass.insertions;
                    memetic_report.scans += pass.scans;
                    memetic_report.prediction_rejected += pass.prediction_rejected;
                    memetic_report.write_back_rejected += pass.write_back_rejected;
                    memetic_report.merit_rejected += pass.merit_rejected;
                }
            }
        }
        if (generation + 1) % islands.migration_interval == 0 {
            migrations += populations.len();
            match i1 {
                Some(i1) => {
                    crossovers += migrate_i1(
                        &mut populations,
                        i1.parents,
                        decoder,
                        template,
                        evaluator,
                        cfg.mutation_sigma,
                    );
                }
                None => {
                    migrate_islands(&mut populations, migrants_per_event);
                }
            }
        }
    }

    let homogenisation = island_divergence(&populations, decoder.pool().len());
    let best = populations
        .iter()
        .flat_map(|population| population.chromosomes.iter())
        .min_by(|a, b| a.fitness().total_cmp(&b.fitness()))
        .cloned()
        .ok_or_else(|| "island Recipe A produced an empty population".to_string())?;
    if !best.fitness().is_finite() {
        return Err("island Recipe A found no legal design".into());
    }

    let best_genome = best.to_genome();
    let decoded = decoder.decode(&best_genome).map_err(|e| e.to_string())?;
    let stack = checked_assemble(decoder, template, &decoded)?;
    let merit = evaluator.evaluate(&stack)?;
    let island_bests = populations
        .iter()
        .map(|population| {
            population
                .chromosomes
                .iter()
                .map(LayerChromosome::fitness)
                .min_by(|a, b| a.total_cmp(b))
                .unwrap_or(f64::INFINITY)
        })
        .collect();

    Ok(IslandRecipeResult {
        best: RecipeAResult {
            best_genome,
            decoded,
            stack,
            merit,
            generations: cfg.generations,
        },
        island_bests,
        migrations,
        migrants_per_event,
        crossovers,
        homogenisation,
        memetic: memetic_report,
    })
}

/// Count final-scan candidates and the subset that actually improves the
/// merit when inserted and evaluated. The analytic P sweep is a predictor;
/// the certificate is the trial score.
fn certificate_scan(
    evaluator: &FitnessEvaluator,
    stack: &navette::smatrix::synthesis::structure::DesignStack,
    operator: &MemeticOperator,
    decoder: &Decoder,
) -> Result<(usize, usize), String> {
    let scan = operator.scan(evaluator, stack)?;
    let current = evaluator.evaluate(stack)?;
    let mut actual = 0usize;
    for candidate in &scan.candidates {
        let mut trial = stack.clone();
        trial.insert_needle_candidate(&operator.pool, candidate)?;
        // A predicted improvement that the decoder cannot represent is not a
        // certificate candidate: the write-back would clamp or repair it away.
        if decoder.validate_stack(&trial).is_err() {
            continue;
        }
        if evaluator.evaluate(&trial)? < current - 1e-12 {
            actual += 1;
        }
    }
    Ok((scan.candidates.len(), actual))
}

/// Apply a terminal LM polish and up to `max_insertions` needle steps to a
/// Recipe A result, then write the final stack back into a fixed-width genome
/// at the decoder's cap.
///
/// This is the Phase 1 "terminal needle/LM" finish: the stack is polished
/// first, the memetic operator scans/inserts/polishes while it can improve,
/// and a final scan must be empty. The final stack is encoded back into the
/// same chromosome width (Lamarckian write-back), re-decoded, and assembled so
/// the returned genome, decoded stack, and design stack agree.
///
/// The finish writes back at the decoder's maximum width, so terminal
/// insertions have room beyond the GA's starting `RecipeAConfig::max_layers`.
/// It errors if the stack grows past that cap, or if an actually improving
/// candidate remains after `max_insertions` steps, so the scan certificate
/// cannot be silently skipped.
pub fn finish_recipe_a(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &mut FitnessEvaluator,
    mut result: RecipeAResult,
    operator: &MemeticOperator,
    max_insertions: usize,
) -> Result<(RecipeAResult, TerminalReport), String> {
    decoder.validate_stack(&result.stack)?;
    let initial_merit = evaluator.evaluate(&result.stack)?;
    evaluator.refine_report(&mut result.stack)?;
    decoder.validate_stack(&result.stack)?;
    let memetic = operator.apply(evaluator, &mut result.stack, max_insertions)?;
    decoder.validate_stack(&result.stack)?;
    let (_, lm) = evaluator.refine_report(&mut result.stack)?;
    decoder.validate_stack(&result.stack)?;
    let lm_converged = lm
        .as_ref()
        .is_none_or(|report| report.termination.is_converged());

    let width = decoder.max_layers().max(result.best_genome.len());
    let (best_genome, decoded, stack) = write_back_stack(decoder, template, &result.stack, width)?;
    // Certify the design that will be returned, not the pre-write-back stack:
    // `encode_films` clamps thicknesses into the decoder bounds and re-decoding
    // can repair an adjacency.
    let (scan_candidates_after, actual_improving_after) =
        certificate_scan(evaluator, &stack, operator, decoder)?;
    if actual_improving_after > 0 {
        return Err(format!(
            "terminal scan certificate failed: {actual_improving_after} of              {scan_candidates_after} scanned candidate(s) still improve the merit              after {} insertion(s)",
            memetic.steps.len()
        ));
    }
    let merit = evaluator.evaluate(&stack)?;
    let min_thickness_nm = stack
        .films()
        .iter()
        .map(|film| film.d_nm)
        .reduce(f64::min)
        .unwrap_or(0.0);

    let report = TerminalReport {
        initial_merit,
        final_merit: merit,
        insertions: memetic.steps.len(),
        scans: memetic.scans + 1,
        scan_candidates_after,
        actual_improving_after,
        lm_converged,
        min_thickness_nm,
    };
    result.best_genome = best_genome;
    result.decoded = decoded;
    result.stack = stack;
    result.merit = merit;
    Ok((result, report))
}

/// Run Recipe A and immediately apply its terminal memetic finish.
pub fn run_recipe_a_finished(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &mut FitnessEvaluator,
    cfg: &RecipeAConfig,
    operator: &MemeticOperator,
    max_insertions: usize,
) -> Result<(RecipeAResult, TerminalReport), String> {
    let result = run_recipe_a(decoder, template, evaluator, cfg)?;
    finish_recipe_a(
        decoder,
        template,
        evaluator,
        result,
        operator,
        max_insertions,
    )
}

/// Refine the best `top` chromosomes in place and write improvements back.
///
/// A chromosome is replaced only when the memetic stack is legal, the
/// canonical write-back is legal, and the re-evaluated merit is strictly
/// better than the chromosome's current fitness. No population member is
/// graded by the operator's prediction alone.
#[allow(clippy::too_many_arguments)]
fn memetic_pass(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &mut FitnessEvaluator,
    operator: &MemeticOperator,
    population: &mut Population<LayerChromosome>,
    top: usize,
    max_insertions: usize,
    width: usize,
) -> Result<MemeticRunReport, String> {
    let mut report = MemeticRunReport::default();
    if top == 0 || population.chromosomes.is_empty() {
        return Ok(report);
    }

    let mut ranked: Vec<usize> = (0..population.chromosomes.len()).collect();
    ranked.sort_by(|&a, &b| {
        population.chromosomes[a]
            .fitness()
            .total_cmp(&population.chromosomes[b].fitness())
    });

    for index in ranked.into_iter().take(top) {
        let current = population.chromosomes[index].fitness();
        if !current.is_finite() {
            continue;
        }
        let genome = population.chromosomes[index].to_genome();
        let Ok(decoded) = decoder.decode(&genome) else {
            continue;
        };
        let Ok(mut stack) = checked_assemble(decoder, template, &decoded) else {
            continue;
        };
        let before = evaluator.evaluate(&stack)?;
        let memetic = operator.apply(evaluator, &mut stack, max_insertions)?;
        report.refined += 1;
        report.scans += memetic.scans;
        report.insertions += memetic.steps.len();
        if !memetic.final_merit.is_finite() || memetic.final_merit >= before {
            report.prediction_rejected += 1;
            continue;
        }
        if decoder.validate_stack(&stack).is_err() || stack.films().len() > width {
            report.write_back_rejected += 1;
            continue;
        }
        let Ok((genome, _, canonical)) = write_back_stack(decoder, template, &stack, width) else {
            report.write_back_rejected += 1;
            continue;
        };
        let merit = evaluator.evaluate(&canonical)?;
        if !merit.is_finite() || merit >= current {
            report.merit_rejected += 1;
            continue;
        }
        let mut chromosome = LayerChromosome::from_dna(genome_to_dna(&genome));
        chromosome.set_fitness(merit);
        population.chromosomes[index] = chromosome;
        report.improved += 1;
    }
    Ok(report)
}

/// Run Recipe A with an optional mid-loop needle schedule (G4/S2).
///
/// This is the memetic arm: every `schedule.interval` generations, the best
/// `schedule.top` chromosomes are refined by the same [`MemeticOperator`] the
/// terminal finish uses, and an improving legal refinement is written back
/// into the population before the next generation. The terminal finish is
/// still required to establish the scan certificate.
///
/// The schedule is deliberately outside [`RecipeAConfig`]: the operator owns
/// spectral inputs and a needle pool, so it cannot be passed through the
/// plain configuration that island and H0 drivers share.
pub fn run_recipe_a_memetic(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &mut FitnessEvaluator,
    cfg: &RecipeAConfig,
    operator: &MemeticOperator,
    schedule: &MemeticSchedule,
) -> Result<MemeticRunResult, String> {
    validate_recipe_a(cfg)?;
    validate_memetic_schedule(cfg, schedule)?;
    template.check_grammar(decoder.pool(), decoder.grammar())?;

    let mut population = initial_population(cfg, cfg.seed);
    let mut report = MemeticRunReport::default();
    for generation in 0..cfg.generations {
        let seed = cfg.seed.map(|base| {
            base.wrapping_add(((generation as u64).wrapping_mul(1009)).wrapping_add(7))
        });
        let ga_started = std::time::Instant::now();
        let mut ga = build_recipe_a_ga(
            cfg,
            decoder,
            template,
            evaluator,
            Some(population),
            1,
            seed,
            None,
        )?;
        ga.run().map_err(|e| e.to_string())?;
        population = ga.population.clone();
        report.ga_seconds += ga_started.elapsed().as_secs_f64();

        if (generation + 1) % schedule.interval == 0 {
            let memetic_started = std::time::Instant::now();
            let pass = memetic_pass(
                decoder,
                template,
                evaluator,
                operator,
                &mut population,
                schedule.top,
                schedule.max_insertions,
                cfg.max_layers,
            )?;
            report.memetic_seconds += memetic_started.elapsed().as_secs_f64();
            report.passes += 1;
            report.refined += pass.refined;
            report.improved += pass.improved;
            report.insertions += pass.insertions;
            report.scans += pass.scans;
            report.prediction_rejected += pass.prediction_rejected;
            report.write_back_rejected += pass.write_back_rejected;
            report.merit_rejected += pass.merit_rejected;
        }
    }

    let best = population
        .chromosomes
        .iter()
        .min_by(|a, b| a.fitness().total_cmp(&b.fitness()))
        .cloned()
        .ok_or_else(|| "Recipe A produced an empty population".to_string())?;
    if !best.fitness().is_finite() {
        return Err("Recipe A found no legal design".into());
    }
    let best_genome = best.to_genome();
    let decoded = decoder.decode(&best_genome).map_err(|e| e.to_string())?;
    let stack = checked_assemble(decoder, template, &decoded)?;
    let merit = evaluator.evaluate(&stack)?;
    Ok(MemeticRunResult {
        best: RecipeAResult {
            best_genome,
            decoded,
            stack,
            merit,
            generations: cfg.generations,
        },
        report,
    })
}

fn validate_island_strategies(strategies: &[IslandStrategy], islands: usize) -> Result<(), String> {
    if strategies.len() != islands {
        return Err(format!(
            "heterogeneous islands: {} strategy entries for {islands} islands",
            strategies.len()
        ));
    }
    for (index, strategy) in strategies.iter().enumerate() {
        if !strategy.mutation_sigma.is_finite() || strategy.mutation_sigma <= 0.0 {
            return Err(format!(
                "heterogeneous islands: island {index} mutation_sigma must be finite and positive"
            ));
        }
    }
    Ok(())
}

fn validate_tribe_constraints(
    constraints: &[TribeConstraint],
    islands: usize,
    pool_len: usize,
) -> Result<(), String> {
    if constraints.len() != islands {
        return Err(format!(
            "tribes: {} constraint entries for {islands} islands",
            constraints.len()
        ));
    }
    for (index, constraint) in constraints.iter().enumerate() {
        if let Some(threshold) = constraint.presence_threshold {
            if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
                return Err(format!(
                    "tribes: island {index} presence_threshold must be in [0, 1]"
                ));
            }
        }
        if let Some(grammar) = &constraint.grammar {
            if grammar.len() != pool_len {
                return Err(format!(
                    "tribes: island {index} grammar covers {} materials, the decoder pool has {pool_len}",
                    grammar.len()
                ));
            }
        }
    }
    Ok(())
}

fn tribe_decoder(base: &Decoder, constraint: &TribeConstraint) -> Result<Decoder, String> {
    let mut decoder = base.clone();
    if let Some(threshold) = constraint.presence_threshold {
        decoder = decoder
            .with_presence_threshold(threshold)
            .map_err(|error| error.to_string())?;
    }
    if let Some(grammar) = &constraint.grammar {
        decoder = decoder
            .with_grammar(grammar.clone())
            .map_err(|error| error.to_string())?;
    }
    Ok(decoder)
}

/// Run Recipe A on the ring with per-island operator and decoder regimes.
pub fn run_recipe_a_tribes(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &RecipeAConfig,
    tribes: &HeterogeneousTribeConfig,
) -> Result<IslandRecipeResult, String> {
    run_recipe_a_islands_with_strategies(
        decoder,
        template,
        evaluator,
        cfg,
        &tribes.islands,
        Some(&tribes.strategies),
        Some(&tribes.constraints),
        None,
        None,
    )
}

/// Run Recipe A on the ring with one operator regime per island.
pub fn run_recipe_a_islands(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &RecipeAConfig,
    islands: &IslandRecipeConfig,
) -> Result<IslandRecipeResult, String> {
    run_recipe_a_islands_with_strategies(
        decoder, template, evaluator, cfg, islands, None, None, None, None,
    )
}

/// Run the ring with a mid-loop memetic schedule on every island.
///
/// The schedule is applied independently to each island's population after
/// its generation, before migration. The result's [`MemeticRunReport`] counts
/// every island pass, so `passes` is normally
/// `islands * floor(generations / interval)`.
pub fn run_recipe_a_islands_memetic(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &RecipeAConfig,
    islands: &IslandRecipeConfig,
    operator: &MemeticOperator,
    schedule: &MemeticSchedule,
) -> Result<IslandRecipeResult, String> {
    run_recipe_a_islands_with_strategies(
        decoder,
        template,
        evaluator,
        cfg,
        islands,
        None,
        None,
        None,
        Some((operator, schedule)),
    )
}

/// Compare H0 and H1 at the same solver-evaluation budget.
///
/// The budget counts `FitnessEvaluator::evaluate` calls and the LM solver's
/// own `evals`; it is a compute proxy, not a wall-clock timer. Each arm gets a
/// fresh counter (ordinary evaluator clones share one), and the caller should
/// use the actual counts in `BudgetArm::evaluations` when reporting the result.
/// The GA is allowed to overshoot by at most one population generation and H0
/// by one start.
pub fn compare_h0_and_ga_at_budget(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    h0_cfg: &H0Config,
    ga_cfg: &RecipeAConfig,
    budget: usize,
) -> Result<BudgetComparison, String> {
    if budget == 0 {
        return Err("budget comparison needs budget >= 1".into());
    }

    let mut h0_cfg = *h0_cfg;
    h0_cfg.starts = h0_cfg.starts.max(budget);
    let mut h0_eval = evaluator.fresh_counter();
    let h0 = run_h0_control_budget(decoder, template, &mut h0_eval, &h0_cfg, None, budget)?;
    let h0_evaluations = h0_eval.evaluations();

    let mut ga_cfg = ga_cfg.clone();
    ga_cfg.generations = ga_cfg.generations.max(budget);
    let ga_eval = evaluator.fresh_counter();
    let ga = run_recipe_a_budget(decoder, template, &ga_eval, &ga_cfg, budget)?;
    let ga_evaluations = ga_eval.evaluations();

    Ok(BudgetComparison {
        budget,
        h0: BudgetArm {
            result: h0,
            evaluations: h0_evaluations,
        },
        ga: BudgetArm {
            result: ga,
            evaluations: ga_evaluations,
        },
    })
}

/// Repeat the equal-budget H0/H1 comparison over fixed seeds.
///
/// This is the repeat-runner skeleton for the §2.7 protocol: every seed gets
/// the same budget and the same decoder/template, and the per-run stacks stay
/// available for distributional reporting. It does not add a statistical test
/// or multiplicity correction; callers must report the actual evaluation
/// counts and must not treat the sample as a finished campaign.
pub fn compare_h0_and_ga_repeats(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    h0_cfg: &H0Config,
    ga_cfg: &RecipeAConfig,
    seeds: &[u64],
    budget: usize,
) -> Result<RepeatComparison, String> {
    if seeds.is_empty() {
        return Err("repeat comparison needs at least one seed".into());
    }
    let mut h0 = Vec::with_capacity(seeds.len());
    let mut ga = Vec::with_capacity(seeds.len());
    for &seed in seeds {
        let mut h0_cfg = *h0_cfg;
        h0_cfg.seed = Some(seed);
        let mut ga_cfg = ga_cfg.clone();
        ga_cfg.seed = Some(seed);
        let comparison =
            compare_h0_and_ga_at_budget(decoder, template, evaluator, &h0_cfg, &ga_cfg, budget)?;
        h0.push(comparison.h0);
        ga.push(comparison.ga);
    }
    Ok(RepeatComparison {
        budget,
        seeds: seeds.to_vec(),
        h0,
        ga,
    })
}

/// Run the I1 cross-island multi-parent experiment.
///
/// This is the same ring and the same per-generation advance as
/// [`run_recipe_a_islands`]; only the migration event differs. The result's
/// `crossovers` and `homogenisation` fields are the experiment readout.
pub fn run_recipe_a_i1(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &RecipeAConfig,
    i1: &I1Config,
) -> Result<IslandRecipeResult, String> {
    let islands = IslandRecipeConfig {
        islands: i1.islands,
        migration_interval: i1.migration_interval,
        migration_count: 1,
    };
    run_recipe_a_islands_with_strategies(
        decoder,
        template,
        evaluator,
        cfg,
        &islands,
        None,
        None,
        Some(i1),
        None,
    )
}

/// Run Recipe A on the ring with a per-island mutation/selection strategy.
pub fn run_recipe_a_heterogeneous_islands(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &RecipeAConfig,
    heterogeneous: &HeterogeneousIslandConfig,
) -> Result<IslandRecipeResult, String> {
    validate_island_strategies(&heterogeneous.strategies, heterogeneous.islands.islands)?;
    run_recipe_a_islands_with_strategies(
        decoder,
        template,
        evaluator,
        cfg,
        &heterogeneous.islands,
        Some(&heterogeneous.strategies),
        None,
        None,
        None,
    )
}

/// Run variable-length Recipe A on the Phase 2 ring.
pub fn run_recipe_a_vl_islands(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &VlRecipeAConfig,
    islands: &IslandRecipeConfig,
) -> Result<IslandRecipeResult, String> {
    run_recipe_a_vl_islands_with_strategies(
        decoder, template, evaluator, cfg, islands, None, None, None,
    )
}

/// Run variable-length Recipe A on the ring with a per-island strategy.
pub fn run_recipe_a_vl_heterogeneous_islands(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &VlRecipeAConfig,
    heterogeneous: &HeterogeneousIslandConfig,
) -> Result<IslandRecipeResult, String> {
    validate_island_strategies(&heterogeneous.strategies, heterogeneous.islands.islands)?;
    run_recipe_a_vl_islands_with_strategies(
        decoder,
        template,
        evaluator,
        cfg,
        &heterogeneous.islands,
        Some(&heterogeneous.strategies),
        None,
        None,
    )
}

/// Run variable-length Recipe A on the ring with per-island decoder regimes.
pub fn run_recipe_a_vl_tribes(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &VlRecipeAConfig,
    tribes: &HeterogeneousTribeConfig,
) -> Result<IslandRecipeResult, String> {
    run_recipe_a_vl_islands_with_strategies(
        decoder,
        template,
        evaluator,
        cfg,
        &tribes.islands,
        Some(&tribes.strategies),
        Some(&tribes.constraints),
        None,
    )
}

fn validate_vl_recipe_a(cfg: &VlRecipeAConfig) -> Result<(), String> {
    if cfg.population_size < 2 {
        return Err("VL Recipe A needs a population of at least 2".into());
    }
    if cfg.min_layers == 0 || cfg.min_layers > cfg.max_layers {
        return Err(format!(
            "VL Recipe A needs 1 <= min_layers <= max_layers; got {} and {}",
            cfg.min_layers, cfg.max_layers
        ));
    }
    if cfg.generations == 0 {
        return Err("VL Recipe A needs at least one generation".into());
    }
    if !cfg.mutation_sigma.is_finite() || cfg.mutation_sigma <= 0.0 {
        return Err("VL Recipe A mutation_sigma must be finite and positive".into());
    }
    if cfg
        .length_penalty
        .is_some_and(|penalty| !penalty.is_finite() || penalty < 0.0)
    {
        return Err("VL Recipe A length_penalty must be finite and non-negative".into());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn build_vl_recipe_a_ga_with(
    cfg: &VlRecipeAConfig,
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    population: Option<Population<LayerChromosome>>,
    generations: usize,
    seed: Option<u64>,
    strategy: Option<&IslandStrategy>,
) -> Result<Ga<LayerChromosome>, String> {
    let init_width = cfg.max_layers;
    let init_fn = move |genes: usize, _alleles: Option<&[KeyGene]>| -> Vec<KeyGene> {
        let mut rng = make_rng();
        let width = if genes == 0 { init_width } else { genes };
        (0..width).map(|_| KeyGene::random(&mut rng)).collect()
    };
    let mutations = vec![
        Mutation::Gaussian(GaussianParams {
            sigma: Some(strategy.map_or(cfg.mutation_sigma, |s| s.mutation_sigma)),
        }),
        Mutation::Insertion,
        Mutation::Deletion,
    ];

    let fitness = recipe_a_fitness(decoder, template, evaluator);
    let mut population = population;
    if let Some(population) = population.as_mut() {
        for chromosome in &mut population.chromosomes {
            chromosome.set_fitness_fn(fitness.clone());
        }
    }

    let mut builder = Ga::<LayerChromosome>::new()
        .with_chromosome_length(ChromosomeLength::Variable {
            min: cfg.min_layers,
            max: cfg.max_layers,
        })
        .with_population_size(cfg.population_size)
        .with_initialization_fn(init_fn)
        .with_fitness_fn(fitness)
        .with_selection_method(strategy.map_or(Selection::Tournament, |s| s.selection))
        .with_crossover_method(Crossover::VariableLength(AlignmentStrategy::Pad))
        .with_mutation_portfolio(mutations)
        .with_survivor_method(Survivor::Fitness)
        .with_problem_solving(ProblemSolving::Minimization)
        .with_threads(1)
        .with_max_generations(generations);
    if let Some(population) = population {
        builder = builder.with_population(population);
    }
    if let Some(penalty) = cfg.length_penalty {
        builder = builder.with_length_penalty(penalty);
    }
    if let Some(seed) = seed {
        builder = builder.with_rng_seed(seed);
    }
    builder.build().map_err(|e| e.to_string())
}

fn build_vl_recipe_a_ga(
    cfg: &VlRecipeAConfig,
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
) -> Result<Ga<LayerChromosome>, String> {
    build_vl_recipe_a_ga_with(
        cfg,
        decoder,
        template,
        evaluator,
        None,
        cfg.generations,
        cfg.seed,
        None,
    )
}

fn vl_initial_population(cfg: &VlRecipeAConfig, seed: Option<u64>) -> Population<LayerChromosome> {
    set_seed(seed);
    let mut rng = make_rng();
    let chromosomes = (0..cfg.population_size)
        .map(|_| {
            let width = rng.random_range(cfg.min_layers..=cfg.max_layers);
            let dna = (0..width).map(|_| KeyGene::random(&mut rng)).collect();
            LayerChromosome::from_dna(dna)
        })
        .collect();
    Population::new(chromosomes)
}

/// Run variable-length Recipe A from a mined seed/proposal bank.
///
/// This is the H7 fallback path: the first `seed_fraction` of the initial
/// population is filled from the supplied genomes (cycled if there are fewer
/// seeds than slots), and the remainder is random. It is deliberately a
/// seed-bank arm, not a diffusion model; the caller must record where the
/// seeds came from. Every seed must decode and fit the variable-length bounds
/// before the run starts.
pub fn run_recipe_a_vl_seeded(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &VlRecipeAConfig,
    seeds: &[Genome],
    seed_fraction: f64,
) -> Result<RecipeAResult, String> {
    validate_vl_recipe_a(cfg)?;
    template.check_grammar(decoder.pool(), decoder.grammar())?;
    if seeds.is_empty() {
        return Err("seeded Recipe A needs at least one seed".into());
    }
    if !seed_fraction.is_finite() || !(0.0..=1.0).contains(&seed_fraction) || seed_fraction == 0.0 {
        return Err("seeded Recipe A seed_fraction must be in (0, 1]".into());
    }
    for (index, seed) in seeds.iter().enumerate() {
        if seed.len() < cfg.min_layers || seed.len() > cfg.max_layers {
            return Err(format!(
                "seed {index} has {} layers, outside [{}, {}]",
                seed.len(),
                cfg.min_layers,
                cfg.max_layers
            ));
        }
        decoder
            .decode(seed)
            .map_err(|error| format!("seed {index}: {error}"))?;
    }

    let seeded_slots = ((cfg.population_size as f64) * seed_fraction).ceil() as usize;
    let seeded_slots = seeded_slots.clamp(1, cfg.population_size);
    set_seed(cfg.seed);
    let mut rng = make_rng();
    let mut chromosomes = Vec::with_capacity(cfg.population_size);
    for slot in 0..cfg.population_size {
        if slot < seeded_slots {
            let seed = &seeds[slot % seeds.len()];
            chromosomes.push(LayerChromosome::from_dna(genome_to_dna(seed)));
        } else {
            let width = rng.random_range(cfg.min_layers..=cfg.max_layers);
            let dna = (0..width).map(|_| KeyGene::random(&mut rng)).collect();
            chromosomes.push(LayerChromosome::from_dna(dna));
        }
    }

    let mut ga = build_vl_recipe_a_ga_with(
        cfg,
        decoder,
        template,
        evaluator,
        Some(Population::new(chromosomes)),
        cfg.generations,
        cfg.seed,
        None,
    )?;
    let best = ga.run().map_err(|e| e.to_string())?.best_chromosome.clone();
    let best_genome = best.to_genome();
    let decoded = decoder.decode(&best_genome).map_err(|e| e.to_string())?;
    let stack = checked_assemble(decoder, template, &decoded)?;
    let merit = evaluator.evaluate(&stack)?;
    Ok(RecipeAResult {
        best_genome,
        decoded,
        stack,
        merit,
        generations: cfg.generations,
    })
}

/// Gaussian proposal expansion for an H7 mined-seed bank.
///
/// This is a proposal generator, not a trained diffusion model: every seed is
/// retained, and `proposals_per_seed` noisy copies are added by perturbing the
/// material and thickness keys while preserving the seed's presence pattern.
/// The decoder still decides legality and presence, and the caller records the
/// seed provenance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProposalConfig {
    pub noise_sigma: f64,
    pub proposals_per_seed: usize,
}

impl Default for ProposalConfig {
    fn default() -> Self {
        Self {
            noise_sigma: 0.08,
            proposals_per_seed: 4,
        }
    }
}

impl ProposalConfig {
    fn validate(self) -> Result<(), String> {
        if !self.noise_sigma.is_finite() || self.noise_sigma < 0.0 {
            return Err("proposal noise_sigma must be finite and non-negative".into());
        }
        if self.proposals_per_seed == 0 {
            return Err("proposal needs at least one copy per seed".into());
        }
        Ok(())
    }
}

fn expand_seed_bank(
    seeds: &[Genome],
    cfg: &VlRecipeAConfig,
    proposal: ProposalConfig,
) -> Result<Vec<Genome>, String> {
    proposal.validate()?;
    if seeds.is_empty() {
        return Err("proposal expansion needs at least one seed".into());
    }
    let seed = cfg.seed.map(|base| base.wrapping_add(7919));
    set_seed(seed);
    let mut rng = make_rng();
    let mut bank = Vec::with_capacity(seeds.len() * (1 + proposal.proposals_per_seed));
    for seed_genome in seeds {
        bank.push(seed_genome.clone());
        for _ in 0..proposal.proposals_per_seed {
            let genes = seed_genome
                .genes()
                .iter()
                .map(|gene| {
                    LayerGene::new(
                        wrap01(gene.material + gaussian(&mut rng) * proposal.noise_sigma),
                        wrap01(gene.thickness + gaussian(&mut rng) * proposal.noise_sigma),
                        gene.presence,
                    )
                })
                .collect::<Vec<_>>();
            bank.push(Genome::from(genes));
        }
    }
    if bank
        .iter()
        .any(|genome| genome.len() < cfg.min_layers || genome.len() > cfg.max_layers)
    {
        return Err("proposal expansion produced a genome outside the VL bounds".into());
    }
    Ok(bank)
}

/// Run variable-length Recipe A from a mined seed bank plus Gaussian proposals.
///
/// The seed bank is expanded by [`ProposalConfig`] before the H7 seeded runner
/// sees it. This is a denoising-style proposal arm, not a learned diffusion
/// model; it gives the initial population more thickness/material diversity
/// without discarding the mined motifs.
pub fn run_recipe_a_vl_proposed(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &VlRecipeAConfig,
    seeds: &[Genome],
    seed_fraction: f64,
    proposal: ProposalConfig,
) -> Result<RecipeAResult, String> {
    validate_vl_recipe_a(cfg)?;
    let bank = expand_seed_bank(seeds, cfg, proposal)?;
    for (index, genome) in bank.iter().enumerate() {
        decoder
            .decode(genome)
            .map_err(|error| format!("proposal {index}: {error}"))?;
    }
    run_recipe_a_vl_seeded(decoder, template, evaluator, cfg, &bank, seed_fraction)
}

/// Run Recipe A with a true variable-length chromosome.
///
/// The GA samples lengths in `[min_layers, max_layers]`; its mutation
/// portfolio mixes Gaussian key moves with insertion and deletion. The
/// decoder still applies presence, grammar, and pinned-edge rules, so an
/// inserted gene is not automatically a present film.
pub fn run_recipe_a_vl(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &VlRecipeAConfig,
) -> Result<RecipeAResult, String> {
    validate_vl_recipe_a(cfg)?;
    template.check_grammar(decoder.pool(), decoder.grammar())?;
    let mut ga = build_vl_recipe_a_ga(cfg, decoder, template, evaluator)?;
    let best = ga.run().map_err(|e| e.to_string())?.best_chromosome.clone();
    let best_genome = best.to_genome();
    let decoded = decoder.decode(&best_genome).map_err(|e| e.to_string())?;
    let stack = checked_assemble(decoder, template, &decoded)?;
    let merit = evaluator.evaluate(&stack)?;
    Ok(RecipeAResult {
        best_genome,
        decoded,
        stack,
        merit,
        generations: cfg.generations,
    })
}

/// Run the true variable-length Recipe A and immediately apply its terminal
/// memetic finish.
pub fn run_recipe_a_vl_finished(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &mut FitnessEvaluator,
    cfg: &VlRecipeAConfig,
    operator: &MemeticOperator,
    max_insertions: usize,
) -> Result<(RecipeAResult, TerminalReport), String> {
    let result = run_recipe_a_vl(decoder, template, evaluator, cfg)?;
    finish_recipe_a(
        decoder,
        template,
        evaluator,
        result,
        operator,
        max_insertions,
    )
}

fn ramp_penalty(ramp: &ParsimonyRamp, generation: usize, generations: usize) -> f64 {
    if generations <= 1 {
        return ramp.end;
    }
    let t = generation as f64 / (generations - 1) as f64;
    ramp.start + (ramp.end - ramp.start) * t
}

/// Run variable-length Recipe A with a linear parsimony ramp.
///
/// Each generation is a one-generation GA advance over the previous
/// population, with `length_penalty` interpolated from `ramp.start` to
/// `ramp.end`. This is single-island; the ring remains
/// [`run_recipe_a_vl_islands`].
pub fn run_recipe_a_vl_ramp(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &VlRecipeAConfig,
    ramp: &ParsimonyRamp,
) -> Result<RecipeAResult, String> {
    validate_vl_recipe_a(cfg)?;
    if !ramp.start.is_finite() || !ramp.end.is_finite() || ramp.start < 0.0 || ramp.end < 0.0 {
        return Err("parsimony ramp coefficients must be finite and non-negative".into());
    }
    template.check_grammar(decoder.pool(), decoder.grammar())?;

    let mut population = vl_initial_population(cfg, cfg.seed);
    let mut best: Option<LayerChromosome> = None;
    for generation in 0..cfg.generations {
        let mut generation_cfg = cfg.clone();
        generation_cfg.length_penalty = Some(ramp_penalty(ramp, generation, cfg.generations));
        let seed = cfg
            .seed
            .map(|base| base.wrapping_add((generation as u64).wrapping_mul(1009).wrapping_add(7)));
        let mut ga = build_vl_recipe_a_ga_with(
            &generation_cfg,
            decoder,
            template,
            evaluator,
            Some(population.clone()),
            1,
            seed,
            None,
        )?;
        ga.run().map_err(|e| e.to_string())?;
        population = ga.population.clone();
        if let Some(candidate) = population
            .chromosomes
            .iter()
            .min_by(|a, b| a.fitness().total_cmp(&b.fitness()))
        {
            if best
                .as_ref()
                .is_none_or(|current| candidate.fitness() < current.fitness())
            {
                best = Some(candidate.clone());
            }
        }
    }

    let best = best.ok_or_else(|| "VL Recipe A produced an empty population".to_string())?;
    if !best.fitness().is_finite() {
        return Err("VL Recipe A found no legal design".into());
    }
    let best_genome = best.to_genome();
    let decoded = decoder.decode(&best_genome).map_err(|e| e.to_string())?;
    let stack = checked_assemble(decoder, template, &decoded)?;
    let merit = evaluator.evaluate(&stack)?;
    Ok(RecipeAResult {
        best_genome,
        decoded,
        stack,
        merit,
        generations: cfg.generations,
    })
}

/// Run true variable-length Recipe A on the Phase 2 ring.
///
/// The ring, migrant cap, and best-replace-worst-if-better rule are the same as
/// [`run_recipe_a_islands`]; the difference is that each island owns a
/// variable-length chromosome and a mutation portfolio that can insert or
/// delete genes. Migration is still host-independent: a legal variable-length
/// migrant is cloned into the destination and the next generation's crossover
/// pads/trims it as needed.
#[allow(clippy::too_many_arguments)]
fn run_recipe_a_vl_islands_with_strategies(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &VlRecipeAConfig,
    islands: &IslandRecipeConfig,
    strategies: Option<&[IslandStrategy]>,
    constraints: Option<&[TribeConstraint]>,
    i1: Option<&I1Config>,
) -> Result<IslandRecipeResult, String> {
    validate_vl_recipe_a(cfg)?;
    if let Some(strategies) = strategies {
        validate_island_strategies(strategies, islands.islands)?;
    }
    if let Some(constraints) = constraints {
        validate_tribe_constraints(constraints, islands.islands, decoder.pool().len())?;
    }
    if islands.islands < 2 {
        return Err("island Recipe A needs at least two islands".into());
    }
    if islands.migration_interval == 0 {
        return Err("island Recipe A migration_interval must be >= 1".into());
    }
    if islands.migration_count == 0 {
        return Err("island Recipe A migration_count must be >= 1".into());
    }
    if let Some(i1) = i1 {
        if i1.islands != islands.islands {
            return Err(format!(
                "I1: {} islands in the policy but {} in the experiment",
                islands.islands, i1.islands
            ));
        }
        if i1.islands < 3 || i1.parents < 2 || i1.parents >= i1.islands {
            return Err("I1 needs at least 3 islands and 2 <= parents < islands".into());
        }
    }
    template.check_grammar(decoder.pool(), decoder.grammar())?;

    let cap = ((cfg.population_size as f64) * 0.05).floor().max(1.0) as usize;
    let migrants_per_event = islands.migration_count.min(cap);
    let mut populations: Vec<Population<LayerChromosome>> = (0..islands.islands)
        .map(|island| {
            let seed = cfg
                .seed
                .map(|base| base.wrapping_add((island as u64).wrapping_mul(1009)));
            vl_initial_population(cfg, seed)
        })
        .collect();
    let mut migrations = 0;
    let mut crossovers = 0;

    for generation in 0..cfg.generations {
        for (island, population) in populations.iter_mut().enumerate() {
            let seed = cfg.seed.map(|base| {
                base.wrapping_add(
                    ((generation * islands.islands + island) as u64)
                        .wrapping_mul(1009)
                        .wrapping_add(7),
                )
            });
            let strategy = strategies.map(|all| &all[island]);
            let island_decoder = match constraints {
                Some(all) => tribe_decoder(decoder, &all[island])?,
                None => decoder.clone(),
            };
            let mut ga = build_vl_recipe_a_ga_with(
                cfg,
                &island_decoder,
                template,
                evaluator,
                Some(population.clone()),
                1,
                seed,
                strategy,
            )?;
            ga.run().map_err(|e| e.to_string())?;
            *population = ga.population.clone();
        }
        if (generation + 1) % islands.migration_interval == 0 {
            migrations += populations.len();
            match i1 {
                Some(i1) => {
                    crossovers += migrate_i1(
                        &mut populations,
                        i1.parents,
                        decoder,
                        template,
                        evaluator,
                        cfg.mutation_sigma,
                    );
                }
                None => {
                    migrate_islands(&mut populations, migrants_per_event);
                }
            }
        }
    }

    let homogenisation = island_divergence(&populations, decoder.pool().len());
    let best = populations
        .iter()
        .flat_map(|population| population.chromosomes.iter())
        .min_by(|a, b| a.fitness().total_cmp(&b.fitness()))
        .cloned()
        .ok_or_else(|| "island Recipe A produced an empty population".to_string())?;
    if !best.fitness().is_finite() {
        return Err("island Recipe A found no legal design".into());
    }
    let best_genome = best.to_genome();
    let decoded = decoder.decode(&best_genome).map_err(|e| e.to_string())?;
    let stack = checked_assemble(decoder, template, &decoded)?;
    let merit = evaluator.evaluate(&stack)?;
    let island_bests = populations
        .iter()
        .map(|population| {
            population
                .chromosomes
                .iter()
                .map(LayerChromosome::fitness)
                .min_by(|a, b| a.total_cmp(b))
                .unwrap_or(f64::INFINITY)
        })
        .collect();

    Ok(IslandRecipeResult {
        best: RecipeAResult {
            best_genome,
            decoded,
            stack,
            merit,
            generations: cfg.generations,
        },
        island_bests,
        migrations,
        migrants_per_event,
        crossovers,
        homogenisation,
        memetic: MemeticRunReport::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use navette::smatrix::angle::Radians;
    use navette::smatrix::synthesis::cycle::NeedleCycleConfig;
    use navette::smatrix::synthesis::merit::{
        ConstraintKind, CurveId, MeritKey, MeritSpec, MeritTarget, SimTransform,
    };
    use navette::smatrix::synthesis::pipeline::SpectralInputs;
    use navette::smatrix::synthesis::structure::LayerSpec;

    use crate::bridge::{BoundaryRules, EvaluatorOptions};
    use crate::decoder::{AdjacencyGrammar, Material, MaterialPool, Pins};

    fn constant(name: &str, n: f64, d_nm: f64) -> LayerSpec {
        LayerSpec::constant(name, n, 0.0, d_nm, 1)
    }

    fn zero_spec() -> MeritSpec {
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

    #[test]
    fn seeded_recipe_a_pads_a_two_layer_genome_to_the_cap() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions::default(),
        )
        .unwrap();
        let cfg = RecipeAConfig {
            population_size: 4,
            generations: 2,
            max_layers: 6,
            mutation_sigma: 0.2,
            seed: Some(5),
        };
        let seed = Genome::from(vec![
            LayerGene::new(0.25, 0.5, 1.0),
            LayerGene::new(0.75, 0.5, 1.0),
        ]);
        let population =
            initial_population_seeded(&cfg, Some(5), std::slice::from_ref(&seed)).unwrap();
        let genome = population.chromosomes[0].to_genome();
        assert_eq!(genome.len(), cfg.max_layers);
        assert_eq!(
            genome
                .genes()
                .iter()
                .filter(|gene| gene.presence >= 0.5)
                .count(),
            2
        );
        let result = run_recipe_a_seeded(&decoder, &template, &evaluator, &cfg, &[seed]).unwrap();
        assert!(result.merit.is_finite());
        assert_eq!(result.best_genome.len(), cfg.max_layers);
    }

    #[test]
    fn recipe_a_refuses_an_insert_grammar_conflict() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let grammar =
            AdjacencyGrammar::from_allowed(vec![vec![true, false], vec![true, true]]).unwrap();
        let decoder = Decoder::new(pool, Pins::none(), grammar).unwrap();
        let rules = BoundaryRules {
            insert: Some(
                [("H".to_string(), vec!["L".to_string()])]
                    .into_iter()
                    .collect(),
            ),
            ..BoundaryRules::default()
        };
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap()
        .with_boundary_rules(rules)
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions::default(),
        )
        .unwrap();

        let err =
            run_recipe_a(&decoder, &template, &evaluator, &RecipeAConfig::default()).unwrap_err();
        assert!(err.contains("insert/grammar conflict"), "{err}");
    }

    #[test]
    fn illuminated_recipe_a_fills_a_mome_grid() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions::default(),
        )
        .unwrap();
        let cfg = RecipeAConfig {
            population_size: 8,
            generations: 3,
            max_layers: 4,
            mutation_sigma: 0.2,
            seed: Some(5),
        };
        let archive_cfg = ArchiveConfig {
            depth_bins: 4,
            thickness_bins: 4,
            max_layers: cfg.max_layers as f64,
            max_thickness_nm: 800.0,
            max_front: 8,
        };
        let result =
            run_recipe_a_illuminated(&decoder, &template, &evaluator, &cfg, archive_cfg).unwrap();
        assert!(!result.archive.is_empty());
        assert!(result.archive.coverage() > 0.0);
        assert!(result.archive.best().is_some());
        assert!(result.best.merit.is_finite());
    }

    #[test]
    fn h0_control_runs_a_multi_start_refinement() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let mut evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions {
                clamp_min_nm: 20.0,
                clamp_max_nm: 150.0,
                ..EvaluatorOptions::default()
            },
        )
        .unwrap();
        let cfg = H0Config {
            starts: 3,
            max_layers: 3,
            mutation_sigma: 0.2,
            needle_steps: 0,
            seed: Some(3),
        };
        let result = run_h0_control(&decoder, &template, &mut evaluator, &cfg, None).unwrap();
        assert!(result.merit.is_finite());
        assert!(result.decoded.len() <= cfg.max_layers);
    }

    #[test]
    fn h0_and_ga_stop_at_the_same_solver_budget() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions {
                clamp_min_nm: 20.0,
                clamp_max_nm: 150.0,
                ..EvaluatorOptions::default()
            },
        )
        .unwrap();
        let h0 = H0Config {
            starts: 1,
            max_layers: 3,
            mutation_sigma: 0.2,
            needle_steps: 0,
            seed: Some(31),
        };
        let ga = RecipeAConfig {
            population_size: 8,
            generations: 1,
            max_layers: 3,
            mutation_sigma: 0.2,
            seed: Some(31),
        };
        let comparison =
            compare_h0_and_ga_at_budget(&decoder, &template, &evaluator, &h0, &ga, 100).unwrap();
        assert_eq!(comparison.budget, 100);
        assert!(comparison.h0.evaluations >= 100);
        assert!(comparison.ga.evaluations >= 100);
        assert!(comparison.h0.result.merit.is_finite());
        assert!(comparison.ga.result.merit.is_finite());
        assert!(comparison.ga.result.generations >= 1);
    }

    #[test]
    fn repeat_runner_keeps_one_arm_per_seed() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions {
                clamp_min_nm: 20.0,
                clamp_max_nm: 150.0,
                ..EvaluatorOptions::default()
            },
        )
        .unwrap();
        let h0 = H0Config {
            starts: 1,
            max_layers: 3,
            mutation_sigma: 0.2,
            needle_steps: 0,
            seed: Some(31),
        };
        let ga = RecipeAConfig {
            population_size: 8,
            generations: 1,
            max_layers: 3,
            mutation_sigma: 0.2,
            seed: Some(31),
        };
        let repeats =
            compare_h0_and_ga_repeats(&decoder, &template, &evaluator, &h0, &ga, &[31, 32], 100)
                .unwrap();
        assert_eq!(repeats.seeds, vec![31, 32]);
        assert_eq!(repeats.h0.len(), 2);
        assert_eq!(repeats.ga.len(), 2);
        assert!(repeats
            .h0
            .iter()
            .chain(&repeats.ga)
            .all(|arm| arm.result.merit.is_finite() && arm.evaluations >= repeats.budget));
    }

    #[test]
    fn recipe_a_runs_and_returns_a_legal_design() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions::default(),
        )
        .unwrap();
        let cfg = RecipeAConfig {
            population_size: 12,
            generations: 4,
            max_layers: 4,
            mutation_sigma: 0.2,
            seed: Some(7),
        };
        let result = run_recipe_a(&decoder, &template, &evaluator, &cfg).unwrap();

        assert!(result.decoded.len() <= cfg.max_layers);
        assert!(result.merit.is_finite(), "{result:?}");
        assert_eq!(evaluator.evaluate(&result.stack).unwrap(), result.merit);
    }

    #[test]
    fn i1_runs_cross_island_children_and_reports_divergence() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions::default(),
        )
        .unwrap();
        let cfg = RecipeAConfig {
            population_size: 9,
            generations: 3,
            max_layers: 3,
            mutation_sigma: 0.2,
            seed: Some(11),
        };
        let i1 = I1Config {
            islands: 3,
            migration_interval: 1,
            parents: 2,
        };
        let result = run_recipe_a_i1(&decoder, &template, &evaluator, &cfg, &i1).unwrap();
        assert_eq!(result.island_bests.len(), 3);
        assert_eq!(result.migrations, cfg.generations * 3);
        assert!(result.crossovers <= cfg.generations * 3);
        assert!(result.homogenisation.is_finite());
        assert!((0.0..=1.0).contains(&result.homogenisation));
        assert!(result.best.merit.is_finite());

        let bad = I1Config {
            islands: 2,
            migration_interval: 1,
            parents: 2,
        };
        assert!(run_recipe_a_i1(&decoder, &template, &evaluator, &cfg, &bad).is_err());
    }

    #[test]
    fn migration_only_replaces_a_worse_destination() {
        let chromosome = |fitness: f64| {
            let mut chromosome = LayerChromosome::from_dna(vec![KeyGene::new(0.0, 0.0, 0.0)]);
            chromosome.set_fitness(fitness);
            chromosome
        };

        let mut islands = vec![
            Population::new(vec![chromosome(1.0), chromosome(5.0)]),
            Population::new(vec![chromosome(2.0), chromosome(9.0)]),
        ];
        assert_eq!(migrate_islands(&mut islands, 1), 2);
        // Ring: 0 gets 1's best (2 < 5); 1 gets 0's best (1 < 9).
        assert_eq!(islands[0].chromosomes[1].fitness(), 2.0);
        assert_eq!(islands[1].chromosomes[1].fitness(), 1.0);

        let mut islands = vec![
            Population::new(vec![chromosome(1.0), chromosome(2.0)]),
            Population::new(vec![chromosome(3.0), chromosome(4.0)]),
        ];
        assert_eq!(migrate_islands(&mut islands, 1), 2);
        // 0's worst (2) is already better than 1's best (3), so it stays;
        // 1's worst (4) is replaced by 0's best (1).
        assert_eq!(islands[0].chromosomes[1].fitness(), 2.0);
        assert_eq!(islands[1].chromosomes[1].fitness(), 1.0);
    }

    #[test]
    fn island_recipe_a_runs_and_returns_a_legal_design() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions::default(),
        )
        .unwrap();
        let cfg = RecipeAConfig {
            population_size: 8,
            generations: 3,
            max_layers: 4,
            mutation_sigma: 0.2,
            seed: Some(13),
        };
        let policy = IslandRecipeConfig {
            islands: 2,
            migration_interval: 1,
            migration_count: 1,
        };
        let result = run_recipe_a_islands(&decoder, &template, &evaluator, &cfg, &policy).unwrap();

        assert_eq!(result.island_bests.len(), 2);
        assert_eq!(result.migrations, cfg.generations * policy.islands);
        assert_eq!(result.migrants_per_event, 1);
        assert!(result.island_bests.iter().all(|merit| merit.is_finite()));
        assert!(result.best.decoded.len() <= cfg.max_layers);
        assert!(result.best.merit.is_finite(), "{result:?}");
        assert_eq!(
            evaluator.evaluate(&result.best.stack).unwrap(),
            result.best.merit
        );
    }

    #[test]
    fn terminal_finish_writes_back_and_clears_the_scan_certificate() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 10.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let spec = zero_spec();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            spec.clone(),
            EvaluatorOptions {
                clamp_min_nm: 10.0,
                clamp_max_nm: 150.0,
                ..EvaluatorOptions::default()
            },
        )
        .unwrap();
        let cfg = RecipeAConfig {
            population_size: 8,
            generations: 3,
            max_layers: 20,
            mutation_sigma: 0.2,
            seed: Some(21),
        };
        let result = run_recipe_a(&decoder, &template, &evaluator, &cfg).unwrap();
        let spectral = SpectralInputs::from_spec(&spec, &[Radians::ZERO], &[550.0]).unwrap();
        let operator = MemeticOperator::new(
            spectral,
            navette::smatrix::synthesis::cycle::MaterialPool::new(),
            NeedleCycleConfig {
                max_needles: 4,
                scan_step_nm: 10.0,
                ..NeedleCycleConfig::default()
            },
        );
        let mut ev = evaluator;
        let (finished, report) =
            finish_recipe_a(&decoder, &template, &mut ev, result, &operator, 4).unwrap();

        assert_eq!(finished.best_genome.len(), decoder.max_layers());
        assert_eq!(finished.decoded.len(), finished.stack.films().len());
        assert_eq!(report.actual_improving_after, 0);
        assert_eq!(report.final_merit, finished.merit);
        assert_eq!(ev.evaluate(&finished.stack).unwrap(), finished.merit);
        assert!(report.min_thickness_nm >= 0.0);
    }

    #[test]
    fn surrogate_screen_mixes_true_and_predicted_evaluations() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions {
                clamp_min_nm: 20.0,
                clamp_max_nm: 150.0,
                ..EvaluatorOptions::default()
            },
        )
        .unwrap();
        let cfg = RecipeAConfig {
            population_size: 12,
            generations: 5,
            max_layers: 4,
            mutation_sigma: 0.2,
            seed: Some(51),
        };
        let result = run_recipe_a_surrogate(
            &decoder,
            &template,
            &evaluator,
            &cfg,
            SurrogateConfig {
                warmup: 4,
                sample_every: 2,
                refit_every: 2,
            },
        )
        .unwrap();
        assert!(result.best.merit.is_finite());
        assert!(result.report.predicted_evaluations > 0);
        assert!(result.report.true_evaluations > 0);
        assert!(result.report.model_fits > 0);
        assert_eq!(
            evaluator.evaluate(&result.best.stack).unwrap(),
            result.best.merit
        );
    }

    #[test]
    fn supplied_population_is_rebound_to_the_real_fitness_function() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions {
                clamp_min_nm: 10.0,
                clamp_max_nm: 150.0,
                ..EvaluatorOptions::default()
            },
        )
        .unwrap();
        let cfg = RecipeAConfig {
            population_size: 8,
            generations: 1,
            max_layers: 4,
            mutation_sigma: 0.2,
            seed: Some(26),
        };
        let population = initial_population(&cfg, cfg.seed);
        assert!(population
            .chromosomes
            .iter()
            .all(|chromosome| chromosome.fitness().is_nan()));

        let mut ga = build_recipe_a_ga(
            &cfg,
            &decoder,
            &template,
            &evaluator,
            Some(population),
            1,
            cfg.seed,
            None,
        )
        .unwrap();
        ga.run().unwrap();
        let best = ga
            .population
            .chromosomes
            .iter()
            .min_by(|a, b| a.fitness().total_cmp(&b.fitness()))
            .unwrap();
        let decoded = decoder.decode(&best.to_genome()).unwrap();
        let stack = checked_assemble(&decoder, &template, &decoded).unwrap();
        assert!(best.fitness().is_finite());
        assert!(best.fitness() > 0.0);
        assert_eq!(best.fitness(), evaluator.evaluate(&stack).unwrap());
    }

    #[test]
    fn mid_loop_memetic_schedule_runs_and_keeps_a_legal_best() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 10.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let spec = zero_spec();
        let mut evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            spec.clone(),
            EvaluatorOptions {
                clamp_min_nm: 10.0,
                clamp_max_nm: 150.0,
                ..EvaluatorOptions::default()
            },
        )
        .unwrap();
        let cfg = RecipeAConfig {
            population_size: 8,
            generations: 4,
            max_layers: 20,
            mutation_sigma: 0.2,
            seed: Some(24),
        };
        let spectral = SpectralInputs::from_spec(&spec, &[Radians::ZERO], &[550.0]).unwrap();
        let operator = MemeticOperator::new(
            spectral,
            navette::smatrix::synthesis::cycle::MaterialPool::new(),
            NeedleCycleConfig {
                max_needles: 2,
                scan_step_nm: 10.0,
                ..NeedleCycleConfig::default()
            },
        );
        let schedule = MemeticSchedule {
            interval: 2,
            top: 2,
            max_insertions: 1,
        };
        let result = run_recipe_a_memetic(
            &decoder,
            &template,
            &mut evaluator,
            &cfg,
            &operator,
            &schedule,
        )
        .unwrap();

        assert_eq!(result.report.passes, 2);
        assert!(result.report.refined > 0);
        assert!(result.report.improved <= result.report.refined);
        assert!((0.0..=1.0).contains(&result.report.memetic_share()));
        assert!(result.best.merit.is_finite());
        assert!(result.best.decoded.len() <= cfg.max_layers);
        assert_eq!(
            evaluator.evaluate(&result.best.stack).unwrap(),
            result.best.merit
        );

        let bad = MemeticSchedule {
            interval: 0,
            top: 1,
            max_insertions: 1,
        };
        assert!(
            run_recipe_a_memetic(&decoder, &template, &mut evaluator, &cfg, &operator, &bad)
                .is_err()
        );
    }

    #[test]
    fn island_memetic_schedule_runs_every_island() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 10.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let spec = zero_spec();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            spec.clone(),
            EvaluatorOptions {
                clamp_min_nm: 10.0,
                clamp_max_nm: 150.0,
                ..EvaluatorOptions::default()
            },
        )
        .unwrap();
        let cfg = RecipeAConfig {
            population_size: 8,
            generations: 4,
            max_layers: 20,
            mutation_sigma: 0.2,
            seed: Some(25),
        };
        let islands = IslandRecipeConfig {
            islands: 2,
            migration_interval: 2,
            migration_count: 1,
        };
        let spectral = SpectralInputs::from_spec(&spec, &[Radians::ZERO], &[550.0]).unwrap();
        let operator = MemeticOperator::new(
            spectral,
            navette::smatrix::synthesis::cycle::MaterialPool::new(),
            NeedleCycleConfig {
                max_needles: 2,
                scan_step_nm: 10.0,
                ..NeedleCycleConfig::default()
            },
        );
        let schedule = MemeticSchedule {
            interval: 2,
            top: 1,
            max_insertions: 1,
        };
        let result = run_recipe_a_islands_memetic(
            &decoder, &template, &evaluator, &cfg, &islands, &operator, &schedule,
        )
        .unwrap();

        assert_eq!(result.memetic.passes, islands.islands * 2);
        assert!(result.memetic.refined > 0);
        assert!((0.0..=1.0).contains(&result.memetic.memetic_share()));
        assert!(result.best.merit.is_finite());
        assert_eq!(
            evaluator.evaluate(&result.best.stack).unwrap(),
            result.best.merit
        );
    }

    #[test]
    fn terminal_finish_refuses_a_stack_outside_decoder_bounds() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let spec = zero_spec();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            spec.clone(),
            EvaluatorOptions::default(),
        )
        .unwrap();
        let cfg = RecipeAConfig {
            population_size: 8,
            generations: 3,
            max_layers: 4,
            mutation_sigma: 0.2,
            seed: Some(23),
        };
        let mut result = run_recipe_a(&decoder, &template, &evaluator, &cfg).unwrap();
        assert!(!result.stack.films().is_empty());
        result.stack.set_thickness(0, 400.0).unwrap();

        let spectral = SpectralInputs::from_spec(&spec, &[Radians::ZERO], &[550.0]).unwrap();
        let operator = MemeticOperator::new(
            spectral,
            navette::smatrix::synthesis::cycle::MaterialPool::new(),
            NeedleCycleConfig {
                max_needles: 1,
                scan_step_nm: 10.0,
                ..NeedleCycleConfig::default()
            },
        );
        let mut ev = evaluator;
        let error =
            finish_recipe_a(&decoder, &template, &mut ev, result, &operator, 1).unwrap_err();
        assert!(error.contains("A4 stack check"), "{error}");
    }

    #[test]
    fn vl_seeded_recipe_runs_from_a_mined_seed_bank() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions {
                clamp_min_nm: 20.0,
                clamp_max_nm: 150.0,
                ..EvaluatorOptions::default()
            },
        )
        .unwrap();
        let cfg = VlRecipeAConfig {
            population_size: 6,
            generations: 2,
            min_layers: 1,
            max_layers: 4,
            mutation_sigma: 0.2,
            length_penalty: None,
            seed: Some(41),
        };
        let seed = Genome::from(vec![
            LayerGene::new(0.25, 0.5, 1.0),
            LayerGene::new(0.75, 0.5, 1.0),
        ]);
        let result = run_recipe_a_vl_seeded(
            &decoder,
            &template,
            &evaluator,
            &cfg,
            std::slice::from_ref(&seed),
            0.5,
        )
        .unwrap();
        assert!(result.merit.is_finite());
        assert!(result.decoded.len() <= cfg.max_layers);

        let proposed = run_recipe_a_vl_proposed(
            &decoder,
            &template,
            &evaluator,
            &cfg,
            std::slice::from_ref(&seed),
            0.75,
            ProposalConfig {
                noise_sigma: 0.1,
                proposals_per_seed: 2,
            },
        )
        .unwrap();
        assert!(proposed.merit.is_finite());
        assert!(proposed.decoded.len() <= cfg.max_layers);
        assert!(run_recipe_a_vl_proposed(
            &decoder,
            &template,
            &evaluator,
            &cfg,
            &[],
            0.5,
            ProposalConfig::default(),
        )
        .is_err());

        let long = Genome::from(vec![LayerGene::new(0.25, 0.5, 1.0); 5]);
        assert!(
            run_recipe_a_vl_seeded(&decoder, &template, &evaluator, &cfg, &[long], 0.5).is_err()
        );
        assert!(run_recipe_a_vl_seeded(&decoder, &template, &evaluator, &cfg, &[], 0.5).is_err());
    }

    #[test]
    fn vl_recipe_a_runs_with_insertion_and_deletion_enabled() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions::default(),
        )
        .unwrap();
        let cfg = VlRecipeAConfig {
            population_size: 8,
            generations: 4,
            min_layers: 1,
            max_layers: 8,
            mutation_sigma: 0.2,
            length_penalty: None,
            seed: Some(31),
        };
        let result = run_recipe_a_vl(&decoder, &template, &evaluator, &cfg).unwrap();

        assert!((cfg.min_layers..=cfg.max_layers).contains(&result.best_genome.len()));
        assert!(result.decoded.len() <= cfg.max_layers);
        assert!(result.merit.is_finite(), "{result:?}");
        assert_eq!(evaluator.evaluate(&result.stack).unwrap(), result.merit);
    }

    #[test]
    fn vl_recipe_a_length_penalty_runs() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions::default(),
        )
        .unwrap();
        let cfg = VlRecipeAConfig {
            population_size: 8,
            generations: 10,
            min_layers: 1,
            max_layers: 8,
            mutation_sigma: 0.2,
            length_penalty: Some(1.0e6),
            seed: Some(41),
        };
        let result = run_recipe_a_vl(&decoder, &template, &evaluator, &cfg).unwrap();

        assert!(result.best_genome.len() <= cfg.max_layers);
        assert!(result.merit.is_finite());
    }

    #[test]
    fn vl_island_recipe_a_migrates_variable_length_genomes() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions::default(),
        )
        .unwrap();
        let cfg = VlRecipeAConfig {
            population_size: 6,
            generations: 2,
            min_layers: 1,
            max_layers: 6,
            mutation_sigma: 0.2,
            length_penalty: None,
            seed: Some(51),
        };
        let policy = IslandRecipeConfig {
            islands: 2,
            migration_interval: 1,
            migration_count: 1,
        };
        let result =
            run_recipe_a_vl_islands(&decoder, &template, &evaluator, &cfg, &policy).unwrap();

        assert_eq!(result.island_bests.len(), 2);
        assert_eq!(result.migrations, cfg.generations * policy.islands);
        assert!(result.best.merit.is_finite());
        assert!((cfg.min_layers..=cfg.max_layers).contains(&result.best.best_genome.len()));
    }

    #[test]
    fn heterogeneous_islands_run_with_per_island_strategies() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions::default(),
        )
        .unwrap();
        let cfg = RecipeAConfig {
            population_size: 8,
            generations: 2,
            max_layers: 4,
            mutation_sigma: 0.2,
            seed: Some(61),
        };
        let heterogeneous = HeterogeneousIslandConfig {
            islands: IslandRecipeConfig {
                islands: 2,
                migration_interval: 1,
                migration_count: 1,
            },
            strategies: vec![
                IslandStrategy {
                    mutation_sigma: 0.1,
                    selection: Selection::Tournament,
                },
                IslandStrategy {
                    mutation_sigma: 0.4,
                    selection: Selection::Rank,
                },
            ],
        };
        let result = run_recipe_a_heterogeneous_islands(
            &decoder,
            &template,
            &evaluator,
            &cfg,
            &heterogeneous,
        )
        .unwrap();

        assert_eq!(result.island_bests.len(), 2);
        assert_eq!(result.migrations, cfg.generations * 2);
        assert!(result.best.merit.is_finite());
    }

    #[test]
    fn tribe_recipe_a_runs_with_per_island_constraints() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions::default(),
        )
        .unwrap();
        let cfg = RecipeAConfig {
            population_size: 6,
            generations: 2,
            max_layers: 4,
            mutation_sigma: 0.2,
            seed: Some(91),
        };
        let tribes = HeterogeneousTribeConfig {
            islands: IslandRecipeConfig {
                islands: 2,
                migration_interval: 1,
                migration_count: 1,
            },
            strategies: vec![
                IslandStrategy {
                    mutation_sigma: 0.1,
                    selection: Selection::Tournament,
                },
                IslandStrategy {
                    mutation_sigma: 0.4,
                    selection: Selection::Rank,
                },
            ],
            constraints: vec![
                TribeConstraint {
                    presence_threshold: Some(0.2),
                    grammar: None,
                },
                TribeConstraint {
                    presence_threshold: Some(0.9),
                    grammar: Some(AdjacencyGrammar::all_allowed(2)),
                },
            ],
        };
        let result = run_recipe_a_tribes(&decoder, &template, &evaluator, &cfg, &tribes).unwrap();

        assert_eq!(result.island_bests.len(), 2);
        assert_eq!(result.migrations, cfg.generations * tribes.islands.islands);
        assert!(result.best.merit.is_finite());
        assert!((1..=cfg.max_layers).contains(&result.best.decoded.len()));

        let strict = tribe_decoder(&decoder, &tribes.constraints[1]).unwrap();
        let genome = Genome::from(vec![LayerGene::new(0.5, 0.5, 0.5)]);
        assert_eq!(decoder.decode(&genome).unwrap().len(), 1);
        assert_eq!(strict.decode(&genome).unwrap().len(), 0);
    }

    #[test]
    fn vl_heterogeneous_islands_run_with_per_island_strategies() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions::default(),
        )
        .unwrap();
        let cfg = VlRecipeAConfig {
            population_size: 6,
            generations: 2,
            min_layers: 1,
            max_layers: 6,
            mutation_sigma: 0.2,
            length_penalty: None,
            seed: Some(71),
        };
        let heterogeneous = HeterogeneousIslandConfig {
            islands: IslandRecipeConfig {
                islands: 2,
                migration_interval: 1,
                migration_count: 1,
            },
            strategies: vec![
                IslandStrategy {
                    mutation_sigma: 0.1,
                    selection: Selection::Tournament,
                },
                IslandStrategy {
                    mutation_sigma: 0.4,
                    selection: Selection::Rank,
                },
            ],
        };
        let result = run_recipe_a_vl_heterogeneous_islands(
            &decoder,
            &template,
            &evaluator,
            &cfg,
            &heterogeneous,
        )
        .unwrap();

        assert_eq!(result.island_bests.len(), 2);
        assert!(result.best.merit.is_finite());
        assert!((cfg.min_layers..=cfg.max_layers).contains(&result.best.best_genome.len()));
    }

    #[test]
    fn vl_tribe_recipe_a_runs_with_per_island_constraints() {
        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions::default(),
        )
        .unwrap();
        let cfg = VlRecipeAConfig {
            population_size: 6,
            generations: 2,
            min_layers: 1,
            max_layers: 6,
            mutation_sigma: 0.2,
            length_penalty: None,
            seed: Some(101),
        };
        let tribes = HeterogeneousTribeConfig {
            islands: IslandRecipeConfig {
                islands: 2,
                migration_interval: 1,
                migration_count: 1,
            },
            strategies: vec![
                IslandStrategy {
                    mutation_sigma: 0.1,
                    selection: Selection::Tournament,
                },
                IslandStrategy {
                    mutation_sigma: 0.4,
                    selection: Selection::Rank,
                },
            ],
            constraints: vec![
                TribeConstraint {
                    presence_threshold: Some(0.2),
                    grammar: None,
                },
                TribeConstraint {
                    presence_threshold: Some(0.95),
                    grammar: None,
                },
            ],
        };
        let result =
            run_recipe_a_vl_tribes(&decoder, &template, &evaluator, &cfg, &tribes).unwrap();

        assert_eq!(result.island_bests.len(), 2);
        assert_eq!(result.migrations, cfg.generations * tribes.islands.islands);
        assert!(result.best.merit.is_finite());
        assert!((cfg.min_layers..=cfg.max_layers).contains(&result.best.best_genome.len()));
    }

    #[test]
    fn parsimony_ramp_interpolates_and_runs() {
        assert_eq!(
            ramp_penalty(&ParsimonyRamp::new(0.0, 10.0).unwrap(), 0, 5),
            0.0
        );
        assert_eq!(
            ramp_penalty(&ParsimonyRamp::new(0.0, 10.0).unwrap(), 4, 5),
            10.0
        );
        assert!((ramp_penalty(&ParsimonyRamp::new(0.0, 10.0).unwrap(), 2, 5) - 5.0).abs() < 1e-12);
        assert_eq!(
            ramp_penalty(&ParsimonyRamp::new(0.0, 10.0).unwrap(), 0, 1),
            10.0
        );
        assert!(ParsimonyRamp::new(-1.0, 2.0).is_err());

        let pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 200.0).unwrap(),
            Material::new("L", 10.0, 150.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0),
            constant("substrate", 1.52, 0.0),
            vec![constant("H", 2.2, 0.0), constant("L", 1.46, 0.0)],
        )
        .unwrap();
        let evaluator = FitnessEvaluator::new(
            vec![550.0],
            &[0.0],
            zero_spec(),
            EvaluatorOptions::default(),
        )
        .unwrap();
        let cfg = VlRecipeAConfig {
            population_size: 6,
            generations: 5,
            min_layers: 1,
            max_layers: 8,
            mutation_sigma: 0.2,
            length_penalty: None,
            seed: Some(81),
        };
        let result = run_recipe_a_vl_ramp(
            &decoder,
            &template,
            &evaluator,
            &cfg,
            &ParsimonyRamp::new(0.0, 1.0e6).unwrap(),
        )
        .unwrap();
        assert!(result.merit.is_finite());
        assert!((cfg.min_layers..=cfg.max_layers).contains(&result.best_genome.len()));
    }
}
