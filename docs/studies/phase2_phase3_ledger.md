# Phase 2 / Phase 3 implementation ledger

Date: 2026-11-15. Scope: the remaining Phase 2 items in E09 v2 §5 plus the
Phase 3 modules that can be built on the same bridge. This is an engineering
ledger, not a campaign report: every number below comes from a test in the
repository, and every limitation is listed rather than inferred away.

## Environment

- `navette` local `dev_ga` at `c9e964d` (A1/A3, A2 introspection, public
  joint scan, and the core `ndarray` 0.16 lift).
- `navette-ga` single crate, LGPL-3.0-or-later.
- `cargo test --all-features`: **69 passed**.
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

The 40-layer bake, equal-compute H0 comparison, and H6/H7 implementations
remain open.

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

The C00-class example now accepts `--memetic`; one release run produced:

```text
merit=1440.475960295
avg_R=0.007941379
max_R=0.011608623
layers=14
terminal_insertions=0
min_thickness_nm=12.341295
scan_candidates_after=53
actual_improving_after=0
passes=10, refined=80, improved=13
insertions=143, scans=152, prediction_rejected=67
write_back_rejected=0, merit_rejected=0
ga_seconds=0.481059, memetic_seconds=0.831807
memetic_share=0.633581
```

Caveat: this is one problem and one config, not an equal-compute H1/H3
comparison. The 63 % share means the memetic arm is much more expensive per
generation; the 0.794 % average reflectance shows it can find a much better
legal design, but the H1/H3 budget-matched campaign is still open. The
terminal scan certificate is still zero actual improvements.

## Phase 2 — H0 control

`engines::run_h0_control` runs independent random starts, a bounded LM
refinement, an optional memetic needle step, and a second refinement. It
returns the same `RecipeAResult` shape as the GA so the comparison harness can
report it without a second result type. A start whose LM path leaves the
decoder's A4 bounds is discarded rather than aborting the arm. Equal-compute
budgeting is still not implemented.

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
the archive illumination is not selection; no 40-layer production filter or
merit-vs-depth bake is claimed; the mid-loop memetic evidence is one C00
overhead run, not an equal-compute H0/H1/H3 campaign; navette-native A4 and
the released-navette gate remain open. The core `ndarray` lift (`c9e964d`) resolves the earlier
navette/moors array-major split; it is a dependency-major change for the next
navette release.
