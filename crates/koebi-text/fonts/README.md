# Embedded fonts

| File | Font | Version | Weight |
|---|---|---|---|
| `NotoSans-Regular.ttf` | Noto Sans Regular | 2.015 | 400 |
| `NotoSans-Medium.ttf` | Noto Sans Medium | 2.015 | 500 |

Static TrueType instances of [Noto Sans](https://github.com/notofonts/latin-greek-cyrillic),
copyright 2022 The Noto Project Authors, licensed under the SIL Open Font License 1.1
([`OFL.txt`](OFL.txt)). The license allows embedding the fonts in Koebi; any distribution of
Koebi must carry the copyright notice and the license text.

`koebi-text` embeds them with `include_bytes!` (`src/fonts.rs`); the design tokens refer to the
family as `NotoSans`. To add a weight, copy the static instance next to these files, list it
here, and register it in `Fonts::with_embedded`.
