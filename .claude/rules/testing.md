---
paths:
  - "**/tests/**"
  - "crates/koebi-testkit/**"
---

# Tests

- Pick the cheapest level that proves the behaviour: unit > integration > functional
  (pixels) > regression (golden) > end-to-end (binary). See `docs/development/testing.md`.
- GPU tests: `let Some(gpu) = koebi_testkit::gpu_or_skip("test_name") else { return };`, then
  end with `gpu.assert_no_validation_errors();`. They skip without a GPU locally but fail in
  CI (`KOEBI_REQUIRE_GPU=1`); never add another skip mechanism.
- Functional tests assert analytically computable pixels with a tolerance of at most 1/255,
  so they hold on every driver (lavapipe, NVIDIA, AMD, Intel, MoltenVK).
- Golden tests: small images (≤ 320×240), `assert_golden!(image, "name", tolerance)`, files in
  `tests/golden/`. Update with `cargo xtask bless`, never by hand; review PNG diffs in the PR.
- Bug fixes: first a test that fails for the reported reason, named after the behaviour (and
  issue number in a comment), then the fix.
- Deterministic tests only: no sleeps or wall-clock assertions (except e2e timeouts), no
  network, no dependence on test order or global state shared between tests.
- Shared helpers for a test crate go in `tests/common/mod.rs`; cross-crate helpers go in
  `koebi-testkit`.
