//! Font sizes and scales, weights, line heights, letter spacing, styles and families.

use super::*;
use koebi_design::define::*;

koebi_design::tokens! {
    group TYPOGRAPHY: Primitive, start = LAYOUT::END;

    /// Font size scale, step 10.
    FONT_SIZE_10 = "font.size.10": Dimension = px(10.0), range(6.0, 200.0);
    /// Font size scale, step 20.
    FONT_SIZE_20 = "font.size.20": Dimension = px(11.0), range(6.0, 200.0);
    /// Font size scale, step 30.
    FONT_SIZE_30 = "font.size.30": Dimension = px(12.0), range(6.0, 200.0);
    /// Font size scale, step 40.
    FONT_SIZE_40 = "font.size.40": Dimension = px(13.0), range(6.0, 200.0);
    /// Font size scale, step 50.
    FONT_SIZE_50 = "font.size.50": Dimension = px(14.0), range(6.0, 200.0);
    /// Font size scale, step 60.
    FONT_SIZE_60 = "font.size.60": Dimension = px(15.0), range(6.0, 200.0);
    /// Font size scale, step 70.
    FONT_SIZE_70 = "font.size.70": Dimension = px(16.0), range(6.0, 200.0);
    /// Font size scale, step 80.
    FONT_SIZE_80 = "font.size.80": Dimension = px(18.0), range(6.0, 200.0);
    /// Font size scale, step 90.
    FONT_SIZE_90 = "font.size.90": Dimension = px(20.0), range(6.0, 200.0);
    /// Font size scale, step 100.
    FONT_SIZE_100 = "font.size.100": Dimension = px(24.0), range(6.0, 200.0);
    /// Font size scale, step 110.
    FONT_SIZE_110 = "font.size.110": Dimension = px(28.0), range(6.0, 200.0);
    /// Font size scale, step 120.
    FONT_SIZE_120 = "font.size.120": Dimension = px(32.0), range(6.0, 200.0);
    /// Font size scale, step 130.
    FONT_SIZE_130 = "font.size.130": Dimension = px(36.0), range(6.0, 200.0);
    /// Font size scale, step 140.
    FONT_SIZE_140 = "font.size.140": Dimension = px(40.0), range(6.0, 200.0);
    /// Font size scale, step 150.
    FONT_SIZE_150 = "font.size.150": Dimension = px(48.0), range(6.0, 200.0);
    /// Font size scale, step 160.
    FONT_SIZE_160 = "font.size.160": Dimension = px(56.0), range(6.0, 200.0);
    /// Font size scale, step 170.
    FONT_SIZE_170 = "font.size.170": Dimension = px(64.0), range(6.0, 200.0);
    /// Font size scale, step 180.
    FONT_SIZE_180 = "font.size.180": Dimension = px(80.0), range(6.0, 200.0);
    /// Font size scale, step 190.
    FONT_SIZE_190 = "font.size.190": Dimension = px(96.0), range(6.0, 200.0);

    /// Font scale factor of 75 %.
    FONT_SCALE_75 = "font-scale.75": Number = 0.75, range(0.5, 4.0);
    /// Font scale factor of 100 %.
    FONT_SCALE_100 = "font-scale.100": Number = 1.0, range(0.5, 4.0);
    /// Font scale factor of 125 %.
    FONT_SCALE_125 = "font-scale.125": Number = 1.25, range(0.5, 4.0);
    /// Font scale factor of 150 %.
    FONT_SCALE_150 = "font-scale.150": Number = 1.5, range(0.5, 4.0);

    /// Base font size of the modular type scale.
    FONT_SCALE_BASE = "font.scale.base": Dimension = px(16.0), range(8.0, 32.0);
    /// Ratio between two consecutive sizes of the modular type scale.
    FONT_SCALE_RATIO = "font.scale.ratio": Number = 1.25, range(1.0, 2.0);

    /// Thin font weight (100).
    FONT_WEIGHT_THIN = "font.weight.thin": FontWeight = weight(100), range(1.0, 1000.0);
    /// Extra light font weight (200).
    FONT_WEIGHT_EXTRA_LIGHT = "font.weight.extra-light": FontWeight =
        weight(200), range(1.0, 1000.0);
    /// Light font weight (300).
    FONT_WEIGHT_LIGHT = "font.weight.light": FontWeight = weight(300), range(1.0, 1000.0);
    /// Regular font weight (400).
    FONT_WEIGHT_REGULAR = "font.weight.regular": FontWeight = weight(400), range(1.0, 1000.0);
    /// Medium font weight (500).
    FONT_WEIGHT_MEDIUM = "font.weight.medium": FontWeight = weight(500), range(1.0, 1000.0);
    /// Semi bold font weight (600).
    FONT_WEIGHT_SEMI_BOLD = "font.weight.semi-bold": FontWeight = weight(600), range(1.0, 1000.0);
    /// Bold font weight (700).
    FONT_WEIGHT_BOLD = "font.weight.bold": FontWeight = weight(700), range(1.0, 1000.0);
    /// Extra bold font weight (800).
    FONT_WEIGHT_EXTRA_BOLD = "font.weight.extra-bold": FontWeight = weight(800), range(1.0, 1000.0);
    /// Black font weight (900).
    FONT_WEIGHT_BLACK = "font.weight.black": FontWeight = weight(900), range(1.0, 1000.0);
    /// Extra black font weight (950).
    FONT_WEIGHT_EXTRA_BLACK = "font.weight.extra-black": FontWeight =
        weight(950), range(1.0, 1000.0);

    /// Single-spaced line height, as a multiple of the font size.
    FONT_LINE_HEIGHT_NONE = "font.line-height.none": Number = 1.0, range(0.8, 3.0);
    /// Tight line height, as a multiple of the font size.
    FONT_LINE_HEIGHT_TIGHT = "font.line-height.tight": Number = 1.15, range(0.8, 3.0);
    /// Snug line height, as a multiple of the font size.
    FONT_LINE_HEIGHT_SNUG = "font.line-height.snug": Number = 1.3, range(0.8, 3.0);
    /// Normal line height, as a multiple of the font size.
    FONT_LINE_HEIGHT_NORMAL = "font.line-height.normal": Number = 1.5, range(0.8, 3.0);
    /// Relaxed line height, as a multiple of the font size.
    FONT_LINE_HEIGHT_RELAXED = "font.line-height.relaxed": Number = 1.7, range(0.8, 3.0);
    /// Loose line height, as a multiple of the font size.
    FONT_LINE_HEIGHT_LOOSE = "font.line-height.loose": Number = 2.0, range(0.8, 3.0);

    /// Compact line height for CJK text, as a multiple of the font size.
    FONT_LINE_HEIGHT_CJK_COMPACT = "font.line-height.cjk.compact": Number = 1.5, range(0.8, 3.0);
    /// Normal line height for CJK text, as a multiple of the font size.
    FONT_LINE_HEIGHT_CJK_NORMAL = "font.line-height.cjk.normal": Number = 1.7, range(0.8, 3.0);
    /// Relaxed line height for CJK text, as a multiple of the font size.
    FONT_LINE_HEIGHT_CJK_RELAXED = "font.line-height.cjk.relaxed": Number = 1.9, range(0.8, 3.0);

    /// Tighter letter spacing, in em.
    FONT_TRACKING_TIGHTER = "font.tracking.tighter": Number = -0.05, range(-0.2, 0.5);
    /// Tight letter spacing, in em.
    FONT_TRACKING_TIGHT = "font.tracking.tight": Number = -0.02, range(-0.2, 0.5);
    /// Normal letter spacing, in em.
    FONT_TRACKING_NORMAL = "font.tracking.normal": Number = 0.0, range(-0.2, 0.5);
    /// Wide letter spacing, in em.
    FONT_TRACKING_WIDE = "font.tracking.wide": Number = 0.05, range(-0.2, 0.5);
    /// Wider letter spacing, in em.
    FONT_TRACKING_WIDER = "font.tracking.wider": Number = 0.1, range(-0.2, 0.5);
    /// Widest letter spacing, in em.
    FONT_TRACKING_WIDEST = "font.tracking.widest": Number = 0.2, range(-0.2, 0.5);

    /// Upright font style (0).
    FONT_STYLE_NORMAL = "font.style.normal": Integer = 0, range(0.0, 1.0);
    /// Italic font style (1).
    FONT_STYLE_ITALIC = "font.style.italic": Integer = 1, range(0.0, 1.0);

    /// Sans-serif font family.
    FONT_FAMILY_SANS = "font.family.sans": FontFamily = family("NotoSans");
    /// Serif font family.
    FONT_FAMILY_SERIF = "font.family.serif": FontFamily = family("NotoSans");
    /// Monospaced font family.
    FONT_FAMILY_MONO = "font.family.mono": FontFamily = family("NotoSans");
    /// Font family for Chinese, Japanese and Korean text.
    FONT_FAMILY_CJK = "font.family.cjk": FontFamily = family("NotoSans");
    /// Serif font family for Chinese, Japanese and Korean text.
    FONT_FAMILY_CJK_SERIF = "font.family.cjk-serif": FontFamily = family("NotoSans");
}
