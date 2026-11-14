// SPDX-License-Identifier: LGPL-3.0-or-later
//! G2/G3 — genome decoder and material pool.
//!
//! Owns the variable-length random-key genome, the material-pool mapping,
//! per-material thickness bounds, pinned layer indices, the adjacency grammar,
//! and the operator masks that say which keys an operator may touch. One
//! decoder serves the GA, the needle seeding path, and repair.
//!
//! Physics-backed tables (bounds, pins, grammar) are derived from navette's
//! A4 API at startup; navette stays the single owner of legality.
