//! Desktop release archives in `target/dist/`.
//!
//! - Windows: `koebi-<version>-<target>.zip` (statically linked SDL: one executable);
//! - Linux: `koebi-<version>-<target>.tar.gz`;
//! - macOS: `Koebi.app` with MoltenVK bundled in `Contents/Frameworks` (SDL loads it first),
//!   zipped. `--universal` builds an `arm64` + `x86_64` binary.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::util::{self, Result};

pub(crate) fn run(target: Option<&str>, universal: bool) -> Result<()> {
    let version = util::workspace_version()?;
    let root = util::workspace_root();
    let dist = root.join("target").join("dist");
    fs::create_dir_all(&dist)?;

    if universal {
        if !cfg!(target_os = "macos") {
            return Err("--universal is only available on macOS".into());
        }
        let arm = build("aarch64-apple-darwin")?;
        let intel = build("x86_64-apple-darwin")?;
        let binary = dist.join("koebi-universal");
        util::run(
            Command::new("lipo")
                .arg("-create")
                .arg("-output")
                .arg(&binary)
                .arg(arm)
                .arg(intel),
        )?;
        return package_macos(&binary, &version, "universal-apple-darwin", &dist);
    }

    let target = match target {
        Some(target) => target.to_owned(),
        None => host_triple()?,
    };
    let binary = build(&target)?;
    let name = format!("koebi-{version}-{target}");
    if target.contains("apple-darwin") {
        package_macos(&binary, &version, &target, &dist)
    } else {
        let stage = dist.join(&name);
        if stage.exists() {
            fs::remove_dir_all(&stage)?;
        }
        fs::create_dir_all(&stage)?;
        fs::copy(
            &binary,
            stage.join(binary.file_name().ok_or("binary has no name")?),
        )?;
        for doc in ["README.md", "CHANGELOG.md"] {
            fs::copy(root.join(doc), stage.join(doc))?;
        }
        let archive = if target.contains("windows") {
            let zip = dist.join(format!("{name}.zip"));
            let script = format!(
                "Compress-Archive -Path '{}' -DestinationPath '{}' -Force",
                stage.display(),
                zip.display()
            );
            util::run(Command::new("powershell").args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &script,
            ]))?;
            zip
        } else {
            let tarball = dist.join(format!("{name}.tar.gz"));
            util::run(
                Command::new("tar")
                    .arg("-czf")
                    .arg(&tarball)
                    .arg("-C")
                    .arg(&dist)
                    .arg(&name),
            )?;
            tarball
        };
        println!("archive: {}", archive.display());
        Ok(())
    }
}

/// Builds the release binary for `target` and returns its path.
fn build(target: &str) -> Result<PathBuf> {
    util::step(&format!("release build for {target}"));
    util::run(util::cargo().args([
        "build",
        "--release",
        "--locked",
        "--package",
        "koebi",
        "--bin",
        "koebi",
        "--target",
        target,
    ]))?;
    let file = if target.contains("windows") {
        "koebi.exe"
    } else {
        "koebi"
    };
    Ok(util::workspace_root()
        .join("target")
        .join(target)
        .join("release")
        .join(file))
}

fn host_triple() -> Result<String> {
    let info = util::output(Command::new("rustc").arg("-vV"))?;
    info.lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .ok_or_else(|| "cannot determine the host target".into())
}

fn package_macos(binary: &Path, version: &str, target: &str, dist: &Path) -> Result<()> {
    util::step("Koebi.app");
    let moltenvk = std::env::var_os("VULKAN_SDK")
        .map(|sdk| PathBuf::from(sdk).join("lib").join("libMoltenVK.dylib"))
        .filter(|path| path.is_file())
        .ok_or("libMoltenVK.dylib not found: install the Vulkan SDK and export VULKAN_SDK")?;
    let app = dist.join("Koebi.app");
    if app.exists() {
        fs::remove_dir_all(&app)?;
    }
    let contents = app.join("Contents");
    for dir in ["MacOS", "Frameworks", "Resources"] {
        fs::create_dir_all(contents.join(dir))?;
    }
    fs::copy(binary, contents.join("MacOS").join("koebi"))?;
    fs::copy(
        &moltenvk,
        contents.join("Frameworks").join("libMoltenVK.dylib"),
    )?;
    let plist = fs::read_to_string(util::workspace_root().join("platform/macos/Info.plist"))?
        .replace("$(KOEBI_VERSION)", version);
    fs::write(contents.join("Info.plist"), plist)?;
    util::run(
        Command::new("codesign")
            .args(["--force", "--deep", "--sign", "-"])
            .arg(&app),
    )?;

    let zip = dist.join(format!("koebi-{version}-{target}.zip"));
    util::run(
        Command::new("ditto")
            .args(["-c", "-k", "--keepParent"])
            .arg(&app)
            .arg(&zip),
    )?;
    println!("archive: {}", zip.display());
    Ok(())
}
