> **Repository note (2026-11-14):** This index and the markdown sources now
> live under `docs/`; the PDFs themselves are not tracked. They live in
> `research/papers/` (git-ignored), matching the filenames below. The Rust
> implementation is the single `navette-ga` package at the repository root
> (`src/`); see the root `README.md` and `docs/integration.md` for how the
> code is built and adopted into Navette.

# Web research: evolutionary / swarm / genetic optimization — and thin-film optical design

**Collected:** 2026-09-25 · **Method:** gossamer CLI (OpenAlex) + arXiv API
direct, PDFs verified (`%PDF` magic, sizes below).
**Tracks:** **(A)** general state of research & latest developments ·
**(B)** thin-film optical simulation/design specifically.

---

## A — General: genetic, swarm, evolutionary & related optimization

| File | Paper (authors, venue, year) | Why it is here |
|---|---|---|
| `A01_survey_pioneering_metaheuristics_2019_2024.pdf` (3.5 MB) | *A survey on pioneering metaheuristic algorithms between 2019 and 2024* (2024/25) | The "state of research" anchor: what is genuinely new vs. repackaged metaphor-methods since 2019. |
| `A02_pso_survey_hybridization_2018.pdf` (3.7 MB) | *Particle Swarm Optimization: a survey of historical and recent developments with hybridization perspectives* (2018) | PSO family tree + why modern PSO is almost always hybrid (local search, DE operators). |
| `A03_multimodal_differential_evolution_review_2025.pdf` (0.7 MB) | *Advancements in Multimodal Differential Evolution: a comprehensive review and future perspectives* (2025) | DE state of the art; niching/multimodality matters because coating merit functions are riddled with local minima. |
| `A04_bayesian_optimization_tutorial_2018.pdf` (1.2 MB) | Frazier — *A Tutorial on Bayesian Optimization* (2018) | The canonical BO reference; sample-efficient alternative when each merit evaluation is expensive. |
| `A05_information_theoretic_bo_survey_2025.pdf` (1.6 MB) | *Information-theoretic Bayesian Optimization: survey and tutorial* (2025) | Latest BO developments (entropy-search family) for expensive black-box objectives. |
| `A06_cma_es_tutorial_2016.pdf` (0.9 MB) | Hansen — *The CMA Evolution Strategy: a tutorial* (2016) | CMA-ES is the default serious baseline for continuous black-box optimization; read before implementing any custom ES. |
| `A07_automated_design_of_metaheuristics_survey_2023.pdf` (1.3 MB) | *Automated Design of Metaheuristic Algorithms: a survey* (2023) | Algorithm configuration/selection & per-instance tuning — the "latest development" direction most relevant to shipping an optimizer inside a tool. |
| `A08_llm_evolutionary_computation_review_2025.pdf` (0.7 MB) | *Large Language Models and Evolutionary Computation: a critical review of bidirectional interactions* (2025) | The other 2025 frontier: LLMs generating/mutating heuristics and interpreting runs. |
| `A09_biased_random_key_genetic_algorithms_review_2023.pdf` (1.9 MB) | *Biased Random-Key Genetic Algorithms: a review* (2023) | BRKGA = the GA variant that actually wins engineering benchmarks; problem-independent, easy to implement well. |
| `A10_metaheuristics_variable_size_mixed_optimization_2024.pdf` (0.5 MB) | *Metaheuristics for (Variable-Size) Mixed Optimization Problems: a unified taxonomy and survey* (2024) | Directly maps to coating design: **variable layer count = variable-size**, thicknesses continuous, materials discrete. |
| `A11_review_315_benchmark_functions_2024.pdf` (26 MB) | *A review of 315 benchmark and test functions for machine-learning optimization algorithms* (2024) | Benchmark suite reference for validating any optimizer before trusting it on coatings. |

## B — Thin-film optical simulation/design with these methods

| File | Paper (authors, venue, year) | Why it is here |
|---|---|---|
| `B01_prism_multilayer_thinfilm_inverse_design_2026.pdf` (2.4 MB) | *PRISM: Position-encoded Regressive Inverse Spectral Model for Multilayer Thin-Film Design* (2026) | Newest thin-film inverse-design direction (learned spectral→stack mapping). |
| `B02_automated_optical_multilayer_design_deep_rl_2020.pdf` (1.6 MB) | *Automated Optical Multi-layer Design via Deep Reinforcement Learning* (2020) | RL as sequential stack builder — the modern counterpart to needle-type constructive methods. |
| `B03_multilayer_thinfilm_deep_q_learning_2020.pdf` (2.4 MB) | Jiang, Yoshie & Chen — *Multilayer optical thin film design with deep Q learning*, Sci Rep (2020) | Deep Q-learning for multilayer design; open access. |
| `B04_quantum_inspired_ga_planar_multilayer_photonic_2024.pdf` (1.8 MB) | *Quantum-Inspired Genetic Algorithm for Designing Planar Multilayer Photonic Structures* (2024) | GA-family method applied straight to multilayer stacks. |
| `B05_multiobjective_broadband_euv_multilayers_2020.pdf` (6.9 MB) | *Multiobjective optimization in design of broadband extreme ultraviolet multilayers* (2020) | Real coating-design case where the merit is inherently multi-objective (reflectivity bands vs. constraints). |
| `B06_gradient_informed_bo_nanophotonics_inverse_design_2026.pdf` (1.5 MB) | *Gradient-Informed Bayesian and Interior Point Optimization for Efficient Inverse Design in Nanophotonics* (2026) | BO + gradients + interior-point for photonic inverse design — the sample-efficient end of the spectrum. |
| `B07_plasmonic_light_trapping_thinfilm_solar_2013.pdf` (0.7 MB) | Pala, Liu, Barnard, Askarov, Garnett et al. — *Optimization of non-periodic plasmonic light-trapping layers for thin-film solar cells*, Nat Commun (2013) | Classic: global optimization of a thin-film stack structure with a physics merit function. |
| `B08_optogpt_foundation_model_multilayer_inverse_design_2023.pdf` (2.5 MB) | Ma, Wang & Guo — *OptoGPT: A Foundation Model for Inverse Design in Optical Multilayer Thin Film Structures* (2023) | Foundation-model direction for multilayer inverse design. |
| `B09_reinforcement_learning_optical_thinfilm_design_2021.pdf` (1.9 MB) | Jiang & Yoshie — *A Reinforcement Learning Method for Optical Thin-Film Design* (2021) | Follow-up to B03, same group. |
| `B10_interpretable_inverse_design_multilayer_2025.pdf` (1.1 MB) | *Interpretable inverse design of optical multilayer thin films* (2025) | Interpretability angle on learned inverse design. |
| `B11_ultrabroadband_absorber_multilayer_optimization_2020.pdf` (1.6 MB) | *Optimized Multilayer Structures With Ultrabroadband Near-Perfect Absorption*, IEEE Photonics J (2020, LSU group author copy) | Optimization of multilayer stacks for absorption targets. |

---

## D — Second round (2026-11-08): island/tribe population models, GA-for-films, inverse-design reviews

**Method:** gossamer MCP (OpenAlex scholarly + web search), PDFs downloaded
with `%PDF` verification, ingested into `kb.db` (mordant 0.10.1).

| File | Paper | Why it is here |
|---|---|---|
| `D01_rucinski_island_topology_2010.pdf` (0.7 MB) | Ruciński, Izzo & Biscani — *On the impact of the migration topology on the Island Model*, Parallel Computing 36 (2010), 128 cited | Island-model topology reference (ring vs. fully-connected vs. random migration). OA: arXiv:1004.4541. |
| `D02_laessig_adaptive_population_2011.pdf` (0.5 MB) | Lässig & Sudholt — *Adaptive population models for offspring populations and parallel EAs* (2011), 79 cited | Theory of adaptive offspring sizing. OA: arXiv:1102.0588. |
| `D03_molesky_inverse_design_nanophotonics_2018.pdf` (5.6 MB) | Molesky, Lin, Piggott, Jin, Vučković & Rodríguez — *Inverse design in nanophotonics*, Nature Photon. (2018), 1902 cited | The canonical inverse-design framing reference. OA: arXiv:1801.06715. |
| `D04_masked_diffusion_multilayer_2026.pdf` (1.1 MB) | Schaible et al. — *Inverse Design of Optical Multilayer Thin Films using Robust Masked Diffusion Models* (2026) | Newest diffusion-model direction for multilayer films. OA: arXiv:2604.01106. |
| `D05_ma_review_multilayer_inverse_design_2025.pdf` (1.8 MB) | Ma, Ma & Guo — *Optical multilayer thin film structure inverse design: from optimization to deep learning*, iScience (2025), 43 cited | Bridge review from classical optimization to DL (OptoGPT group). OA preprint: arXiv:2409.17199. |
| `D06_ferrucci_mapreduce_pga_2017.pdf` (1.3 MB) | Ferrucci, Salza & Sarro — *Using Hadoop MapReduce for Parallel GAs: global vs. grid vs. island* (2017), 49 cited | Head-to-head of the three classic parallel population models. OA: UCL Discovery. |
| `D07_ejigu_ga_thinfilm_2010.pdf` (0.3 MB) | Ejigu & Lacquet — *The use of a genetic algorithm in optical thin film design and optimisation*, S. Afr. J. Sci. 106(7/8) (2010), 4 pp | GA-for-coatings paper. Diamond OA via SAJS, hand-fetched 2026-11-08. |
| `D10_tang_floorplan_2007.pdf` (0.3 MB) | Tang & Lau — *A Parallel Genetic Algorithm for Floorplan Area Optimization* (2007), 19 cited | Compact island-model GA case study. OA: QUT ePrints. |
| `D12_bass_angular_spectrum_preprint_2024.pdf` (2.2 MB) | Bass et al. — *An Angular Spectrum Approach to Inverse Synthesis…* (arXiv:2407.06535, 2024) | Preprint Twin of the paywalled Optica angular-spectrum thin-film paper (B-track-adjacent). OA: arXiv. |
| `D13_aboutaib_noise_island_2023.pdf` (1.5 MB) | Aboutaib & Sutton — *The Influence of Noise on Multi-parent Crossover for an Island Model Genetic Algorithm*, ACM TELO 4(2) (2023), 28 pp | Rigorous runtime analysis: island model + multi-parent crossover under noise (OneMax/additive). Bronze OA via ACM, hand-fetched 2026-11-08. Companion: D17 IEEE CIS report (same line). |
| `D15_adamidis_parallel_ea_review_1998.pdf` (0.1 MB) | Adamidis — *Parallel evolutionary algorithms: a review* (HERCMA 1998) | Same-author replacement for the dead 1994 PGA-bibliography link: early PEA review (global/grid/island era). OA: IHU repository DSpace REST. |

| `D11_chen_tribes_ctcm_2024.pdf` (5.5 MB) | Chen, Li, Khan & Mirjalili — *Competition of tribes and cooperation of members (CTCM)*, Expert Syst. Appl. 265 (2025), 70 cited | Tribe-structured EA paper (21 pp). CC-BY via KU portal, hand-fetched 2026-11-08. |
| `D16_cao_iqga_multilayer_2026.pdf` (3.0 MB) | Cao et al. — *Inverse Design of Multilayer Optical Films Based on an Improved Quantum Genetic Algorithm*, Surface Technology 55(12) (2026) | QGA-family multilayer inverse design: discrete-continuous hybrid encoding, multi-elite guidance, transfer-matrix forward solver. OA: surface-techj.com. Consolation find while hunting Liu tQGA. |
| `D17_aboutaib_ieee_cis_report_2021.pdf` (0.2 MB) | Aboutaib — *Robustness of Parallel EAs to noise* (IEEE CIS Graduate Fellowship report, 2021, 6 pp) | Same research line as the walled Aboutaib & Sutton 2023 island-noise paper (same authors, UMD visit). OA: cis.ieee.org. |

**Could not fetch (paywalled / bot-walled / dead links):** Buscema *GenD* (2004, Wiley 403) ·
Liu et al. *Tensorized Quantum GA for thin-film inverse problems* (2026 — no OA outside Wiley: no preprint, S2 empty, HUST group page links DOI only, RG request-only; see D16 for a same-family OA alternative) · Ejigu & Lacquet *GA in thin film design* (2010, sajs.co.za rejected by
fetch guard — hand-fetched 2026-11-08, see D07) · Hussain *EA for optical constants* (2010, OA link 404) · Levine
*PGA for set partitioning* (1994, osti.gov timeouts) · Rocha et al. *hybrid
PGA for laminates* (2013, ScienceDirect-walled) · Adamidis *PGA bibliography*
(1994, CiteSeerX 404) · Aboutaib & Sutton *noise on multi-parent crossover,
island GA* (2023, ACM paywall).

## E — Rust evolutionary crates for the stack synthesiser (2026-11-08)

**Method:** docs.rs verification + upstream GitHub READMEs (raw), curated into
markdown concepts, ingested into `kb.db`.

| File | Content | Why it is here |
|---|---|---|
| `E00_rust_crates_consolidated.md` | **Consolidated survey (supersedes E01–E04 choice sections; originals retained)**: 55 crates with versions/downloads/repos, per-crate sheets, capability matrix, final choice, paper/lit map; corrections (genetic_algorithms repo URL, genetic-algorithm 0.28 upgrade, nsga_rs 1.4.1, evo-engine GH-only, pymoors PyPI-only) | Single entry point: master verdict table + E00 §4 final choice + §5 paper/lit map. |
| `PAPER_ga_thinfilm_strategies_v4.md` | **v4 review-response revision (current): all 19 independent-review findings repaired — §10 math re-derived vs C00 pp.114–124 (V40; inverted decision rule corrected, (10.2) de-factored, unified numbering), C01 branch-and-bound verified (V41); exit criteria as scan certificates; H0–H7 comparison set + statistical protocol; TMM→S-matrix claim map (App. B); ledger V1–V41; supersedes v3** | Read this, not v1/v3, for the current synthesis. |
| `PAPER_ga_thinfilm_strategies_v3.md` | **v3 paper (52 KB, 18 sections): every paragraph expanded against primary chunk evidence; ledger extended to V39; new DQD/TMM-adjoint upgrade (V32), Lamarckian-deviation flag (V21), adaptive-islands experiment flag (V27); v1/v2 retained** | Superseded by v4 (review repairs); retained for the record. |
| `E09_rust_implementation_strategy_v2.md` | **v2 review-response revision (current): owner-note edit completed; C3 governing split (depend Phases 0–2, gated copy Phase 3); scan-certificate exits + A6 P(z) amendment; H0 control + stats protocol; R1–R10 owned; E12/E6/C4 fixes; supersedes v1** | Build reference. |
| `E09_rust_implementation_strategy.md` | **Rust build plan from PAPER v3: navette 0.8.0 owns S-matrix+needle+bounded-LM (opt-basin inside); E00 picks for GA/MOO/QD/surrogate/study/proof; 9 glue items (~1,500 lines) + 5 fresh builds (MOME/DQD/LLM/POME); A1–A5 navette amendments (owner); phased build with exit criteria** | Superseded by v2; retained for the record. |
| `E10_cherrypick_reassembly_v2.md` | **v2 review-response revision (current): C3 governing split recorded; genevo shared with E09 Phase 3; easy_ga run-only rule; R3–R5 maintenance rules; supersedes v1** | Legal+build reference before lifting any code. |
| `E10_cherrypick_reassembly.md` | **Cherry-pick verdicts: license buckets (A copy-freely / B MPL-file-discipline / C GPL+AGPL isolate / D unknown avoid); depend-vs-copy table; single-crate layout (<2 kSLoC copied vs ~1,500 fresh); fork-rot rules** | Superseded by v2; retained for the record. |
| `E08_optimal_strategies_verified.md` | **Consolidated + expanded: PAPER+E05+E06+E00 → 10 optimal strategies (S1–S10) with literature verdicts (V1–V15 ledger from chunk-level graph searches), two updated recipes, crate assignments, 5-item work list** | Strategy reference: §1 ledger for what is proven vs lore; §3 for the ten strategies; §4 for the recipes. |
| `E01_rust_ea_crates.md` | Survey of 4 core crates (`genetic_algorithms`, `optirustic`, `genetic-algorithm`, `Evo-Engine` + bespoke Rayon loop) + verified details on 5 more (`optirustic`, `metaheuristics-nature`, `radiate`, `easy_ga`, `rlevo`) | Crate landscape for the variable-length island-GA → parsimony penalty → memetic needle-refinement architecture. |
| `PAPER_ga_thinfilm_strategies.md` | **Synthesis paper**: best GA strategies for thin-film design, mined from the full graph — VMVOP framing (A10), 5 encodings, operators, islands/tribes, parsimony, MOGA, memetic needle loop, Rust architecture, 5 mermaid diagrams + stack SVG | The paper: §9 recommended pipeline (Island GA → parsimony → needle/argmin refinement → moors Pareto → C01-style validation). |
| `E03_rust_crates_round2.md` | Round 2: `moors` (preferred MOEA specialist), `ecrs` (beta, watch), `argmin` (memetic-refinement toolbox), `darwin-rs`/`rsgenetic` (do-not-adopt); corrections: `pushgen`/`push-trait` are not EC crates, `goldens` nonexistent | Refines the choice: `moors` over `nsga_rs` for Pareto work, `argmin` (Nelder-Mead/L-BFGS) for the refinement stage. |
| `E02_rust_ea_details.md` | README-level deep dive: 17-engine table + operator catalogue for `genetic_algorithms` v3.0.0, `nsga_rs` rename note, `radiate`/`rlevo`/`metaheuristics-nature`/`easy_ga`/`Evo-Engine` detail sheets, crate choice (§8) | Implementation reference: variable-length chromosomes + length penalty, IslandGa migration, memetic LocalSearchOperator, batch TMM evaluation. |
| `E04_rust_crates_round3.md` | Round 3: paper-vs-crates gap matrix + missed crates — `genoxide` (union crate), `basin` 3.3.0 (supersedes `argmin`), `symbios-genetics` (only QD/MAP-Elites crate), `mahf` (GECCO'23, COCO), `blackopt`/`optimizer` (Optuna-style MOO), `oxigen` (age/niches/caches), `egobox` (EGO), CMA-ES/DE/PSO specialists, `kurobako` bench harness | Deltas: `basin` over `argmin`, `genoxide` as second framework pillar, QD merit-vs-depth map angle, VL+parsimony stay single-sourced. |
| `E05_new_strategies_2026.md` | New strategies from 2017–2026 research: SAIL, BOP-Elites EJIE, MOME+CVT embeddings (G09 template), POME offline QD, material-pool illumination, diffusion priors + rule discovery, LLM moves; prioritised build order | Amends the PAPER pipeline: G09 MOME is the strongest 80-layer template; per-bin fronts are the one feature to build. |
| `E06_constrained_diverse_design.md` | Design guide answering all constraint/population/guidance dimensions: count minimisation, flexible N, pool ladder, pinned ends, global+per-material thickness bounds, order grammars, islands/tribes/ages/species, curriculum + steering, LLM roles + guardrails, assembled architecture | The build blueprint: decoder owns bounds/grammar/pinning; archive is the product; crate assignments for every stage. |
| `E07_gap_analysis_literature_crates.md` | Two-way gap matrix (crate features↔literature) + 5 new strategies from G11–G15: learned descriptors, unbound archives, differentiable TMM-QD, QD-guided diffusion, high-dim spectral descriptors | Closes QD lit gaps (DQD, unsupervised, MOQD-unbounded, QD-diffusion, DMS); top open lit: ALPS/Hornby, TPE/BOHB, MOO primaries; build list: MOME, POME, DQD-loop, growing archives. |

## F — Mirjalili book collection (2026-11-08)

**Method:** shared Google Drive folder on seyedalimirjalili.com (file IDs
scraped via stealth-browser-rendered listing), downloaded directly, ingested
into `kb.db` (mordant 0.10.1).

| File | Book | Notes |
|---|---|---|
| `F01_mirjalili_woa_book.pdf` (47.7 MB) | *The Whale Optimization Algorithm* (688 pp) | WOA monograph. |
| `F02_mirjalili_gwo_book.pdf` (44.0 MB) | *Grey Wolf Optimizer* book (786 pp) | GWO monograph. |
| `F03_mirjalili_evolutionary_ml.pdf` (7.9 MB) | *Evolutionary Machine Learning Techniques* (287 pp) | EA+ML methods. |
| `F04_mirjalili_optimization_ml.pdf` (21.4 MB) | *Optimization Algorithms in Machine Learning* (189 pp) | Added 15.07.2025. |
| `F05_mirjalili_swarm_intelligence.pdf` (10.0 MB) | *Advances in Swarm Intelligence* (416 pp) | Swarm survey. |
| `F06_mirjalili_nature_inspired_optimizers.pdf` (13.3 MB) | *Nature-Inspired Optimizers* (Springer 2020, 245 pp) | Edited volume. |
| `F07_mirjalili_multiobjective_optimization.pdf` (4.9 MB) | *Multi-Objective Optimization using…* (Springer 2020, 66 pp) | Short volume. |
| `F08_mirjalili_ea_nn_book.pdf` (9.5 MB) | *EA + NN* book (164 pp) | Evolutionary neural nets. |
| `F09_mirjalili_clustering.pdf` (6.7 MB) | *Clustering* (253 pp) | Clustering with metaheuristics. |
| `F10_mirjalili_hand_posture.pdf` (12.8 MB) | *Optimisation Algorithms for Hand Posture Estimation* (210 pp) | Applied case. |

## G — Algorithm classics backing the round-3 crates (2026-11-09)

**Method:** crates.io survey (E04) → graph coverage matrix → arXiv/author-hosted OA PDFs for the genuine gaps. QD/MAP-Elites/novelty/NEAT had no primary source in-graph; BO/EGO had tutorials but not the expensive-evaluation framing.

| File | Work | Why it is here |
|---|---|---|
| `G01_map_elites_2015.pdf` | Mouret & Clune — *Illuminating search spaces by mapping elites*, arXiv:1504.04909 (15 pp. preprint) | The QD foundational paper behind `symbios-genetics`: elites per niche cell, not one champion — basis for the merit-vs-depth map angle. |
| `G02_bo_tutorial_2010.pdf` | Brochu, Cora & de Freitas — *A tutorial on Bayesian optimization of expensive cost functions*, arXiv:1012.2599 (49 pp.) | EI/PI machinery behind `egobox`, `genoxide` BO, `blackopt`/`optimizer` TPE/MORBO: surrogate + acquisition for expensive merit evaluations. |
| `G04_neat_2002.pdf` | Stanley & Miikkulainen — *Evolving neural networks through augmenting topologies*, Evol Comput 10(2) (30 pp. MIT Press author copy) | Topology growth + speciation + historical markings: the growth precedent for variable-length stacks; speciation primitive used by `symbios-genetics`. |
| `G05_qd_review_2016.pdf` | Pugh, Soros & Stanley — *Quality diversity: a new frontier for evolutionary computation*, Front Robot AI (17 pp., OA) | QD taxonomy (novelty search, MAP-Elites, CVT) covering the Lehman & Stanley 2011 novelty work still paywalled; frames QD vs MOO vs novelty. |
| `G06_bop_elites_2023.pdf` | Kent et al. — *BOP-Elites: Bayesian optimisation for QD with black-box descriptors*, arXiv:2307.09326 (15 pp.) | EJIE acquisition over fitness+descriptor GPs; QD-vs-MOO-vs-multimodal decision guide; mispredicted-descriptor rule. |
| `G07_sail_2017.pdf` | Gaier, Asteroth & Mouret — *Data-efficient exploration/optimisation/modelling of diverse designs through SAIL*, arXiv:1702.03713 (7 pp.) | The surrogate-assisted illumination recipe: acquisition map → sample → refit → prediction map. |
| `G08_ua_qd_offline_2026.pdf` | *Uncertainty-aware QD for offline biological sequence design (POME)*, arXiv:2611.00716 (25 pp.) | Ensemble-pessimistic offline QD from logged runs, zero new simulations — the exhaust-reuse template. |
| `G09_moqd_molecular_2026.pdf` | Mashak & Schrum — *CVT archives + chemical embeddings for MOQD in molecular design*, arXiv:2604.05622 (4 pp.) | **Strongest coating template**: MOME local fronts + CVT on ChemBERTa→UMAP manifold beats NSGA-II/MAP-Elites/SA on 4 objectives; SMILES genome, no crossover. |
| `G10_qd_crystal_2024.pdf` | *Illuminating the property space in crystal structure prediction with QD*, arXiv:2403.03511 (9 pp.) | Property-map precedent for material-pool illumination. |
| `G11_differentiable_qd_2021.pdf` | Fontaine et al. — *Differentiable Quality Diversity*, NeurIPS 2021 (29 pp.) | Gradient-based QD (CMA-MAEGA/DQD): differentiate the simulator, fill archives by gradient + ES hybrid. TMM adjoints make this directly implementable. |
| `G12_unsupervised_qd_deceptive_2025.pdf` | Coiffard et al. — *Overcoming deceptiveness with unsupervised QD* (11 pp.) | Learned descriptors beat hand-picked axes on deceptive landscapes — retire depth×thickness as the only map. |
| `G13_moqd_unbounded_2025.pdf` | Janmohamed et al. — *MOQD in unstructured and unbounded spaces* (13 pp.) | Grid-free MOQD for unbounded (variable-length) genomes; growing/unstructured archives. |
| `G14_qd_diffusion_2026.pdf` | *Spectral-guided QD diffusion* (29 pp.) | QD guidance inside diffusion sampling → diverse portfolios; the E05.6 engine with a paper. |
| `G15_qd_highdim_measures_2026.pdf` | *Discount model search for QD in high-dimensional measure spaces*, ICLR 2026 (39 pp.) | Full spectra as descriptors made tractable; covers CMA-MAE state of the art. |

Deliberately skipped (gated, adequately covered by proxies): Hornby 2006 ALPS (ACM; `oxigen` age-unfitness documented in E04), Jones et al. 1998 EGO (JGO; EI covered by G02/A04), Bergstra et al. 2011 TPE (NIPS PDF unlocated; sampler docs in E04), Lehman & Stanley 2011 novelty (ECJ; covered by G05), COCO platform paper (wrong arXiv id guessed and dropped — `1604.06376` is stochastic games; A11 + `kurobako`/`mahf-coco` docs carry the benchmarking angle).

## C — Needle-method & GA-for-coatings classics: hunt results

**Retrieved (in folder):**

| File | Work | Notes |
|---|---|---|
| `C00_furman_tikhonravov_basics_optics_multilayers.pdf` (1.7 MB) | Sh. A. Furman & A. V. Tikhonravov — *Basics of Optics of Multilayer Systems*, Éditions Frontières (1992, 242 pp.) | The foundational textbook, author-hosted on optilayer.com. Covers transfer-matrix theory, synthesis, refinement and the optimality conditions the needle method is built on. |
| `C01_guaranteed_global_optimization_thinfilm_2019.pdf` (2.5 MB) | Azunre, Jean & Rotschild — *Guaranteed global optimization of thin-film optical systems*, New J Phys (2019, IOP open access) | Deterministic branch-and-bound global optimization for thin films — the rigorous counterpart to stochastic GA/PSO. Companion: Azunre's MIT PhD thesis *A parallel branch-and-bound algorithm for thin-film optical systems…* (2014, DSpace@MIT — bot-walled for automated download, fetch by hand). |

**Confirmed paywalled (no OA copy found anywhere indexed — hunt log below).**
Get these via an Optica/SPIE subscription, or the ResearchGate author copy
(human browser; bots are Cloudflare-walled):

| Paper (citations) | DOI | Content in one line | Access pointers |
|---|---|---|---|
| Tikhonravov, Trubetskov & DeBell — *Application of the needle optimization technique to the design of optical coatings*, Appl Opt 35 (1996), pp. 5493–5508 (304 cited) | 10.1364/ao.35.005493 | **THE needle paper**: insert infinitesimal layers at analytically optimal positions, re-optimize, repeat. | RG author PDF: `researchgate.net/publication/49654448_…` · Optica · NRC record: metadata only |
| Dobrowolski & DeBell — *Implementation of a numerical needle method for thin-film design*, Appl Opt 35 (1996), pp. 5484–5492 (81 cited) | 10.1364/ao.35.005484 | The flexible variant: needle positions found *numerically*, so the merit can be anything (e.g. CIE color coordinates). | NRC record page (metadata only) · Optica |
| Sullivan & Dobrowolski — *Refinement of optical multilayer systems with different optimization procedures*, Appl Opt 29 (1990), pp. 2876–2882 (154 cited) | 10.1364/ao.29.002876 | Head-to-head of refinement optimizers (damped least squares et al.) — the baseline every refinement comparison cites. | Optica |
| Martin, Rivory & Schoenauer — *Synthesis of optical multilayer systems using genetic algorithms*, Appl Opt 34 (1995), pp. 2247–2254 (116 cited) | 10.1364/ao.34.002247 | **THE GA-for-coatings paper**: variable-length genomes for stack synthesis. No author copy in HAL (310 Schoenauer deposits checked), none in OpenAlex/Unpaywall. | Optica |
| Shi, Li & Raman — *Optimization of Multilayer Optical Films with a Memetic Algorithm and Mixed Integer Programming*, ACS Photonics (2017) (160 cited) | 10.1021/acsphotonics.7b01136 | **THE memetic paper for films**: GA global search + local refinement + mixed-integer programming for material choice. No preprint, no repository copy found. | ACS |
| Tikhonravov, Trubetskov & DeBell — *Optical coating design approaches based on the needle optimization technique*, Appl Opt 46 (2007), pp. 704–710 (149 cited) | 10.1364/ao.46.000704 | The mature-needle review: approaches, gradual evolution, constrained needle. | Optica |
| Tikhonravov & Trubetskov — *Modern design tools and a new paradigm in optical coating design*, Appl Opt 51 (2012), p. 7319 (70 cited) | 10.1364/ao.51.007319 | Late review: where needle + global optimization stood in 2012. | Optica |
| Dobrowolski — *Completely Automatic Synthesis of Optical Thin Film Systems*, Appl Opt 4 (1965), p. 937 (116 cited) | 10.1364/ao.4.000937 | The founding automatic-synthesis paper. | Optica |
| Dobrowolski — *Optical thin film synthesis program based on the use of Fourier transforms*, Appl Opt 17 (1978), p. 3039 (191 cited) | 10.1364/ao.17.003039 | Fourier-transform synthesis classic (the main rival family to needle). | Optica |

**Hunt log (dead ends, so nobody repeats them):** Semantic Scholar API —
persistently 429 on this IP; fatcat/Internet Archive — unreachable
(connection timeout); De Gruyter/Brill & MDPI — bot-walled (JS/Akamai);
MIT DSpace bitstreams — AWS WAF challenge; NRC repository — only metadata
for the 1996 needle papers (manuscript not deposited); HAL — nothing for
Schoenauer/needle/coating; dblp & CiteSeerX — bot-walled/dead;
ResearchGate — full text exists but Cloudflare-walled; Optica viewmedia —
JS-gated. Unpaywall confirms the six papers above have no OA copy.

## Could not fetch (paywalled or bot-blocked) — key references to chase separately

- **Needle method classics** (Tikhonravov et al.; the constructive refinement
  baseline every coating optimizer is compared against) — see §C above:
  the 1996 needle paper and the 1990 refinement comparison are Appl. Opt.
  paywalled with no OA copy; the next-best retrievable items are the
  author-hosted Furman & Tikhonravov textbook (C00) and the NRC record
  pages. Human path: ResearchGate author PDF or Optica subscription.
- **GA-for-coatings classics** (Martin/Rivory/Schoenauer 1995; Shi/Li/Raman
  2017 memetic) — see §C: both paywalled, no preprint/repository copy
  anywhere indexed (HAL, OpenAlex, Unpaywall all checked).
- *Wide-angle deep-UV antireflective multilayers via discrete-to-continuous
  optimization* (Nanophotonics 2023, doi:10.1515/nanoph-2023-0102) —
  De Gruyter/Brill blocked automated download (JS challenge).
- *Inverse Design for Coating Parameters in Nano-Film Growth* (MDPI
  Photonics 9(8):513, doi:10.3390/photonics9080513) — MDPI/Akamai blocked
  automated download.
- Replacement strategy used: arXiv-native papers covering the same angles
  (B04–B06) instead.

## Working takeaways (to be deepened from the PDFs)

1. **No single winner** — the 2019–2024 survey (A01) and the "315 functions"
   review (A11) both push the same conclusion: rank depends on landscape;
   always benchmark CMA-ES + DE + BO on *your* merit before committing.
2. **Hybridization is the norm** (A02, A03): pure GA/PSO rarely wins;
   memetic combos (global + needle/gradient refinement) dominate coating
   practice — matches the classical needle + GA literature.
3. **Variable-size framing** (A10) is the right mental model for stack
   synthesis: layer count discrete, thicknesses continuous, materials
   categorical — BRKGA (A09) handles exactly this encoding well.
4. **Expensive merits → BO or surrogate-assisted EC** (A04–A06, B06):
   each transfer-matrix evaluation is cheap, but full merit landscapes
   with manufacturing constraints are not; BO earns its place there.
5. **Automation frontier** (A07, A08): per-instance algorithm selection
   and LLM-assisted heuristic design — worth watching before hand-tuning
   operators.
6. **Thin-film specifics** (B01–B07): the field is moving from
   pure global search → learned inverse models + RL constructors +
   multi-objective formulations; needle-type local refinement remains
   the refinement stage in hybrid pipelines.
