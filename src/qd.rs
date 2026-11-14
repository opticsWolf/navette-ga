// SPDX-License-Identifier: LGPL-3.0-or-later
//! S4/F1 — quality-diversity archive.
//!
//! MAP-Elites / CVT archive glue over `symbios-genetics`, descriptors computed
//! from navette outputs, and the F1 MOME per-bin Pareto fronts built in the
//! symbios fork. The gradient path (A2 thickness Jacobian) and the standard
//! archive path (layer count, motif flags) are kept separate deliberately.
