// SPDX-License-Identifier: LGPL-3.0-or-later
//! G7 — study layer.
//!
//! Thin wiring over `blackopt` / `optimizer`: enqueue QD bright cells and
//! proposal operators, pin layers with PartialFixed, prune dead tribes, and
//! explain campaigns with fANOVA. The campaign object stays ours; the crates
//! stay dependencies.
