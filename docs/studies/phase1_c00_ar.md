# Phase 1 C00-class AR ledger

**Date:** 2026-11-14
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
merit=1209.344267754
avg_R=0.006592960
max_R=0.016106028
layers=16
insertions=8
lm_converged=true
min_thickness_nm=10.000000
scan_candidates_after=56
actual_improving_after=0
stack=L:110.070,H:34.870,L:27.428,H:45.576,L:24.395,L:19.879,H:15.983,L:107.396,L:65.207,L:13.241,L:11.541,L:10.000,L:174.970,L:237.085,L:283.371,L:174.927
```

The stack is listed ambient-side first; thicknesses are nm. Average and maximum
reflectance are fractions (0.00659 = 0.659 %, 0.01611 = 1.611 %).

## Certificate

- **LM:** the final `refine_thicknesses_report` terminated by a convergence
  test (`lm_converged = true`).
- **d_min:** the smallest film is `10.000 nm`, exactly the configured lower
  bound; the finish did not leave a film below it.
- **Scan:** the final analytic sweep returned 56 predicted-improving candidates,
  but **trial insertion and real merit evaluation found zero actual
  improvements** (`actual_improving_after = 0`). That is the Phase 1 scan
  certificate: no scanned candidate insertion improves the joint merit.
- **Write-back:** the terminal stack was encoded into the decoder genome,
  re-decoded, and reassembled; the stored genome, decoded stack, and
  `DesignStack` agree.

## Reproduction notes

The run is deterministic with the pinned seed because the GA's Rayon fan-out is
disabled. `genetic_algorithms`' own docs note that seeded reproducibility
otherwise requires a fixed thread pool. The material-pool order and values are
recorded by `ga::materials`; no hidden library state is used.

## Not covered

This ledger does not claim C00's exact published design, a fabricated sample, or
roughness/absorption robustness. Those are later Phase 2/3 runs. It does close
the Phase 1 code-and-ledger caveat: terminal needle/LM, a real material loader,
the scan certificate, and a recorded stack/merit are present.
