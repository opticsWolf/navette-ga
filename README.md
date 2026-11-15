# navette-ga

[![License: LGPL-3.0-or-later](https://img.shields.io/badge/license-LGPL--3.0--or--later-blue)](COPYING.LESSER)

Genetic / evolutionary thin-film synthesis on top of [navette](https://github.com/opticsWolf/Navette).
This is the working repository for the strategy set in
[`docs/strategy/`](docs/strategy/): variable-length genomes, material-pool
decoding, islands/tribes, needle-memetic refinement, multi-objective splits,
quality-diversity archives, and the study/proof layer around them.

**Status:** Phase 2 and the implementable Phase 3 modules are in place.
`ga::decoder` (G2), the `ga::bridge` core (G1), the A1 memetic operator, the
G3 material loader, both Recipe A GA paths (single island and the Phase 2
ring), heterogeneous islands/tribes, static and ramped parsimony, and the
terminal LM/needle finish are implemented and tested. The Phase 1 exit
artifact is recorded in
[`docs/studies/phase1_c00_ar.md`](docs/studies/phase1_c00_ar.md): a C00-class
H/L AR over 400–900 nm with Lamarckian write-back and a zero-actual-improvement
scan certificate. The same example can run the H3 schedule (`--memetic`) or
an equal-solver-budget H0/H1 comparison (`--h0-budget N`), the H6 surrogate
screen (`--surrogate`), or the H7 mined-seed fallback (`--seeded`); the first
C00 comparison puts H1 ahead on all five seeds at 2k and 20k evaluations, and
the H6 screen is cheaper but not better at equal true budget on this cheap-TMM
fixture. The forty-layer seeded filter artifact is
`examples/phase2_filter_40.rs`; `examples/phase2_filter_depth.rs` is the
merit-versus-depth bake. `examples/phase2_campaign.rs` runs the H0–H7
dispatcher over three fixed seeds and prints ERT plus Holm-adjusted pairwise
rank sums.

The boundary path now covers read-only joint gap acceptance, structural
candidate enumeration, transactional chosen-locus insertion, the spectral
joint scan/step/run, one-cycle joint equivalence with `run_needle_cycles`,
cleanup edge preservation, and a two-step multi-material joint run on H/L/B
with a fixed cover environment. A2 introspection (`ga-introspect` merit parts
and the thickness residual Jacobian) and consumer-side A4 stack validation are
exposed through the bridge/decoder. Phase 2/3 modules add the S4/F1 MOME
archive and illuminated Recipe A run, the S6 `moors` split, the H0 control,
I1 cross-island multi-parent crossover, the G7 campaign queue/pins/pruner,
the G8 kurobako problem/H0–H7 registry and dispatcher, the G4/S2 mid-loop
needle schedule for single-island and ring runs, the H6 linear surrogate, and
the H7 mined-seed fallback. Full suite: **81 tests**;
`--all-features` includes the kurobako proof module.

Limits are explicit: navette-native A4 (per-material bounds and the
insert/migrate grammar hook), the H7 diffusion proposal, a GA-discovered
Recipe B filter (the seeded 40-layer artifact and depth bake exist), and the
full multi-seed H0–H7 campaign remain; a three-seed C00 campaign is
recorded in the ledger.
The mid-loop schedule has one C00 overhead measurement, not a controlled arm
result. Consumer-side A4 is now
enforced on every GA/terminal/joint path; the earlier C00 ledger was
superseded because it used an LM clamp below the decoder's material floor. The ledger is
[`docs/studies/phase2_phase3_ledger.md`](docs/studies/phase2_phase3_ledger.md).
Publishing waits for a navette release containing A1/A2/A3. The specification of
record is
[`E09_rust_implementation_strategy_v2.md`](docs/strategy/E09_rust_implementation_strategy_v2.md)
and [`E10_cherrypick_reassembly_v2.md`](docs/strategy/E10_cherrypick_reassembly_v2.md).

## One crate, one package

The repository is a **single Cargo package**, `navette-ga` (library
`navette_ga` plus the `navette-ga` binary). The former subsystem split is a
module split inside `src/`; there are no subcrates and no workspace.

```
Cargo.toml                 # package navette-ga
Cargo.lock
CREDIT.md                  # provenance ledger (copied code only)
src/
  lib.rs                   # crate docs + module map
  decoder.rs               # G2/G3: VL keys, material pool, bounds, pins, grammar
  materials.rs             # G3: navette material library -> decoder pool/template
  bridge.rs                # G1/G4: fitness bridge, boundary rules, needle-memetic operator
  engines.rs               # S1-S3: Recipe A GA (fixed/VL) + island driver
  qd.rs                    # S4/F1: MAP-Elites / MOME archive + illuminated run
  moors_split.rs           # S6: moors non-dominated split
  study.rs                 # G7: campaign queue/pins/pruner/importance
  bench.rs                 # G8/G9: kurobako problem + H0-H7 registry (feature)
  main.rs                  # Recipe A/B binary scaffold
examples/
  phase0_bench.rs          # G9: 61-wavelength evaluate/LM timing harness
  phase2_depth_bake.rs     # C00 depth/MOME bake with production screen
  phase2_filter_40.rs      # forty-layer seeded H/L edge-filter artifact
  phase2_filter_depth.rs   # 10/20/30/40/50-layer filter depth bake
  phase2_campaign.rs       # H0-H7 campaign + ERT + Holm (bench feature)
docs/
  studies/                 # Phase 1 C00-class AR ledger and run artifacts
  README.md                # research/paper index
  integration.md           # how to add or link this crate in Navette
  strategy/                # E08-E10: the implementation strategy set
  surveys/                 # E00-E07: Rust crate surveys / gap analysis
  papers/                  # PAPER_* markdown drafts + technical review PDF
staging/STAGED-COPIES.md   # E09 v2 §1.1 copy gate
research/                  # local-only: PDFs, kb.db, probe artifacts
vendor/                    # local-only upstream mirrors
```

`research/` and `vendor/` are outside git: they hold roughly 360 MB of source
PDFs, a multi-GB research database, and pinned upstream checkouts.
`docs/README.md` records what the PDFs are; `CREDIT.md` records the mirror
commits.

The strategy documents predate the single-crate consolidation; their
subsystem names are normalized to the module map above.

## Build

Rust 1.90+ is the declared MSRV. The repository pins **1.98.0** in
`rust-toolchain.toml` so the opt-in `bench` feature works: `kurobako` pulls
`rlua` 0.17.1, which does not compile on rustc 1.100 yet. The **default**
build (without `bench`) does compile on current stable.

The package expects the Navette checkout as a **sibling**:

```
<parent>/Navette/rust/navette
<parent>/navette_ga
```

Then:

```sh
cd navette_ga
cargo check
cargo test
cargo check --all-features   # includes the bench feature
```

The single layout dependency is the `navette` entry in `[dependencies]`. If
Navette lives elsewhere, change that one path; if you want the published
engine, drop the `path` (the version already is `0.10.0`).

## Features

| Feature | Default | Pulls | Why |
|---|---|---|---|
| `bench` | off | `kurobako` | G8/G9 proof harness; `kurobako -> rlua` needs the 1.98 toolchain |

Everything else (genetic algorithm, QD archive, Pareto, GP/EGO, BO, study
layer) is a normal dependency of the one crate.

## Publishing

The repository root is the package, so `cargo publish` publishes the whole
implementation as one crate. The `navette` path dependency carries a version
(`0.10.0`) and is stripped on publish; `bench` stays opt-in. See
[`docs/integration.md`](docs/integration.md) for adding the crate to the
Navette repository directly, or linking it from there.

## Integrating with Navette

- **Direct add:** copy `Cargo.toml` + `src/` into `Navette/rust/navette-ga/`,
  change the `navette` path to `../navette`, and add
  `"rust/navette-ga"` to Navette's `members`. No workspace table to merge.
- **Link:** Navette takes a path or git dependency on `navette-ga`.

Full recipes, including the git-dependency caveat for the `navette` path, are
in [`docs/integration.md`](docs/integration.md).

## License

LGPL-3.0-or-later, the same license as navette. See
[`COPYING.LESSER`](COPYING.LESSER) (the LGPL-3.0 additional permissions) and
[`COPYING`](COPYING) (GPL-3.0). Upstream code copied under the Phase-3 gate is
attributed in [`CREDIT.md`](CREDIT.md) and keeps its original license notice;
Apache-2.0, MIT, and MPL-2.0 sources are compatible with LGPL-3.0-or-later.
`navette` itself is consumed as a dependency and is never copied here. All
resolved dependencies are permissive or LGPL-3.0-or-later; the build contains
no GPL, AGPL, or unknown-license code.
