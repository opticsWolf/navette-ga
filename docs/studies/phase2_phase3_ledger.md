# Phase 2 / Phase 3 implementation ledger

Date: 2026-11-15. Scope: the remaining Phase 2 items in E09 v2 §5 plus the
Phase 3 modules that can be built on the same bridge. This is an engineering
ledger, not a campaign report: every number below comes from a test in the
repository, and every limitation is listed rather than inferred away.

## Environment

- `navette` local `dev_ga` at `c25a0d5` (A1/A3 plus the A2 introspection and
  public joint-scan additions below).
- `navette-ga` single crate, LGPL-3.0-or-later.
- `cargo test --all-features`: **64 passed**.
- `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`: clean.
- `navette` with `ga-introspect`: **801 passed, 1 ignored**.

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
  `FitnessEvaluator::thickness_residual_jacobian`. The test verifies the parts
  sum to the scalar merit and the Jacobian has one column per free film.
  The returned Jacobian is the **residual** matrix plus its shape and film
  rows; `merit_thickness_gradient` composes `2 Jᵀr`. Only a version-matched
  `DMatrix` wrapper remains (navette's `ndarray` major differs from
  `moors`').

Honesty boundary: the Jacobian prices thickness moves, not insertion
positions. It is not a P(z) adjoint.

## Phase 2 — A4 consumer-side legality

`Decoder::validate_stack` resolves every film to a pool material, checks the
material's thickness bounds, and checks every adjacent pair against the
decoder grammar. This is the consumer half of A4; navette's native
per-material bounds and insert/migrate grammar hook remain open because they
require changing navette's material schema and LM parameter bounds.

## Phase 2 — H0 control

`engines::run_h0_control` runs independent random starts, a bounded LM
refinement, an optional memetic needle step, and a second refinement. It
returns the same `RecipeAResult` shape as the GA so the comparison harness can
report it without a second result type.

## Phase 2 — S4/F1 archive and S6 split

- `qd::MomeArchive` is a two-dimensional MAP-Elites grid over layer count and
  total thickness. Each cell keeps a nondominated front over
  `(merit, total thickness)`; `insert_at` is the DQD/gradient door and
  `GradientDescriptor::from_jacobian` turns an A2 matrix into two normalized
  axes.
- `engines::run_recipe_a_illuminated` records every legal generation member
  while the ordinary Recipe A selection runs. This is the S4-lite map, not a
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
merit-vs-depth bake is claimed; navette-native A4 and the released-navette
gate remain open.
