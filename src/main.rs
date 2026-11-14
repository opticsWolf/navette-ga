// SPDX-License-Identifier: LGPL-3.0-or-later
//! Recipe A/B driver scaffold. The actual campaign wiring lands in Phase 1.

fn main() {
    println!(
        "navette-ga {} ({})",
        navette_ga::VERSION,
        navette_ga::STATUS
    );
    println!(
        "modules: decoder, bridge, engines, qd, study{}",
        if cfg!(feature = "bench") {
            ", bench"
        } else {
            ""
        }
    );
}
