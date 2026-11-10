//! The palettes of the built-in themes, as data: the key colors each palette passes through,
//! the contrast each step keeps with its theme's background, and what each theme changes.
//!
//! Palettes are ordered by contrast, as in Adobe Spectrum: a step is a contrast with the
//! background, not a color ([`koebi_core::palette`]). Semantic tokens name steps (primary text
//! is `gray.900`, an accent fill `blue.900`), and every theme gives each step the color that
//! keeps that contrast against its own background. The token tables and the theme modules
//! hold the resulting values as literals, written by `tests/palettes.rs` from this data
//! (`KOEBI_BLESS=1`) and checked by it: a value may be tuned by hand as long as its step keeps
//! its contrast within the test's tolerance.
//!
//! The color settings use the same data to make a palette again from a chosen color.

use koebi_core::palette::{ColorScale, HUE_CONTRASTS, HUE_STEPS};
use koebi_core::{Color, Oklch};
use koebi_design::kind;
use koebi_design::tier::Semantic;
use koebi_design::value::{Definition, Value};
use koebi_design::{DesignSystem, Layers, Scope, ThemeId, Token, TokenId};

use crate::semantic;

/// A hue's palette: its name (`color.<name>.<step>`) and the key colors its scale passes
/// through, Spectrum's light palette of that hue, from step 100 to 1600.
#[derive(Clone, Copy, Debug)]
pub struct HuePalette {
    /// The palette's name in token paths.
    pub name: &'static str,
    /// Key colors (`0xRRGGBB`), lightest first.
    pub keys: [u32; 16],
}

/// The 18 hue palettes, in the order of the token tables.
#[allow(
    clippy::mistyped_literal_suffixes,
    reason = "colors written 0xRR_GG_BB: a last byte of 16 or 32 is not an integer suffix"
)]
pub const HUES: [HuePalette; 18] = [
    HuePalette {
        name: "red",
        keys: [
            0xFF_F6_F5, 0xFF_EB_E8, 0xFF_D6_D1, 0xFF_BC_B4, 0xFF_9D_91, 0xFF_76_65, 0xFF_51_3D,
            0xF0_38_23, 0xD7_32_20, 0xB7_28_18, 0x9C_21_13, 0x81_1B_0E, 0x68_15_0A, 0x50_10_06,
            0x3B_0B_04, 0x1D_05_02,
        ],
    },
    HuePalette {
        name: "orange",
        keys: [
            0xFF_F6_E7, 0xFF_EC_CF, 0xFF_DA_9E, 0xFF_C1_5E, 0xFF_A2_13, 0xFC_7D_00, 0xE8_6A_00,
            0xD4_5B_00, 0xC2_4E_00, 0xA7_3E_00, 0x90_33_00, 0x76_29_00, 0x5F_20_00, 0x49_18_00,
            0x34_12_00, 0x19_08_00,
        ],
    },
    HuePalette {
        name: "yellow",
        keys: [
            0xFF_F8_CC, 0xFF_F1_97, 0xFF_DE_2C, 0xF5_C7_00, 0xE6_AF_00, 0xD2_95_00, 0xC1_83_00,
            0xAF_74_00, 0x9E_66_00, 0x86_55_00, 0x72_48_00, 0x5D_3B_00, 0x4B_2F_00, 0x38_23_00,
            0x28_19_00, 0x12_0B_00,
        ],
    },
    HuePalette {
        name: "chartreuse",
        keys: [
            0xF6_FB_DE, 0xEA_F6_AD, 0xD0_EC_46, 0xB6_DB_00, 0xA3_C4_00, 0x8F_AC_00, 0x80_99_00,
            0x72_89_00, 0x66_7A_00, 0x56_67_00, 0x49_57_00, 0x3C_47_00, 0x2F_39_00, 0x23_2B_00,
            0x19_1E_00, 0x0B_0E_00,
        ],
    },
    HuePalette {
        name: "celery",
        keys: [
            0xEB_FF_DC, 0xC5_FF_9C, 0x9D_F7_5C, 0x81_E4_3A, 0x6E_CE_2A, 0x5D_B4_1F, 0x52_A1_19,
            0x48_90_14, 0x40_81_11, 0x34_6D_0C, 0x2C_5C_09, 0x23_4B_06, 0x1B_3C_03, 0x13_2E_00,
            0x0C_21_00, 0x04_0F_00,
        ],
    },
    HuePalette {
        name: "green",
        keys: [
            0xED_FC_F1, 0xD7_F7_E1, 0xAD_EE_C5, 0x6B_E3_A2, 0x2B_D1_7D, 0x12_B8_67, 0x0B_A4_5D,
            0x07_93_55, 0x05_83_4E, 0x03_6E_45, 0x02_5D_3C, 0x01_4C_34, 0x00_3D_2C, 0x00_2E_22,
            0x00_21_19, 0x00_0F_0C,
        ],
    },
    HuePalette {
        name: "seafoam",
        keys: [
            0xEB_FB_F6, 0xD3_F6_EA, 0xA9_ED_D8, 0x5C_E1_C2, 0x10_CF_A9, 0x0D_B5_95, 0x0B_A2_86,
            0x09_90_78, 0x07_81_6D, 0x05_6C_5C, 0x03_5C_50, 0x01_4B_43, 0x00_3C_36, 0x00_2E_28,
            0x00_21_1D, 0x00_0F_0E,
        ],
    },
    HuePalette {
        name: "cyan",
        keys: [
            0xEE_FA_FE, 0xD9_F4_FD, 0xB7_E7_FC, 0x8A_D5_FF, 0x5C_C0_FF, 0x30_A7_FE, 0x1D_95_E7,
            0x12_86_CD, 0x0B_78_B3, 0x04_66_91, 0x00_57_79, 0x00_47_62, 0x00_39_4E, 0x00_2B_3B,
            0x00_1F_2B, 0x00_0E_14,
        ],
    },
    HuePalette {
        name: "blue",
        keys: [
            0xF5_F9_FF, 0xE5_F0_FE, 0xCB_E2_FE, 0xAC_CF_FD, 0x8E_B9_FC, 0x72_9E_FD, 0x5D_89_FF,
            0x4B_75_FF, 0x3B_63_FB, 0x27_4D_EA, 0x1D_3E_CF, 0x15_32_AD, 0x10_28_8C, 0x0C_1F_69,
            0x0E_18_43, 0x07_0B_1E,
        ],
    },
    HuePalette {
        name: "indigo",
        keys: [
            0xF7_F8_FF, 0xEB_EE_FF, 0xD8_DE_FF, 0xC0_C9_FF, 0xA7_B2_FF, 0x91_97_FE, 0x84_80_FE,
            0x7A_6A_FD, 0x71_55_FA, 0x63_38_EE, 0x54_24_DB, 0x45_13_BF, 0x37_06_A0, 0x2A_00_81,
            0x1F_00_62, 0x11_00_36,
        ],
    },
    HuePalette {
        name: "purple",
        keys: [
            0xFB_F7_FE, 0xF4_EB_FC, 0xEB_DA_F9, 0xDD_C1_F6, 0xD0_A7_F3, 0xBF_8A_EE, 0xB2_72_EB,
            0xA6_5C_E7, 0x9A_47_E2, 0x86_28_D9, 0x73_0D_CC, 0x5D_00_B1, 0x4B_00_90, 0x3B_00_6F,
            0x2C_00_54, 0x17_00_2D,
        ],
    },
    HuePalette {
        name: "fuchsia",
        keys: [
            0xFE_F6_FF, 0xFD_E9_FF, 0xFA_D3_FF, 0xF7_B5_FF, 0xF3_93_FF, 0xEC_69_FF, 0xDF_4D_F5,
            0xC8_44_DC, 0xB5_39_C8, 0x9C_28_AF, 0x87_1B_9A, 0x71_0F_83, 0x5C_04_6D, 0x48_00_58,
            0x36_00_42, 0x1D_00_23,
        ],
    },
    HuePalette {
        name: "magenta",
        keys: [
            0xFF_F5_F8, 0xFF_E8_F0, 0xFF_D5_E3, 0xFF_B9_D0, 0xFF_98_BB, 0xFF_70_9F, 0xFF_48_85,
            0xF0_2D_6E, 0xD9_23_61, 0xBA_16_50, 0xA3_05_3E, 0x88_00_33, 0x6F_00_28, 0x56_00_1E,
            0x40_00_16, 0x23_00_0C,
        ],
    },
    HuePalette {
        name: "pink",
        keys: [
            0xFF_F6_FC, 0xFF_E8_F7, 0xFF_D3_F0, 0xFF_B5_E6, 0xFF_94_DB, 0xFF_67_CC, 0xF2_4C_B8,
            0xE4_34_A3, 0xCE_2A_92, 0xB0_1F_7B, 0x98_16_68, 0x80_0C_55, 0x69_03_44, 0x53_00_35,
            0x3E_00_27, 0x21_00_15,
        ],
    },
    HuePalette {
        name: "turquoise",
        keys: [
            0xEE_FB_FB, 0xD1_F5_F5, 0xA9_EC_ED, 0x6F_DD_E4, 0x27_CA_D8, 0x0F_B1_C0, 0x0C_9E_AB,
            0x0A_8D_99, 0x08_7E_89, 0x05_6B_74, 0x03_5A_62, 0x01_4A_51, 0x00_3B_41, 0x00_2C_31,
            0x00_20_23, 0x00_0F_11,
        ],
    },
    HuePalette {
        name: "cinnamon",
        keys: [
            0xFD_F7_F3, 0xF9_EC_E5, 0xF4_DA_CB, 0xED_C4_AC, 0xE5_AA_88, 0xD4_91_6C, 0xC6_7E_58,
            0xB8_6D_46, 0xAA_5E_38, 0x93_4D_2B, 0x80_3E_20, 0x6E_30_15, 0x5C_23_0B, 0x48_19_06,
            0x34_12_04, 0x18_08_02,
        ],
    },
    HuePalette {
        name: "brown",
        keys: [
            0xFC_F7_F2, 0xF7_EE_E1, 0xEF_DD_C3, 0xE5_C8_9D, 0xD6_B1_7B, 0xBE_9B_68, 0xAB_8A_5A,
            0x9A_7B_4D, 0x8B_6D_42, 0x77_5B_32, 0x67_4C_23, 0x58_3D_15, 0x46_31_11, 0x34_25_0D,
            0x26_1A_09, 0x10_0C_04,
        ],
    },
    HuePalette {
        name: "silver",
        keys: [
            0xF7_F7_F7, 0xEF_EF_EF, 0xDF_DF_DF, 0xCC_CC_CC, 0xB7_B7_B7, 0xA0_A0_A0, 0x8F_8F_8F,
            0x80_80_80, 0x72_72_72, 0x60_60_60, 0x51_51_51, 0x42_42_42, 0x34_34_34, 0x27_27_27,
            0x1C_1C_1C, 0x0C_0C_0C,
        ],
    },
];

/// The steps of the neutral palette (`color.gray.<step>`): 25 the deepest surface, 50 the
/// window's background, then away from it to 1000, the strongest text.
pub const NEUTRAL_STEPS: [u16; 16] = [
    25, 50, 75, 100, 150, 200, 250, 300, 350, 400, 500, 600, 700, 800, 900, 1000,
];

/// The contrast of each neutral step with step 50, the window's background. 25 and 50 are
/// given by the theme; 1000 is the end of the scale. Measured on the dark theme, whose grays
/// these are: 75 the controls' area, 100 the canvas, 150 the panels, 200 nested surfaces, 300
/// fields, 350 a hovered field, 400 borders (3:1), 500 the faintest marks, 600 to 900 text (900
/// the primary, above 15:1).
pub const NEUTRAL_CONTRASTS: [f32; 16] = [
    0.0, 1.0, 1.115, 1.18, 1.278, 1.45, 1.69, 2.03, 2.30, 3.95, 5.75, 8.09, 10.9, 13.3, 15.3, 21.0,
];

/// The contrast of each hue step with the background of a light theme: Spectrum's
/// ([`HUE_CONTRASTS`]).
pub const LIGHT_HUE_CONTRASTS: [f32; 16] = HUE_CONTRASTS;

/// The contrast of each hue step with the background of a dark theme: the ratio of the step
/// below in a light theme. A fill holding white text at 4.5:1 needs a luminance below 0.18:
/// step 900 has it on both sides (4.81:1 against white, 3.96:1 against Koebi's dark gray), so
/// that a semantic token names the same step for the same use in every theme.
pub const DARK_HUE_CONTRASTS: [f32; 16] = [
    1.03, 1.06, 1.15, 1.33, 1.61, 2.0, 2.61, 3.23, 3.96, 4.81, 6.31, 7.94, 10.04, 12.36, 14.85,
    17.04,
];

/// The contrast white content keeps on a fill of a dark theme, from hue step 900 and neutral
/// step 400 down: a background lighter than Koebi's dark gray would otherwise lift those steps
/// above the luminance that white text needs.
pub const WHITE_CONTENT: f32 = 4.55;

/// The last hue step that holds white content in a dark theme (step 900).
pub const LAST_HUE_FILL: usize = 8;

/// The last neutral step that holds white content in a dark theme (step 400).
pub const LAST_NEUTRAL_FILL: usize = 9;

/// In a dark theme, darkens the steps of `fills` that white content would not read on.
fn hold_white_content(scheme: Scheme, scale: &ColorScale, fills: &mut [Color]) {
    if scheme != Scheme::Dark {
        return;
    }
    for step in fills {
        if Color::WHITE.contrast_ratio(*step) < WHITE_CONTENT {
            *step = scale.with_contrast(Color::WHITE, WHITE_CONTENT);
        }
    }
}

/// Whether a theme's background is dark or light: which way its palettes go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    /// Light content on dark surfaces.
    Dark,
    /// Dark content on light surfaces.
    Light,
}

impl Scheme {
    /// The scheme of a theme whose background is `background`: dark below the luminance
    /// where black and white content contrast equally with it.
    pub fn of(background: Color) -> Self {
        if background.relative_luminance() < 0.18 {
            Self::Dark
        } else {
            Self::Light
        }
    }

    /// The contrast of each hue step with the background.
    pub const fn hue_contrasts(self) -> [f32; 16] {
        match self {
            Self::Dark => DARK_HUE_CONTRASTS,
            Self::Light => LIGHT_HUE_CONTRASTS,
        }
    }
}

/// The 16 steps of a hue palette made from one chosen color, for a theme whose background is
/// `background`: the seed's scale at each step's contrast, the steps holding white content
/// darkened where a dark background needs it. What the color settings write when a role takes
/// a color of the user's.
pub fn from_seed(seed: Color, background: Color) -> [Color; 16] {
    let scheme = Scheme::of(background);
    let scale = ColorScale::from_seed(seed.with_alpha(1.0));
    let mut steps = scale.steps(background, &scheme.hue_contrasts());
    hold_white_content(scheme, &scale, &mut steps[..=LAST_HUE_FILL]);
    steps
}

/// What a theme's palettes are made of.
#[derive(Clone, Copy, Debug)]
pub struct ThemePalettes {
    /// The theme's key (`dark`, `snow`).
    pub theme: &'static str,
    /// Dark or light.
    pub scheme: Scheme,
    /// `gray.25`, the deepest surface.
    pub deepest: u32,
    /// `gray.50`, the window's background: what every step's contrast is measured against.
    pub background: u32,
    /// Key colors of the neutral scale: its tint along lightness.
    pub neutral_keys: &'static [u32],
    /// The contrast of each neutral step with the background (see [`NEUTRAL_CONTRASTS`]).
    pub neutral_contrasts: [f32; 16],
    /// Chroma of the hues, as a share of their keys': below 1 for muted themes.
    pub chroma: f32,
    /// Hues made from one chosen color instead of Spectrum's keys: (name, `0xRRGGBB`).
    pub seeds: &'static [(&'static str, u32)],
}

impl ThemePalettes {
    /// The contrast of each hue step with the background.
    pub const fn hue_contrasts(&self) -> [f32; 16] {
        self.scheme.hue_contrasts()
    }

    /// The theme's background.
    pub fn background(&self) -> Color {
        Color::from_hex(self.background)
    }

    /// The neutral palette, step by step ([`NEUTRAL_STEPS`]).
    pub fn neutral(&self) -> [Color; 16] {
        let keys: Vec<Color> = self
            .neutral_keys
            .iter()
            .map(|&key| Color::from_hex(key))
            .collect();
        let scale = ColorScale::new(&keys).unwrap_or_else(|| ColorScale::from_seed(Color::BLACK));
        let background = self.background();
        let mut steps = self
            .neutral_contrasts
            .map(|ratio| scale.with_contrast(background, ratio));
        steps[0] = Color::from_hex(self.deepest);
        steps[1] = background;
        self.hold_white_content(&scale, &mut steps[..=LAST_NEUTRAL_FILL]);
        steps
    }

    /// In a dark theme, darkens the steps of `fills` that white content would not read on.
    fn hold_white_content(&self, scale: &ColorScale, fills: &mut [Color]) {
        hold_white_content(self.scheme, scale, fills);
    }

    /// The palette of `hue`, step by step ([`koebi_core::palette::HUE_STEPS`]); `None` for an
    /// unknown hue.
    pub fn hue(&self, hue: &str) -> Option<[Color; 16]> {
        let palette = HUES.iter().find(|palette| palette.name == hue)?;
        let scale = if let Some(&(_, seed)) = self.seeds.iter().find(|(name, _)| *name == hue) {
            ColorScale::from_seed(Color::from_hex(seed))
        } else {
            let keys = palette.keys.map(|key| {
                let lch = Color::from_hex(key).to_oklch();
                Oklch::new(lch.l, lch.c * self.chroma, lch.h).to_color()
            });
            ColorScale::new(&keys)?
        };
        let mut steps = scale.steps(self.background(), &self.hue_contrasts());
        self.hold_white_content(&scale, &mut steps[..=LAST_HUE_FILL]);
        Some(steps)
    }
}

/// The palettes of every built-in theme, the base theme first.
pub const THEMES: [ThemePalettes; 6] = [
    ThemePalettes {
        theme: "dark",
        scheme: Scheme::Dark,
        deepest: 0x00_00_00,
        background: 0x13_13_13,
        neutral_keys: &[0x13_13_13],
        neutral_contrasts: NEUTRAL_CONTRASTS,
        chroma: 1.0,
        seeds: &[],
    },
    ThemePalettes {
        theme: "dark-pink",
        scheme: Scheme::Dark,
        deepest: 0x00_00_00,
        background: 0x13_13_13,
        neutral_keys: &[0x13_13_13],
        neutral_contrasts: NEUTRAL_CONTRASTS,
        chroma: 1.0,
        seeds: &[],
    },
    // `RapidRAW`'s Muted Green: dark green surfaces, warm off-white text going to sepia.
    ThemePalettes {
        theme: "muted-green",
        scheme: Scheme::Dark,
        deepest: 0x0D_10_0B,
        background: 0x16_1A_14,
        neutral_keys: &[
            0x16_1A_14, 0x2D_32_28, 0x4B_52_45, 0x9B_A0_96, 0xE3_E1_DC, 0xF4_EF_DC,
        ],
        neutral_contrasts: NEUTRAL_CONTRASTS,
        chroma: 0.8,
        seeds: &[("celery", 0x8E_A0_66)],
    },
    // Warm light: reddish brown text, salmon accent (RapidRAW's Snow).
    ThemePalettes {
        theme: "snow",
        scheme: Scheme::Light,
        deepest: 0xFF_FF_FF,
        background: 0xF8_F5_F3,
        neutral_keys: &[0x3A_25_20, 0x7D_5F_55, 0xC9_B8_B1, 0xF3_EC_E9, 0xFF_FF_FF],
        neutral_contrasts: NEUTRAL_CONTRASTS,
        chroma: 1.0,
        seeds: &[("cinnamon", 0xD7_7B_6B)],
    },
    // Cool light: blue-gray text, slate accent (RapidRAW's Arctic).
    ThemePalettes {
        theme: "arctic",
        scheme: Scheme::Light,
        deepest: 0xFF_FF_FF,
        background: 0xF3_F6_F9,
        neutral_keys: &[0x1C_27_33, 0x64_78_8C, 0xB8_C4_CF, 0xF0_F5_F9, 0xFF_FF_FF],
        neutral_contrasts: NEUTRAL_CONTRASTS,
        chroma: 1.0,
        seeds: &[("indigo", 0x64_78_8C)],
    },
    // Black surfaces and white content: every surface below the borders black or nearly.
    ThemePalettes {
        theme: "high-contrast",
        scheme: Scheme::Dark,
        deepest: 0x00_00_00,
        background: 0x00_00_00,
        neutral_keys: &[0x00_00_00, 0xFF_FF_FF],
        neutral_contrasts: [
            0.0, 1.0, 1.04, 1.07, 1.1, 1.14, 1.2, 1.3, 1.45, 9.0, 12.0, 14.0, 16.0, 18.0, 19.5,
            21.0,
        ],
        chroma: 1.0,
        seeds: &[],
    },
];

/// A color role: a semantic palette of 16 steps (`semantic.<key>.color.<step>`) that every
/// state, part and component of its family aliases. Changing its steps changes them all.
#[derive(Clone, Copy, Debug)]
pub struct ColorRole {
    /// Its key in token paths.
    pub key: &'static str,
    /// Its name in the interface.
    pub name: &'static str,
    /// What it means.
    pub description: &'static str,
    /// The hue palette the base theme gives it.
    pub palette: &'static str,
    /// Its steps, 100 to 1600.
    pub steps: [Token<kind::Color, Semantic>; 16],
    /// The color it is made from (`semantic.<key>.color.source`).
    pub source: Token<kind::Color, Semantic>,
}

/// The color roles, the accent first.
pub const ROLES: [ColorRole; 5] = [
    ColorRole {
        key: "accent",
        name: "Accent",
        description: "Selection, focus, main actions.",
        palette: "blue",
        steps: [
            semantic::ACCENT_COLOR_100,
            semantic::ACCENT_COLOR_200,
            semantic::ACCENT_COLOR_300,
            semantic::ACCENT_COLOR_400,
            semantic::ACCENT_COLOR_500,
            semantic::ACCENT_COLOR_600,
            semantic::ACCENT_COLOR_700,
            semantic::ACCENT_COLOR_800,
            semantic::ACCENT_COLOR_900,
            semantic::ACCENT_COLOR_1000,
            semantic::ACCENT_COLOR_1100,
            semantic::ACCENT_COLOR_1200,
            semantic::ACCENT_COLOR_1300,
            semantic::ACCENT_COLOR_1400,
            semantic::ACCENT_COLOR_1500,
            semantic::ACCENT_COLOR_1600,
        ],
        source: semantic::ACCENT_COLOR_SOURCE,
    },
    ColorRole {
        key: "info",
        name: "Informative",
        description: "Information needing no action.",
        palette: "blue",
        steps: [
            semantic::INFO_COLOR_100,
            semantic::INFO_COLOR_200,
            semantic::INFO_COLOR_300,
            semantic::INFO_COLOR_400,
            semantic::INFO_COLOR_500,
            semantic::INFO_COLOR_600,
            semantic::INFO_COLOR_700,
            semantic::INFO_COLOR_800,
            semantic::INFO_COLOR_900,
            semantic::INFO_COLOR_1000,
            semantic::INFO_COLOR_1100,
            semantic::INFO_COLOR_1200,
            semantic::INFO_COLOR_1300,
            semantic::INFO_COLOR_1400,
            semantic::INFO_COLOR_1500,
            semantic::INFO_COLOR_1600,
        ],
        source: semantic::INFO_COLOR_SOURCE,
    },
    ColorRole {
        key: "positive",
        name: "Positive",
        description: "Success, what is valid or added.",
        palette: "green",
        steps: [
            semantic::POSITIVE_COLOR_100,
            semantic::POSITIVE_COLOR_200,
            semantic::POSITIVE_COLOR_300,
            semantic::POSITIVE_COLOR_400,
            semantic::POSITIVE_COLOR_500,
            semantic::POSITIVE_COLOR_600,
            semantic::POSITIVE_COLOR_700,
            semantic::POSITIVE_COLOR_800,
            semantic::POSITIVE_COLOR_900,
            semantic::POSITIVE_COLOR_1000,
            semantic::POSITIVE_COLOR_1100,
            semantic::POSITIVE_COLOR_1200,
            semantic::POSITIVE_COLOR_1300,
            semantic::POSITIVE_COLOR_1400,
            semantic::POSITIVE_COLOR_1500,
            semantic::POSITIVE_COLOR_1600,
        ],
        source: semantic::POSITIVE_COLOR_SOURCE,
    },
    ColorRole {
        key: "notice",
        name: "Notice",
        description: "Warnings, what needs attention.",
        palette: "orange",
        steps: [
            semantic::NOTICE_COLOR_100,
            semantic::NOTICE_COLOR_200,
            semantic::NOTICE_COLOR_300,
            semantic::NOTICE_COLOR_400,
            semantic::NOTICE_COLOR_500,
            semantic::NOTICE_COLOR_600,
            semantic::NOTICE_COLOR_700,
            semantic::NOTICE_COLOR_800,
            semantic::NOTICE_COLOR_900,
            semantic::NOTICE_COLOR_1000,
            semantic::NOTICE_COLOR_1100,
            semantic::NOTICE_COLOR_1200,
            semantic::NOTICE_COLOR_1300,
            semantic::NOTICE_COLOR_1400,
            semantic::NOTICE_COLOR_1500,
            semantic::NOTICE_COLOR_1600,
        ],
        source: semantic::NOTICE_COLOR_SOURCE,
    },
    ColorRole {
        key: "negative",
        name: "Negative",
        description: "Errors and destructive actions.",
        palette: "red",
        steps: [
            semantic::NEGATIVE_COLOR_100,
            semantic::NEGATIVE_COLOR_200,
            semantic::NEGATIVE_COLOR_300,
            semantic::NEGATIVE_COLOR_400,
            semantic::NEGATIVE_COLOR_500,
            semantic::NEGATIVE_COLOR_600,
            semantic::NEGATIVE_COLOR_700,
            semantic::NEGATIVE_COLOR_800,
            semantic::NEGATIVE_COLOR_900,
            semantic::NEGATIVE_COLOR_1000,
            semantic::NEGATIVE_COLOR_1100,
            semantic::NEGATIVE_COLOR_1200,
            semantic::NEGATIVE_COLOR_1300,
            semantic::NEGATIVE_COLOR_1400,
            semantic::NEGATIVE_COLOR_1500,
            semantic::NEGATIVE_COLOR_1600,
        ],
        source: semantic::NEGATIVE_COLOR_SOURCE,
    },
];

/// What an editable palette is: which tokens it writes and how it is made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// A color role (`semantic.<key>.color.<step>`), made like a hue.
    Role,
    /// The neutral palette (`primitive.color.gray.<step>`), tinted by a color.
    Neutral,
    /// A hue palette (`primitive.color.<key>.<step>`).
    Hue,
}

/// A palette the color settings can make again from one chosen color, its *source*: every
/// state, part and component aliasing its steps follows. Written as overrides of a theme, so
/// that the tokens stay the truth and a reset gives the theme's own palette back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Editable {
    /// Its key in token paths (`accent`, `gray`, `red`).
    pub key: &'static str,
    /// Its name in the interface.
    pub name: &'static str,
    /// What it colors.
    pub description: &'static str,
    /// Which tokens it writes.
    pub family: Family,
}

impl Editable {
    /// The token holding the color the palette is made from.
    pub fn source_path(&self) -> String {
        match self.family {
            Family::Role => format!("semantic.{}.color.source", self.key),
            Family::Neutral => "semantic.neutral.color.source".to_owned(),
            Family::Hue => format!("primitive.color.{}.source", self.key),
        }
    }

    /// The tokens of its steps, from the closest to the background.
    pub fn step_paths(&self) -> Vec<String> {
        match self.family {
            Family::Role => HUE_STEPS
                .iter()
                .map(|step| format!("semantic.{}.color.{step}", self.key))
                .collect(),
            Family::Neutral => NEUTRAL_STEPS
                .iter()
                .map(|step| format!("primitive.color.gray.{step}"))
                .collect(),
            Family::Hue => HUE_STEPS
                .iter()
                .map(|step| format!("primitive.color.{}.{step}", self.key))
                .collect(),
        }
    }

    /// Its steps made from `seed` for a theme whose deepest surface is `deepest` and whose
    /// background is `background`; a neutral palette keeps `contrasts`, its theme's.
    pub fn generate(
        &self,
        seed: Color,
        deepest: Color,
        background: Color,
        contrasts: &[f32; 16],
    ) -> [Color; 16] {
        match self.family {
            Family::Neutral => neutral_from_seed(seed, deepest, background, contrasts),
            Family::Role | Family::Hue => from_seed(seed, background),
        }
    }
}

/// The most chroma a tint gives the neutral palette: grays stay grays.
pub const MAX_TINT: f32 = 0.06;

/// The neutral palette tinted by `seed`: the hue of the seed, its chroma up to [`MAX_TINT`],
/// the theme's own lightness for the deepest surface and the background, then each step at its
/// contrast with the new background.
pub fn neutral_from_seed(
    seed: Color,
    deepest: Color,
    background: Color,
    contrasts: &[f32; 16],
) -> [Color; 16] {
    let tint = seed.with_alpha(1.0).to_oklch();
    let chroma = tint.c.min(MAX_TINT);
    let keys = [
        Oklch::new(0.08, chroma, tint.h).to_color(),
        Oklch::new(0.5, chroma, tint.h).to_color(),
        Oklch::new(0.96, chroma * 0.5, tint.h).to_color(),
    ];
    let scale = ColorScale::new(&keys).unwrap_or_else(|| ColorScale::from_seed(seed));
    let background = scale.at(background.to_oklch().l);
    let mut steps = contrasts.map(|ratio| scale.with_contrast(background, ratio));
    steps[0] = scale.at(deepest.to_oklch().l);
    steps[1] = background;
    hold_white_content(
        Scheme::of(background),
        &scale,
        &mut steps[..=LAST_NEUTRAL_FILL],
    );
    steps
}

/// Every palette the color settings edit: the roles, the neutral palette, then the hues.
pub fn editable() -> impl Iterator<Item = Editable> {
    const HUE_NAMES: [&str; 18] = [
        "Red",
        "Orange",
        "Yellow",
        "Chartreuse",
        "Celery",
        "Green",
        "Seafoam",
        "Cyan",
        "Blue",
        "Indigo",
        "Purple",
        "Fuchsia",
        "Magenta",
        "Pink",
        "Turquoise",
        "Cinnamon",
        "Brown",
        "Silver",
    ];
    let roles = ROLES.iter().map(|role| Editable {
        key: role.key,
        name: role.name,
        description: role.description,
        family: Family::Role,
    });
    let neutral = core::iter::once(Editable {
        key: "gray",
        name: "Neutral",
        description: "A tint of every gray.",
        family: Family::Neutral,
    });
    let hues = HUES.iter().zip(HUE_NAMES).map(|(hue, name)| Editable {
        key: hue.name,
        name,
        description: "Tags, charts, what aliases it.",
        family: Family::Hue,
    });
    roles.chain(neutral).chain(hues)
}

fn find(design: &DesignSystem, path: &str) -> Option<TokenId> {
    design.find(path)
}

fn color_in(design: &DesignSystem, theme: ThemeId, token: TokenId) -> Option<Color> {
    let view = design.view(theme, Layers::THEME | Layers::THEME_OVERRIDES)?;
    match view.value(token)? {
        Value::Color(color) => Some(color),
        _ => None,
    }
}

/// The color `palette` is made from in `theme`.
pub fn source(design: &DesignSystem, theme: ThemeId, palette: &Editable) -> Option<Color> {
    color_in(design, theme, find(design, &palette.source_path())?)
}

/// `true` when `palette` is made from a color of the user's in `theme`.
pub fn is_seeded(design: &DesignSystem, theme: ThemeId, palette: &Editable) -> bool {
    find(design, &palette.source_path())
        .is_some_and(|source| design.override_of(Scope::Theme(theme), source).is_some())
}

/// Makes `palette` again from `seed` in `theme`: its source and its steps become overrides of
/// the theme, each step at its contrast with the theme's background.
///
/// # Errors
///
/// When a token is missing from the schema or refuses its value ([`koebi_design::Error`]).
pub fn seed(
    design: &mut DesignSystem,
    theme: ThemeId,
    palette: &Editable,
    seed: Color,
) -> koebi_design::Result<()> {
    let missing = |path: &str| koebi_design::Error::UnknownToken(path.to_owned());
    let gray = |design: &DesignSystem, step: u16| {
        let path = format!("primitive.color.gray.{step}");
        let token = find(design, &path).ok_or_else(|| missing(&path))?;
        color_in(design, theme, token).ok_or_else(|| missing(&path))
    };
    // A tinted neutral palette starts from the theme's own grays, not from a former tint.
    let neutral_reset = palette.family == Family::Neutral && is_seeded(design, theme, palette);
    if neutral_reset {
        reset(design, theme, palette)?;
    }
    let deepest = gray(design, 25)?;
    let background = gray(design, 50)?;
    let key = design.theme(theme).map(|info| info.key().to_owned());
    let contrasts = key
        .as_deref()
        .and_then(self::theme)
        .map_or(NEUTRAL_CONTRASTS, |palettes| palettes.neutral_contrasts);
    let steps = palette.generate(seed.with_alpha(1.0), deepest, background, &contrasts);
    let scope = Scope::Theme(theme);
    let source = palette.source_path();
    let token = find(design, &source).ok_or_else(|| missing(&source))?;
    design.set_override(
        scope,
        token,
        Definition::Value(Value::Color(seed.with_alpha(1.0))),
    )?;
    for (path, color) in palette.step_paths().iter().zip(steps) {
        let token = find(design, path).ok_or_else(|| missing(path))?;
        design.set_override(scope, token, Definition::Value(Value::Color(color)))?;
    }
    Ok(())
}

/// Gives `palette` back its theme's own steps in `theme`: removes the overrides [`seed`] made.
///
/// # Errors
///
/// When the theme is unknown ([`koebi_design::Error`]).
pub fn reset(
    design: &mut DesignSystem,
    theme: ThemeId,
    palette: &Editable,
) -> koebi_design::Result<()> {
    let scope = Scope::Theme(theme);
    for path in core::iter::once(palette.source_path()).chain(palette.step_paths()) {
        if let Some(token) = find(design, &path) {
            design.reset(scope, token)?;
        }
    }
    Ok(())
}

/// The palettes of the theme `key`.
pub fn theme(key: &str) -> Option<&'static ThemePalettes> {
    THEMES.iter().find(|palettes| palettes.theme == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seeded_palette_follows_its_background() {
        let seed = Color::from_hex(0xD7_7B_6B);
        for background in [Color::from_hex(0x13_13_13), Color::from_hex(0xF8_F5_F3)] {
            let steps = from_seed(seed, background);
            let contrasts = Scheme::of(background).hue_contrasts();
            // Step 900 holds white text; the steps after it reach their contrast.
            assert!(Color::WHITE.contrast_ratio(steps[LAST_HUE_FILL]) >= 4.5);
            for (step, target) in steps.iter().zip(contrasts).skip(LAST_HUE_FILL + 1) {
                let ratio = step.contrast_ratio(background);
                assert!(
                    (ratio - target).abs() <= target * 0.04 || ratio > 15.0,
                    "{ratio}"
                );
            }
        }
        assert_eq!(Scheme::of(Color::BLACK), Scheme::Dark);
        assert_eq!(Scheme::of(Color::WHITE), Scheme::Light);
    }
}
