// SPDX-License-Identifier: LGPL-3.0-or-later
//! G8/G9 — proof harness and performance checks.
//!
//! kurobako problem definitions (Recipes A/B, the H0–H7 comparison set, the
//! V9/V21 ablations) plus the G9 micro-benchmark that checks the bridge and
//! the needle-memetic cadence against navette's published scan/LM timings.
//!
//! Enabled by the `bench` feature: kurobako pulls `rlua` 0.17.1, which does
//! not compile on rustc 1.100, so this module stays out of the default build.
