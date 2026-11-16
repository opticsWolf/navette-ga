# H0–H7 implementation learnings

**Status:** analysis of the code and campaigns at `36d4270` (84 tests),
updated with the all-arm seeded campaign after `5fe6810` (86 tests).
**Scope:** the eight comparison arms in `src/bench.rs`, the engines they call,
and the C00 / long-pass / short-pass campaigns in
`docs/studies/phase2_phase3_ledger.md`.
**Purpose:** separate what the implementation has actually demonstrated from
what the arm labels imply, and revise the architecture/plan assumptions
accordingly.

## 0. Evidence base

The findings below rest on four kinds of evidence:

1. **C00 AR campaign** — ten seeds, population 24, 30 generations, 16-layer
   cap, H3 schedule 10/2/1, fixed target 30,000.
2. **Long-pass filter campaign** — seven seeds, 12 individuals, 12
   generations, 40-layer GA cap, 60/80-film decoder, terminal insertions 10 or
   20.
3. **Short-pass filter campaign** — the same setup with reversed bands and
   chirp; includes cap and terminal-budget sweeps.
4. **Growth probes** — chirped seeds from 10 to 40 layers, with the A1
   memetic operator and a final LM.

The original scope limit was that only **H1/H2** consumed
`ComparisonOptions::initial_genomes` and **H7** used its seed bank. That gap
is closed: H0 now consumes one seeded start and H1–H6 seed their populations
(H4 every island). Section 0.1 records the seven-seed all-arm results.

## 0.1 All-arm seeded update (2026-11-16)

All fixed-width arms now accept the same two-layer seed; default behavior is
unchanged when `initial_genomes` is empty. Seven seeds, 40-layer GA cap,
20 terminal insertions, decoder width 80, one seeded pair:

| arm | long-pass best | layers | short-pass best | layers | deep |
|---|---:|---:|---:|---:|---|
| H0 | 1,065,970.763457 | 23 | 650,958.663020 | 19 | no |
| H1 | 5,628,078.588306 | 26 | 6,231,605.586023 | 18 | no |
| H2 | 115.468412 | 64 | 0.487692 | 62 | yes |
| H3 | 220,526.186485 | 29 | 189,000.676487 | 25 | no |
| H4 | 8,998,622.288615 | 13 | 5,203,999.394324 | 18 | no |
| H5 | 11,983,554.029035 | 25 | 9,597,228.143330 | 22 | no |
| H6 | 17,044,481.376999 | 25 | 9,299,826.248700 | 16 | no |
| H7 | 546.192961 | 44 | 8.990342 | 73 | yes |

This confirms the main finding rather than changing it: seeding helps H3
(29/25 layers, 220k/189k merit) but not enough without terminal growth;
H4/H5/H6 remain weak filter optimizers; H0's best is unchanged because its
other random starts dominate; only H2/H7 pass the deep screen. Holm-significant
pairs: 23/28 long-pass, 17/28 short-pass. The seed moves the fixed-width arms
closer together, so the seeded short-pass family separates less than the
random-start family.

## 1. Executive findings

### F1. “Start from two layers” and “search width two” are different experiments

The first two-layer run used `--max-layers 2`. That set the fixed-width
Recipe A genome to two slots, so H0–H6 had no spare capacity. H2 ended at
4/8 layers, which looked like a failure of fixed-width genomes. It was not:
`max_layers` is the chromosome width, not an initial-layer count.

The later seeded path pads a two-layer genome with `presence = 0` slots up to
a 40-slot width. H1 then reached 26/18 layers through presence activation, and
H2 reached 64/62 layers with the terminal finish. The architecture needs to
distinguish:

- **initial active layers** (a seeded design),
- **genome capacity** (the maximum encodable width),
- **refinement capacity** (needle/LM insertions beyond the genome).

Conflating them produced a misleading negative result.

### F2. Terminal local refinement is the dominant deep-filter mechanism

On the long-pass fixture, H2 (GA + terminal needle/LM) and H7 (VL seed +
terminal) are the only deep-screen passes. H1, H3, H4, H5, and H6 do not have
a terminal finish in the dispatcher and all remain far from a 40-layer deep
filter. H3's mid-loop memetic pass is not equivalent: it improves C00 but
does not carry the filter to depth.

The evidence supports a two-stage model:

1. search produces a promising architecture/thickness pattern;
2. terminal needle insertion plus LM supplies the last local depth/quality.

The second stage is not optional for the current deep-filter target.

### F3. Depth is budget-limited before it is target-limited

The short-pass fixture initially looked like a negative depth-transfer result:
H2 35 layers and H7 38 at the default 40-layer GA cap / 10 terminal
insertions. A cap sweep and terminal sweep changed the conclusion:

| short-pass setting | H2 layers | H2 pass / stop | deep |
|---|---:|---:|---|
| cap 40, terminal 10 | 35 | 0.0772 % / 99.8788 % | no |
| cap 42, terminal 10 | 39 | 0.0753 % / 99.9142 % | no |
| cap 44, terminal 10 | 40 | 0.0655 % / 99.9282 % | yes |
| cap 40, terminal 20 | 60 | 0.0068 % / 99.9979 % | yes |
| cap 40, two-layer seed, terminal 20 | 62 | 0.0082 % / 99.9964 % | yes |

H7 similarly reaches 55 layers at terminal 20 and 73 layers from a two-layer
seed. The 40-layer threshold is therefore a **budget-dependent reachability
boundary**, not a fixed property of short-pass physics. The required GA width
and terminal budget are target-specific; the layer count itself is not
intrinsically impossible.

### F4. Presence-coded fixed width can grow, but only inside its width

H1 started from two active genes in a 40-slot genome and reached 26 layers on
long-pass and 18 on short-pass. This proves that presence genes can activate
spare slots and that a fixed-width genome is not limited to its initial active
count. But H1 cannot exceed 40 without terminal insertions, and it does not
have them. H2 starts from the same seed and reaches 64/62 because the terminal
finish crosses the genome-width boundary.

The correct mental model is:

> fixed-width + presence = variable **effective** length up to the cap;
> true VL + terminal = variable **encoded** length plus local growth.

### F5. Arm rankings are fixture-dependent; there is no universal H0–H7 ranking

C00 and the two filter fixtures disagree:

- H3 has the best C00 single run (1,095 merit / 0.61 % average R).
- H2 has the best long-pass deep result (43 layers / 0.048 % pass).
- H2 and H7 are the only long-pass deep-screen passes.
- H7 has the best short-pass merit at the default budget; H2 wins target
  success and is better once seeded/deepened.
- H4 and H5 are consistently weak optimizers on the filter fixtures; their
  purpose is population/archive structure, not filter merit.

A campaign result is a statement about a fixture, target, width, terminal
budget, seed count, and evaluation budget. It is not a global ranking.

### F6. H0 is a control, not a contender at shallow random starts

H0’s independent random starts plus LM are a useful classical baseline, but
they do not compete with H1 at equal solver budget on C00, and H0 misses the
filter target. The control is deliberately hard, but the result is clear:
local refinement from random starts is not a substitute for population-based
architecture search at these depths. H0 now accepts one seeded start, but the
seven-seed best is unchanged because the remaining random starts still
dominate.

### F7. H6 is a compute-saving screen, not a better optimizer

The linear ridge surrogate used 247 true evaluations versus 452 for the
125-evaluation H1 control, but produced worse quality on the cheap-TMM C00
fixture. Its best use case is an evaluator where each true TMM call is
expensive; on cheap TMM it is a loss. It is also not a GP/Bayesian optimizer:
there is no uncertainty model or acquisition function. The only safety
property currently guaranteed is that the final reported design is
re-evaluated by navette.

### F8. H7’s seed transfer works; its Gaussian proposal does not

The H7 seed-bank fallback is real: a two-layer seed grows to 44 layers on
long-pass and 73 on short-pass with the terminal pass. A 40-layer chirped seed
reaches 47 layers on long-pass. That is useful architecture transfer.

The Gaussian proposal expansion was effectively neutral on C00 (16,671.9 vs
16,669.3 merit) and did not solve the deep-architecture problem. It is a
local thickness perturbation, not a trained diffusion/LLM prior. H7 is
currently **seeded VL + terminal**, not a generative model.

### F9. The statistical protocol needs at least seven seeds for a 28-pair family

With five seeds, the smallest possible two-sided rank-sum p-value under
complete separation is `0.00794`; after Holm over 28 pairs it becomes
`0.2222`. No pair can be significant at 0.05. Seven seeds lower the complete
separation value enough for a Holm-significant pair.

Current results:

- C00, ten seeds: 12 of 28 Holm-significant pairs.
- Long-pass, seven seeds: 24 of 28.
- Short-pass, seven seeds: 23 of 28; H2 and H7 are not separable at the
  default budget.
- Short-pass with the seeded/deepened H2: H2 and H7 separate (Holm 0.0197).

The plan should treat seed count as a design parameter, not an afterthought.

### F10. The implementation invariants are now part of the architecture

Several bugs were not numerical noise; they changed scientific conclusions:

- **Material order:** decoder pool and template were index-coupled. A
  hand-built H/L template against an L/H decoder silently swapped materials.
  `StackTemplate::check_grammar` now refuses the mismatch.
- **Transactional insertion:** accepting a needle on pre-polish merit could
  leave a worse post-polish stack. The operator now commits only after
  post-polish re-evaluation.
- **LM start floor:** navette accepted a sub-floor inserted film as `x0`
  under `ClampUpAlways`; `5adb40c` clamps the start to the policy floor.
- **Width/terminal budget:** terminal insertions can exceed the decoder width;
  the campaign now auto-sizes the decoder or fails explicitly.
- **Evaluator clamp vs decoder bounds:** a terminal LM can push a film outside
  the decoder’s material bounds if the two are not aligned; the test fixture
  now uses matching bounds.

These are A4/grammar/budget contracts. They belong in the architecture, not in
example-only patches.

## 2. Arm-by-arm assessment

| Arm | What it actually is | Strongest evidence | Main limitation / gap |
|---|---|---|---|
| **H0** | Multi-start → LM → optional needle → LM | Robust local convergence; useful classical control | No learning/population; one seeded start does not rescue it; poor at deep filter targets |
| **H1** | Fixed-width GA only | Cheapest arm; best pure-GA C00 run (10,087 / 2.00 %) | No LM/needle; fixed width; no terminal; plateaus at shallow depth |
| **H2** | Fixed-width GA → terminal needle/LM (S1) | Best deep long-pass result (43 layers, 0.048 % / 99.666 %); 64/62 with seeded two-layer start | Width/terminal sensitive; only improving insertions; high variance; not pure GA |
| **H3** | Fixed-width GA + mid-loop memetic top-k (S2) | Best C00 single run (1,095 / 0.61 %) | No terminal; fixed width; filter results weak (22 layers); schedule overhead |
| **H4** | Heterogeneous islands/tribes (S3) | Tests diversity/migration/constraints | Fixed width; no terminal; deep-filter results poor; often near H1 |
| **H5** | MOME archive illumination (S4-lite) | Archive coverage and diagnostics | Archive does not steer selection; not full QD; weak optimizer; no terminal |
| **H6** | Linear ridge surrogate screen (S7) | ~45 % true-eval saving; final re-eval guarantee | Linear, not GP; quality loss on cheap TMM; no terminal/VL |
| **H7** | VL seeded/mined GA + optional proposal + terminal (S8) | Only true VL arm; 47-layer long-pass; 73-layer short-pass from a two-layer seed | Seed-initialized; Gaussian proposal negative; no diffusion; seed-dependent success |

### H2 vs H7: what is actually being compared

H2 and H7 are **not** a clean “fixed vs variable length” experiment. They
differ in at least four dimensions:

1. H2’s GA is fixed-width; H7’s is variable-length.
2. H2 starts random (unless seeded); H7 starts from the mined seed bank.
3. Both can use terminal needle/LM, but H2’s terminal width is driven by the
   decoder and H7’s by the VL seed plus decoder.
4. H2’s result is GA-initialized; H7’s is seed-initialized.

Any claim that “VL is better” from H2 vs H7 is confounded. The defensible claim
is narrower: **with a seed bank and terminal refinement, H7 reaches deep
filters; H2 can also reach them when the GA width and terminal budget are
available.**

## 3. Architecture assessment

### What the proposed architecture got right

- The engine split is sound: decoder/bridge/engines/bench/qd/study separate
  representation, physics, search, and reporting.
- A1 staged needles, A2 thickness-only Jacobian introspection, A3 batch
  evaluation, and consumer-side A4 are the right boundaries.
- A pinned H0–H7 registry with tests prevents arm relabeling.
- The solver-evaluation counter and censored ERT are the right compute units.
- Transactional insertion, material-order validation, and final true
  re-evaluation are necessary correctness properties.
- The diagnostic honesty in code (H6 linear, H7 not diffusion) is essential.

### Where the architecture/plan is mis-specified

#### A. The H labels mix independent axes

H0–H7 currently combine:

- representation: fixed vs variable length,
- initialization: random vs seeded,
- refinement: none vs mid-loop vs terminal,
- population: single vs islands vs archive,
- evaluation: exact vs surrogate,
- proposal: none vs Gaussian.

Because the labels are bundles, the campaign cannot attribute an effect to one
axis. H2/H7 is the clearest confound. H3/H4/H5/H6 are also evaluated without
terminal refinement, so their filter failures do not isolate their population
or evaluation ideas.

**Recommendation:** keep H0–H7 as named presets for published comparability,
but expose the underlying dimensions as composable configuration. A minimal
matrix would be:

```text
representation: Fixed { width } | Variable { min, max }
initialization: Random | Genomes(bank)
refinement: None | MidLoop(schedule) | Terminal(insertions)
population: Single | Islands(strategy) | Archive
evaluation: Exact | Surrogate(model)
```

Then the paper arms become pinned points in that space, and cross-factor
experiments (VL + islands, VL + archive, fixed + terminal, surrogate +
terminal) become possible.

#### B. “Initial genomes” is a targeted arm input, not a config-wide concern

All fixed-width arms now consume `ComparisonOptions::initial_genomes`: H0
uses one seeded start, H1–H6 seed their populations, and H4 seeds every
island. The implementation is additive arm paths, not a widening of
`RecipeAConfig` or `H0Config`. The seven-seed all-arm table is in section 0.1;
it still holds seed, cap, terminal budget, and evaluation budget fixed.

#### C. Terminal refinement is under-specified as an experimental factor

The terminal finish is the main deep-filter mechanism, but the campaign
exposes it through H2 and H7 only. It needs:

- an explicit insertion budget,
- a decoder-width rule,
- a seed/initial-population rule,
- a certificate that zero actually improving candidates remain,
- a report of final layers, minimum thickness, and true evaluations.

The short-pass cap/terminal sweeps show that reporting only “40-layer cap”
without the terminal budget is not enough.

#### D. H5 is S4-lite, not a QD optimizer

The archive records legal members but does not steer selection. That is a
useful diagnostic, but H5 should not be read as evidence for or against
archive-guided search. A full S4 arm would need archive-driven variation or
selection.

#### E. H6 and H7 labels overstate the implemented models

- H6 is linear ridge regression, not a GP or Bayesian optimizer.
- H7’s proposal is Gaussian perturbation, not a trained diffusion model.

Both are honest in code comments and the ledger, but the arm names S7/S8 can
mislead a reader. The architecture should either rename them or carry a
“model class” field in the campaign output.

#### F. The deep screen and 40-layer target are not universal

The 40-layer threshold is a useful filter screen, but the short-pass result
shows that the reachable layer count depends on cap, terminal budget, and
seed. The architecture should make the screen parameters part of the result
record: minimum thickness policy, layer target, cap, terminal budget,
wavelength bands, and merit weights.

## 4. Assumptions and confidence

| Assumption | Confidence | Reason |
|---|---|---|
| Deep filters need local refinement after architecture search | High | Only H2/H7 with terminal reach deep-screen quality; H1/H3–H6 without terminal do not |
| Fixed-width presence can grow within its width | High | H1 26/18 layers from a two-layer seed in a 40-slot genome |
| Width/terminal budget dominates reachable depth | High | Short-pass 35 → 40 → 60 layers under cap/terminal changes |
| 40 layers is not a universal target requirement | High | Short-pass 35/38 layers already has <0.1 % pass / >99.8 % stop; deeper is better but not necessary for every screen |
| Seed transfer is useful but seed-dependent | Medium | H7 44/73 layers from two-layer seed; target success 5/7 and 3/7 |
| H6 helps only when true evaluation is expensive | Medium | 45 % true-eval saving but quality loss on cheap TMM |
| H3/H4/H5 rankings generalize beyond C00 | Low | Filter results are poor and the arms lack terminal refinement |
| H2 vs H7 isolates VL vs fixed | Low | At least four confounded dimensions |
| Five seeds suffice for 28-pair Holm claims | Rejected | Minimum adjusted p = 0.2222 |
| A4/navette-native ownership is optional | Rejected | Material-order, x0-floor, and clamp bugs changed results |

## 5. Plan revisions

1. **Preserve H0–H7 as presets, but add orthogonal experiment axes.** This is
   the main architecture lesson. Do not add more bundled arms before the
   existing bundles are decomposable.
2. **Initial-genome plumbing is complete for H0–H6.** The seven-seed all-arm
   table is recorded; H7 remains on its separate seed bank. The next step is a
   factorial design, not another seed path.
3. **Make terminal refinement explicit in every report.** Record insertion
   budget, decoder width, final layers, min thickness, true evaluations, and
   certificate status.
4. **Use target-specific depth screens.** Keep the 40-layer screen for the
   long-pass production target, but do not treat it as a universal quality
   threshold. Report optical quality and layer count separately.
5. **Standardize the statistical design.** Seven seeds minimum for the
   28-pair family; report best, median, ERT, and Holm-adjusted pairwise tests.
6. **Keep H6 scoped to expensive-evaluation regimes.** A GP/BO arm is a new
   model, not a tuning change to the linear screen.
7. **Do not call H7 a diffusion arm until a learned proposal exists.** Seed
   transfer and terminal growth are the demonstrated capabilities.
8. **Prioritize A4-native ownership and the navette release gate.** The
   consumer checks caught real bugs; native ownership should make them
   impossible rather than test-detectable.
9. **Add the missing cross-factor experiments before making universal
   claims:** VL + islands, VL + archive, fixed + terminal, and surrogate +
   terminal. The all-arm seeded start is now recorded; it is not a full
   factorial design.

## 6. Open questions

- Does H3’s mid-loop schedule become competitive on filters when combined
  with terminal refinement, or is its C00 advantage target-specific?
- Does H4’s island diversity help once the population has variable length and
  a terminal finish?
- Can H5’s archive steer selection without losing H2’s local convergence?
- Does a GP/BO H6 beat the linear screen on an expensive evaluator, or does
  the feature space remain too small?
- Can a learned H7 proposal beat seed transfer, or is the seed bank the real
  signal?
- What is the minimum seed count and fixture count needed before the H0–H7
  family claims are publishable?
- How should A4-native per-material bounds and pins interact with terminal
  insertion widths?

## 7. Bottom line

The implementation has validated the **modular engine** and the **local
refinement** thesis, but not the **bundled-arm ranking** thesis. The strongest
reliable result is:

> global search plus terminal needle/LM refinement reaches deep filters; the
> required depth is set by GA width and terminal budget, not by the arm label.

The strongest negative results are:

> fixed-width presence alone does not replace terminal growth; H6’s linear
> screen does not improve cheap-TMM quality; H7’s Gaussian proposal is not the
> missing architecture prior; and five-seed families cannot support 28-pair
> Holm claims.

The plan should therefore move from “more H arms” toward “orthogonal axes plus
pinned presets,” complete the missing seed paths, and report the budget box
with every depth claim.
