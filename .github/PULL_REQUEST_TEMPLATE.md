<!--
Title: Conventional Commits, it becomes the squash commit message on develop.
  feat(render): add rounded rectangle borders
Branch: feature/* or bugfix/* -> develop | release/X.Y.Z or hotfix/X.Y.Z -> main
-->

## Summary

<!-- What changes and why. Link the issue: "Closes #123". -->

## Type of change

- [ ] Feature
- [ ] Bug fix (a regression test reproduces the bug)
- [ ] Performance
- [ ] Refactoring (no behaviour change)
- [ ] Documentation, CI or tooling
- [ ] Release / hotfix

## How it was tested

- [ ] Unit tests
- [ ] Integration tests (headless GPU)
- [ ] Functional / golden-image tests (`cargo xtask bless` run and PNGs reviewed if visuals changed)
- [ ] End-to-end (`cargo run -p koebi -- --frames 120`)
- [ ] Manually on: <!-- Windows / Linux / macOS / Android / iOS -->

## Checklist

- [ ] `cargo xtask ci` passes locally
- [ ] No new dependency, or an ADR in `docs/adr/` justifies it (and `deny.toml` is updated)
- [ ] Every new `unsafe` block has a `// SAFETY:` comment
- [ ] Public items are documented; docs/ADRs updated when behaviour or architecture changes
- [ ] `CHANGELOG.md` needs no manual edit (generated from commits) or was updated for a release

## AI assistance

<!-- Optional: tools used (e.g. Claude Code) and what was reviewed by hand. Authors stay accountable for every line. -->
