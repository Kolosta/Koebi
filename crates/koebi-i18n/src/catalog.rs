//! A translation read against the template: its header (language, names, plural rule), what
//! is wrong in it, and how much of the template it translates.
//!
//! The build script fails on the errors; `cargo xtask i18n check` also reports the warnings.
//! This module has no dependency: the build script and `cargo xtask i18n` include it as it is,
//! next to [`super::po`] and [`super::plural`].

use std::collections::HashSet;

use super::plural::{self, Op};
use super::po::{self, Catalog, Entry};

/// What the header of a translation declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    /// `Language`: its code, as the file is named (`fr`, `pt_BR`).
    pub language: String,
    /// `X-Native-Name`: its name in itself (`Français`).
    pub native_name: String,
    /// `X-English-Name`: its name in English (`French`).
    pub english_name: String,
    /// `nplurals` of `Plural-Forms`: how many forms a message with a count has.
    pub forms: usize,
    /// `plural` of `Plural-Forms`, compiled: which form a count takes.
    pub rule: Vec<Op>,
}

/// How much a problem matters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// The catalog cannot be used as it is: the build fails.
    Error,
    /// Probably a mistake, reported by `cargo xtask i18n check`.
    Warning,
}

/// A problem of a catalog, at a line of its file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// Its line, from 1; 0 for the whole file.
    pub line: usize,
    /// How much it matters.
    pub severity: Severity,
    /// What is wrong.
    pub reason: String,
}

impl Problem {
    fn error(line: usize, reason: impl Into<String>) -> Self {
        Self {
            line,
            severity: Severity::Error,
            reason: reason.into(),
        }
    }

    fn warning(line: usize, reason: impl Into<String>) -> Self {
        Self {
            line,
            severity: Severity::Warning,
            reason: reason.into(),
        }
    }
}

/// Counts beyond which a plural rule is not checked: they repeat the last hundreds.
const CHECKED_COUNTS: u64 = 1000;

/// Reads the header of a translation.
///
/// # Errors
///
/// The first field missing or wrong: `Language`, `X-Native-Name`, `X-English-Name`, or a
/// `Plural-Forms` whose rule does not compile or gives a form beyond `nplurals`.
pub fn header(catalog: &Catalog) -> Result<Header, Problem> {
    let line = catalog.header.line;
    let field = |name: &str| {
        catalog
            .header_field(name)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| Problem::error(line, format!("the header has no {name} field")))
    };
    let language = field("Language")?;
    let native_name = field("X-Native-Name")?;
    let english_name = field("X-English-Name")?;
    let plural_forms = field("Plural-Forms")?;
    let (forms, expression) = plural_forms_of(&plural_forms)
        .ok_or_else(|| Problem::error(line, "Plural-Forms reads `nplurals=N; plural=RULE;`"))?;
    if !(1..=6).contains(&forms) {
        return Err(Problem::error(line, "nplurals goes from 1 to 6"));
    }
    let rule = plural::compile(expression)
        .map_err(|reason| Problem::error(line, format!("the plural rule: {reason}")))?;
    if let Some(n) = (0..=CHECKED_COUNTS).find(|&n| plural::eval(&rule, n) >= forms as u64) {
        return Err(Problem::error(
            line,
            format!("the plural rule gives {n} a form beyond nplurals={forms}"),
        ));
    }
    Ok(Header {
        language,
        native_name,
        english_name,
        forms,
        rule,
    })
}

/// `nplurals` and the rule of a `Plural-Forms` value.
fn plural_forms_of(value: &str) -> Option<(usize, &str)> {
    let (count, rest) = value.split_once(';')?;
    let forms = count.trim().strip_prefix("nplurals")?.trim_start();
    let forms = forms.strip_prefix('=')?.trim().parse().ok()?;
    let rule = rest.trim().strip_prefix("plural")?.trim_start();
    let rule = rule.strip_prefix('=')?.trim();
    Some((forms, rule.trim_end_matches(';').trim()))
}

/// What is wrong with the messages of a translation whose header is `header`.
pub fn check(catalog: &Catalog, header: &Header) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut seen = HashSet::new();
    for entry in catalog.entries.iter().filter(|entry| !entry.obsolete) {
        if !seen.insert(entry.key()) {
            problems.push(Problem::error(
                entry.line,
                "this message is in the file twice",
            ));
        }
        if entry.context.as_deref() == Some("") {
            problems.push(Problem::error(entry.line, "an empty msgctxt: leave it out"));
        }
        check_strings(entry, header, &mut problems);
    }
    let used: Vec<_> = (0..=CHECKED_COUNTS)
        .map(|n| plural::eval(&header.rule, n))
        .collect();
    if let Some(form) = (0..header.forms as u64).find(|form| !used.contains(form)) {
        problems.push(Problem::warning(
            catalog.header.line,
            format!("no count takes plural form {form}"),
        ));
    }
    problems
}

fn check_strings(entry: &Entry, header: &Header, problems: &mut Vec<Problem>) {
    let translated: Vec<&String> = entry
        .strings
        .iter()
        .filter(|text| !text.is_empty())
        .collect();
    if translated.is_empty() {
        return;
    }
    match &entry.plural {
        Some(_) if entry.strings.len() != header.forms => problems.push(Problem::error(
            entry.line,
            format!(
                "{} plural forms where the language has {}",
                entry.strings.len(),
                header.forms
            ),
        )),
        None if entry.strings.len() > 1 => problems.push(Problem::error(
            entry.line,
            "plural forms for a message without msgid_plural",
        )),
        _ => {}
    }
    let mut source = po::placeholders(&entry.id);
    if let Some(plural) = &entry.plural {
        source.extend(po::placeholders(plural));
        source.sort_unstable();
        source.dedup();
    }
    let mut missing_everywhere = source.clone();
    for text in &translated {
        let found = po::placeholders(text);
        for name in &found {
            if !source.contains(name) {
                problems.push(Problem::error(
                    entry.line,
                    format!("{{{name}}} is not in the English message"),
                ));
            }
        }
        missing_everywhere.retain(|name| !found.contains(name));
        if entry.plural.is_none() {
            for name in source.iter().filter(|name| !found.contains(name)) {
                problems.push(Problem::warning(
                    entry.line,
                    format!("the translation leaves out {{{name}}}"),
                ));
            }
        }
    }
    if entry.plural.is_some() {
        for name in missing_everywhere {
            problems.push(Problem::warning(
                entry.line,
                format!("no plural form holds {{{name}}}"),
            ));
        }
    }
    let edges = |text: &str| {
        (
            text.starts_with(char::is_whitespace),
            text.ends_with(char::is_whitespace),
        )
    };
    if translated
        .iter()
        .any(|text| edges(text) != edges(&entry.id))
    {
        problems.push(Problem::warning(
            entry.line,
            "the translation starts or ends with spaces the English message does not, or the other way",
        ));
    }
}

/// How much of a template a translation covers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Messages of the template it translates.
    pub translated: usize,
    /// Messages it translates but marks to review: shown in English.
    pub fuzzy: usize,
    /// Messages of the template it does not translate.
    pub untranslated: usize,
    /// Messages of the template.
    pub total: usize,
}

impl Counts {
    /// The share translated, as a whole percentage rounded down.
    pub fn percent(&self) -> usize {
        (self.translated * 100)
            .checked_div(self.total)
            .unwrap_or(100)
    }
}

/// How much of `template` the translation `catalog` covers.
pub fn counts(catalog: &Catalog, template: &Catalog) -> Counts {
    let mut counts = Counts::default();
    for wanted in template.entries.iter().filter(|entry| !entry.obsolete) {
        counts.total += 1;
        let entry = catalog
            .entries
            .iter()
            .find(|entry| !entry.obsolete && entry.key() == wanted.key());
        match entry {
            Some(entry) if entry.is_translated() => counts.translated += 1,
            Some(entry) if entry.is_fuzzy() => counts.fuzzy += 1,
            _ => counts.untranslated += 1,
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "msgid \"\"\nmsgstr \"\"\n\"Language: fr\\n\"\n\"X-Native-Name: Français\\n\"\n\"X-English-Name: French\\n\"\n\"Plural-Forms: nplurals=2; plural=(n > 1);\\n\"\n";

    fn catalog(messages: &str) -> Catalog {
        po::parse(&format!("{HEADER}\n{messages}")).unwrap()
    }

    #[test]
    fn a_header_declares_the_language_and_its_plural_rule() {
        let header = header(&catalog("")).unwrap();
        assert_eq!(header.language, "fr");
        assert_eq!((header.native_name.as_str(), header.forms), ("Français", 2));
        assert_eq!(plural::eval(&header.rule, 2), 1);
        let broken = po::parse(
            "msgid \"\"\nmsgstr \"Language: fr\\nPlural-Forms: nplurals=1; plural=n;\\n\"\n",
        )
        .unwrap();
        assert!(super::header(&broken).is_err());
    }

    #[test]
    fn placeholders_and_plural_forms_are_checked() {
        let messages = "msgid \"{count} file\"\nmsgid_plural \"{count} files\"\nmsgstr[0] \"un fichier\"\nmsgstr[1] \"{count} fichiers\"\n\nmsgid \"Saved {name}\"\nmsgstr \"Enregistré {nom}\"\n\nmsgid \"One\"\nmsgid_plural \"Many\"\nmsgstr[0] \"Un\"\n";
        let catalog = catalog(messages);
        let header = header(&catalog).unwrap();
        let problems = check(&catalog, &header);
        let errors: Vec<_> = problems
            .iter()
            .filter(|problem| problem.severity == Severity::Error)
            .map(|problem| problem.line)
            .collect();
        // `{nom}` is unknown, and `One` has one form of two; `un fichier` is fine.
        assert_eq!(errors.len(), 2, "{problems:?}");
        assert!(
            problems
                .iter()
                .any(|problem| problem.severity == Severity::Warning
                    && problem.reason.contains("{name}"))
        );
    }

    #[test]
    fn counts_follow_the_template() {
        let template = po::parse("msgid \"\"\nmsgstr \"\"\n\nmsgid \"A\"\nmsgstr \"\"\n\nmsgid \"B\"\nmsgstr \"\"\n\nmsgid \"C\"\nmsgstr \"\"\n")
            .unwrap();
        let translated = catalog(
            "msgid \"A\"\nmsgstr \"a\"\n\n#, fuzzy\nmsgid \"B\"\nmsgstr \"b\"\n\nmsgid \"Gone\"\nmsgstr \"parti\"\n",
        );
        let counts = counts(&translated, &template);
        assert_eq!(
            counts,
            Counts {
                translated: 1,
                fuzzy: 1,
                untranslated: 1,
                total: 3
            }
        );
        assert_eq!(counts.percent(), 33);
    }
}
