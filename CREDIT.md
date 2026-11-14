# CREDIT.md — provenance ledger for navette-ga (E10 v2, rule: `source = crate@version, files:`)

Our license: LGPL-3.0-or-later (crate), the same license as navette. Nothing
below is copied into the build yet — see `staging/STAGED-COPIES.md` for the
gated staging area. Dependency-only crates need no entries here (their licenses
ride with cargo); entries are added if and only if a file is copied in.
Copied files retain their original license (Apache-2.0 / MIT / MPL-2.0); every
source on the candidate list is compatible with LGPL-3.0-or-later.

## Staged (not yet copied — Phase-3 gate, E09 v2 §1.1)

- `source = genetic_algorithms@3.0.0` (Apache-2.0), repo
  `https://github.com/leimbernon/rust_genetic_algorithms.git`, checkout
  `efd7545`, mirror at `vendor/sources/genetic_algorithms/` (git-ignored).
  Candidate files: `src/types/chromosomes/length.rs`,
  `src/operations/mutation/length_mutation.rs`,
  `src/operations/crossover/variable_length.rs`,
  `src/operations/survivor/parsimony.rs`,
  `src/engines/island/{mod,configuration,migration,topology}.rs`.
  Attribution on copy: keep the Apache-2.0 notice (files carry `//!` doc
  headers, no per-file license banner — record crate + version + upstream
  path per file in the copy header).
- `source = genevo@0.7.1` (MIT OR Apache-2.0), repo
  `https://github.com/innoave/genevo`, tag `v0.7.1` (`3d4431e`), mirror at
  `vendor/genevo-0.7.1/` (git-ignored).
  Candidate files: `src/simulation/{mod,simulator}.rs`,
  `src/operator/{mod,prelude}.rs`, `src/population/mod.rs`, `src/genetic.rs`,
  `src/ga/builder.rs` (Simulation/Termination/Operator/Population/Genotype
  traits + GenomeBuilder backbone).
- `source = symbios-genetics@0.2.3` (MIT), repo
  `https://github.com/TheJanusStream/symbios-genetics`, checkout `8cd1aa2`,
  mirror at `vendor/sources/symbios-genetics/` (git-ignored).
  Disposition: FORK base for F1 MOME per-bin fronts (not a file copy) —
  fork carries the MIT license + attribution; offer F1 upstream as PR.

## License boundaries (unchanged from E10 v2)

- navette (LGPL-3.0-or-later): cargo dependency only (A5 decided 2026-11-14:
  navette-ga is LGPL-3.0-or-later too); never copied. The dependency-only rule
  is architectural, not a license obstacle.
- mahf-coco (GPL-3.0-or-later): external CLI harness only (§4 rule 5:
  CI enforces the boundary).
- easy_ga / simulated_annealing / genetic (Bucket D): run-only or avoided;
  nothing enters the tree.
