# Koebi

[![CI](https://github.com/Kolosta/Koebi/actions/workflows/ci.yml/badge.svg?branch=develop)](https://github.com/Kolosta/Koebi/actions/workflows/ci.yml)

Koebi is a cross-platform application written in Rust that draws its entire interface with its
own Vulkan renderer: widgets, canvases and compositing, with precise control over colors and
design tokens. SDL3 provides windows, input and the application lifecycle.

| Windows | Linux | macOS | Android | iOS |
|---|---|---|---|---|
| Vulkan | Vulkan | MoltenVK | Vulkan | MoltenVK |

<img src="crates/koebi-render/tests/golden/showcase_2x.png" alt="Anti-aliased rounded rectangles rendered by koebi-render" width="320">

*A golden test image: SDF rounded rectangles with borders and premultiplied alpha blending.*

## Status

Early foundation. A window opens and the renderer draws anti-aliased rounded rectangles on all
five platforms, with CI running unit, integration, pixel, golden-image and end-to-end tests on
each of them. Text, layout and widgets come next.

## Design

- **One rendering layer for everything**, like Blender's GPU module, but Vulkan-only and with
  zero-cost static dispatch ([why](docs/architecture/gpu-abstraction.md)).
- **Two runtime dependencies**: `ash` (Vulkan bindings) and `sdl3-sys` (SDL3 bindings).
  Everything else is written in-house ([ADR 0003](docs/adr/0003-minimal-dependencies.md)).
- **Colors as designers expect them**: sRGB values, gamma-space blending, OKLCH design tokens,
  WCAG contrast checks ([ADR 0004](docs/adr/0004-color-pipeline.md)).
- **Zero idle cost**: the app renders only when something changes.

```text
apps/koebi/            application (composition root)
crates/koebi-core/     geometry, color science, logging
crates/koebi-gpu/      Vulkan render hardware interface
crates/koebi-render/   2D renderer (instanced SDF primitives)
crates/koebi-platform/ SDL3 platform layer
crates/koebi-testkit/  headless GPU and golden-image test tools
xtask/                 development automation (cargo xtask)
platform/              Android project, Apple bundle templates
```

## Quick start

Install Rust ([rustup](https://rustup.rs)), a C compiler, CMake and the
[Vulkan SDK](https://vulkan.lunarg.com/sdk/home) (details per OS in
[setup.md](docs/development/setup.md)), then:

```sh
git clone https://github.com/Kolosta/Koebi.git
cd Koebi
cargo xtask install-hooks
cargo run -p koebi
```

Common commands:

```sh
cargo xtask ci                  # format, clippy, tests, docs, shaders, dependencies, spelling
cargo nextest run --workspace   # tests (or cargo test --workspace)
cargo xtask android             # Android APK (docs/development/platforms.md)
cargo xtask ios --run           # iOS simulator
cargo xtask help                # everything else
```

## Documentation

- [Architecture overview](docs/architecture/overview.md) and
  [decision records](docs/adr/README.md)
- [Contributing](CONTRIBUTING.md), [gitflow](docs/development/gitflow.md),
  [coding standards](docs/development/coding-standards.md), [testing](docs/development/testing.md)
- [CI/CD](docs/development/ci-cd.md) and [platforms](docs/development/platforms.md)
- [Working with Claude Code](docs/ai/claude-code.md)

## License

No license has been chosen yet: until a `LICENSE` file is added, all rights are reserved by the
author.
