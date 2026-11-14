# E07 — Two-way gap analysis: literature↔crates, and the strategies that close them

*Analysis date 2026-11-09. Direction A: crate features lacking primary
literature. Direction B: literature topics lacking any crate. Closes with the
new strategies the second hunt surfaced (G11–G15).*

## A. Crate features: literature status

| Crate feature | Literature now | Still missing |
|---|---|---|
| MAP-Elites, NSLC, CVT archives | G01, G05, G10 (primaries) | — closed |
| SAIL / BOP-Elites / EJIE | G06, G07 (primaries) | — closed |
| MOME, POME, SMILES-evolution | G08, G09 (primaries) | — closed |
| Unsupervised QD, deceptiveness | **G12 (primary, 2025)** | AURORA original (GECCO gated; ideas covered) |
| MOQD unbounded/unstructured | **G13 (primary, 2025)** | — closed |
| Differentiable QD, CMA-MAE/D | **G11 DQD (primary)**; G15 covers CMA-MAE | CMA-ME original (GECCO gated; superseded by CMA-MAE/DQD) |
| QD + diffusion priors | **G14 (primary, 2026)** | — closed |
| High-dim measures, discount search | **G15 (primary, ICLR 2026)** | — closed |
| Age layers / ALPS (oxigen, E06 §9) | none — crate docs only | **Hornby 2006 (ACM gated)** — top literature gap |
| TPE / BOHB / Hyperband (blackopt, optimizer) | crate docs only (NIPS PDF fetch failed) | **Bergstra et al. 2011; Falkner et al. 2018** |
| NSGA-III / MOEA/D / SPEA2 / SMS-EMOA primaries | F07 book-level; crate docs | IEEE-gated originals (author copies on researchers' sites — hand-fetch) |
| JADE / SHADE / L-SHADE primaries | A03 review-level | gated originals; acceptable via review |
| GSK / EO primaries (sefar) | A02/F02 mention-level | weak — accept (niche algorithms) |
| fANOVA primary (blackopt/optimizer) | A04/G02 adjacent | weak — accept (methods paper, not coating-relevant) |
| IPOP/BIPOP restarts | A06 tutorial-level | acceptable |

## B. Literature topics with no crate (build list)

1. **MOME per-bin fronts** (G09) — no implementation; feature proposal for
   `symbios-genetics` stands (E05).
2. **POME offline QD** (G08) — no implementation; script over `symbios` +
   `egobox` (E05).
3. **Unsupervised descriptors** (G12) — no crate learns descriptor spaces;
   implement: autoencoder over stack genomes → CVT archive (G09 pattern).
4. **DQD through a differentiable simulator** (G11) — TMM admits analytic
   adjoints, so gradient-based QD (CMA-MAEGA/DQD loop) is directly
   implementable for coatings; no crate offers the loop (`genoxide` has
   gradients + QD-adjacent pieces, not the loop).
5. **MOQD for unbounded spaces** (G13) — grids and fixed CVT assume bounded
   measures; variable-length stacks are unbounded → needs growing/unstructured
   archives (NSLC-style) + per-bin fronts; no crate.
6. **Discount-model search** (G15) — no crate; relevant when descriptors are
   high-dimensional (e.g. full spectra as behaviour).
7. **QD-guided diffusion samplers** (G14) — no crate; the E05.6 engine with a
   paper behind it now.
8. Covered pairs (no action): pruners→`optimizer`, CVT-MAP-Elites→`symbios`,
   NSLC/novelty→`symbios`, COCO→`mahf-coco`, EJIE→hand-wired
   `genoxide`+`symbios`, age methods→`oxigen` (docs-only, lit gap above).

## C. New strategies distilled (what to actually do differently)

1. **Stop hand-picking descriptor axes (G12).** Depth×thickness maps assume we
   know the right diversity axes; on deceptive landscapes (C01) the axes are
   the trap. Learn descriptors from elites (autoencoder/contrastive, refit as
   the archive grows) and illuminate *that* space; keep one hand-built map
   only as the human-readable dashboard.
2. **Unbind the archive (G13).** Fixed grids/CVT break under variable-length
   genomes. Use growing archives: unstructured elites + novelty admission,
   CVT re-seeded from the learned manifold each epoch (G09), local fronts per
   cell (MOME) so growth never collapses trade-offs.
3. **Differentiate the simulator (G11).** TMM is analytic — push adjoints
   through it and run gradient-based QD for archive fill, ES-based QD for
   escape; hybridise by cell (gradient where smooth, random where kinked).
   This is the fastest plausible route to illuminated 80-layer maps.
4. **Guide the generator, don't just sample it (G14 + E05.6).** QD guidance
   inside diffusion sampling over stack strings yields diverse *portfolios*,
   not one guess; distil motifs (B10-style) from the portfolio, not from GA
   logs.
5. **Treat high-dimensional behaviour as normal (G15).** Full R(λ,θ) spectra
   are legitimate descriptors; discount-model search keeps them tractable.
   Beats compressing behaviour to 2 hand axes before the map is built.

## Sources
Graph searches 2026-11-09 (12 queries, §A/B tables); G11 (NeurIPS 2021, 29pp),
G12 (11pp), G13 (13pp), G14 (29pp), G15 (ICLR 2026, 39pp); arXiv author/title
searches (Cully/Fontaine/AURORA/CMA-ME — originals GECCO-gated, ideas covered
by G11/G12); NIPS TPE PDF fetch failed (docs-only, §A).
