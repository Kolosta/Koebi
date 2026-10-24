# 0006. Testing strategy: five levels, software GPUs in CI

- Status: Accepted
- Date: 2026-10-24
- Deciders: @Kolosta

## Context

GPU code fails in ways unit tests do not see: synchronization hazards, invalid API usage,
driver differences, visual regressions. CI runners have no discrete GPU, and Koebi targets five
platforms. Tests must be fast enough to run on every pull request and must never pass by
silently skipping.

## Decision

Five test levels, each with a clear purpose (details in
[testing.md](../development/testing.md)):

1. **Unit**: pure logic beside the code.
2. **Integration**: public APIs through `crates/<crate>/tests/`, including a headless Vulkan
   device.
3. **Functional**: render offscreen and compare pixels with analytic expectations.
4. **Regression**: compare offscreen renders with committed golden PNG images, with a small
   tolerance for rasterizer differences.
5. **End-to-end**: launch the real binary with a real window for a fixed number of frames.

GPU environment:

- CI renders with real Vulkan drivers: Mesa lavapipe on Linux and Windows, MoltenVK on macOS.
- Validation layers with synchronization validation are required in CI
  (`KOEBI_VALIDATION=required`); any validation error fails the test or the application exit
  code.
- GPU tests skip locally when no Vulkan driver exists, but CI sets `KOEBI_REQUIRE_GPU=1` so
  they fail instead of skipping.
- On Android and iOS, unit and integration tests run inside the emulator and the simulator;
  GPU tests are skipped there by default, and an application smoke test checks that the app
  presents frames without validation errors.

Clean-code checks run beside the tests: rustfmt, clippy pedantic with `-D warnings`, rustdoc
with `-D warnings`, cargo-deny, typos, SPIR-V freshness, and Miri nightly on `koebi-core`.

## Consequences

- Rendering changes are verified pixel by pixel on three different Vulkan implementations.
- Golden images must be regenerated (`cargo xtask bless`) and reviewed after intended visual
  changes.
- Software rasterizers do not reveal every hardware driver bug; testing on real devices stays
  a manual step before releases.

## Alternatives considered

- **Mocking Vulkan**: tests the mock, not the driver interaction.
- **Self-hosted GPU runners**: real hardware, but maintenance and security costs too high now.
- **SwiftShader instead of lavapipe**: also viable; lavapipe ships in Ubuntu packages and has
  maintained Windows builds, which keeps CI setup simple.
