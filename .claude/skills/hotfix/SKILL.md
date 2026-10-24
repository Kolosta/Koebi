---
name: hotfix
description: Fix a production bug with a gitflow hotfix X.Y.Z branch from main (regression test first, patch version bump, PR to main).
argument-hint: "<X.Y.Z> <issue-or-description>"
disable-model-invocation: true
allowed-tools: Bash(git *) Bash(gh *) Bash(cargo *)
---

Hotfix: $ARGUMENTS

1. The version is the current released version with the patch number incremented.
2. Branch from main:
   ```sh
   git fetch origin && git switch main && git pull --ff-only origin main
   git switch -c hotfix/<X.Y.Z>
   ```
3. Reproduce first: add a regression test that fails for the reported reason (commit
   `test(<area>): reproduce <bug>`), then fix it with the smallest safe change
   (`fix(<area>): ...`).
4. `cargo xtask version <X.Y.Z>`, update `CHANGELOG.md`, commit `chore(release): <X.Y.Z>`.
5. `cargo xtask ci`, then push and open the PR to **main**:
   `gh pr create --base main --title "fix(<area>): <summary>"`.
6. After merge, CD tags and publishes `v<X.Y.Z>` and opens the back-merge PR into develop;
   it must be merged (merge commit) so the fix is not lost.
