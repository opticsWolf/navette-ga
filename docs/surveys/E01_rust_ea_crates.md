# Rust crates for genetic / evolutionary algorithms (surveyed 2026-11-08)

Sources: crate READMEs fetched 2026-11-08 (GitHub, static fetch) + web
search snippets. Versions as observed that day; check crates.io / docs.rs
for newer releases.

## 1. `genetic_algorithms` — leimbernon (first choice for coating work)

- Repo: https://github.com/leimbernon/genetic-algorithms · Docs: https://docs.rs/genetic_algorithms · Crate: https://lib.rs/crates/genetic_algorithms
- Version observed: **v3.0.0** (Rust >= 1.81, MIT/Apache-2.0, ~24K SLoC). WASM-compatible, Criterion benches incl. `island_ga`.
- **17 engines**: standard GA, NSGA-II/III, MOEA/D, SPEA2, SMS-EMOA, IBEA, **island model**, DE, CMA-ES with IPOP/BIPOP restarts, PSO (Global/Ring/VonNeumann topologies), UMDA-EDA, Scatter Search, Cellular GA, ALPS, Hill Climbing, permutation search, GP (`GpGa`).
- Why it fits thin-film stack synthesis: **variable-length chromosomes**
  (`ChromosomeLength::Variable`, Insertion/Deletion mutation,
  `VariableLength` crossover) + length-penalty parsimony pressure map
  directly to variable layer count; **memetic local search**
  (Lamarckian/Baldwinian) pairs with needle/gradient refinement;
  **surrogate-assisted evaluation** for expensive merits; constraint
  handling (Static/Dynamic/Adaptive/Death penalties); Hall of Fame elite
  archive; adaptive operator selection (bandit); rayon multithreading;
  lifecycle observers + PNG/SVG charts.
- Benchmarks feature: Sphere, Rastrigin, Ackley, ZDT1-6, DTLZ1-7,
  Hypervolume/GD/IGD/Spread indicators.

## 2. `evo-engine` — CanReader (GitHub-only, not on crates.io)

- Repo: https://github.com/CanReader/Evo-Engine · pure Rust, no C deps, Rust 2021.
- GA + DE (rand/1, best/1, current-to-best/1) + CMA-ES (diagonal) + NSGA-II + **island model with ring migration** (rayon `parallel` feature).
- Genomes: real, binary, permutation. Operators: SBX/BLX-alpha/arithmetic/UNDX crossover; polynomial/Gaussian/Cauchy/adaptive mutation; tournament/SUS/rank selection; OX/PMX + swap/insert/inversion for permutations.
- Benchmark suite (Rastrigin, Rosenbrock, Ackley, Schwefel, Griewank, Levy, ZDT1-3, OneMax, TSP, Knapsack), CSV/JSON history export, evolution-loop callbacks, serde `serialize` feature.
- Smaller and easier to read than (1); island model is ring-topology only.

## 3. `genetic-algorithm` — Elixir-book-inspired single GA

- Crate: https://lib.rs/crates/genetic_algorithm · design triple: `Genotype` (search space) + fitness function + run loop.
- Good template for a bespoke variable-length stack encoding; not a full framework (no island model, no MO).

## 4. `evolutionary` — parallel-GA framework

- Docs: https://docs.rs/evolutionary · supports Bin/Real/Permutation/Integer codings with built-in operators.
- Candidate if the need is "parallel single-population GA with custom coding" rather than island/multi-objective machinery.

## 5. `optirustic` — dedicated MOEA (verified v1.2.3, 2026-06-21)

- Docs: https://docs.rs/optirustic · Repo: https://github.com/s-simoncelli/optirustic · owner s-simoncelli.
- NSGA2 / NSGA3 / AdaptiveNSGA3 only (no single-objective engines); real/integer/boolean/choice variables incl. constraints; rayon multithreaded evaluation; JSON history export + resume-from-file; companion Python plotting package.
- Caveats (2026-11-08): banner says the crate is **moving to `nsga-rs`** (same author); docs.rs **failed to build 1.2.3** — read the 1.2.2 docs. Relevance as claimed: lighter MOEA-only alternative when the merit is a genuine Pareto problem (reflectivity bands vs. AOI stability) and you don't need island/single-objective machinery.

## 6. `metaheuristics-nature` — baseline benchmarking toolkit (verified v10.1.0)

- Docs: https://docs.rs/metaheuristics-nature · Repo: https://github.com/KmolYuan/metaheuristics-nature-rs.
- RGA / DE / PSO / Firefly / TLBO behind one uniform `Solver` API (swap algorithms by replacing one struct), Pareto container for MOO, reproducible ChaCha RNG single- and multi-thread, rayon + `no_std`/`libm` features.
- Correction: no Firefly/GA hybrids found — the five listed methods plus Pareto history. No island model. Best use as stated: quick comparative baselines on continuous merit landscapes before committing to a heavy framework.

## 7. `radiate` — full EC framework incl. GP (verified v1.3.2)

- Docs: https://docs.rs/radiate · Repo: https://github.com/pkalivas/radiate · site https://pkalivas.github.io/radiate/.
- Correction: the crate is named **`radiate`** (not `radiate-gp`). Bit/Float/Char chromosomes + codecs, rich operator set (incl. blend/arithmetic mutators, Boltzmann/elite selection), `Ecosystem` with species grouping, `BatchFitnessEvaluator` for amortized batch fitness.
- Relevance: overkill for fixed stack vectors, but the species/ecosystem structure and batch evaluator are interesting if tribe-style (cf. D11 CTCM) grouping or batched TMM evaluation is wanted inside a framework.

## 8. `easy_ga` — minimal single GA (verified v1.2.1)

- Docs: https://docs.rs/easy_ga · Repo: https://github.com/RubenRubioM/easy_ga/.
- Tiny surface: `Gene` trait + `GeneticAlgorithm` struct + selection + samples. As claimed: fastest path to a bespoke fixed-length prototype; no island/MO/multithreading story.

## 9. `rlevo` — RL + evo on Burn, heavyweight (verified v0.6.1, corrected framing)

- Docs: https://docs.rs/rlevo · Repo: https://github.com/anthonytorlucci/rlevo.
- Correction: not primarily "tensor-native EAs" — it is **deep RL (DQN/C51/QR-DQN/PPO/PPG/DDPG/TD3/SAC) + evolutionary optimization** on the Burn framework, with `core/envs/rl/evo/hybrid/harness` subcrates. The `evo` module does cover GA/ES/CMA-ES/DE/EP/GP/swarm/EDAs/memetic/neuroevolution/co-evolution with ask/tell APIs, and Burn backends (`flex` CPU default, `wgpu` GPU) give tensor/GPU execution.
- Relevance: the only surveyed crate covering the RL-constructor direction (cf. B02/B03/B09 deep-RL stack builders) in the same dependency as EAs — but Burn is a heavy dependency; adopt only if the RL-constructor + EA hybrid is really on the roadmap.

## Relevance to the D-track papers

- Island-model theory (D01 Ruciński: migration topology; D02 Lässig: adaptive population sizing; D06 Ferrucci: global/grid/island comparison) can be tested almost directly in crates (1) and (2).
- Tribe competition (Chen 2024, unfetched — paywalled) has no ready-made crate counterpart; closest knob is (1)'s island model + adaptive operator selection, or a custom tribe layer on top of (2)'s composable operators.
- For a coating optimizer: prototype with (1) — variable-length chromosomes + memetic refinement + surrogate evaluation cover the A10 variable-size framing and the hybrid GA+needle practice in one dependency.
