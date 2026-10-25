# Koebi — project memory for Claude Code

<!-- Keep this file under ~150 lines: it is loaded into every session. Put area-specific
     guidance in .claude/rules/ (loaded only for matching files) and procedures in
     .claude/skills/ (loaded on demand). Block HTML comments like this one cost no tokens. -->

Koebi is a cross-platform (Windows, Linux, macOS, Android, iOS) Rust application that draws
its entire UI with its own Vulkan renderer. SDL3 only provides windows, input, lifecycle and the
Vulkan surface. Performance, precise color control and a clean architecture are the priorities.

## Non-negotiable rules

1. **Dependencies**: runtime crates are `ash` and `sdl3-sys`, nothing else. Never run `cargo add`
   or edit `[workspace.dependencies]` on your own: a new crate needs an ADR (`/adr`) and the
   maintainer's approval, then an entry in `deny.toml` (CI rejects any unlisted crate).
2. **Git**: never commit on `main`/`develop`, never push to `main`, never force-push. Work on a
   gitflow branch and open a pull request (`/feature`, `/pr`). Commits and PR titles follow
   Conventional Commits. Humans author every commit: no `Co-Authored-By` trailer, no
   "Generated with" line or session link in commits, PRs, issues or release notes.
3. **Done means verified**: run `/check` (or `cargo xtask ci`) before saying a task is finished.
   New behaviour ships with tests at the right level (see Testing); a bug fix starts with a
   failing regression test.
4. **Unsafe**: allowed only in `koebi-gpu`, `koebi-platform` and FFI glue. Every `unsafe` block
   has a `// SAFETY:` comment stating why its invariants hold. Safe public APIs must make
   undefined behaviour impossible.
5. **Errors, not panics**: library code returns `Result` with its crate's `Error` type.
   `unwrap()` only in tests; `expect("why this cannot fail")` only for real invariants.
6. **Hot paths**: no allocation per frame (reuse buffers, `Scene::clear`), no hidden GPU waits
   (`wait_idle`, `submit_and_wait`) outside init, resize and teardown.
7. **Language**: answer the user in the language they write in; write code, comments, docs,
   commits and PRs in English.

## Repository map

```text
apps/koebi/            the application (composition root); e2e tests in tests/
crates/koebi-core/     geometry, color science (sRGB, linear, OKLab/OKLCH, WCAG, color vision), input
                       vocabulary (keys, modifiers, buttons), logging; no deps
crates/koebi-design/   design token engine: typed tiers, token tables, themes, overrides, views
crates/koebi-tokens/   Koebi's design tokens (primitive, semantic, component) and built-in themes
crates/koebi-ui/       components drawn from typed token styles, immediate-mode interaction
crates/koebi-gpu/      Vulkan RHI: loader, instance, device, memory, resources, pipelines, presenter
crates/koebi-render/   2D renderer: Scene -> instanced draws; GLSL in shaders/, SPIR-V committed
crates/koebi-platform/ SDL3 layer: App trait (main callbacks), windows, events, Vulkan surface
crates/koebi-testkit/  dev-only: headless GPU, offscreen readback, golden PNG images
xtask/                 automation, std-only (`cargo xtask help`)
platform/              Android Gradle project, iOS/macOS Info.plist templates
docs/                  architecture, ADRs, development guides, AI workflow
```

Layering (enforced by Cargo): `app -> ui -> render -> gpu -> core`, `app -> platform -> core`
and `ui -> tokens -> design -> core`.
`render`, `gpu` and `ui` never see SDL; `platform` never sees the renderer; `core` depends on
nothing.

## Commands

```sh
cargo run -p koebi                  # run (add `-- --frames 120` to exit after 120 frames)
cargo xtask ci                      # local CI gate: fmt, clippy, tests, docs, shaders, deny, typos
cargo nextest run --workspace       # all tests (or cargo test --workspace)
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask shaders                 # GLSL -> SPIR-V (Vulkan SDK glslc); commit the .spv files
cargo xtask bless                   # regenerate golden images after an intended visual change
cargo xtask android | ios --run     # mobile builds (docs/development/platforms.md)
```

Useful environment: `KOEBI_LOG=debug`, `KOEBI_VALIDATION=1|required`, `KOEBI_ADAPTER=<name>`,
`KOEBI_REQUIRE_GPU=1` (GPU tests fail instead of skipping, as in CI).

## Git workflow (gitflow)

- `main`: released code only, tagged `vX.Y.Z`. `develop`: integration, the default branch.
- `feature/<issue>-<slug>`, `bugfix/<issue>-<slug>`: from develop, PR to develop, squash merge.
- `release/X.Y.Z`: from develop (`cargo xtask version X.Y.Z`), PR to main, merge commit.
  `hotfix/X.Y.Z`: from main, PR to main. CD then tags, publishes and opens the main -> develop
  back-merge PR automatically.
- Skills: `/feature`, `/pr`, `/release`, `/hotfix`. Rules: `cargo xtask gitflow-check`.

## Testing (pick the right level)

- **Unit**: `#[cfg(test)] mod tests` beside the code (math, color, parsing, policies).
- **Integration**: `crates/<crate>/tests/*.rs`, public API only; GPU tests start with
  `let Some(gpu) = koebi_testkit::gpu_or_skip("name") else { return };` and end with
  `gpu.assert_no_validation_errors();`.
- **Functional**: render offscreen and assert exact pixels (`crates/koebi-render/tests/functional.rs`).
- **Regression**: golden PNGs with `assert_golden!` (`tests/golden/`, update with `cargo xtask bless`).
- **End-to-end**: launch the real binary (`apps/koebi/tests/e2e.rs`).

## Conventions (details in .claude/rules/ and docs/development/coding-standards.md)

- Rust 2024, toolchain pinned by `rust-toolchain.toml`; clippy `pedantic` with `-D warnings`;
  `#[allow(lint, reason = "...")]` always carries a reason. A hook runs `cargo fmt` after edits.
- RAII everywhere: GPU objects are destroyed by `Drop`, through the device's deferred-deletion
  queue when the GPU may still use them. Never call `vkDestroy*` by hand outside `koebi-gpu`.
- Every public item has a doc comment; every module starts with `//!` explaining its role.
- Log with `koebi_core::{error,warn,info,debug,trace}!`, never `println!` in libraries.
- Colors are `koebi_core::Color` (gamma-encoded sRGB, straight alpha). UI values come from design
  tokens (`koebi-design`, ADR 0008): components read component tokens, never raw literals.

## Working efficiently (tokens and context)

- Read the module you need, not whole crates; use the LSP tool (rust-analyzer plugin) for
  definitions and references instead of grepping.
- Delegate broad searches to the Explore agent and reviews to `rust-reviewer` or
  `vulkan-auditor`; their verbose output stays out of the main context.
- `/check` and `/ci-debug` run in a forked context and return only a summary.
- Start unrelated tasks with `/clear`; read `docs/` only when a task touches that area.

# Compact instructions

When compacting, keep: the task goal and acceptance criteria, decisions and their reasons,
files changed, failing test names with their key error lines, and open TODOs. Drop: full build
logs, file listings and file contents that can be re-read.
