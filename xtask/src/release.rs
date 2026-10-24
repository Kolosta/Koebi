//! Release versions derived from Conventional Commits, and release pull request checks.
//!
//! Koebi follows Semantic Versioning. The next version is computed from the commits since the
//! last `vX.Y.Z` tag: a breaking change bumps the major version (the minor one before 1.0),
//! a `feat` bumps the minor version, anything else the patch version. Tags are annotated and
//! created only by the Release workflow. See `docs/development/gitflow.md` (Versioning).

use std::process::Command;

use crate::commits;
use crate::gitflow::{self, BranchKind};
use crate::util::{self, Result};
use crate::version::Version;

/// How far a set of commits requires the version to move.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Bump {
    Patch,
    Minor,
    Major,
}

impl Bump {
    fn name(self) -> &'static str {
        match self {
            Self::Patch => "patch",
            Self::Minor => "minor",
            Self::Major => "major",
        }
    }
}

/// A commit's message: its header line and the rest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Commit {
    pub(crate) header: String,
    pub(crate) body: String,
}

impl Commit {
    /// The bump this commit requires, `None` outside the convention (merge commits...).
    fn bump(&self) -> Option<Bump> {
        let (prefix, _) = self.header.split_once(": ")?;
        let kind = prefix.split(['(', '!']).next().unwrap_or(prefix);
        if !commits::TYPES.contains(&kind) {
            return None;
        }
        let breaking_footer = self.body.lines().any(|line| {
            line.starts_with("BREAKING CHANGE:") || line.starts_with("BREAKING-CHANGE:")
        });
        Some(if prefix.ends_with('!') || breaking_footer {
            Bump::Major
        } else if kind == "feat" {
            Bump::Minor
        } else {
            Bump::Patch
        })
    }
}

/// The largest bump required by `commits`, `None` if no commit follows the convention.
pub(crate) fn required_bump(commits: &[Commit]) -> Option<Bump> {
    commits.iter().filter_map(Commit::bump).max()
}

/// `version` moved by `bump`. Before 1.0, a breaking change bumps the minor version.
pub(crate) fn bumped(version: Version, bump: Bump) -> Version {
    match (bump, version.major) {
        (Bump::Major, 0) | (Bump::Minor, _) => Version {
            minor: version.minor + 1,
            patch: 0,
            ..version
        },
        (Bump::Major, _) => Version {
            major: version.major + 1,
            minor: 0,
            patch: 0,
        },
        (Bump::Patch, _) => Version {
            patch: version.patch + 1,
            ..version
        },
    }
}

/// Parses `git log --format=%s%x1f%b%x1e` output.
fn parse_log(log: &str) -> Vec<Commit> {
    log.split('\x1e')
        .map(|record| record.trim_start_matches(['\r', '\n']))
        .filter(|record| !record.trim().is_empty())
        .map(|record| {
            let (header, body) = record.split_once('\x1f').unwrap_or((record, ""));
            Commit {
                header: header.trim().to_owned(),
                body: body.trim().to_owned(),
            }
        })
        .collect()
}

fn git(args: &[&str]) -> Result<String> {
    util::output(
        Command::new("git")
            .current_dir(util::workspace_root())
            .args(args),
    )
}

/// The highest `vX.Y.Z` tag reachable from `HEAD`.
fn latest_tag() -> Result<Option<Version>> {
    let tags = git(&["tag", "--merged", "HEAD", "--list", "v*"])?;
    Ok(tags
        .lines()
        .filter_map(|tag| tag.trim().strip_prefix('v'))
        .filter_map(|version| version.parse().ok())
        .max())
}

/// Commits after tag `v<since>`, or every commit without a tag.
fn commits_since(since: Option<Version>) -> Result<Vec<Commit>> {
    let range = since.map(|version| format!("v{version}..HEAD"));
    let mut args = vec!["log", "--format=%s%x1f%b%x1e"];
    if let Some(range) = &range {
        args.push(range);
    }
    Ok(parse_log(&git(&args)?))
}

/// The next version from the workspace version (first release) or the commits since the
/// last tag; also returns a human-readable reason.
fn compute_next(
    workspace: Version,
    latest: Option<Version>,
    commits: &[Commit],
) -> Result<(Version, String)> {
    let Some(tag) = latest else {
        return Ok((
            workspace,
            format!("first release: the workspace version {workspace}"),
        ));
    };
    let bump = required_bump(commits)
        .ok_or_else(|| format!("no Conventional Commit since v{tag}: nothing to release"))?;
    Ok((
        bumped(tag, bump),
        format!(
            "{} bump: {} commit(s) since v{tag}",
            bump.name(),
            commits.len()
        ),
    ))
}

/// `cargo xtask version --next`: prints the next version on stdout, the reason on stderr.
pub(crate) fn print_next() -> Result<()> {
    let workspace: Version = util::workspace_version()?.parse()?;
    let latest = latest_tag()?;
    let commits = commits_since(latest)?;
    let (next, reason) = compute_next(workspace, latest, &commits)?;
    eprintln!("{reason}");
    println!("{next}");
    Ok(())
}

/// Everything a release pull request is checked against.
struct ReleaseFacts<'a> {
    hotfix: bool,
    version: Version,
    workspace: Version,
    tag_exists: bool,
    changelog: &'a str,
    latest: Option<Version>,
    commits: &'a [Commit],
}

/// Returns the problems of a release or hotfix pull request (empty when it is ready).
fn release_problems(facts: &ReleaseFacts<'_>) -> Vec<String> {
    let version = facts.version;
    let mut problems = Vec::new();
    if facts.workspace != version {
        problems.push(format!(
            "Cargo.toml has version {}: run `cargo xtask version {version}`",
            facts.workspace
        ));
    }
    if facts.tag_exists {
        problems.push(format!("tag v{version} already exists"));
    }
    let heading = format!("## [{version}]");
    if !facts
        .changelog
        .lines()
        .any(|line| line.starts_with(&heading))
    {
        problems.push(format!(
            "CHANGELOG.md has no `{heading}` section: run `git cliff --tag v{version} -o CHANGELOG.md`"
        ));
    }
    if let Some(tag) = facts.latest {
        if version <= tag {
            problems.push(format!(
                "{version} is not newer than the latest release v{tag}"
            ));
        } else if facts.hotfix {
            if (version.major, version.minor) != (tag.major, tag.minor) {
                problems.push(format!("a hotfix only changes the patch number of v{tag}"));
            }
        } else if let Some(bump) = required_bump(facts.commits) {
            let minimum = bumped(tag, bump);
            if version < minimum {
                problems.push(format!(
                    "the commits since v{tag} require at least {minimum} ({} bump)",
                    bump.name()
                ));
            }
        }
    }
    problems
}

/// `cargo xtask release-check --head <branch>`: validates a release or hotfix pull request
/// (version, tag, changelog, version bump). Other branches pass without checks.
pub(crate) fn check(head: &str) -> Result<()> {
    let (hotfix, version) = match gitflow::classify(head)? {
        BranchKind::Release(version) => (false, version),
        BranchKind::Hotfix(version) => (true, version),
        _ => {
            println!("{head} is not a release or hotfix branch: nothing to check");
            return Ok(());
        }
    };
    let changelog = std::fs::read_to_string(util::workspace_root().join("CHANGELOG.md"))?;
    let latest = latest_tag()?;
    let commits = commits_since(latest)?;
    let tag = format!("v{version}");
    let facts = ReleaseFacts {
        hotfix,
        version,
        workspace: util::workspace_version()?.parse()?,
        tag_exists: git(&["tag", "--list", &tag])?.trim() == tag,
        changelog: &changelog,
        latest,
        commits: &commits,
    };
    let problems = release_problems(&facts);
    if problems.is_empty() {
        println!("release check: {head} is ready to release v{version}");
        Ok(())
    } else {
        Err(format!(
            "release check failed for {head}:\n  - {}",
            problems.join("\n  - ")
        )
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(header: &str) -> Commit {
        Commit {
            header: header.to_owned(),
            body: String::new(),
        }
    }

    fn v(text: &str) -> Version {
        text.parse().unwrap()
    }

    #[test]
    fn commit_types_map_to_bumps() {
        assert_eq!(
            commit("feat(render): add borders").bump(),
            Some(Bump::Minor)
        );
        assert_eq!(commit("fix: clamp radii").bump(), Some(Bump::Patch));
        assert_eq!(commit("docs: explain tags").bump(), Some(Bump::Patch));
        assert_eq!(commit("feat!: new scene API").bump(), Some(Bump::Major));
        assert_eq!(
            commit("fix(gpu)!: drop KHR paths").bump(),
            Some(Bump::Major)
        );
        let footer = Commit {
            header: "refactor(core): rename Rect".to_owned(),
            body: "Details.\n\nBREAKING CHANGE: Rect::new takes min and max".to_owned(),
        };
        assert_eq!(footer.bump(), Some(Bump::Major));
        assert_eq!(commit("Merge pull request #3 from x").bump(), None);
        assert_eq!(commit("update stuff").bump(), None);
        assert_eq!(commit("wip: something").bump(), None);
    }

    #[test]
    fn required_bump_is_the_largest() {
        assert_eq!(required_bump(&[]), None);
        let commits = [commit("fix: a"), commit("feat: b"), commit("docs: c")];
        assert_eq!(required_bump(&commits), Some(Bump::Minor));
    }

    #[test]
    fn bumps_follow_semver_with_the_pre_1_0_rule() {
        assert_eq!(bumped(v("0.1.0"), Bump::Major), v("0.2.0"));
        assert_eq!(bumped(v("0.1.3"), Bump::Minor), v("0.2.0"));
        assert_eq!(bumped(v("0.1.3"), Bump::Patch), v("0.1.4"));
        assert_eq!(bumped(v("1.2.3"), Bump::Major), v("2.0.0"));
        assert_eq!(bumped(v("1.2.3"), Bump::Minor), v("1.3.0"));
        assert_eq!(bumped(v("1.2.3"), Bump::Patch), v("1.2.4"));
    }

    #[test]
    fn git_log_records_are_parsed() {
        let log = "feat: a\x1f\x1e\nfix(gpu): b\x1fbody line\nBREAKING CHANGE: x\n\x1e\n";
        let commits = parse_log(log);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0], commit("feat: a"));
        assert_eq!(commits[1].header, "fix(gpu): b");
        assert_eq!(commits[1].bump(), Some(Bump::Major));
    }

    #[test]
    fn next_version_comes_from_the_workspace_then_the_commits() {
        let (first, _) = compute_next(v("0.1.0"), None, &[]).unwrap();
        assert_eq!(first, v("0.1.0"));
        let (next, reason) =
            compute_next(v("0.1.0"), Some(v("0.1.0")), &[commit("feat: x")]).unwrap();
        assert_eq!(next, v("0.2.0"));
        assert!(reason.starts_with("minor bump"), "{reason}");
        assert!(compute_next(v("0.1.0"), Some(v("0.1.0")), &[]).is_err());
    }

    fn facts<'a>(version: &str, changelog: &'a str, commits: &'a [Commit]) -> ReleaseFacts<'a> {
        ReleaseFacts {
            hotfix: false,
            version: v(version),
            workspace: v(version),
            tag_exists: false,
            changelog,
            latest: Some(v("0.1.0")),
            commits,
        }
    }

    #[test]
    fn a_consistent_release_has_no_problem() {
        let commits = [commit("feat: x")];
        let changelog = "# Changelog\n\n## [0.2.0] - 2026-11-01\n";
        assert_eq!(
            release_problems(&facts("0.2.0", changelog, &commits)),
            Vec::<String>::new()
        );
    }

    #[test]
    fn release_problems_are_reported() {
        let commits = [commit("feat: x")];
        let changelog = "# Changelog\n\n## [0.1.1] - 2026-11-01\n";
        let mut facts = facts("0.1.1", changelog, &commits);
        facts.workspace = v("0.1.0");
        facts.tag_exists = true;
        let problems = release_problems(&facts).join("\n");
        assert!(problems.contains("cargo xtask version 0.1.1"), "{problems}");
        assert!(problems.contains("already exists"), "{problems}");
        assert!(problems.contains("require at least 0.2.0"), "{problems}");

        let problems = release_problems(&ReleaseFacts {
            changelog: "## [Unreleased]\n",
            ..facts
        })
        .join("\n");
        assert!(problems.contains("no `## [0.1.1]` section"), "{problems}");
    }

    #[test]
    fn hotfixes_only_bump_the_patch_number() {
        let changelog = "## [0.2.0]\n## [0.1.1]\n## [0.1.0]\n";
        let mut patch = facts("0.1.1", changelog, &[]);
        patch.hotfix = true;
        assert_eq!(release_problems(&patch), Vec::<String>::new());
        let mut minor = facts("0.2.0", changelog, &[]);
        minor.hotfix = true;
        assert_eq!(release_problems(&minor).len(), 1);
        assert!(release_problems(&minor)[0].contains("patch number"));
        let mut stale = facts("0.1.0", changelog, &[]);
        stale.hotfix = true;
        assert_eq!(release_problems(&stale).len(), 1);
        assert!(release_problems(&stale)[0].contains("not newer"));
    }
}
