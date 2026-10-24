# Testing

Five levels, from cheapest to most expensive ([ADR 0006](../adr/0006-testing-strategy.md)).
Pick the cheapest level that proves the behaviour.

| Level | Where | Proves |
|---|---|---|
| Unit | `#[cfg(test)] mod tests` beside the code | pure logic: math, color, parsing, policies |
| Integration | `crates/<crate>/tests/*.rs` | public APIs, including a headless Vulkan device |
| Functional | `crates/koebi-render/tests/functional.rs` | rendered pixels match analytic expectations |
| Regression | `crates/koebi-render/tests/regression.rs` + `tests/golden/*.png` | complex renders match reviewed images |
| End-to-end | `apps/koebi/tests/e2e.rs` | the real binary opens a window, renders, exits cleanly |

Beside tests, CI checks formatting, clippy, rustdoc, dependencies, spelling and SPIR-V
freshness, and runs Miri nightly on `koebi-core`.

## Running tests

```sh
cargo nextest run --workspace        # or: cargo test --workspace
cargo test --workspace --doc         # doctests (nextest does not run them)
cargo nextest run -p koebi-render    # one crate
KOEBI_REQUIRE_GPU=1 KOEBI_VALIDATION=required cargo nextest run --workspace   # as CI does
```

`cargo xtask ci` runs everything CI runs.

## GPU tests

GPU tests use `koebi-testkit`:

```rust
#[test]
fn clear_color_fills_the_target() {
    let Some(gpu) = koebi_testkit::gpu_or_skip("clear_color_fills_the_target") else {
        return;
    };
    let image = gpu
        .render_offscreen(extent, TextureFormat::Rgba8Unorm, |encoder, target| {
            // record commands into `encoder`, drawing into `target`
        })
        .unwrap();
    assert_eq!(image.pixel(0, 0), [17, 19, 24, 255]);
    gpu.assert_no_validation_errors();
}
```

- Without a Vulkan driver the test prints a skip notice and passes, except when
  `KOEBI_REQUIRE_GPU=1` (CI), where it fails.
- Validation layers are enabled when available; `KOEBI_VALIDATION=required` makes their absence
  an error. Any validation error fails `assert_no_validation_errors`.
- On Android and iOS, GPU tests skip unless `KOEBI_REQUIRE_GPU=1` is forwarded.
- Use `KOEBI_ADAPTER=llvmpipe` (or another name) to force a specific GPU.

## Functional tests

Assert pixels whose value can be computed by hand: solid fills, edges on whole pixels, known
blend results. Allow at most 1/255 difference per channel so tests hold on every driver
(lavapipe, MoltenVK, NVIDIA, AMD, Intel).

## Golden images

A golden image is a reviewed reference render, committed as a PNG. A regression test renders
the same scene offscreen, reads the pixels back and compares them with the reference. Unit and
functional tests check what can be computed by hand; golden tests catch everything else that
can change silently: anti-aliasing, blending, color conversion, scaling, draw order.

```rust
koebi_testkit::assert_golden!(image, "showcase", Tolerance::new(3, 8));
```

The renderer has two references today, described in
[`crates/koebi-render/tests/golden/README.md`](../../crates/koebi-render/tests/golden/README.md):
the same "showcase" scene at scale 1 (`showcase.png`) and at scale 2 (`showcase_2x.png`).

Golden tests render scenes written for the test, not the application's screen: changing what
the application shows does not break them, and they do not check its look. They break only
when the renderer draws the same scene differently.

- References live in `tests/golden/` of the crate; keep images small (at most 320x240).
- `Tolerance::new(max_channel_delta, max_differing_pixels)` absorbs anti-aliasing differences
  between rasterizers: channel differences up to `max_channel_delta` are ignored, and at most
  `max_differing_pixels` pixels may exceed it. Keep it tight.
- On failure, the actual, expected and difference images are written to
  `target/tmp/golden-failures/`; CI uploads them as an artifact.
- After an intended visual change, regenerate and review:

```sh
cargo xtask bless
git diff --stat -- '*.png'
```

Never edit reference images by hand. Reviewers compare the PNGs in the pull request.

## End-to-end tests

`apps/koebi/tests/e2e.rs` launches the built binary with `--frames N` and checks the exit code
and log output. Exit codes: 0 success, 1 failure, 2 unsupported environment, 3 validation
errors, 64 invalid command line. On Linux CI the window opens on Xvfb.

## Mobile

- Android: CI builds every test binary for `x86_64-linux-android`, runs them on an emulator
  through `adb`, then installs the APK and checks that it renders 120 frames
  (`.github/scripts/android-emulator.sh`).
- iOS: CI runs `cargo test --target aarch64-apple-ios-sim` inside a booted simulator
  (`.github/scripts/ios-simulator-runner.sh`), then launches `Koebi.app` with
  `cargo xtask ios --run`.

## Writing good tests

- Name tests `<subject>_<behaviour>`: `rect_contains_is_min_inclusive_max_exclusive`.
- A bug fix starts with a test that fails for the reported reason.
- Deterministic only: no sleeps, no network, no order dependence, no shared mutable state.
- Shared helpers: `tests/common/mod.rs` inside a crate, `koebi-testkit` across crates.
