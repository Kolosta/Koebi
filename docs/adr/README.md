# Architecture decision records

An ADR records one significant decision: its context, the decision, its consequences and the
alternatives that were rejected. Write one before adding a dependency, changing a public API
shape, choosing a rendering technique or changing the platform strategy (`/adr` in Claude Code
creates it from the [template](0000-template.md)).

ADRs are immutable once accepted. To change a decision, write a new ADR that supersedes the old
one and link both.

| ADR | Title | Status |
|---|---|---|
| [0001](0001-vulkan-only-rhi.md) | Vulkan-only render hardware interface | Accepted |
| [0002](0002-sdl3-platform-layer.md) | SDL3 platform layer with main callbacks | Accepted |
| [0003](0003-minimal-dependencies.md) | Minimal dependencies | Accepted |
| [0004](0004-color-pipeline.md) | Color pipeline: sRGB values, gamma-space blending, OKLCH tokens | Accepted, token item superseded by 0008 |
| [0005](0005-shader-toolchain.md) | Ahead-of-time GLSL to SPIR-V, committed binaries | Accepted |
| [0006](0006-testing-strategy.md) | Testing strategy: five levels, software GPUs in CI | Accepted |
| [0007](0007-gitflow-and-delivery.md) | Gitflow with automated delivery | Accepted |
| [0008](0008-design-token-system.md) | Design token system: typed tiers, compiled tables, layered resolution | Accepted |
| [0009](0009-design-state-persistence.md) | Persistence of the design system state: a versioned binary file keyed by paths | Accepted |
| [0010](0010-design-token-interchange.md) | Design token interchange: the W3C Design Tokens format with a Koebi extension | Accepted |
