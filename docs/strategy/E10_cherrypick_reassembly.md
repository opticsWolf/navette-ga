# E10 — Cherry-picking crate code into a purpose-built crate: license verdicts + reassembly plan

*Question: can we lift code from the surveyed crates and reassemble it into one
new purpose-focused crate? Answer: yes for ~90 % of the shortlist — the
licenses sort the crates into four buckets below (SPDX `license` fields from
crates.io metadata, 2026-11-09; `non-standard`/`AML` mean the crate publishes
no recognised SPDX identifier). Rule of the road: **depend** where the API is
clean, **copy** only what you must mutate, **isolate** the copyleft, and never
copy the unknown.*

## 1. License buckets (latest non-yanked version each)

**Bucket A — copy freely (MIT / Apache-2.0, keep copyright notices):**
`moors`, `symbios-genetics`, `evolutionary`, `radiate`, `pso-rs`, `sefar`,
`optlib`, `qopt`, `neat-rs`, `rustneat`, `crfmnes`, `fcmaes-core`,
`metaheuristics-nature`, `kurobako`, `optimizer`, `genetic-rs`, `nsga_rs`,
`optirustic` (all MIT); `genetic_algorithms`, `egobox`, `oxicuda-evol`,
`scirs2-optimize` (Apache-2.0); `basin`, `blackopt`, `genoxide`, `argmin`,
`cmaes`, `haru_cmaes`, `differential-evolution`, `ys_differential_evolution`,
`frostfire`, `mincost`, `ganesh`, `rlevo`, `ecrs`, `rsgenetic`,
`genetic-algorithm`, `genevo` (MIT OR Apache-2.0 dual).

**Bucket B — copy with file-level discipline (MPL-2.0):** `oxigen`,
`ratio-genetic`. MPL permits combination into a larger work; any *modified*
MPL file stays MPL with its notice intact. Practical handling: copy the
age-unfitness / survival-pressure modules as separate files, keep their
headers, note them in a `THIRD-PARTY-MPL` file. No contagion to your own files.

**Bucket C — do not copy into the new crate (strong copyleft):**
`mahf`/`mahf-bmf`/`mahf-coco` (GPL-3.0-or-later — copying GPL code obligates
the whole crate to GPL-3.0); `red-queen-core` (**AGPL-3.0** — and, substantively,
an LLM-jailbreak red-teaming framework, bassrehab/red-queen, single 0.1.0
release 2026-03-16, 229 dl: MAP-Elites over *attack strategies* for AI safety,
not a general EC library — wrong domain, immaterial single release, strongest
copyleft: avoid entirely, not even as a dependency of a service).
Use `mahf-coco` exclusively as an **external harness over a process/CLI
boundary** (it already is a CLI-shaped tool): no linking, no license contact.

**Bucket D — do not copy (unknown terms):** `genetic` (license string literally
"AML" — most likely the author's 2015-era shorthand for Apache/MIT dual
licensing, but the upstream repo (Kerosene2000/genetic-rs) is deleted and no
LICENSE text survives, so it is unverifiable and therefore unusable; the crate
itself is a Dec-2015 toy, "a little lib to use genetic algorithm", sustained
by a decade of short-name drive-by downloads), `easy_ga` + `simulated_annealing` (non-standard
license strings — read the actual LICENSE text before touching; both are
anyway trivial to reimplement cleanly). `push-trait` (CC0 — free, but
irrelevant to EC).

**navette (LGPL-3.0-or-later):** owner-controlled — A5 decides. Until then,
consume it as a **cargo dependency**, never copy its files into the new crate
(copying LGPL code in makes that part LGPL and muddies the A5 options).

## 2. Depend vs copy vs reimplement (per subsystem)

| Subsystem | Disposition | Source & licence cost |
|---|---|---|
| Bounded LM / refiners | **Depend**: `basin` (MIT/Apache) via navette `opt-basin` | Zero copied lines |
| Pareto fronts | **Depend**: `moors` (MIT) | Zero copied lines |
| Surrogate (Gpx/Egor) | **Depend**: `egobox` (Apache-2.0) | Zero copied lines |
| Study layer | **Depend**: `blackopt`/`optimizer` (MIT/Apache, MIT) | Zero copied lines |
| Proof harness | **Depend/CLI**: `kurobako` (MIT) | Zero copied lines |
| Second pillar (BO/MOEA-D) | **Depend**: `genoxide` (MIT/Apache, pinned) | Zero copied lines |
| VL chromosome + Insertion/Deletion + Pad crossover + length penalty | **Copy** from `genetic_algorithms` (Apache-2.0) | ~hundreds of lines; keep Apache notice + attribution; this is THE copy — no other VL source exists (V11) |
| Island-GA driver (topology, migration, heterogeneous islands) | **Copy-skeleton** from `genetic_algorithms` IslandGa, rewired to navette A3 batch eval | Apache-2.0, attribute; mutate freely |
| Engine composability backbone (Simulation loop, Termination OR/AND combinators, Operator/Population/Genotype traits, GenomeBuilder) | **Copy-skeleton** from `genevo` 0.7 (MIT/Apache) as the `ga::engines` assembly backbone, with mahf's builder-DSL ergonomics as the API reference | Dormancy irrelevant for copies (pin 0.7); attribute; this is the Rust-idiomatic answer to "try seven hybrids in a week" without GPL contact |
| QD archive base (MAP-Elites/CVT/speciation) | **Fork** `symbios-genetics` (MIT) → add MOME per-bin fronts (F1) in-fork | MIT; offer F1 upstream as PR, carry fork until merged |
| Age layering (ALPS pattern) | **Copy** `oxigen` age-unfitness module (MPL-2.0, Bucket B file discipline) *or* reimplement (~60 lines) — reimplementing dodges even MPL notices; recommended | Either clean |
| CMA-ES / JADE / SHADE cores | **Copy** from `genoxide` or `cmaes` (MIT/Apache) if the second-pillar dep proves too heavy for the inner loop | Attribute; benchmark before forking |
| Baseline engines (RGA/DE/PSO/Firefly) | **Copy** `metaheuristics-nature` (MIT) Solver-shaped baselines | Small, self-contained |
| Tribes/CTCM logic | **Reimplement** from D11 (paper, not code — no crate owns it) | Ours |
| Decoder / pool / grammar (G2/G3) | **Reimplement** (v3 work item 5; A4 moves legality into navette) | Ours |
| SAIL/POME/DQD/LLM glue (G5–G7, F2–F4) | **Reimplement** (no crate composes these) | Ours |
| `mahf-coco` comparisons | **External harness only** (Bucket C boundary) — but see below: this is a genuine *need*, not just a quarantine. Three distinct needs: (1) **mahf-coco**: COCO/BBOB bindings as the S10 publication-proof runner for the seven-hybrid comparison + V9/V21 ablations — GPL triggers on conveyance, not on in-house benchmarking, so local use is unrestricted; distribute only our code + scripts with mahf-coco as a documented external requirement (CLI/process boundary is belt-and-braces). (2) **mahf core**: builder-DSL composability + `boundary::Saturation` + replacement schemes + state-tracking/logging interfaces as *design patterns to mirror* (reimplement, never copy) — the architectural template for `ga::engines`. (3) **mahf-bmf**: nothing (A11/kurobako/GA-benches cover it; only exact function parity would justify a look). **mahf-tsplib**: nothing (TSP-only). | No contact |

## 3. Proposed layout (`navette-ga`, one crate)

```
navette-ga/                        # one package, LGPL-3.0-or-later (same as navette)
├── src/decoder.rs                 # G2+G3: VL keys, bounds[m], grammar, pins, pool (fresh)
├── src/engines.rs                 # copied: VL chromosome+operators (genetic_algorithms, Apache),
│                                  # island driver skeleton (Apache), age layering (fresh reimpl),
│                                  # baselines (metaheuristics-nature, MIT) — NOTICE file per source
├── src/qd.rs                      # fork of symbios-genetics (MIT) + F1 MOME + F2 descriptors
├── src/bridge.rs                  # G1/G4: fitness bridge + memetic operator over A1–A3 (fresh)
├── src/study.rs                   # G7: thin wrappers over blackopt/optimizer deps (fresh)
├── src/bench.rs                   # G8/G9: kurobako problem + perf harness (feature `bench`)
├── src/main.rs                    # Recipe A/B binaries
├── CREDIT.md + per-file headers   # Bucket A/B attribution; MPL file list
└── Cargo.toml                     # one package; subsystem dependencies stay normal deps
```

Why this shape: the **copied surface is minimal** (VL machinery that exists
nowhere else + island skeleton + QD fork base); everything with a clean API
stays a dependency (no fork drift); copyleft never crosses a link boundary
(GPL via CLI, LGPL via dep until A5); unknown-license crates contribute
nothing. Estimated copied volume: <2 kSLoC against ~1,500 lines of fresh glue
(E09 §3) — the new crate is overwhelmingly ours, with provenance recorded per
file.

## 4. Maintenance rules (avoiding fork rot)

1. Copy at pinned versions; record `source = crate@version, files:` in
   CREDIT.md so updates are diffable.
2. Prefer upstream PRs for anything generally useful (F1 MOME → symbios;
   A1–A4 → navette, trivially, since owner = you) and shrink the fork with
   every release.
3. Never cherry-pick from Bucket C/D; never vendor navette files (dep only).
4. Re-run `cargo deny`-style license checks in CI so a transitive dep can't
   smuggle GPL/AGPL into the workspace later (`scirs2-optimize`-scale
   mono-repos and `cool-japan`-style orgs are the shape of that risk).

*Sources: crates.io `license` fields (two meta batches, 2026-11-09); E00 (code
provenance per crate); E09 (glue/fresh inventories G1–G9/F1–F5, A1–A5); v3
§§5–9/13 (what the copied modules must do). SPDX note: `MIT/Apache-2.0`
(slash) vs `MIT OR Apache-2.0` both mean dual-licensed choice; `AML` and
`non-standard` are not valid SPDX — treated as all-rights-reserved until the
LICENSE file says otherwise.*
