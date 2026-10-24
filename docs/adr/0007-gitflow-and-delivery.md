# 0007. Gitflow with automated delivery

- Status: Accepted
- Date: 2026-10-24
- Deciders: @Kolosta

## Context

Koebi ships versioned builds for five platforms. `main` must always contain released code,
integration happens continuously, and both humans and AI agents must follow the same rules
without relying on memory.

## Decision

- Gitflow branches: `main` (released, tagged `vX.Y.Z`), `develop` (integration, default
  branch), `feature/*` and `bugfix/*` from develop, `release/X.Y.Z` from develop,
  `hotfix/X.Y.Z` from main.
- Features and bug fixes are squash-merged into develop; the pull request title, a
  Conventional Commit, becomes the commit message and feeds the changelog.
- Release and hotfix branches are merged into main with a merge commit; the back-merge from
  main to develop also uses a merge commit, so history stays connected.
- The version is set on the release branch (`cargo xtask version X.Y.Z`). Every push to main
  whose version has no tag yet triggers delivery: all platforms are packaged, the tag and the
  GitHub Release are created, and a back-merge pull request is opened.
- Rules are code: branch rulesets in `.github/rulesets/`, branch and commit validation in
  `xtask` (used by git hooks, CI and Claude Code skills alike), applied to GitHub by
  `scripts/github/setup-repository.sh`.
- Two stable required checks, `CI result` and `PR policy result`, aggregate all jobs so
  rulesets do not change when jobs are added.

## Consequences

- `main` is always releasable, and a release is a reviewed pull request.
- More ceremony than trunk-based development, mitigated by skills (`/feature`, `/pr`,
  `/release`, `/hotfix`) and automation.
- Back-merge pull requests must not be squashed, or develop loses the release ancestry.

## Alternatives considered

- **GitHub flow or trunk-based development**: simpler, but no stabilization branch and no
  separation between integrated and released code.
- **The git-flow command-line extension**: a local tool; xtask and skills enforce the same
  model on every machine and in CI.
- **release-please or semantic-release**: automatic version bumps, but more tooling and less
  explicit control over release content; can be revisited later.
