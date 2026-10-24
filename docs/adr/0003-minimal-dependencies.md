# 0003. Minimal dependencies

- Status: Accepted
- Date: 2026-10-24
- Deciders: @Kolosta

## Context

Koebi is a low-level, performance-critical application. Every dependency adds build time,
binary size, supply-chain risk and abstractions we do not control. Rust makes adding crates
easy, so the policy must be explicit and enforced by tooling, for humans and AI agents alike.

## Decision

Runtime dependencies are limited to two crates:

| Crate | Why |
|---|---|
| `ash` | Generated Vulkan bindings, no runtime dependency (default features off) |
| `sdl3-sys` | Raw SDL3 bindings ([ADR 0002](0002-sdl3-platform-layer.md)) |

Their build-time dependencies are accepted: `sdl3-src`, `cmake`, `cc`, `find-msvc-tools`,
`shlex`, `rpkg-config`. Everything else is written in-house and kept small and tested: the
Vulkan loader (~100 lines), logging, color science, plain-old-data casts, the PNG codec used by
golden tests, and `xtask` (standard library only).

Enforcement:

- `deny.toml` lists every allowed crate (`[bans] allow`), denies duplicate versions, wildcards
  and unknown sources, and allows only MIT, Apache-2.0 and Zlib licenses. CI runs
  `cargo deny check` on every pull request.
- Adding a crate requires a new ADR, the maintainer's approval and a `deny.toml` update.
  `cargo add` is an "ask" permission for Claude Code, and `CLAUDE.md` forbids it.
- Developer tools (cargo-nextest, cargo-deny, typos, git-cliff, cargo-ndk) are installed as
  binaries and are not dependencies of the workspace.

## Consequences

- Small attack surface, fast clean builds, and complete control over hot paths.
- We maintain code that crates would otherwise provide; it must stay minimal and covered by
  tests.
- Some future needs (font parsing, image decoding) will either be written in-house or argued
  for in a new ADR.

## Alternatives considered

- **Use mainstream crates freely** (`libloading`, `log`, `bytemuck`, `png`, `thiserror`): each
  is good, but together they multiply the dependency graph for little code saved.
- **No dependency at all**: rewriting Vulkan or SDL bindings would be pure duplication; `ash`
  and `sdl3-sys` are thin, generated and dependency-free at runtime.
