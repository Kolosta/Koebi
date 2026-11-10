# Embedded fonts

| File | Font | Version | Weight |
|---|---|---|---|
| `NotoSans-Regular.ttf` | Noto Sans Regular | 2.015 | 400 |
| `NotoSans-Medium.ttf` | Noto Sans Medium | 2.015 | 500 |
| `NotoSansJP-Regular.ttf` | Noto Sans JP Regular | Google Fonts v57 | 400 |
| `NotoSansJP-Medium.ttf` | Noto Sans JP Medium | Google Fonts v57 | 500 |

Static TrueType instances of [Noto Sans](https://github.com/notofonts/latin-greek-cyrillic),
copyright 2022 The Noto Project Authors, licensed under the SIL Open Font License 1.1
([`OFL.txt`](OFL.txt)). The license allows embedding the fonts in Koebi; any distribution of
Koebi must carry the copyright notice and the license text.

Noto Sans JP (kana, kanji and the Latin it holds), copyright 2014-2021 Adobe, with Reserved Font
Name 'Source', is licensed under the SIL Open Font License 1.1 as well
([`OFL-NotoSansJP.txt`](OFL-NotoSansJP.txt)). These are the static TrueType instances Google
Fonts serves for the family; `Fonts::with_embedded` makes them the fallbacks of every style, so
that Japanese shows whatever the style's font (the interface's Japanese translation, Japanese
text in the gallery). Korean and Chinese come from the system's fonts when it has them
(`koebi_text::system`).

`koebi-text` embeds them with `include_bytes!` (`src/fonts.rs`); the design tokens refer to the
family as `NotoSans`. To add a weight, copy the static instance next to these files, list it
here, and register it in `Fonts::with_embedded`.
