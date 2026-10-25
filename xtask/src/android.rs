//! Android build: `libkoebi.so` (cargo-ndk) + SDL's Java glue + APK (Gradle).
//!
//! SDL's `SDLActivity` checks that its Java sources match the native library version
//! exactly, so they are copied from the `sdl3-src` crate locked in `Cargo.lock` at every
//! build instead of being vendored.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use crate::util::{self, Result};

/// Minimum Android API level (Android 8.0): Vulkan 1.1+ drivers, `AAudio`, modern NDK.
const MIN_SDK: &str = "26";

pub(crate) struct Options {
    pub(crate) release: bool,
    pub(crate) abis: Vec<String>,
    pub(crate) gradle: bool,
}

/// Devices (`arm64-v8a`) and emulators on x86-64 hosts (`x86_64`).
pub(crate) fn default_abis() -> Vec<String> {
    vec!["arm64-v8a".into(), "x86_64".into()]
}

fn triple(abi: &str) -> Result<&'static str> {
    Ok(match abi {
        "arm64-v8a" => "aarch64-linux-android",
        "armeabi-v7a" => "armv7-linux-androideabi",
        "x86_64" => "x86_64-linux-android",
        "x86" => "i686-linux-android",
        other => return Err(format!("unknown Android ABI {other:?}").into()),
    })
}

pub(crate) fn run(options: &Options) -> Result<()> {
    if !util::has_cargo_subcommand("ndk") {
        return Err(
            "cargo-ndk is required: `cargo install cargo-ndk --locked`, and set ANDROID_NDK_HOME"
                .into(),
        );
    }
    let root = util::workspace_root();
    let project = root.join("platform").join("android");
    let profile = if options.release { "release" } else { "debug" };

    for abi in &options.abis {
        let triple = triple(abi)?;
        util::step(&format!("libkoebi.so for {abi} ({triple})"));
        let mut build = util::cargo();
        build.args([
            "ndk",
            "--target",
            abi,
            "--platform",
            MIN_SDK,
            "rustc",
            "--package",
            "koebi",
            "--lib",
            "--crate-type",
            "cdylib",
            "--locked",
        ]);
        if options.release {
            build.arg("--release");
        }
        // sdl3-sys builds SDL with CMake: route it through the NDK toolchain, with the
        // processor-name fix described in platform/android/cmake/android.toolchain.cmake.
        if std::env::var_os("TARGET_CMAKE_TOOLCHAIN_FILE").is_none() {
            let toolchain = project.join("cmake").join("android.toolchain.cmake");
            build.env(
                "TARGET_CMAKE_TOOLCHAIN_FILE",
                toolchain.to_string_lossy().replace('\\', "/"),
            );
        }
        // CMake's default generator on Windows (Visual Studio) cannot target Android.
        if cfg!(windows) && std::env::var_os("CMAKE_GENERATOR").is_none() {
            build.env("CMAKE_GENERATOR", "Ninja");
        }
        util::run(&mut build)?;

        let out = root.join("target").join(triple).join(profile);
        let jni = project
            .join("app")
            .join("src")
            .join("main")
            .join("jniLibs")
            .join(abi);
        fs::create_dir_all(&jni)?;
        let sdl = sdl_library(&out)
            .ok_or_else(|| format!("libSDL3.so not found under {}", out.display()))?;
        for (source, library) in [
            (out.join("libkoebi.so"), "libkoebi.so"),
            (sdl, "libSDL3.so"),
        ] {
            fs::copy(&source, jni.join(library))
                .map_err(|e| format!("cannot copy {}: {e}", source.display()))?;
        }
    }

    util::step("SDL Java sources");
    let sdl = util::registry_crate_dir("sdl3-src")?;
    let java = [
        "SDL",
        "android-project",
        "app",
        "src",
        "main",
        "java",
        "org",
        "libsdl",
        "app",
    ];
    let source = java.iter().fold(sdl, |path, part| path.join(part));
    let destination = project.join("app/src/generated/java/org/libsdl/app");
    if destination.exists() {
        fs::remove_dir_all(&destination)?;
    }
    util::copy_dir(&source, &destination)?;
    println!("copied {} -> {}", source.display(), destination.display());

    if options.gradle {
        util::step("Gradle");
        let gradlew = project.join(if cfg!(windows) {
            "gradlew.bat"
        } else {
            "gradlew"
        });
        let task = if options.release {
            "assembleRelease"
        } else {
            "assembleDebug"
        };
        util::run(Command::new(&gradlew).current_dir(&project).args([
            task,
            "--no-daemon",
            "--stacktrace",
        ]))?;
        println!(
            "APK in {}",
            project
                .join("app/build/outputs/apk")
                .join(profile)
                .display()
        );
    }
    Ok(())
}

/// The SDL library to package for a profile directory (`target/<triple>/<profile>`).
///
/// The sdl3-sys build script copies `libSDL3.so` next to the crate outputs, but only when it
/// runs: when the build outputs come from a cache, the build script is up to date and the copy
/// may be missing. The library `CMake` installed in the build script's output directory is then
/// used, the newest one if several builds exist.
fn sdl_library(profile_dir: &Path) -> Option<PathBuf> {
    let copy = profile_dir.join("libSDL3.so");
    if copy.is_file() {
        return Some(copy);
    }
    let mut found: Vec<(SystemTime, PathBuf)> = Vec::new();
    for entry in fs::read_dir(profile_dir.join("build")).ok()?.flatten() {
        if !entry.file_name().to_string_lossy().starts_with("sdl3-sys-") {
            continue;
        }
        for lib in ["lib", "lib64"] {
            let path = entry.path().join("out").join(lib).join("libSDL3.so");
            if let Ok(metadata) = fs::metadata(&path) {
                found.push((metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH), path));
            }
        }
    }
    found
        .into_iter()
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh profile directory, removed on drop.
    struct Profile(PathBuf);

    impl Profile {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("xtask-{}-{name}", std::process::id()));
            fs::remove_dir_all(&dir).ok();
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn file(&self, relative: &str) -> PathBuf {
            let path = self.0.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, b"elf").unwrap();
            path
        }
    }

    impl Drop for Profile {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).ok();
        }
    }

    #[test]
    fn sdl_library_is_found_when_the_build_script_did_not_run() {
        // Issue #51: with build outputs restored from a cache, only CMake's copy exists.
        let profile = Profile::new("cached");
        let installed = profile.file("build/sdl3-sys-0123abcd/out/lib/libSDL3.so");
        profile.file("build/other-crate-42/out/lib/libSDL3.so.txt");
        assert_eq!(sdl_library(&profile.0), Some(installed));
    }

    #[test]
    fn sdl_library_prefers_the_copy_next_to_the_crate_outputs() {
        let profile = Profile::new("fresh");
        profile.file("build/sdl3-sys-0123abcd/out/lib64/libSDL3.so");
        let copy = profile.file("libSDL3.so");
        assert_eq!(sdl_library(&profile.0), Some(copy));
    }

    #[test]
    fn sdl_library_is_absent_without_a_build() {
        let profile = Profile::new("empty");
        assert_eq!(sdl_library(&profile.0), None);
    }

    #[test]
    fn abis_map_to_rust_targets() {
        assert_eq!(triple("arm64-v8a").unwrap(), "aarch64-linux-android");
        assert_eq!(triple("x86_64").unwrap(), "x86_64-linux-android");
        assert!(triple("mips").is_err());
    }
}
