//! `cargo xtask i18n`: the translation catalogs kept in step with the sources
//! (`docs/development/translation.md`).
//!
//! - `extract` reads every message the sources mark (`tr`, `trc`, `trn`, `trnc`, `n_!`,
//!   `nc_!`) and
//!   writes the template `crates/koebi-i18n/locales/koebi.pot`.
//! - `update` extracts, then merges the template into every catalog: new messages arrive
//!   untranslated, removed ones become obsolete (`#~`), and a message whose English changed
//!   only in case or final punctuation keeps its translation, marked fuzzy.
//! - `stats` prints how much of the template each catalog translates (`--markdown` for a
//!   table).
//! - `check` fails when the template is out of date, a catalog misses messages of the
//!   template, or a catalog has an error; it prints the warnings. `cargo xtask ci` runs it.
//! - `new <code> --name <native> --english <name> --plural <Plural-Forms>` starts a catalog.

#[allow(
    dead_code,
    unreachable_pub,
    reason = "shared with koebi-i18n's build script"
)]
#[path = "../../crates/koebi-i18n/src/catalog.rs"]
mod catalog;
#[allow(
    dead_code,
    unreachable_pub,
    reason = "shared with koebi-i18n's build script"
)]
#[path = "../../crates/koebi-i18n/src/plural.rs"]
mod plural;
#[allow(
    dead_code,
    unreachable_pub,
    reason = "shared with koebi-i18n's build script"
)]
#[path = "../../crates/koebi-i18n/src/po.rs"]
mod po;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::util::{self, Result};

/// Where the catalogs live, from the workspace root.
const LOCALES: &str = "crates/koebi-i18n/locales";
/// The template's file name.
const TEMPLATE: &str = "koebi.pot";
/// The folders whose sources hold messages, from the workspace root.
const SOURCES: &[&str] = &["apps", "crates"];
/// Crates whose sources are not scanned: the translation crate's own examples and tests, and
/// test helpers.
const SKIPPED: &[&str] = &["crates/koebi-i18n", "crates/koebi-testkit"];

/// Runs `cargo xtask i18n <command>`.
pub(crate) fn run(args: &[String]) -> Result<()> {
    let value = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|i| args.get(i + 1))
            .map(String::as_str)
    };
    match args.first().map(String::as_str) {
        Some("extract") => {
            let template = extract()?;
            write_template(&template)?;
            println!("{TEMPLATE}: {} messages", template.entries.len());
            Ok(())
        }
        Some("update") => update(),
        Some("stats") => stats(args.iter().any(|arg| arg == "--markdown")),
        Some("check") => check(),
        Some("new") => {
            let code = args.get(1).ok_or("usage: cargo xtask i18n new <code> --name <native> --english <name> --plural <Plural-Forms>")?;
            new_catalog(
                code,
                value("--name").ok_or("--name <native name> is required")?,
                value("--english").ok_or("--english <English name> is required")?,
                value("--plural").ok_or("--plural \"nplurals=N; plural=RULE;\" is required")?,
            )
        }
        _ => Err("usage: cargo xtask i18n <extract | update | stats [--markdown] | check | new <code> ...>".into()),
    }
}

fn locales() -> PathBuf {
    util::workspace_root().join(LOCALES)
}

/// The catalogs of the languages, by code.
fn catalog_files() -> Result<Vec<(String, PathBuf)>> {
    let mut files: Vec<(String, PathBuf)> = fs::read_dir(locales())?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "po"))
        .filter_map(|path| {
            let code = path.file_stem()?.to_str()?.to_owned();
            Some((code, path))
        })
        .collect();
    files.sort();
    Ok(files)
}

fn read_catalog(path: &Path) -> Result<po::Catalog> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    po::parse(&text)
        .map_err(|error| format!("{}:{}: {}", path.display(), error.line, error.reason).into())
}

fn write_catalog(path: &Path, catalog: &po::Catalog) -> Result<()> {
    fs::write(path, po::write(catalog))
        .map_err(|error| format!("{}: {error}", path.display()).into())
}

// --- Extraction --------------------------------------------------------------------------------

/// A message found in the sources.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Found {
    context: Option<String>,
    id: String,
    plural: Option<String>,
    comments: Vec<String>,
    /// The file, from the workspace root with `/`: no line, so that the template changes with
    /// the messages rather than with every line moved.
    reference: String,
    /// Its line, for errors.
    line: usize,
}

/// The template the sources give.
fn extract() -> Result<po::Catalog> {
    let root = util::workspace_root();
    let mut files = Vec::new();
    for folder in SOURCES {
        rust_files(&root, &root.join(folder), &mut files)?;
    }
    files.sort();
    let mut entries: Vec<po::Entry> = Vec::new();
    let mut index: HashMap<(Option<String>, String), usize> = HashMap::new();
    for path in files {
        let relative = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(&path)?;
        // Test modules close the files that have one, by convention; a test-only item before
        // them does not end the file.
        let code = test_module(&text).map_or(text.as_str(), |at| &text[..at]);
        for found in scan(code, &relative).map_err(|error| format!("{relative}:{error}"))? {
            let key = (found.context.clone(), found.id.clone());
            if let Some(&at) = index.get(&key) {
                let entry = &mut entries[at];
                if entry.plural != found.plural {
                    return Err(format!(
                        "{}:{}: {:?} has another plural form in {}",
                        found.reference,
                        found.line,
                        found.id,
                        entry.references.join(", ")
                    )
                    .into());
                }
                if !entry.references.contains(&found.reference) {
                    entry.references.push(found.reference);
                }
                for comment in found.comments {
                    if !entry.comments.contains(&comment) {
                        entry.comments.push(comment);
                    }
                }
            } else {
                index.insert(key, entries.len());
                let forms = if found.plural.is_some() { 2 } else { 1 };
                entries.push(po::Entry {
                    comments: found.comments,
                    references: vec![found.reference],
                    context: found.context,
                    id: found.id,
                    plural: found.plural,
                    strings: vec![String::new(); forms],
                    ..po::Entry::default()
                });
            }
        }
    }
    Ok(po::Catalog {
        header: template_header(),
        entries,
    })
}

/// Where the test module of `text` starts: `#[cfg(test)]` followed by `mod`.
fn test_module(text: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(at) = text[from..].find("#[cfg(test)]") {
        let start = from + at;
        let after = text[start + "#[cfg(test)]".len()..].trim_start();
        let after = after.strip_prefix("pub(crate) ").unwrap_or(after);
        if after.starts_with("mod ") {
            return Some(start);
        }
        from = start + 1;
    }
    None
}

fn template_header() -> po::Entry {
    po::Entry {
        translator_comments: vec![
            "Koebi's messages, in English: written by `cargo xtask i18n extract`, never by hand."
                .to_owned(),
        ],
        strings: vec![
            [
                "Project-Id-Version: Koebi",
                "Report-Msgid-Bugs-To: https://github.com/Kolosta/Koebi/issues",
                "MIME-Version: 1.0",
                "Content-Type: text/plain; charset=UTF-8",
                "Content-Transfer-Encoding: 8bit",
                "Language: ",
                "Plural-Forms: nplurals=INTEGER; plural=EXPRESSION;",
                "X-Native-Name: ",
                "X-English-Name: ",
                "",
            ]
            .join("\n"),
        ],
        ..po::Entry::default()
    }
}

fn write_template(template: &po::Catalog) -> Result<()> {
    write_catalog(&locales().join(TEMPLATE), template)
}

fn rust_files(root: &Path, folder: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if SKIPPED.iter().any(|skipped| relative == *skipped) {
            continue;
        }
        if path.is_dir() {
            // Integration tests, benchmarks and build outputs hold no message.
            let name = entry.file_name();
            if matches!(name.to_str(), Some("tests" | "benches" | "target")) {
                continue;
            }
            rust_files(root, &path, out)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

/// A token of Rust source, as far as marks are concerned.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Token {
    Ident(String),
    Str(String),
    Punct(char),
    /// A `//` comment's text, without its slashes.
    Comment(String),
}

/// The tokens of `code`, each with its line.
fn tokenize(code: &str) -> std::result::Result<Vec<(Token, usize)>, String> {
    let chars: Vec<char> = code.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut line = 1;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c == '/' && next == Some('/') {
            let end = chars[i..]
                .iter()
                .position(|&c| c == '\n')
                .map_or(chars.len(), |n| i + n);
            let text: String = chars[i + 2..end].iter().collect();
            // Doc comments document code, they are not messages' comments.
            if !text.starts_with('/') && !text.starts_with('!') {
                tokens.push((Token::Comment(text.trim().to_owned()), line));
            }
            i = end;
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
            while i < chars.len() {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    if chars[i] == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
        } else if c == '"' || ((c == 'b' || c == 'c') && next == Some('"')) {
            let start_line = line;
            i += if c == '"' { 1 } else { 2 };
            let (value, end, lines) =
                cooked_string(&chars, i).map_err(|reason| format!("{start_line}: {reason}"))?;
            tokens.push((Token::Str(value), start_line));
            line += lines;
            i = end;
        } else if (c == 'r' && matches!(next, Some('"' | '#')))
            || ((c == 'b' || c == 'c')
                && next == Some('r')
                && matches!(chars.get(i + 2), Some('"' | '#')))
        {
            let start = if c == 'r' { i + 1 } else { i + 2 };
            let hashes = chars[start..].iter().take_while(|&&c| c == '#').count();
            if chars.get(start + hashes) != Some(&'"') {
                // `r#ident`: a raw identifier.
                let end = ident_end(&chars, start + hashes);
                tokens.push((
                    Token::Ident(chars[start + hashes..end].iter().collect()),
                    line,
                ));
                i = end;
                continue;
            }
            let body = start + hashes + 1;
            let closing: Vec<char> = std::iter::once('"')
                .chain(std::iter::repeat_n('#', hashes))
                .collect();
            let end = (body..chars.len())
                .find(|&at| chars[at..].starts_with(&closing))
                .ok_or_else(|| format!("{line}: a raw string never ends"))?;
            let value: String = chars[body..end].iter().collect();
            let start_line = line;
            line += value.matches('\n').count();
            tokens.push((Token::Str(value), start_line));
            i = end + closing.len();
        } else if c == '\'' {
            // A character literal, or a lifetime or label.
            if next == Some('\\') {
                let end = (i + 2..chars.len())
                    .find(|&at| chars[at] == '\'')
                    .ok_or_else(|| format!("{line}: a character literal never ends"))?;
                i = end + 1;
            } else if chars.get(i + 2) == Some(&'\'') {
                i += 3;
            } else {
                i += 1;
            }
        } else if c.is_alphabetic() || c == '_' {
            let end = ident_end(&chars, i);
            tokens.push((Token::Ident(chars[i..end].iter().collect()), line));
            i = end;
        } else if c.is_ascii_digit() {
            i = ident_end(&chars, i);
        } else {
            tokens.push((Token::Punct(c), line));
            i += 1;
        }
    }
    Ok(tokens)
}

fn ident_end(chars: &[char], from: usize) -> usize {
    (from..chars.len())
        .find(|&at| !(chars[at].is_alphanumeric() || chars[at] == '_'))
        .unwrap_or(chars.len())
}

/// A string literal's value from `from` (past its quote): the value, the index past the
/// closing quote, and the line feeds it holds.
fn cooked_string(
    chars: &[char],
    from: usize,
) -> std::result::Result<(String, usize, usize), String> {
    let mut value = String::new();
    let mut lines = 0;
    let mut i = from;
    while i < chars.len() {
        match chars[i] {
            '"' => return Ok((value, i + 1, lines)),
            '\\' => {
                let escape = chars.get(i + 1).copied().ok_or("a string never ends")?;
                i += 2;
                match escape {
                    'n' => value.push('\n'),
                    'r' => value.push('\r'),
                    't' => value.push('\t'),
                    '0' => value.push('\0'),
                    '\\' | '"' | '\'' => value.push(escape),
                    'x' => {
                        let hex: String = chars
                            .get(i..i + 2)
                            .ok_or("a broken \\x escape")?
                            .iter()
                            .collect();
                        let code =
                            u8::from_str_radix(&hex, 16).map_err(|_| "a broken \\x escape")?;
                        value.push(char::from(code));
                        i += 2;
                    }
                    'u' => {
                        let end = (i..chars.len())
                            .find(|&at| chars[at] == '}')
                            .ok_or("a broken \\u escape")?;
                        let hex: String = chars[i + 1..end].iter().filter(|&&c| c != '_').collect();
                        let code =
                            u32::from_str_radix(&hex, 16).map_err(|_| "a broken \\u escape")?;
                        value.push(char::from_u32(code).ok_or("a broken \\u escape")?);
                        i = end + 1;
                    }
                    '\n' => {
                        // A continued line: the line feed and the next line's indentation go.
                        lines += 1;
                        while i < chars.len() && chars[i].is_whitespace() {
                            if chars[i] == '\n' {
                                lines += 1;
                            }
                            i += 1;
                        }
                    }
                    '\r' if chars.get(i) == Some(&'\n') => {
                        lines += 1;
                        i += 1;
                        while i < chars.len() && chars[i].is_whitespace() {
                            if chars[i] == '\n' {
                                lines += 1;
                            }
                            i += 1;
                        }
                    }
                    other => return Err(format!("an unknown escape \\{other}")),
                }
            }
            c => {
                if c == '\n' {
                    lines += 1;
                }
                value.push(c);
                i += 1;
            }
        }
    }
    Err("a string never ends".to_owned())
}

/// The marks of a file: what each function takes as literals.
const MARKS: &[(&str, bool, usize)] = &[
    // (name, has a context first, messages after it)
    ("tr", false, 1),
    ("trc", true, 1),
    ("trn", false, 2),
    ("trnc", true, 2),
];

/// The messages `code` marks, `path` naming it in references.
fn scan(code: &str, path: &str) -> std::result::Result<Vec<Found>, String> {
    let tokens = tokenize(code)?;
    let mut found = Vec::new();
    for (at, (token, line)) in tokens.iter().enumerate() {
        let Token::Ident(name) = token else { continue };
        // Not a method (`x.tr(`), not the function's own definition (`fn tr(`).
        let before = at.checked_sub(1).map(|i| &tokens[i].0);
        if matches!(before, Some(Token::Punct('.')))
            || matches!(before, Some(Token::Ident(word)) if word == "fn")
        {
            continue;
        }
        let (context_first, messages, args_at) = if name == "n_" || name == "nc_" {
            if tokens.get(at + 1).map(|t| &t.0) != Some(&Token::Punct('!')) {
                continue;
            }
            (name == "nc_", 1, at + 2)
        } else if let Some(&(_, context, count)) = MARKS.iter().find(|(mark, _, _)| mark == name) {
            (context, count, at + 1)
        } else {
            continue;
        };
        if tokens.get(args_at).map(|t| &t.0) != Some(&Token::Punct('(')) {
            continue;
        }
        let wanted = messages + usize::from(context_first);
        let mut literals = Vec::new();
        let mut next = args_at + 1;
        while literals.len() < wanted {
            match tokens.get(next).map(|t| &t.0) {
                Some(Token::Str(value)) => literals.push(value.clone()),
                _ => break,
            }
            next += 1;
            if literals.len() < wanted && tokens.get(next).map(|t| &t.0) != Some(&Token::Punct(','))
            {
                break;
            }
            next += 1;
        }
        if literals.len() < wanted {
            // `tr(action.label)`, `trc("key", name)`: text marked where it is written, with `n_!`
            // or `nc_!`.
            let context_only = context_first && literals.len() == 1;
            if !literals.is_empty() && !context_only {
                return Err(format!(
                    "{line}: `{name}` takes literals or no literal at all"
                ));
            }
            continue;
        }
        let mut literals = literals.into_iter();
        let context = if context_first { literals.next() } else { None };
        if context.as_deref() == Some("") {
            return Err(format!(
                "{line}: an empty context: call the function without one"
            ));
        }
        let id = literals.next().unwrap_or_default();
        if id.is_empty() {
            return Err(format!("{line}: an empty message"));
        }
        let plural = literals.next();
        found.push(Found {
            context,
            id,
            plural,
            comments: translator_comments(&tokens, at, *line),
            reference: path.to_owned(),
            line: *line,
        });
    }
    Ok(found)
}

/// The `// TRANSLATORS:` comment on the lines just above the mark at `at`, on `line`.
fn translator_comments(tokens: &[(Token, usize)], at: usize, line: usize) -> Vec<String> {
    // Back to the first token of the mark's line, then the comments right above it.
    let mut first = at;
    while first > 0
        && tokens[first - 1].1 == line
        && !matches!(tokens[first - 1].0, Token::Comment(_))
    {
        first -= 1;
    }
    let mut comments = Vec::new();
    let mut expected = line;
    let mut i = first;
    while i > 0 {
        let (Token::Comment(text), comment_line) = &tokens[i - 1] else {
            break;
        };
        if *comment_line + 1 != expected {
            break;
        }
        comments.push(text.clone());
        expected = *comment_line;
        i -= 1;
    }
    comments.reverse();
    let start = comments
        .iter()
        .position(|text| text.starts_with("TRANSLATORS:"));
    start.map_or_else(Vec::new, |start| vec![comments[start..].join(" ")])
}

// --- Merging -----------------------------------------------------------------------------------

/// The text of a message with its case and final punctuation left out: a change of these keeps
/// the translation, marked to review.
fn loose(text: &str) -> String {
    text.trim_end_matches(|c: char| c.is_ascii_punctuation() || c == '…' || c.is_whitespace())
        .to_lowercase()
}

/// `catalog` brought in step with `template`.
fn merge(catalog: &po::Catalog, template: &po::Catalog, forms: usize) -> po::Catalog {
    let mut old: Vec<Option<&po::Entry>> = catalog.entries.iter().map(Some).collect();
    let mut entries = Vec::new();
    for wanted in &template.entries {
        let exact = old
            .iter()
            .position(|entry| entry.is_some_and(|entry| entry.key() == wanted.key()));
        let close = exact.or_else(|| {
            old.iter().position(|entry| {
                entry.is_some_and(|entry| {
                    entry.context == wanted.context
                        && !entry.strings.iter().all(String::is_empty)
                        && loose(&entry.id) == loose(&wanted.id)
                })
            })
        });
        let previous = close.and_then(|index| old[index].take());
        let forms = if wanted.plural.is_some() { forms } else { 1 };
        let mut entry = po::Entry {
            comments: wanted.comments.clone(),
            references: wanted.references.clone(),
            context: wanted.context.clone(),
            id: wanted.id.clone(),
            plural: wanted.plural.clone(),
            strings: vec![String::new(); forms],
            ..po::Entry::default()
        };
        if let Some(previous) = previous {
            entry
                .translator_comments
                .clone_from(&previous.translator_comments);
            entry.flags.clone_from(&previous.flags);
            for (text, old) in entry.strings.iter_mut().zip(&previous.strings) {
                text.clone_from(old);
            }
            let changed = previous.id != wanted.id || previous.plural != wanted.plural;
            if changed && !entry.is_fuzzy() && !entry.strings.iter().all(String::is_empty) {
                entry.flags.push("fuzzy".to_owned());
            }
        }
        entries.push(entry);
    }
    // What the sources no longer hold stays for its translation, out of the way.
    for entry in old.into_iter().flatten() {
        if !entry.strings.iter().all(String::is_empty) {
            let mut kept = entry.clone();
            kept.obsolete = true;
            kept.references.clear();
            kept.comments.clear();
            entries.push(kept);
        }
    }
    po::Catalog {
        header: catalog.header.clone(),
        entries,
    }
}

fn update() -> Result<()> {
    let template = extract()?;
    write_template(&template)?;
    println!("{TEMPLATE}: {} messages", template.entries.len());
    for (code, path) in catalog_files()? {
        let catalog = read_catalog(&path)?;
        let header = catalog::header(&catalog).map_err(|problem| {
            format!("{}:{}: {}", path.display(), problem.line, problem.reason)
        })?;
        let merged = merge(&catalog, &template, header.forms);
        write_catalog(&path, &merged)?;
        let counts = catalog::counts(&merged, &template);
        println!(
            "{code}.po: {} translated, {} fuzzy, {} untranslated ({}%)",
            counts.translated,
            counts.fuzzy,
            counts.untranslated,
            counts.percent()
        );
    }
    Ok(())
}

// --- Statistics and checks ---------------------------------------------------------------------

fn stats(markdown: bool) -> Result<()> {
    let template = read_catalog(&locales().join(TEMPLATE))?;
    if markdown {
        println!("| Language | Translated | To review | Untranslated | Done |");
        println!("| --- | ---: | ---: | ---: | ---: |");
    } else {
        println!(
            "{:<28}{:>12}{:>11}{:>14}{:>7}",
            "Language", "Translated", "To review", "Untranslated", "Done"
        );
    }
    for (code, path) in catalog_files()? {
        let catalog = read_catalog(&path)?;
        let name = catalog::header(&catalog).map_or_else(
            |_| code.clone(),
            |header| format!("{} ({code})", header.native_name),
        );
        let counts = catalog::counts(&catalog, &template);
        let translated = format!("{}/{}", counts.translated, counts.total);
        if markdown {
            println!(
                "| {name} | {translated} | {} | {} | {}% |",
                counts.fuzzy,
                counts.untranslated,
                counts.percent()
            );
        } else {
            println!(
                "{name:<28}{translated:>12}{:>11}{:>14}{:>6}%",
                counts.fuzzy,
                counts.untranslated,
                counts.percent()
            );
        }
    }
    Ok(())
}

fn check() -> Result<()> {
    let path = locales().join(TEMPLATE);
    let mut committed = read_catalog(&path)?;
    committed.clear_lines();
    let template = extract()?;
    if committed != template {
        return Err(format!(
            "{} is out of date with the sources: run `cargo xtask i18n update` and commit the catalogs",
            path.display()
        )
        .into());
    }
    let mut errors = 0;
    for (code, path) in catalog_files()? {
        let catalog = read_catalog(&path)?;
        let at = |line: usize| format!("{}:{line}", path.display());
        let header = match catalog::header(&catalog) {
            Ok(header) => header,
            Err(problem) => {
                eprintln!("error: {}: {}", at(problem.line), problem.reason);
                errors += 1;
                continue;
            }
        };
        if header.language != code {
            eprintln!(
                "error: {}: the header says Language: {}",
                at(catalog.header.line),
                header.language
            );
            errors += 1;
        }
        for problem in catalog::check(&catalog, &header) {
            let severity = match problem.severity {
                catalog::Severity::Error => {
                    errors += 1;
                    "error"
                }
                catalog::Severity::Warning => "warning",
            };
            eprintln!("{severity}: {}: {}", at(problem.line), problem.reason);
        }
        let missing = template
            .entries
            .iter()
            .filter(|wanted| {
                !catalog
                    .entries
                    .iter()
                    .any(|entry| !entry.obsolete && entry.key() == wanted.key())
            })
            .count();
        if missing > 0 {
            eprintln!(
                "error: {}: {missing} messages of the template are missing: run `cargo xtask i18n update`",
                path.display()
            );
            errors += 1;
        }
    }
    if errors > 0 {
        return Err(format!("{errors} translation error(s)").into());
    }
    println!("translations: OK");
    stats(false)
}

fn new_catalog(code: &str, name: &str, english: &str, plural_forms: &str) -> Result<()> {
    let path = locales().join(format!("{code}.po"));
    if path.exists() {
        return Err(format!("{} exists already", path.display()).into());
    }
    let valid = code.split('_').enumerate().all(|(i, part)| {
        !part.is_empty()
            && if i == 0 {
                part.chars().all(|c| c.is_ascii_lowercase())
            } else {
                part.chars().all(|c| c.is_ascii_alphanumeric())
            }
    });
    if !valid {
        return Err(format!("{code:?}: a code is a language in lowercase, then `_` and a region or script (`pt_BR`, `zh_Hans`)").into());
    }
    let template = read_catalog(&locales().join(TEMPLATE))?;
    let header = po::Entry {
        translator_comments: vec![format!("{english} translation of Koebi.")],
        strings: vec![
            [
                "Project-Id-Version: Koebi".to_owned(),
                "Report-Msgid-Bugs-To: https://github.com/Kolosta/Koebi/issues".to_owned(),
                "MIME-Version: 1.0".to_owned(),
                "Content-Type: text/plain; charset=UTF-8".to_owned(),
                "Content-Transfer-Encoding: 8bit".to_owned(),
                format!("Language: {code}"),
                format!("Plural-Forms: {plural_forms}"),
                format!("X-Native-Name: {name}"),
                format!("X-English-Name: {english}"),
                String::new(),
            ]
            .join("\n"),
        ],
        ..po::Entry::default()
    };
    let empty = po::Catalog {
        header,
        entries: Vec::new(),
    };
    let checked = catalog::header(&empty).map_err(|problem| problem.reason)?;
    write_catalog(&path, &merge(&empty, &template, checked.forms))?;
    println!(
        "{}: {} messages to translate",
        path.display(),
        template.entries.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(code: &str) -> Vec<(Option<String>, String, Option<String>)> {
        scan(code, "a.rs")
            .unwrap()
            .into_iter()
            .map(|found| (found.context, found.id, found.plural))
            .collect()
    }

    #[test]
    #[allow(
        clippy::needless_raw_string_hashes,
        reason = "the sample holds a raw string that `\"#` closes"
    )]
    fn marks_are_found_with_their_literals_only() {
        let code = r##"
            let a = tr("Open");
            let b = koebi_i18n::trc("menu", "Close");
            let c = trn("{count} file", "{count} files", n);
            static D: &str = n_!("Save");
            static K: &str = nc_!("key", "Delete");
            let e = tr(label); // marked where it is written
            let l = trc("key", name);
            let f = "tr(\"Not a mark\")";
            fn tr(x: &str) {}
            let g = text.tr("Method");
            let h = attr("Other");
            /* tr("Commented") */
            let i = tr(r#"Raw "quoted""#);
            let j = tr("Line \
                        continued");
        "##;
        let none = None::<String>;
        assert_eq!(
            ids(code),
            [
                (none.clone(), "Open".to_owned(), none.clone()),
                (Some("menu".to_owned()), "Close".to_owned(), none.clone()),
                (
                    none.clone(),
                    "{count} file".to_owned(),
                    Some("{count} files".to_owned())
                ),
                (none.clone(), "Save".to_owned(), none.clone()),
                (Some("key".to_owned()), "Delete".to_owned(), none.clone()),
                (none.clone(), "Raw \"quoted\"".to_owned(), none.clone()),
                (none.clone(), "Line continued".to_owned(), none),
            ]
        );
    }

    #[test]
    fn only_the_test_module_ends_the_scan() {
        let text = "a\n#[cfg(test)]\nfn helper() {}\nb\n#[cfg(test)]\nmod tests {}\n";
        assert_eq!(test_module(text), text.rfind("#[cfg(test)]"));
        assert_eq!(test_module("no tests"), None);
    }

    #[test]
    fn translator_comments_come_from_the_lines_above() {
        let code = "// Not for translators.\n// TRANSLATORS: the menu that opens files,\n// a verb.\nlet a = tr(\"Open\");\nlet b = tr(\"Close\");\n";
        let found = scan(code, "a.rs").unwrap();
        assert_eq!(
            found[0].comments,
            ["TRANSLATORS: the menu that opens files, a verb."]
        );
        assert_eq!(found[0].reference, "a.rs");
        assert_eq!(found[1].comments, [] as [String; 0]);
        assert!(scan("tr(\"a\" + x)", "a.rs").is_ok());
        assert!(scan("trc(\"\", \"a\")", "a.rs").is_err());
    }

    #[test]
    fn a_merge_keeps_translations_and_puts_old_ones_away() {
        let template = po::parse("msgid \"\"\nmsgstr \"\"\n\nmsgid \"Open\"\nmsgstr \"\"\n\nmsgid \"Save As\"\nmsgstr \"\"\n\nmsgid \"New\"\nmsgstr \"\"\n").unwrap();
        let catalog = po::parse("msgid \"\"\nmsgstr \"Language: fr\\n\"\n\nmsgid \"Open\"\nmsgstr \"Ouvrir\"\n\nmsgid \"Save as…\"\nmsgstr \"Enregistrer sous…\"\n\nmsgid \"Gone\"\nmsgstr \"Parti\"\n").unwrap();
        let merged = merge(&catalog, &template, 2);
        let summary: Vec<_> = merged
            .entries
            .iter()
            .map(|entry| {
                (
                    entry.id.as_str(),
                    entry.strings[0].as_str(),
                    entry.is_fuzzy(),
                    entry.obsolete,
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                ("Open", "Ouvrir", false, false),
                ("Save As", "Enregistrer sous…", true, false),
                ("New", "", false, false),
                ("Gone", "Parti", false, true),
            ]
        );
        assert_eq!(merged.header, catalog.header);
    }
}
