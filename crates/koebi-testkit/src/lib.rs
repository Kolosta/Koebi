//! Test infrastructure shared by Koebi's integration, functional and regression tests.
//!
//! - [`gpu_or_skip`]: a headless Vulkan device with validation layers, or a skip when the
//!   machine has no usable GPU. In CI, `KOEBI_REQUIRE_GPU=1` turns the skip into a failure
//!   so missing GPU coverage never goes unnoticed.
//! - [`HeadlessGpu::render_offscreen`]: renders into a texture and reads the pixels back.
//! - [`Image`] and [`assert_golden!`]: golden-image regression tests stored as PNG files,
//!   which GitHub renders side by side in pull request diffs.
//!
//! Only ever use this crate from `[dev-dependencies]`.

#![allow(
    clippy::panic,
    reason = "assertion helpers report failures by panicking"
)]
#![allow(
    clippy::print_stderr,
    reason = "skips and blessings are reported on stderr"
)]

mod golden;
mod gpu;
mod image;
mod png;

pub use golden::{Tolerance, assert_golden_at};
pub use gpu::{HeadlessGpu, gpu_or_skip, require_gpu};
pub use image::{Image, ImageDiff};
pub use png::{PngError, decode_png, encode_png};

/// Compares an image with `tests/golden/<name>.png` of the calling crate.
///
/// Set `KOEBI_BLESS=1` (or run `cargo xtask bless`) to create or update golden files after an
/// intended visual change. On mismatch, the actual image and a diff visualization are written
/// to `target/tmp/golden-failures/` (uploaded as CI artifacts) and the test fails.
///
/// Usable from integration tests only (relies on `CARGO_TARGET_TMPDIR`).
#[macro_export]
macro_rules! assert_golden {
    ($image:expr, $name:expr) => {
        $crate::assert_golden!($image, $name, $crate::Tolerance::default())
    };
    ($image:expr, $name:expr, $tolerance:expr) => {
        $crate::assert_golden_at(
            &::std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("golden"),
            &::std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("golden-failures"),
            $name,
            &$image,
            $tolerance,
        )
    };
}
