# E10 v2 — Cherry-picking crate code into a purpose-built crate: license verdicts + reassembly plan

> **Naming note (2026-11-14):** the reassembly target is the single
> `navette-ga` crate; the module names below live under `src/`. The
> license verdicts and copy gate are unchanged. The current adoption recipes
> are in `docs/integration.md`.

*Question: can we lift code from the surveyed crates and reassemble it into one
new purpose-focused crate? Answer: yes for ~90 % of the shortlist — the
licenses sort the crates into four buckets below (SPDX `license` fields from
crates.io metadata, 2026-11-09; `non-standard`/`AML` mean the crate publishes
no recognised SPDX identifier). Rule of the road: **depend** where the API is
clean, **copy** only what you must mutate, **isolate** the copyleft, and never
copy the unknown. v2 responds to the independent technical review: the C3
architecture fork is resolved by the governing-split decision (§0), the
genevo backbone is now shared with E09 v2 (§2), easy_ga carries a run-only
rule (C5), and the maintenance rules own R3–R5.*

## 0. Governing-split decision (review C3, resolved with E09 v2)

The pre-v2 E09/E10 drafts (since removed) prescribed different architectures
for the VL chromosome, the island driver, and the QD archive with no
reconciliation. The single governing strategy, recorded identically in both v2
documents:

- **Phases 0–2: E09 governs — depend.** Every crate below with a clean API is
  consumed as a cargo dependency. No copies, no forks (F1 excepted as a
  branch, not a direction).
- **Phase 3: E10 governs the three named copies, gated on measured need.**
  The VL chromosome + operators copy, the island-driver skeleton copy (both
  Apache-2.0, from `genetic_algorithms`), and the `symbios` fork (MIT, for F1
  MOME) land only for modules that demonstrably required mutation during
  Phases 0–2, with written proof in S10. Everything else in §2 marked
  "Depend" stays a dependency permanently.
- The `genevo` 0.7 backbone is adopted into E09 v2's Phase 3 as the
  `ga::engines` assembly skeleton — no longer an E10-only decision (§2).

## 1. License buckets (latest non-yanked version each)

**Bucket A — copy freely (MIT / Apache-2.0, keep copyright notices):**
`moors`, `symbios-genetics`, `evolutionary`, `radiate`, `pso-rs`, `sefar`,
`optlib`, `qopt`, `neat-rs`, `rustneat`, `crfmnes`, `fcmaes-core`,
`metaheuristics-nature`, `kurobako`, `optimizer`, `genetic-rs`, `nsga_rs`,
`optirustic` (all MIT); `genetic_algorithms`, `egobox`, `oxicuda-evol`,
`scirs2-optimize` (Apache-2.0); `basin`, `blackopt`, `genoxide`, `argmin`,
`cmaes`, `haru_cmaes`, `differential-evolution`, `ys_differential_evolution`,
`frostfire`, `mincost`, `ganesh`, `rlevo`, `ecrs`, `rsgenetic`,
`genetic-algorithm`, `genevo` (MIT OR Apache-2.0 dual). (`argmin` sits here
as a crate and stays available as navette's optional opt-argmin backend —
retired only as a direct dependency of this stack, review C4.)

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
GPL triggers on conveyance, not in-house benchmarking — local S10 use is
unrestricted; distribute only our code + scripts with mahf-coco as a
documented external requirement. Review R5 hardening: dependency-license
checks alone are not the boundary — §4 rule 5 adds an explicit CI rule for
the CLI-only invariant.

**Bucket D — do not copy (unknown terms):** `genetic` (license string literally
"AML" — most likely the author's 2015-era shorthand for Apache/MIT dual
licensing, but the upstream repo (Kerosene2000/genetic-rs) is deleted and no
LICENSE text survives, so it is unverifiable and therefore unusable; the crate
itself is a Dec-2015 toy, "a little lib to use genetic algorithm", sustained
by a decade of short-name drive-by downloads), `easy_ga` + `simulated_annealing`
(non-standard license strings — read the actual LICENSE text before touching;
both are anyway trivial to reimplement cleanly). `push-trait` (CC0 — free, but
irrelevant to EC). **Run-only rule (review C5):** `easy_ga` may run locally as
a prototype baseline (E09 v2 §1) — never distribute results embedding it,
never vendor it, never let it enter the workspace dependency tree.

**navette (LGPL-3.0-or-later):** A5 decided 2026-11-14 — `navette-ga` itself
is LGPL-3.0-or-later, the same license as navette. Consume navette as a
**cargo dependency** and never copy its files into the crate: the dependency-
only rule is architectural (one owner of the engine), no longer a licensing
obstacle.

## 2. Depend vs copy vs reimplement (per subsystem)

| Subsystem | Disposition | Source & licence cost |
|---|---|---|
| Bounded LM / refiners | **Depend**: `basin` (MIT/Apache) via navette `opt-basin` | Zero copied lines |
| Pareto fronts | **Depend**: `moors` (MIT) | Zero copied lines |
| Surrogate (Gpx/Egor) | **Depend**: `egobox-gp`/`egobox-ego` (Apache-2.0; not the umbrella `egobox` binding crate — E00 erratum 2026-11-10) | Zero copied lines |
| Study layer | **Depend**: `blackopt`/`optimizer` (MIT/Apache, MIT) | Zero copied lines |
| Proof harness | **Depend/CLI**: `kurobako` (MIT) | Zero copied lines |
| Second pillar (BO/MOEA-D) | **Depend**: `genoxide` (MIT/Apache, pinned) | Zero copied lines |
| VL chromosome + Insertion/Deletion + Pad crossover + length penalty | **Copy (gated, §0)** from `genetic_algorithms` (Apache-2.0) | ~hundreds of lines; keep Apache notice + attribution; this is THE copy — no other VL source exists (V11) |
| Island-GA driver (topology, migration, heterogeneous islands) | **Copy-skeleton (gated, §0)** from `genetic_algorithms` IslandGa, rewired to navette A3 batch eval. **Policy:** keep the driver, set E09 v2 §2.8.1 defaults (Ring, k=10, one best migrant, best-replace-worst-if-better with §7 legality repair); expose knobs for I1. `IslandGa`'s `BestReplaceWorst` replaces unconditionally, so the copied skeleton adds the fitness check. | Apache-2.0, attribute; mutate freely |
| Engine composability backbone (Simulation loop, Termination OR/AND combinators, Operator/Population/Genotype traits, GenomeBuilder) | **Copy-skeleton (gated, §0)** from `genevo` 0.7 (MIT/Apache) as the `ga::engines` assembly backbone, with mahf's builder-DSL ergonomics as the API reference; shared with E09 v2 Phase 3 | Dormancy irrelevant for copies (pin 0.7); attribute; this is the Rust-idiomatic answer to "try seven hybrids in a week" without GPL contact |
| QD archive base (MAP-Elites/CVT/speciation) | **Fork** `symbios-genetics` (MIT) → add MOME per-bin fronts (F1) in-fork | MIT; offer F1 upstream as PR, carry fork until merged; shrink per release (R4) |
| Age layering (ALPS pattern) | **Copy** `oxigen` age-unfitness module (MPL-2.0, Bucket B file discipline) *or* reimplement (~60 lines) — reimplementing dodges even MPL notices; recommended | Either clean |
| CMA-ES / JADE / SHADE cores | **Copy** from `genoxide` or `cmaes` (MIT/Apache) if the second-pillar dep proves too heavy for the inner loop | Attribute; benchmark before forking |
| Baseline engines (RGA/DE/PSO/Firefly) | **Copy** `metaheuristics-nature` (MIT) Solver-shaped baselines | Small, self-contained |
| Tribes/CTCM logic | **Reimplement** from D11 (paper, not code — no crate owns it) | Ours |
| Decoder / pool / grammar (G2/G3) | **Reimplement** (v4 work item 5; A4 moves legality into navette) | Ours |
| Needle-material boundary rules (`InsertRules`, outer/inner edge lists, `NeedleRules`) | **Depend/reuse**: navette `NeedleRules` + `MaterialPool::with_insert_rules` + `DesignStack::set_edge_materials`; bridge/decoder glue only | Zero copied lines; labels resolved by `StackTemplate` |
| SAIL/POME/DQD/LLM glue (G5–G7, F2–F4) | **Reimplement** (no crate composes these) | Ours |
| `mahf-coco` comparisons | **External harness only** (Bucket C boundary) — but see below: this is a genuine *need*, not just a quarantine. Three distinct needs: (1) **mahf-coco**: COCO/BBOB bindings as the S10 publication-proof runner for the H0–H7 comparison + V9/V21 ablations — GPL triggers on conveyance, not on in-house benchmarking, so local use is unrestricted; distribute only our code + scripts with mahf-coco as a documented external requirement (CLI/process boundary is belt-and-braces, plus the §4 rule-5 CI invariant). (2) **mahf core**: builder-DSL composability + `boundary::Saturation` + replacement schemes + state-tracking/logging interfaces as *design patterns to mirror* (reimplement, never copy) — the architectural template for `ga::engines`. (3) **mahf-bmf**: nothing (A11/kurobako/GA-benches cover it; only exact function parity would justify a look). **mahf-tsplib**: nothing (TSP-only). | No contact |

**Round-2 policy note (E15/E16/G11/R11).** The island row is not just a
driver copy. E09 v2 §2.8.1 fixes the topology, interval, rate, selection,
replacement and parent-pool scope; I1 is an experiment, not a default. The
copy must retain the policy knobs and add the fitter-than-worst check that
`IslandGa` omits.

## 3. Proposed layout (`navette-ga`, one crate)

```
navette-ga/                        # one package, LGPL-3.0-or-later (same as navette)
├── src/decoder.rs                 # G2+G3: VL keys, bounds[m], grammar, pins, pool (fresh)
├── src/engines.rs                 # gated copies: VL chromosome+operators (genetic_algorithms, Apache),
│                                  # island driver skeleton (Apache), genevo backbone (MIT/Apache),
│                                  # age layering (fresh reimpl), baselines (metaheuristics-nature, MIT)
│                                  # — NOTICE file per source; lands in Phase 3 on measured need (§0)
├── src/qd.rs                      # fork of symbios-genetics (MIT) + F1 MOME + F2 descriptors
├── src/bridge.rs                  # G1/G4: fitness bridge + memetic operator over A1–A3 (fresh)
├── src/study.rs                   # G7: thin wrappers over blackopt/optimizer deps (fresh)
├── src/bench.rs                   # G8/G9: kurobako problem + perf harness (feature `bench`)
├── src/main.rs                    # Recipe A/B binaries
├── CREDIT.md + per-file headers   # Bucket A/B attribution; MPL file list
└── Cargo.toml                     # one package; subsystem dependencies stay normal deps
```

Why this shape: the **copied surface is minimal** (VL machinery that exists
nowhere else + island skeleton + genevo backbone + QD fork base, all gated);
everything with a clean API stays a dependency (no fork drift); copyleft never
crosses a link boundary (GPL via CLI + CI invariant; LGPL stays a dependency
under the same license); unknown-license crates contribute nothing. Estimated
copied volume: <2 kSLoC against ~1,500 lines of fresh glue (E09 v2 §3) — the
new crate is overwhelmingly ours, with provenance recorded per file.

## 4. Maintenance rules (avoiding fork rot)

1. Copy at pinned versions; record `source = crate@version, files:` in
   CREDIT.md so updates are diffable.
2. Prefer upstream PRs for anything generally useful (F1 MOME → symbios;
   A1–A6 → navette, trivially, since owner = you) and shrink the fork with
   every release — fork-shrink has an owner and a per-release cadence (R4),
   not good intentions.
3. Never cherry-pick from Bucket C/D; never vendor navette files (dep only).
4. Re-run `cargo deny`-style license checks in CI so a transitive dep can't
   smuggle GPL/AGPL into the workspace later (`scirs2-optimize`-scale
   mono-repos and `cool-japan`-style orgs are the shape of that risk).
5. CI enforces the mahf-coco CLI-only invariant explicitly (no link, no path,
   no feature unification across the process boundary — R5): dependency-license
   checks alone are not the boundary.

*Sources: crates.io `license` fields (two meta batches, 2026-11-09); E00 (code
provenance per crate); E09 v2 (glue/fresh inventories G1–G9/F1–F4, A1–A6,
§1.1 gate); v4 §§5–9/13 (what the copied modules must do) + Appendices A/B;
independent technical review (C3/C4/C5/R3/R4/R5 — each rule above traces to an
ID). SPDX note: `MIT/Apache-2.0` (slash) vs `MIT OR Apache-2.0` both mean
dual-licensed choice; `AML` and `non-standard` are not valid SPDX — treated as
all-rights-reserved until the LICENSE file says otherwise.*
