# Working with Claude Code

Koebi is set up so that Claude Code follows the same rules as human contributors, with as
little context as possible loaded at any time. This page lists what is configured, the options
Claude Code offers, and how to keep token usage under control.

## What is configured

| File | Loaded | Purpose |
|---|---|---|
| `CLAUDE.md` | every session | project memory: rules, map, commands, gitflow, testing (under 150 lines) |
| `.claude/rules/*.md` | when Claude reads matching files (`paths:` frontmatter) | area rules: Rust, unsafe/FFI, GPU, render/shaders, platform, tests, CI, docs |
| `.claude/skills/*/SKILL.md` | descriptions always; body on use | procedures, invoked as `/name` or by Claude |
| `.claude/agents/*.md` | on delegation | specialised subagents with their own context |
| `.claude/settings.json` | every session | shared permissions, hooks, plugins |
| `.claude/settings.local.json` | every session, not committed | personal overrides |
| `.github/workflows/claude*.yml` | on GitHub | `@claude` mentions and automated reviews (opt-in) |

Personal preferences belong in `~/.claude/CLAUDE.md` and `~/.claude/settings.json`, never in
the repository.

### Skills

| Skill | Invoked by | Does |
|---|---|---|
| `/feature <issue or text>` | you or Claude | creates a `feature/*` or `bugfix/*` branch from an up-to-date develop |
| `/pr` | you | runs the checks, pushes, opens a pull request with the template |
| `/release <X.Y.Z>` | you | release branch, version bump, changelog, pull request into main |
| `/hotfix <X.Y.Z>` | you | hotfix branch from main with a regression test first |
| `/check` | you or Claude | `cargo xtask ci` in a forked context; returns only failures |
| `/ci-debug` | you or Claude | reads failed GitHub Actions logs in a forked context; diagnosis and fix |
| `/adr <title>` | you or Claude | writes an architecture decision record from the template |
| `/add-dependency` | you or Claude | evaluates a crate against the dependency policy, prepares the ADR |
| `/gpu-resource` | you or Claude | adds a Vulkan object type following RHI conventions |
| `/render-primitive` | you or Claude | adds a 2D primitive: instance struct, shaders, pipeline, tests |

Skills that change shared state (`/pr`, `/release`, `/hotfix`) set
`disable-model-invocation: true`: only a human can start them.

### Subagents

| Agent | Tools | Use for |
|---|---|---|
| `rust-reviewer` | read-only + shell | review a diff: soundness, layering, performance, tests, docs |
| `vulkan-auditor` | read-only + shell | synchronization, lifetimes, valid usage, portability of GPU code |
| `test-engineer` | read, write, shell | write tests at the right level; reproduce a bug first |

Built-in agents complement them: `Explore` for broad read-only searches, `Plan` for
implementation plans, `general-purpose` for multi-step research. Ask explicitly, for example
"use the vulkan-auditor agent on this diff". Their transcripts stay out of the main context.

### Hooks

A `PostToolUse` hook runs `cargo fmt --all` after Claude edits or writes a `.rs` file, so
formatting never costs a review round. Review hook changes carefully in pull requests: hooks
run shell commands with your permissions.

### Permissions

| Level | Commands |
|---|---|
| Allowed | builds, tests, clippy, fmt, docs, `cargo xtask`, `cargo deny`, typos, read-only git and `gh`, `git add`/`commit`/`switch`, documentation sites |
| Ask | `cargo add`, `cargo update`, `gh pr merge`, `gh release`, `git rebase`, `git reset --hard`, `git tag` |
| Denied | force pushes, pushes to `main`, `cargo publish`, reading `.env`, keystores and certificates |

`git push` to feature branches asks for confirmation by default. Server-side rulesets protect
`main` and `develop` regardless of local settings.

### Plugins

`rust-analyzer-lsp@claude-plugins-official` is enabled for the project. It gives Claude
go-to-definition, references and diagnostics through rust-analyzer (installed by
`rust-toolchain.toml`), which is more precise and cheaper than grepping and reading files.
If it is not installed yet, install it with `/plugin` from the `claude-plugins-official`
marketplace.

## Options worth knowing

### Built-in commands

| Command | Use |
|---|---|
| `/clear` | start a fresh context for an unrelated task |
| `/compact [focus]` | summarise the conversation, keeping what the focus names |
| `/context` | see what fills the context window |
| `/cost`, `/usage` | session cost and plan usage |
| `/model` | switch model for the session |
| `/memory` | edit CLAUDE.md files and auto memory |
| `/rewind` | restore code and conversation to a checkpoint |
| `/resume` | continue an earlier session |
| `/agents`, `/hooks`, `/permissions`, `/plugin`, `/mcp`, `/config` | manage the configuration |
| `/code-review`, `/security-review`, `/simplify` | built-in review passes on the current diff |
| `/install-github-app` | set up the GitHub App and secrets for the workflows |

### Permission modes

Shift+Tab cycles through modes. **Plan mode** reads and proposes without editing: use it for
architecture changes, new GPU objects and anything touching `unsafe`. **Accept edits** suits
routine feature work, since the deny list still applies. Never use the bypass mode on this
repository.

### Models

Use the most capable model (Fable or Opus) for architecture, Vulkan synchronization and
`unsafe` code; Sonnet for routine changes, tests and docs; Haiku for quick lookups. Subagents
inherit the session model (`model: inherit`); set a cheaper model in an agent's frontmatter if
its task is mechanical.

### Parallel work

Run one Claude session per git worktree, each on its own feature branch, to work on several
tasks without mixing contexts or files:

```sh
git worktree add ../koebi-text feature/42-text-shaping
```

### MCP servers

MCP servers add tools (GitHub, issue trackers, documentation search). None is configured: the
`gh` CLI is already allowed for GitHub, and documentation domains are allowed for web fetches.
Every server adds tool descriptions to the context, so add one only for a recurring need, in
`.mcp.json` for the team or with `claude mcp add` for yourself.

### Other plugins

The official marketplaces (`/plugin`) offer LSP plugins for other languages and workflow
plugins such as code review, commit helpers, PR review toolkits, security guidance and hook
authoring. Koebi's skills already cover the git workflow; enable a plugin only if it adds
something, because its skills and agents add descriptions to every session.

### GitHub Actions

- `claude.yml`: mention `@claude` in an issue or pull request comment to ask a question or
  request an implementation. Claude pushes its work to a `claude/*` branch and links a pull
  request into develop, which goes through the normal checks.
- `claude-code-review.yml`: reviews every non-draft pull request from this repository with the
  official code-review plugin, as inline comments.

Both are disabled until you run `/install-github-app` and set the repository variable:

```sh
gh variable set CLAUDE_CODE_ENABLED --body true
```

### Headless mode

`claude -p "<prompt>"` runs non-interactively, for scripts and scheduled tasks. It honours the
same settings and permissions.

## Managing context and tokens

What is always in context: `CLAUDE.md`, skill and agent descriptions, enabled tools. Everything
else loads on demand. The setup keeps the always-loaded part small:

1. **Short CLAUDE.md**: rules and pointers only. Area rules load by path; rationale lives in
   `docs/` and is read only when a task needs it.
2. **Forked skills**: `/check` and `/ci-debug` run verbose commands in a separate context and
   return a short summary.
3. **Subagents for breadth**: searches across many files and reviews run in their own context.
4. **LSP over grep**: precise symbol lookups instead of reading whole files.
5. **Compaction guidance**: the `# Compact instructions` section of `CLAUDE.md` says what to
   keep (goal, decisions, changed files, failing tests, TODOs) and what to drop (logs,
   listings).
6. **Stable prompts cache well**: Claude Code caches the unchanged beginning of the context, so
   edit `CLAUDE.md` deliberately rather than constantly.

Habits:

- One task per session; `/clear` when switching topics.
- Ask for specific files and functions rather than "look at the renderer".
- Prefer `cargo xtask ci --fast` and single-crate test runs while iterating.
- Watch `/context` when a session feels slow, and compact with a focus before it fills up.
- Put durable learnings in the right place: project rules in `CLAUDE.md` or `.claude/rules/`
  through a pull request, personal facts in your own memory.

## Recommended workflows

**Feature**: `/feature 42` → plan mode to agree on the design → implement with tests → `/check`
→ `rust-reviewer` (and `vulkan-auditor` for GPU code) → `/pr`.

**Bug**: `test-engineer` writes the failing test → fix → `/check` → `/pr`.

**Red CI**: `/ci-debug` → fix on the branch → push.

**Release**: `/release 0.2.0` → merge the pull request → merge the automatic back-merge.

**New dependency**: `/add-dependency <crate>` → ADR → maintainer decision.

## Accountability

Authors are responsible for every line they submit, generated or not. The pull request
template has an "AI assistance" section; reviewers apply the same standards to all code.
