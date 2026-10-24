---
paths:
  - "docs/**"
  - "*.md"
---

# Documentation

- Audience first: contributors and future maintainers who know Rust but not this codebase.
  Explain *why*, link to code (`crates/koebi-gpu/src/presenter.rs`) for the *how*.
- One topic per file; update the index in `docs/README.md` when adding a page.
- Architecture decisions are ADRs (`/adr`): context, decision, consequences, alternatives.
  ADRs are immutable once accepted; supersede them with a new ADR instead of rewriting.
- Keep CLAUDE.md short (< 150 lines). Area-specific guidance belongs in `.claude/rules/`,
  procedures in `.claude/skills/`, rationale in `docs/`.
- English, present tense, short sentences, no marketing language. Commands in fenced blocks.
