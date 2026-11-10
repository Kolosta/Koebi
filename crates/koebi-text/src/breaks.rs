//! Where lines may break: the part of Unicode's line breaking algorithm (UAX #14) that Latin and
//! CJK text written left to right need.
//!
//! - A line must break after a line feed.
//! - It may break after spaces (the space stays at the end of the line, hanging past its width)
//!   and after hyphens and dashes followed by a letter or a digit.
//! - It may break before and after every ideograph, kana and Hangul syllable, except where the
//!   Japanese *kinsoku* rules forbid it: never before closing punctuation, small kana, the
//!   prolonged sound mark, iteration marks and sentence-ending marks; never after opening
//!   punctuation.
//! - Nowhere else: a word stays whole (an overlong word breaks between any two characters, which
//!   the layout decides).

/// `true` for characters written in ideographic, kana or Hangul scripts.
pub fn is_cjk(c: char) -> bool {
    matches!(u32::from(c),
        0x2E80..=0x2FFF   // radicals, Kangxi
        | 0x3000..=0x303F // CJK symbols and punctuation
        | 0x3040..=0x30FF // Hiragana, Katakana
        | 0x3100..=0x31FF // Bopomofo, Hangul compatibility jamo, Kanbun, Katakana extensions
        | 0x3400..=0x4DBF // CJK extension A
        | 0x4E00..=0x9FFF // CJK unified ideographs
        | 0xA960..=0xA97F // Hangul jamo extended A
        | 0xAC00..=0xD7AF // Hangul syllables
        | 0xF900..=0xFAFF // CJK compatibility ideographs
        | 0xFF00..=0xFFEF // half-width and full-width forms
        | 0x20000..=0x3FFFF // supplementary ideographic planes
    )
}

/// `true` for characters a line never starts with (*kinsoku*: closing punctuation, small kana,
/// the prolonged sound mark, iteration marks, sentence-ending marks).
pub fn no_break_before(c: char) -> bool {
    matches!(
        c,
        '、' | '。'
            | '，'
            | '．'
            | '）'
            | '」'
            | '』'
            | '】'
            | '〕'
            | '〉'
            | '》'
            | '〙'
            | '〗'
            | '〟'
            | '’'
            | '”'
            | '｠'
            | '»'
            | ')'
            | ']'
            | '}'
            | ','
            | '.'
            | ':'
            | ';'
            | '!'
            | '?'
            | 'ぁ'
            | 'ぃ'
            | 'ぅ'
            | 'ぇ'
            | 'ぉ'
            | 'っ'
            | 'ゃ'
            | 'ゅ'
            | 'ょ'
            | 'ゎ'
            | 'ゕ'
            | 'ゖ'
            | 'ァ'
            | 'ィ'
            | 'ゥ'
            | 'ェ'
            | 'ォ'
            | 'ッ'
            | 'ャ'
            | 'ュ'
            | 'ョ'
            | 'ヮ'
            | 'ヵ'
            | 'ヶ'
            | 'ー'
            | 'ゝ'
            | 'ゞ'
            | 'ヽ'
            | 'ヾ'
            | '々'
            | '〻'
            | '・'
            | '！'
            | '？'
            | '：'
            | '；'
            | '‼'
            | '⁇'
            | '⁈'
            | '⁉'
            | '…'
            | '‥'
            | '〜'
            | '％'
            | '°'
    ) || matches!(u32::from(c), 0x31F0..=0x31FF)
}

/// `true` for characters a line never ends with (opening punctuation).
pub fn no_break_after(c: char) -> bool {
    matches!(
        c,
        '（' | '「'
            | '『'
            | '【'
            | '〔'
            | '〈'
            | '《'
            | '〘'
            | '〖'
            | '〝'
            | '‘'
            | '“'
            | '｟'
            | '«'
            | '('
            | '['
            | '{'
            | '￥'
            | '＄'
    )
}

/// How a line may break between two characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Break {
    /// It must (after a line feed).
    Mandatory,
    /// It may.
    Allowed,
    /// It may not.
    Never,
}

/// How a line may break between `before` and `after`.
pub fn between(before: char, after: char) -> Break {
    if before == '\n' {
        return Break::Mandatory;
    }
    if no_break_after(before) || no_break_before(after) || after.is_whitespace() {
        return Break::Never;
    }
    let hyphen = matches!(before, '-' | '\u{2010}' | '\u{2013}' | '\u{2014}');
    if before.is_whitespace()
        || (hyphen && after.is_alphanumeric())
        || is_cjk(before)
        || is_cjk(after)
    {
        Break::Allowed
    } else {
        Break::Never
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn breaks(text: &str) -> Vec<usize> {
        let chars: Vec<char> = text.chars().collect();
        (1..chars.len())
            .filter(|&i| between(chars[i - 1], chars[i]) != Break::Never)
            .collect()
    }

    #[test]
    fn latin_words_break_after_spaces_and_hyphens() {
        assert_eq!(breaks("a cat sat"), [2, 6]);
        assert_eq!(breaks("well-known"), [5]);
        assert_eq!(breaks("two  spaces"), [5], "after the last space only");
        assert_eq!(breaks("unbreakable"), [] as [usize; 0]);
        assert_eq!(between('\n', 'a'), Break::Mandatory);
    }

    #[test]
    fn cjk_breaks_between_characters_but_not_against_kinsoku() {
        // 日本語です。: anywhere between ideographs and kana, not before the full stop.
        assert_eq!(breaks("日本語です。"), [1, 2, 3, 4]);
        // Not before small kana or the prolonged sound mark, not after an opening bracket.
        assert_eq!(breaks("「ちょっと」"), [4]);
        assert_eq!(breaks("コーヒー"), [2]);
        // Latin within Japanese: a break on each side.
        assert_eq!(breaks("Koebiは"), [5]);
        // Hangul syllables.
        assert_eq!(breaks("한국어"), [1, 2]);
    }
}
