//! Semantic versions and the workspace version bump.

use std::fmt;
use std::str::FromStr;

use crate::util::{self, Result};

/// A `MAJOR.MINOR.PATCH` version without pre-release or build metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Version {
    pub(crate) major: u64,
    pub(crate) minor: u64,
    pub(crate) patch: u64,
}

impl FromStr for Version {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        let parts: Vec<&str> = text.split('.').collect();
        let parse = |part: &str| {
            let valid = !part.is_empty()
                && part.chars().all(|c| c.is_ascii_digit())
                && (part == "0" || !part.starts_with('0'));
            valid.then(|| part.parse().ok()).flatten()
        };
        match parts.as_slice() {
            [major, minor, patch] => match (parse(major), parse(minor), parse(patch)) {
                (Some(major), Some(minor), Some(patch)) => Ok(Self {
                    major,
                    minor,
                    patch,
                }),
                _ => Err(format!("{text:?} is not MAJOR.MINOR.PATCH")),
            },
            _ => Err(format!("{text:?} is not MAJOR.MINOR.PATCH")),
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Reads `version` from the `[workspace.package]` table.
pub(crate) fn read_workspace_version(manifest: &str) -> Result<String> {
    let (_, line) = find_version_line(manifest)?;
    Ok(line
        .split('"')
        .nth(1)
        .ok_or("malformed version line")?
        .to_owned())
}

/// Returns `manifest` with the `[workspace.package]` version replaced.
pub(crate) fn replace_workspace_version(manifest: &str, version: Version) -> Result<String> {
    let (index, _) = find_version_line(manifest)?;
    let mut lines: Vec<String> = manifest.lines().map(str::to_owned).collect();
    lines[index] = format!("version = \"{version}\"");
    let mut updated = lines.join("\n");
    if manifest.ends_with('\n') {
        updated.push('\n');
    }
    Ok(updated)
}

fn find_version_line(manifest: &str) -> Result<(usize, &str)> {
    let mut in_table = false;
    for (index, line) in manifest.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_table = trimmed == "[workspace.package]";
        } else if in_table && trimmed.starts_with("version") && trimmed.contains('=') {
            return Ok((index, trimmed));
        }
    }
    Err("no `version` in [workspace.package]".into())
}

/// `cargo xtask version X.Y.Z`: updates `Cargo.toml` and `Cargo.lock`.
pub(crate) fn set(text: &str) -> Result<()> {
    let version: Version = text.parse()?;
    let path = util::workspace_root().join("Cargo.toml");
    let manifest = std::fs::read_to_string(&path)?;
    let current: Version = read_workspace_version(&manifest)?.parse()?;
    if version == current {
        // The first release ships the version the workspace already has.
        println!("Cargo.toml is already at {version}");
        return Ok(());
    }
    if version < current {
        return Err(format!("{version} must be greater than the current {current}").into());
    }
    std::fs::write(&path, replace_workspace_version(&manifest, version)?)?;
    util::run(util::cargo().args(["update", "--workspace"]))?;
    println!(
        "version {current} -> {version}\nnext: `git cliff --tag v{version} -o CHANGELOG.md`, \
         commit `chore(release): {version}`, `cargo xtask release-check --head <branch>`, \
         then open the pull request to main"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[package]\nversion = \"9.9.9\"\n";

    #[test]
    fn versions_parse_strictly() {
        assert_eq!(
            "1.20.3".parse(),
            Ok(Version {
                major: 1,
                minor: 20,
                patch: 3
            })
        );
        for invalid in ["1.2", "1.2.3.4", "01.2.3", "1.2.x", "v1.2.3", ""] {
            assert!(invalid.parse::<Version>().is_err(), "{invalid}");
        }
        assert!("0.10.0".parse::<Version>().unwrap() > "0.9.9".parse().unwrap());
    }

    #[test]
    fn only_the_workspace_version_is_replaced() {
        assert_eq!(read_workspace_version(MANIFEST).unwrap(), "0.1.0");
        let updated = replace_workspace_version(MANIFEST, "0.2.0".parse().unwrap()).unwrap();
        assert!(updated.contains("[workspace.package]\nversion = \"0.2.0\""));
        assert!(updated.contains("[package]\nversion = \"9.9.9\""));
        assert!(updated.ends_with('\n'));
    }
}
