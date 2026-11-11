//! Compiles the catalogs of `locales/` into the static tables of `catalogs.rs` in `OUT_DIR`.
//!
//! The template `locales/koebi.pot` gives the number of messages; each `locales/<code>.po`
//! becomes a language, its translated messages sorted by context then text. The build fails on
//! the first catalog that cannot be used as it is, naming its file and line: a broken PO line, a
//! header without its fields, a plural rule that does not compile, a translation with plural
//! forms the language does not have or a placeholder the English message does not hold.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a build script talks to Cargo on stdout and reports its errors on stderr"
)]

#[allow(dead_code, unreachable_pub, reason = "shared with the crate and xtask")]
#[path = "src/catalog.rs"]
mod catalog;
#[allow(dead_code, unreachable_pub, reason = "shared with the crate and xtask")]
#[path = "src/plural.rs"]
mod plural;
#[allow(dead_code, unreachable_pub, reason = "shared with the crate and xtask")]
#[path = "src/po.rs"]
mod po;

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs, process};

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("set by Cargo"));
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("set by Cargo"));
    // The directory is scanned whole: a catalog added, changed or removed reruns the script.
    println!("cargo::rerun-if-changed=locales");
    match build(&manifest.join("locales")) {
        Ok(code) => {
            if let Err(error) = fs::write(out.join("catalogs.rs"), code) {
                eprintln!("error: cannot write catalogs.rs: {error}");
                process::exit(1);
            }
        }
        Err(message) => {
            eprintln!("error: {message}");
            process::exit(1);
        }
    }
}

fn read(path: &Path) -> Result<po::Catalog, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    po::parse(&text).map_err(|error| format!("{}:{}: {}", path.display(), error.line, error.reason))
}

fn build(locales: &Path) -> Result<String, String> {
    let template = read(&locales.join("koebi.pot"))?;
    let total = template
        .entries
        .iter()
        .filter(|entry| !entry.obsolete)
        .count();
    let mut files: Vec<PathBuf> = fs::read_dir(locales)
        .map_err(|error| format!("{}: {error}", locales.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "po"))
        .collect();
    files.sort();
    let mut code = String::new();
    code.push_str("/// Every language: English, then each catalog by code.\n");
    code.push_str("static LANGUAGES: &[Language] = &[\n");
    let _ = writeln!(
        code,
        "    Language {{ code: \"en\", name: \"English\", english_name: \"English\", translated: {total}, total: {total}, plural: ENGLISH_PLURAL, messages: &[] }},"
    );
    for path in &files {
        language(path, &template, total, &mut code)?;
    }
    code.push_str("];\n");
    Ok(code)
}

fn language(
    path: &Path,
    template: &po::Catalog,
    total: usize,
    code: &mut String,
) -> Result<(), String> {
    let catalog = read(path)?;
    let at = |line: usize, reason: &str| format!("{}:{line}: {reason}", path.display());
    let header = catalog::header(&catalog).map_err(|problem| at(problem.line, &problem.reason))?;
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    if header.language != stem {
        return Err(at(
            catalog.header.line,
            &format!(
                "the header says Language: {}, the file is named {stem}",
                header.language
            ),
        ));
    }
    if let Some(problem) = catalog::check(&catalog, &header)
        .into_iter()
        .find(|problem| problem.severity == catalog::Severity::Error)
    {
        return Err(at(problem.line, &problem.reason));
    }
    let counts = catalog::counts(&catalog, template);
    let mut messages: Vec<&po::Entry> = catalog
        .entries
        .iter()
        .filter(|entry| entry.is_translated())
        .collect();
    messages.sort_by(|a, b| {
        (a.context.as_deref().unwrap_or(""), a.id.as_str())
            .cmp(&(b.context.as_deref().unwrap_or(""), b.id.as_str()))
    });
    let _ = writeln!(
        code,
        "    Language {{ code: {:?}, name: {:?}, english_name: {:?}, translated: {}, total: {total}, plural: &[{}], messages: &[",
        header.language,
        header.native_name,
        header.english_name,
        counts.translated,
        rule(&header.rule),
    );
    for entry in messages {
        let strings: Vec<String> = entry
            .strings
            .iter()
            .map(|text| format!("{text:?}"))
            .collect();
        let _ = writeln!(
            code,
            "        ({:?}, {:?}, &[{}]),",
            entry.context.as_deref().unwrap_or(""),
            entry.id,
            strings.join(", ")
        );
    }
    code.push_str("    ] },\n");
    Ok(())
}

/// A compiled plural rule as Rust.
fn rule(ops: &[plural::Op]) -> String {
    let ops: Vec<String> = ops
        .iter()
        .map(|op| match op {
            plural::Op::N => "plural::Op::N".to_owned(),
            plural::Op::Number(value) => format!("plural::Op::Number({value})"),
            plural::Op::Not => "plural::Op::Not".to_owned(),
            plural::Op::Binary(binary) => {
                format!("plural::Op::Binary(plural::Binary::{})", binary.name())
            }
            plural::Op::Select => "plural::Op::Select".to_owned(),
        })
        .collect();
    ops.join(", ")
}
