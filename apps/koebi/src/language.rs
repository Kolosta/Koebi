//! The language of the interface: English until another is chosen in the settings, a language
//! or the system's, kept in the data directory; `--lang` shows another for one session without
//! changing the choice.
//!
//! The choice is a file of one line, `language.txt`: a language's code (`fr`), or `system`.

use std::path::Path;

use koebi_i18n::Language;

use crate::saved::SavedFile;

/// What `language.txt` holds when the interface follows the system.
const SYSTEM: &str = "system";

/// What the settings choose.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Choice {
    /// This language.
    Language(&'static Language),
    /// The system's first preferred language that Koebi shows, else English.
    System,
}

impl PartialEq for Choice {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Language(a), Self::Language(b)) => core::ptr::eq(*a, *b),
            (Self::System, Self::System) => true,
            _ => false,
        }
    }
}

/// The language of the interface, and where it comes from.
#[derive(Debug)]
pub(crate) struct LanguageSetting {
    /// Chosen in the settings; `None` until one is: the interface is then in English.
    chosen: Option<Choice>,
    /// The system's preferred languages, best first (`fr-FR`, `en`).
    system: Vec<String>,
    /// `--lang`: shown for this session, over the choice, which it leaves as it is.
    session: Option<&'static Language>,
    /// Where the choice is kept; `None` without a data directory.
    file: Option<SavedFile>,
}

impl LanguageSetting {
    /// Reads the choice kept in `dir`. The language it gives, [`Self::shown`], is `session` if
    /// any, else the choice, else English: [`Self::apply`] shows it.
    pub(crate) fn new(
        dir: Option<&Path>,
        system: Vec<String>,
        session: Option<&'static Language>,
    ) -> Self {
        let file = dir.map(|dir| SavedFile::new(dir, "language.txt"));
        let chosen = file.as_ref().and_then(read_choice);
        Self {
            chosen,
            system,
            session,
            file,
        }
    }

    /// The language the system prefers among those Koebi shows, else English.
    pub(crate) fn system_language(&self) -> &'static Language {
        koebi_i18n::matching(self.system.iter().map(String::as_str))
            .unwrap_or_else(koebi_i18n::english)
    }

    /// What the settings chose; `None` until they do (the interface is in English).
    pub(crate) fn chosen(&self) -> Option<Choice> {
        self.chosen
    }

    /// The language shown.
    pub(crate) fn shown(&self) -> &'static Language {
        if let Some(language) = self.session {
            return language;
        }
        match self.chosen {
            Some(Choice::Language(language)) => language,
            Some(Choice::System) => self.system_language(),
            None => koebi_i18n::english(),
        }
    }

    /// Chooses `choice` and keeps it; [`Self::apply`] shows it. A session's `--lang` gives way
    /// to the choice.
    pub(crate) fn choose(&mut self, choice: Choice) {
        self.chosen = Some(choice);
        self.session = None;
        let Some(file) = &self.file else {
            return;
        };
        let line = match choice {
            Choice::Language(language) => language.code(),
            Choice::System => SYSTEM,
        };
        if let Err(error) = file.write(&format!("{line}\n")) {
            koebi_core::warn!("cannot save the language: {error}");
        }
    }

    /// Shows the interface in [`Self::shown`]: every window redraws in it.
    pub(crate) fn apply(&self) {
        let shown = self.shown();
        if koebi_i18n::set_language(shown) || shown.code() != "en" {
            koebi_core::info!(
                "interface in {} ({}%)",
                shown.english_name(),
                shown.percent()
            );
        }
    }
}

/// The choice kept in `file`; `None` for a missing file or a language Koebi no longer shows.
fn read_choice(file: &SavedFile) -> Option<Choice> {
    let texts = match file.read() {
        Ok(texts) => texts,
        Err(error) => {
            koebi_core::warn!("cannot read the language: {error}");
            return None;
        }
    };
    let line = texts.first()?.lines().next()?.trim().to_owned();
    if line == SYSTEM {
        return Some(Choice::System);
    }
    if line.is_empty() {
        return None;
    }
    let found = koebi_i18n::find(&line);
    if found.is_none() {
        koebi_core::warn!("language {line:?} is not one Koebi shows: the interface is in English");
    }
    found.map(Choice::Language)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directory(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("koebi-language-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        dir
    }

    #[test]
    fn english_until_a_language_or_the_system_is_chosen_and_kept() {
        let dir = directory("choice");
        let french = koebi_i18n::find("fr").unwrap();
        let japanese = koebi_i18n::find("ja").unwrap();
        // Whatever the system prefers, English at first.
        let mut setting =
            LanguageSetting::new(Some(&dir), vec!["de-DE".into(), "fr-CA".into()], None);
        assert_eq!(setting.shown().code(), "en");
        assert!(setting.chosen().is_none());
        // The system's: its first language Koebi shows.
        setting.choose(Choice::System);
        assert_eq!(setting.shown().code(), "fr");
        setting.choose(Choice::Language(japanese));
        // A new start reads the choice back; a session's language does not change it.
        let again = LanguageSetting::new(Some(&dir), vec!["fr".into()], Some(french));
        assert_eq!(again.shown().code(), "fr");
        assert_eq!(again.chosen(), Some(Choice::Language(japanese)));
        let mut again = LanguageSetting::new(Some(&dir), Vec::new(), None);
        assert_eq!(again.shown().code(), "ja");
        again.choose(Choice::System);
        let back = LanguageSetting::new(Some(&dir), vec!["ja-JP".into()], None);
        assert_eq!(back.chosen(), Some(Choice::System));
        assert_eq!(back.shown().code(), "ja");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_language_koebi_does_not_show_gives_english() {
        let dir = directory("unknown");
        let file = SavedFile::new(&dir, "language.txt");
        file.write("xx\n").unwrap();
        assert!(read_choice(&file).is_none());
        file.write("system\n").unwrap();
        assert_eq!(read_choice(&file), Some(Choice::System));
        file.write("ja\n").unwrap();
        assert_eq!(
            read_choice(&file),
            Some(Choice::Language(koebi_i18n::find("ja").unwrap()))
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
