# Translating Koebi

Koebi's interface is written in English, the source language, and translated into other
languages with [GNU gettext](https://www.gnu.org/software/gettext/manual/) catalogs: the
format that Poedit, Weblate, Crowdin, Lokalize and Virtaal read and write. The decision and
its alternatives are in [ADR 0023](../adr/0023-translations.md).

| Language | Catalog | Status |
| --- | --- | --- |
| English | the sources | source language |
| Français (French) | `fr.po` | complete |
| 日本語 (Japanese) | `ja.po` | complete, to review by a native speaker |

`cargo xtask i18n stats` prints the current numbers (`--markdown` for this table), and
Settings › Language shows each language's completion.

## How it works

- Every message the interface shows is written in English in the sources, marked by a function
  of `koebi-i18n` (`tr("Save")`, see [For developers](#for-developers)).
- `cargo xtask i18n extract` collects the marked messages into the **template**
  `crates/koebi-i18n/locales/koebi.pot`. Nobody edits it by hand.
- Each language has a **catalog**, `crates/koebi-i18n/locales/<code>.po`: the template's
  messages with their translations.
- At build time, `koebi-i18n`'s build script checks every catalog and compiles it into static
  tables. A failing check fails the build and names the file and line. Looking a message up is
  a binary search, with no allocation or lock.
- At run time, Koebi shows the language saved in the settings or given by `--lang <code>`. By
  default it uses the system's first preferred language that has a catalog, else English.
  Changing the language redraws every window at once.
- A message the catalog does not translate, or marks *fuzzy* (to review), shows in English.

## For translators

You need no Rust to translate: a PO editor, or any text editor, is enough.

### Tools

- [Poedit](https://poedit.net/) (Windows, macOS, Linux; free), recommended. It shows the
  English next to the translation, the comments for translators and the plural forms, and it
  checks placeholders.
- [Lokalize](https://apps.kde.org/lokalize/) (KDE) and
  [Gtranslator](https://wiki.gnome.org/Apps/Gtranslator) (GNOME) on Linux.
- Web platforms (Weblate, Crowdin) read the same files. Koebi does not run one yet: the
  catalogs in the repository are the reference, and pull requests bring changes in.
- A text editor works too. Keep the file in UTF-8 and the quotes and `\n` escapes as they are.

### Workflow

1. Fork the repository, then make a branch from `develop`, `feature/<issue>-<language>-translation` (see
   [gitflow](gitflow.md)).
2. With Rust installed, bring the catalogs up to date first: `cargo xtask i18n update`.
3. Open `crates/koebi-i18n/locales/<code>.po` in your editor and translate. Poedit lists the
   untranslated messages first, then the fuzzy ones.
4. Check the catalog: `cargo xtask i18n check`. It reports errors (a placeholder that does not
   exist, a wrong number of plural forms) and warnings (a placeholder left out, spaces at the
   ends that differ from the English).
5. See the result: `cargo run -p koebi -- --lang <code>`.
6. Commit with a Conventional Commits message (`feat(i18n): complete the German translation`)
   and open a pull request to `develop`. CI runs the same check (*Translations*).

Without Rust, edit the `.po` file on GitHub and open the pull request from there: CI checks it.

### What a catalog holds

```po
#. TRANSLATORS: the menu that opens files, a verb.
#: apps/koebi/src/chrome.rs
msgid "Open"
msgstr "Ouvrir"

#, fuzzy
msgid "Save as…"
msgstr "Enregistrer sous…"

msgctxt "key"
msgid "Space"
msgstr "Espace"

msgid "{count} file"
msgid_plural "{count} files"
msgstr[0] "{count} fichier"
msgstr[1] "{count} fichiers"
```

- `#.` lines are comments from the developers for you, and `#:` lines are the files that use
  the message.
- `#, fuzzy` marks a translation to review. `update` adds it when the English changed only in
  case or punctuation. Check the translation, then remove the mark (Poedit: *Needs work*).
  Until then the message shows in English.
- `msgctxt` tells apart identical English words used in different places:
  - `key` is a key's name on its cap (`Space`, `Esc`, `L Ctrl`);
  - `shortcut` is what follows a key in a shortcut (`release`, `double click`, `drag up`).
- `{count}`, `{name}`: **placeholders**, which Koebi fills.
  - Keep their names as they are. Move them where your language puts them.
  - A plural form may leave `{count}` out when it says the number in words.
  - `{{` and `}}` write a brace.
- Plural forms: the header's `Plural-Forms` gives the number of forms and the rule that picks
  one for a count. Give one `msgstr[n]` per form.

### Conventions

All languages:

- Translate the meaning in the interface, not the words. Keep it short: labels share their
  space with others.
- Follow the operating system's own vocabulary in your language (Microsoft's and Apple's style
  guides and terminology are good references).
- Action labels read as the action (`Save`, `Delete messages`). Descriptions are one sentence
  in the third person (`Closes every window and quits.`).
- Key names are the ones printed on keyboards in your language.

French:

- Labels and menu items use the infinitive (*Enregistrer*, *Supprimer les messages*).
  Descriptions use the indicative (*Ferme toutes les fenêtres et quitte.*).
- Capitalize only the first word and proper nouns.
- Typography:
  - a no-break space (U+00A0) goes before `:`, and a narrow no-break space (U+202F) before
    `;`, `!` and `?`;
  - use the ellipsis `…` and the apostrophe `'` as the English does;
  - quotes are « » with no-break spaces inside.
- Keyboard: *Maj* (Shift), *Ctrl*, *Alt*, *Entrée*, *Échap*, *Suppr*, *Espace*, *Retour
  arrière*, *Tab*, *Verr. maj.*

Japanese:

- Labels and menu items are nouns or short verb phrases (保存, メッセージを削除).
  Descriptions and messages use the polite style (〜します).
- Use full-width Japanese punctuation (、。「」), and half-width digits and Latin letters.
  Put no space between Japanese and Latin text.
- Katakana loanwords keep the final long vowel (ユーザー, フォルダー), as Microsoft's
  Japanese style guide does.
- Keys keep the names printed on Japanese keyboards, mostly in Latin letters (Shift, Ctrl,
  Enter, Esc).

### Adding a language

```sh
cargo xtask i18n new de --name Deutsch --english German --plural "nplurals=2; plural=(n != 1);"
```

This writes `de.po` with every message of the template to translate. The code is the
language in lowercase, optionally followed by `_` and a region or script (`pt_BR`, `zh_Hans`).
Plural rules of common languages, from the CLDR and the gettext manual:

| Languages | `Plural-Forms` |
| --- | --- |
| Japanese, Chinese, Korean, Vietnamese, Thai | `nplurals=1; plural=0;` |
| English, German, Dutch, Swedish, Italian, Spanish, Portuguese (Portugal) | `nplurals=2; plural=(n != 1);` |
| French, Brazilian Portuguese | `nplurals=2; plural=(n > 1);` |
| Russian, Ukrainian | `nplurals=3; plural=(n%10==1 && n%100!=11 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 \|\| n%100>=20) ? 1 : 2);` |
| Polish | `nplurals=3; plural=(n==1 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 \|\| n%100>=20) ? 1 : 2);` |
| Czech, Slovak | `nplurals=3; plural=(n==1) ? 0 : (n>=2 && n<=4) ? 1 : 2;` |

Right-to-left languages (Arabic, Hebrew) and scripts that need complex shaping (Devanagari,
Thai) can be translated, but Koebi does not lay them out correctly yet: text is shaped for
Latin and CJK written left to right ([ADR 0022](../adr/0022-text-on-the-gpu.md)). Chinese and
Korean show with the system's fonts; Japanese has its own embedded font.

## For developers

Every text the interface shows goes through `koebi-i18n`.

| Function | When |
| --- | --- |
| `tr("Save")` | a message |
| `trc("key", "Space")` | the same English word means different things in two places |
| `trn("{count} file", "{count} files", n)` | a message with a count, then `fill` |
| `fill(text, &[("count", &n)])`, `fill_into(&mut buffer, …)` | placeholders; `fill_into` and `write_filled` reuse a buffer, for text built every frame |
| `n_!("Save")` | text stored before it shows (action labels and descriptions in statics, tables in `const`), translated with `tr(label)` where it shows |

Rules:

- **Whole sentences, with placeholders.** Never build a sentence from translated pieces
  (`tr("Delete") + " " + name`). Word order differs between languages: write
  `fill(tr("Delete {name}"), &[("name", &name)])`.
- **Never translate:**
  - identifiers and anything saved or parsed (token paths, action ids, keymap and workspace
    files, key names in shortcuts' text form);
  - log messages and developer errors;
  - the documentation of design tokens, which is developer documentation and stays in English;
  - what users typed (layer names, documents).
- **Translate where the text shows**, every frame. A label kept in a long-lived structure must
  be read again when `koebi_i18n::revision()` changes: the windows redraw then.
- **Help the translators:**
  - put a `// TRANSLATORS:` comment on the line just above a mark when a message is short or
    ambiguous;
  - use a context (`trc`) when the same English word must translate differently.
- **After adding, changing or removing messages**, run `cargo xtask i18n update` and commit the
  template and the catalogs with the code. CI fails when the template is out of date. The new
  messages show in English in every language until they are translated.

The extractor (`xtask/src/i18n.rs`) reads the literal arguments of `tr`, `trc`, `trn`, `trnc`
and `n_!` in `apps/` and `crates/`, outside test modules. A call whose message is not a literal
(`tr(action.label)`) is skipped: the message is marked where it is written, with `n_!`.
