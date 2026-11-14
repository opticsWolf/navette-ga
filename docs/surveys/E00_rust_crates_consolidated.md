# E00 — Rust evolutionary-optimisation crates: consolidated survey (E01–E04)

*Consolidates `E01_rust_ea_crates.md`, `E02_rust_ea_details.md`,
`E03_rust_crates_round2.md`, `E04_rust_crates_round3.md` (originals retained;
choice sections of E02 §8, E03 §6, E04 §3 are superseded by §8 here).
Survey 2026-11-08/09. Method: crates.io API (metadata + download counts),
per-crate metadata endpoints, 25 upstream READMEs, docs.rs spot checks.
Versions = crates.io `max_version` at survey time unless noted. Download
counts are cumulative install estimates, not quality signals (`genevo` 248k
dormant vs `symbios-genetics` 4k current). Licenses stated only where
verified from README/docs badges; otherwise *verify in Cargo.toml/SPDX*.*

## 0. Corrections to earlier rounds

- `genetic_algorithms` repo is `leimbernon/rust_genetic_algorithms` (E01 gave
  a wrong URL); v3.0.0 confirmed current (2026-06-24, 38k dl).
- `genetic-algorithm` (basvanwesting) is far livelier than E01 §3 implied:
  **v0.28.0, 83k downloads, updated 2026-10-08** — Evolve ES + HillClimb +
  Permutation strategies, rayon `par_fitness`, staleness detection, criterion
  profiling. Upgraded: best single-population scaffold (beats `easy_ga`).
- `evolutionary` (IgorFroehner) **v0.5.0, updated 2026-10-14**: parallel-by-
  default framework, Bin/Int/IntPerm/Real + custom encodings, custom
  operators, `ExperimentRunner` (parallel repeats), seeded reproducibility,
  MIT. No island/MO. Parallel single-pop alternative.
- `nsga_rs` (s-simoncelli) **v1.4.1** (2026-07-18) — rename from `optirustic`
  complete (`optirustic` frozen at 1.2.3, 15.4k dl); but narrower than
  `moors`, still superseded for Pareto work.
- `evo-engine` confirmed **not on crates.io** (GitHub-only, CanReader).
  `pymoors` is PyPI-only (not a crate). `radiate-gp` is not a separate crate
  (use `radiate` 1.3.2). `argmin` 0.11.0 updated 2025-09-28 — quiet, not
  dead, but `basin` (days-old releases) is still the forward pick.
- `qopt`/`sefar` upstream repos are gone (docs.rs-only). `cool-japan` org
  crates (`oxicuda-evol`, `scirs2-optimize`) ride a 1.3M-SLoC mono-repo
  README; their download counts reflect the org — verify scope before citing.

## 1. Master verdict table

| Crate | Ver | Scope | Verdict |
|---|---|---|---|
| `genetic_algorithms` | 3.0.0 | 17-engine GA platform | **Default foundation** — only VL + parsimony + IslandGa + memetic slot |
| `genoxide` | 0.x α | GA/ES/CMA/DE/PSO/GP/NEAT/MOO/LS/BO | **Second pillar** — everything except VL; watch API churn |
| `moors` | 0.2.11 | NSGA-II/III, R-NSGA-II, Age-MOEA, REVEA, SPEA-II | **Pareto engine** (+pymoors) |
| `basin` | 3.3.0 | GD/CG/BFGS/L-BFGS-B/TR/NM/CMA-ES, constrained | **Refiner toolbox** (supersedes `argmin`) |
| `symbios-genetics` | 0.2 | GA/NSGA-II/MAP-Elites/CVT/Novelty/speciation | **QD archive**; needs MOME feature (E05) |
| `mahf` (+bmf/coco/tsplib) | 0.1.0 | modular metaheuristic builder, GPL-3.0 | Research harness + COCO |
| `blackopt` / `optimizer` | 0.1.0 | Optuna-style studies, MOO, pruners | Study/orchestration layer |
| `egobox` | 1.0.1 | EGO (Egor + Gpx) + Python | Surrogate loop |
| `oxigen` | 2.x | parallel GA, age/niches/caches, MPL-2.0 | Age/niche machinery source |
| `radiate` | 1.3.2 | GA/GP/NEAT/speciation/novelty + Python | Species/tribes hook, batch eval |
| `genetic-algorithm` | 0.28.0 | Evolve/HillClimb/Permutation, par_fitness | Best single-pop scaffold |
| `evolutionary` | 0.5.0 | parallel-first GA framework, MIT | Parallel single-pop alternative |
| `metaheuristics-nature` | 10.1.0 | RGA/DE/PSO/Firefly/TLBO one-API | Cheap baselines |
| Evo-Engine (GH-only) | 1.3.0 | GA/PSO/DE/ES, K-means islands | Adaptive-island hook |
| `cmaes` / `haru_cmaes` / `crfmnes` | — | CMA-ES / fast fork / NES | CMA-ES specialists |
| `differential-evolution` / `ys_differential_evolution` | — | jDE / DE | DE specialists |
| `pso-rs` / `pso` / `pos_pso` | — | PSO (rayon in pso-rs) | PSO specialists |
| `sefar` | 0.2.0 | GA/EM-like/GSK/PSO, rayon+serde | Only GSK source |
| `frostfire` / `simulated_annealing` / `mincost` | — | SA toolkits | Annealing (coating classics) |
| `ganesh` | 0.27 | minimisation/sampling, pure L-BFGS-B | Refiner alternative |
| `fcmaes-core` | — | parallel gradient-free, MIT | Retry-parallelism pattern |
| `kurobako` | — | benchmark CLI harness | Comparison proof |
| `rlevo` | 0.6.1 | RL+EAs on Burn | Only with RL-constructor stage |
| `genevo` | 0.7 | trait-block GA kit (dormant 2022) | Ideas, not dependency |
| `genetic-rs` family | — | small MIT GA kit + speciation | Prototype spike |
| `rustneat` / `neat-rs` | — | NEAT (dormant 18/20) | Cite, don't depend |
| `optlib` / `genetic` / `ratio-genetic` | — | textbook/small GA pairs | Teaching / niche |
| `easy_ga` / `rsgenetic` / `darwin-rs` | 1.2.1/1.8.1/0.4.0 | minimal loops | `easy_ga` for throwaways; others retired |
| `ecrs` | 0.1.0-beta.4 | GA/ACO/Firefly/PSO (beta) | Watch-only |
| `red-queen-core` / `qopt` / `scirs2-optimize` | — | new / orphaned / mega-repo | Verify scope first |
| `nsga_rs` | 1.4.1 | NSGA2/3/Adaptive (narrower than moors) | Superseded by `moors` |
| `argmin` | 0.11.0 | broad solver toolbox | Superseded by `basin` |

## 2. Foundation frameworks

### 2.1 `genetic_algorithms` v3.0.0 (leimbernon/rust_genetic_algorithms, MIT/Apache, 38k dl, Rust ≥ 1.81, WASM-clean, rayon-parallel)

17 engines, generic over chromosome/gene types:

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
Criterion benches incl. `island_ga`.

### 2.2 `genoxide` 0.x alpha (tachsin/genoxide, MIT/Apache, PyPI wheels + any-language CLI)
GA (generational, steady-state, μ+λ, μ,λ, memetic schemes, classic operators)
+ evolution strategies + CMA-ES + differential evolution (JADE/SHADE/L-SHADE,
IPOP/BIPOP restarts) + particle swarms + genetic programming (strongly typed
trees, subtree/one-point crossover, subtree/point/hoist/shrink/constant
mutation, bloat control) + neuroevolution (MLP/RNN weight evolution, NEAT,
pole-balancing tasks) + MOO (NSGA-II/III, SPEA2, MOEA/D, SMS-EMOA + quality
indicators) + local search (hill climbing, simulated annealing, tabu search,
iterated local search, Nelder-Mead simplex with random restarts) +
gradient-based (L-BFGS-B from few variables to millions, GD/momentum/Nesterov,
Adam/AdamW with schedules, finite-difference fallback, MMA/GCMMA with
constraint Jacobians, continuation runs through smooth→sharp stages keeping
optimizer state) + Bayesian optimisation (Matérn/SE GP, hyperparameter MLE,
log-EI/EI/PI/CB maximised by L-BFGS-B, log transforms, Kriging-believer /
constant-liar batches, async evaluation, per-constraint PoF modelling, integer
genes, standalone fittable GP) + genomes (bit-packed binary, integer, real,
permutation, self-adaptive real) + test problems (Rastrigin/Rosenbrock/Branin,
CEC-2006 g01–g24, welded beam, pressure vessel, ZDT/DTLZ/BNH/OSY with fronts)
+ Engine (parallel/batch/async evaluation, island models, constraints,
checkpoints, bit-exact reproducible seeds). Benchmarked against 16 libraries
under published rules (OneMax-1000 GA, Rastrigin-30 DE, Rosenbrock-10 CMA-ES,
10 seeds, instruction-count version tracking). For coatings: one dependency
covers PAPER Ch.5–14 except variable-length encoding; BO-with-PoF is the
natural surrogate for expensive RCWA/FEM evaluations (Ch.14). Watch item: API
still moves between 0.x versions (roadmap + AGENTS.md in repo).

### 2.3 `radiate` v1.3.2 (pkalivas/radiate, 56k dl, site + Python package)
One pipeline from `Vec<char>` string-matching to NEAT graph evolution and
symbolic-regression trees. Single- and multi-objective GA, opt-in NEAT
neuroevolution (graph genomes), tree GP, opt-in speciation (diversity),
novelty search, first-class metric tracking, Dynamic Expression DSL for
adaptive rates / custom metrics, parallelism built in. Bit/Float/Char
chromosomes + codecs; rich operator set (blend/arithmetic mutators,
Boltzmann/elite selection); `Ecosystem` with species grouping (tribes hook,
cf. D11 CTCM); `BatchFitnessEvaluator` for amortized batch fitness. Rust core
ships a Python package (`pip install radiate`): merit prototyping starts in
Python, moves to Rust. Builder API
(`GeneticEngine::builder().codec(…).fitness_fn(…).build()`). (Name warning:
the crate is `radiate`, not `radiate-gp` — no such crate.)

### 2.4 `mahf` 0.1.0 + bmf/coco/tsplib (mahf-opt, **GPL-3.0**, GECCO'23 Companion paper + technical report, cite as wurth2023/stegherr2023)
Framework for modular construction *and evaluation* of metaheuristics — not
only EAs but any metaheuristic incl. non-population, constructive, hybrid.
Builder-DSL assembly (`initialization::RandomSpread → evaluate →
update_best_individual → while_(LessThanN) { selection::Tournament →
recombination::ArithmeticCrossover → mutation::NormalMutation →
boundary::Saturation → evaluate → replacement::MuPlusLambda }`), heuristic
templates under `src/heuristics`, components under `src/components`,
effortless state management/tracking, flexible runtime logging.
`mahf-bmf`: continuous benchmark functions. **`mahf-coco`: COCO bindings**
(publication-grade optimizer comparisons). `mahf-tsplib`: TSPLIB bindings.
The modular answer to "try seven hybrids in a week" (PAPER Ch.15). Caveat:
GPL-3.0 — fine for papers, check before productising. Requires gcc/clang.

### 2.5 `oxigen` 2.x (Martin1887/oxigen, **MPL-2.0**, 38.5k dl)
"Fast, parallel, extensible and adaptable" GA (N-Queens-255 in seconds, with
published criterion benches: 29 benches from crossover/mutation through all
survival-pressure variants with ns/iter tables). `Genotype` trait (any struct
with iter/into_iter/from_iter + generate/mutate/fitness/is_solution).
Customisable via traits: MutationRate/SelectionRate (constant/linear/
quadratic schedules), Age (no/linear/quadratic unfitness with threshold —
**ALPS-like age layering**, E06 §9), Selection (accumulated Roulette,
Tournaments, Cup), Crossover (SingleCrossPoint/MultiCrossPoint/UniformCross),
SurvivalPressure (large catalogue incl. most-similar replacement =
crowding/niching), PopulationRefitness (Niches built-in), StopCriterion
(SolutionFound/Generation/Progress). Fitness caching per individual
(disablable for stochastic merits) + optional `global_cache` HashMap feature
(skip re-evaluation of rediscovered individuals — the expensive-fitness
pattern); progress/population CSV logging; seedable initial individuals;
**resume-from-population** (phased runs, curriculum warm-starts) and
**in-fitness coevolution** (sub-GA re-executions inside fitness). Fixed
`genotype_size` — no variable length. Port the age/niche/cache/resume ideas;
adopt whole only for fixed-length parallel GA.

### 2.6 `genevo` 0.7 (innoave/genevo, MIT/Apache, 248k dl — biggest count, last release 2022)
Trait building blocks (Simulation/Algorithm/Termination/Operator/Population/
Phenotype/Genotype/FitnessFunction) with common implementations: SelectionOp,
CrossoverOp, MutationOp, RecombinationOp, ReinsertionOp; `Simulator` with
iteration/time statistics; composable termination (max-iter OR min-fitness…);
`PopulationBuilder` + `GenomeBuilder`; rayon multithreading (disabled on wasm
via `wasm-bindgen` feature); genotype helpers (`fixedbitset`, `smallvec`
features); examples (knapsack, monkeys, queens). Adopt for ideas, not for new
builds — dormant.

### 2.7 `genetic-algorithm` 0.28.0 (basvanwesting, 83k dl, updated 2026-10-08)
Evolve (evolution strategy) + HillClimb + Permutation strategies with a
strategy-selection guide (HillClimb when crossover is inefficient, e.g.
ordering problems; Permutation for small spaces with guarantees);
BinaryGenotype/real/permutation genotypes; `with_par_fitness()` rayon
evaluation; staleness detection + fitness scaling helpers; binary fitness
caching example; criterion profiling with flamegraphs. No island/MO.
Best single-population scaffold — beats `easy_ga` for anything beyond a
throwaway.

### 2.8 `evolutionary` 0.5.0 (IgorFroehner/evolutionary, MIT, 2026-10-14)
"Fully extensible framework for parallelized GAs": Bin/Int/IntPerm/Real +
custom encodings, custom operators, parallel-by-default (re-exports its rayon
as `evolutionary::rayon` so operators need no rayon dep), seeded
reproducibility under parallelism, `ExperimentRunner` (same builder repeated
in parallel, best/collected). No island/MO. Parallel single-pop alternative.

### 2.9 Evo-Engine 1.3.0 (CanReader/Evo-Engine, **GitHub-only — not on crates.io**, MIT/Apache, Rust 2021, no C deps)
Fluent `EvolutionLauncher` builder (`population_size → evaluate_with →
selection → crossover → mutation → epochs → bounds → launch`), ask/tell API,
hybrid CPU-thread/GPU backends, GA + DE (rand/1, best/1, current-to-best/1) +
CMA-ES (diagonal) + NSGA-II + **island model with ring migration** (rayon
`parallel` feature) + **adaptive K-means island assignment** (nearest to
D-track dynamic-population ideas in a light crate). Genomes: real, binary,
permutation. Operators: SBX/BLX-α/arithmetic/UNDX crossover;
polynomial/Gaussian/Cauchy/adaptive mutation; tournament/SUS/rank selection;
OX/PMX + swap/insert/inversion. Benchmark suite (Rastrigin/Rosenbrock/Ackley/
Schwefel/Griewank/Levy, ZDT1–3, OneMax, TSP, Knapsack), CSV/JSON history,
evolution-loop callbacks, serde `serialize` feature. Smaller and easier to
read than `genetic_algorithms`; islands are ring + K-means only.

### 2.10 `metaheuristics-nature` v10.1.0 (KmolYuan, 126k dl, updated 2024-05-02)
RGA / DE / PSO / Firefly / TLBO behind one uniform `Solver` API (swap
algorithms by replacing one struct) with per-method defaults; Pareto
container keeps history-best set for MOO; reproducible ChaCha RNG single- and
multi-thread; rayon + `no_std`/`libm` features; generation callbacks. Docs
explicitly recommend gradient methods (e.g. OSQP) when derivatives are known
— the analogue of our needle/gradient refinement note. Correction: no
Firefly/GA hybrids found — the five listed methods plus Pareto history. No
island model. Role: 50-line comparative baselines on continuous merit
landscapes before committing to a heavy framework.

### 2.11 `easy_ga` v1.2.1 (RubenRubioM, MPL-2.0, updated 2022-07-17, spare-time project)
Tiny surface: `Gene` trait (init/calculate_fitness/crossover/mutate;
Clone+Copy) + `GeneticAlgorithm` builder (population/iterations/mutation/
selection rates, fitness goal) + Roulette/Tournament/Random/Stochastic
selection + run-to-end or generation-stepped execution (mid-run
intervention) + verbosity-graded file logger. Roadmap (multithreading,
CSV/JSON export) open. Fixed-length throwaway prototypes only.

### 2.12 Retired / teaching / niche GA kits
`rsgenetic` v1.8.1 (m-decoster, 160k dl — legacy count, updated 2021-01-21):
Phenotype trait + single-threaded Simulator, Maximize/Tournament/Stochastic
selectors, stagnation early-stopping; author in maintenance mode. Same niche
as `easy_ga` but single-threaded — `easy_ga` wins; neither is a foundation.
`darwin-rs` v0.4.0 (willi-kappler, MIT, 2017-06-26): Individual/Population/
Simulation + builders, jobsteal parallelism, TSP/Sudoku/Queens/OCR examples;
pre-1.0 deps (error-chain 0.10, log 0.3). Historical interest only.
`optlib` (Jenyay — GA + PSO textbook pair, dormant 2020): teaching only.
`genetic` (Kerosene2000, 16k dl, 2015): tiny lib, historical.
`ratio-genetic` (GitLab ratio-case, 7.7k dl): niche. `genetic-rs` family
(hypercodec, 15k dl, MIT): rayon + speciation + derive macros +
mitosis/knockout vocabulary — worth a prototype spike.

### 2.13 `rustneat` / `neat-rs` (dormant 2018/2020, 19.7k/14k dl)
NEAT topology growth + speciation — the Ch.5 growth-precedent citation and
the `symbios` speciation module's ancestor. Cite, don't depend.

## 3. Multi-objective specialists & study layer

### 3.1 `moors` v0.2.11 (andresliszt/moo-rs, MIT, 14.5k dl, + PyPI `pymoors` via pyo3)
NSGA-II, NSGA-III, R-NSGA-II (reference-point steering — the §10 guided-target
mechanism), Age-MOEA, REVEA, SPEA-II — wider many-objective bench than
`nsga_rs` (stops at AdaptiveNSGA3). Builder API per algorithm (`Nsga2Builder`
etc.): user closures for fitness (population ndarray → fitness matrix) and
constraints (≤ 0 convention, `impl_constraints_fn!` macro with eq/ineq +
ε-tolerance + gene bounds), pluggable sampling/crossover/mutation, duplicate
cleaners (exact hash-based, ε-ball, or none), feasibility-first tournament,
`keep_infeasible` option. Numeric stack: ndarray + faer. No single-objective,
island, or variable-length machinery — same MOEA-only scope as `nsga_rs`.
Verdict: strongest Pareto engine; preferred over `nsga_rs`.

### 3.2 `nsga_rs` v1.4.1 (s-simoncelli/nsga_rs — rename from `optirustic` complete; `optirustic` frozen 1.2.3)
NSGA2 (Deb 2002) / NSGA3 (Deb–Jain 2014) / AdaptiveNSGA3; minimise/maximise
any objective count, constrained or not; real/integer/boolean/choice
variables; rayon evaluation; JSON history export + resume-from-file; Python
companion for Pareto plotting. Use only for genuine Pareto merits
(reflectivity bands vs AOI stability); no single-objective or island
machinery; narrower bench than `moors` — superseded.

### 3.3 `blackopt` 0.1.0 (brayniac, MIT/Apache) + `optimizer` (raimannma/rust-optimizer)
Optuna-model black-box studies: define-by-run + ask-and-tell, pluggable
storage (in-memory incl.), `enqueue_trial` (seed known-good stacks — warm
starts, curriculum seeds), **10–12 samplers**: Random, TPE (+Multivariate),
Grid, QMC-Halton (+Sobol), CMA-ES, MORBO (2-objective MOBO), NSGA-II/III,
MOEA/D (+DE, GP, BOHB in `optimizer`), BruteForce, **PartialFixed** (fix some
parameters = pinned start/stop layers, PAPER §7); pruners Median/Percentile/
Threshold/Patient/Hyperband/SuccessiveHalving/Wilcoxon/Nop (mid-run tribe
killing, §10); parameter importance fANOVA + Spearman (`optimizer` + HTML
reports + polars export); Float/Int/Categorical/Bool/Enum params; Pareto
front extraction; JSONL journal resumption + tokio parallel (`optimizer`).
Lab-notebook layer above the GA: MOO front (Ch.12) + orchestration (Ch.14).

## 4. Quality-diversity (only `symbios-genetics`)

`symbios-genetics` 0.2 (TheJanusStream, MIT, 4k dl): SimpleGA (elitism +
tournaments) + NSGA-II + **MAP-Elites + CVT-MAP-Elites** (Voronoi archives
for high-dim/non-uniform descriptor spaces) + **Novelty Search** (kNN
behavioural distance, configurable novelty/fitness blend) + standalone
NEAT-style **speciation** (CompatibilityDistance you supply, explicit fitness
sharing, adaptive threshold toward target species count, stable species IDs)
+ `Evaluator` returning (fitness, objectives, descriptor) + `Evolver` trait
(hot-swap SimpleGA↔MapElites) + rayon `parallel` (default, needs Send+Sync)
+ serde reproducibility + QD metrics (coverage, qd_score) + CSV archive export
(`export` feature; seahash genotype hashes for joins) + pre-made scorers
(Displacement/UpAlignment/Height/EnergyEfficiency, Multiply/Sum/Penalize/
Normalize combinators, CompositeEvaluator). New-paper angles it unlocks:
merit-vs-depth illumination (E04), MOME per-bin fronts + learned-descriptor
loop + DQD loop (E05/E07 — all three are build-it features, no crate has
them yet). Speciation module doubles as tribes mechanism (Ch.9/13).

## 5. Refiners (memetic finish)

### 5.1 `basin` 3.3.0 (jolars/basin, MIT/Apache, basin.rs, days-fresh releases)
Numerical optimisation with solver-owned convergence, **first-class
constraints** (tenet 4), read-only observer layer, `RunControl`, backend
tiering (Vec/nalgebra/ndarray/faer): GradientDescent, SGD, NonlinearCG,
ProjectedGradientDescent (= repair inside the optimiser), BFGS,
L-BFGS/L-BFGS-B, TrustRegion, NelderMead, CMA-ES; warm-started states
(custom simplex, warm inverse Hessian, anisotropic covariance),
`from_start` convenience with compile-time gating, line searches, scalar root
finders. Refiner pick behind `LocalSearchOperator` (LM/NM/L-BFGS finish,
PAPER Ch.11). Supersedes `argmin`.
### 5.2 `argmin` 0.11.0 (argmin-rs, MIT/Apache, 5.6M dl, book at argmin-rs.org)
Type-agnostic over math backends. Solvers: backtracking/More-Thuente/
Hager-Zhang line searches; trust-region (Cauchy/Dogleg/Steihaug); steepest
descent; CG + nonlinear CG; Newton/Newton-CG; quasi-Newton
(BFGS/L-BFGS/DFP/SR1/SR1-TrustRegion); Gauss-Newton (+linesearch); golden-
section; Landweber; Brent (opt + root); Nelder-Mead; simulated annealing;
PSO. Services: observers (screen/disk/custom), checkpointing, external-solver
compat (egobox, cobyla). Superseded by `basin` for new work; retained for its
solver breadth.
### 5.3 `ganesh` 0.27 (denehoffman, Apache/MIT, 65k dl)
Minimisation + sampling through scalar-/linalg-generic traits
(`CostFunction`, optional `Gradient`, finite-difference default); **pure-Rust
L-BFGS-B**; PyO3-downstream pattern (typed config/init/callback/transform/
result objects reusable without publishing a Python package); parameter
transforms; codspeed-benchmarked. Refiner-side alternative to `basin`.

## 6. Single-algorithm specialists

- **CMA-ES:** `cmaes` (pengowen123, 25k dl — IPOP/BIPOP restarts, bench suite,
  safe default); `haru_cmaes` (pycma-faithful, claims ~3.6x over `cmaes`,
  faer backend planned); `crfmnes` (millardjn, MIT — CR-FM-NES natural ES).
  (A06 tutorial is the in-graph theory.)
- **DE:** `differential-evolution` (martinus, MIT/Apache, 9.4k dl —
  self-adaptive jDE, one-liner global polish); `ys_differential_evolution`
  (11k dl). Modern JADE/SHADE/L-SHADE live in `genetic_algorithms`/`genoxide`.
- **PSO:** `pso-rs` (MIT, 20k dl — rayon parallel eval, indicatif progress);
  `pso`; `pos_pso`. (Metaheuristics-nature and genoxide bundle their own.)
- **SA/annealing:** `frostfire` (modular, rigorous SA); `simulated_annealing`
  (paveloom, 5.4k dl); `simulated-anneal`; `anneal-core`; `mincost`
  (MIT/Apache heuristic toolkit). Thin-film SA literature is extensive
  (Dobrowolski lineage) — these are its Rust executors; `genoxide` also
  bundles SA/tabu/ILS/NM-restarts.
- **Exotic:** `sefar` 0.2.0 (MIT, docs.rs-only — eo/EM-like + GA + **GSK**
  + PSO, rayon+serde, benches module; only GSK source).

## 7. Surrogates, benchmarks, RL, watch-list, corrections

- **Surrogate/BO:** `egobox` 1.0.1 (relf, Apache-2.0, 38.6k dl — Egor
  optimiser + Gpx GP-mixture surrogate + Python module; the light Ch.14
  loop); BO depth lives in `genoxide` (Matérn/SE, EI/PI/CB, Kriging-believer
  batches, PoF) and `blackopt`/`optimizer` (TPE/MORBO/GP/BOHB).
- **Benchmarking:** `kurobako` (optuna, 57k dl — CLI harness: continuous/
  discrete/categorical problems, concurrent-worker simulation, markdown+PNG
  reports; the A11 angle made concrete); `mahf-bmf`/`mahf-coco`/`mahf-tsplib`
  (0.1.0 each — functions, COCO, TSPLIB); criterion benches in
  `genetic_algorithms`/`oxigen`/`cmaes`/`genetic-algorithm`; `genoxide`'s
  16-library public rules.
- **RL-hybrid:** `rlevo` 0.6.1 (anthonytorlucci, 147 dl, updated 2026-11-01):
  deep RL (DQN/C51/QR-DQN/PPO/PPG/DDPG/TD3/SAC) + classical EAs
  (real/binary GA, ES incl. CMA-ES/CMSA-ES, EP, DE, CGP/GEP, UMDA/Bayesian
  EDAs, compact GA) + swarms (PSO/ACO/Firefly/Cuckoo/Bat/GWO/ABC/WOA/Salp) +
  NEAT/weight-evolution/NAS + competitive/cooperative co-evolution with HoF +
  memetic wrapper (HC/NM/restarts/SA) + const-generic Observation/Action dims
  + Burn backends (Flex CPU, wgpu GPU) + versioned EpisodeRecord HTML replays
  + `rlevo-hybrid` evolution-guided policy loop. Only surveyed crate covering
  the RL-constructor direction (B02/B03/B09) beside EAs — adopt only with an
  RL stage; Burn is heavy otherwise.
- **Fitness caching/parallel eval patterns:** `oxigen` (per-individual +
  global HashMap; disable for stochastic merits), `genetic_algorithms`
  `BatchFitnessEvaluator`, `radiate` batch evaluator, `genevo`/`pso-rs`/
  `symbios` rayon, `fcmaes-rust` (MIT — fcmaes port; retry parallelism turns
  idle cores into chances: 7/7 vs 0/7 equal-wall), `optimizer` tokio,
  `genetic-algorithm` `par_fitness`.
- **Beta/watch:** `ecrs` 0.1.0-beta.4 (ecrs-org, MIT/Apache, AGH — generic GA
  with plug-in operators: Single/Two/MultiPoint/Uniform/Ordered/PMX crossover,
  Roulette/Random/Rank/RankR/Tournament/SUS/Boltzmann selection,
  Identity/FlipBit/Interchange/Reversing mutation, RandomPoints/BitStrings
  generators; + Ant System ACO + Firefly + PSO; probe logging; test functions;
  rayon/nalgebra/push-trait optional — the only ACO+GA/PSO/Firefly bundle, ant
  construction heuristics fit layer-sequence building; no MSRV policy, no
  island/MO — watch/contribute, don't adopt); `red-queen-core` (229 dl —
  unproven); `qopt` (repo gone); `scirs2-optimize` (mega-repo scope).
- **Corrections (not EC / not crates):** `pushgen` (AndWass push-iterator,
  0.0.2), `push-trait` (clarcharr, CC0, collection push), `goldens`
  (nonexistent — docs.rs 404), `pymoors` (PyPI-only), `evo-engine` (GH-only),
  `radiate-gp` (no such crate), `cool-japan` scope caution.

## 8. Final choice (supersedes E02 §8, E03 §6, E04 §3)

- **Foundation:** `genetic_algorithms` (variable-length, IslandGa, MOEA bench).
- **Second pillar:** `genoxide` (everything else; prove BO + MOEA/D on a
  40-layer filter before committing).
- **Pareto split-off:** `moors`. **Refiners:** `basin` (alt: `ganesh`).
  **QD archive:** `symbios-genetics` (+MOME/learned-descriptor/DQD features).
  **Study layer:** `blackopt`/`optimizer` (enqueue, pinning, pruning, fANOVA).
  **Surrogate-only:** `egobox`. **Comparison proof:** `mahf-coco`/`kurobako`
  (+`mahf` harness, mind GPL-3.0).
- **Prototypes/baselines:** `genetic-algorithm` (best scaffold),
  `evolutionary` (parallel alt), `metaheuristics-nature` (50-line baselines),
  `easy_ga` (throwaways). **Conditional:** `rlevo` (RL stage), Evo-Engine
  (adaptive K-means islands), `radiate` (species/batch), `oxigen` (age/cache
  patterns), `ecrs` (watch), CMA/DE/PSO/SA specialists per need.
- **Retired:** `darwin-rs`, `rsgenetic`, `nsga_rs` (use moors), `argmin`
  (use basin), `genevo`/`rustneat`/`neat-rs` (cite only).

## 9. Paper & literature map
VL+parsimony → PAPER §5 (A09/A10/D07); islands/tribes → §9
(D01/D02/D06/D11/D13/D15/D17, oxigen age, radiate species, Evo-Engine
K-means); constraints → §7 (A06/F08/moors/basin/mahf); operators → §8
(A06/E02); needle+LM → §10–11 (C00/basin/NM); MOO → §12 (B05/F07/moors);
diversity/QD → §13+QD (D16/B04/G01/G05–G15/symbios); surrogates/BO → §14
(D04/B09/G02/egobox/genoxide); hybrids → §15 (mahf); benchmarks → A11/
kurobako/COCO. Open lit gaps: ALPS/Hornby, TPE/BOHB, MOO primaries (E07 §A).
Build list (no crate yet): MOME fronts, POME offline, learned descriptors,
DQD loop, growing archives (E05/E07).

*Sources: E01–E04 files verbatim where noted; crates.io metadata 2026-11-09
(two meta batches); 28 upstream READMEs (`C:/Users/Main/_probe/readme_*.md`);
docs.rs (sefar algos, basin 3.3.0). Version policy: re-check `max_version`
before depending — pre-1.0 crates move fast.*
