---
name: feature
description: Start a gitflow feature or bugfix branch from an up-to-date develop. Use when beginning any new piece of work, before editing files.
argument-hint: "<issue-number> <short-slug> [--bugfix]"
allowed-tools: Bash(git *) PowerShell(git *)
---

Start a gitflow branch for: $ARGUMENTS

Current state:
- branch: !`git branch --show-current`
- uncommitted changes: !`git status --short`

Steps:

1. If there are uncommitted changes, stop and ask the user whether to commit, stash or keep
   them; never discard work.
2. Build the name: `feature/<issue>-<slug>` (or `bugfix/<issue>-<slug>` with `--bugfix` or when
   the work fixes a bug). The slug is lowercase, words separated by `-`, at most ~5 words, e.g.
   `feature/42-rounded-rect-borders`. Without an issue number, use only the slug.
3. Validate it: `cargo xtask gitflow-check --head <branch>`.
4. Update develop and branch from it:
   ```sh
   git fetch origin
   git switch develop
   git pull --ff-only origin develop
   git switch -c <branch>
   ```
   If `develop` does not exist locally, use `git switch -c develop --track origin/develop` first.
5. Confirm the branch name and remind the plan: small Conventional Commits, tests at the right
   level, `/check` before `/pr`.
