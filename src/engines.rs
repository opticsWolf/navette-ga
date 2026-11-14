// SPDX-License-Identifier: LGPL-3.0-or-later
//! S1–S3 — evolutionary engines and assembly backbone.
//!
//! Starts as a dependency on `genetic_algorithms` 3.0 (island GA, VL
//! chromosome, length mutation, variable-length crossover, parsimony), then
//! adopts the `genevo` 0.7 Simulation/Operator/Population traits when the
//! E09 v2 §1.1 copy gate opens. Both copies are Apache-2.0 / MIT-Apache and
//! are recorded in `CREDIT.md` at the moment a file moves in.
//!
//! Heterogeneous islands and tribes live here; migrations and topology are
//! configuration, not separate crates.
