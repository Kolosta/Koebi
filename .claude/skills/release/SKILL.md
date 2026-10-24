---
name: release
description: Cut a gitflow release from develop (computed SemVer version, changelog, release check, PR to main). The CD workflow then tags, publishes and opens the back-merge.
argument-hint: "[X.Y.Z]"
disable-model-invocation: true
allowed-tools: Bash(git *) Bash(gh *) Bash(cargo xtask *) Bash(git-cliff *) Bash(git cliff *)
---

Prepare a release. Requested version: "$ARGUMENTS" (empty: use the computed one).

1. Start from an up-to-date develop and compute the version:
   ```sh
   git fetch origin --tags && git switch develop && git pull --ff-only origin develop
   cargo xtask version --next
   ```
   The command prints the SemVer version required by the Conventional Commits since the last
   tag (breaking change: major, minor before 1.0; `feat`: minor; otherwise patch). Use it
   unless the user asked for a higher one; never go lower.
2. Branch: `git switch -c release/<X.Y.Z>`.
3. Bump: `cargo xtask version <X.Y.Z>` (updates `Cargo.toml` and `Cargo.lock`; a no-op for
   the first release, whose version is already set).
4. Changelog: `git cliff --tag v<X.Y.Z> -o CHANGELOG.md` if git-cliff is installed; otherwise
   move the `[Unreleased]` entries under `## [X.Y.Z] - <date>` by hand.
5. Commit `chore(release): <X.Y.Z>` (no attribution trailer), then check everything:
   ```sh
   cargo xtask release-check --head release/<X.Y.Z>
   cargo xtask ci
   ```
6. Push and open the PR to **main** (merged with a merge commit, never squashed):
   ```sh
   git push -u origin HEAD
   gh pr create --base main --title "chore(release): <X.Y.Z>" --body "Release <X.Y.Z>. See CHANGELOG.md."
   ```
7. Only release fixes may land on the release branch (`fix:` commits). Never create the tag by
   hand: after the merge, the Release workflow creates the annotated tag `v<X.Y.Z>`, the GitHub
   Release with all platform artifacts, and the `main -> develop` back-merge PR, which the
   user merges with a merge commit.
