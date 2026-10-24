# Gitflow

Koebi follows gitflow ([ADR 0007](../adr/0007-gitflow-and-delivery.md)). Branch rules live in
`xtask/src/gitflow.rs` and are checked by the `pre-push` hook and the `PR policy` workflow.

```mermaid
gitGraph
    commit id: "v0.1.0" tag: "v0.1.0"
    branch develop
    checkout develop
    branch feature/12-rounded-rects
    commit id: "work"
    checkout develop
    merge feature/12-rounded-rects id: "feat: ... (squash)"
    branch release/0.2.0
    commit id: "chore(release): 0.2.0"
    checkout main
    merge release/0.2.0 tag: "v0.2.0"
    checkout develop
    merge main id: "back-merge"
```

## Branches

| Branch | From | Pull request into | Merge method |
|---|---|---|---|
| `main` | | | released code only, tagged `vX.Y.Z` |
| `develop` | `main` | | integration, default branch |
| `feature/<slug>` | `develop` | `develop` | squash |
| `bugfix/<slug>` | `develop` | `develop` | squash |
| `release/X.Y.Z` | `develop` | `main` | merge commit |
| `hotfix/X.Y.Z` | `main` | `main` | merge commit |
| back-merge `main` | | `develop` | merge commit (opened automatically) |

`<slug>` is lowercase letters, digits, `.`, `_` or `-`, ideally starting with the issue number:
`feature/42-text-shaping`. Bot branches (`dependabot/*`, `claude/*`) target develop.

Nobody pushes to `main` or `develop` directly: rulesets require a pull request and a green CI.

## Daily work

```sh
git switch develop && git pull --ff-only
git switch -c feature/42-text-shaping
# ... commits ...
git push -u origin HEAD
gh pr create --base develop --fill
```

Keep the branch current by merging `develop` into it, or by rebasing and pushing with
`--force-with-lease` (your own feature branch only).

In Claude Code, `/feature <issue or description>` and `/pr` do the same steps.

## Commits

[Conventional Commits](https://www.conventionalcommits.org): `type(scope): subject`, subject in
lowercase imperative, no final period. Aim for a header of 72 characters; the hook rejects more
than 100.

| Type | Use |
|---|---|
| `feat` | new capability |
| `fix` | bug fix |
| `perf` | performance improvement |
| `refactor` | internal change, same behaviour |
| `test` | tests only |
| `docs` | documentation only |
| `build` | Cargo, toolchain, dependencies |
| `ci` | GitHub Actions, CI scripts |
| `chore` | maintenance, releases |
| `revert` | revert a previous commit |
| `style` | formatting only |

Scopes name the area: `core`, `gpu`, `render`, `platform`, `app`, `testkit`, `xtask`, `ci`,
`android`, `ios`, `docs`. A breaking change adds `!` (`feat(gpu)!: ...`) and explains it in a
`BREAKING CHANGE:` footer.

Feature branches may contain any commits; the **pull request title** becomes the squash commit
on develop, so the title must follow the convention (checked by CI).

## Pull requests

- Fill in the template: summary, linked issue (`Closes #42`), how it was tested.
- Required checks: `CI result` and `PR policy result`. Review conversations must be resolved.
- Approvals required: 0 while the project has one maintainer; raise
  `required_approving_review_count` in `.github/rulesets/*.json` when the team grows.
- Squash-merge features and bug fixes: the commit on develop is the pull request title alone.
  The head branch is deleted automatically.

## Versioning

Koebi follows [Semantic Versioning](https://semver.org). The version of a release is derived
from the Conventional Commits since the last tag:

| Commits since the last tag | Before 1.0 | From 1.0 |
|---|---|---|
| A breaking change (`!` after the type, or a `BREAKING CHANGE:` footer) | minor | major |
| At least one `feat` | minor | minor |
| Anything else | patch | patch |

```sh
cargo xtask version --next    # prints the next version and why
```

The workspace version in `Cargo.toml` is the single source of truth: every crate inherits it,
the Android and Apple packages read it, and the release tag repeats it.

Release tags are named `vX.Y.Z`. They are annotated, created only by the Release workflow
after a release or hotfix pull request is merged into `main`, and immutable: the
`release-tags` ruleset forbids deleting or moving them. Never create a release tag by hand.

The changelog is generated from commit titles by [git-cliff](https://git-cliff.org)
(`cliff.toml`): `feat` lands in "Added", `fix` in "Fixed", and so on. On develop, each squash
commit is a pull request title, so good titles make a good changelog.

## Releases

1. Compute the version from an up-to-date develop:
   `git switch develop && git pull --ff-only && cargo xtask version --next`.
2. Branch `release/X.Y.Z` from develop.
3. `cargo xtask version X.Y.Z` (updates `Cargo.toml` and `Cargo.lock`).
4. `git cliff --tag vX.Y.Z -o CHANGELOG.md`, then commit `chore(release): X.Y.Z`.
5. `cargo xtask release-check --head release/X.Y.Z`: the branch, `Cargo.toml` and
   `CHANGELOG.md` agree, the tag does not exist, and the version is at least what the commits
   require. The PR policy runs the same check.
6. Push and open the pull request into `main`. Only fixes land on the release branch. Merge
   with a merge commit.
7. The Release workflow packages every platform, creates the annotated tag `vX.Y.Z` and the
   GitHub Release, and opens the `main` to `develop` back-merge pull request. Merge it with a
   merge commit.

In Claude Code: `/release` (or `/release X.Y.Z`).

## Hotfixes

1. Branch `hotfix/X.Y.Z` from `main`: the latest release with the patch number incremented.
2. Write the regression test, fix, `cargo xtask version X.Y.Z`, update the changelog.
3. `cargo xtask release-check --head hotfix/X.Y.Z`, then open the pull request into `main` and
   merge it with a merge commit. Delivery and the back-merge are automatic.

In Claude Code: `/hotfix <description>`.

## Local hooks

`cargo xtask install-hooks` sets `core.hooksPath` to `.githooks/`:

| Hook | Check |
|---|---|
| `pre-commit` | `cargo fmt --all --check` |
| `commit-msg` | Conventional Commits header (`cargo xtask commit-msg`) |
| `pre-push` | refuses pushes to `main` and `develop`; validates the branch name |

The server-side rulesets enforce the same rules, so a skipped hook only delays the failure.
