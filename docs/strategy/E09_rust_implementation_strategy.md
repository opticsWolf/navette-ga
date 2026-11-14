# E09 — Rust implementation strategy for GA thin-film synthesis (derived from PAPER v3)

*Derived from `PAPER_ga_thinfilm_strategies_v3.md` (S1–S10, Recipes A/B) with
crate picks cherry-picked from `E00_rust_crates_consolidated.md` (55 crates).
Ground rule from the user: **navette owns the optics** — scatter (S-) matrix,
not TMM, plus its native needle algorithm. Everything else is assigned below.
*Owner note (2026-11-09): navette is the user's own crate — it can be amended
and adapted as required. All G4(b)/G6 VERIFY items below are therefore recast
as **navette amendment specs (A1–A5)** in §2.9; F5 (upstream request) is
deleted. Churn risk (§6.3) is owner-controlled; LGPL (§6.1) is a licensing
decision, not a constraint.*

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
  needle insertion on AR 61λ, 4→26 films: scan 2.2–2.6 ms, LM re-optimise
  19–66 ms — *the scan is ~4 % of an insertion; time goes to re-optimisation*.
  Consequence for budgeting (§5): merit scans are nearly free; LM finishes
  dominate; size GA populations accordingly.
- **Our refiner picks are already inside it**: optional backends `opt-basin`
  (**basin**), `opt-argmin` (**argmin**), `opt-minpack-lm` — all MIT/Apache,
  all declared LGPL-compatible by upstream. The S2 memetic wiring therefore
  runs *through navette's own optimizer abstraction*, not around it.
**Amendment path (owner-controlled):** where the README documents only the
program-document flow (`run_needle`), §2.9 specifies the staged native APIs
to add (A1–A5) instead of VERIFY-and-fallback. Default build stays lean
behind feature gates; GA-facing surface is additive only.

S-matrix vs TMM consequences (§7 of v3 stands, physics upgraded): coherent
blocks / incoherent flags are now first-class (design the decoder to flag
thick layers); per-λ reference-index columns and differential-phase keys make
phase/delay targets expressible; guided-mode search adds a failure-mode
diagnostic (parasitic modes) no TMM loop had.

## 1. Subsystem → crate assignments

| Subsystem (v3 strategy) | Crate (version) | Why this one |
|---|---|---|
| Optics + merit + needle + LM (S1/S2 core) | **navette 0.8.0** + `opt-basin` | Native needle, bounded LM w/ removal signal, joint merit, benches; basin backend = E00 refiner pick already integrated |
| VL island GA (S1/S2/S3) | **genetic_algorithms 3.0.0** | Only VL + length penalty + IslandGa + memetic slot + batch eval (E00 §2.1) |
| Second evolutionary pillar / BO (S7) | **genoxide 0.x** (pin) | BO+PoF, MOEA/D, DE/SHADE; prove on 40-layer filter before committing |
| Pareto split-off (S6) | **moors 0.2.11** (+pymoors plots) | NSGA-III/R-NSGA-II/Age-MOEA/REVEA, feasibility-first (V39) |
| QD archive (S4) | **symbios-genetics 0.2** | Only QD crate (MAP-Elites/CVT/speciation); MOME + DQD are fresh builds (§4) |
| Lightweight surrogate (S7) | **egobox 1.0.1** (Gpx/Egor) | Surrogate-only loop without genoxide weight |
| Study layer (S9) | **blackopt 0.1.0 / optimizer** | enqueue_trial (QD bright cells), PartialFixed (pins), pruners, fANOVA |
| Proof harness (S10) | **kurobako** (+ `mahf-coco`, mind GPL-3.0) | Registered comparisons incl. V9 ablation |
| Prototypes/baselines | **genetic-algorithm 0.28.0**, `evolutionary` 0.5.0, `metaheuristics-nature` 10.1.0 | Scaffolds before framework commitment |
| Conditional | `rlevo` (RL stage only), `radiate` (species/batch), `oxigen` (age/cache patterns), Evo-Engine (K-means islands, GH-only) | Per-need, per E00 §8 |
| Retired for this build | `nsga_rs`, `argmin` (direct), `darwin-rs`, `rsgenetic`, `ecrs` (watch) | Superseded per E00 |

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
still, batch). Parallelism nesting: navette rayons over λ×θ *inside* each
eval while the GA rayons over the population — benchmark nested vs
`RAYON_NUM_THREADS=1`-inside-navette with population-level parallelism
outside; for 61λ-class grids population-outer usually wins, for dense
(λ×angle×environment) grids navette-inner wins. Record the decision in S10.
Pinned layers and grammar legality are enforced in the decoder, so the bridge
never sees an illegal stack (defence in depth: navette's refusals stay on as
the backstop — "refuses what it cannot honour").

**2.2 Decoder module `ga::decoder` (G2, fresh, ~300 lines).** Owns: VL random
keys → material pool indices (similarity-ordered, §2.3) + thicknesses within
`bounds[m]` + absence thresholds → count; hard-coded layer-1/layer-N pins
excluded from operator masks; adjacency-grammar matrix (construct-legal,
retry ≤20×, then repair); sub-cutoff merge/delete; graded-span pass-through
(foundation layers the needle designs around). Shared by GA, needle seeding,
and repair — v3 §7's one-place rule. Operator masks (which keys mutate) are
part of its API so islands/tribes share legality with different strategies.

**2.3 Material pool loader (G3, fresh glue + offline step).** Navette materials
are data (model name + params) → load pool → similarity-order (rung 2 minimum:
sort by index-contrast/dispersion-slope, adjacent-swap mutation) → offline
latent embedding (rung 3: VAE-tSNE per B09 in Python, export 2-D table; genome
mutates in latent space, decoder maps to nearest pool member). G10-style pool
illumination (merit-potential vs contrast/slope/edge from TMM/S-matrix sweeps)
seeds islands from bright cells. Pool swaps = one table change, no retraining
(v3 §6).

**2.4 Needle-as-mutation wiring (G4).** Two integration depths, in order:
(a) *Program-level (now, no navette changes):* memetic operator shells a
`run_needle` pass — genome → design document → fixed small insertion budget
(M ≤ 2) + bounded-LM polish via `opt-basin` → parse back → Lamarckian
write-back (V21 deviation flagged; Baldwinian variant as the ablation's second
factor). (b) *Native per-step (VERIFY then build):* confirm `DesignStack`
exposes scan/insert/polish as Rust calls beneath the document flow; if yes,
bind the operator directly (no serialisation round-trip, ~10× less overhead
per memetic step); if no, file the upstream feature request (opticsWolf ships
fast: navette 0.5→0.8 in a year, ikat 0.12→0.13.5 in weeks) and stay on (a).
V30 scope respected: needle steps refine proximity designs mid-loop; cold
growth stays stochastic.

**2.5 MOO split-off.** `moors` builders with fitness closures over the G1
bridge; objectives (merit, N, total thickness, robustness perturbation,
angle-aware terms per V31); constraints via `impl_constraints_fn!`
(feasibility-first, V39); R-NSGA-II reference points for guided targets (v3
§12). Scalarise last.

**2.6 QD archive (G5/G6 + F-builds).** `symbios-genetics` MAP-Elites/CVT over
descriptors computed from navette outputs (depth, total thickness, motif flags
from field profiles `|E(z)|`, DBR-presence). SAIL loop for full-wave cases
(`symbios` archive + `egobox` GPs + UCB/EI). DQD upgrade (V32): navette's
*analytic Jacobian* (builtin LM) is the gradient source — wire TMM-adjoint-style
descriptor gradients into a CMA-MAE loop before reaching for GP surrogates.
MOME per-bin fronts: fresh feature (§4 F1).

**2.7 Study + proof.** `blackopt`/`optimizer` study owns the campaign: enqueue
QD bright cells and diffusion/LLM proposals, PartialFixed pins, pruners kill
dead tribes, fANOVA explains. `kurobako` problem definition wraps Recipes A/B
+ V9/V21 ablations; `mahf-coco` for publication comparisons.

**2.9 Navette amendments (A1–A5, owner-implemented — replaces all VERIFYs)**

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
  (`ga-introspect`) to keep default builds lean.
- **A3 — batch evaluation.** `evaluate_many(&[DesignStack]) -> Vec<Merit>`
  with rayon over population × λ × θ jointly *inside* navette — settles the
  §2.1 nesting question structurally (one scheduler sees the whole grid)
  instead of empirically per campaign.
- **A4 — decoder-adjacent physics in navette (moves v3 work item 5 inward).**
  Per-material default thickness bounds from material metadata
  (`bounds[m]`, overridable per run); pinned layer indices excluded from the
  LM free set *and* the needle scan mask; adjacency-grammar hook as a
  validator trait (`GrammarCheck: fn legal(a: MatId, b: MatId) -> bool`)
  consulted at construct/insert/migrate time. The external `ga::decoder`
  (G2) keeps genome/key logic; navette owns legality — process knowledge
  still lives in one place per side of the boundary.
- **A5 — licensing decision (owner call, before Phase 2).** Options: keep
  LGPL-3.0 (research-safe, product needs dylib/process boundary per §6.1);
  add an explicit static-linking exception for Rust rlib use; or dual-license
  the pure engine crate MIT/Apache while keeping bindings LGPL. Any option
  dissolves §6.1 as a project risk; record the choice in S10.

**2.8 Recipe mapping. (A1–A5 specified in §2.9 above.)** Recipe A: G1+G2+G3(rung 2) + single island +
terminal navette needle + LM. Recipe B: full stack — seeding (S8 proposals via
enqueue), S5 rung 3, S3 heterogeneous islands/tribes, S2 via G4, S4 archive
throughout, S6 moors split, S7 screening past TMM, S10 registration.

## 3. Glue inventory (must-write, sized)

| # | Glue | Size | Depends on |
|---|---|---|---|
| G1 | Fitness bridge (genome → DesignStack → merit + breakdown, batched) | ~200 lines | navette 0.8 (VERIFY per-step vs doc flow) |
| G2 | `ga::decoder` (VL keys, bounds[m], grammar, pins, masks) | ~300 lines | — (first code, v3 work item 5) |
| G3 | Pool loader + similarity order + latent table import + illumination sweep | ~150 + offline Python | navette materials-as-data |
| G4 | Needle memetic operator (A1 stages directly; doc-level only until A1 lands) | ~150 lines | A1 (owner) |
| G5 | QD descriptors from navette outputs + archive glue | ~150 lines | symbios 0.2, navette compute keys |
| G6 | DQD wiring (A2 Jacobian → descriptor gradients → CMA-MAE loop) | ~200 lines | A2 (owner) |
| G7 | Study wiring (enqueue/pin/prune/fANOVA callbacks) | ~100 lines | blackopt/optimizer |
| G8 | kurobako problem + A11 suite baselines | ~150 lines | G1 |
| G9 | Perf harness (nesting decision, bench vs README 2.2–2.6/19–66 ms) | ~100 lines | G1 |

Total glue ≈ 1,500 lines + offline embedding script. No glue duplicates a
crate: navette owns physics/merit/needle/LM; cab stands on E00 picks.

## 4. Fresh implementations (no crate does these)

- **F1 — MOME per-bin Pareto fronts in `symbios-genetics`** (month-scale,
  highest leverage, v3 work item 1): local fronts per archive cell; propose
  upstream, build as a fork first. Unblocks S4's core.
- **F2 — Learned-descriptor loop** (quarter): autoencoder/contrastive
  descriptors over spectra/fields (G12 lineage) replacing hand axes; BOP-Elites
  (G06) already models black-box descriptors — F2 supplies better ones.
- **F3 — Diffusion/LLM proposal operator** (quarter): stack-string prior
  (V38 shared representation) + named-move LLM operator with PCFBench
  ablations; logged, credit-assigned, rate-limited (v3 §14 guardrails).
- **F4 — POME offline pass** (weeks): ensemble-pessimistic proposals from run
  logs with zero new solves (G08); pure glue + egobox ensembles, listed here
  because no off-shelf crate composes it.
- **F5 (owner amendment, replaces upstream request):** A1 staged needle API +
  A2 introspection + A3 batch eval — specified in §2.9, no fallback needed
  since the owner implements them. G4(a) doc-level operator still ships first
  so Phase 1 never waits on amendments.

## 5. Build order with exit criteria

- **Phase 0 — pins + amendments (days):** `navette = "=0.8.0"`, toolchain
  1.88+, G9 micro-bench reproducing README numbers; land A1+A3 (smallest
  surface, unblocks G1/G4), spec A2/A4. Exit: scan/LM timings within 2× of
  published; memetic operator calls stages, not documents.
- **Phase 1 — Recipe A (1–2 weeks):** G2 + G1 + G3(rung 2) + genetic_algorithms
  single island + terminal needle/LM; reproduce a C00-class AR (Fig. 2.5
  lineage) with P(z)≈0 certificate. Exit: fabricated-plausible stack + ledger
  entry.
- **Phase 2 — Recipe B core (month):** heterogeneous islands/tribes, G4(a)
  needle-memetic, dynamic parsimony, moors split (S6), rung-3 pool. Exit:
  40-layer filter + merit-vs-depth map (S4-lite via symbios grid).
- **Phase 3 — archives + studies (month):** F1 MOME, G5/G6 (SAIL/DQD),
  G7 study layer, G8 registration. Exit: seven-hybrid comparison (v3 §15)
  rerunnable by a third party.
- **Phase 4 — frontier (quarter):** F2/F3/F4, full-wave evaluator entry via
  SAIL. Exit: V9/V21 ablations published, V12 re-cited.

## 6. Risks

1. **LGPL-3.0 → owner licensing decision (A5)**: risk only until the owner
   picks an option; G1 bridge stays behind a trait regardless, so no code
   churn either way.
2. **API churn: owner-controlled** — pin exact versions for reproducibility
   (S10 reruns), but amendment scheduling is internal, not upstream diplomacy.
   Keep all navette contact in G1/G4 so internal refactors touch two files.
3. **Rayon nesting** (G9 decides empirically; README's own bench notes
   dispatch-bound kernels — re-measure, don't assume).
4. **ndarray drift** across navette/moors/basin/egobox: Vec<f64> boundaries
   (§1 rule); convert at edges.
5. **A1/A2 slippage**: G4(a) doc-level operator and GP-surrogate SAIL ship
   regardless — amendments accelerate, never gate. (Replaces old G4(b)/G6
   VERIFY-failure risk.)

*Sources: navette crates.io meta 2026-11-09 (`meta2_navette.json`),
`readme_opticsWolf_Navette.md` (30 KB upstream README), `rust/navette/
Cargo.toml` (name/version/license/features incl. opt-basin/opt-argmin/
opt-minpack-lm), PAPER v3 (§§2–16 → S1–S10/Recipes), E00 (crate picks),
E05–E08 (strategies/ledger). Toolchain note: same opticsWolf author ships
ikat (cf. ikat_issues reports) — fast, responsive upstream; file F5/G4(b)
requests rather than forking silently.*
