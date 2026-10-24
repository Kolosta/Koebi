---
name: adr
description: Write an Architecture Decision Record in docs/adr/ for a significant technical decision (new dependency, public API shape, rendering technique, platform strategy).
argument-hint: "<short title>"
---

Create an ADR titled: $ARGUMENTS

1. Find the next number: list `docs/adr/` and take the highest `NNNN-*.md` + 1 (4 digits).
2. Copy the structure of `docs/adr/0000-template.md` into `docs/adr/NNNN-<kebab-title>.md`.
3. Fill every section from the conversation and the code; keep it to one page:
   - **Context**: the forces and constraints (performance, platforms, dependency policy).
   - **Decision**: what we do, stated in one or two sentences, then details.
   - **Consequences**: what becomes easier or harder, risks, follow-ups.
   - **Alternatives considered**: each with the reason it was rejected.
4. Status is `Proposed` until the maintainer accepts it in review; then `Accepted`. Never
   rewrite an accepted ADR: supersede it with a new one and link both ways.
5. Add the ADR to the list in `docs/adr/README.md`.
