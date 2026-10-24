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
   merge; a squash commit is the pull request title alone, a merge commit takes the title and
   body;
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

## Project board

The public "Koebi" project shows the roadmap: every open issue with its status, priority,
area and size, grouped by milestones (`v0.1.0 Foundation`, `v0.2.0 Text`...). Creating a
project needs the `project` scope, which `gh auth login` does not request by default:

```sh
gh auth login -h github.com -s project     # once; opens the browser
bash scripts/github/setup-project.sh       # safe to re-run
```

The script creates the project (fields Priority, Area and Size next to the built-in Status),
links it to the repository, makes it public, adds every open issue and sets Priority and Area
from the labels. It also creates the views:

| View | Layout | Shows |
|---|---|---|
| Board | kanban, one column per Status | the work in progress |
| Backlog | table, sorted by priority | open issues without a milestone |
| By milestone | table, grouped by milestone | the roadmap, release by release |
| Good first issues | table | open `good first issue` and `help wanted` issues |

One setting exists only in the web interface, and the script reminds it: in the project's
Workflows, enable "Auto-add to project" with the filter `is:issue is:open`, so new issues join
the board. "Item closed" and "Pull request merged" already move items to Done.

Re-run the script after creating issues in bulk; "Auto-add" covers the rest.

### Projects, milestones and labels

They complement each other:

- **Labels** classify an issue in its repository: type, area, platform, priority,
  `good first issue`.
- **Milestones** say which release an issue targets; an issue has at most one, and the
  milestone page shows the progress of that release.
- **The project** organizes the work across issues, pull requests and even repositories, with
  its own fields (Status, Size), views (board, tables, roadmap), filters, sorting and
  automation. Labels and milestones appear in it as fields, so views can filter and group by
  them. An issue can belong to several projects.

## Verifying

```sh
gh api repos/{owner}/{repo}/rulesets --jq '.[].name'
gh repo view --json defaultBranchRef --jq .defaultBranchRef.name
```

Then open a test pull request from a `feature/*` branch: both required checks must appear.
