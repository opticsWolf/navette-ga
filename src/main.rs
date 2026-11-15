// SPDX-License-Identifier: LGPL-3.0-or-later
//! Recipe A/B driver scaffold. The campaign CLI is still a Phase 2/3 follow-up.

fn main() {
    println!(
        "navette-ga {} ({})",
        navette_ga::VERSION,
        navette_ga::STATUS
    );
    println!(
        "modules: decoder, bridge, engines, qd, moors_split, study{}",
        if cfg!(feature = "bench") {
            ", bench"
        } else {
            ""
        }
    );
}
