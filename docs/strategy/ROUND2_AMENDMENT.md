# Round-2 amendment — island policy, parent pool, and I1

**Status:** adopted 2026-11-14. This is the provenance record for incorporating
the second round of the consolidated technical review
(`Technical_Review_GA_ThinFilm_Strategy_Set.md`, round-2 additions dated
2026-11-11) into PAPER v4, E09 v2, and E10 v2.

## Why this amendment exists

The current documents responded to **round 1** of the review: PAPER v4, E09 v2,
and `docs/README.md` all said **19 findings / 10 gaps / 10 risks**, and E09 v2
closed at **R1–R10**. Round 2 adds exactly four register items:

| Item | Type | Subject |
|---|---|---|
| **E15** | finding (minor) | Migration policy underspecified for the declared default parallel model |
| **E16** | finding (minor) | Multi-parent crossover parent-pool scope (within-island vs post-migration) never stated |
| **G11** | gap | No cross-island recombination operator described or foreclosed |
| **R11** | risk (low–medium) | Unspecified migration policy can homogenise or starve heterogeneous islands |

Round 2 also records **S11** (multi-population coverage is broad and well
witnessed), which is a strength, not a repair. The consolidated total is
therefore **21 findings / 11 gaps / 11 risks / 11 strengths**.

## Decisions

### E15/R11 — migration policy minimum

`k` alone is not a policy. Recipe B now fixes:

| Knob | Decision | Basis |
|---|---|---|
| Topology | Ring by default, for heterogeneous and homogeneous islands alike; extended rings are the first connectivity upgrade; fully connected is an ablation, not a default | D01 finds extended rings strongest and fully connected among the weakest for DE; D13 finds ring/torus can beat a single receiver at low noise; `IslandGa` defaults to Ring |
| Interval | `k = 10` generations; ablate {5, 10, 20, 30} | `IslandGa` default 10; its docs give 5–20 as the typical range; D06's period 30 is the slow-flow end |
| Rate and selection | One best individual per island per event (`migration_count = 1`), capped at 5% of the population; D06's 5%-of-best setting is the high-rate ablation | `IslandGa` default 1; its docs say 1–2 is sufficient and more homogenises; D01 describes elitist migration; D06 uses 5% best |
| Replacement | Overwrite the destination's worst individual only if the migrant is better; then run the §7 legality check/repair; never overwrite the destination's best | D01: incoming individuals replace the worst "provided that they were better"; D13: the best-received solution replaces the current one "if fitter". `IslandGa`'s `BestReplaceWorst` replaces unconditionally, so the wrapper/copy adds the check |
| Timing | After evaluation of generation `k`, before the next breeding cycle | `IslandGa` migration cycle: evolve → select migrants → send → replace → continue |
| Parent pool | Migrants join the destination breeding pool; recombination is within-island by default | Design decision that preserves the heterogeneous-island rationale; E16 |

The policy is tagged **heuristic/design decision**, not a literature constant.
R11 is owned in E09 v2 §6 and validated against a single-island ablation under
the fixed-seed protocol.

### E16 — parent-pool scope

Default: **within-island, post-migration**. Migrants enter the destination
pool, then crossover samples from that pool. Cross-island multi-parent
crossover is **not** the default. This is a specification choice, not a claim
that local mating is superior.

### G11 — I1 cross-island multi-parent experiment

D13's single-receiver model is the citable cross-island design: at a migration
event it chooses `k` parents from different islands and applies majority-vote
crossover; the many-receiver variant does the same at several receivers. I1
registers that operator as an experiment:

- Trigger at a small fixed fraction of migration events (default 10%, ablated).
- Draw `k` parents from the post-migration pools of at least two islands.
- Compare against (a) migrate-then-mate-locally, (b) within-island multi-parent
  crossover, and (c) a panmictic single population at equal evaluation budget.
- Primary readout: merit and depth at fixed budget.
- Secondary readout: pairwise island distance / material-mix entropy
  (homogenisation) and migration cost.
- I1 is an island-policy experiment alongside D02's adaptive-island-count item,
  **not** an H0–H7 arm.

## Database evidence (queried 2026-11-14)

Read-only LadybugDB queries against `research/kb.db` (`okfgraph.toml`
`database.db_path = "research/kb.db"`) verified the source claims:

| Concept ID | What it supports |
|---|---|
| `@pdf-36c8eb236d15/D01_rucinski_island_topology_2010` | Names the Island Model parameter set: number of islands, migration topology, **migration rate**, **migration frequency**, and the remaining migration algorithm. Its experiments use elitist migration: "from every island the best individuals were selected for migration, and the incoming individuals replaced the worst ones … provided that they were better." Its conclusion ranks extended rings (`Ring+1+2`, `Ring+1+2+3`) as the safest DE choice and places Fully Connected among the worst. |
| `@pdf-f1187a49e73f/D06_ferrucci_mapreduce_pga_2017` | Concrete setting: "migration period to 30 applying a ring topology exchange, whereas the number of migrants to 5% of the best individuals per island." |
| `@pdf-b8c3cfc3196b/D13_aboutaib_noise_island_2023` | Single-receiver island model with migration rate τ and parent pool size k; at migration it chooses `I ⊆ {1,…,μ}` with `|I| = k` and applies k-parent crossover to parents from different islands. The many-receiver variant samples k solutions per receiver. Ring/torus can outperform the single receiver at low noise; received solutions replace the current one only if fitter. |
| `@pdf-a85a7aa11989/D02_laessig_adaptive_population_2011` | Island model with migration and a complete topology; adaptive island counts are explicitly open territory (the paper's V27 experiment). |
| `@pdf-26226a37d78a/D11_chen_tribes_ctcm_2024` | Tribes/CTCM uses migration and inter-tribe competition; supports the migration-as-arbitrator framing, not a specific rate/replacement policy. |

The dependency source `vendor/sources/genetic_algorithms/src/engines/island/`
confirms the implementation defaults used above: `IslandConfiguration` defaults
to `num_islands = 4`, `migration_interval = 10`, `migration_count = 1`,
`topology = Ring`, `migration_policy = BestReplaceWorst`; its module docs say
ring is the best starting point, 5–20 generations is typical, 1–2 migrants is
sufficient, and more migrants homogenise. `replace_worst` in
`migration.rs` replaces unconditionally, which is why the amendment adds the
fitter-than-worst check at the wrapper/copy boundary.

## Implementation notes

- **Phase 2 (`genetic_algorithms` dependency):** use the `IslandGa` defaults
  (Ring, k=10, count=1, BestReplaceWorst), but wrap migration with the
  fitter-than-worst check and the §7 legality repair; expose the knobs for the
  E15 ablation.
- **Phase 3 (gated copy):** the copied island skeleton carries the check
  natively; keep the same default values so published Phase-2 runs remain
  comparable.
- **I1:** implement only after the baseline policy and H0 control are measured;
  report it separately from H0–H7.
- **Fully connected:** keep as an ablation. D01's edge-count growth and its
  weak DE ranking argue against making it the default.

## Verification log

- `cargo metadata` / lockfile checks are unrelated to this documentation-only
  change; no rebuild is required.
- DB queried read-only with LadybugDB; `okf search` needs
  `ORT_DYLIB_PATH` pointing at a compatible ONNX Runtime and
  `--precision fp16` because the graph is pinned to fp16. The search path was
  verified with the D01/D13 query before the policy text was written.
- The full review Markdown remains an external artefact; only its round-1 PDF
  and this amendment record are part of the repository history.

## Open questions

1. Does I1 beat migrate-then-mate-locally on **both** merit/depth and
   homogenisation at equal budget? If it only buys merit by collapsing island
   diversity, keep it out of the default.
2. Is the 5% cap on migration rate enough for large populations, or does the
   one-migrant default starve islands above P ≈ 200? The E15 ablation should
   include 1, 2, and 5% rates.
3. Does extended-ring connectivity (`Ring+1+2`) justify a `Custom` topology in
   Phase 3, or is plain Ring sufficient for the heterogeneous-island regime?
