# Phase 1 C00-class AR ledger

**Date:** 2026-11-15 (A4-enforced rerun of the 2026-11-14 artifact)
**Command:** `cargo run --release --example phase1_c00_ar`
**Seed:** `20261114`
**Engine:** `genetic_algorithms 3.0.0` with its `parallel` feature disabled so
the seeded run is deterministic; navette's A3 batch door remains parallel for
GA evaluation.

This is the Phase 1 exit artifact for the E09 v2 §5 Recipe A line: a
two-material H/L antireflection search over 400–900 nm at normal incidence,
followed by a terminal LM polish and needle steps. It is C00-class in scope
(two-material AR, no absorption, no roughness), not a reproduction of C00's
published numerical optimum.

## Problem

| Item | Value |
|---|---|
| Wavelengths | 400–900 nm, 25 nm step (21 points) |
| Angle | 0° |
| Target | `Rs = 0` on every point |
| Ambient / substrate | air (n = 1.0) / glass (n = 1.52) |
| Materials | H (n = 2.30), L (n = 1.45), bounds 10–300 nm |
| Decoder | all-allowed grammar, no pins, 32 keys |
| GA | population 64, 100 generations, Tournament/Uniform/Gaussian, σ = 0.2 |
| Terminal | LM, then up to 20 needle insertions, LM, scan certificate |

## Recorded result

```text
PHASE1_C00_AR
merit=24474.958774584
avg_R=0.033589064
max_R=0.042487618
layers=8
insertions=20
lm_converged=true
min_thickness_nm=19.705781
scan_candidates_after=67
actual_improving_after=0
stack=L:146.482,L:22.846,L:19.706,L:135.887,L:226.173,L:275.571,L:268.676,L:227.716
```

The stack is listed ambient-side first; thicknesses are nm. Average and maximum
reflectance are fractions (0.03359 = 3.359 %, 0.04249 = 4.249 %).

**Superseded result.** An earlier revision of this ledger recorded a 16-layer
H/L stack at `avg_R = 0.659 %`. That run used an LM clamp of `5 nm`, below the
decoder's `10 nm` material bound. The terminal stack only appeared to satisfy
the bound because `encode_films` clamped the written-back thicknesses after the
scan certificate had been computed. With A4 enforcement the evaluator and the
decoder now share the same `10 nm` floor, and the old artifact is not a legal
design. The run above is the replacement recorded under the enforced bound.

## Certificate

- **LM:** the final `refine_thicknesses_report` terminated by a convergence
  test (`lm_converged = true`).
- **d_min:** the smallest film is `19.706 nm`, above the configured `10 nm`
  floor; the finish did not leave a film below it.
- **Scan:** the final analytic sweep returned 67 predicted-improving candidates,
  but **trial insertion and real merit evaluation found zero actual
  improvements** (`actual_improving_after = 0`). That is the Phase 1 scan
  certificate: no scanned candidate insertion improves the joint merit.
- **Write-back:** the terminal stack was encoded into the decoder genome,
  re-decoded, and reassembled; the stored genome, decoded stack, and
  `DesignStack` agree. The certificate is now computed **after** this
  write-back, not before it.
- **A4:** every stage of the finish validates the decoder's per-material
  thickness bounds and adjacency grammar. The evaluator clamp is `10–300 nm`,
  matching the material floor; a stack outside those bounds is refused rather
  than silently clamped.

## Reproduction notes

The run is deterministic with the pinned seed because the GA's Rayon fan-out is
disabled. `genetic_algorithms`' own docs note that seeded reproducibility
otherwise requires a fixed thread pool. The material-pool order and values are
recorded by `ga::materials`; no hidden library state is used.

## Phase 2 H3 follow-up

The same example exposes the mid-loop schedule:

```text
cargo run --release --example phase1_c00_ar -- --memetic
```

It runs `run_recipe_a_memetic` with the default schedule
(`interval = 20`, `top = 2`, `max_insertions = 1`), then the same terminal
finish. The recorded run reached `merit = 4559.194362058`,
`avg_R = 1.1705 %`, 12 layers, and a zero actual-improvement terminal
certificate. The schedule refined 10 chromosomes and accepted 3 write-backs;
its wall-time share was 5.6–6.3 % across repeated runs. The earlier
aggressive setting (`10 / 8 / 2`) reached `avg_R = 0.7941 %` but cost 63.4 %
of wall time and was worse on this seed; the Phase 2 ledger holds the full
schedule table and caveat.

## Equal-budget H0 control

`cargo run --release --example phase1_c00_ar -- --h0-budget 2000` runs the
classical random-start/LM control and H1 at the same solver-evaluation count.
The recorded runs gave H0 `avg_R = 16.08 %` at 2,048 evaluations versus H1
`3.3849 %` at 2,049; at 20,000 evaluations H0 improved to `6.7780 %` while H1
stayed at `3.3849 %`. This is one seed and one target; see the Phase 2 ledger
for the caveat.

## Not covered

This ledger does not claim C00's exact published design, a fabricated sample, or
roughness/absorption robustness. Those are later Phase 2/3 runs. It does close
the Phase 1 code-and-ledger caveat: terminal needle/LM, a real material loader,
the scan certificate, and a recorded stack/merit are present.
