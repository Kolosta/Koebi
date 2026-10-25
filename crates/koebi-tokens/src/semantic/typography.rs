//! Typographic properties per text style, and spacing around text blocks.

use super::*;
use crate::primitive;

koebi_design::tokens! {
    group TYPOGRAPHY: Semantic, start = SCALE::END;

    /// Font family of headings.
    FONT_FAMILY_HEADING = "font.family.heading": FontFamily -> primitive::FONT_FAMILY_SERIF;
    /// Font family of body text.
    FONT_FAMILY_BODY = "font.family.body": FontFamily -> primitive::FONT_FAMILY_SANS;
    /// Font family of monospaced text.
    FONT_FAMILY_MONO = "font.family.mono": FontFamily -> primitive::FONT_FAMILY_MONO;
    /// Font family of Chinese, Japanese and Korean text.
    FONT_FAMILY_CJK = "font.family.cjk": FontFamily -> primitive::FONT_FAMILY_CJK;

    /// Default font size of the interface.
    FONT_SIZE_DEFAULT = "font.size.default": Dimension -> primitive::FONT_SIZE_50;

    /// Font size of display text, size s.
    FONT_SIZE_DISPLAY_S = "font.size.display.s": Dimension -> primitive::FONT_SIZE_170;
    /// Font size of display text, size m.
    FONT_SIZE_DISPLAY_M = "font.size.display.m": Dimension -> primitive::FONT_SIZE_180;
    /// Font size of display text, size l.
    FONT_SIZE_DISPLAY_L = "font.size.display.l": Dimension -> primitive::FONT_SIZE_190;

    /// Font size of headings, size xs.
    FONT_SIZE_HEADING_XS = "font.size.heading.xs": Dimension -> primitive::FONT_SIZE_80;
    /// Font size of headings, size s.
    FONT_SIZE_HEADING_S = "font.size.heading.s": Dimension -> primitive::FONT_SIZE_90;
    /// Font size of headings, size m.
    FONT_SIZE_HEADING_M = "font.size.heading.m": Dimension -> primitive::FONT_SIZE_100;
    /// Font size of headings, size l.
    FONT_SIZE_HEADING_L = "font.size.heading.l": Dimension -> primitive::FONT_SIZE_120;
    /// Font size of headings, size xl.
    FONT_SIZE_HEADING_XL = "font.size.heading.xl": Dimension -> primitive::FONT_SIZE_140;

    /// Font size of body text, size s.
    FONT_SIZE_BODY_S = "font.size.body.s": Dimension -> primitive::FONT_SIZE_50;
    /// Font size of body text, size m.
    FONT_SIZE_BODY_M = "font.size.body.m": Dimension -> primitive::FONT_SIZE_60;
    /// Font size of body text, size l.
    FONT_SIZE_BODY_L = "font.size.body.l": Dimension -> primitive::FONT_SIZE_70;

    /// Font size of detail text, size s.
    FONT_SIZE_DETAIL_S = "font.size.detail.s": Dimension -> primitive::FONT_SIZE_30;
    /// Font size of detail text, size m.
    FONT_SIZE_DETAIL_M = "font.size.detail.m": Dimension -> primitive::FONT_SIZE_40;
    /// Font size of detail text, size l.
    FONT_SIZE_DETAIL_L = "font.size.detail.l": Dimension -> primitive::FONT_SIZE_50;

    /// Font size of labels, size s.
    FONT_SIZE_LABEL_S = "font.size.label.s": Dimension -> primitive::FONT_SIZE_40;
    /// Font size of labels, size m.
    FONT_SIZE_LABEL_M = "font.size.label.m": Dimension -> primitive::FONT_SIZE_50;
    /// Font size of labels, size l.
    FONT_SIZE_LABEL_L = "font.size.label.l": Dimension -> primitive::FONT_SIZE_60;

    /// Font size of monospaced text, size s.
    FONT_SIZE_MONO_S = "font.size.mono.s": Dimension -> primitive::FONT_SIZE_50;
    /// Font size of monospaced text, size m.
    FONT_SIZE_MONO_M = "font.size.mono.m": Dimension -> primitive::FONT_SIZE_60;
    /// Font size of monospaced text, size l.
    FONT_SIZE_MONO_L = "font.size.mono.l": Dimension -> primitive::FONT_SIZE_70;

    /// Font size of code, size m.
    FONT_SIZE_CODE_M = "font.size.code.m": Dimension -> primitive::FONT_SIZE_60;

    /// Default font scale factor.
    FONT_SCALE_DEFAULT = "font-scale.default": Number -> primitive::FONT_SCALE_100;

    /// Font weight of display text, size l.
    FONT_WEIGHT_DISPLAY_L = "font.weight.display.l": FontWeight -> primitive::FONT_WEIGHT_BOLD;

    /// Font weight of headings, size s.
    FONT_WEIGHT_HEADING_S = "font.weight.heading.s": FontWeight -> primitive::FONT_WEIGHT_SEMI_BOLD;
    /// Font weight of headings, size m.
    FONT_WEIGHT_HEADING_M = "font.weight.heading.m": FontWeight -> primitive::FONT_WEIGHT_SEMI_BOLD;
    /// Font weight of headings, size l.
    FONT_WEIGHT_HEADING_L = "font.weight.heading.l": FontWeight -> primitive::FONT_WEIGHT_SEMI_BOLD;
    /// Font weight of headings, size xl.
    FONT_WEIGHT_HEADING_XL = "font.weight.heading.xl": FontWeight ->
        primitive::FONT_WEIGHT_SEMI_BOLD;

    /// Font weight of body text, size m.
    FONT_WEIGHT_BODY_M = "font.weight.body.m": FontWeight -> primitive::FONT_WEIGHT_REGULAR;

    /// Font weight of labels, size m.
    FONT_WEIGHT_LABEL_M = "font.weight.label.m": FontWeight -> primitive::FONT_WEIGHT_MEDIUM;

    /// Font weight of monospaced text, size m.
    FONT_WEIGHT_MONO_M = "font.weight.mono.m": FontWeight -> primitive::FONT_WEIGHT_REGULAR;

    /// Line height of display text, size l.
    FONT_LINE_HEIGHT_DISPLAY_L = "font.line-height.display.l": Number ->
        primitive::FONT_LINE_HEIGHT_TIGHT;

    /// Line height of headings, size m.
    FONT_LINE_HEIGHT_HEADING_M = "font.line-height.heading.m": Number ->
        primitive::FONT_LINE_HEIGHT_SNUG;
    /// Line height of headings, size xl.
    FONT_LINE_HEIGHT_HEADING_XL = "font.line-height.heading.xl": Number ->
        primitive::FONT_LINE_HEIGHT_TIGHT;

    /// Line height of body text, size m.
    FONT_LINE_HEIGHT_BODY_M = "font.line-height.body.m": Number ->
        primitive::FONT_LINE_HEIGHT_NORMAL;

    /// Line height of detail text, size s.
    FONT_LINE_HEIGHT_DETAIL_S = "font.line-height.detail.s": Number ->
        primitive::FONT_LINE_HEIGHT_NORMAL;

    /// Line height of labels, size m.
    FONT_LINE_HEIGHT_LABEL_M = "font.line-height.label.m": Number ->
        primitive::FONT_LINE_HEIGHT_SNUG;

    /// Line height of CJK headings, size m.
    FONT_LINE_HEIGHT_CJK_HEADING_M = "font.line-height.cjk.heading.m": Number ->
        primitive::FONT_LINE_HEIGHT_CJK_COMPACT;

    /// Line height of CJK body text, size m.
    FONT_LINE_HEIGHT_CJK_BODY_M = "font.line-height.cjk.body.m": Number ->
        primitive::FONT_LINE_HEIGHT_CJK_NORMAL;

    /// Letter spacing of headings, size xl.
    FONT_TRACKING_HEADING_XL = "font.tracking.heading.xl": Number -> primitive::FONT_TRACKING_TIGHT;

    /// Letter spacing of body text, size m.
    FONT_TRACKING_BODY_M = "font.tracking.body.m": Number -> primitive::FONT_TRACKING_NORMAL;

    /// Letter spacing of labels, size m.
    FONT_TRACKING_LABEL_M = "font.tracking.label.m": Number -> primitive::FONT_TRACKING_WIDE;

    /// Space above headings, size m.
    TEXT_MARGIN_TOP_HEADING_M = "text.margin.top.heading.m": Dimension -> primitive::SPACING_400;
    /// Space above headings, size l.
    TEXT_MARGIN_TOP_HEADING_L = "text.margin.top.heading.l": Dimension -> primitive::SPACING_500;
    /// Space above headings, size xl.
    TEXT_MARGIN_TOP_HEADING_XL = "text.margin.top.heading.xl": Dimension -> primitive::SPACING_600;

    /// Space above body text, size m.
    TEXT_MARGIN_TOP_BODY_M = "text.margin.top.body.m": Dimension -> primitive::SPACING_300;

    /// Space below headings, size m.
    TEXT_MARGIN_BOTTOM_HEADING_M = "text.margin.bottom.heading.m": Dimension ->
        primitive::SPACING_200;
    /// Space below headings, size xl.
    TEXT_MARGIN_BOTTOM_HEADING_XL = "text.margin.bottom.heading.xl": Dimension ->
        primitive::SPACING_300;

    /// Space below body text, size m.
    TEXT_MARGIN_BOTTOM_BODY_M = "text.margin.bottom.body.m": Dimension -> primitive::SPACING_200;
}
