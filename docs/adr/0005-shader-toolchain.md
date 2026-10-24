# 0005. Ahead-of-time GLSL to SPIR-V, committed binaries

- Status: Accepted
- Date: 2026-10-24
- Deciders: @Kolosta

## Context

Vulkan consumes SPIR-V. Compiling shaders at runtime needs a compiler library (shaderc, naga),
which contradicts [ADR 0003](0003-minimal-dependencies.md) and is impractical on mobile.
Compiling in `build.rs` would require every contributor and CI job to install a shader
compiler. The bytes that ship must be reproducible and reviewed.

## Decision

- Shaders are written in GLSL 4.50 under `crates/koebi-render/shaders/`.
- `cargo xtask shaders` compiles them with `glslc` from the Vulkan SDK
  (`--target-env=vulkan1.2 -O`) into `shaders/spirv/*.spv`, which are committed.
- The crate embeds them with `koebi_gpu::include_spirv!`, which guarantees 4-byte alignment
  and checks the SPIR-V magic number at compile time.
- The Vulkan SDK version is pinned in `.github/vulkan-sdk-version`. The CI `shaders` job
  recompiles with that version and fails if a committed `.spv` is stale or orphaned
  (`cargo xtask shaders --check`).

## Consequences

- Building and running Koebi needs no shader compiler; only shader authors need the SDK.
- A GLSL change must be followed by `cargo xtask shaders` and a commit of the `.spv` files;
  CI catches omissions.
- Different `glslc` versions may emit different bytes: use the pinned SDK version locally.

## Alternatives considered

- **shaderc or naga at build or run time**: extra dependencies, C++ builds, slower builds.
- **WGSL**: a different language and toolchain for no gain on a Vulkan-only renderer.
- **Slang**: attractive for larger shader code bases and ships with the Vulkan SDK; it still
  outputs SPIR-V, so it can be adopted later through a new ADR without changing the runtime.
