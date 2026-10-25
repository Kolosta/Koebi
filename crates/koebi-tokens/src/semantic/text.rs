//! Text styles: typography composites bundling a font family, size, weight, line height and letter
//! spacing. Each field is a token of its own, so an override can change one property of one style.

use super::*;
use crate::primitive;

koebi_design::typography! {
    group TEXT: Semantic, start = INPUT::END;

    /// Text style of display text, size l.
    TEXT_DISPLAY_L = "text.display.l" {
        font_family: primitive::FONT_FAMILY_SERIF,
        font_size: FONT_SIZE_DISPLAY_L,
        font_weight: primitive::FONT_WEIGHT_BOLD,
        line_height: primitive::FONT_LINE_HEIGHT_TIGHT,
        letter_spacing: primitive::FONT_TRACKING_NORMAL,
    }

    /// Text style of display text, size m.
    TEXT_DISPLAY_M = "text.display.m" {
        font_family: primitive::FONT_FAMILY_SERIF,
        font_size: FONT_SIZE_DISPLAY_M,
        font_weight: primitive::FONT_WEIGHT_BOLD,
        line_height: primitive::FONT_LINE_HEIGHT_TIGHT,
        letter_spacing: primitive::FONT_TRACKING_NORMAL,
    }

    /// Text style of display text, size s.
    TEXT_DISPLAY_S = "text.display.s" {
        font_family: primitive::FONT_FAMILY_SERIF,
        font_size: FONT_SIZE_DISPLAY_S,
        font_weight: primitive::FONT_WEIGHT_BOLD,
        line_height: primitive::FONT_LINE_HEIGHT_TIGHT,
        letter_spacing: primitive::FONT_TRACKING_NORMAL,
    }

    /// Text style of headings, size xl.
    TEXT_HEADING_XL = "text.heading.xl" {
        font_family: primitive::FONT_FAMILY_SANS,
        font_size: FONT_SIZE_HEADING_XL,
        font_weight: primitive::FONT_WEIGHT_SEMI_BOLD,
        line_height: primitive::FONT_LINE_HEIGHT_TIGHT,
        letter_spacing: primitive::FONT_TRACKING_TIGHT,
    }

    /// Text style of headings, size l.
    TEXT_HEADING_L = "text.heading.l" {
        font_family: primitive::FONT_FAMILY_SANS,
        font_size: FONT_SIZE_HEADING_L,
        font_weight: primitive::FONT_WEIGHT_SEMI_BOLD,
        line_height: primitive::FONT_LINE_HEIGHT_SNUG,
        letter_spacing: primitive::FONT_TRACKING_NORMAL,
    }

    /// Text style of headings, size m.
    TEXT_HEADING_M = "text.heading.m" {
        font_family: primitive::FONT_FAMILY_SANS,
        font_size: FONT_SIZE_HEADING_M,
        font_weight: primitive::FONT_WEIGHT_SEMI_BOLD,
        line_height: primitive::FONT_LINE_HEIGHT_SNUG,
        letter_spacing: primitive::FONT_TRACKING_NORMAL,
    }

    /// Text style of headings, size s.
    TEXT_HEADING_S = "text.heading.s" {
        font_family: primitive::FONT_FAMILY_SANS,
        font_size: FONT_SIZE_HEADING_S,
        font_weight: primitive::FONT_WEIGHT_SEMI_BOLD,
        line_height: primitive::FONT_LINE_HEIGHT_SNUG,
        letter_spacing: primitive::FONT_TRACKING_NORMAL,
    }

    /// Text style of headings, size xs.
    TEXT_HEADING_XS = "text.heading.xs" {
        font_family: primitive::FONT_FAMILY_SANS,
        font_size: FONT_SIZE_HEADING_XS,
        font_weight: primitive::FONT_WEIGHT_SEMI_BOLD,
        line_height: primitive::FONT_LINE_HEIGHT_SNUG,
        letter_spacing: primitive::FONT_TRACKING_NORMAL,
    }

    /// Text style of body text, size l.
    TEXT_BODY_L = "text.body.l" {
        font_family: primitive::FONT_FAMILY_SANS,
        font_size: FONT_SIZE_BODY_L,
        font_weight: primitive::FONT_WEIGHT_REGULAR,
        line_height: primitive::FONT_LINE_HEIGHT_NORMAL,
        letter_spacing: primitive::FONT_TRACKING_NORMAL,
    }

    /// Text style of body text, size m.
    TEXT_BODY_M = "text.body.m" {
        font_family: primitive::FONT_FAMILY_SANS,
        font_size: FONT_SIZE_BODY_M,
        font_weight: primitive::FONT_WEIGHT_REGULAR,
        line_height: primitive::FONT_LINE_HEIGHT_NORMAL,
        letter_spacing: primitive::FONT_TRACKING_NORMAL,
    }

    /// Text style of body text, size s.
    TEXT_BODY_S = "text.body.s" {
        font_family: primitive::FONT_FAMILY_SANS,
        font_size: FONT_SIZE_BODY_S,
        font_weight: primitive::FONT_WEIGHT_REGULAR,
        line_height: primitive::FONT_LINE_HEIGHT_NORMAL,
        letter_spacing: primitive::FONT_TRACKING_NORMAL,
    }

    /// Text style of detail text, size l.
    TEXT_DETAIL_L = "text.detail.l" {
        font_family: primitive::FONT_FAMILY_SANS,
        font_size: FONT_SIZE_DETAIL_L,
        font_weight: primitive::FONT_WEIGHT_REGULAR,
        line_height: primitive::FONT_LINE_HEIGHT_NORMAL,
        letter_spacing: primitive::FONT_TRACKING_NORMAL,
    }

    /// Text style of detail text, size m.
    TEXT_DETAIL_M = "text.detail.m" {
        font_family: primitive::FONT_FAMILY_SANS,
        font_size: FONT_SIZE_DETAIL_M,
        font_weight: primitive::FONT_WEIGHT_REGULAR,
        line_height: primitive::FONT_LINE_HEIGHT_NORMAL,
        letter_spacing: primitive::FONT_TRACKING_NORMAL,
    }

    /// Text style of detail text, size s.
    TEXT_DETAIL_S = "text.detail.s" {
        font_family: primitive::FONT_FAMILY_SANS,
        font_size: FONT_SIZE_DETAIL_S,
        font_weight: primitive::FONT_WEIGHT_REGULAR,
        line_height: primitive::FONT_LINE_HEIGHT_NORMAL,
        letter_spacing: primitive::FONT_TRACKING_NORMAL,
    }

    /// Text style of labels, size l.
    TEXT_LABEL_L = "text.label.l" {
        font_family: primitive::FONT_FAMILY_SANS,
        font_size: FONT_SIZE_LABEL_L,
        font_weight: primitive::FONT_WEIGHT_MEDIUM,
        line_height: primitive::FONT_LINE_HEIGHT_SNUG,
        letter_spacing: primitive::FONT_TRACKING_WIDE,
    }

    /// Text style of labels, size m.
    TEXT_LABEL_M = "text.label.m" {
        font_family: primitive::FONT_FAMILY_SANS,
        font_size: FONT_SIZE_LABEL_M,
        font_weight: primitive::FONT_WEIGHT_MEDIUM,
        line_height: primitive::FONT_LINE_HEIGHT_SNUG,
        letter_spacing: primitive::FONT_TRACKING_WIDE,
    }

    /// Text style of labels, size s.
    TEXT_LABEL_S = "text.label.s" {
        font_family: primitive::FONT_FAMILY_SANS,
        font_size: FONT_SIZE_LABEL_S,
        font_weight: primitive::FONT_WEIGHT_MEDIUM,
        line_height: primitive::FONT_LINE_HEIGHT_SNUG,
        letter_spacing: primitive::FONT_TRACKING_WIDE,
    }

    /// Text style of monospaced text, size l.
    TEXT_MONO_L = "text.mono.l" {
        font_family: primitive::FONT_FAMILY_MONO,
        font_size: FONT_SIZE_MONO_L,
        font_weight: primitive::FONT_WEIGHT_REGULAR,
        line_height: primitive::FONT_LINE_HEIGHT_NORMAL,
        letter_spacing: primitive::FONT_TRACKING_NORMAL,
    }

    /// Text style of monospaced text, size m.
    TEXT_MONO_M = "text.mono.m" {
        font_family: primitive::FONT_FAMILY_MONO,
        font_size: FONT_SIZE_MONO_M,
        font_weight: primitive::FONT_WEIGHT_REGULAR,
        line_height: primitive::FONT_LINE_HEIGHT_NORMAL,
        letter_spacing: primitive::FONT_TRACKING_NORMAL,
    }

    /// Text style of monospaced text, size s.
    TEXT_MONO_S = "text.mono.s" {
        font_family: primitive::FONT_FAMILY_MONO,
        font_size: FONT_SIZE_MONO_S,
        font_weight: primitive::FONT_WEIGHT_REGULAR,
        line_height: primitive::FONT_LINE_HEIGHT_NORMAL,
        letter_spacing: primitive::FONT_TRACKING_NORMAL,
    }

    /// Text style of code, size m.
    TEXT_CODE_M = "text.code.m" {
        font_family: primitive::FONT_FAMILY_MONO,
        font_size: FONT_SIZE_CODE_M,
        font_weight: primitive::FONT_WEIGHT_REGULAR,
        line_height: primitive::FONT_LINE_HEIGHT_NORMAL,
        letter_spacing: primitive::FONT_TRACKING_NORMAL,
    }
}
