# STAGED-COPIES.md — cherry-pick staging area (gate: E09 v2 §1.1)

Files below are IDENTIFIED and READY, not copied. A module moves from here
into `src/engines.rs` (or the symbios fork) only with written S10 proof that
it required mutation during Phases 0–2. Mirrors live in
`vendor/sources/` (GA, symbios) and `vendor/genevo-0.7.1/` (all git-ignored);
`vendor/` is outside the crate build.

The crate itself is LGPL-3.0-or-later, the same license as navette. Each
source below keeps its own license when copied; all listed licenses are
compatible with LGPL-3.0-or-later.

## 1. VL chromosome + operators — genetic_algorithms@3.0.0 (Apache-2.0)

| Module (upstream path) | Lines | Role in `src/engines.rs` | Adaptation required |
|---|---|---|---|
| `src/types/chromosomes/length.rs` | 41 | `ChromosomeLength::Variable{min,max}` semantics | None (take as-is) |
| `src/operations/mutation/length_mutation.rs` | 170 | `length_insertion/deletion_mutation` skeleton | **Must adapt**: upstream insertion clones a gene from existing DNA; coating insertion must sample from the pool (rung-2 similarity order / rung-3 latent step, E09 v2 §2.3). Deletion maps directly to d_min removal. |
| `src/operations/crossover/variable_length.rs` | 132 | Trim/Pad alignment for unequal stacks | Pad source: upstream pads from own DNA — adapt to pool-aware padding or Trim-only + repair. |
| `src/operations/survivor/parsimony.rs` | ~56+ | `apply_parsimony_pressure` / length-penalty coefficient | Schedule wrapper (V9 dynamic ramp) lives on our side. |
| `src/engines/island/mod.rs` | 738 | IslandGa driver skeleton | Rewire evaluation to navette A3 batch eval; heterogeneous-island configs ours. |
| `src/engines/island/configuration.rs` | 131 | Island configs | As above. |
| `src/engines/island/migration.rs` | 390 | Migration policies | Elite-exchange + tribe masks ours. |
| `src/engines/island/topology.rs` | 119 | Ring/fully-connected topologies | Take as-is. |

NOT staged: `src/operations/mutation/insertion.rs` (91 lines) — verified
permutation-only (TSP-style move-within, no length change). Staging it would
be the exact wrong-module error this ledger exists to prevent.

## 2. Assembly backbone — genevo@0.7.1 (MIT OR Apache-2.0)

| Module (tag v0.7.1) | Role |
|---|---|
| `src/simulation/{mod,simulator}.rs` | Simulation loop |
| `src/operator/{mod,prelude}.rs` | Operator trait + prelude |
| `src/population/mod.rs` | Population/Genotype traits |
| `src/genetic.rs` | Core genetic traits |
| `src/ga/builder.rs` | GenomeBuilder backbone (mahf builder-DSL ergonomics as API reference) |

Dormancy irrelevant for copies (pinned tag); attribute per file on copy.

## 3. QD archive — symbios-genetics@0.2.3 (MIT, fork — not a copy)

Fork base (whole crate): `src/algorithms/{map_elites,cvt_map_elites}.rs` +
`src/algorithms/archive.rs` + `src/speciation.rs`. F1 (MOME per-bin fronts)
is built in-fork, offered upstream as PR, fork shrinks per release (R4).
`novelty_search.rs` / `nsga2.rs` / `simple.rs` carried only as long as the
fork needs them.
