# Rust evolutionary crates — implementation details (E02)

Deep-dive companion to E01, sourced from upstream READMEs + docs.rs (2026-11-08).
Crate choice for the thin-film stack synthesiser is §8.

## 1. `genetic_algorithms` v3.0.0 (leimbernon) — the full platform

17 engines, generic over chromosome/gene types, rayon-parallel, Apache-2.0,
Rust ≥ 1.81, WASM-clean (CI-gated, rayon falls back to sequential).

| Engine | Module | Use for |
|---|---|---|
| `Ga<U>` | `ga` | standard single-pop GA, full operator support, adaptive mode, elitism |
| `IslandGa<U>` | `island` | island model, configurable migration topology / frequency / migrant selection |
| `DeEngine<U>` | `de` | Differential Evolution, 5 mutation strategies + JADE / L-SHADE self-tuning |
| `CmaEngine<U>` | `cma` | CMA-ES, Hansen defaults, Jacobi eigendecomposition (no LAPACK), IPOP/BIPOP restarts |
| `PsoEngine<U>` | `pso` | PSO with inertia schedules (const / linear-decay / random), global / ring / VonNeumann topology |
| `EdaEngine<U>` / `EdaRealEngine<U>` | `eda` | UMDA Bernoulli (binary) / Gaussian (continuous), no crossover-mutation |
| `ScatterEngine<U>` | `scatter` | Scatter Search with reference-set diversification |
| `CellularEngine<U>` | `cellular` | cellular GA on 2-D toroidal grid, 4 neighbourhoods |
| `AlpsEngine<U>` | `alps` | age-layered population structure, 3 age schemes, cross-layer mating |
| `HillClimbEngine<U>` | `hill_climb` | stochastic / steepest-ascent local-search baseline |
| `PermutateEngine<U>` | `permutate` | exhaustive permutation baseline (safety-gated) |
| `GpGa<N>` | `gp` | tree GP, subtree/point/hoist mutation, ramped half-and-half, MathNode/BoolNode sets |
| `Nsga2Ga<U>` | `nsga2` | standard 2–3 objective MOO |
| `Nsga3Ga<U>` | `nsga3` | reference-point many-objective (4+) |
| `MoeaDGa<U>` | `moead` | decomposition scalarisation (Tchebycheff / PBI / weighted sum) |
| `Spea2Ga<U>` | `spea2` | strength-Pareto archive + k-NN density |
| `SmsEmoaGa<U>` / `IbeaGa<U>` | `sms_emoa` / `ibea` | hypervolume-contribution / indicator-driven selection |

Operator catalogue. Selection: Random, RouletteWheel, SUS, Tournament, Rank,
Boltzmann, Truncation, Clearing, Lexicase, EpsilonLexicase. Crossover: Cycle,
MultiPoint, Uniform, SinglePoint, OX, PMX, SBX, BLX-α, Arithmetic, Clone,
Rejuvenate, EdgeRecombination, Undx, Spx, Pcx, plus VariableLength(Trim|Pad).
Mutation: Swap, Inversion, Scramble, Value, BitFlip, Creep, Gaussian, Polynomial,
NonUniform, PermutationInsert, Cauchy, LevyFlight, Uniform, ListValue, Differential,
SelfAdaptiveGaussian — plus **Insertion / Deletion** (grow/shrink by one gene).
Survivor: Fitness, Age, MuPlusLambda, MuCommaLambda, DeterministicCrowding.
Diversity extensions: MassExtinction / MassGenesis / MassDegeneration /
MassDeduplication. Adaptive GA recomputes crossover/mutation rates per pair from
relative fitness; AOS strategies (probability matching, adaptive pursuit, bandit)
select operators by credit.

Variable-length support (the thin-film hook). `ChromosomeLength::Variable {min,max}`
arms Insertion/Deletion; `Crossover::VariableLength(Trim|Pad)` aligns parents of
unequal length; `.with_length_penalty(λ)` applies parsimony pressure at survivor
selection — exactly the dynamic-parsimony-penalty slot of the planned architecture.
Genotypes: Binary, Range<T> (per-gene bounds via MultiRangeChromosome), List<T>
symbolic alphabet (material choice), permutation-safe UniqueChromosome.

Memetic slot. `LocalSearchOperator` wraps any engine in Lamarckian (write back)
or Baldwinian (fitness-only) local search — the needle/gradient refinement step.
`BatchFitnessEvaluator` amortises batched merit calls (one TMM sweep per
generation); `SurrogateModel` supports surrogate-assisted search for expensive
objectives. Constraint handling: static / dynamic / adaptive / death penalties +
feasibility rules (manufacturability: min thickness, allowed materials).
HallOfFame elite archive (dedup + min-distance diversity). Observer system:
13 lifecycle hooks (incl. migration hooks for IslandGa, Pareto hooks for
NSGA-II/III, restart hook for CMA-ES) with Log/Composite/Metrics/Tracing
implementations — usable as the run telemetry bus.
Benchmarks feature: Sphere/Rastrigin/Ackley, ZDT1–6, DTLZ1–7, hypervolume/GD/IGD/
spread indicators. `visualization` feature: PNG/SVG fitness/diversity/Pareto plots.

## 2. `nsga_rs` (ex-`optirustic`) — dedicated MOEA

The upstream rename flagged in E01 §5 has happened: the repo is now **NSGA RS**,
crate `nsga_rs`, same author. NSGA2 (Deb 2002), NSGA3 (Deb–Jain 2014),
AdaptiveNSGA3 (Jain 2014); minimise/maximise any objective count, constrained or
not; real/integer/boolean/choice variables; rayon evaluation; JSON history
export + resume-from-file; Python companion for Pareto plotting. Use only when
the merit is a genuine Pareto problem; no single-objective or island machinery.

## 3. `metaheuristics-nature` v10.1.0 — benchmarking harness

RGA / DE / PSO / Firefly / TLBO behind one `Solver::build(algorithm, objective)`
API with per-method defaults — swap methods by changing one struct. Reproducible
seeded RNG in single- and multi-threaded (rayon) runs; Pareto container keeps
history-best set for MOO; callback-per-generation telemetry; `no_std`/`libm`
friendly. Docs explicitly recommend gradient methods (e.g. OSQP) when the
derivative is known — the analogue of our gradient/needle refinement note.
Role: 50-line comparative baselines on fixed-length merit landscapes.

## 4. `radiate` v1.3.2 — Rust+Python EC framework

One pipeline from `Vec<char>` string-matching to NEAT graph evolution and
symbolic-regression trees. Single- and multi-objective GA, opt-in NEAT
neuroevolution (graph genomes), tree GP, opt-in speciation (diversity), novelty
search, first-class metric tracking, Dynamic Expression DSL for adaptive rates /
custom metrics, parallelism built in. Rust core ships a Python package too
(`pip install radiate`), so merit prototyping can start in Python and move to
Rust. Builder API (`GeneticEngine::builder().codec(…).fitness_fn(…).build()`),
codecs incl. Char/Bit/Float. Tribe-adjacent: species grouping is the closest
ready-made analogue of CTCM tribes (D11) among surveyed crates.

## 5. `easy_ga` v1.2.1 — minimal loop

`Gene` trait (user implements init/calculate_fitness/crossover/mutate;
requires Clone+Copy), `GeneticAlgorithm` builder (population/iterations/
mutation/selection rates, fitness goal), Roulette/Tournament/Random/Stochastic
selection, run-to-end or generation-stepped execution (allows mid-run
intervention), verbosity-graded logger with file save. MPL-2.0, explicitly a
spare-time side project; roadmap items (multithreading, CSV/JSON export) open.
Role: fixed-length prototype scaffold only.

## 6. `rlevo` v0.6 — RL + evolution on Burn

Deep RL (DQN/C51/QR-DQN, PPO/PPG, DDPG/TD3/SAC with entropy tuning) and classical
EAs (real/binary GA, ES incl. CMA-ES/CMSA-ES, EP, DE, CGP/GEP, UMDA-style and
Bayesian EDAs, compact GA) plus swarms (PSO, ACO, Firefly, Cuckoo, Bat, GWO,
ABC, WOA, Salp), NEAT / weight-evolution / NAS neuroevolution, competitive and
cooperative co-evolution with hall of fame, and a **memetic wrapper + local
search** (hill climbing, Nelder–Mead, restarts, SA) composable with any
population strategy. Const-generic `Observation<OR>`/`Action<AR>` dimensions
(type-level mismatch errors); Burn tensors throughout with feature-selected
backends (Flex CPU, wgpu GPU); typed versioned `EpisodeRecord` run provenance
with static-HTML replay reports; `rlevo-hybrid` evolution-guided policy loop
(RolloutFitness + EvolutionaryLoop). Adopt only if the RL-constructor + EA
hybrid is on the roadmap — Burn is a heavy dependency for a pure stack optimiser.

## 7. `Evo-Engine` v1.3.0 — detail sheet (from E01)

Fluent `EvolutionLauncher` builder (`population_size → evaluate_with →
selection → crossover → mutation → epochs → bounds → launch`), ask/tell API,
`IslandModel` (generational + adaptive K-means island assignment),
hybrid CPU-thread/GPU backends, GA/PSO/DE/ES, MIT/Apache-2.0. The island model
with adaptive K-means assignment is the nearest ready-made match to the D-track
dynamic-population literature (D15/D16/D03) among lightweight crates.

## 8. Crate choice for the thin-film synthesiser

`genetic_algorithms` covers every architecture slot (variable-length +
parsimony penalty + IslandGa + memetic local search + BatchFitnessEvaluator for
TMM sweeps + NSGA-III/MOEA-D if the merit goes Pareto) and is the default
choice. Pair with `metaheuristics-nature` for cheap baselines and `nsga_rs` if
strict Pareto work splits off. `radiate` (species) and `Evo-Engine`
(adaptive K-means islands) are the fallback islands/tribes hooks. `rlevo`
enters only with an RL-constructor stage; `easy_ga` only for throwaway
prototypes. Bespoke Rayon+TMM loop remains the latency ceiling play.
