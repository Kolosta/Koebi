//! Gitflow branch naming and pull request targeting rules.
//!
//! The single source of truth used by the `pre-push` hook, the `PR policy` CI workflow and
//! the Claude Code skills. See `docs/development/gitflow.md`.

use crate::util::Result;
use crate::version::Version;

/// Kind of branch, derived from its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BranchKind {
    /// Released code; only release/hotfix merges land here.
    Main,
    /// Integration branch.
    Develop,
    /// `feature/<slug>`: new work, from and back to develop.
    Feature,
    /// `bugfix/<slug>`: non-urgent fix, from and back to develop.
    Bugfix,
    /// `release/X.Y.Z`: stabilization, from develop to main.
    Release(Version),
    /// `hotfix/X.Y.Z`: urgent fix, from main to main.
    Hotfix(Version),
    /// Branches opened by bots (Dependabot, Claude Code GitHub Action) targeting develop.
    Automation,
}

const NAMING: &str = "\
branch names must be one of:
  feature/<slug>   new work          (from develop, PR to develop)
  bugfix/<slug>    non-urgent fix    (from develop, PR to develop)
  release/X.Y.Z    release prep      (from develop, PR to main)
  hotfix/X.Y.Z     urgent fix        (from main,    PR to main)
where <slug> is lowercase: letters, digits, '.', '_', '-' (e.g. feature/42-rounded-rects)";

/// Classifies a branch name, rejecting names outside the convention.
pub(crate) fn classify(branch: &str) -> Result<BranchKind> {
    let invalid = || format!("invalid branch name {branch:?}\n{NAMING}");
    let kind = match branch.split_once('/') {
        None if branch == "main" => BranchKind::Main,
        None if branch == "develop" => BranchKind::Develop,
        Some(("feature", slug)) if is_slug(slug) => BranchKind::Feature,
        Some(("bugfix", slug)) if is_slug(slug) => BranchKind::Bugfix,
        Some(("release", version)) => BranchKind::Release(version.parse().map_err(|_| invalid())?),
        Some(("hotfix", version)) => BranchKind::Hotfix(version.parse().map_err(|_| invalid())?),
        Some(("dependabot" | "claude", rest)) if !rest.is_empty() => BranchKind::Automation,
        _ => return Err(invalid().into()),
    };
    Ok(kind)
}

fn is_slug(slug: &str) -> bool {
    let mut chars = slug.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
}

/// Checks that a pull request from `head` into `base` follows gitflow.
pub(crate) fn check_pull_request(head: &str, base: &str) -> Result<()> {
    let head_kind = classify(head)?;
    let expected = match head_kind {
        // `main` targets develop for the back-merge opened automatically after a release.
        BranchKind::Feature | BranchKind::Bugfix | BranchKind::Automation | BranchKind::Main => {
            "develop"
        }
        BranchKind::Release(_) | BranchKind::Hotfix(_) => "main",
        BranchKind::Develop => {
            return Err("develop is never merged directly: open a release/X.Y.Z branch".into());
        }
    };
    if base == expected {
        Ok(())
    } else {
        Err(format!("{head} must target {expected}, not {base}\n{NAMING}").into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_names_are_classified() {
        assert_eq!(classify("main").unwrap(), BranchKind::Main);
        assert_eq!(classify("develop").unwrap(), BranchKind::Develop);
        assert_eq!(
            classify("feature/42-rounded-rects").unwrap(),
            BranchKind::Feature
        );
        assert_eq!(
            classify("bugfix/swapchain.resize").unwrap(),
            BranchKind::Bugfix
        );
        assert_eq!(
            classify("release/1.2.0").unwrap(),
            BranchKind::Release(Version {
                major: 1,
                minor: 2,
                patch: 0
            })
        );
        assert!(matches!(
            classify("hotfix/1.2.1").unwrap(),
            BranchKind::Hotfix(_)
        ));
        assert_eq!(
            classify("dependabot/cargo/ash-0.39").unwrap(),
            BranchKind::Automation
        );
        assert_eq!(
            classify("claude/issue-12-20260101").unwrap(),
            BranchKind::Automation
        );
    }

    #[test]
    fn invalid_names_are_rejected_with_guidance() {
        for name in [
            "feat/x",
            "feature/",
            "feature/Upper",
            "feature/-x",
            "release/1.2",
            "fix",
            "hotfix/v1.0.0",
        ] {
            let error = classify(name).unwrap_err().to_string();
            assert!(error.contains("feature/<slug>"), "{name}: {error}");
        }
    }

    #[test]
    fn pull_request_targets_follow_gitflow() {
        assert!(check_pull_request("feature/1-a", "develop").is_ok());
        assert!(check_pull_request("bugfix/2-b", "develop").is_ok());
        assert!(check_pull_request("release/0.2.0", "main").is_ok());
        assert!(check_pull_request("hotfix/0.2.1", "main").is_ok());
        assert!(check_pull_request("main", "develop").is_ok());
        assert!(check_pull_request("dependabot/cargo/x", "develop").is_ok());

        assert!(check_pull_request("feature/1-a", "main").is_err());
        assert!(check_pull_request("release/0.2.0", "develop").is_err());
        assert!(check_pull_request("develop", "main").is_err());
    }
}
