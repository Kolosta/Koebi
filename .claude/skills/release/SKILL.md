---
name: release
description: Cut a gitflow release X.Y.Z from develop (version bump, changelog, PR to main). The CD workflow then tags, publishes and opens the back-merge.
argument-hint: "<X.Y.Z>"
disable-model-invocation: true
allowed-tools: Bash(git *) Bash(gh *) Bash(cargo xtask *) Bash(git-cliff *) Bash(git cliff *)
---

Prepare release $ARGUMENTS.

1. Check the version: SemVer, greater than the current one (`[workspace.package]` in
   `Cargo.toml`). Breaking changes since the last tag → major (minor while `0.x`), features →
   minor, fixes only → patch. Inspect `git log --oneline $(git describe --tags --abbrev=0 2>/dev/null || git rev-list --max-parents=0 HEAD)..origin/develop`.
2. Branch from an up-to-date develop:
   ```sh
   git fetch origin && git switch develop && git pull --ff-only origin develop
   git switch -c release/<X.Y.Z>
   ```
3. Bump: `cargo xtask version <X.Y.Z>` (updates Cargo.toml and Cargo.lock).
4. Changelog: `git cliff --tag v<X.Y.Z> -o CHANGELOG.md` if git-cliff is installed; otherwise
   move the `[Unreleased]` entries of `CHANGELOG.md` under `## [X.Y.Z] - <date>` by hand.
5. Commit `chore(release): <X.Y.Z>`, run `cargo xtask ci`, then push and open the PR to
   **main** (merge commit, never squash):
   ```sh
   git push -u origin HEAD
   gh pr create --base main --title "chore(release): <X.Y.Z>" --body "Release <X.Y.Z>. See CHANGELOG.md."
   ```
6. Only release fixes may land on the release branch (`fix:` commits). After merge, the Release
   workflow creates tag `v<X.Y.Z>`, the GitHub Release with all platform artifacts, and the
   `main -> develop` back-merge PR: tell the user to merge it with a merge commit.
