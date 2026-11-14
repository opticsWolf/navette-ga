# E09 v2 — Rust implementation strategy for GA thin-film synthesis (derived from PAPER v4)

> **Naming note (2026-11-14):** the implementation is the single `navette-ga`
> crate; `ga::decoder`, `ga::bridge`, `ga::engines`, `ga::qd`, `ga::study`, and
> `ga::bench` below are modules under `src/`. The strategy content is unchanged.

*Derived from `PAPER_ga_thinfilm_strategies_v4.md` (S1–S10, Recipes A/B,
Appendices A/B) with crate picks from `E00_rust_crates_consolidated.md`
(55 crates). Responds to the independent technical review: round 1 (19
findings, 10 gaps, 10 risks) plus round 2 (E15/E16 migration policy and
parent-pool scope, G11 cross-island recombination, R11 island-policy risk) —
21 findings, 11 gaps, 11 risks. All E09-directed findings repaired, all gaps
scheduled, all risks owned. Ground rule from the user: **navette owns the
optics** — scatter (S-) matrix, not TMM, plus its native needle algorithm.
Everything else is assigned below. Architecture decision (review C3, jointly recorded
with E10 v2): **E10 governs the three named copies** (VL chromosome +
operators, island-driver skeleton, symbios fork) **and E09 governs every
dependency** — one strategy, with the copy gated as below. Owner amendments
A1–A6 are specified in §2.9; the F5/G4(b) upstream-request protocol is gone —
amendments are implemented by the owner, never requested.*

## Implementation checkpoint (2026-11-14)

- **Navette `dev_ga`:** A3 `SmatrixContext::evaluate_many` (`f0748cf`) and
  A1 `synthesis::staged` scan/insert/polish/cleanup (`b02c03f`). The staged
  surface is additive; `run_needle_cycles` is unchanged.
- **`navette-ga`:** G2 decoder, G1 bridge, A1 memetic operator, and Recipe A
  GA paths: single-island and the Phase 2 ring (`genetic_algorithms` ->
  three-key chromosome -> decoder -> navette merit).
- **Phase 1 exit artifact:** G3 rung-2 material loader backed by navette's
  public ordered material evaluation; terminal LM + needle finish with
  Lamarckian genome write-back and an actual-trial scan certificate; C00-class
  AR ledger at `docs/studies/phase1_c00_ar.md` (16 layers, avg R = 0.659 %,
  no actually improving scanned candidate). Full suite: 33 tests.
- **Phase 0 code exit:** quarter-wave and two-layer V-coat checks pass; the G9
  micro-bench covers evaluate, LM, A1 scan, and scan+insert+polish; the memetic
  operator calls the stages, not a document round-trip.
- **Release gate:** A1/A3 are not in published navette 0.10.0. The bridge is
  local-path-only until a navette release carries them; `cargo publish` is
  expected to fail verification meanwhile.
- **Remaining:** A2 (Jacobian/merit introspection), A4 (native bounds/grammar
  API), Phase 2 of §2.4.1 (joint boundary needles, joint gaps,
  multi-material edges), heterogeneous tribes, and variable-length
  insertion/deletion operators.

## 0. What navette already is (and why the strategy collapses around it)

- **Pure-Rust engine crate** (`rust/navette`, rlib) + PyO3 wheel (`navette-py`,
  maturin, Python 3.12+). Rust + rayon data-parallelism across the full
  (wavelength × angle) grid; global S-matrix collapse for cache efficiency;
  **O(N) scaling** in layer count; three coherence modes with refusal instead
  of guesses; 16 dispersion models + 6 EMA rules as data; graded media,
  guided-mode eigensearch, Jones/Stokes polarisation, differential-phase
  observables.
- **Native needle pipeline** (`synthesis/`, native `DesignStack` driver):
  infinitesimal-test-layer scan at every candidate position, insertion where
  joint merit improves most (Tikhonravov, merit-driven, target-aware);
  **bounded LM thickness optimisation** ("builtin": QR step solve, gain-ratio
  damping, analytic Jacobian; bounds by step veto + clamp, so a thickness
  *may finish exactly on a bound — which is how the synthesis loop learns a
  film wants removing*); impact-ranked cleanup (merge, thin-layer removal,
  re-optimisation); multi-domain joint merit (spectral + angular + CIE color,
  per-target λ windows); **multi-environment joint synthesis**
  (`run_needle(design, environments)` — one film set, K surroundings, honest
  linear cost); program documents compiling to bit-identical merit.
- **Published benches**: one LM thickness optimise 2.1 ms (re-fold 9 %); one
  full synthesis run on AR 61λ, 4→26 films (eleven interior insertions at +2
  layers each, §10.4 arithmetic): scan 2.2–2.6 ms per insertion, LM
  re-optimise 19–66 ms per insertion — *the scan share runs 3.2–12.3 % across
  the stated corners (≈4 % only at the slowest re-optimisation end); time
  goes to re-optimisation*. Consequence for budgeting (§5): merit scans are
  nearly free; LM finishes dominate; size GA populations accordingly.
- **Our refiner picks are already inside it**: optional backends `opt-basin`
  (**basin**), `opt-argmin` (**argmin**), `opt-minpack-lm` — all MIT/Apache,
  all declared LGPL-compatible by upstream. The S2 memetic wiring therefore
  runs *through navette's own optimizer abstraction*, not around it. (`argmin`
  is retired as a direct dependency of this stack — use basin — and retained
  solely as navette's optional engine backend; the two statements are
  compatible and this note is the cross-reference.)
**Amendment path (owner-controlled):** where the README documents only the
program-document flow (`run_needle`), §2.9 specifies the staged native APIs
to add (A1–A6). Default build stays lean behind feature gates; GA-facing
surface is additive only.

S-matrix vs TMM consequences (v4 Appendix B is the governing map, physics
upgraded): coherent blocks / incoherent flags are now first-class (design the
decoder to flag thick layers); per-λ reference-index columns and
differential-phase keys make phase/delay targets expressible; guided-mode
search adds a failure-mode diagnostic (parasitic modes) no TMM loop had. V16's
TMM speed evidence is re-benched against the §0 numbers above; "TMM adjoint"
wording is corrected everywhere to S-matrix-adjoint-or-A2-Jacobian (which
prices thickness moves, not insertion positions — §2.6 enumerates exactly
which descriptor gradients exist).

## 1. Subsystem → crate assignments

| Subsystem (v4 strategy) | Crate (version) | Why this one |
|---|---|---|
| Optics + merit + needle + LM (S1/S2 core) | **navette 0.10.x** + `opt-basin` | Native needle, bounded LM w/ removal signal, joint merit, benches; basin backend = E00 refiner pick already integrated |
| VL island GA (S1/S2/S3) | **genetic_algorithms 3.0.0** (dependency in Phases 0–2; gated copy in Phase 3, §1.1) | Only VL + length penalty + IslandGa + memetic slot + batch eval (E00 §2.1) |
| Second evolutionary pillar / BO (S7) | **genoxide 0.x** (pin) | BO+PoF, MOEA/D, DE/SHADE; prove on 40-layer filter before committing |
| Pareto split-off (S6) | **moors 0.2.11** (+pymoors plots) | NSGA-III/R-NSGA-II/Age-MOEA/REVEA, feasibility-first (V39) |
| QD archive (S4) | **symbios-genetics 0.2** (dependency; fork for F1, §1.1) | Only QD crate (MAP-Elites/CVT/speciation); MOME + DQD are fresh builds (§4) |
| Lightweight surrogate (S7) | **egobox-gp 0.37.2 + egobox-ego 0.43.0** (Gpx/Egor library crates directly — the umbrella `egobox 1.0.1` crate is only the PyO3 python binding and does not build; E00 erratum 2026-11-10) | Surrogate-only loop without genoxide weight |
| Study layer (S9) | **blackopt 0.1.0 / optimizer** | enqueue_trial (QD bright cells), PartialFixed (pins), pruners, fANOVA |
| Proof harness (S10) | **kurobako** (+ `mahf-coco` CLI-only, mind GPL-3.0) | Registered comparisons incl. V9 ablation, H0–H7 set (§2.7) |
| Prototypes/baselines | **genetic-algorithm 0.28.0**, `evolutionary` 0.5.0, `metaheuristics-nature` 10.1.0 | Scaffolds before framework commitment |
| `easy_ga` | Run-only baseline (never distributed/vendored) | E10 v2 bucket D: license unverifiable |
| Conditional | `rlevo` (RL stage only), `radiate` (species/batch), `oxigen` (age/cache patterns), Evo-Engine (K-means islands, GH-only) | Per-need, per E00 §8 |
| Retired for this build | `nsga_rs`, `argmin` (direct), `darwin-rs`, `rsgenetic`, `ecrs` (watch) | Superseded per E00 |

### 1.1 Architecture decision (review C3, resolved with E10 v2)

E09's depend-on-crates and E10's copy-into-`navette-ga` prescribed different
architectures for the same three subsystems with no reconciliation. The
governing split, recorded identically in both documents:

- **Phases 0–2 depend.** `genetic_algorithms` (VL island GA),
  `symbios-genetics` (QD archive base), `genevo`-style backbone needs — all
  consumed as cargo dependencies. Rationale: fastest path to Phase 1 exit,
  validates the V11 survey claims against the running crates, zero fork drift
  while the science is still moving.
- **Phase 3 copies, gated on measured need.** The `ga::engines` copy (VL
  chromosome + operators, island-driver skeleton, both Apache-2.0) and the
  `symbios` fork (F1 MOME) land only for modules that demonstrably required
  mutation during Phases 0–2. The gate is written proof in S10 ("module X was
  forked because behaviour Y could not be configured"), not anticipation.
  `genevo` 0.7's trait backbone (Simulation/Termination/Operator/Population)
  is adopted at the same gate as the `ga::engines` assembly skeleton — it is
  no longer an E10-only decision.
- **Never copied** (unchanged): navette files (dependency under the same
  license; A5 resolved), Bucket C/D crates, anything with a clean API (moors,
  basin, egobox, blackopt, genoxide, kurobako stay dependencies permanently).

One strategy governs every phase; the copy is a conditional refactor, not a
second strategy.

Boundary rule (learned from navette's `ndarray` + `ndarray017` + moors'
ndarray/faer + basin backends): **cross crate boundaries as `Vec<f64>` +
plain structs, never as ndarray types** — ndarray major-version drift across
four crates is the most predictable build break available. Convert at the
edges; keep the hot loop inside navette types.

## 2. Detailed subsystem design

**2.1 Fitness bridge (G1).** `BatchFitnessEvaluator`-compatible evaluator:
genome → `ga::decoder` (§2.2) → navette `DesignStack` (+ environments) →
joint-merit `f64` (+ per-target breakdown for QD descriptors). Batch a full
generation into one call to amortise stack setup (README: setup 0.009 % —
still, batch). A3 batch evaluation settles the parallelism-nesting question
structurally (one scheduler sees population × λ × θ jointly inside navette)
instead of empirically per campaign; G9 keeps the micro-bench as the check.
Pinned layers and grammar legality are enforced in the decoder, so the bridge
never sees an illegal stack (defence in depth: navette's refusals stay on as
the backstop — "refuses what it cannot honour").

**2.2 Decoder module `ga::decoder` (G2, fresh, ~300 lines).** Owns: VL random
keys → material pool indices (similarity-ordered, §2.3) + thicknesses within
`bounds[m]` + absence thresholds → count; hard-coded layer-1/layer-N pins
excluded from operator masks; adjacency-grammar matrix (construct-legal,
retry ≤20×, then repair); sub-cutoff merge/delete; graded-span pass-through
(foundation layers the needle designs around). Shared by GA, needle seeding,
and repair — v4 §7's one-place rule. Operator masks (which keys mutate) are
part of its API so islands/tribes share legality with different strategies.
Constraint-ownership rule (review R2): the decoder derives its tables from
navette's A4 physics API at startup — bounds, pins, and grammar each have
exactly one owner, the sync direction is decoder-from-navette, and no second
source of truth exists at runtime. Navette's `NeedleRules` (insert rules plus
edge lists) is a host/needle and edge contract, not the adjacency grammar;
§2.4.1 fixes the cross-check before either table is populated.

**2.3 Material pool loader (G3, fresh glue + offline step).** Navette materials
are data (model name + params) → load pool → similarity-order (rung 2 minimum:
sort by index-contrast/dispersion-slope, adjacent-swap mutation) → offline
latent embedding (rung 3: VAE-tSNE per B09 in Python, export 2-D table; genome
mutates in latent space, decoder maps to nearest pool member). G10-style pool
illumination (merit-potential vs contrast/slope/edge from S-matrix sweeps)
seeds islands from bright cells. Pool swaps = one table change, no retraining
(v4 §6).

**2.4 Needle-as-mutation wiring (G4).** Two integration depths, in order:
(a) *Program-level (now, no navette changes):* memetic operator shells a
`run_needle` pass — genome → design document → fixed small insertion budget
(M ≤ 2) + bounded-LM polish via `opt-basin` → parse back → Lamarckian
write-back (V21 deviation flagged; Baldwinian variant as the ablation's second
factor). (b) *Native per-step (A1 implemented by the owner per the schedule in
§5; G4(a) ships first so Phase 1 never waits):* the operator calls A1's
`scan/insert/polish/cleanup` stages directly — no serialisation round-trip,
an order of magnitude less overhead per memetic step. V30 scope respected:
needle steps refine proximity designs mid-loop; cold growth stays stochastic.

**2.4.1 Material-boundary rules (Phase 1 implemented; Phase 2 investigation).**
Navette already exposes the mechanism this crate needs: `InsertRules` maps a
host label to the labels of needles it accepts, `MaterialPool::with_insert_rules`
turns that into the per-host matrix, `NeedleRules { insert, outer, inner,
seed_edges }` applies it to a design and sets the `EdgeMaterials`, and
`boundary_accepts`/`gap_accepts` enforce the rules at interior, boundary, and
shared-design loci. This is the "needle material boundary" to reuse, not
re-invent.

**Phase 1 (implemented).** `StackTemplate` carries a validated `BoundaryRules`
(`insert`, `outer`, `inner`; `seed_edges` is refused). Construction resolves
every label and rejects empty edge lists. `assemble` sets the edge materials
and errors when the decoded first/last films are not on their lists;
`needle_pool` applies the host→needle matrix, and
`MemeticOperator::from_template` uses that pool. A single-material edge list
can also become decoder `Pins` through `edge_pins`; multi-material lists stay
validation-only pending the Phase 2 seed-or-pin choice. No second insert
matrix is stored. `check_grammar` also requires every concrete `H → N` insert
pair to be legal in both directions in the decoder's adjacency grammar, and
`run_recipe_a` calls that check before starting. Tests cover unknown/empty
labels, assembled-edge compliance, edge-to-pin mapping, grammar
bidirectionality, a memetic scan restricted to the allowed needle material,
and a flat boundary needle accepted by the insert rules but rejected when the
inner edge list excludes it.

The remaining investigation has five parts:

1. **Edge lists vs decoder pins — pins, no seeding.** The decoder pins fix a
   material index in the genome; navette's edge lists constrain the built
   stack. Decision: `seed_edges` is not used, because it inserts a film outside
   the genome and breaks layer-count write-back. A single-material list becomes
   decoder pins (`edge_pins`); a multi-material list is checked by `assemble`.
   A contradiction is an assembly error, never a silent layer.
2. **Grammar interaction — consistency check, not derivation.** `InsertRules`
   is host → needle, not neighbour → neighbour. A split needle `H | N | H`
   creates both adjacencies, so Phase 1 requires every concrete `H → N` pair
   to be legal in both directions; `check_grammar` reports a conflict instead
   of silently deriving a second grammar. The `"*"` host is rejected by
   `BoundaryRules` because template films always resolve to a pool entry, so it
   would silently do nothing. The grammar stays the decoder's single source of
   adjacency truth.
3. **Boundary needles.** The flat staged `scan` is now covered: with
   `boundary_needles` on, a boundary needle is accepted when both hosts allow
   it and rejected when the edge list excludes it. The joint `gap_accepts`
   path, multi-environment equivalence, and the no-edge-move assertion for
   polish/cleanup remain.
4. **Evidence.** Phase 1 covers label resolution, empty/unknown edge lists,
   assembled edges, edge-to-pin mapping, grammar bidirectionality, and a
   restricted scan, a flat boundary needle filtered by host and edge rules,
   multi-material validation, and the `"*"` rejection. Phase 2 adds
   contradiction diagnostics, a multi-environment gap, equivalence with
   `run_needle_cycles` under the same `NeedleRules`, and a diffusion-barrier /
   edge-layer filter as the acceptance case; no-rules runs stay bit-identical.
5. **Scope.** Phase 1 is complete. Phase 2 adds joint boundary needles, joint
   gaps, and multi-material joint validation.

**2.5 MOO split-off.** `moors` builders with fitness closures over the G1
bridge; objectives (merit, N, total thickness, robustness perturbation,
angle-aware terms per V31); constraints via `impl_constraints_fn!`
(feasibility-first, V39); R-NSGA-II reference points for guided targets (v4
§12). Scalarise last. Angular robustness is composed deliberately (review R7):
multi-environment synthesis is the primary mechanism (native to navette);
angle-aware selection is the fallback for objectives the environments cannot
express — never both on the same objective unexamined.

**2.6 QD archive (G5/G6 + F-builds).** `symbios-genetics` MAP-Elites/CVT over
descriptors computed from navette outputs. Descriptor-gradient enumeration
(review G5 — exactly which descriptors enter the DQD loop): **gradient path**
(total thickness, merit-derived quantities via the A2 Jacobian — continuous
and priced by the builtin LM); **standard archive only** (layer count, motif
flags from field profiles `|E(z)|`, DBR-presence — not differentiable, modelled
by BOP-Elites/GPs instead). SAIL loop for full-wave cases (`symbios` archive
+ `egobox` GPs + UCB/EI). DQD upgrade (V32): wire descriptor gradients into a
CMA-MAE loop before reaching for GP surrogates. MOME per-bin fronts: fresh
feature (§4 F1); the molecule-to-coating transfer is registered as hypothesis
(review G10), not settled evidence.

**2.7 Study + proof.** `blackopt`/`optimizer` study owns the campaign: enqueue
QD bright cells and diffusion/LLM proposals, PartialFixed pins, pruners kill
dead tribes, fANOVA explains. `kurobako` problem definition wraps Recipes A/B
+ the H0–H7 comparison set + V9/V21 ablations; `mahf-coco` for publication
comparisons (external CLI only — GPL boundary rule, §6.5). Statistical
protocol (review G2, mandatory for every stochastic arm): fixed seeds,
repeats, distributional reporting (box plots / expected running time),
rank-sum tests with multiplicity correction. Classical control arm H0
(review G1): needle-plus-refinement multi-start under equal compute — C00's
own programme, the strongest competitor at 5–10 layers, scheduled first so a
negative result narrows S2–S4 honestly instead of late.

Island-policy experiment I1 (review G11, round 2): cross-island multi-parent
crossover is registered separately from H0–H7 and follows the same fixed-seed
protocol; it reports merit/depth at fixed budget plus a homogenisation metric
(pairwise island distance / material-mix entropy). The default remains
migrate-then-mate-locally; §2.8.1 fixes the migration policy.

**2.8 Recipe mapping and island policy.** Recipe A: G1+G2+G3(rung 2) + single
island + terminal navette needle + LM. Recipe B: full stack — seeding (S8
proposals via enqueue, mined-motif fallback armed), S5 rung 3, S3 heterogeneous
islands/tribes under the §2.8.1 island policy, S2 via G4, S4 archive
throughout, S6 moors split, S7 screening past TMM, S10 registration. I1
(cross-island multi-parent) is an experiment on top, not part of the default
stack.

**2.8.1 Round-2 island policy (E15/R11; mirrors PAPER v4 §9).** "Every k
generations" is not a policy. Recipe B fixes the minimum below; every value is
a heuristic design decision to be ablated, not a literature constant.

| Knob | Default | Evidence / rationale |
|---|---|---|
| Topology | Ring; extended ring (`Ring+1+2`) is the first connectivity upgrade via `Custom` in Phase 3; fully connected is an ablation | D01: extended rings strongest, fully connected among the weakest for DE; D13: ring/torus can beat a single receiver at low noise (not at scale); `IslandGa` defaults to Ring |
| Interval | `k = 10`; ablate {5, 10, 20, 30} | `IslandGa` default 10; its docs give 5–20 as typical; D06's period 30 is the slow-flow end |
| Rate and selection | One best individual per island per event (`migration_count = 1`), capped at 5% of P | `IslandGa` default 1; its docs say 1–2 is sufficient and more homogenises; D06's 5%-of-best is the high-rate ablation |
| Replacement | Overwrite the destination's worst only if the migrant is better; then §7 legality repair; never overwrite the destination's best | D01: incoming replace worst "provided that they were better"; D13: replace only if fitter. `IslandGa`'s `BestReplaceWorst` is unconditional, so the Phase 2 wrapper applies the check itself and the Phase 3 copy adds it natively |
| Timing | After evaluation of generation k, before the next breeding cycle | `IslandGa` migration cycle |
| Parent pool | Migrants join the destination pool; recombination is within-island by default (E16) | I1 is the only cross-island exception |

**2.8.2 Evidence and implementation.** Verified against `research/kb.db`
(2026-11-14): `D01` defines the migration topology/rate/frequency parameters
and uses elitist "best replace worst if better"; `D06` uses a ring, period 30,
5% best migrants; `D13`'s single-receiver model samples k parents from
different islands at a migration event (the I1 reference) and replaces only if
fitter; `D02` keeps adaptive island counts as an experiment. The dependency
source `vendor/sources/genetic_algorithms/src/engines/island/` confirms the
`IslandGa` defaults (4 islands, interval 10, count 1, Ring,
BestReplaceWorst); `migration.rs::replace_worst` replaces unconditionally, so
the Phase 2 wrapper and Phase 3 copy add the fitness check. Phase 2's wrapper
is now `engines::run_recipe_a_islands`: it advances each island through the
single-population `Ga` one generation at a time and applies the check itself,
because the dependency's `IslandGa` exposes no pre-replacement hook. Phase 3's
copied skeleton will carry the check natively.
I1 is implemented only after the baseline policy and H0 control are measured,
and is reported separately from H0–H7.

Ablations before freezing Phase 2 exits: I1 vs migrate-then-mate-locally on
merit/depth and homogenisation; migration rate 1 / 2 / 5%; plain ring vs
extended ring; fully connected only as the low-diameter ablation.

**2.9 Navette amendments (A1–A6, owner-implemented)**

- **A1 — staged needle API.** Split the `run_needle` pipeline into callable
  stages on `DesignStack`: `scan() -> Vec<NeedleCandidate>` (position, sign,
  predicted δF), `insert(candidate)`, `polish(scope: PolishScope)` (free-set
  subset: all / last-K / named), `cleanup() -> CleanupReport` (merged/removed
  indices). The GA memetic operator (G4) then calls stages directly — no
  document round-trip. `run_needle` becomes a thin loop over the same stages
  (no behaviour change, existing benches stay valid).
- **A2 — optimiser introspection for DQD (G6).** Expose the builtin LM's
  analytic Jacobian (`jacobian() -> DMatrix<f64>`, merit × free-thickness) and
  per-target merit breakdown (`merit_parts() -> Vec<(TargetId, f64)>`) so the
  QD/DQD loop reads descriptor gradients without re-solving. Feature-gated
  (`ga-introspect`) to keep default builds lean. (Honesty note per v4
  Appendix B: this Jacobian prices thickness moves, not insertion positions —
  it is not a P(z) adjoint and is never described as one.)
- **A3 — batch evaluation.** `evaluate_many(&[DesignStack]) -> Vec<Merit>`
  with rayon over population × λ × θ jointly *inside* navette — settles the
  §2.1 nesting question structurally (one scheduler sees the whole grid)
  instead of empirically per campaign.
- **A4 — decoder-adjacent physics in navette (moves v4 work item 5 inward).**
  Per-material default thickness bounds from material metadata
  (`bounds[m]`, overridable per run); pinned layer indices excluded from the
  LM free set *and* the needle scan mask; adjacency-grammar hook as a
  validator trait (`GrammarCheck: fn legal(a: MatId, b: MatId) -> bool`)
  consulted at construct/insert/migrate time. The external `ga::decoder`
  (G2) keeps genome/key logic; navette owns legality — one owner per side of
  the boundary, decoder tables derived from this API at startup (§2.2 rule).
  The consumer side reuses the existing `NeedleRules` (`InsertRules` plus
  outer/inner edge lists) through §2.4.1; A4 does not create a parallel
  boundary-rule API.
- **A5 — licensing decision (owner call, before Phase 2).** Options: keep
  LGPL-3.0 (research-safe, product needs dylib/process boundary per §6.1);
  add an explicit static-linking exception for Rust rlib use; or dual-license
  the pure engine crate MIT/Apache while keeping bindings LGPL. Any option
  dissolves §6.1 as a project risk; record the choice in S10. (Review R3: the
  decision gates any distribution story — decide on schedule, not late.)
  **Decision (2026-11-14): keep LGPL-3.0-or-later; `navette-ga` uses the same
  license as navette. The dependency-only rule for navette files is
  architectural, not a license obstacle.**
- **A6 — P(z)-readout amendment (new; from review C2/P0-3).** *Specified, not
  scheduled:* expose per-position insertion gains from an S-matrix adjoint IF
  the adjoint is derived and sign-checked against a second source
  (Tikhonravov 1982) per v4 Appendix A. Until A6 lands, the empirical scan IS
  the certificate of record and every exit criterion is stated in scan terms
  (§5) — exit criteria that cannot be measured get silently waived, so none
  here invoke P(z).

## 3. Glue inventory (must-write, sized)

| # | Glue | Size | Depends on |
|---|---|---|---|
| G1 | Fitness bridge (genome → DesignStack → merit + breakdown, batched) | ~200 lines | navette 0.10.x + A3 (owner) |
| G2 | `ga::decoder` (VL keys, bounds[m], grammar, pins, masks) | ~300 lines | — (first code, v4 work item 5) |
| G3 | Pool loader + similarity order + latent table import + illumination sweep | ~150 + offline Python | navette materials-as-data |
| G4 | Needle memetic operator (A1 stages; §2.4.1 boundary rules) | ~150 lines | A1 (owner) |
| G5 | QD descriptors from navette outputs + archive glue | ~150 lines | symbios 0.2, navette compute keys |
| G6 | DQD wiring (A2 Jacobian → descriptor gradients → CMA-MAE loop) | ~200 lines | A2 (owner) |
| G7 | Study wiring (enqueue/pin/prune/fANOVA callbacks) | ~100 lines | blackopt/optimizer |
| G8 | kurobako problem + A11 suite baselines + H0–H7 set | ~150 lines | G1 |
| G9 | Perf harness (nesting check, bench vs README 2.2–2.6/19–66 ms) | ~100 lines | G1 |

Total glue ≈ 1,500 lines + offline embedding script. No glue duplicates a
crate: navette owns physics/merit/needle/LM; the `navette-ga` crate (E10 v2)
stands on E00 picks.

## 4. Fresh implementations (no crate does these)

- **F1 — MOME per-bin Pareto fronts in `symbios-genetics`** (month-scale,
  highest leverage, v4 work item 1): local fronts per archive cell; propose
  upstream, build as a fork first (per §1.1 gate). Unblocks S4's core.
- **F2 — Learned-descriptor loop** (quarter): autoencoder/contrastive
  descriptors over spectra/fields (G12 lineage) replacing hand axes; BOP-Elites
  (G06) already models black-box descriptors — F2 supplies better ones.
- **F3 — Diffusion/LLM proposal operator** (quarter): stack-string prior
  (V38 shared representation) + named-move LLM operator with PCFBench
  ablations; logged, credit-assigned, rate-limited (v4 §14 guardrails).
  Availability/licensing/cost unassessed (review G7) — mined-motif seeding is
  the armed fallback, and the operator ships model-independent regardless.
- **F4 — POME offline pass** (weeks): ensemble-pessimistic proposals from run
  logs with zero new solves (G08); pure glue + egobox ensembles, listed here
  because no off-shelf crate composes it.

## 5. Build order with exit criteria

- **Phase 0 — pins + amendments + ground truth (days):** `navette = "=0.10.0"`,
  `rust-version 1.90` / toolchain 1.98.0, G9 micro-bench reproducing README
  numbers; land A1+A3 (smallest surface, unblocks G1/G4), spec A2/A4. Closed-form
  unit checks gate the bridge (review G9): single quarter-wave layer and
  two-layer V-coat with
  known optima — the cheapest end-to-end correctness check available. Exit:
  scan/LM timings within 2× of published; memetic operator calls stages, not
  documents; unit checks green.
- **Phase 1 — Recipe A (1–2 weeks engineering; a day's compute):** G2 + G1 +
  G3(rung 2) + genetic_algorithms single island + terminal needle/LM; consume
  navette `NeedleRules` through §2.4.1 (insert/edge lists, no seeding);
  reproduce a C00-class AR (Fig. 2.5 lineage) under the scan certificate — no
  scanned candidate insertion improves the joint merit, LM converged,
  d_min-stable (review C2; discrete scope stated, P(z)≈0 reserved for
  post-A6 campaigns). Exit: fabricated-plausible stack + ledger entry + no
  forbidden needle placement under the rule matrix.
  **Status:** implemented; the ledger is `docs/studies/phase1_c00_ar.md`
  (16-layer H/L AR, 400–900 nm, avg R = 0.659 %, LM converged, d_min at the
  configured 10 nm bound, zero actual improving scanned candidates).
- **Phase 2 — Recipe B core (month):** heterogeneous islands/tribes under the
  §2.8.1 island policy (ring, k=10, one best migrant,
  best-replace-worst-if-better with legality repair; the ring driver is
  implemented as `run_recipe_a_islands`), G4(a) needle-memetic,
  Phase 2 of §2.4.1 (joint boundary needles, joint gaps, multi-material edges),
  dynamic parsimony, moors split (S6), rung-3 pool, H0 control arm running in
  parallel (review G1/R6: if the control wins at equal compute, S2–S4 narrow
  to refinement machinery honestly). Exit: 40-layer filter + merit-vs-depth
  map (S4-lite via symbios grid) + a diffusion-barrier/edge-layer rule case. Budget box (review
  G8, parametric): population evals dominate — P individuals × G generations
  millisecond-scale solves is tens of minutes at 64×200; memetic overhead per
  round (top-m designs × scan-ms + LM-tens-of-ms) stays single-digit percent
  at every-k cadences — the cadence k is the budget knob; redo this arithmetic
  with measured numbers before freezing Phase 2 exits (review R8).
- **Phase 3 — archives + studies (month):** F1 MOME, G5/G6 (SAIL/DQD),
  G7 study layer, G8 registration, I1 cross-island multi-parent experiment
  (G11, reported separately from H0–H7), §1.1 copy gate evaluated. Exit: the
  H0–H7 comparison set (H1–H7 = S1/S2/S3/S4/S6/S7/S8 embodiments, H0 =
  classical needle-plus-refinement multi-start; review R9) rerunnable by a
  third party under the §2.7 statistical protocol.
- **Phase 4 — frontier (quarter):** F2/F3/F4, full-wave evaluator entry via
  SAIL. Exit: V9/V21 ablations published, V12 re-cited.

## 6. Risks (review R1–R11, owned)

1. **§10 sign errors → future P(z) feature (R1, high but contained):** the
   empirical scan stays the needle of record; M1–M3 repaired in v4 Appendix A;
   no P(z) code before the A6 second-source sign-check.
2. **Two constraint sources (R2, high):** §2.2 sync rule — decoder derives
   from the A4 API at startup, single owners, fixed direction.
3. **LGPL decision timing (R3, resolved 2026-11-14):** the crate is
   LGPL-3.0-or-later, same as navette; the G1 trait boundary limits churn
   either way.
4. **Fork/version churn (R4, medium):** E10 v2 maintenance rules govern; owner
   + per-release cadence assigned for shrinking the symbios fork and the
   §1.1-gated copies.
5. **GPL boundary (R5, medium):** mahf-coco CLI-only plus an explicit CI rule
   (E10 v2) — license-dependency checks alone are not the boundary.
6. **Science risk: control may win at 30–80 layers (R6, medium):** H0 runs
   from Phase 2; a control victory narrows the strategy honestly.
7. **Redundant angular mechanisms (R7, low):** decided in §2.5 —
   multi-environment primary, angle-aware fallback.
8. **Memetic cost share (R8, low):** §5 budget box before Phase 2 exits;
   cadence k tunes it.
9. **Undefined comparison set (R9, low):** H0–H7 enumerated in §2.7/§5 —
   the Phase 3 exit is now evaluable.
10. **Scale claims past the teaching example (R10, low):** v4 §1 scopes them
    as hypothesis (work item 6: catalogue witness or narrow the claim).
11. **Island policy homogenises or starves heterogeneous islands (R11,
    low–medium):** §2.8.1 fixes ring topology, k=10, one best migrant,
    best-replace-worst-if-better, and within-island mating. Validate against a
    single-island ablation and I1 under the §2.7 protocol; if pairwise island
    distance collapses, raise k or lower the migration rate before adding
    operators.

*Sources: navette crates.io meta 2026-11-09, upstream README, `rust/navette/
Cargo.toml` (features incl. opt-basin/opt-argmin/opt-minpack-lm), PAPER v4
(§§2–16 + Appendices A/B), E00 (crate picks), E05–E08 (strategies/ledger),
E10 v2 (governing-split decision, license buckets), independent technical
review (findings/gaps/risks register — every item above traces to an ID).*
