---
name: add-dependency
description: Evaluate whether a new crate (runtime, build or dev) is justified under Koebi's minimal-dependency policy, and prepare the ADR. Use whenever a task seems to need a crate that is not already in the workspace.
argument-hint: "<crate-name> <why it is needed>"
---

Evaluate the dependency request: $ARGUMENTS

Koebi's policy (docs/adr/0003-minimal-dependencies.md): runtime crates are `ash` and
`sdl3-sys` only. Do not run `cargo add` and do not edit `Cargo.toml` or `deny.toml` until the
maintainer explicitly approves.

1. First try to avoid it: can the need be met with `std`, an existing crate, or ~200 lines of
   our own code (as done for the Vulkan loader, logging and PNG codec)? If yes, propose that.
2. Otherwise assess the crate and present a table:
   - purpose and the exact API we would use;
   - license (must be MIT, Apache-2.0 or Zlib to pass cargo-deny);
   - transitive dependencies (`cargo tree -p <crate>` in a scratch project), build time and
     binary size impact;
   - `unsafe` usage, maintenance activity, MSRV, platform support (Android, iOS included);
   - runtime or build-time only.
3. Recommend accept or reject with reasons. If accepting, draft the ADR with `/adr` and list
   the exact changes: `[workspace.dependencies]` entry, the crate's `Cargo.toml`, and every new
   crate name to add to `[bans] allow` in `deny.toml`.
