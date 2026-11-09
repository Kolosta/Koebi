//! The application's icon, `koebi_icons::app::ICON`: drawn once at start-up for the windows (task
//! bars, window switchers, title bars, the dock on macOS), and at every size the platforms' icon
//! files need (`platform/`, kept up to date by a test).

use koebi_icons::app;
use koebi_platform::{IconImage, Window};
use koebi_svg::raster::{Colors, render};

/// The sizes given to windows, the largest first: the system picks the one that fits.
const WINDOW_SIZES: [u32; 7] = [256, 128, 64, 48, 32, 24, 16];

/// The icon drawn at the sizes windows show it at.
#[derive(Debug, Default)]
pub(crate) struct AppIcon {
    images: Vec<(u32, Vec<u8>)>,
}

impl AppIcon {
    /// Draws the icon, on platforms whose windows show one.
    pub(crate) fn new() -> Self {
        if cfg!(any(target_os = "android", target_os = "ios")) {
            return Self::default();
        }
        Self {
            images: WINDOW_SIZES.map(|side| (side, pixels(side))).into(),
        }
    }

    /// Gives `window` the icon.
    pub(crate) fn apply(&self, window: &Window) {
        let images: Vec<IconImage<'_>> = self
            .images
            .iter()
            .map(|(side, pixels)| IconImage {
                side: *side,
                pixels,
            })
            .collect();
        if let Err(error) = window.set_icon(&images) {
            koebi_core::warn!("the window keeps the system's icon: {error}");
        }
    }
}

/// The icon as straight-alpha RGBA pixels, a square of `side`, in its own colors.
pub(crate) fn pixels(side: u32) -> Vec<u8> {
    render(&app::ICON.compile(), side, Colors::Original)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_is_drawn_at_every_size_windows_show() {
        let icon = AppIcon::new();
        if cfg!(any(target_os = "android", target_os = "ios")) {
            assert_eq!(icon.images.len(), 0);
            return;
        }
        assert_eq!(icon.images.len(), WINDOW_SIZES.len());
        for (side, pixels) in &icon.images {
            assert_eq!(pixels.len(), (side * side * 4) as usize);
            // The middle of the logo is its white inside: opaque.
            let middle = ((side / 2 * side + side / 2) * 4) as usize;
            assert_eq!(pixels[middle + 3], 255, "{side}");
            // Its corners are empty.
            assert_eq!(pixels[3], 0, "{side}");
        }
    }

    /// The workspace's root.
    fn root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// The icon as a PNG of `side`, drawn once per size.
    fn png(side: u32, cache: &mut std::collections::HashMap<u32, Vec<u8>>) -> Vec<u8> {
        cache
            .entry(side)
            .or_insert_with(|| koebi_testkit::encode_png(side, side, &pixels(side)))
            .clone()
    }

    /// The icon at 80 % on an opaque square of `side` in the window's background (dark theme), as
    /// a PNG: iOS shows transparent pixels of app icons black.
    fn opaque_png(side: u32) -> Vec<u8> {
        let design = koebi_design::DesignSystem::new(&koebi_tokens::KOEBI).unwrap();
        let background: koebi_core::Color =
            design.get(koebi_tokens::component::WINDOW_BACKGROUND_COLOR_DEFAULT);
        let inner = (side * 4).div_ceil(5);
        let logo = pixels(inner);
        let offset = (side - inner) / 2;
        let back = background.to_rgba8();
        let mut out = Vec::with_capacity((side * side * 4) as usize);
        for y in 0..side {
            for x in 0..side {
                let inside =
                    (offset..offset + inner).contains(&x) && (offset..offset + inner).contains(&y);
                let over = if inside {
                    let at = (((y - offset) * inner + x - offset) * 4) as usize;
                    [logo[at], logo[at + 1], logo[at + 2], logo[at + 3]]
                } else {
                    [0; 4]
                };
                let alpha = u32::from(over[3]);
                for channel in 0..3 {
                    let mixed = (u32::from(over[channel]) * alpha
                        + u32::from(back[channel]) * (255 - alpha)
                        + 127)
                        / 255;
                    out.push(u8::try_from(mixed).unwrap());
                }
                out.push(255);
            }
        }
        koebi_testkit::encode_png(side, side, &out)
    }

    /// An ICO file of PNG images (Windows Vista and later read them at every size).
    fn ico(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let count = u16::try_from(images.len()).unwrap();
        let mut out = Vec::new();
        out.extend_from_slice(&[0, 0, 1, 0]);
        out.extend_from_slice(&count.to_le_bytes());
        let mut offset = 6 + 16 * images.len();
        for (side, png) in images {
            // 256 is written 0.
            let byte = u8::try_from(*side % 256).unwrap();
            out.extend_from_slice(&[byte, byte, 0, 0, 1, 0, 32, 0]);
            out.extend_from_slice(&u32::try_from(png.len()).unwrap().to_le_bytes());
            out.extend_from_slice(&u32::try_from(offset).unwrap().to_le_bytes());
            offset += png.len();
        }
        for (_, png) in images {
            out.extend_from_slice(png);
        }
        out
    }

    /// An ICNS file of PNG images, each under its type.
    fn icns(images: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
        let mut body = Vec::new();
        for (kind, png) in images {
            body.extend_from_slice(kind);
            body.extend_from_slice(&u32::try_from(png.len() + 8).unwrap().to_be_bytes());
            body.extend_from_slice(png);
        }
        let mut out = b"icns".to_vec();
        out.extend_from_slice(&u32::try_from(body.len() + 8).unwrap().to_be_bytes());
        out.extend_from_slice(&body);
        out
    }

    /// The platforms' icon files, from the workspace's root, as the icon makes them.
    fn platform_files() -> Vec<(&'static str, Vec<u8>)> {
        let mut cache = std::collections::HashMap::new();
        let mut files = Vec::new();
        let windows = [256, 128, 64, 48, 32, 24, 16].map(|side| (side, png(side, &mut cache)));
        files.push(("platform/windows/koebi.ico", ico(&windows)));
        let macos = [
            (*b"icp4", 16),
            (*b"icp5", 32),
            (*b"icp6", 64),
            (*b"ic07", 128),
            (*b"ic08", 256),
            (*b"ic09", 512),
            (*b"ic10", 1024),
            (*b"ic11", 32),
            (*b"ic12", 64),
            (*b"ic13", 256),
            (*b"ic14", 512),
        ]
        .map(|(kind, side)| (kind, png(side, &mut cache)));
        files.push(("platform/macos/Koebi.icns", icns(&macos)));
        for (path, side) in [
            ("platform/ios/AppIcon60x60@2x.png", 120),
            ("platform/ios/AppIcon60x60@3x.png", 180),
            ("platform/ios/AppIcon76x76@2x~ipad.png", 152),
            ("platform/ios/AppIcon83.5x83.5@2x~ipad.png", 167),
        ] {
            files.push((path, opaque_png(side)));
        }
        for (path, side) in [
            (
                "platform/android/app/src/main/res/mipmap-mdpi/ic_launcher.png",
                48,
            ),
            (
                "platform/android/app/src/main/res/mipmap-hdpi/ic_launcher.png",
                72,
            ),
            (
                "platform/android/app/src/main/res/mipmap-xhdpi/ic_launcher.png",
                96,
            ),
            (
                "platform/android/app/src/main/res/mipmap-xxhdpi/ic_launcher.png",
                144,
            ),
            (
                "platform/android/app/src/main/res/mipmap-xxxhdpi/ic_launcher.png",
                192,
            ),
        ] {
            files.push((path, png(side, &mut cache)));
        }
        files
    }

    /// The images of a PNG, ICO or ICNS file, decoded: `(width, height, rgba)` each.
    fn images_of(bytes: &[u8]) -> Vec<(u32, u32, Vec<u8>)> {
        let decode = |png: &[u8]| koebi_testkit::decode_png(png).unwrap();
        if bytes.starts_with(b"icns") {
            let mut images = Vec::new();
            let mut at = 8;
            while at < bytes.len() {
                let len = u32::from_be_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
                images.push(decode(&bytes[at + 8..at + len]));
                at += len;
            }
            images
        } else if bytes.starts_with(&[0, 0, 1, 0]) {
            let count = usize::from(u16::from_le_bytes([bytes[4], bytes[5]]));
            (0..count)
                .map(|index| {
                    let entry = &bytes[6 + 16 * index..6 + 16 * (index + 1)];
                    let len = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
                    let offset = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
                    decode(&bytes[offset..offset + len])
                })
                .collect()
        } else {
            vec![decode(bytes)]
        }
    }

    #[test]
    fn the_platforms_icon_files_are_drawn_from_the_icon() {
        let bless = std::env::var("KOEBI_BLESS").is_ok_and(|value| value == "1");
        for (path, made) in platform_files() {
            let file = root().join(path);
            if bless {
                std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                std::fs::write(&file, &made).unwrap();
                continue;
            }
            let committed = std::fs::read(&file)
                .unwrap_or_else(|_| panic!("{path} is missing: run with KOEBI_BLESS=1"));
            let (expected, found) = (images_of(&made), images_of(&committed));
            assert_eq!(
                expected.len(),
                found.len(),
                "{path}: run with KOEBI_BLESS=1"
            );
            for ((width, height, pixels), (w, h, found)) in expected.iter().zip(&found) {
                assert_eq!((width, height), (w, h), "{path}: run with KOEBI_BLESS=1");
                // Within float rounding of the platform's sine and cosine.
                let close = pixels.iter().zip(found).all(|(a, b)| a.abs_diff(*b) <= 2);
                assert!(
                    close,
                    "{path} differs from the icon: run with KOEBI_BLESS=1"
                );
            }
        }
    }
}
