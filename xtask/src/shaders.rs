//! GLSL → SPIR-V compilation with the Vulkan SDK's `glslc`.
//!
//! SPIR-V binaries are committed so that building Koebi needs no shader compiler. CI runs
//! `--check` with the pinned SDK (`.github/vulkan-sdk-version`): `glslc` output is
//! deterministic for a given version, so any difference means the binaries are stale.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::util::{self, Result};

/// Directories holding GLSL sources; binaries go to `<dir>/spirv/<file>.spv`.
const SHADER_DIRS: [&str; 1] = ["crates/koebi-render/shaders"];
const STAGES: [&str; 3] = ["vert", "frag", "comp"];
const TARGET_ENV: &str = "vulkan1.2";

pub(crate) fn run(check: bool) -> Result<()> {
    let glslc = util::find_tool("glslc").ok_or(
        "glslc not found: install the Vulkan SDK version pinned in .github/vulkan-sdk-version",
    )?;
    let spirv_val = util::find_tool("spirv-val");
    let version = util::output(Command::new(&glslc).arg("--version"))?;
    println!("using {}", version.lines().next().unwrap_or("glslc"));

    let root = util::workspace_root();
    let scratch = root.join("target").join("xtask-shaders");
    fs::create_dir_all(&scratch)?;
    let mut problems: Vec<String> = Vec::new();
    let mut compiled_count = 0;

    for dir in SHADER_DIRS.map(|d| root.join(d)) {
        let out_dir = dir.join("spirv");
        let mut sources: Vec<PathBuf> = fs::read_dir(&dir)?
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| STAGES.contains(&e))
            })
            .collect();
        sources.sort();

        for source in &sources {
            let file = source
                .file_name()
                .and_then(|f| f.to_str())
                .ok_or("bad file name")?;
            let name = format!("{file}.spv");
            let compiled = scratch.join(&name);
            util::run(
                Command::new(&glslc)
                    .arg(format!("--target-env={TARGET_ENV}"))
                    .arg("-O")
                    .arg("-o")
                    .arg(&compiled)
                    .arg(source),
            )?;
            if let Some(validator) = &spirv_val {
                util::run(
                    Command::new(validator)
                        .args(["--target-env", TARGET_ENV])
                        .arg(&compiled),
                )?;
            }
            let committed = out_dir.join(&name);
            if check {
                if fs::read(&committed).ok() != Some(fs::read(&compiled)?) {
                    problems.push(format!("{} is stale", committed.display()));
                }
            } else {
                fs::create_dir_all(&out_dir)?;
                fs::copy(&compiled, &committed)?;
            }
            compiled_count += 1;
        }

        // Binaries whose source was deleted or renamed.
        if out_dir.is_dir() {
            for entry in fs::read_dir(&out_dir)? {
                let path = entry?.path();
                let stem = path
                    .file_name()
                    .and_then(|f| f.to_str())
                    .and_then(|f| f.strip_suffix(".spv"));
                if stem.is_some_and(|stem| !dir.join(stem).is_file()) {
                    if check {
                        problems.push(format!("{} has no source", path.display()));
                    } else {
                        fs::remove_file(&path)?;
                    }
                }
            }
        }
    }

    if problems.is_empty() {
        println!("{compiled_count} shader(s) OK");
        Ok(())
    } else {
        Err(format!(
            "SPIR-V out of date:\n  {}\nrun `cargo xtask shaders` with the pinned Vulkan SDK and commit the result",
            problems.join("\n  ")
        )
        .into())
    }
}
