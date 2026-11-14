// SPDX-License-Identifier: LGPL-3.0-or-later
//! S1–S3 — evolutionary engines and assembly backbone.
//!
//! Starts with the single-island Recipe A path on `genetic_algorithms` 3.0:
//! a fixed-width chromosome of three random keys per layer, decoded by
//! [`crate::decoder`] and evaluated by [`crate::bridge`]. Presence keys make
//! the decoded stack variable-length even though the engine's DNA is fixed
//! width; insertion/deletion mutation and the island driver follow in the
//! Phase 2/3 work.
//!
//! The `genevo` 0.7 Simulation/Operator/Population traits are adopted when
//! the E09 v2 §1.1 copy gate opens. Both copies are Apache-2.0 /
//! MIT-Apache and are recorded in `CREDIT.md` at the moment a file moves in.

use std::borrow::Cow;

use genetic_algorithms::chromosomes::ChromosomeLength;
use genetic_algorithms::configuration::ProblemSolving;
use genetic_algorithms::fitness::FitnessFnWrapper;
use genetic_algorithms::ga::Ga;
use genetic_algorithms::operations::{Crossover, GaussianParams, Mutation, Selection, Survivor};
use genetic_algorithms::rng::make_rng;
use genetic_algorithms::traits::{
    ChromosomeT, ConfigurationT, CrossoverConfig, GeneT, LinearChromosome, MutationConfig,
    OperatorCompat, RealValuedMutation, SelectionConfig, StoppingConfig,
};
use rand::Rng;

use crate::bridge::{FitnessEvaluator, StackTemplate};
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

/// Best design from one Recipe A run.
#[derive(Clone, Debug)]
pub struct RecipeAResult {
    pub best_genome: Genome,
    pub decoded: DecodedStack,
    pub stack: navette::smatrix::synthesis::structure::DesignStack,
    pub merit: f64,
    pub generations: usize,
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
    if cfg.population_size < 2 {
        return Err("Recipe A needs a population of at least 2".into());
    }
    if cfg.max_layers == 0 {
        return Err("Recipe A needs max_layers >= 1".into());
    }
    if !cfg.mutation_sigma.is_finite() || cfg.mutation_sigma <= 0.0 {
        return Err("Recipe A mutation_sigma must be finite and positive".into());
    }
    template.check_grammar(decoder.pool(), decoder.grammar())?;

    let fit_decoder = decoder.clone();
    let fit_template = template.clone();
    let fit_evaluator = evaluator.clone();
    let fitness_fn = move |dna: &[KeyGene]| -> f64 {
        let genome = Genome::from(
            dna.iter()
                .map(|gene| LayerGene::new(gene.material, gene.thickness, gene.presence))
                .collect::<Vec<_>>(),
        );
        let Ok(decoded) = fit_decoder.decode(&genome) else {
            return f64::INFINITY;
        };
        let Ok(stack) = fit_template.assemble(&decoded) else {
            return f64::INFINITY;
        };
        fit_evaluator.evaluate(&stack).unwrap_or(f64::INFINITY)
    };

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
        .with_fitness_fn(fitness_fn)
        .with_selection_method(Selection::Tournament)
        .with_crossover_method(Crossover::Uniform)
        .with_mutation_method(Mutation::Gaussian(GaussianParams {
            sigma: Some(cfg.mutation_sigma),
        }))
        .with_survivor_method(Survivor::Fitness)
        .with_problem_solving(ProblemSolving::Minimization)
        .with_max_generations(cfg.generations);
    if let Some(seed) = cfg.seed {
        builder = builder.with_rng_seed(seed);
    }

    let mut ga = builder.build().map_err(|e| e.to_string())?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use navette::smatrix::angle::Radians;
    use navette::smatrix::synthesis::merit::{
        ConstraintKind, CurveId, MeritKey, MeritSpec, MeritTarget, SimTransform,
    };
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
}
