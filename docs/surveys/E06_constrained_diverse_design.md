# E06 — Diverse + optimal thin-film designs under real constraints: a design guide

*Analysis date 2026-11-09. Mined from all tracks (A/B/C/D/E/F/G + PAPER + E05).
Each section: goal → mechanisms (with graph sources) → crate mapping →
concrete setting. §12 gives the assembled architecture.*

## 1. The governing idea: replace single-champion search with illuminated archives

Every demand below (fewer layers, pool limits, pinned ends, order rules) is a
*dimension of variation*, not just a constraint. The G-track result (G01, G05,
G06, G09) says: keep an archive keyed by those dimensions holding the best
merit per cell (MAP-Elites), a local Pareto front per cell where objectives
conflict (MOME, G09), and model fitness + descriptors with surrogates where
evaluation is expensive (SAIL G07, BOP-Elites G06). Diversity and optimality
stop competing the moment the archive — not a scalar — is the product.

## 2. Minimising layer count (without fixing it)

Four cooperating mechanisms (PAPER §5 + E02 + E05):
- **Insertion/deletion asymmetry**: delete slightly more often than insert
  once past a warm-up; count drifts down while merit selection holds quality.
- **Dynamic parsimony ramp**: penalty λ(t)·N with λ growing over generations
  (F08 adaptive-penalty class) — explore deep early, compress late.
- **N as a Pareto objective** (Ch.12, B05, moors/E03): display the
  merit-vs-depth curve; the fab picks the knee. Strictly better than any
  penalty weight for the 40/80-layer decision.
- **MOME per-bin fronts** (G09, §11.3 below): each (depth, thickness) cell
  holds its own trade-off front — minimisation becomes map-reading.

## 3. Flexible layer count, never predefined

Native answer: `ChromosomeLength::Variable(n_min, n_max)` + unequal (Pad)
crossover + absence keys in the BRKGA decoder (keys below threshold = layer
absent, A09/E02). Growth precedent is old (needle splits layers, C00 §2.4)
and new (NEAT complexification, G04; SMILES string edits without crossover,
G09). Only `genetic_algorithms` offers VL among surveyed crates — verify
this before committing; everything else needs fixed-length workarounds.

## 4. Limited materials from a pool — the three-rung ladder

- **Rung 1, fixed menu** (B03: 2-material solar absorber via deep Q-learning;
  D07: per-position materials): fastest, least expressive.
- **Rung 2, pool index**: decoder key → pool slot; uniform reindex is a
  cataclysm (tantala→silver), so mutate by *adjacent swap in a
  similarity-ordered pool* (sort pool by index/dispersion first).
- **Rung 3, semantic latent space** (B09: 300 materials VAE-tSNE-embedded by
  N(λ) similarity; G09: ChemBERTa→UMAP manifold for CVT niches): mutate with
  small Gaussian steps in dispersion space — tantala→niobia, never
  tantala→silver. Division of labour that works: RL/A3C agents on materials,
  GA on thicknesses underneath (B09, zero-human-intervention demo).
- Cap pool size explicitly (B03 proves 2 materials can win absorbers) and let
  the archive (§1) show what each extra material buys.

## 5. Fixed layer types at start/end positions

Do not constrain — **remove from the genome**: hard-code layer 1 and layer N
in the decoder (adhesion layer on substrate, protective cap in air), exclude
from mutation/crossover/migration (PAPER §7). Equivalents: `PartialFixed`
sampler (`blackopt`), pinned genes with masked operators. Repair rule: any
operator output touching a pinned position is overwritten, not penalised —
repair beats penalty for equalities (A06).

## 6. Constrained thicknesses (global bounds)

`[d_min, d_max]` per run (e.g. 1.5 nm metrology floor, stress-driven ceiling):
repair-first (clamp + delete sub-cutoff + local re-optimise = miniature
needle step, A06/C00), penalties only for soft preferences (F08 taxonomy:
static → dynamic → adaptive → death for hard violations). `basin` contributes
ProjectedGD/L-BFGS-B (projection is repair inside the optimiser); `mahf`
`boundary::Saturation`; `moors` feasibility-first selection.

## 7. Constrained thicknesses per material type (deposition physics)

Bounds live in a **material-indexed table**, not one global pair: metals need
continuity minima (island growth below ~5–10 nm), dielectrics have stress
ceilings, absorbers have skin-depth relevance floors. Implementation: decoder
owns `bounds[m] = (lo, hi)`; mutation draws within the active material's
bounds; material-swap mutation re-clamps thickness into the new material's
bounds (or resamples — resampling explores better). Repair order matters:
snap material → clamp thickness → merge/delete → re-optimise. No surveyed
crate does this natively; it is ~30 lines in a custom decoder over
`genetic_algorithms` or `genoxide`.

## 8. Constrained material order (adjacency grammars)

Forbidden/required adjacencies (diffusion barriers between metal and silver,
no absorber directly on substrate, DBR pairs must alternate) form a small
grammar. Three enforcement levels, cheapest first: **decoder construction**
(only generate legal neighbours — like G09's validity-preserving SMILES
edits, retry ≤20× then new parent); **repair** (insert barrier / swap pair);
**penalty/death** (F08) as backstop for island migrants that break the
grammar. Encode the grammar once as an adjacency matrix shared by all
islands — migrants stay legal, tribes can still differ in *strategy* (§9).

## 9. Populations: islands, tribes, ages, species

- **Islands** (D01/D06/D13/D15/D17): ring migration of elites every k
  generations; *heterogeneous* islands (different operators/rates per island,
  e.g. silica-rich vs thin-absorber vs many-layer populations, PAPER Fig.4)
  beat homogeneous rings. `genetic_algorithms` IslandGa + `genoxide` islands.
- **Tribes/CTCM** (D11): strategy-level diversity — tribes run different
  search logics and compete/cooperate at member level. Assign tribes to
  constraint regimes (one tribe depth-minimising, one merit-maximising, one
  robustness-maximising) and let migration arbitrate.
- **Age layers/ALPS** (`oxigen` age-unfitness): protect young deep stacks
  from immediate competition with refined shallow ones — the growth-vs-polish
  conflict (§2) resolved structurally.
- **Speciation** (`symbios-genetics` NEAT-style sharing; `genetic-rs`):
  fitness sharing by stack-similarity distance keeps Bragg-mirror clones
  from eating the population (the B10 phase-shift-collapse failure).
- **Adaptive sizing** (D02 Lässig): grow population with problem difficulty;
  **Hall-of-Fame + min-distance** (E02) as the archive's memory.

## 10. Guided targets while optimising (curriculum + steering)

Do not attack the full spec on generation 0:
- **Spectral curriculum**: single wavelength → band → band+angle → band+angle
  +robustness; each stage seeds the next (warm-start = `oxigen` resume,
  `basin` warm states). Monotone: merit weights only tighten.
- **Reference-point steering** (R-NSGA-II, `moors`): an aspiration point in
  (merit, N, thickness) space pulls the front without scalar weights.
- **Human-in-the-loop via the archive**: inspect the QD map mid-run (G07
  prediction map), freeze bright cells (`blackopt` enqueue_trial), kill dead
  regions (pruners: Median/Hyperband, `optimizer`). The map makes steering
  visual instead of alchemical.
- **Target-conditioned seeding**: D04 token models / diffusion priors (E05.6)
  propose on-target starting stacks; GA owns everything after.

## 11. LLM-assisted guidance (roles + guardrails, A08/PCFBench)

Useful roles, weakest-first: **rule miner** (B10-style motif extraction from
archives — safest, fully auditable); **proposal seeder** (named initial
stacks → enqueue); **named-move mutator** ("insert 2H needle at field
antinode", "swap to higher-contrast pair") called at low rate alongside
blind operators, credit-assigned like any operator (E02); **analyst**
(fANOVA-style importance + run summaries). Guardrails from PCFBench
(2610.32485): ablate whether the model reasons about TMM physics or
pattern-matches old designs; keep blind operators as the control arm; never
let the LLM set weights or constraints unsupervised. No Rust crate bundles
this — wire API calls as a custom operator; log every suggestion for audit.

## 12. Assembled architecture (what to build)

```
pool+grammar+bound-tables → decoder (VL keys, pinned ends, legal neighbours)
  → heterogeneous islands/tribes (depth/mert/robustness regimes, ring migration,
     age protection, speciation, adaptive sizing)
  → QD+MOME archive (depth × thickness cells, local fronts; SAIL/BOP-Elites
     surrogates when evaluator is full-wave; POME pass over logs)
  → memetic finish (needle insertion + basin L-BFGS-B/NM, Lamarck or Baldwin)
  → Pareto front (moors) + robustness objectives (D17/B05) → fab picks knee
  steering: curriculum targets, reference points, map inspection, LLM moves
```

Crate assignments: `genetic_algorithms` (VL islands) + `moors` (fronts,
feasibility) + `basin` (refiners) + `symbios-genetics` (+MOME feature, §11.3
of E05) + `egobox`/`genoxide` (surrogates) + `blackopt`/`optimizer` (study
layer: enqueue, pinning, pruning, fANOVA) + `mahf-coco`/`kurobako` (proof).
Custom code stays where it belongs: decoder (bounds/grammar/pinning),
descriptor functions, LLM operator, archive glue.

## Sources
A06/A08/A09/A10/A11, B03/B05/B09/B10, C00/C01, D01/D02/D04/D06/D11/D13/D15/D17,
E01–E05, F01/F02/F05/F07/F08, G01/G05–G10, PAPER §§5–9/12–15.
