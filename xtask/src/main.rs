//! Koebi development automation: `cargo xtask <command>`.
//!
//! Every repetitive task (local CI, shaders, golden images, releases, mobile packaging) is a
//! Rust function here instead of a shell script, so it behaves the same on Windows, Linux and
//! macOS, for humans, CI and AI agents. Standard library only.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "xtask is a command-line tool"
)]

mod android;
mod ci;
mod commits;
mod gitflow;
mod ios;
mod package;
mod shaders;
mod util;
mod version;

use std::process::ExitCode;

use util::Result;

const HELP: &str = "\
Usage: cargo xtask <command> [options]

Quality
  ci [--fast]                     Run the local CI gate (fmt, clippy, tests, docs, shaders, deny, typos)
  shaders [--check]               Compile GLSL to SPIR-V (--check: fail if committed SPIR-V is stale)
  bless                           Regenerate golden images after an intended visual change

Git workflow
  install-hooks                   Use the versioned git hooks in .githooks/
  gitflow-check --head <b> [--base <b>]
                                  Validate a branch name, and a pull request target with --base
  commit-msg <file>               Validate a Conventional Commits message (commit-msg hook)
  version <X.Y.Z>                 Set the workspace version (release and hotfix branches)

Packaging
  package [--target <triple>] [--universal]
                                  Build a release archive for desktop (target/dist/)
  android [--release] [--abi <abi,...>] [--no-gradle]
                                  Build libkoebi.so with cargo-ndk and the APK with Gradle
  ios [--release] [--device] [--run]
                                  Build Koebi.app for the iOS simulator (or a device), optionally run it";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match dispatch(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("\nxtask error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn dispatch(args: &[String]) -> Result<()> {
    let (command, rest) = args
        .split_first()
        .map_or(("help", &[][..]), |(c, r)| (c.as_str(), r));
    let has = |flag: &str| rest.iter().any(|arg| arg == flag);
    let value = |flag: &str| {
        rest.iter()
            .position(|arg| arg == flag)
            .and_then(|i| rest.get(i + 1))
            .map(String::as_str)
    };
    match command {
        "ci" => ci::run(has("--fast")),
        "shaders" => shaders::run(has("--check")),
        "bless" => ci::bless(),
        "install-hooks" => util::install_hooks(),
        "gitflow-check" => {
            let head = value("--head").ok_or("--head <branch> is required")?;
            match value("--base") {
                Some(base) => gitflow::check_pull_request(head, base)?,
                None => {
                    gitflow::classify(head)?;
                }
            }
            println!("gitflow: OK");
            Ok(())
        }
        "commit-msg" => {
            let file = rest.first().ok_or("usage: cargo xtask commit-msg <file>")?;
            commits::check_message_file(file.as_ref())
        }
        "version" => version::set(rest.first().ok_or("usage: cargo xtask version <X.Y.Z>")?),
        "package" => package::run(value("--target"), has("--universal")),
        "android" => android::run(&android::Options {
            release: has("--release"),
            abis: value("--abi").map_or_else(android::default_abis, |list| {
                list.split(',').map(str::to_owned).collect()
            }),
            gradle: !has("--no-gradle"),
        }),
        "ios" => ios::run(&ios::Options {
            release: has("--release"),
            device: has("--device"),
            run: has("--run"),
        }),
        "help" | "--help" | "-h" => {
            println!("{HELP}");
            Ok(())
        }
        other => Err(format!("unknown command {other:?}\n\n{HELP}").into()),
    }
}
