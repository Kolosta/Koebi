#!/usr/bin/env bash
# One-time, idempotent GitHub configuration for Koebi's gitflow (safe to re-run):
#   - creates `develop` from `main` and makes it the default branch;
#   - merge settings (squash for features, merge commits for releases, no rebase merges);
#   - Actions token read-only by default, allowed to open the release back-merge PR;
#   - rulesets from .github/rulesets/*.json (PRs + green CI required on main/develop, no
#     force-push or deletion, immutable release tags);
#   - labels used by the issue templates and the labeler;
#   - security features (Dependabot alerts/updates, secret scanning, CodeQL, private reports).
#
# Requirements: GitHub CLI authenticated as a repository admin (`gh auth login`), run from
# the repository root. On Windows, use Git Bash: `bash scripts/github/setup-repository.sh`.
set -euo pipefail

repo="${1:-$(gh repo view --json nameWithOwner --jq .nameWithOwner)}"
echo "Configuring $repo"

step() { printf '\n==> %s\n' "$1"; }

step "develop branch"
if gh api "repos/$repo/branches/develop" > /dev/null 2>&1; then
    echo "develop exists"
else
    sha="$(gh api "repos/$repo/git/ref/heads/main" --jq .object.sha)"
    gh api "repos/$repo/git/refs" -f ref=refs/heads/develop -f sha="$sha" > /dev/null
    echo "created develop from main ($sha)"
fi

step "repository settings"
gh api -X PATCH "repos/$repo" \
    -f default_branch=develop \
    -F allow_squash_merge=true \
    -F allow_merge_commit=true \
    -F allow_rebase_merge=false \
    -F allow_auto_merge=true \
    -F allow_update_branch=true \
    -F delete_branch_on_merge=true \
    -f squash_merge_commit_title=PR_TITLE \
    -f squash_merge_commit_message=PR_BODY \
    -f merge_commit_title=PR_TITLE \
    -f merge_commit_message=PR_BODY \
    -F has_issues=true \
    -F has_projects=true \
    -F has_wiki=false > /dev/null
echo "default branch: develop; squash + merge commits allowed; head branches auto-deleted"

step "GitHub Actions permissions"
gh api -X PUT "repos/$repo/actions/permissions/workflow" \
    -f default_workflow_permissions=read \
    -F can_approve_pull_request_reviews=true > /dev/null
echo "GITHUB_TOKEN read-only by default; workflows may open pull requests"

step "rulesets"
existing="$(gh api "repos/$repo/rulesets" --jq '.[] | "\(.name)=\(.id)"')"
for file in .github/rulesets/*.json; do
    name="$(basename "$file" .json)"
    id="$(printf '%s\n' "$existing" | sed -n "s/^$name=//p")"
    if [ -n "$id" ]; then
        gh api -X PUT "repos/$repo/rulesets/$id" --input "$file" > /dev/null
        echo "updated ruleset $name"
    else
        gh api -X POST "repos/$repo/rulesets" --input "$file" > /dev/null
        echo "created ruleset $name"
    fi
done

step "labels"
while IFS='|' read -r name color description; do
    [ -n "$name" ] || continue
    gh label create "$name" --repo "$repo" --color "$color" --description "$description" --force > /dev/null
    echo "label: $name"
done << 'LABELS'
type: feature|1d76db|New capability
type: bug|d73a4a|Something is broken
type: perf|5319e7|Performance improvement
type: refactor|c5def5|Internal change without behaviour change
type: docs|0075ca|Documentation
type: chore|ededed|Maintenance
area: core|bfd4f2|koebi-core: math, color, logging
area: gpu|bfd4f2|koebi-gpu: Vulkan RHI
area: render|bfd4f2|koebi-render: 2D renderer and shaders
area: platform|bfd4f2|koebi-platform: SDL3, windows, events
area: app|bfd4f2|The koebi application
area: tests|bfd4f2|Tests and koebi-testkit
area: ci|bfd4f2|GitHub Actions and automation
area: docs|bfd4f2|docs/ and Markdown
area: tooling|bfd4f2|xtask, hooks, Cargo configuration
area: ai|bfd4f2|Claude Code configuration
platform: windows|f9d0c4|Windows-specific
platform: linux|f9d0c4|Linux-specific
platform: macos|f9d0c4|macOS-specific
platform: android|f9d0c4|Android-specific
platform: ios|f9d0c4|iOS-specific
priority: critical|b60205|Fix immediately (hotfix)
priority: high|d93f0b|Next in line
priority: medium|fbca04|Planned
priority: low|0e8a16|Nice to have
status: needs-triage|fef2c0|Waiting for a maintainer
status: blocked|000000|Blocked by something else
status: needs-adr|c2e0c6|Needs an architecture decision record first
release|0e8a16|Release or hotfix
dependencies|0366d6|Dependency updates
good first issue|7057ff|Good for newcomers
help wanted|008672|Extra attention is needed
breaking change|b60205|Changes a public API or behaviour
LABELS

step "security features"
gh api -X PUT "repos/$repo/vulnerability-alerts" > /dev/null && echo "Dependabot alerts on" || echo "Dependabot alerts: enable in Settings > Code security"
gh api -X PUT "repos/$repo/automated-security-fixes" > /dev/null && echo "Dependabot security updates on" || echo "Dependabot security updates: enable manually"
gh api -X PUT "repos/$repo/private-vulnerability-reporting" > /dev/null && echo "private vulnerability reporting on" || echo "private vulnerability reporting: enable manually"
gh api -X PATCH "repos/$repo" --input - > /dev/null << 'JSON' && echo "secret scanning + push protection on" || echo "secret scanning: enable manually"
{"security_and_analysis": {"secret_scanning": {"status": "enabled"}, "secret_scanning_push_protection": {"status": "enabled"}}}
JSON
gh api -X PATCH "repos/$repo/code-scanning/default-setup" -f state=configured > /dev/null && echo "CodeQL default setup on" || echo "CodeQL: enable the default setup in Settings > Code security"

cat << 'NEXT'

Done. Optional next steps:
  - Claude Code on GitHub: run /install-github-app in Claude Code (adds the app and secret),
    then: gh variable set CLAUDE_CODE_ENABLED --body true
  - Android release signing secrets: KOEBI_ANDROID_KEYSTORE_BASE64, KOEBI_ANDROID_KEYSTORE_PASSWORD,
    KOEBI_ANDROID_KEY_ALIAS, KOEBI_ANDROID_KEY_PASSWORD
  - A project board: gh auth refresh -s project && gh project create --owner @me --title Koebi
NEXT
