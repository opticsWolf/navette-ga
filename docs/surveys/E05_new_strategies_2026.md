# E05 — New strategies from recent research (QD + surrogate + generative), mapped to thin-film synthesis

*Survey date 2026-11-09. Sources: G01/G05–G10 ingested this round (BOP-Elites,
SAIL, POME offline QD, CVT-MOME molecules, crystal QD) + 2026 arXiv abstracts
(diffusion priors, LLM-guided evolution, PCFBench). Each item: mechanism,
thin-film mapping, crate mapping, and what beats the current PAPER pipeline.*

## 1. SAIL: surrogate-assisted illumination (G07, Gaier–Asteroth–Mouret 2017)

Loop: fit GP surrogate on evaluated stacks → run MAP-Elites *on the
acquisition function* (acquisition map) → sample from it, evaluate with the
real solver → refit → final prediction map of elites per cell. UCB balances
explore/exploit; GPs work from few samples. TMM is cheap, but full-wave
RCWA/FEM/FDTD for patterned or EUV stacks is not (B09 says so explicitly) —
SAIL is what makes illumination affordable the day the evaluator gets
expensive. Beats plain MAP-Elites on sample efficiency by an order of
magnitude in their robot-arm demo. Crates: `symbios-genetics` (archive) +
`egobox`/`genoxide` (GP + UCB/EI) — no single crate does both yet.

## 2. BOP-Elites: joint improvement over fitness AND descriptors (G06, 2023)

Expected Joint Improvement of Elites (EJIE): model fitness *and* behavioural
descriptors with GPs; the acquisition rewards points that improve the elite
of the cell they are *predicted* to land in; mispredicted-descriptor points
score 0 (or lower bound) — a principled way to handle learned/black-box
descriptors. Their Table I is also the decision guide the PAPER lacked: QD
vs MOO vs multimodal vs multi-task vs constrained optimisation are
different problems (each archive cell = same objective, different
descriptor-constraint). For coatings: descriptors like "dominant motif"
(DBR presence, B10) or "field-profile class" are black-box — BOP-Elites is
the only method that models them. Crates: `genoxide` BO + `symbios` archive,
wired by hand.

## 3. MOME + CVT over learned embeddings (G09, Mashak–Schrum 2026)

The closest existing work to coating synthesis, on NLO molecules: MOME keeps
a *local Pareto front per archive bin* (not one elite); CVT replaces the
uniform grid with Voronoi cells seeded by k-means on a learned manifold —
ChemBERTa-2 (77M params, 10M PubChem SMILES) → UMAP-10 — so niches sit where
molecules actually cluster, not in infeasible corners; result: higher global
hypervolume *and* higher QD-score than grid MOME, NSGA-II, μ+λ, and SA on the
same 4-objective problem. Genome = SMILES string, 7 structure mutations, **no
crossover** (validity beats recombination). Read-across: stacks-as-strings
(D04 tokens), B09 dispersion embeddings as the measure space, TMM/HF as the
expensive evaluator, insertion/deletion as the mutation dictionary. This is
the single strongest template for the 80-layer filter recipe. Crate gap: no
Rust crate implements MOME — `symbios-genetics` (archive + NSGA-II) is the
natural host; propose per-bin fronts as its next feature.

## 4. POME: offline uncertainty-aware QD (G08, 2026)

Prior-guided Offline MAP-Elites for sequence design (nucleic acids/proteins):
learn from a fixed log of past evaluations (no new simulations), pessimise
with ensembles where the surrogate is uncertain, keep diversity so at least
one candidate survives wet-lab validation. Coating analogue: every lab and
every paper run leaves logs of (stack → spectra); POME turns that exhaust
into a diverse proposal set *with zero new TMM calls* — then validate the
shortlist exactly (spot-check discipline, C01). Uncertainty handling is the
piece the PAPER's archive-mining (B10) missed. Crates: implement over
`symbios` + `egobox` GPs; nothing off-the-shelf.

## 5. Property illumination for materials (G10, crystal QD 2024)

MAP-Elites over (property, property) maps for crystal structures. Transfer:
precompute illumination maps *of the material pool itself* — merit potential
vs (index contrast, dispersion slope, absorption edge) — before any stack
search, and seed island populations from bright cells. Turns material choice
(Ch.6) from a menu into a map. Cheap (B09 embeddings + TMM sweeps), no new
crate needed.

## 6. Diffusion priors + rule discovery (arXiv 2609.28401, 2026, abstract)

Label-free diffusion model learns the distribution of valid high-performing
freeform photonic designs; rules are then *extracted* from the generator.
This is B10 (interpretable design) with a modern engine: for stacks, train a
diffusion/sequence prior on (material, thickness) strings (D04's token view
is the on-ramp), sample conditionally on targets, then distil motifs. Pairs
with strategy 4 (offline data) and strategy 1 (surrogate scoring).

## 7. LLM in the loop + benchmark discipline (2026 abstracts)

LLM-guided mutation operators (medical pipelines 2606.07342; A08's LLM+EC
review in-graph) transfer directly: an LLM proposing *physically named* moves
("insert a 2H needle at the field antinode") beats Gaussian thickness noise
for escape moves. PCFBench (2610.32485) is the warning label: benchmark
whether the model reasons about physics or pattern-matches. Methodological
takeaway: register the Ch.15 seven-hybrid comparison under `kurobako` or
`mahf-coco` (E04) so the next group can rerun it.

## 8. Prioritised build order (what changes in the PAPER pipeline)

1. **Now, no new code:** G10-style material-pool illumination + G06's
   QD-vs-MOO decision rule (§12/13 text patch).
2. **Glue code (weeks):** SAIL loop (`symbios` + `egobox`) for RCWA-grade
   evaluations; POME pass over archived runs.
3. **One feature (month):** per-bin Pareto fronts in `symbios-genetics`
   (= MOME, G09 template) — the 80-layer recipe's core.
4. **Research (quarter):** diffusion prior over stack strings + rule
   distillation (G09-meets-2609.28401); LLM named-move operators with
   PCFBench-style ablations.

## Sources
G01, G05, G06 (pp.3–6), G07 (pp.2–5), G08 (title/abstract/chunks), G09
(full text), G10 + arXiv ids 2609.28401, 2608.18333, 2610.32485, 2611.00716,
2604.05622, 2307.09326, 1702.03713, 2403.03511, 2610.08367. Tool note:
page-range extraction on multi-author PDFs trips a metadata validator
(`author` list vs string); full-text extract works — author lists need the
same subset care as §5–7 of the ikat report.
