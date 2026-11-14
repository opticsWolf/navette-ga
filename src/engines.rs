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
use crate::decoder::{DecodedStack, Decoder, Genome, LayerGene};

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

fn recipe_a_fitness(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
) -> impl Fn(&[KeyGene]) -> f64 + Send + Sync + 'static {
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
        let Ok(stack) = template.assemble(&decoded) else {
            return f64::INFINITY;
        };
        evaluator.evaluate(&stack).unwrap_or(f64::INFINITY)
    }
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
    let init_width = cfg.max_layers;
    let init_fn = move |genes: usize, _alleles: Option<&[KeyGene]>| -> Vec<KeyGene> {
        let mut rng = make_rng();
        let width = if genes == 0 { init_width } else { genes };
        (0..width).map(|_| KeyGene::random(&mut rng)).collect()
    };

    let mut builder = Ga::<LayerChromosome>::new()
        .with_chromosome_length(ChromosomeLength::Fixed(cfg.max_layers))
        .with_population_size(cfg.population_size)
        .with_initialization_fn(init_fn)
        .with_fitness_fn(recipe_a_fitness(decoder, template, evaluator))
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
    let stack = template.assemble(&decoded)?;
    let merit = evaluator.evaluate(&stack)?;

    Ok(RecipeAResult {
        best_genome,
        decoded,
        stack,
        merit,
        generations: cfg.generations,
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
fn run_recipe_a_islands_with_strategies(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &RecipeAConfig,
    islands: &IslandRecipeConfig,
    strategies: Option<&[IslandStrategy]>,
) -> Result<IslandRecipeResult, String> {
    validate_recipe_a(cfg)?;
    if let Some(strategies) = strategies {
        validate_island_strategies(strategies, islands.islands)?;
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
            let mut ga = build_recipe_a_ga(
                cfg,
                decoder,
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
            migrations += migrate_islands(&mut populations, migrants_per_event);
        }
    }

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
    let stack = template.assemble(&decoded)?;
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
    })
}

/// Count final-scan candidates and the subset that actually improves the
/// merit when inserted and evaluated. The analytic P sweep is a predictor;
/// the certificate is the trial score.
fn certificate_scan(
    evaluator: &FitnessEvaluator,
    stack: &navette::smatrix::synthesis::structure::DesignStack,
    operator: &MemeticOperator,
) -> Result<(usize, usize), String> {
    let scan = operator.scan(evaluator, stack)?;
    let current = evaluator.evaluate(stack)?;
    let mut actual = 0usize;
    for candidate in &scan.candidates {
        let mut trial = stack.clone();
        trial.insert_needle_candidate(&operator.pool, candidate)?;
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
    let initial_merit = evaluator.evaluate(&result.stack)?;
    evaluator.refine_report(&mut result.stack)?;
    let memetic = operator.apply(evaluator, &mut result.stack, max_insertions)?;
    let (_, lm) = evaluator.refine_report(&mut result.stack)?;
    let lm_converged = lm
        .as_ref()
        .is_none_or(|report| report.termination.is_converged());
    let (scan_candidates_after, actual_improving_after) =
        certificate_scan(evaluator, &result.stack, operator)?;
    if actual_improving_after > 0 {
        return Err(format!(
            "terminal scan certificate failed: {actual_improving_after} of              {scan_candidates_after} scanned candidate(s) still improve the merit              after {} insertion(s)",
            memetic.steps.len()
        ));
    }

    let width = decoder.max_layers().max(result.best_genome.len());
    let films = result
        .stack
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
    let best_genome = decoder
        .encode_films(&films, width)
        .map_err(|error| error.to_string())?;
    let decoded = decoder.decode(&best_genome).map_err(|e| e.to_string())?;
    let stack = template.assemble(&decoded)?;
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

/// Run Recipe A on the ring with one operator regime per island.
pub fn run_recipe_a_islands(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &RecipeAConfig,
    islands: &IslandRecipeConfig,
) -> Result<IslandRecipeResult, String> {
    run_recipe_a_islands_with_strategies(decoder, template, evaluator, cfg, islands, None)
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
    run_recipe_a_vl_islands_with_strategies(decoder, template, evaluator, cfg, islands, None)
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

    let mut builder = Ga::<LayerChromosome>::new()
        .with_chromosome_length(ChromosomeLength::Variable {
            min: cfg.min_layers,
            max: cfg.max_layers,
        })
        .with_population_size(cfg.population_size)
        .with_initialization_fn(init_fn)
        .with_fitness_fn(recipe_a_fitness(decoder, template, evaluator))
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
    let stack = template.assemble(&decoded)?;
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
    let stack = template.assemble(&decoded)?;
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
fn run_recipe_a_vl_islands_with_strategies(
    decoder: &Decoder,
    template: &StackTemplate,
    evaluator: &FitnessEvaluator,
    cfg: &VlRecipeAConfig,
    islands: &IslandRecipeConfig,
    strategies: Option<&[IslandStrategy]>,
) -> Result<IslandRecipeResult, String> {
    validate_vl_recipe_a(cfg)?;
    if let Some(strategies) = strategies {
        validate_island_strategies(strategies, islands.islands)?;
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
            let mut ga = build_vl_recipe_a_ga_with(
                cfg,
                decoder,
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
            migrations += migrate_islands(&mut populations, migrants_per_event);
        }
    }

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
    let stack = template.assemble(&decoded)?;
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
