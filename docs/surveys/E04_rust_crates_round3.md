# E04 — Rust EA crates, round 3: gap analysis vs the thin-film paper + missed crates

*Survey date 2026-11-09. Method: crates.io API (16 queries, `sort=downloads`),
per-crate metadata, 22 upstream READMEs, docs.rs spot checks. Versions below
are crates.io `max_version` at survey time.*

## 1. What the paper needs vs what rounds 1–2 covered

Shopping list from `PAPER_ga_thinfilm_strategies` (§§3–16), mapped against
E01–E03 (`genetic_algorithms`, `moors`, `argmin`, `radiate`, `easy_ga`,
`metaheuristics-nature`, `ecrs`, `optirustic`/`nsga_rs`, `rlevo`, Evo-Engine,
`darwin-rs`, `rsgenetic`, `pushgen`/`push-trait`):

| Paper need | Rounds 1–2 | Gap closed in round 3 |
|---|---|---|
| Variable-length encoding (Ch.5) | `genetic_algorithms` only | Still the only one — no new VL crate found |
| Island model + migration (Ch.9) | `genetic_algorithms` IslandGa | `genoxide` island models; `oxigen` resume/coevolution pattern |
| Length penalty / parsimony (Ch.5) | `genetic_algorithms` only | Still unique |
| Memetic polish NM/L-BFGS (Ch.11) | `argmin` | **`basin` supersedes `argmin`** (see §3); `genoxide` local-search suite + L-BFGS-B + continuation; `ganesh` pure-Rust L-BFGS-B |
| CMA-ES (A06 repair, Ch.8) | none | `cmaes` (+BIPOP), `haru_cmaes` (~3.6x faster claim), `basin`, `genoxide`, `blackopt`, `optimizer`, `crfmnes` (NES) |
| Differential evolution (filter literature) | none | `differential-evolution` (self-adaptive jDE), `ys_differential_evolution`, `genoxide` (JADE/SHADE/L-SHADE), `optimizer` |
| PSO (diversity engines) | `metaheuristics-nature` only | `pso-rs` (rayon), `genoxide`, `optlib`, `sefar` |
| SA / tabu / ILS (classic coating literature) | none | `genoxide` (SA+tabu+ILS+NM), `frostfire` (rigorous SA), `simulated_annealing`, `mincost` |
| MOO NSGA-II/III, MOEA/D (Ch.12) | `moors` | `genoxide` (NSGA-II/III, SPEA2, MOEA/D, SMS-EMOA + indicators), `blackopt` + `optimizer` (Optuna-style ask-and-tell MOO), `symbios-genetics` (NSGA-II) |
| Quality-Diversity / MAP-Elites (Ch.13) | none — **new angle** | `symbios-genetics` (MAP-Elites + CVT-MAP-Elites + Novelty Search + NEAT speciation primitive); only QD crate in Rust |
| NEAT / speciation (growth precedent, Ch.5) | none | `rustneat`, `neat-rs` (both dormant), `symbios` speciation module, `genetic-rs` speciation, `genoxide` NEAT |
| Surrogate / Bayesian optimisation (Ch.14) | none | `egobox` (Egor + Gpx GP mixture), `genoxide` BO (GP, EI/PI/CB, Kriging-believer batches, PoF constraints), `blackopt` (TPE, MORBO), `optimizer` (TPE, GP, BOHB) |
| Constraints first-class (Ch.7) | `moors` | `basin` (constraint tenet, ProjectedGD, L-BFGS-B), `genoxide` (constraints + PoF), `mahf` (boundary::Saturation) |
| Benchmarking (A11) | `genetic_algorithms` benches | `kurobako` (CLI harness, concurrent-worker simulation, plots), `mahf-coco` (COCO bindings), `oxigen`/`cmaes` criterion benches |
| Python bridge | `moors` (pymoors) | `genoxide` (PyPI wheels), `egobox` (Python module), `ganesh` (PyO3 downstream pattern) |
| Framework / modularity | `radiate`, `genevo` missed earlier | `mahf` (GECCO'23 paper, builder DSL), `genevo` (trait blocks, rayon, wasm — dormant since 2022), `oxigen` (trait customisation, MPL-2.0) |
| Fitness caching / parallel eval | `genetic_algorithms` batch | `oxigen` (per-individual + global HashMap cache), `genevo`/`pso-rs`/`symbios` (rayon), `fcmaes-rust` (retry parallelism), `optimizer` (tokio) |

Two paper needs stay single-sourced: variable-length encoding and length
penalties (`genetic_algorithms` alone). Everything else now has ≥2 options.

## 2. New crates that matter (ranked)

### 2.1 `genoxide` — the union crate (alpha, pre-1.0, MIT/Apache)
GA (generational, steady-state, μ+λ, μ,λ, memetic) + ES + CMA-ES + DE
(JADE/SHADE/L-SHADE, IPOP/BIPOP) + PSO + GP + NEAT + NSGA-II/III, SPEA2,
MOEA/D, SMS-EMOA with indicators + local search (hill climbing, SA, tabu,
ILS, NM with restarts) + L-BFGS-B/Adam/MMA + BO (Matérn/SE GP, EI/PI/CB,
Kriging-believer batches, PoF constraints, integer genes) + island models +
constraints + checkpoints + bit-exact reproducible seeds + Python wheels +
any-language CLI. Benchmarked against 16 libraries with published rules.
For coatings: one dependency covers Ch.5–14 except variable-length
encoding; BO-with-PoF is the natural surrogate for expensive RCWA/FEM
evaluations (Ch.14). Watch item: API still moves between 0.x.

### 2.2 `basin` 3.3.0 — the `argmin` successor (MIT/Apache, jolars)
Numerical optimisation with solver-owned convergence, **first-class
constraints** (design tenet 4), observers, `RunControl`, backend tiering
(Vec/nalgebra/ndarray/faer): GradientDescent, SGD, NonlinearCG,
ProjectedGradientDescent, BFGS, L-BFGS/L-BFGS-B, TrustRegion, NelderMead,
CMA-ES (matched the cma-es query), warm-started states, custom simplexes.
Updated days before this survey; `argmin` 0.11 looks frozen by comparison.
**Recommendation change: put `basin`, not `argmin`, behind
`LocalSearchOperator` for the LM/NM/L-BFGS finish (Ch.11).**

### 2.3 `symbios-genetics` 0.2 — the only QD crate (MIT)
SimpleGA + NSGA-II + MAP-Elites + CVT-MAP-Elites (Voronoi archives for
high-dim descriptors) + Novelty Search (kNN behavioural distance, blendable)
+ standalone NEAT-style speciation with explicit fitness sharing +
`Evaluator` returning (fitness, objectives, descriptor) + rayon + serde
reproducibility + QD metrics (coverage, qd_score) + CSV archive export.
**New paper angle:** illuminate merit-vs-depth instead of penalising depth —
a MAP-Elites archive keyed by (layer count, total thickness) displaying the
best merit per cell *is* the parsimony debate of Ch.5 rendered as a map.
Speciation module doubles as a tribes mechanism (Ch.9/13).

### 2.4 `mahf` 0.1.0 — the research framework (GPL-3.0!, GECCO'23)
Builder-DSL assembly (`initialization → selection → recombination →
mutation → boundary → replacement`), heuristic templates, `mahf-bmf`
benchmarks, **`mahf-coco` COCO bindings**, TSPLIB bindings. The modular
answer to "try seven hybrids in a week" (Ch.15). Caveat: GPL-3.0
licensing — fine for papers, check before productising.

### 2.5 `blackopt` 0.1.0 + `optimizer` — Optuna-style studies (MIT/Apache)
Ask-and-tell + define-by-run, NSGA-II/III (+MOEA/D, MORBO, TPE in
`optimizer`), pruners (Median/Percentile/Hyperband), fANOVA importance,
`enqueue_trial` (seed known-good stacks!), `PartialFixedSampler`
(= pinned start/stop layers, Ch.7!), JSONL journal resumption
(`optimizer`), tokio parallel (`optimizer`). Use for the MOO front
(Ch.12) and the learned-model orchestration (Ch.14), not for the inner GA.

### 2.6 `oxigen` 2.x — parallel GA with age structure (MPL-2.0)
Trait-customisable rates/selection/crossover/survival, **age unfitness
(ALPS-like age layering — a diversity mechanism, Ch.13)**, niches
refitness, most-similar survival pressure (crowding), per-individual +
global fitness caches, resume-from-population, in-fitness coevolution,
published criterion benches. Fixed `genotype_size` — no variable length —
but the age/niche machinery ports conceptually to any island GA.

### 2.7 `egobox` / `egobox-ego` — EGO toolbox (MIT/Apache)
Egor optimiser + Gpx GP-mixture surrogate with Python module. The
surrogate-assisted coating loop (Ch.14): fit Gpx on (stack → merit), Egor
proposes the next TMM batch. Lighter than `genoxide`'s BO if only the
surrogate is wanted.

### 2.8 CMA-ES / DE / PSO specialists
- `cmaes` (pengowen123): IPOP/BIPOP restarts, bench suite — the safe default.
- `haru_cmaes`: pycma-faithful, claims ~3.6x over `cmaes`, faer backend planned.
- `crfmnes`: CR-FM-NES (natural ES, cost-reduction variant).
- `differential-evolution` (martinus): self-adaptive jDE (Brest) — one-liner global polish.
- `pso-rs` (20k dl): rayon parallel eval, indicatif progress.
- `sefar` 0.2 (MIT, SaadDAHMANI): eo/EM-like + GA + GSK (gaining-sharing knowledge) + PSO, rayon+serde, benches module. Narrow but the only GSK source.
- `optlib` (Jenyay): GA + PSO textbook pair, dormant 2020 — teaching only.

### 2.9 SA / annealing / benchmarking infrastructure
- `frostfire`: rigorous modular SA; `simulated_annealing`, `mincost`: small SA toolkits (thin-film SA literature is extensive — Dobrowolski lineage).
- `kurobako` (optuna, 57k dl): CLI black-box benchmark harness — continuous/discrete/categorical problems, concurrent-worker simulation, markdown+PNG reports. The A11 angle made concrete.
- `ganesh` (denehoffman): minimisation+sampling traits, **pure-Rust L-BFGS-B**, PyO3-downstream pattern, parameter transforms. Refiner-side alternative to `basin`.
- `fcmaes-rust` (dietmarwo): fcmaes port with *retry parallelism* (16 idle cores → 16 chances; 7/7 vs 0/7 on equal wall time) — the migration-budget argument for island counts.
- `genevo` (innoave, 248k dl): trait building blocks, rayon, wasm — biggest download count in the survey, but last release 2022; adopt for ideas, not for new builds.
- `rustneat` / `neat-rs`: NEAT topology growth — dormant (2018/2020), kept as the Ch.5 growth-precedent citation, not a dependency.
- `genetic-rs` family (hypercodec): rayon + speciation + derive macros + mitosis/knockout vocabulary — small, MIT, worth a prototype spike.
- Caution: `cool-japan` org crates (`oxicuda-evol`, `scirs2-optimize`) ride a 1.3M-SLoC mono-repo README; download counts reflect the org, verify scope before citing. `qopt`/`sefar` upstream repos are gone (docs.rs only).

## 3. Revised recommendation deltas (amend E01–E03)

1. `basin` replaces `argmin` as the refiner toolbox (active, constrained, backend-tiered).
2. `genoxide` becomes the second framework pillar next to `genetic_algorithms`: everything except variable-length lives there; evaluate its BO + MOEA/D against `moors` + `egobox` on a 40-layer filter.
3. `symbios-genetics` adds the QD archive (MAP-Elites over depth/thickness cells) and the speciation primitive — prototype the Ch.5 merit-vs-depth map with it.
4. `blackopt`/`optimizer` cover study-level MOO + orchestration (ask-and-tell, enqueue, pinning, fANOVA) — the lab-notebook layer above the GA.
5. `mahf` (+COCO) is the publication-grade harness for the Ch.15 seven-hybrid comparison; mind GPL-3.0.
6. Unchanged: `genetic_algorithms` stays the default (only VL + parsimony + IslandGa + BRKGA decoder story), `moors` stays the Pareto engine, `darwin-rs`/`rsgenetic` stay retired, `ecrs` stays watch-only.

## Sources
crates.io API + metadata 2026-11-09; READMEs: genevo, oxigen, genetic-rs,
mahf, blackopt, genoxide, symbios-genetics, cmaes, differential-evolution,
egobox, ganesh, kurobako, rust-optimizer, haru_cmaes, fcmaes-rust, mincost,
pso-rs, rustneat, differential_evolution (ys), oxicuda, crfmnes,
rust-optimization; docs.rs: sefar (/algos: eo, go, gsk, pso), basin 3.3.0.
