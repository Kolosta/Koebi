# Contributing to Koebi

Thank you for your interest. This page is the short version; each section links to the full
guide.

## Before you start

- Read the [architecture overview](docs/architecture/overview.md) and the
  [coding standards](docs/development/coding-standards.md).
- For anything larger than a small fix, open an issue first (templates: bug, feature, task).
  Architecture changes and new dependencies need an [ADR](docs/adr/README.md).
- Set up your machine with [setup.md](docs/development/setup.md), then install the git hooks:

```sh
cargo xtask install-hooks
```

## Finding work

- The [project board](https://github.com/Kolosta/Koebi/projects) shows the roadmap: each open
  issue with its status (Todo, In Progress, Done), priority, area and size.
- [Milestones](https://github.com/Kolosta/Koebi/milestones) group issues by release:
  `v0.2.0 Text`, `v0.3.0 UI toolkit`, and so on. Issues without a milestone form the backlog.
- Good starting points carry the `good first issue` or `help wanted` label. Issues labeled
  `status: needs-adr` start with an [ADR](docs/adr/README.md) pull request.
- Comment on an issue to claim it before starting, so that nobody duplicates the work.

## Workflow

Koebi uses [gitflow](docs/development/gitflow.md):

1. Branch from `develop`, named after the issue: `feature/<issue>-<slug>` or
   `bugfix/<issue>-<slug>`. Move the issue to "In Progress" on the board.
2. Commit with [Conventional Commits](https://www.conventionalcommits.org):
   `feat(render): add rounded rectangle borders`.
3. Run the local gate before pushing:

   ```sh
   cargo xtask ci
   ```

4. Open a pull request into `develop`, early as a draft if you like, and fill in the
   template with `Closes #<issue>`. Its title must be a Conventional Commit: it becomes the
   squash commit and the changelog entry.
5. `CI result` and `PR policy result` must be green, and review conversations resolved. The
   merge closes the issue, which moves to "Done".

Never push to `main` or `develop`; the rulesets reject it anyway.

## What reviewers look for

- **Tests at the right level** ([testing.md](docs/development/testing.md)). Bug fixes start
  with a test that reproduces the bug. Visual changes include regenerated golden images
  (`cargo xtask bless`) for review.
- **No new dependency** without an accepted ADR ([ADR 0003](docs/adr/0003-minimal-dependencies.md)).
- **Sound unsafe code**: every `unsafe` block has a `// SAFETY:` comment, and safe APIs cannot
  cause undefined behaviour.
- **No per-frame allocation or GPU wait** in hot paths.
- **Documentation**: public items documented, `docs/` updated when behaviour or architecture
  changes.

## AI-assisted contributions

AI tools are welcome. The repository ships a Claude Code configuration that applies the same
rules ([claude-code.md](docs/ai/claude-code.md)). You remain responsible for every line you
submit: read, understand and test generated code, and mention the tools used in the pull
request.

## Reporting security issues

Do not open public issues for vulnerabilities; see [SECURITY.md](SECURITY.md).

## Conduct

Participation is governed by the [code of conduct](CODE_OF_CONDUCT.md).
