//! Local CI gate, mirroring `.github/workflows/ci.yml`.

use std::process::Command;

use crate::util::{self, Result};

enum Outcome {
    Passed,
    Skipped(&'static str),
    Failed(String),
}

fn outcome(result: Result<()>) -> Outcome {
    match result {
        Ok(()) => Outcome::Passed,
        Err(error) => Outcome::Failed(error.to_string()),
    }
}

/// `cargo xtask ci`: runs every check and reports all failures at the end.
pub(crate) fn run(fast: bool) -> Result<()> {
    let mut results: Vec<(&str, Outcome)> = Vec::new();

    util::step("format");
    results.push((
        "fmt",
        outcome(util::run(util::cargo().args(["fmt", "--all", "--check"]))),
    ));

    util::step("clippy");
    let clippy = util::cargo()
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ])
        .status();
    results.push((
        "clippy",
        match clippy {
            Ok(status) if status.success() => Outcome::Passed,
            Ok(status) => Outcome::Failed(format!("clippy exited with {status}")),
            Err(error) => Outcome::Failed(error.to_string()),
        },
    ));

    util::step("tests");
    let tests = if util::has_cargo_subcommand("nextest") {
        util::run(util::cargo().args(["nextest", "run", "--workspace", "--locked"]))
    } else {
        util::run(util::cargo().args(["test", "--workspace", "--all-targets", "--locked"]))
    };
    results.push(("tests", outcome(tests)));
    let doctests = util::run(util::cargo().args(["test", "--workspace", "--doc", "--locked"]));
    results.push(("doctests", outcome(doctests)));

    if !fast {
        util::step("docs");
        let docs = util::run(
            util::cargo()
                .args(["doc", "--workspace", "--no-deps", "--locked"])
                .env("RUSTDOCFLAGS", "-D warnings"),
        );
        results.push(("docs", outcome(docs)));

        util::step("shaders");
        results.push((
            "shaders",
            if util::find_tool("glslc").is_some() {
                outcome(crate::shaders::run(true))
            } else {
                Outcome::Skipped("glslc not found (install the Vulkan SDK)")
            },
        ));

        util::step("dependencies");
        results.push((
            "cargo-deny",
            if util::has_cargo_subcommand("deny") {
                outcome(util::run(util::cargo().args(["deny", "check"])))
            } else {
                Outcome::Skipped("cargo install cargo-deny --locked")
            },
        ));

        util::step("spelling");
        results.push((
            "typos",
            if util::find_tool("typos").is_some() {
                let mut typos = Command::new("typos");
                typos.current_dir(util::workspace_root());
                outcome(util::run(&mut typos))
            } else {
                Outcome::Skipped("cargo install typos-cli --locked")
            },
        ));
    }

    println!("\n==> summary");
    let mut failed = 0;
    for (name, outcome) in &results {
        match outcome {
            Outcome::Passed => println!("  PASS  {name}"),
            Outcome::Skipped(reason) => println!("  SKIP  {name} ({reason})"),
            Outcome::Failed(error) => {
                failed += 1;
                println!("  FAIL  {name}: {error}");
            }
        }
    }
    if failed == 0 {
        Ok(())
    } else {
        Err(format!("{failed} check(s) failed").into())
    }
}

/// `cargo xtask bless`: regenerates every golden image.
pub(crate) fn bless() -> Result<()> {
    util::run(
        util::cargo()
            .args(["test", "--workspace", "--locked"])
            .env("KOEBI_BLESS", "1")
            .env("KOEBI_REQUIRE_GPU", "1"),
    )?;
    println!("golden images updated: review them (`git diff --stat -- '*.png'`) and commit them");
    Ok(())
}
