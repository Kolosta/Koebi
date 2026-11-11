//! PO files (GNU gettext): the format translators and their tools read and write (Poedit,
//! Weblate, Crowdin, Lokalize, Virtaal).
//!
//! What Koebi uses of it: translator comments (`# `), comments for translators taken from the
//! sources (`#. `), references (`#: file:line`), flags (`#, fuzzy`), contexts (`msgctxt`),
//! singular and plural messages (`msgid`, `msgid_plural`, `msgstr`, `msgstr[n]`), obsolete
//! entries (`#~ `) and the header (the entry of the empty `msgid`). Strings use C escapes
//! (`\n`, `\t`, `\"`, `\\`) and may continue on the next lines.
//!
//! This module has no dependency: the build script and `cargo xtask i18n` include it as it is.

use std::fmt::Write as _;

/// One message of a catalog.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    /// Comments of the translators (`# `).
    pub translator_comments: Vec<String>,
    /// Comments for the translators, taken from the sources (`#. `).
    pub comments: Vec<String>,
    /// Where the message is used (`#: path:line`).
    pub references: Vec<String>,
    /// Flags (`#, fuzzy`): `fuzzy` marks a translation to review, not shown.
    pub flags: Vec<String>,
    /// What disambiguates two identical messages (`msgctxt`).
    pub context: Option<String>,
    /// The message in English, the source language.
    pub id: String,
    /// Its plural form, for messages with a count.
    pub plural: Option<String>,
    /// The translations: one, or one per plural form.
    pub strings: Vec<String>,
    /// No longer in the sources, kept for its translation (`#~ `).
    pub obsolete: bool,
    /// The line of its `msgid` in the file, for errors.
    pub line: usize,
}

impl Entry {
    /// `true` when it is marked to review.
    pub fn is_fuzzy(&self) -> bool {
        self.flags.iter().any(|flag| flag == "fuzzy")
    }

    /// `true` when it holds a translation shown in the interface: not fuzzy, not obsolete, and
    /// every form translated.
    pub fn is_translated(&self) -> bool {
        !self.is_fuzzy()
            && !self.obsolete
            && !self.strings.is_empty()
            && self.strings.iter().all(|text| !text.is_empty())
    }

    /// The key of the message: its context and its text.
    pub fn key(&self) -> (Option<&str>, &str) {
        (self.context.as_deref(), &self.id)
    }
}

/// A catalog: its header and its messages, in file order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Catalog {
    /// The header (`msgid ""`), its fields one per line (`Language: fr`).
    pub header: Entry,
    /// The messages.
    pub entries: Vec<Entry>,
}

impl Catalog {
    /// Sets every entry's line to 0: two catalogs then compare by what they hold, not by where
    /// it lies in their files.
    pub fn clear_lines(&mut self) {
        self.header.line = 0;
        for entry in &mut self.entries {
            entry.line = 0;
        }
    }

    /// The value of the header field `name` (`Language`, `Plural-Forms`).
    pub fn header_field(&self, name: &str) -> Option<&str> {
        let text = self.header.strings.first()?;
        text.lines().find_map(|line| {
            let (field, value) = line.split_once(':')?;
            (field.trim() == name).then(|| value.trim())
        })
    }
}

/// A line of a PO file that cannot be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// Its line, from 1.
    pub line: usize,
    /// What is wrong.
    pub reason: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.reason)
    }
}

/// The value of a quoted PO string (`"..."`), its escapes resolved.
fn unquote(text: &str, line: usize) -> Result<String, ParseError> {
    let error = |reason: &str| ParseError {
        line,
        reason: reason.to_owned(),
    };
    let inner = text
        .trim()
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .ok_or_else(|| error("a string goes between double quotes"))?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            if c == '"' {
                return Err(error("a double quote inside a string is written \\\""));
            }
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            other => return Err(error(&format!("unknown escape \\{}", other.unwrap_or(' ')))),
        }
    }
    Ok(out)
}

/// `text` as a quoted PO string.
fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Which field the next continuation line goes to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    None,
    Context,
    Id,
    Plural,
    String(usize),
}

/// Reads a PO file.
///
/// # Errors
///
/// The first line that is not PO: a keyword out of place, a broken string, an unknown escape.
pub fn parse(text: &str) -> Result<Catalog, ParseError> {
    let mut entries: Vec<Entry> = Vec::new();
    let mut current = Entry::default();
    let mut field = Field::None;
    let mut started = false;
    let mut finish = |current: &mut Entry, started: &mut bool| {
        if *started {
            entries.push(core::mem::take(current));
        }
        *started = false;
    };
    for (number, raw) in text.lines().enumerate() {
        let line = number + 1;
        let mut content = raw.trim();
        let obsolete = content.starts_with("#~");
        if obsolete {
            content = content[2..].trim_start();
        }
        if content.is_empty() {
            continue;
        }
        let error = |reason: &str| ParseError {
            line,
            reason: reason.to_owned(),
        };
        // A comment after a message's strings starts the next message.
        let starts_entry = content.starts_with('#')
            || content.starts_with("msgctxt")
            || (content.starts_with("msgid ") && field != Field::Context);
        if starts_entry && matches!(field, Field::String(_)) {
            finish(&mut current, &mut started);
            field = Field::None;
        }
        if let Some(comment) = content.strip_prefix('#') {
            if let Some(rest) = comment.strip_prefix('.') {
                current.comments.push(rest.trim().to_owned());
            } else if let Some(rest) = comment.strip_prefix(':') {
                current
                    .references
                    .extend(rest.split_whitespace().map(str::to_owned));
            } else if let Some(rest) = comment.strip_prefix(',') {
                current.flags.extend(
                    rest.split(',')
                        .map(|flag| flag.trim().to_owned())
                        .filter(|flag| !flag.is_empty()),
                );
            } else if comment.starts_with('|') {
                // A previous message: not kept.
            } else {
                current
                    .translator_comments
                    .push(comment.trim_start().to_owned());
            }
            started = true;
            continue;
        }
        started = true;
        current.obsolete |= obsolete;
        if let Some(rest) = content.strip_prefix("msgctxt ") {
            current.context = Some(unquote(rest, line)?);
            field = Field::Context;
        } else if let Some(rest) = content.strip_prefix("msgid_plural ") {
            current.plural = Some(unquote(rest, line)?);
            field = Field::Plural;
        } else if let Some(rest) = content.strip_prefix("msgid ") {
            current.id = unquote(rest, line)?;
            current.line = line;
            field = Field::Id;
        } else if let Some(rest) = content.strip_prefix("msgstr[") {
            let (index, value) = rest
                .split_once(']')
                .ok_or_else(|| error("msgstr[n] needs its ]"))?;
            let index: usize = index
                .parse()
                .map_err(|_| error("a plural form is a number"))?;
            if index != current.strings.len() {
                return Err(error("plural forms go in order from msgstr[0]"));
            }
            current.strings.push(unquote(value, line)?);
            field = Field::String(index);
        } else if let Some(rest) = content.strip_prefix("msgstr ") {
            current.strings = vec![unquote(rest, line)?];
            field = Field::String(0);
        } else if content.starts_with('"') {
            let more = unquote(content, line)?;
            match field {
                Field::Context => current
                    .context
                    .get_or_insert_with(String::new)
                    .push_str(&more),
                Field::Id => current.id.push_str(&more),
                Field::Plural => current
                    .plural
                    .get_or_insert_with(String::new)
                    .push_str(&more),
                Field::String(index) => current.strings[index].push_str(&more),
                Field::None => return Err(error("a string without a keyword before it")),
            }
        } else {
            return Err(error("not a PO line"));
        }
    }
    finish(&mut current, &mut started);
    let mut catalog = Catalog::default();
    for entry in entries {
        if entry.id.is_empty() && entry.context.is_none() && !entry.obsolete {
            catalog.header = entry;
        } else {
            catalog.entries.push(entry);
        }
    }
    Ok(catalog)
}

/// A string written over lines of at most 80 columns, as `msgmerge` does: the first line empty,
/// then pieces cut after spaces and after line feeds.
fn write_string(out: &mut String, keyword: &str, text: &str, prefix: &str) {
    let single = format!("{prefix}{keyword} {}", quote(text));
    if single.chars().count() <= 80 && !text.trim_end_matches('\n').contains('\n') {
        out.push_str(&single);
        out.push('\n');
        return;
    }
    let _ = writeln!(out, "{prefix}{keyword} \"\"");
    let mut piece = String::new();
    let mut width = 0;
    for c in text.chars() {
        piece.push(c);
        width += 1;
        if c == '\n' || (c == ' ' && width >= 60) {
            let _ = writeln!(out, "{prefix}{}", quote(&piece));
            piece.clear();
            width = 0;
        }
    }
    if !piece.is_empty() {
        let _ = writeln!(out, "{prefix}{}", quote(&piece));
    }
}

fn write_entry(out: &mut String, entry: &Entry) {
    let prefix = if entry.obsolete { "#~ " } else { "" };
    for comment in &entry.translator_comments {
        let _ = writeln!(out, "# {comment}");
    }
    for comment in &entry.comments {
        let _ = writeln!(out, "#. {comment}");
    }
    for reference in &entry.references {
        let _ = writeln!(out, "#: {reference}");
    }
    if !entry.flags.is_empty() {
        let _ = writeln!(out, "#, {}", entry.flags.join(", "));
    }
    if let Some(context) = &entry.context {
        write_string(out, "msgctxt", context, prefix);
    }
    write_string(out, "msgid", &entry.id, prefix);
    if let Some(plural) = &entry.plural {
        write_string(out, "msgid_plural", plural, prefix);
        for (index, text) in entry.strings.iter().enumerate() {
            write_string(out, &format!("msgstr[{index}]"), text, prefix);
        }
    } else {
        write_string(
            out,
            "msgstr",
            entry.strings.first().map_or("", String::as_str),
            prefix,
        );
    }
}

/// Writes a catalog: the header, then the messages, then the obsolete ones, a blank line apart.
pub fn write(catalog: &Catalog) -> String {
    let mut out = String::new();
    write_entry(&mut out, &catalog.header);
    let (current, obsolete): (Vec<&Entry>, Vec<&Entry>) =
        catalog.entries.iter().partition(|entry| !entry.obsolete);
    for entry in current.into_iter().chain(obsolete) {
        out.push('\n');
        write_entry(&mut out, entry);
    }
    out
}

/// The placeholders of a message (`{name}`, `{count}`), sorted, each once: a translation must
/// hold the same.
pub fn placeholders(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else {
            break;
        };
        let name = &after[..end];
        if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            found.push(name);
        }
        rest = &after[end + 1..];
    }
    found.sort_unstable();
    found.dedup();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"# French translation of Koebi.
msgid ""
msgstr ""
"Language: fr\n"
"Plural-Forms: nplurals=2; plural=(n > 1);\n"

# A translator's note.
#. TRANSLATORS: the menu that opens files.
#: apps/koebi/src/chrome.rs:12
msgctxt "menu"
msgid "Open"
msgstr "Ouvrir"

#, fuzzy
msgid "Save as…"
msgstr "Enregistrer sous…"

msgid "{count} file"
msgid_plural "{count} files"
msgstr[0] "{count} fichier"
msgstr[1] "{count} fichiers"

msgid ""
"A long message "
"on two lines."
msgstr "Un long message en deux lignes."

#~ msgid "Gone"
#~ msgstr "Parti"
"#;

    #[test]
    fn a_catalog_reads_every_kind_of_entry() {
        let catalog = parse(SAMPLE).unwrap();
        assert_eq!(catalog.header_field("Language"), Some("fr"));
        assert_eq!(
            catalog.header_field("Plural-Forms"),
            Some("nplurals=2; plural=(n > 1);")
        );
        assert_eq!(catalog.entries.len(), 5);
        let open = &catalog.entries[0];
        assert_eq!(open.key(), (Some("menu"), "Open"));
        assert_eq!(open.strings, ["Ouvrir"]);
        assert_eq!(open.comments, ["TRANSLATORS: the menu that opens files."]);
        assert_eq!(open.references, ["apps/koebi/src/chrome.rs:12"]);
        assert!(open.is_translated());
        assert!(catalog.entries[1].is_fuzzy() && !catalog.entries[1].is_translated());
        assert_eq!(catalog.entries[2].strings.len(), 2);
        assert_eq!(catalog.entries[3].id, "A long message on two lines.");
        assert_eq!(
            catalog.entries[3].strings,
            ["Un long message en deux lignes."]
        );
        assert!(catalog.entries[4].obsolete);
    }

    #[test]
    fn a_catalog_written_reads_back_the_same() {
        let mut catalog = parse(SAMPLE).unwrap();
        let mut again = parse(&write(&catalog)).unwrap();
        catalog.clear_lines();
        again.clear_lines();
        assert_eq!(again, catalog);
    }

    #[test]
    fn broken_lines_say_where_they_are() {
        let error = parse("msgid \"a\"\nmsgstr \"b\\q\"\n").unwrap_err();
        assert_eq!(error.line, 2);
        assert!(parse("msgid \"a\nmsgstr \"b\"\n").is_err());
        assert_eq!(parse("hello\n").unwrap_err().line, 1);
    }

    #[test]
    fn placeholders_are_named_in_braces() {
        assert_eq!(
            placeholders("Saved {count} of {total} in {count}"),
            ["count", "total"]
        );
        assert_eq!(placeholders("No {} or { space }"), [] as [&str; 0]);
    }
}
