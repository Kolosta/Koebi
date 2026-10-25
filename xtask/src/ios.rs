//! iOS build: the `koebi` binary wrapped in an ad-hoc signed `Koebi.app`.
//!
//! SDL is linked statically and MoltenVK too (see `crates/koebi-gpu/build.rs`), so the bundle
//! is just the executable and its `Info.plist`. Device builds need a real signing identity
//! and provisioning profile before installation (see `docs/development/platforms.md`).

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::util::{self, Result};

const BUNDLE_ID: &str = "io.github.kolosta.koebi";
const DEPLOYMENT_TARGET: &str = "15.0";
const SMOKE_FRAMES: &str = "120";

pub(crate) struct Options {
    pub(crate) release: bool,
    pub(crate) device: bool,
    pub(crate) run: bool,
}

pub(crate) fn run(options: &Options) -> Result<()> {
    if !cfg!(target_os = "macos") {
        return Err("iOS builds require macOS with Xcode".into());
    }
    let triple = match (options.device, cfg!(target_arch = "aarch64")) {
        (true, _) => "aarch64-apple-ios",
        (false, true) => "aarch64-apple-ios-sim",
        (false, false) => "x86_64-apple-ios",
    };
    let profile = if options.release { "release" } else { "debug" };
    let root = util::workspace_root();

    util::step(&format!("koebi for {triple}"));
    let mut build = util::cargo();
    build
        .args([
            "build",
            "--package",
            "koebi",
            "--bin",
            "koebi",
            "--target",
            triple,
            "--locked",
        ])
        .env("IPHONEOS_DEPLOYMENT_TARGET", DEPLOYMENT_TARGET);
    if options.release {
        build.arg("--release");
    }
    util::run(&mut build)?;

    util::step("Koebi.app");
    let out = root.join("target").join(triple).join(profile);
    let app = out.join("Koebi.app");
    if app.exists() {
        fs::remove_dir_all(&app)?;
    }
    fs::create_dir_all(&app)?;
    fs::copy(out.join("koebi"), app.join("koebi"))?;
    let build_number = std::env::var("GITHUB_RUN_NUMBER").unwrap_or_else(|_| "1".into());
    let plist = fs::read_to_string(root.join("platform/ios/Info.plist"))?
        .replace("$(KOEBI_VERSION)", &util::workspace_version()?)
        .replace("$(KOEBI_BUILD)", &build_number)
        .replace("$(KOEBI_BUNDLE_ID)", BUNDLE_ID);
    fs::write(app.join("Info.plist"), plist)?;
    util::run(
        Command::new("codesign")
            .args(["--force", "--sign", "-", "--timestamp=none"])
            .arg(&app),
    )?;
    println!("app bundle: {}", app.display());

    if options.run {
        if options.device {
            return Err("--run only supports the simulator".into());
        }
        run_in_simulator(&app)?;
    }
    Ok(())
}

fn run_in_simulator(app: &Path) -> Result<()> {
    let list =
        util::output(Command::new("xcrun").args(["simctl", "list", "devices", "available"]))?;
    let udid =
        pick_simulator(&list).ok_or("no available iPhone simulator (install one in Xcode)")?;
    util::step(&format!("simulator {udid}"));
    // Booting an already booted simulator fails harmlessly.
    if util::run(Command::new("xcrun").args(["simctl", "boot", &udid])).is_err() {
        println!("(simulator already booted)");
    }
    util::run(Command::new("xcrun").args(["simctl", "bootstatus", &udid, "-b"]))?;
    util::run(
        Command::new("xcrun")
            .args(["simctl", "install", &udid])
            .arg(app),
    )?;
    let log = util::output(
        Command::new("xcrun")
            .args([
                "simctl",
                "launch",
                "--console-pty",
                "--terminate-running-process",
                &udid,
                BUNDLE_ID,
            ])
            .args(["--frames", SMOKE_FRAMES])
            .env("SIMCTL_CHILD_KOEBI_LOG", "info")
            .env("SIMCTL_CHILD_MTL_DEBUG_LAYER", "1")
            .env("SIMCTL_CHILD_MVK_CONFIG_LOG_LEVEL", "4"),
    )?;
    println!("{log}");
    let marker = format!("presented {SMOKE_FRAMES} frames");
    if log.contains(&marker) {
        Ok(())
    } else {
        Err(format!("the app did not report `{marker}`").into())
    }
}

/// Picks a booted iPhone simulator, else the first available one, from
/// `xcrun simctl list devices available` output.
fn pick_simulator(list: &str) -> Option<String> {
    let iphones: Vec<&str> = list
        .lines()
        .filter(|line| line.trim_start().starts_with("iPhone"))
        .collect();
    let line = iphones
        .iter()
        .find(|line| line.contains("(Booted)"))
        .or_else(|| iphones.first())?;
    line.split(['(', ')'])
        .map(str::trim)
        .find(|part| part.len() == 36 && part.chars().filter(|c| *c == '-').count() == 4)
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = "\
== Devices ==
-- iOS 26.0 --
    iPhone 17 (11111111-2222-3333-4444-555555555555) (Shutdown)
    iPhone 17 Pro (AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE) (Booted)
    iPad Air (99999999-8888-7777-6666-555555555555) (Shutdown)";

    #[test]
    fn booted_iphone_is_preferred() {
        assert_eq!(
            pick_simulator(LIST).as_deref(),
            Some("AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE")
        );
    }

    #[test]
    fn first_iphone_is_used_when_none_is_booted() {
        let list = LIST.replace("(Booted)", "(Shutdown)");
        assert_eq!(
            pick_simulator(&list).as_deref(),
            Some("11111111-2222-3333-4444-555555555555")
        );
        assert_eq!(pick_simulator("== Devices =="), None);
    }
}
