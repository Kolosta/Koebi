#!/usr/bin/env bash
# Creates or updates the public "Koebi" GitHub project (Projects v2) and fills it with the
# repository's open issues; safe to re-run:
#   - fields Priority, Area and Size next to the built-in Status;
#   - the project linked to the repository (its "Projects" tab) and made public;
#   - every open issue added, Priority and Area set from its labels.
#
# Requirements: GitHub CLI authenticated as the owner with the `project` scope, run from the
# repository root (Git Bash on Windows):
#   gh auth login -h github.com -s project
#   bash scripts/github/setup-project.sh
set -euo pipefail

repo="${1:-$(gh repo view --json nameWithOwner --jq .nameWithOwner)}"
owner="${repo%%/*}"
title="Koebi"

step() { printf '\n==> %s\n' "$1"; }

step "project"
number="$(gh project list --owner "$owner" --limit 100 --format json \
    --jq "[.projects[] | select(.title == \"$title\") | .number][0] // empty")"
if [ -z "$number" ]; then
    number="$(gh project create --owner "$owner" --title "$title" --format json --jq .number)"
    echo "created project #$number"
else
    echo "project #$number exists"
fi
gh project edit "$number" --owner "$owner" --visibility PUBLIC \
    --description "Roadmap and work in progress of Koebi, a Rust and Vulkan vector editor and compositor." \
    --readme "$(cat << 'README'
Roadmap and work in progress of [Koebi](https://github.com/Kolosta/Koebi).

- **Status**: Todo, then In Progress when a branch exists, Done when the pull request is merged.
- **Priority** and **Area** mirror the `priority: *` and `area: *` labels.
- **Size**: rough effort, XS (an hour) to XL (several weeks).
- **Milestones** group issues by release: `v0.2.0 Text`, `v0.3.0 UI toolkit`...

Contributing: pick a Todo issue, ideally `good first issue` or `help wanted`, comment to claim it,
then follow [CONTRIBUTING.md](https://github.com/Kolosta/Koebi/blob/develop/CONTRIBUTING.md).
README
)" > /dev/null
if gh project link "$number" --owner "$owner" --repo "$repo" > /dev/null 2>&1; then
    echo "linked to $repo"
else
    echo "already linked to $repo"
fi
project_id="$(gh project view "$number" --owner "$owner" --format json --jq .id)"

step "fields"
ensure_field() { # <name> <comma-separated options>
    local names
    names="$(gh project field-list "$number" --owner "$owner" --format json --jq '.fields[].name')"
    # grep reads everything (no -q): an early exit could break the pipe under pipefail.
    if printf '%s\n' "$names" | grep -Fx -- "$1" > /dev/null; then
        echo "field $1 exists"
    else
        gh project field-create "$number" --owner "$owner" --name "$1" \
            --data-type SINGLE_SELECT --single-select-options "$2" > /dev/null
        echo "created field $1"
    fi
}
ensure_field "Priority" "Critical,High,Medium,Low"
ensure_field "Area" "Core,GPU,Render,Text,UI,Platform,App,Tests,CI,Docs,Tooling,AI"
ensure_field "Size" "XS,S,M,L,XL"

# "Field|Option|FieldId|OptionId" lines for every single-select option.
options="$(gh project field-list "$number" --owner "$owner" --format json \
    --jq '.fields[] | select(.options != null) | . as $f | .options[] | "\($f.name)|\(.name)|\($f.id)|\(.id)"')"

set_option() { # <item id> <field> <option>
    [ -n "$3" ] || return 0
    local ids
    ids="$(printf '%s\n' "$options" |
        awk -F'|' -v f="$2" -v o="$3" '$1 == f && $2 == o && !found { print $3 "|" $4; found = 1 }')"
    if [ -z "$ids" ]; then
        echo "  no option $3 in field $2"
        return 0
    fi
    gh project item-edit --id "$1" --project-id "$project_id" \
        --field-id "${ids%%|*}" --single-select-option-id "${ids##*|}" > /dev/null
}

# Maps a label list ("type: chore,area: ci,priority: high") to field options.
priority_of() {
    case ",$1," in
        *",priority: critical,"*) echo Critical ;;
        *",priority: high,"*) echo High ;;
        *",priority: medium,"*) echo Medium ;;
        *",priority: low,"*) echo Low ;;
    esac
}
area_of() {
    local area
    area="$(printf '%s\n' "$1" | tr ',' '\n' | sed -n 's/^area: //p' | sed -n 1p)"
    case "$area" in
        gpu | ui | ci | ai) printf '%s\n' "$area" | tr '[:lower:]' '[:upper:]' ;;
        "") ;;
        *) printf '%s%s\n' "$(printf '%s' "${area:0:1}" | tr '[:lower:]' '[:upper:]')" "${area:1}" ;;
    esac
}

step "issues"
gh issue list --repo "$repo" --state open --limit 500 --json url,labels \
    --jq '.[] | "\(.url)|\([.labels[].name] | join(","))"' |
    while IFS='|' read -r url labels; do
        item="$(gh project item-add "$number" --owner "$owner" --url "$url" --format json --jq .id)"
        set_option "$item" "Priority" "$(priority_of "$labels")"
        set_option "$item" "Area" "$(area_of "$labels")"
        echo "added $url"
    done

cat << NEXT

Done: https://github.com/users/$owner/projects/$number
Two settings only exist in the web interface (project, then "..." > Settings / Workflows):
  - Workflows > "Auto-add to project": enable it for $repo with the filter "is:issue is:open",
    so new issues join the board automatically.
  - Views: add a Board view grouped by Status, and a Roadmap view by Milestone.
NEXT
