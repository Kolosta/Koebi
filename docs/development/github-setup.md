# GitHub setup

Repository settings, branch protection and labels are code. One script applies them, and it
is safe to re-run after any change.

## Prerequisites

- GitHub CLI authenticated as a repository administrator:

```sh
gh auth login
```

- The `main` branch exists on GitHub.

## Apply the configuration

From the repository root (Git Bash on Windows):

```sh
bash scripts/github/setup-repository.sh            # current repository
bash scripts/github/setup-repository.sh owner/name # explicit repository
```

The script:

1. creates `develop` from `main` if missing, and makes it the default branch;
2. allows squash merges and merge commits, disables rebase merges, deletes head branches after
   merge, and uses the pull request title and body for merge commits;
3. makes the Actions token read-only by default and lets workflows open pull requests (needed
   for the release back-merge);
4. creates or updates the rulesets in `.github/rulesets/`;
5. creates the labels used by issue templates and the labeler;
6. enables Dependabot alerts and security updates, secret scanning with push protection,
   private vulnerability reporting and CodeQL default setup, where the plan allows it.

## Rulesets

| Ruleset | Target | Rules |
|---|---|---|
| `main` | `main` | no deletion or force push; pull request required; merge commits only; `CI result` and `PR policy result` required, branch up to date; conversations resolved |
| `develop` | `develop` | same, with squash or merge commits allowed, branch need not be up to date |
| `release-tags` | tags `v*` | tags cannot be deleted, moved or recreated |

Approvals required: 0 (single maintainer). Raise `required_approving_review_count` in the JSON
files and re-run the script when reviewers join. Repository administrators can bypass rules
only if a bypass actor is added; none is configured.

## Optional integrations

- **Claude Code on GitHub**: run `/install-github-app` in Claude Code (installs the app and
  the `ANTHROPIC_API_KEY` or OAuth secret), then enable the workflows:

```sh
gh variable set CLAUDE_CODE_ENABLED --body true
```

- **Android release signing**: add the four `KOEBI_ANDROID_*` secrets described in
  [ci-cd.md](ci-cd.md#signing).
- **Project board**: `gh auth refresh -s project` then
  `gh project create --owner @me --title Koebi`.

## Verifying

```sh
gh api repos/{owner}/{repo}/rulesets --jq '.[].name'
gh repo view --json defaultBranchRef --jq .defaultBranchRef.name
```

Then open a test pull request from a `feature/*` branch: both required checks must appear.
