//! Golden-image regression testing.

use std::path::Path;

use crate::image::Image;

/// How much an image may deviate from its golden file.
///
/// GPU rasterization is deterministic per driver, but anti-aliased edges may differ by a few
/// units between vendors (lavapipe, NVIDIA, MoltenVK...). The default ignores channel
/// deltas up to 2/255 and allows no pixel beyond that.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tolerance {
    /// Channel differences up to this value are ignored.
    pub max_channel_delta: u8,
    /// Number of pixels allowed to exceed `max_channel_delta`.
    pub max_differing_pixels: usize,
}

impl Tolerance {
    /// Bit-exact comparison.
    pub const EXACT: Self = Self {
        max_channel_delta: 0,
        max_differing_pixels: 0,
    };

    /// Creates a tolerance.
    pub const fn new(max_channel_delta: u8, max_differing_pixels: usize) -> Self {
        Self {
            max_channel_delta,
            max_differing_pixels,
        }
    }
}

impl Default for Tolerance {
    fn default() -> Self {
        Self::new(2, 0)
    }
}

fn bless_requested() -> bool {
    std::env::var("KOEBI_BLESS")
        .is_ok_and(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true"))
}

/// Compares `actual` with `<golden_dir>/<name>.png`. Prefer the
/// [`assert_golden!`](crate::assert_golden) macro, which fills in the directories.
///
/// # Panics
///
/// Panics when the golden file is missing or unreadable, or the difference exceeds
/// `tolerance`.
pub fn assert_golden_at(
    golden_dir: &Path,
    failure_dir: &Path,
    name: &str,
    actual: &Image,
    tolerance: Tolerance,
) {
    let golden_path = golden_dir.join(format!("{name}.png"));
    if bless_requested() {
        std::fs::create_dir_all(golden_dir)
            .unwrap_or_else(|e| panic!("cannot create {}: {e}", golden_dir.display()));
        std::fs::write(&golden_path, actual.to_png())
            .unwrap_or_else(|e| panic!("cannot write {}: {e}", golden_path.display()));
        eprintln!("blessed golden image {}", golden_path.display());
        return;
    }

    let bytes = std::fs::read(&golden_path).unwrap_or_else(|e| {
        panic!(
            "golden image {} is missing ({e}); run `cargo xtask bless` to create it",
            golden_path.display()
        )
    });
    let expected = Image::from_png(&bytes)
        .unwrap_or_else(|e| panic!("cannot decode {}: {e}", golden_path.display()));

    let save_failure = |diff: Option<&Image>| {
        if std::fs::create_dir_all(failure_dir).is_ok() {
            let _ignored = std::fs::write(
                failure_dir.join(format!("{name}.actual.png")),
                actual.to_png(),
            );
            let _ignored = std::fs::write(
                failure_dir.join(format!("{name}.expected.png")),
                expected.to_png(),
            );
            if let Some(diff) = diff {
                let _ignored =
                    std::fs::write(failure_dir.join(format!("{name}.diff.png")), diff.to_png());
            }
        }
    };

    if (actual.width(), actual.height()) != (expected.width(), expected.height()) {
        save_failure(None);
        panic!(
            "golden {name}: size {}x{} differs from expected {}x{}",
            actual.width(),
            actual.height(),
            expected.width(),
            expected.height()
        );
    }

    let diff = actual.diff(&expected, tolerance.max_channel_delta);
    if diff.differing_pixels > tolerance.max_differing_pixels {
        save_failure(Some(&diff.visualization));
        panic!(
            "golden {name}: {} pixel(s) differ by more than {} (max delta {}); images saved to {}",
            diff.differing_pixels,
            tolerance.max_channel_delta,
            diff.max_channel_delta,
            failure_dir.display()
        );
    }
}
