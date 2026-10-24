---
name: pr
description: Open a pull request for the current gitflow branch with the correct base, a Conventional Commits title and the project template.
argument-hint: "[draft]"
disable-model-invocation: true
allowed-tools: Bash(git *) Bash(gh *) Bash(cargo xtask *)
---

Open a pull request for the current branch. Options: $ARGUMENTS

Context:
- branch: !`git branch --show-current`
- status: !`git status --short`

Steps:

1. Determine the base with gitflow: `feature/*`, `bugfix/*` → `develop`;
   `release/*`, `hotfix/*` → `main`. Verify with
   `cargo xtask gitflow-check --head <branch> --base <base>`. Refuse `main`/`develop` as head.
2. Make sure the work is complete: no uncommitted changes, and `cargo xtask ci --fast` passes
   (run `/check` if it was not run since the last change).
3. Summarize the commits since the base: `git log --oneline origin/<base>..HEAD` and
   `git diff --stat origin/<base>...HEAD`.
4. Title: one Conventional Commits line (it becomes the squash commit on develop), e.g.
   `feat(render): add rounded rectangle borders`. Lowercase subject, no trailing period.
5. Body: fill `.github/PULL_REQUEST_TEMPLATE.md` (summary, `Closes #N`, type, how it was
   tested, checklist). Mention golden images or SPIR-V that changed.
6. Push and create (pushing requires the user's confirmation):
   ```sh
   git push -u origin HEAD
   gh pr create --base <base> --title "<title>" --body-file <file> [--draft]
   ```
7. Report the PR URL and the CI checks to watch (`gh pr checks --watch`).
