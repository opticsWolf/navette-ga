# navette-ga

[![License: LGPL-3.0-or-later](https://img.shields.io/badge/license-LGPL--3.0--or--later-blue)](COPYING.LESSER)

Genetic / evolutionary thin-film synthesis on top of [navette](https://github.com/opticsWolf/Navette).
This is the working repository for the strategy set in
[`docs/strategy/`](docs/strategy/): variable-length genomes, material-pool
decoding, islands/tribes, needle-memetic refinement, multi-objective splits,
quality-diversity archives, and the study/proof layer around them.

**Status:** Phase 1 in progress. `ga::decoder` (G2), the `ga::bridge` core
(G1), the A1 memetic operator, and a single-island Recipe A GA path are
implemented and tested. The GA runs `genetic_algorithms` over three random
keys per layer, decodes with presence-coded variable length, assembles a
navette `DesignStack`, and minimizes joint merit. Closed-form quarter-wave and
V-coat checks pass, and the G9 micro-bench covers evaluate, LM, A1 scan, and
scan+insert+polish. A1/A3 live on the local Navette `dev_ga` checkout. The
bridge now carries navette's insert and edge-list boundary rules, maps
single-material edges to decoder pins, and checks insert pairs against the
adjacency grammar (E09 v2 §2.4.1 Phase 1), and verifies flat boundary needles
against host and edge rules; single-material edges become pins, multi-material
edges validate at assembly, and seeding is rejected. Joint boundary needles,
joint gaps, multi-material edge checks, A2/A4, and the Phase 2 island driver
remain. Publishing waits for a navette release containing A1/A3. The
specification of record is
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
  bridge.rs                # G1/G4: fitness bridge, boundary rules, needle-memetic operator
  engines.rs               # S1-S3: Recipe A GA + island/tribe backbone
  qd.rs                    # S4/F1: MAP-Elites / MOME archive glue
  study.rs                 # G7: blackopt/optimizer study wiring
  bench.rs                 # G8/G9: kurobako problems + perf harness (feature)
  main.rs                  # Recipe A/B binary scaffold
examples/
  phase0_bench.rs          # G9: 61-wavelength evaluate/LM timing harness
docs/
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
