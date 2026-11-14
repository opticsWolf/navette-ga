# research/ — local-only research bundle

This directory is **not tracked** (see `.gitignore`) except for this README.
It holds the heavy, workstation-local material behind the strategy documents:

```
papers/      source PDFs (A01…G15); the index and why each is here is
             docs/README.md
kb.db        the okfgraph/mordant knowledge bundle (~2 GB)
artifacts/   ad-hoc HTML/JSON captures from the research sessions
probe/       LaTeX/PDF build probes for the paper drafts
```

The tracked equivalents are:

- `docs/README.md` — the paper index (titles, venues, relevance, provenance);
- `docs/surveys/E00…E07` — Rust crate surveys and gap analysis;
- `docs/strategy/E08…E10` — the verified strategy and implementation set;
- `docs/papers/PAPER_*` — markdown drafts of the project paper;
- `docs/papers/Technical_Review_GA_ThinFilm_Strategy_Set.pdf` — the small,
  tracked review response.

`okfgraph.toml` at the repository root points at `research/kb.db`. If the
database is moved or rebuilt, update that path rather than duplicating it.
