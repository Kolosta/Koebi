//! Process, path and tool helpers.

use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Result type of every xtask command.
pub(crate) type Result<T, E = Box<dyn Error>> = std::result::Result<T, E>;

/// Root of the Cargo workspace.
pub(crate) fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap_or(manifest_dir).to_path_buf()
}

/// `cargo`, as invoked by `cargo xtask` (respects rustup's toolchain selection).
pub(crate) fn cargo() -> Command {
    let mut command = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command.current_dir(workspace_root());
    command
}

/// Prints a section title.
pub(crate) fn step(title: &str) {
    println!("\n==> {title}");
}

/// Runs a command, streaming its output, and fails on a non-zero exit status.
pub(crate) fn run(command: &mut Command) -> Result<()> {
    println!("$ {}", describe(command));
    let status = command
        .status()
        .map_err(|e| format!("cannot run {}: {e}", describe(command)))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("`{}` failed with {status}", describe(command)).into())
    }
}

/// Runs a command and returns its standard output.
pub(crate) fn output(command: &mut Command) -> Result<String> {
    let result = command
        .output()
        .map_err(|e| format!("cannot run {}: {e}", describe(command)))?;
    if result.status.success() {
        Ok(String::from_utf8_lossy(&result.stdout).into_owned())
    } else {
        Err(format!(
            "`{}` failed with {}:\n{}",
            describe(command),
            result.status,
            String::from_utf8_lossy(&result.stderr)
        )
        .into())
    }
}

fn describe(command: &Command) -> String {
    let program = command.get_program().to_string_lossy();
    let args: Vec<_> = command.get_args().map(OsStr::to_string_lossy).collect();
    format!("{program} {}", args.join(" ")).trim().to_owned()
}

/// Finds an executable in the Vulkan SDK (`$VULKAN_SDK/bin`) or on `PATH`.
pub(crate) fn find_tool(name: &str) -> Option<PathBuf> {
    let file = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    let mut dirs = Vec::new();
    if let Some(sdk) = env::var_os("VULKAN_SDK") {
        dirs.push(PathBuf::from(&sdk).join("bin"));
        dirs.push(PathBuf::from(&sdk).join("Bin"));
    }
    if let Some(path) = env::var_os("PATH") {
        dirs.extend(env::split_paths(&path));
    }
    dirs.into_iter()
        .map(|dir| dir.join(&file))
        .find(|candidate| candidate.is_file())
}

/// `true` if `cargo <subcommand> --version` works (e.g. `nextest`, `deny`, `ndk`).
pub(crate) fn has_cargo_subcommand(subcommand: &str) -> bool {
    cargo()
        .args([subcommand, "--version"])
        .output()
        .is_ok_and(|output| output.status.success())
}

/// The workspace version from the root `Cargo.toml`.
pub(crate) fn workspace_version() -> Result<String> {
    let manifest = std::fs::read_to_string(workspace_root().join("Cargo.toml"))?;
    crate::version::read_workspace_version(&manifest)
}

/// Locked version of a package, read from `Cargo.lock`.
pub(crate) fn locked_version(package: &str) -> Result<String> {
    let lock = std::fs::read_to_string(workspace_root().join("Cargo.lock"))?;
    let marker = format!("name = \"{package}\"");
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line.trim() == marker {
            let version = lines
                .next()
                .and_then(|l| l.trim().strip_prefix("version = \""));
            if let Some(version) = version.and_then(|v| v.strip_suffix('"')) {
                return Ok(version.to_owned());
            }
        }
    }
    Err(format!("{package} not found in Cargo.lock").into())
}

/// Source directory of a registry crate (downloads it if needed).
pub(crate) fn registry_crate_dir(package: &str) -> Result<PathBuf> {
    let version = locked_version(package)?;
    run(cargo().args(["fetch", "--locked"]))?;
    let cargo_home = env::var_os("CARGO_HOME").map(PathBuf::from).or_else(|| {
        env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .map(|home| PathBuf::from(home).join(".cargo"))
    });
    let registry = cargo_home
        .ok_or("cannot locate CARGO_HOME")?
        .join("registry")
        .join("src");
    let wanted = format!("{package}-{version}");
    std::fs::read_dir(&registry)?
        .filter_map(std::result::Result::ok)
        .map(|index| index.path().join(&wanted))
        .find(|dir| dir.is_dir())
        .ok_or_else(|| format!("{wanted} not found under {}", registry.display()).into())
}

/// Recursively copies `from` into `to` (which is created).
pub(crate) fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Points git at the versioned hooks in `.githooks/`.
pub(crate) fn install_hooks() -> Result<()> {
    run(Command::new("git").current_dir(workspace_root()).args([
        "config",
        "core.hooksPath",
        ".githooks",
    ]))?;
    println!(
        "git hooks installed: pre-commit (format), commit-msg (Conventional Commits), pre-push (gitflow)"
    );
    Ok(())
}
