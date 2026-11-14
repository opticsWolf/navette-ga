# Rust crates, second round (E03) — MOEA specialists, classic GA scaffolds, local-search toolbox

Verification + README survey, 2026-11-08. Companion to E01/E02.
Three corrections to the suggested list first: `pushgen` is a push-iterator
library (AndWass, v0.0.2), `push-trait` genericises collection `push`/`insert`
(clarcharr, CC0) — neither is evolutionary computation. `goldens` does not
exist on crates.io/docs.rs (404). The EC-relevant Push-family item is ECRS's
Push support (optional `push-trait` dependency).

## 1. `moors` v0.2.11 — the MOEA specialist (repo: moo-rs)

Docs: https://docs.rs/moors · Repo: https://github.com/andresliszt/moo-rs ·
Author andresliszt, MIT, pymoo-inspired. Pure-Rust core (`moors`) + Python twin
(`pymoors`, pyo3, `pip install pymoors`) mirroring the optirustic/nsga_rs
Python-plotting pattern from E01.
Algorithms: NSGA-II, NSGA-III, R-NSGA-II, Age-MOEA, REVEA, SPEA-II —
a wider many-objective bench than nsga_rs (which stops at AdaptiveNSGA3).
Builder API per algorithm (`Nsga2Builder` etc.): user closures for fitness
(population ndarray → fitness matrix) and constraints (≤ 0 convention,
`impl_constraints_fn!` macro with eq/ineq + ε-tolerance + gene bounds),
pluggable sampling / crossover / mutation, duplicate cleaners
(exact hash-based, ε-ball, or none), feasibility-first tournament,
`keep_infeasible` option. Numeric stack: ndarray + faer.
Verdict: the strongest `nsga_rs` rival for strict-Pareto work — broader
algorithm bench, cleaner constraint story, same Rust+Python shape.
No single-objective, island, or variable-length machinery: same MOEA-only
scope limitation as nsga_rs.

## 2. `ecrs` v0.1.0-beta.4 — academic EC toolkit (repo: ecrs-org/ecrs)

Docs: https://docs.rs/ecrs · AGH-university project, MIT/Apache, explicitly
early-development (breaking changes without notice). Generic GA with plug-in
operator sets (crossover: SinglePoint/TwoPoint/MultiPoint/Uniform/Ordered/PMX;
selection: RouletteWheel/Random/Rank/RankR/Tournament/SUS/Boltzmann; mutation:
Identity/FlipBit/Interchange/Reversing; generators RandomPoints/BitStrings),
plus Ant System (ACO), Firefly, and PSO implementations; probe-based logging
system; test-functions module; rayon/nalgebra/push-trait optional.
Verdict: interesting as the only surveyed crate bundling ACO alongside
GA/PSO/Firefly (ant-colony construction heuristics are a plausible
layer-sequence builder), but beta status + no MSRV policy + no island/MO story
rule it out as a foundation. Watch or contribute, don't adopt yet.

## 3. `darwin-rs` v0.4.0 (2017) — dormant classic

Docs: https://docs.rs/darwin-rs · Repo: willi-kappler/darwin-rs, MIT.
Individual/Population/Simulation + builder types, jobsteal-based parallelism,
examples TSP/Sudoku/Queens/OCR. Last release 2017-06-26 (deps: error-chain
0.10, log 0.3 — pre-1.0 ecosystem). Verdict: historical interest only;
superseded in every respect by `genetic_algorithms` and `Evo-Engine`.

## 4. `rsgenetic` v1.8.1 — minimal, maintenance mode

Docs: https://docs.rs/rsgenetic · Repo: m-decoster/RsGenetic, MIT/Apache.
Phenotype trait (fitness/crossover/mutate) + single-threaded sequential
Simulator only; Maximize/Tournament/Stochastic selectors; early stopping on
stagnation; generic fitness types. Author states maintenance mode, no capacity
for the needed API redesign. Verdict: same niche as `easy_ga` (E02 §5) but
single-threaded — `easy_ga` wins that niche; neither is a foundation candidate.

## 5. `argmin` v0.11.0 — the local-refinement toolbox

Docs: https://docs.rs/argmin · Site/book: https://www.argmin-rs.org/book/ ·
MIT/Apache, type-agnostic over math backends (nalgebra/ndarray/custom).
Solvers: backtracking/More-Thuente/Hager-Zhang line searches, trust-region
(Cauchy/Dogleg/Steihaug), steepest descent, CG + nonlinear CG, Newton/Newton-CG,
quasi-Newton (BFGS/L-BFGS/DFP/SR1/SR1-TrustRegion), Gauss-Newton (+linesearch
variant), golden-section, Landweber, Brent (opt + root), Nelder-Mead, simulated
annealing, PSO. Framework services: observers (screen/disk/custom), checkpointing
for crash-prone environments, external-solver compat (egobox, cobyla) via the
Solver trait.
Verdict: not a GA — the memetic-refinement half of the architecture. L-BFGS
(gradient, when adjoints exist) and Nelder-Mead (gradient-free, matches the
TMM-merit reality) are the two refinement candidates; observers/checkpointing
mirror the telemetry story of `genetic_algorithms` §1. Pair, don't choose:
`genetic_algorithms` IslandGa/NSGA + `argmin` Nelder-Mead/L-BFGS refinement
covers the full Global Search → Parsimony → Memetic Refinement pipeline in
pure Rust.

## 6. Updated crate choice (supersedes E02 §8)

- Global search: `genetic_algorithms` (variable-length, IslandGa, MOEA bench).
- Strict Pareto split-off: `moors` preferred over `nsga_rs` (wider bench:
  R-NSGA-II, Age-MOEA, REVEA; better constraints; same Rust+Python shape).
- Baselines: `metaheuristics-nature`.
- Memetic refinement: `argmin` (Nelder-Mead gradient-free default, L-BFGS when
  adjoints exist) behind `genetic_algorithms`' LocalSearchOperator slot.
- Fallback islands/tribes hooks: `radiate` species, `Evo-Engine` adaptive
  K-means islands. Prototypes: `easy_ga`. RL-hybrid only: `rlevo`.
- Do not adopt: `darwin-rs` (dormant 2017), `rsgenetic` (maintenance mode,
  single-threaded), `ecrs` (beta — watch), `pushgen`/`push-trait`/`goldens`
  (not EC crates / nonexistent).
