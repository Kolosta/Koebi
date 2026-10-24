---
name: hotfix
description: Fix a production bug with a gitflow hotfix X.Y.Z branch from main (regression test first, patch version bump, release check, PR to main).
argument-hint: "<issue-or-description>"
disable-model-invocation: true
allowed-tools: Bash(git *) Bash(gh *) Bash(cargo *)
---

Hotfix: $ARGUMENTS

1. The version is the latest release with the patch number incremented (`v0.3.1` after
   `v0.3.0`): `git fetch origin --tags && git tag --list 'v*' --sort=-v:refname | head -n1`.
2. Branch from main:
   ```sh
   git switch main && git pull --ff-only origin main
   git switch -c hotfix/<X.Y.Z>
   ```
3. Reproduce first: add a regression test that fails for the reported reason (commit
   `test(<area>): reproduce <bug>`), then fix it with the smallest safe change
   (`fix(<area>): ...`).
4. `cargo xtask version <X.Y.Z>`, update `CHANGELOG.md` (`git cliff --tag v<X.Y.Z> -o
   CHANGELOG.md`), commit `chore(release): <X.Y.Z>` (no attribution trailer).
5. `cargo xtask release-check --head hotfix/<X.Y.Z>` and `cargo xtask ci`, then push and open
   the PR to **main**: `gh pr create --base main --title "fix(<area>): <summary>"`.
6. After the merge, CD creates the annotated tag `v<X.Y.Z>`, publishes it and opens the
   back-merge PR into develop; it must be merged (merge commit) so the fix is not lost.
