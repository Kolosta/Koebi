#!/usr/bin/env bash
# Creates or updates the public "Koebi" GitHub project (Projects v2) and fills it with the
# repository's open issues; safe to re-run:
#   - fields Priority, Area and Size next to the built-in Status;
#   - views Board (kanban by Status), Backlog, By milestone and Good first issues;
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

Views: **Board** (kanban by status), **Backlog** (issues without a milestone, by priority),
**By milestone** (the roadmap, release by release) and **Good first issues**.

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

step "views"
# Field database ids (the views API wants integers) and the names of existing views.
meta="$(gh api graphql -F login="$owner" -F number="$number" -f query='
    query($login: String!, $number: Int!) {
      user(login: $login) {
        projectV2(number: $number) {
          fields(first: 50) { nodes { ... on ProjectV2FieldCommon { name databaseId } } }
          views(first: 50) { nodes { name } }
        }
      }
    }' --jq '.data.user.projectV2 | (.fields.nodes[] | "field|\(.name)|\(.databaseId)"), (.views.nodes[] | "view|\(.name)|")')"
field_id() { printf '%s\n' "$meta" | awk -F'|' -v n="$1" '$1 == "field" && $2 == n && !found { print $3; found = 1 }'; }
title_f="$(field_id Title)"
status_f="$(field_id Status)"
priority_f="$(field_id Priority)"
area_f="$(field_id Area)"
size_f="$(field_id Size)"
labels_f="$(field_id Labels)"
milestone_f="$(field_id Milestone)"
assignees_f="$(field_id Assignees)"
user_id="$(gh api "users/$owner" --jq .id)"

create_view() { # <name> <JSON body>
    if printf '%s\n' "$meta" | awk -F'|' -v n="$1" '$1 == "view" && $2 == n { found = 1 } END { exit !found }'; then
        echo "view $1 exists"
        return 0
    fi
    # The documentation names the path parameter user_id: try the login, then the numeric id.
    local who
    for who in "$owner" "$user_id"; do
        if printf '%s' "$2" | gh api --method POST "users/$who/projectsV2/$number/views" \
            -H "X-GitHub-Api-Version: 2026-03-10" --input - > /dev/null 2>&1; then
            echo "created view $1"
            return 0
        fi
    done
    echo "  could not create view $1 through the API: add it in the web interface"
}
create_view "Board" "$(printf '{"name":"Board","layout":"board","vertical_group_by":[%s],"visible_fields":[%s,%s,%s,%s]}' \
    "$status_f" "$priority_f" "$area_f" "$size_f" "$assignees_f")"
create_view "Backlog" "$(printf '{"name":"Backlog","layout":"table","filter":"is:open no:milestone","sort_by":[[%s,"asc"]],"visible_fields":[%s,%s,%s,%s,%s,%s]}' \
    "$priority_f" "$title_f" "$status_f" "$priority_f" "$area_f" "$size_f" "$labels_f")"
create_view "By milestone" "$(printf '{"name":"By milestone","layout":"table","filter":"is:open","group_by":[%s],"sort_by":[[%s,"asc"]],"visible_fields":[%s,%s,%s,%s,%s,%s]}' \
    "$milestone_f" "$priority_f" "$title_f" "$status_f" "$priority_f" "$area_f" "$size_f" "$assignees_f")"
create_view "Good first issues" "$(printf '{"name":"Good first issues","layout":"table","filter":"is:open label:\\"good first issue\\",\\"help wanted\\"","visible_fields":[%s,%s,%s,%s,%s]}' \
    "$title_f" "$status_f" "$area_f" "$size_f" "$labels_f")"

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
    so new issues join the board automatically ("Item closed" and "Pull request merged"
    already move items to Done).
  - The default "View 1" can be deleted or renamed from its tab menu.
NEXT
