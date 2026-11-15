// SPDX-License-Identifier: LGPL-3.0-or-later
//! `navette-ga` — genetic and evolutionary thin-film synthesis on top of the
//! [navette](https://github.com/opticsWolf/Navette) optical engine.
//!
//! This is **one crate**. The former subsystem split is now a module split:
//!
//! | Module | Strategy item | Role |
//! |---|---|---|
//! | [`decoder`] | G2/G3 | variable-length keys, material pool, bounds, pins, grammar |
//! | [`bridge`] | G1/G4 | genome → navette [`DesignStack`] → merit, boundary rules, needle-memetic operator |
//! | [`engines`] | S1–S3 | Recipe A GA, island/tribe backbone, VL operators |
//! | [`materials`] | G3 | navette material library → decoder pool + stack template |
//! | [`qd`] | S4/F1 | MAP-Elites / MOME archive, illuminated run, gradient descriptors |
//! | [`moors_split`] | S6 | `moors` non-dominated split-off |
//! | [`study`] | G7 | campaign queue, pins, pruner, importance |
//! | `bench` | G8/G9 | kurobako problems and performance harness (feature `bench`) |
//!
//! The strategy of record is `docs/strategy/E09_rust_implementation_strategy_v2.md`
//! and `docs/strategy/E10_cherrypick_reassembly_v2.md`; those documents predate
//! the single-crate consolidation, so their subsystem names are normalized to
//! the module map above.
//!
//! [`DesignStack`]: https://docs.rs/navette/latest/navette/smatrix/synthesis/structure/struct.DesignStack.html

pub mod bridge;
pub mod decoder;
pub mod engines;
pub mod materials;
pub mod moors_split;
pub mod qd;
pub mod study;

#[cfg(feature = "bench")]
pub mod bench;

/// The crate version, as declared in `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The current implementation phase.
pub const STATUS: &str =
    "Phase 2 implementation + Phase 3 modules: joint/spectral needles, A2 introspection, A4 enforcement, mid-loop S2, H6 linear surrogate, H7 seed fallback, H0-H7 dispatcher/campaign/ERT/family-Holm, MOME, I1, study, kurobako; Phase 2 evidence (GA-discovered 40-layer filter, H7 diffusion, scientific campaign) open";
