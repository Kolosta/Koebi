# 0023. Translations: gettext catalogs compiled into the binary

- Status: Accepted
- Date: 2026-11-10
- Deciders: @Kolosta

## Context

The interface must be translatable at every level:
- menus, editors, settings, the gallery, tooltips and the status bar;
- action labels and descriptions, key names on caps;
- what widgets write themselves (the text field's actions, the color picker).

English is the source language. French and Japanese ship complete, and more languages come
from contributors.

Translators need tools they already know. That means catalogs carrying:
- a context, for identical words used in different places;
- plural forms by each language's rule;
- comments for translators, and marks for translations to review;
- completion figures.

The application draws text every frame, so a lookup must cost little and allocate nothing.
Runtime dependencies are limited to `ash` and `sdl3-sys` ([ADR 0003](0003-minimal-dependencies.md)).

## Decision

- **GNU gettext PO.**
  - The template is `crates/koebi-i18n/locales/koebi.pot`, with one catalog per language
    (`fr.po`, `ja.po`).
  - The messages are the English texts themselves (`msgid "Save"`), with `msgctxt` contexts,
    `msgid_plural` and `msgstr[n]` plural forms, `#.` comments taken from the sources, and
    `#, fuzzy` marks.
  - Two header fields name the language (`X-Native-Name`, `X-English-Name`).
- **A std-only crate, `koebi-i18n`:**
  - a PO reader and writer (`po.rs`);
  - plural rules, gettext's C subset compiled to a small stack program (`plural.rs`);
  - checks against the template (`catalog.rs`).
- **Compiled in.** The build script, which includes these modules as they are, checks every
  catalog and fails on the first error, naming the file and line:
  - a header without its fields;
  - a plural rule that does not compile or gives a form beyond `nplurals`;
  - a wrong number of plural forms;
  - a placeholder the English message does not hold;
  - a message listed twice.

  It then writes static tables sorted by context and message. Fuzzy and empty translations are
  left out and show in English.
- **API:**
  - `tr`, `trc`, `trn`, `trnc` return `&str` from the static tables, by binary search: no
    allocation, no lock;
  - `fill`, `fill_into` and `write_filled` replace named `{placeholders}`;
  - `n_!` marks text stored in statics, translated where it shows.

  The language is a global atomic, and `revision()` changes with it so that windows redraw.
- **Tooling, std-only, in `xtask`:** `cargo xtask i18n extract | update | stats | check | new`.
  - The extractor reads the literal arguments of the marks with a small Rust lexer (strings,
    raw strings, comments, character literals).
  - `update` merges like `msgmerge`.
  - `check` runs in CI. References name files without lines, so that moving code does not
    change the template.
- **Choice of language:** the saved setting, else `--lang`, else the system's preferred
  languages (`SDL_GetPreferredLocales`), else English.
- **Not translated:**
  - identifiers and saved formats;
  - logs and developer errors;
  - the documentation of design tokens, which is developer documentation;
  - what users type.

## Consequences

- Translators use Poedit, Lokalize, Weblate or a text editor. `cargo xtask i18n stats` and
  Settings › Language show each language's completion.
- Translating costs nothing at run time beyond a binary search per label, and a broken catalog
  never reaches a build.
- Every message change requires `cargo xtask i18n update`, and CI enforces it. New messages
  show in English until translated.
- The catalogs add about 100 KiB of strings to the binary per complete language.
- Developers mark every shown string. Text kept in long-lived structures must be read again
  when the language changes.
- Right-to-left and complex scripts can be translated but not laid out until text shaping
  covers them.

## Alternatives considered

- **Project Fluent (`.ftl`).** Its message syntax handles grammar (gender, cases) better than
  gettext's plural forms alone. However:
  - its crates are runtime dependencies, and writing a Fluent parser and resolver ourselves is
    far larger than PO;
  - fewer translation tools read it;
  - messages are addressed by identifiers, which drift from the English they stand for.
- **ICU MessageFormat.** Its plural and select rules are powerful, but the format is complex
  to implement without ICU, and PO tools only partly support it.
- **Our own TOML or JSON files.** These need no parser beyond a small one, but no translation
  tool, no fuzzy marks and no plural conventions come with them.
- **Platform resources** (Android `strings.xml`, Apple `.strings`). There would be one format
  per platform, while Koebi draws its own interface everywhere.
- **Loading catalogs at run time from files.** This would let translators test without
  building, but a broken file would be found only at run time, and the files would have to be
  packaged on every platform (Android assets, iOS bundles). It may come later as a development
  option, reusing `po.rs`.
