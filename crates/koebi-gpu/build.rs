//! Links MoltenVK statically on iOS, where no dynamic Vulkan loader exists.
//!
//! The static `MoltenVK.xcframework` ships with the LunarG Vulkan SDK for macOS (iOS
//! component). Set `KOEBI_MOLTENVK_XCFRAMEWORK` to use another copy. Other targets need
//! nothing here: the Vulkan loader is opened at runtime (see `src/loader.rs`).

use std::env;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=KOEBI_MOLTENVK_XCFRAMEWORK");
    println!("cargo::rerun-if-env-changed=VULKAN_SDK");

    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "ios" {
        return;
    }

    // Rust moved the simulator marker from `target_abi` to `target_env` in 1.91.
    let simulator = env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|v| v == "sim")
        || env::var("CARGO_CFG_TARGET_ABI").is_ok_and(|v| v == "sim")
        || env::var("TARGET").is_ok_and(|t| t.ends_with("-sim") || t.starts_with("x86_64"));

    let Some(xcframework) = find_xcframework() else {
        println!(
            "cargo::error=MoltenVK.xcframework not found. Install the Vulkan SDK for macOS with \
             its iOS component and export VULKAN_SDK, or set KOEBI_MOLTENVK_XCFRAMEWORK."
        );
        return;
    };
    let Some(slice) = find_slice(&xcframework, simulator) else {
        println!(
            "cargo::error=no {} slice with libMoltenVK.a in {}",
            if simulator {
                "iOS simulator"
            } else {
                "iOS device"
            },
            xcframework.display()
        );
        return;
    };

    println!("cargo::rustc-link-search=native={}", slice.display());
    println!("cargo::rustc-link-lib=static=MoltenVK");
    println!("cargo::rustc-link-lib=c++");
    for framework in [
        "Foundation",
        "Metal",
        "QuartzCore",
        "IOSurface",
        "UIKit",
        "CoreGraphics",
    ] {
        println!("cargo::rustc-link-lib=framework={framework}");
    }
}

fn find_xcframework() -> Option<PathBuf> {
    if let Some(path) = env::var_os("KOEBI_MOLTENVK_XCFRAMEWORK") {
        return Some(PathBuf::from(path));
    }
    // VULKAN_SDK points at `<sdk>/<version>/macOS`; MoltenVK lives next to it.
    let sdk = PathBuf::from(env::var_os("VULKAN_SDK")?);
    let root = sdk.parent()?.to_path_buf();
    [
        root.join("iOS/lib/MoltenVK.xcframework"),
        root.join("MoltenVK/MoltenVK.xcframework"),
        root.join("MoltenVK/static/MoltenVK.xcframework"),
        sdk.join("lib/MoltenVK.xcframework"),
    ]
    .into_iter()
    .find(|path| path.is_dir())
}

fn find_slice(xcframework: &Path, simulator: bool) -> Option<PathBuf> {
    // Slices are named like `ios-arm64` and `ios-arm64_x86_64-simulator`.
    std::fs::read_dir(xcframework)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|path| {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            name.starts_with("ios-")
                && name.ends_with("-simulator") == simulator
                && path.join("libMoltenVK.a").is_file()
        })
}
