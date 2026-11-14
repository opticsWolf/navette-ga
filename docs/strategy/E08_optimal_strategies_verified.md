# E08 — Optimal strategies for GA thin-film synthesis: consolidated, expanded, and literature-checked

*Consolidates the strategy spine of `PAPER_ga_thinfilm_strategies_v4.md` (S1–S10,
Recipes A/B, Appendices A/B), `E05_new_strategies_2026.md` (QD/surrogate/generative),
`E06_constrained_diverse_design.md` (constraints/diversity architecture), and
`E00_rust_crates_consolidated.md` (55 crates). The V1–V15 ledger was verified by
chunk-level graph searches on 2026-11-09 against PAPER v3 and remains valid; PAPER v4
adds V16–V41 and the round-1/2 review repairs. Verdicts: **CONFIRMED** (primary text
found), **QUALIFIED** (mechanism confirmed, integration is ours), **HEURISTIC** (practice
wisdom, no primary located), **GAP** (nothing in-graph — build or fetch it). Survey-scoped
crate claims ("only X does Y") mean "among surveyed crates", not the world.*

## 0. Current status (2026-11-14)

- **Current documents:** `PAPER_ga_thinfilm_strategies_v4.md` (strategy),
  `E09_rust_implementation_strategy_v2.md` (build), `E10_cherrypick_reassembly_v2.md`
  (licensing/copy gate). The pre-v2 E09/E10 drafts were stale and have been removed.
- **Architecture:** one `navette-ga` crate, LGPL-3.0-or-later, with `ga::*` modules;
  navette 0.10.0 owns optics, merit, needle, and LM. A5 is resolved.
- **Material-boundary Phase 1:** `StackTemplate` now carries navette-style
  `BoundaryRules` (`InsertRules` host→needle map plus outer/inner edge lists);
  assembled stacks validate their edges and the memetic pool honors the insert
  matrix; single-material edge lists can become decoder pins, and insert pairs
  are checked for bidirectional grammar legality; the flat boundary needle is
  covered, including edge-list rejection. Pins are used for single-material
  edges and seeding is rejected. Phase 2 now has read-only joint gap
  acceptance (`gap_accepts_needle` / `joint_gap_accepts`), structural
  candidate enumeration (`joint_gap_sites`), and transactional chosen-locus
  insertion (`joint_gap_insert`), including through a multi-material edge
  list, the B barrier/edge-layer case is covered, and one-cycle flat staged
  equivalence with `run_needle_cycles` is tested; a spectral joint scan/polish
  run, joint `run_needle_cycles` equivalence, and full multi-material joint runs
  remain (E09 v2 §2.4.1). It is a data contract, not a second adjacency grammar.
- **Phase 1 exit artifact:** the G3 material loader consumes navette's ordered
  material evaluation; the terminal LM/needle finish writes back into the
  genome and the actual-trial scan certificate is recorded in
  `docs/studies/phase1_c00_ar.md` (C00-class H/L AR, 400–900 nm).
- **Review status:** 21 findings / 11 gaps / 11 risks (round 1: 19/10/10; round 2 adds
  E15/E16/G11/R11). S1–S10 remain the strategy spine; certification is the C2
  scan certificate (P(z)≈0 is A6-only), and S3's migration policy is fixed in
  E09 v2 §2.8.1 (ring, k=10, one best migrant capped at 5%,
  best-replace-worst-if-better with legality repair, within-island mating).
  Cross-island multi-parent recombination is experiment I1, not default.
- **Weakest links:** V9 (schedule lore), V10 (cost phrasing), V11 (survey-scoped),
  V12 (page-level re-citation owed). No strategy claim is retracted.

## 1. Verification ledger

| # | Claim (source) | Verdict | Evidence |
|---|---|---|---|
| V1 | Repair with box constraints is standard CMA-ES practice (PAPER §7 ← A06) | CONFIRMED | A06 chunk 866: "Repair available as for example with box-constraints"; chunk 869: repaired solution = smallest-distance feasible point |
| V2 | VAE-tSNE dispersion embedding + A3C/GA split on materials/thicknesses, solar absorber, zero-human demo (PAPER §6 ← B09) | CONFIRMED | B09 chunk 12: "VAE-tSNE feature reduction method to map the high dimensional complex refractive index … onto a two-dimensional semantic space"; chunk 84: "RL based on A3C and GA to optimize … multi-layer solar absorber" |
| V3 | MAP-Elites illuminates (merit, descriptor) maps; BOP-Elites models fitness+descriptors jointly, QD-vs-MOO decision table (E05 §§1–2 ← G01/G06) | CONFIRMED | G01 chunk 0/318: "Illuminating search spaces by mapping elites"; G06 chunks 6/240/312: archive of elites, unit-hypercube inputs, BOP-Elites vs MAP-Elites/SOBOL convergence figure |
| V4 | Island topology matters; noisy-island runtime analysis exists (PAPER v4 §9 ← D01/D13) | CONFIRMED | D01 chunk 0: "On the Impact of the Migration Topology on the Island Model"; D13 chunk 4: "Island model, noisy optimization, runtime analysis"; round-2 policy in E09 v2 §2.8.1 fixes ring/k=10/one best migrant/replace-if-better |
| V5 | BRKGA decoder pattern, unit-hypercube population (PAPER §5 ← A09) | CONFIRMED | A09 review ingested (title chunk 0); decoder description is PAPER's own synthesis of it — mechanism confirmed, coating mapping ours |
| V6 | Qubit-amplitude encoding with rotation gates (PAPER §13 ← B04) | CONFIRMED | B04 chunk 15: "QGA employs the probability amplitudes of qubits to encode chromosomes … and utilizes quantum rotation [gates]" |
| V7 | VMVOP class: layer count is a dimensional variable (PAPER §2 ← A10) | CONFIRMED | A10 ingested (chunk 324: neighbourhood/mutation-operator definitions for variable-size); the "collapses the VMVOP" argument is ours, consistent with the taxonomy |
| V8 | Inverse design of stacks from spectra is the D04 problem frame (PAPER §14) | CONFIRMED | D04 chunk 5: "Inverse design of optical multilayer stacks seeks to infer layer materials, thicknesses, and ordering from a desired target spectrum" |
| V9 | Insertion-probability decay schedule "mirroring SA cooling" (PAPER §5) | HEURISTIC | No primary located. Sensible (matches dynamic-penalty logic of F08 class), but label it tuning lore until an ablation exists — register under kurobako/mahf-coco when tested |
| V10 | QIGA diversity "without extra evaluations" (PAPER §13) | QUALIFIED | Encoding confirmed (V6); the cost claim is interpretive — rotation-gate updates are cheap but not free, and D16's evaluation budget needs its own read |
| V11 | Only `genetic_algorithms` offers VL + length penalty (E00/E04) | QUALIFIED | True among the 55 surveyed crates (E04 gap matrix); `radiate`/`oxigen`/`genoxide` have no VL path — but this is a survey result, re-check before committing |
| V12 | OptoLlama 2.9×/3.45× gains, DBR motifs recovered (PAPER §6/§14 ← D04) | CONFIRMED-in-PAPER | Numbers come from D04's full text as ingested; chunk search surfaced the problem frame (V8) not the numbers — trust but re-cite page numbers when writing up |
| V13 | F08 static/dynamic/adaptive/death penalty taxonomy (PAPER §7) | CONFIRMED | F08 chunk 131: "Penalty Functions for Handling Constraints" heading located; taxonomy per the ingested book text |
| V14 | G09 MOME per-bin fronts + CVT on learned manifolds beats NSGA-II/MAP-Elites (E05 §3) | CONFIRMED-in-E05 | From G09 full-text ingest (E05); no Rust crate implements MOME — build item, `symbios-genetics` host |
| V15 | POME offline QD with zero new sims; SAIL acquisition-map loop (E05 §§1/4) | CONFIRMED-in-E05 | From G07/G08 ingests; glue code, no off-shelf crate |

Refuted: nothing structural. Weakest links flagged honestly: V9 (schedule lore), V10
(cost phrasing), V12 (page-level re-citation owed), V11 (survey-scoped).

## 2. What the literature forces (non-negotiable core)

1. **Encode for growth (A10 + C00 §2.4 + G04).** The dimensional variable must be
   searchable: VL chromosomes with insertion/deletion, Pad crossover, absence keys
   (PAPER §5, V7). Growth precedent is dual — needle splits (C00) and NEAT
   complexification (G04) — so stochastic + deterministic growers (V-mechanism §4)
   is redundancy, not duplication.
2. **Constraints live in the decoder (A06 + F08 + E03-moors).** Repair-first for
   bounds/equalities (V1: clamp/snap/overwrite, never penalise what you can fix);
   penalty taxonomy for soft prefs (V13); feasibility-first tournaments always.
   Pinned ends are decoder constants, not constraints (PAPER §7, E06 §5).
3. **Certify with the scan (C00; review C2).** The empirical scan certificate
   is the only local-optimality certificate in the pipeline: no scanned
   candidate insertion improves the joint merit, LM converged, d_min stable.
   P(z)≈0 is an A6 readout reserved for post-A6 campaigns, not an exit test;
   every recipe still ends with a needle pass + LM finish (PAPER v4 §§10–11).
4. **Illuminate, don't just optimise (G01/G06/G09).** The archive keyed by
   (depth, thickness, motif) *is* the parsimony answer (V3); per-bin fronts where
   objectives conflict (V14); surrogate the acquisition map when the evaluator is
   full-wave (V15: SAIL/BOP-Elites).
5. **Mutate materials in dispersion space (B09).** Index-space mutation is a
   category error; similarity-ordered pools at minimum, latent semantic space at
   best (V2). RL-on-materials/GA-on-thicknesses split is demonstrated, not
   speculative.
6. **Migration is a policy, not a topology label (E15/R11).** Ring default,
   one best migrant per island every k=10 generations (capped at 5% of P),
   overwrite the destination's worst only if the migrant is better, then repair
   legality. Migrants join the destination pool but mate within it by default;
   cross-island multi-parent crossover is I1, an experiment.

## 3. Optimal strategies (proposed, each tagged)

**S1 — Reference hybrid: phased GA → needle → LM (CONFIRMED core).** Random-key VL
encoding → heterogeneous islands → dynamic parsimony ramp → needle insertions at
argmax of the empirical scan gain (M=3–5, Δz₀≈10 nm visible) → d_min
projection → LM/NM finish → scan certificate (P(z)≈0 reserved for A6).
Default for 5–30 layers; never wrong, sometimes slow.
**S2 — Needle-as-mutation memetic loop (QUALIFIED).** S1's needle step inside the GA
loop as a Lamarckian operator via `LocalSearchOperator` + `basin` refiners. Mechanism
confirmed (C00), loop integration ours — the 30–80-layer growth-stall answer.
**S3 — Heterogeneous islands + tribes by constraint regime (CONFIRMED + QUALIFIED).**
Ring migration under E09 v2 §2.8.1 (k=10, one best migrant,
best-replace-worst-if-better with legality repair), one tribe each for
depth/merit/robustness, age protection (oxigen pattern), speciation against
Bragg-clone collapse (V4 + E06 §9). Migrants join the destination pool but mate
within it by default; cross-island multi-parent recombination is I1 and must
beat the single-island ablation on merit and homogenisation (R11). D11 CTCM is
the strategy-diversity witness.
**S4 — QD archive as the product (CONFIRMED lit, GAP in crates).** MAP-Elites over
(depth, thickness) → MOME per-bin fronts (build in `symbios-genetics`) → SAIL loop
for RCWA-grade cases → POME pass over logs (V3/V14/V15). Turns every "how many
layers" argument into map-reading.
**S5 — Semantic pool with rung discipline (CONFIRMED).** Rung 2 (similarity-ordered
pool + adjacent-swap mutation) is the minimum shippable; rung 3 (B09 latent space)
for ≥50-material pools; cap pool size, let the archive price each extra material (V2).
**S6 — MOO with N as an objective (CONFIRMED).** `moors` split-off (NSGA-III/R-NSGA-II),
robustness objectives (D17), scalarise last with weights read off the front (B05/F07).
**S7 — Surrogate screening, exact evaluation (CONFIRMED pattern).** Learned model
pre-screens, TMM decides (D05); `egobox` Gpx or `genoxide` BO; never let the surrogate
overrule physics on the shortlist (C01 spot-check discipline).
**S8 — Diffusion/LLM proposal layer (QUALIFIED, guardrailed).** Stack-string priors
(D04 token view, E05.6) seed islands; LLM named moves ("2H needle at antinode") as
credit-assigned operators with blind-operator controls and PCFBench ablations (E06 §11).
Never unsupervised weights/constraints.
**S9 — Curriculum + visual steering (HEURISTIC, cheap).** λ→band→angle→robustness
stages with warm starts; mid-run map inspection, enqueue bright cells, prune dead
regions (E06 §10). No literature against it; costs nothing.
**S10 — Register every comparison (methodology).** Seven-hybrid + H0 control +
ablations (V9's schedule lore included) under `kurobako`/`mahf-coco` so claims
graduate from HEURISTIC to CONFIRMED (E05 §7, A11 suites).

## 4. Two updated recipes

**Recipe A (AR 5–10 layers, a day's work):** random-key VL encoding, single island,
biased crossover, insertion/deletion on, light parsimony; rung-2 pool; decoder pins +
bounds; needle check (scan certificate: no improving insertion); LM finish;
fabricate. (S1+S5+§2.2.)
**Recipe B (steep filter 40–80+ layers):** diffusion/token seeding (S8) → semantic pool
(S5) → zero-parsimony insertion-heavy start → heterogeneous islands/tribes (S3,
policy in E09 v2 §2.8.1) → needle-as-mutation every k generations (S2) → dynamic
ramp + d_min cycles → MOME archive illuminated throughout (S4) → moors front with
N + robustness objectives (S6) → LM finish on knees → fabricate the knee, not the
extremum. Surrogate screen (S7) wherever the evaluator exceeds TMM.

## 5. Crate assignments (from E00 §8, unchanged, mapped to strategies)

`navette 0.10.0` (optics/merit/needle/LM; A1–A6 amendments) + `genetic_algorithms`
(S1/S2/S3: VL islands, memetic slot, batch eval; dependency Phases 0–2, gated copy
Phase 3) + `genoxide` (S7/second pillar, BO+MOEA/D to prove on a 40-layer filter) +
`moors` (S6) + `basin` (S1/S2 refiners) + `symbios-genetics` (S4; +MOME/learned-
descriptor/DQD features to build) + `blackopt`/`optimizer` (S9 steering:
enqueue/pin/prune/fANOVA) + `egobox-gp`/`egobox-ego` (S7 light) + `mahf-coco`/
`kurobako` (S10). Custom: decoder (bounds[m] table, adjacency grammar, pins),
descriptors, LLM operator, archive glue (E06 §12).

## 6. Open gaps → work list

1. MOME per-bin fronts in `symbios-genetics` (V14) — month-scale, highest leverage.
2. Ablate V9 (insertion schedule) vs constant vs N-as-objective; publish under S10.
3. Re-cite V12 with D04 page numbers; hand-fetch ALPS/TPE/BOHB/MOO primaries (E07 §A).
4. SAIL glue (`symbios`+`egobox`) the day the evaluator goes full-wave (V15).
5. Per-material `bounds[m]` + adjacency-matrix decoder (~30 lines, E06 §7–8)
   — first code in the new crate.
6. **Round-2 island policy (E15/R11):** ablate migration rate (1/2/5%), plain vs
   extended ring, and I1 cross-island multi-parent vs within-island mating; report
   merit/depth plus homogenisation. The Phase 2 wrapper is implemented
   (`run_recipe_a_islands`); the Phase 3 copy remains.

*Method note: V1–V15 verdicts from `okf search --target chunks` (4 + 4 + 4 query
batches), 2026-11-09; full chunk texts in the cited concepts. PAPER v4's V16–V41 and
the round-2 E15/E16/G11/R11 register are recorded in PAPER v4 and E09 v2. Re-run the
ledger when G-track or hand-fetched primaries land — V9/V10/V12 are the ones most
likely to move.*
