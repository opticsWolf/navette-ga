# Phase 2 / Phase 3 implementation ledger

Date: 2026-11-15. Scope: the remaining Phase 2 items in E09 v2 §5 plus the
Phase 3 modules that can be built on the same bridge. This is an engineering
ledger, not a campaign report: every number below comes from a test in the
repository, and every limitation is listed rather than inferred away.

## Environment

- `navette` local `dev_ga` at `c9e964d` (A1/A3, A2 introspection, public
  joint scan, and the core `ndarray` 0.16 lift).
- `navette-ga` single crate, LGPL-3.0-or-later.
- `cargo test --all-features`: **75 passed**.
- `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`: clean.
- `navette` with `ga-introspect`: **801 passed, 1 ignored**.

## Review follow-up (2026-11-15)

The first review of the Phase 2/3 surface found four integration gaps that the
unit tests did not catch:

- `Decoder::validate_stack` existed but was not called by the GA, H0, terminal
  finish, or joint step. It is now called on every assembled design, after
  every refinement/operator stage, and **after** the terminal write-back.
  `finish_recipe_a` certifies the returned stack, not the pre-write-back one.
  A terminal stack outside the decoder bounds is refused; this invalidated the
  earlier C00 ledger, which used a 5 nm LM clamp below the 10 nm material
  bound, and the ledger has been regenerated.
- `joint_needle_step` now stages the whole move on a cloned `SmatrixContext`
  and design, polishes the trial, evaluates it, and commits only on a strict
  merit improvement. The interior `split_design` + `insert_seed` path is
  therefore transactional too. `joint_needle_step_checked` /
  `run_joint_needle_steps_checked` additionally enforce the decoder A4
  contract on the trial.
- The illuminated run now uses the chromosome fitness the GA already computed
  instead of re-running the solve for every archived member.
- `bench::ComparisonArm` was re-aligned with E09 §2.7 / PAPER v4 §15:
  H1 GA-only, H2 phased GA→needle→LM, H3 needle-memetic, H4 heterogeneous
  islands/tribes, H5 QD/MOME, H6 surrogate screening, H7 seeded proposal.
  The registry test now pins the strategy-item mapping.
- Supplied `Population`s were silently using `FitnessFnWrapper::default()`
  (the constant zero function): `Ga::with_population` does not rebind each
  chromosome, and `Population::fitness_calculation` only evaluates `NaN`
  fitness. The island and illuminated runs therefore carried fitness 0 while
  the actual stack merit was finite. Both GA builders now rebind the real
  fitness function on every supplied chromosome; a regression test checks a
  supplied population's best fitness against a fresh evaluation. This changed
  the island/illuminated trajectories and is a correctness fix, not a tuning
  choice.

The 40-layer steep-edge filter, the multi-seed equal-compute H0/H1/H3
campaign, and H6/H7 implementations remain open.

## Phase 2 — joint boundary needles

The two-environment fixture has a bare coat in `front` and the same coat
behind a fixed 300 nm `H` cover in `back`, so a shared locus occupies different
rows in the two assemblies. The pool has H, L, and a barrier material B. B is
accepted between H and L, filtered from both free edges, and admitted at both
edges only when the multi-material edge lists contain it.

- `joint_gap_accepts` and `joint_gap_sites` are read-only; a query leaves every
  film thickness unchanged.
- `joint_gap_insert` commits a chosen boundary needle only after every
  environment's template accepts it; a refused locus leaves the design and
  `CompiledEnvironments::gaps()` unchanged.
- `joint_needle_step` clones the context and design, applies the candidate,
  polishes, and commits only if the real merit improves; the checked variant
  also applies the decoder's A4 bounds/grammar to the trial.
- The public spectral scan is `navette::smatrix::synthesis::cycle::
  joint_needle_scan`, wrapped as `ga::bridge::joint_needle_scan`.
  `joint_needle_step` applies one spectral candidate (interior inserts split
  the design and every template; boundary inserts use `joint_gap_insert`) and
  runs the bounded polish. `run_joint_needle_steps` repeats it.
- A two-step spectral joint run on the H/L/B, two-environment fixture inserts
  at least one candidate and keeps the expanded assemblies aligned.
- One joint macro cycle (`run_needle_cycles`, one needle) and one staged
  spectral step agree after the same initial refinement: same film count,
  same base material sequence, thickness difference below `1e-6 nm` (the
  macro labels the inserted film `B#1`; the staged path keeps `B`, which is
  the same material).
- Cleanup with `max_removals = 0` preserves the first and last film materials.

## Phase 2 — A2 introspection

`navette` now has an opt-in `ga-introspect` feature:

- `MeritSpec::try_merit_parts` returns each target's squared-residual
  contribution in registration order.
- `SmatrixContext::merit_parts` reads the parts from the current stack.
- `SmatrixContext::thickness_residual_jacobian` returns the analytic
  row-major residual Jacobian, its shape, and the film row each free-thickness
  column starts at. It returns `None` for specs the analytic path declines
  (colour, joint deposits) rather than silently differencing.
- The bridge exposes `FitnessEvaluator::merit_parts` and
  `FitnessEvaluator::thickness_residual_jacobian`; the GA and terminal paths
  validate every assembled stack against the decoder's A4 contract. The test verifies the parts
  sum to the scalar merit and the Jacobian has one column per free film.
  The returned Jacobian is the **residual** matrix plus its shape and film
  rows; `merit_thickness_gradient` composes `2 Jᵀr`. With the core on
  `ndarray` 0.16, `JacobianReport::as_array2` is a zero-copy view and
  `into_array2` is a no-extra-copy owned matrix. A nalgebra `DMatrix` adapter
  remains optional and unnecessary for DQD.

Honesty boundary: the Jacobian prices thickness moves, not insertion
positions. It is not a P(z) adjoint.

## Phase 2 — A4 consumer-side legality

`Decoder::validate_stack` resolves every film to a pool material, checks the
material's thickness bounds, and checks every adjacent pair against the
decoder grammar. It is now called by the GA fitness closure, H0, the island
best path, the illuminated run, the terminal finish (before and after every
stage, and after write-back), and by the `*_checked` joint-step variants.
This is the consumer half of A4; navette's native per-material bounds and
insert/migrate grammar hook remain open because they require changing
navette's material schema and LM parameter bounds. Enforcing the consumer half
invalidated the earlier C00 ledger, which had used a 5 nm LM clamp below the
10 nm decoder floor; see `phase1_c00_ar.md`.

## Phase 2 — G4/S2 mid-loop memetic schedule

`engines::run_recipe_a_memetic` runs the ordinary fixed-width Recipe A one
generation at a time. Every `MemeticSchedule::interval` generations it ranks
the population, decodes and assembles the best `top` chromosomes, applies the
same `MemeticOperator` the terminal finish uses, and writes an improving stack
back into the chromosome at `RecipeAConfig::max_layers`. A write-back is
accepted only when:

- the operator's real post-polish merit is finite and below the starting
  merit;
- the trial stack and the canonical re-decoded stack satisfy the decoder A4
  contract;
- the stack fits the fixed width;
- the canonical stack re-evaluates strictly better than the chromosome's
  current fitness.

`run_recipe_a_islands_memetic` applies the schedule to every island after its
generation and before migration. `MemeticRunReport` records passes, refined
chromosomes, accepted write-backs, insertions, scans, rejection reasons, GA
time, memetic time, and `memetic_share()`.

### Budget box (C00, seed 20261114)

The example accepts `--memetic`, `--memetic-interval`, `--memetic-top`, and
`--memetic-insertions`. All rows use the same 64x100, 32-layer C00 config; each
is one deterministic run, not a repeat distribution.

| schedule (k / top / max) | merit | avg R | layers | iterations / accepted | memetic share |
|---|---:|---:|---:|---:|---:|
| H1, no schedule | 24474.958775 | 3.3589 % | 8 | — | 0 % |
| 10 / 1 / 1 | 24474.614047 | 3.3596 % | 9 | 10 / 2 | 4.67 % |
| **20 / 2 / 1 (default)** | **4559.194362** | **1.1705 %** | **12** | **10 / 3** | **5.56–6.27 %** |
| 10 / 2 / 1 | 600.795854 | 0.4669 % | 23 | 20 / 6 | 20.64 % |
| 10 / 8 / 2 | 1440.475960 | 0.7941 % | 14 | 80 / 13 | 63.36 % |

The 20/2/1 default is the useful single-digit-share point: it moves the
average reflectance from 3.36 % to 1.17 % for roughly 6 % wall time on this
run. The aggressive 10/8/2 schedule is not better on this seed and costs 63 %;
"more operator" is not automatically more design. The 10/2/1 row finds the
best recorded reflectance here but spends ~21 % of wall time, so it is a
quality-first setting, not the default.

Caveats: one problem, one config, one seed; wall time is machine-dependent;
the comparison is not an equal-evaluation-budget H0/H1/H3 campaign; and the
terminal scan certificate remains zero actual improvements in every row. The
budget conclusion is therefore a tuning result for this fixture, not a general
Phase 2 claim.

## Phase 2 — depth bake on the C00 AR fixture

`examples/phase2_depth_bake.rs` runs the same H/L, 400–900 nm antireflection
target at fixed width caps 4, 8, 12, 16, 24, 32, and 40 (seed
`20261116 + depth`, population 32, 40 generations). Every returned stack is
screened by `Decoder::validate_stack`, a 10 nm thickness floor, and the
declared layer cap; the best points are inserted into `MomeArchive`.

```text
depth   merit          avg R      decoded layers   d_min nm   screen
4       16669.237404   2.8109 %   1                99.904     pass
8       16690.799543   2.8117 %   3                19.565     pass
12      17201.764607   2.8455 %   1                111.976    pass
16      24272.193840   3.3497 %   3                131.052    pass
24      23577.381474   3.3052 %   5                26.160     pass
32      249481.228730  8.3255 %   7                22.881     pass
40      140932.513746  7.0548 %   16               48.940     pass
```

The result is deliberately reported as a **negative scale signal**, not a
40-layer success: the fixed-width/presence-coded H1 path does not grow a
40-layer design on this target, and the higher caps do not improve the merit.
That is expected for a five-to-ten-layer antireflection problem, where extra
layers are not useful, but it means this bake cannot stand in for Recipe B's
steep-edge filter. All screens pass because the designs that survive are small;
it does not prove a 40-layer manufacturable filter.

The archive contains six occupied cells and the depth/thickness front is
printed by the example. The next filter step needs a target that rewards depth
and the insertion-heavy Recipe B path, not a larger cap on the AR fixture.

## Phase 2 — H0 control

`engines::run_h0_control` runs independent random starts, a bounded LM
refinement, an optional memetic needle step, and a second refinement. It
returns the same `RecipeAResult` shape as the GA so the comparison harness can
report it without a second result type. A start whose LM path leaves the
decoder's A4 bounds is discarded rather than aborting the arm. The budgeted
runner and the first equal-solver-work comparison are below.

## Phase 2 — equal-budget H0 vs H1

`FitnessEvaluator` now carries a shared solver-work counter: one `evaluate`
call, one per stack in `evaluate_batch`, and the LM solver's own `evals` from
`refine_report`. Ordinary evaluator clones share the counter; `fresh_counter()`
is the comparison-arm fork. `compare_h0_and_ga_at_budget` gives H0 and H1
separate counters and stops each at the requested budget.

C00-class run, five seeds (`20261114..20261118`), same seed per pair, same
32-layer search space; H0 is random-start needle-plus-LM multi-start, H1 is
the ordinary Recipe A GA. The C00 example reproduces this with
`--h0-budget 2000 --h0-repeats 5`.

| budget | arm | median merit | median avg R | actual evaluations | wins |
|---:|---|---:|---:|---:|---:|
| 2,000 | H0 | 992891.305252 | 18.9000 % | 2,045–2,309 | 0/5 |
| 2,000 | H1 | 24685.958395 | 3.3849 % | 2,049 | **5/5** |
| 20,000 | H0 | 126253.031531 | 6.7780 % | 20,005–20,285 | 0/5 |
| 20,000 | H1 | 24196.228802 | 3.3391 % | 20,033 | **5/5** |

The two-sided Mann–Whitney normal approximation with tie correction gives
`p = 0.0082` at 2k and `p = 0.0053` at 20k. These are not exact p-values
because the H0 samples contain ties (all five 20k runs reach the same local
optimum); no multiplicity correction is applied because there is one pair.
`study::holm_bonferroni` is available for the future H0–H7 family.

On this fixture the classical control loses at both budgets; the H0 arm
improves with more LM work but does not catch the GA. H0's random 32-layer
starts are a deliberately hard version of the control, and all five H0 seeds
converge to the same best local optimum at 20k, so this is evidence against
the strong "H0 wins at 5–10 layers" risk, not a universal claim. The H1
plateau also shows that more evaluations alone do not improve this fixed-width
search after it converges.

The repeat runner is fixed-seed; the rank-sum primitive is normal-approximate
when ties are present, and ERT reporting and the full H0–H7 multiplicity plan
are not claimed yet.

## Phase 2 — S4/F1 archive and S6 split

- `qd::MomeArchive` is a two-dimensional MAP-Elites grid over layer count and
  total thickness. Each cell keeps a nondominated front over
  `(merit, total thickness)`; `insert_at` is the DQD/gradient door and
  `GradientDescriptor::from_jacobian` turns an A2 matrix into two normalized
  axes.
- `engines::run_recipe_a_illuminated` records every legal generation member
  while the ordinary Recipe A selection runs, using the fitness the GA already
  computed rather than re-solving each member. This is the S4-lite map, not a
  QD-driven search.
- `moors_split::split` maps `(merit, total thickness, layer count)` into
  `moors`' fast non-dominated sort and returns the fronts.

## Phase 3 — H7 seeded/proposal fallback

`engines::run_recipe_a_vl_seeded` fills the first `seed_fraction` of a
variable-length Recipe A population from a supplied `Genome` bank and the rest
randomly. Seeds are validated against the decoder and the VL bounds before the
run. This is the mined-motif fallback for S8/H7, not a diffusion model: the
caller must record where the seeds came from.

The C00 example exposes `--seeded`. It uses a three-layer H/L/H motif at
25 % population share and the same 100-generation config as the other arms. The
recorded run returned `merit = 16669.222449`, `avg_R = 2.8109 %`, two decoded
layers, and a zero-improvement terminal scan certificate. This is a seed-bank
demonstration on one target, not an H7-vs-H1 equal-budget result and not
evidence that a diffusion proposal is implemented.

## Phase 3 — I1

`engines::run_recipe_a_i1` keeps the ring and per-generation advance, but at
each migration event builds one child from `parents` distinct source islands
and replaces a destination's worst only when the child is strictly better.
`IslandRecipeResult` now reports accepted `crossovers` and a material-mix
`homogenisation` distance (mean pairwise L1 between island histograms, in
`[0, 1]`). The default `run_recipe_a_islands` path reports zero crossovers and
the same divergence.

## Phase 3 — G7 study layer

`study::Campaign` owns the queue, pins, running/completed/pruned ledger,
percentile pruner, requeue path, and a deterministic one-way fANOVA-style
importance readout. Pins override queued values; non-finite merits are
refused; pruning keeps the better fraction and never deletes completed data.

## Phase 3 — G8 registration

`bench` (feature) defines `ComparisonArm::H0…H7` in the fixed order and a
kurobako `Problem` over a fixed Recipe A space: two parameters per layer
(material index and normalized thickness), one scalar merit, one step. The
problem uses the same decoder/template/evaluator as the GA, so a solver is
measured on the same physics.

Limitations: the kurobako registration is a problem definition, not a full
H0–H7 dispatch (H6 screening and the H7 seeded proposal arm are not wired);
the archive illumination is not selection; the C00 depth bake is a negative
scale signal, not a 40-layer steep-edge filter; the equal-budget H0/H1 result
is five seeds on one target; the H7 arm is a mined-seed fallback, not a
diffusion implementation; H6 is still unwired; navette-native A4 and the
released-navette gate remain open. The core `ndarray` lift (`c9e964d`) resolves the earlier
navette/moors array-major split; it is a dependency-major change for the next
navette release.
