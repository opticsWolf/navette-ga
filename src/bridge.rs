// SPDX-License-Identifier: LGPL-3.0-or-later
//! G1/G4 — fitness bridge and needle-memetic operator.
//!
//! The bridge turns a decoded genome into a navette `DesignStack` (plus any
//! environments), evaluates the joint merit, and returns the scalar fitness
//! with the per-target breakdown used by the QD descriptors. A full generation
//! is batched so stack setup is amortised.
//!
//! The memetic operator runs the needle pipeline over a genome: program-level
//! (`run_needle`) first, then the staged A1 API (`scan/insert/polish/cleanup`)
//! once it lands. Lamarckian write-back is the default; the Baldwinian variant
//! is kept for the V21 ablation.
