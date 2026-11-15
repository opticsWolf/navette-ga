# Phase 2 / Phase 3 implementation ledger

Date: 2026-11-15. Scope: the remaining Phase 2 items in E09 v2 §5 plus the
Phase 3 modules that can be built on the same bridge. This is an engineering
ledger, not a campaign report: every number below comes from a test in the
repository, and every limitation is listed rather than inferred away.

## Environment

- `navette` local `dev_ga` at `5adb40c` (A1/A3, A2 introspection, public
  joint scan, the core `ndarray` 0.16 lift, and the LM x0-floor fix).
- `navette-ga` single crate, LGPL-3.0-or-later.
- `cargo test --all-features`: **83 passed**.
- `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`: clean.
- `navette` with `ga-introspect`: **802 passed, 1 ignored**.

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

The H6 linear-surrogate arm and the H7 mined-seed fallback are now
implemented; the full H7 diffusion proposal, the multi-seed H0/H1/H3
campaign, and a Recipe B discovery run remain open.

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

## Phase 2 — forty-layer steep-edge filter

`examples/phase2_filter_40.rs` builds a chirped H/L quarter-wave stack and
optimizes it against a long-pass edge target: R ≈ 0 below 500 nm and R ≈ 1
above 600 nm. The stack is kept at 40 layers with
`ThinLayerPolicy::ClampUpAlways`; all films are optimized and the evaluator
uses the same 10 nm floor as the decoder.

The chirped seed is initially reflective everywhere (merit `38312021.609`).
A bounded LM run (10,000 iterations, 14,227 evaluations) produced:

```text
merit=17161.441837190
layers=40
min_thickness_nm=10.000000
passband_avg_R=0.007983
stopband_avg_R=0.990983
production_filter=pass
R(450)=0.007374
R(500)=0.011982
R(550)=0.674465
R(600)=0.984061
R(700)=0.999171
```

The LM hit its iteration cap (`lm_converged=false`), so this is a feasible
40-layer production filter, not a converged optimum. It is a seeded stack plus
thickness refinement, **not** a GA discovery of a 40-layer design. The next
Recipe B step is to run insertion-heavy H7/H3 from a seed bank and compare the
discovered filter against this artifact under the same production screen.

## Phase 2 — filter merit-versus-depth bake

`examples/phase2_filter_depth.rs` runs the same edge target at depths
10, 20, 30, 40, and 50 with the same chirped H/L construction,
`ClampUpAlways`, and a 5,000-iteration LM cap. The seed is reflective
everywhere (merit in the millions); only the layer count changes.

```text
depth  finish merit    passband avg R   stopband avg R   min nm
10     4,413,530.93    4.5534 %         75.4175 %        25.227
20       179,951.70    2.2608 %         95.1641 %        10.000
30         2,879.44    0.6750 %         99.5302 %        10.000
40         5,270.69    0.9470 %         99.5898 %        10.000
50           474.53    0.4456 %         99.9525 %        46.058
```

This is the positive scale signal the C00 antireflection bake could not
produce: depth buys stopband rejection on a target that actually needs it.
It is not monotonic at 40 layers (30 was better at the same 5k LM cap), and
the 50-layer point is best in this table. The 40-layer artifact in the
previous section uses a different chirp and a 10k cap, so the two 40-layer
numbers are not directly comparable. All five stacks pass the decoder's A4
check; none is a GA-discovered architecture.

## Phase 2 — needle-grown filter (Recipe B growth)

`examples/phase2_filter_growth.rs` starts from a 10-layer chirped H/L stack
(5 pairs), refines it, then applies the A1 memetic operator for up to 30
insertions before a final LM. It is the insertion-heavy counterpart to the
seeded 40-layer artifact.

```text
seed merit      13,431,992.654090
after LM         4,413,542.120515   10 layers
operator         4 insertions, 5 scans, merit 255,123.721271, 18 layers
final LM           254,930.958714   18 layers
passband avg R     1.6721 %
stopband avg R    94.0613 %
min thickness      10.000 nm
production          fail (< 40 layers)
R(450)=0.000002  R(500)=0.058757  R(550)=0.662993
R(600)=0.895544  R(700)=0.964255
```

The operator accepts only individually improving insertions. With the
transactional post-polish acceptance, growth is non-monotonic in seed depth:

| seed pairs | start layers | accepted (30 cap) | final layers | merit | pass avg R | stop avg R | production |
|---:|---:|---:|---:|---:|---:|---:|---|
| 5 | 10 | 3 | 16 | 233,321.391201 | 1.7657 % | 94.3269 % | fail |
| 10 | 20 | 6 | 32 | 6,537.839811 | 0.1815 % | 99.0804 % | fail |
| 12 | 24 | 5 | 34 | 2,786.433973 | 0.1558 % | 99.4401 % | fail |
| 14 | 28 | 5 | 38 | 1,156.308727 | 0.1261 % | 99.6113 % | fail |
| 15 | 30 | 4 | 38 | 1,156.306762 | 0.1262 % | 99.6113 % | fail |
| 16 | 32 | 5 | 42 | 445.046595 | 0.0732 % | 99.7654 % | pass |
| 17 | 34 | 2 | 38 | 296.912897 | 0.0529 % | 99.8548 % | fail |
| 18 | 36 | 4 | 44 | 59.540343 | 0.0303 % | 99.9161 % | pass |
| 19 | 38 | 2 | 42 | 20.521238 | 0.0168 % | 99.9522 % | pass |
| 20 | 40 | 8 | 56 | 34.181808 | 0.0194 % | 99.9632 % | pass |

The earlier 30-layer “local optimum” result is superseded: with transactional
acceptance a 30-layer start now accepts four insertions and reaches 38 layers.
Growth crosses the 40-layer production count from a 32-layer start (42 layers)
and from a 36-layer start (44 layers); the 34-layer start stalls at 38, so the
crossing is not monotonic.

The default growth run is now `--pairs 18 --insertions 2`:

```text
seed layers        36
insertions          2
final layers       40
merit               60.066004
passband avg R      0.0259 %
stopband avg R     99.9156 %
min thickness      10.000 nm
production          pass
R(450)=0.000287  R(500)=0.000522  R(550)=0.177778
R(600)=0.998308  R(700)=0.998731
```

This is a 40-layer production filter reached by needle growth from a 36-layer
chirped seed. It is still seed-initialized, not a GA-discovered architecture,
but it closes the “40-layer reached by growth” gap; the seeded 40-layer + LM
artifact remains the record for the exact seeded construction. `lm_converged`
is false at the 3,000-iteration cap, so this is a feasible artifact rather than
a converged optimum.

The run also exposed a navette start-point bug: an inserted film below the
10 nm floor was passed to LM as x0, and `ClampUpAlways` rejected it before the
first iteration. Navette `dev_ga` now clamps x0 to `[lb_floor, clamp_max]`
(`5adb40c`); the regression test starts at 5 nm under a 10 nm floor.

## Phase 3 — filter cross-fixture campaign

`examples/phase2_filter_campaign.rs` runs all eight H0–H7 arms on the steep-edge
filter fixture with five seeds (`20261121..20261125`), 12 individuals, 12
generations, a 40-layer H1 cap, a 60-layer decoder cap, and a 30,000 target.
H7 enables the terminal LM/needle finish (`finish_h7 = true`); H2 uses the
same terminal finish but no mined seed bank.

| arm | best merit | median merit | median evals | best layers | best pass R | best stop R | min nm | deep screen |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| H0 | 2,508,738.644838 | 2,537,335.623404 | 5,016 | 19 | 7.4384 % | 82.3686 % | 11.28 | no |
| H1 | 11,507,570.963514 | 15,700,600.461266 | 157 | 21 | 31.0453 % | 68.4584 % | 13.65 | no |
| H2 | **969.321869** | 10,872.832468 | 2,503 | **43** | **0.0479 %** | **99.6655 %** | 10.00 | **yes** |
| H3 | 450,359.454717 | 613,838.646928 | 181 | 22 | 2.3688 % | 92.3681 % | 11.33 | no |
| H4 | 11,813,287.226372 | 11,990,956.171653 | 313 | 22 | 35.2440 % | 67.0423 % | 39.31 | no |
| H5 | 13,226,060.715772 | 15,599,501.719540 | 157 | 24 | 57.0656 % | 76.3681 % | 10.07 | no |
| H6 | 16,273,132.297823 | 17,419,979.811375 | 52 | 21 | 42.3815 % | 62.9791 % | 33.93 | no |
| H7 | **203.083830** | **1,287.867335** | 3,042 | **47** | **0.0533 %** | **99.8444 %** | 10.00 | **yes** |

“Deep screen” means at least 40 layers, a 10 nm minimum thickness, passband
average `R < 2 %`, and stopband average `R > 99 %`.

At the fixed 30,000 target, H2 succeeds on 4/5 seeds with ERT `4,045.8`; H7
succeeds on 5/5 with ERT `3,302.0`. H0, H1, H3, H4, H5, and H6 have no
success, so their ERT is `none`.

H2 is the first **GA-initialized** design to cross 40 layers and pass the deep
screen: it starts from `run_recipe_a` without a mined seed bank, then the
terminal finish grows it to 43 layers at 0.0479 % pass / 99.6655 % stop. H7,
which starts from the 40-layer chirped seed, reaches 47 layers at 0.0533 % /
99.8444 %. Both are seeded or GA-initialized rather than a pure fixed-width
GA discovery, but the H2 path no longer needs the mined seed bank. H3 alone
reaches 22 layers and 2.37 % pass; H1/H4/H5/H6 remain far from the target.

The five-seed family has 28 pairs. No pair survives Holm at 0.05: the smallest
raw two-sided rank-sum value is `0.00794` (complete separation), which becomes
`0.2222` after correction. With 28 pairs, five seeds cannot produce a
Holm-significant pair even under complete separation; seven seeds are the
minimum for that ceiling. This is a target-success cross-fixture check, not a
family-level ranking.

The first run also exposed a material-order bug in the example: the decoder
pool is ordered L,H by index, while the hand-built template was H,L, so the
canonical write-back silently swapped materials. The example now uses
`materials::stack_template` and `StackTemplate::check_grammar` refuses a
decoder/template order mismatch before any solve. The write-back itself now
preserves merit once the orders match.

## Phase 3 — H0–H7 dispatch and campaign loop

`bench::run_comparison_arm` now dispatches the registry to the engine that
implements each arm:

| arm | driver |
|---|---|
| H0 | `run_h0_control` |
| H1 | `run_recipe_a` |
| H2 | `run_recipe_a` + terminal `finish_recipe_a` |
| H3 | `run_recipe_a_memetic` |
| H4 | `run_recipe_a_tribes` with alternating island mutation regimes |
| H5 | `run_recipe_a_illuminated` |
| H6 | `run_recipe_a_surrogate` |
| H7 | `run_recipe_a_vl_seeded` / `run_recipe_a_vl_proposed`, optionally terminal-finished |

`ComparisonOptions` carries the shared configs, the operator, the mined seed
bank, the terminal insertion cap, and the optional `finish_h7` terminal
finish. H2/H3 return an explicit error when no operator is supplied; H7
returns an explicit error when the seed bank is empty, and its terminal
finish needs the same operator. The dispatcher resets the shared solver counter before each arm and
returns the counter with the result.

`bench::run_comparison_campaign` applies a fixed seed list to every arm
(`recipe`, `h0`, and `vl` seed fields) and returns `CampaignArm` records with
best/median merit and median evaluations. `bench::expected_running_time`
turns that campaign into a fixed-target ERT table: a run succeeds when its
final merit is `<= target_merit`, and `ert = sum(evaluations over all runs) /
successes` censors unsuccessful runs at their observed length. Arms with no
success report `None`. `bench::family_rank_sum` then runs every pairwise
Mann–Whitney comparison in the campaign and applies Holm–Bonferroni over the
whole set of pairs. The dispatch test runs all eight arms on the small
fixture, the campaign test runs H0/H1/H6 over two seeds, the ERT test checks
the all-success and no-success cases, and the family test checks the Holm
adjustment. These are infrastructure tests, not a scientific H0–H7 campaign.

## Phase 3 — ten-seed H0–H7 campaign

`examples/phase2_campaign.rs` runs all eight arms on the C00 antireflection
fixture with seeds `20261121..20261130`, population 24, 30 generations, a
16-layer cap, two islands, and the default H3 schedule (10/2/1). The target is
fixed before the run at `merit <= 30,000`. `--seeds N` and `--target X` expose
the same campaign knobs; the earlier three-seed table is superseded.

| arm | best merit | median merit | median evals | best avg R | layers |
|---|---:|---:|---:|---:|---:|
| H0 | 24,129.155869 | 24,131.772343 | 1,918 | 3.3367 % | 3 |
| H1 | 10,087.289778 | 22,603.001902 | 745 | 2.0015 % | 4 |
| H2 | 3,386.352950 | 22,560.503673 | 783 | 1.0965 % | 4 |
| H3 | **1,095.863799** | 22,576.545574 | 797 | **0.6105 %** | 7 |
| H4 | 16,669.532388 | 18,191.068420 | 1,489 | 2.8110 % | 2 |
| H5 | 19,035.854603 | 23,046.255052 | 745 | 2.9958 % | 1 |
| H6 | 23,639.934509 | 27,137.870962 | **31** | 3.2991 % | 3 |
| H7 | 16,669.257814 | 16,689.336434 | 745 | 2.8109 % | 2 |

At the fixed 30,000 target: H0 has 6/10 successes and ERT 3,344.5; H1 10/10
and 745.0; H2 10/10 and 790.2; H3 10/10 and 796.9; H4 10/10 and 1,489.0; H5
10/10 and 745.0; H6 6/10 and 80.7; H7 10/10 and 745.0. H6's low ERT reflects
only 31 true TMM calls and censored failures, not better quality; its median
merit is above the target. H2 and H3 have the best single runs but their
medians still resemble H1, so the memetic/terminal stages are high-variance
on this fixture.

The Holm-adjusted family has 28 pairs. Twelve survive at 0.05:

```text
H0-H2 0.015636   H0-H3 0.015636   H0-H4 0.004034   H0-H7 0.004034
H1-H6 0.006559   H1-H7 0.042450   H2-H6 0.006559
H3-H6 0.008004   H4-H6 0.004085   H5-H6 0.008004
H5-H7 0.004085   H6-H7 0.004085
```

H0 is significantly worse than H2/H3/H4/H7; H6 is significantly worse in
quality than every other arm except H0; H7 is significantly better than H1
and H5. H0–H1 and H3–H7 are not significant after Holm (adjusted 0.0619).
This is fixed-seed evidence on one fixture and one target, not a universal
ranking.

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
when ties are present. The campaign has a fixed-target ERT report, but the
full H0–H7 multiplicity plan is not claimed yet.

## Phase 3 — H6 linear surrogate screen

`engines::run_recipe_a_surrogate` wraps the fixed-width Recipe A fitness
closure with a small ridge-regularized linear model over depth, total
thickness, mean/min thickness, and material fractions. The first `warmup`
calls and every `sample_every`-th later call are evaluated by navette; the rest
return the current prediction. The final best stack is always re-evaluated by
navette, so a surrogate hallucination cannot become the reported merit.

C00 with the default screen (`warmup=16`, `sample_every=4`, `refit_every=8`):

```text
surrogate_true=125
surrogate_predicted=324
surrogate_samples=124
surrogate_fits=15
surrogate_total_evaluations=247
merit=24129.357559928
avg_R=0.033369146
layers=5
terminal_insertions=20
scan_candidates_after=37
actual_improving_after=0
```

It used 125 true merit evaluations in the GA closure and 247 in total
including the terminal finish. The direct H1 control at a 125-evaluation GA
budget (then the same terminal finish) used 452 total evaluations and reached
`avg_R = 2.6733 %` with 18 layers. H6 therefore saved about 45 % of the true
evaluations but was **worse** in quality on this cheap-TMM fixture. That is
consistent with the strategy's own scope: surrogate screening is for regimes
where a true evaluation is expensive, not a substitute for TMM when TMM is
already cheap. This is a linear screening arm, not a GP/Bayesian optimizer,
and the comparison is one seed and one target.

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

`engines::run_recipe_a_vl_proposed` adds an optional proposal bank: each seed
is retained and `proposals_per_seed` Gaussian copies perturb the material and
thickness keys while preserving the seed's presence pattern. In the C00
campaign, four copies at `noise_sigma = 0.10` left H7 at two decoded layers
and `2.8108 %` average reflectance, essentially unchanged from the raw seed
bank. That is a useful negative: proposal expansion adds local thickness
diversity, not the missing deep architecture. It is a denoising-style
proposal generator, not a trained diffusion model.

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

Limitations: the C00 H0–H7 campaign is ten seeds on one fixture and one fixed
target; the significant pairs are a within-fixture result, not a universal
ranking; the kurobako registration is a problem definition; the archive illumination is not
selection;
the C00 depth bake is a negative scale signal; the filter depth bake is one
target and a 5k LM cap; the seeded 40-layer filter is a stack plus LM, and
the needle-grown 40-layer result is seed-initialized, not a GA discovery; the
equal-budget H0/H1 result is five seeds on one target; the
H7 arm is a mined-seed fallback, not a diffusion implementation; H6 is a
linear screen, not a GP; navette-native A4 and the released-navette gate
remain open. The core `ndarray` lift (`c9e964d`) resolves the earlier
navette/moors array-major split; it is a dependency-major change for the next
navette release.
