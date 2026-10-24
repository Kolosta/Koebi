//! Android build: `libkoebi.so` (cargo-ndk) + SDL's Java glue + APK (Gradle).
//!
//! SDL's `SDLActivity` checks that its Java sources match the native library version
//! exactly, so they are copied from the `sdl3-src` crate locked in `Cargo.lock` at every
//! build instead of being vendored.

use std::fs;
use std::process::Command;

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
        util::run(&mut build)?;

        // sdl3-sys copies libSDL3.so next to the crate outputs.
        let out = root.join("target").join(triple).join(profile);
        let jni = project
            .join("app")
            .join("src")
            .join("main")
            .join("jniLibs")
            .join(abi);
        fs::create_dir_all(&jni)?;
        for library in ["libkoebi.so", "libSDL3.so"] {
            fs::copy(out.join(library), jni.join(library))
                .map_err(|e| format!("cannot copy {library} from {}: {e}", out.display()))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abis_map_to_rust_targets() {
        assert_eq!(triple("arm64-v8a").unwrap(), "aarch64-linux-android");
        assert_eq!(triple("x86_64").unwrap(), "x86_64-linux-android");
        assert!(triple("mips").is_err());
    }
}
