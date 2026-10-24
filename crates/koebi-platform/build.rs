//! Links clang's runtime builtins into iOS builds.
//!
//! SDL's `UIKit` code uses `@available` checks, which compile to calls to
//! `__isPlatformVersionAtLeast` from clang's compiler-rt (`libclang_rt.ios.a`, or
//! `libclang_rt.iossim.a` for the simulator). rustc links Apple targets with `-nodefaultlibs`,
//! so the statically linked SDL needs that archive explicitly. Other targets need nothing.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    // iOS builds only link on macOS hosts; elsewhere (cross-checks) there is nothing to find.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("ios") || !cfg!(target_os = "macos") {
        return;
    }
    // Rust moved the simulator marker from `target_abi` to `target_env` in 1.91.
    let simulator = env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|v| v == "sim")
        || env::var("CARGO_CFG_TARGET_ABI").is_ok_and(|v| v == "sim")
        || env::var("TARGET").is_ok_and(|t| t.ends_with("-sim") || t.starts_with("x86_64"));
    let runtime = if simulator { "iossim" } else { "ios" };

    let resource_dir = Command::new("xcrun")
        .args(["clang", "--print-resource-dir"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()));
    let Some(darwin) = resource_dir.map(|dir| dir.join("lib").join("darwin")) else {
        println!(
            "cargo::error=cannot locate clang's resource directory (`xcrun clang --print-resource-dir`)"
        );
        return;
    };
    if !darwin.join(format!("libclang_rt.{runtime}.a")).is_file() {
        println!(
            "cargo::error=libclang_rt.{runtime}.a not found in {}",
            darwin.display()
        );
        return;
    }

    println!("cargo::rustc-link-search=native={}", darwin.display());
    // `-bundle`: the archive is universal (several architectures), which rustc cannot copy
    // into an rlib; the final link reads it directly.
    println!("cargo::rustc-link-lib=static:-bundle=clang_rt.{runtime}");
}
