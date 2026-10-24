#!/usr/bin/env bash
# Bootstraps the public "Koebi" GitHub project (Projects v2), then keeps it in sync with the
# repository's open issues. Safe to re-run: it only adds what is missing, so everything changed
# in the web interface (views, colors, field values, README) is kept.
#   - project created public with its description and README, linked to the repository;
#   - fields Priority and Area (colored like the labels) and Size, next to the built-in Status;
#   - views Board, Backlog, By milestone, All items and Good first issues;
#   - every open issue added; newly added issues get Priority and Area from their labels.
#
# Requirements: GitHub CLI authenticated as the owner with the `project` scope, run from the
# repository root (Git Bash on Windows):
#   gh auth login -h github.com -p https -w -s project
#   bash scripts/github/setup-project.sh
set -euo pipefail

repo="${1:-$(gh repo view --json nameWithOwner --jq .nameWithOwner)}"
owner="${repo%%/*}"
title="Koebi"

step() { printf '\n==> %s\n' "$1"; }

step "project"
number="$(gh project list --owner "$owner" --limit 100 --format json \
    --jq "[.projects[] | select(.title == \"$title\") | .number][0] // empty")"
created=false
if [ -z "$number" ]; then
    number="$(gh project create --owner "$owner" --title "$title" --format json --jq .number)"
    created=true
    gh project edit "$number" --owner "$owner" --visibility PUBLIC \
        --description "Roadmap and work in progress of Koebi, a Rust and Vulkan vector editor and compositor." \
        --readme "$(cat << 'README'
Roadmap and work in progress of [Koebi](https://github.com/Kolosta/Koebi).

- **Status**: Todo, then In Progress when a branch exists, Done when the pull request is merged.
- **Priority** and **Area** start from the `priority: *` and `area: *` labels.
- **Size**: rough effort, XS (an hour) to XL (several weeks, split it into sub-issues).
- **Milestones** group issues by release: `v0.2.0 Text`, `v0.3.0 UI toolkit`...

Views: **Board** (kanban by status), **Backlog** (open issues without a milestone, to triage),
**By milestone** (every open issue, release by release), **All items** (everything, done
included) and **Good first issues**.

Contributing: pick a Todo issue, ideally `good first issue` or `help wanted`, comment to claim it,
then follow [CONTRIBUTING.md](https://github.com/Kolosta/Koebi/blob/develop/CONTRIBUTING.md).
README
)" > /dev/null
    echo "created project #$number (public)"
else
    echo "project #$number exists: description, README and visibility left as they are"
fi
if gh project link "$number" --owner "$owner" --repo "$repo" > /dev/null 2>&1; then
    echo "linked to $repo"
else
    echo "already linked to $repo"
fi
project_id="$(gh project view "$number" --owner "$owner" --format json --jq .id)"

step "fields"
field_names="$(gh project field-list "$number" --owner "$owner" --format json --jq '.fields[].name')"
create_field() { # <name> "Option|COLOR|description"...
    local name="$1"
    shift
    # grep reads everything (no -q): an early exit could break the pipe under pipefail.
    if printf '%s\n' "$field_names" | grep -Fx -- "$name" > /dev/null; then
        echo "field $name exists"
        return 0
    fi
    local options="" option color description spec
    for spec in "$@"; do
        IFS='|' read -r option color description <<< "$spec"
        options+="${options:+,}{\"name\":\"$option\",\"color\":\"$color\",\"description\":\"$description\"}"
    done
    printf '{"query":"mutation($input: CreateProjectV2FieldInput!) { createProjectV2Field(input: $input) { projectV2Field { ... on ProjectV2SingleSelectField { name } } } }","variables":{"input":{"projectId":"%s","dataType":"SINGLE_SELECT","name":"%s","singleSelectOptions":[%s]}}}' \
        "$project_id" "$name" "$options" | gh api graphql --input - > /dev/null
    echo "created field $name"
}
create_field "Priority" \
    "Critical|RED|Fix immediately (hotfix)" \
    "High|ORANGE|Next in line" \
    "Medium|YELLOW|Planned" \
    "Low|GREEN|Nice to have"
# Product areas get a color each; tooling and process areas share gray.
create_field "Area" \
    "Core|BLUE|koebi-core: math, color, logging" \
    "GPU|PURPLE|koebi-gpu: Vulkan RHI" \
    "Render|PINK|koebi-render: 2D renderer and shaders" \
    "Text|ORANGE|koebi-text: fonts, shaping, glyph atlas" \
    "UI|YELLOW|koebi-ui: layout, widgets, design system" \
    "Platform|GREEN|koebi-platform: SDL3, windows, events" \
    "App|RED|The koebi application" \
    "Tests|GRAY|Tests and koebi-testkit" \
    "CI|GRAY|GitHub Actions and automation" \
    "Docs|GRAY|docs/ and Markdown" \
    "Tooling|GRAY|xtask, hooks, Cargo configuration" \
    "AI|GRAY|Claude Code configuration"
create_field "Size" \
    "XS|GRAY|An hour or two" \
    "S|GRAY|About a day" \
    "M|GRAY|A few days" \
    "L|GRAY|One to two weeks" \
    "XL|GRAY|Several weeks: split it into sub-issues"

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
# Field database ids (the views API wants integers) and the existing views.
meta="$(gh api graphql -F login="$owner" -F number="$number" -f query='
    query($login: String!, $number: Int!) {
      user(login: $login) {
        projectV2(number: $number) {
          fields(first: 50) { nodes { ... on ProjectV2FieldCommon { name databaseId } } }
          views(first: 50) { nodes { id name } }
        }
      }
    }' --jq '.data.user.projectV2 | (.fields.nodes[] | "field|\(.name)|\(.databaseId)"), (.views.nodes[] | "view|\(.name)|\(.id)")')"
field_id() { printf '%s\n' "$meta" | awk -F'|' -v n="$1" '$1 == "field" && $2 == n && !found { print $3; found = 1 }'; }
title_f="$(field_id Title)"
status_f="$(field_id Status)"
priority_f="$(field_id Priority)"
area_f="$(field_id Area)"
size_f="$(field_id Size)"
labels_f="$(field_id Labels)"
milestone_f="$(field_id Milestone)"
assignees_f="$(field_id Assignees)"
progress_f="$(field_id "Sub-issues progress")"
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
create_view "All items" "$(printf '{"name":"All items","layout":"table","sort_by":[[%s,"asc"]],"visible_fields":[%s,%s,%s,%s,%s,%s,%s,%s,%s]}' \
    "$priority_f" "$title_f" "$assignees_f" "$status_f" "$priority_f" "$area_f" "$size_f" "$milestone_f" "$labels_f" "$progress_f")"
create_view "Good first issues" "$(printf '{"name":"Good first issues","layout":"table","filter":"is:open label:\\"good first issue\\",\\"help wanted\\"","visible_fields":[%s,%s,%s,%s,%s]}' \
    "$title_f" "$status_f" "$area_f" "$size_f" "$labels_f")"

# A new project starts with an empty "View 1"; the views above replace it.
if [ "$created" = true ]; then
    printf '%s\n' "$meta" | awk -F'|' '$1 == "view" && $2 == "View 1" { print $3 }' |
        while read -r view_id; do
            gh api graphql -f view_id="$view_id" \
                -f query='mutation($view_id: ID!) { deleteProjectV2View(input: { viewId: $view_id }) { clientMutationId } }' > /dev/null
            echo "deleted the default View 1"
        done
fi

step "issues"
# Issues already on the board keep their values: only new ones get Priority and Area.
present="$(gh project item-list "$number" --owner "$owner" --limit 1000 --format json \
    --jq '.items[].content.url // empty')"
gh issue list --repo "$repo" --state open --limit 500 --json url,labels \
    --jq '.[] | "\(.url)|\([.labels[].name] | join(","))"' |
    while IFS='|' read -r url labels; do
        if printf '%s\n' "$present" | grep -Fx -- "$url" > /dev/null; then
            continue
        fi
        item="$(gh project item-add "$number" --owner "$owner" --url "$url" --format json --jq .id)"
        set_option "$item" "Priority" "$(priority_of "$labels")"
        set_option "$item" "Area" "$(area_of "$labels")"
        echo "added $url"
    done

cat << NEXT

Done: https://github.com/users/$owner/projects/$number
One setting only exists in the web interface: in the project's Workflows, enable
"Auto-add to project" for $repo with the filter "is:issue is:open", so new issues join the
board automatically ("Item closed" and "Pull request merged" already move items to Done).
NEXT
