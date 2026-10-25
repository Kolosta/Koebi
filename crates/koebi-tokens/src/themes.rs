//! Built-in themes. The dark theme is the base theme: the token tables define it. Each theme below
//! lists the aliases it changes; every other token keeps its base definition.

use crate::{primitive, semantic};

koebi_design::theme! {
    /// Light theme: dark content on light surfaces.
    LIGHT = "light", "Light" {
        semantic::BACKGROUND_BASE_DEFAULT => primitive::COLOR_GRAY_50;
        semantic::BACKGROUND_SUNKEN_DEFAULT => primitive::COLOR_GRAY_100;
        semantic::BACKGROUND_RAISED_DEFAULT => primitive::COLOR_GRAY_25;
        semantic::BACKGROUND_FLOATING_DEFAULT => primitive::COLOR_GRAY_25;
        semantic::TEXT_COLOR_PRIMARY_DEFAULT => primitive::COLOR_GRAY_900;
        semantic::TEXT_COLOR_SECONDARY_DEFAULT => primitive::COLOR_GRAY_700;
        semantic::TEXT_COLOR_TERTIARY_DEFAULT => primitive::COLOR_GRAY_600;
        semantic::TEXT_COLOR_DISABLED_DEFAULT => primitive::COLOR_GRAY_400;
        semantic::ICON_COLOR_PRIMARY_DEFAULT => primitive::COLOR_GRAY_900;
        semantic::ICON_COLOR_SECONDARY_DEFAULT => primitive::COLOR_GRAY_600;
        semantic::BORDER_COLOR_DEFAULT => primitive::COLOR_GRAY_300;
        semantic::BORDER_COLOR_SUBTLE_DEFAULT => primitive::COLOR_GRAY_200;
        semantic::BORDER_COLOR_STRONG_DEFAULT => primitive::COLOR_GRAY_400;
        semantic::BORDER_COLOR_DISABLED_DEFAULT => primitive::COLOR_GRAY_300;
        semantic::BACKGROUND_APP_CONTROL => primitive::COLOR_GRAY_100;
        semantic::BACKGROUND_SURFACE_CANVAS => primitive::COLOR_GRAY_50;
        semantic::BACKGROUND_SURFACE_RAISED => primitive::COLOR_GRAY_25;
        semantic::BACKGROUND_APP_CHILD => primitive::COLOR_GRAY_25;
        semantic::BACKGROUND_APP_FRAME => primitive::COLOR_GRAY_25;
        semantic::BACKGROUND_APP_FRAME_HOVER => primitive::COLOR_GRAY_100;
    }

    /// The dark theme with a muted green accent.
    MUTED_GREEN = "muted-green", "Muted green" {
        semantic::ACCENT_COLOR_100 => primitive::COLOR_GREEN_100;
        semantic::ACCENT_COLOR_200 => primitive::COLOR_GREEN_200;
        semantic::ACCENT_COLOR_300 => primitive::COLOR_GREEN_300;
        semantic::ACCENT_COLOR_400 => primitive::COLOR_GREEN_400;
        semantic::ACCENT_COLOR_500 => primitive::COLOR_GREEN_500;
        semantic::ACCENT_COLOR_600 => primitive::COLOR_GREEN_600;
        semantic::ACCENT_COLOR_700 => primitive::COLOR_GREEN_700;
        semantic::ACCENT_COLOR_800 => primitive::COLOR_GREEN_800;
        semantic::ACCENT_COLOR_900 => primitive::COLOR_GREEN_900;
        semantic::ACCENT_COLOR_1000 => primitive::COLOR_GREEN_1000;
        semantic::ACCENT_COLOR_1100 => primitive::COLOR_GREEN_1100;
        semantic::ACCENT_COLOR_1200 => primitive::COLOR_GREEN_1200;
        semantic::ACCENT_COLOR_1300 => primitive::COLOR_GREEN_1300;
        semantic::ACCENT_COLOR_1400 => primitive::COLOR_GREEN_1400;
        semantic::ACCENT_COLOR_1500 => primitive::COLOR_GREEN_1500;
        semantic::ACCENT_COLOR_1600 => primitive::COLOR_GREEN_1600;
    }

    /// High-contrast theme: black surfaces, near-white content and thicker borders.
    HIGH_CONTRAST = "high-contrast", "High contrast" {
        semantic::BACKGROUND_BASE_DEFAULT => primitive::COLOR_GRAY_1000;
        semantic::BACKGROUND_RAISED_DEFAULT => primitive::COLOR_GRAY_900;
        semantic::BACKGROUND_FLOATING_DEFAULT => primitive::COLOR_GRAY_900;
        semantic::TEXT_COLOR_PRIMARY_DEFAULT => primitive::COLOR_GRAY_25;
        semantic::TEXT_COLOR_SECONDARY_DEFAULT => primitive::COLOR_GRAY_100;
        semantic::TEXT_COLOR_TERTIARY_DEFAULT => primitive::COLOR_GRAY_200;
        semantic::TEXT_COLOR_DISABLED_DEFAULT => primitive::COLOR_GRAY_400;
        semantic::TEXT_COLOR_PLACEHOLDER_DEFAULT => primitive::COLOR_GRAY_300;
        semantic::ICON_COLOR_PRIMARY_DEFAULT => primitive::COLOR_GRAY_25;
        semantic::ICON_COLOR_SECONDARY_DEFAULT => primitive::COLOR_GRAY_100;
        semantic::ICON_COLOR_TERTIARY_DEFAULT => primitive::COLOR_GRAY_200;
        semantic::BORDER_COLOR_DEFAULT => primitive::COLOR_GRAY_25;
        semantic::BORDER_COLOR_SUBTLE_DEFAULT => primitive::COLOR_GRAY_100;
        semantic::BORDER_COLOR_STRONG_DEFAULT => primitive::COLOR_GRAY_25;
        semantic::BORDER_COLOR_DISABLED_DEFAULT => primitive::COLOR_GRAY_300;
        semantic::BORDER_WIDTH_THIN => primitive::STROKE_WIDTH_200;
        semantic::BORDER_WIDTH_MEDIUM => primitive::STROKE_WIDTH_300;
        semantic::BORDER_WIDTH_THICK => primitive::STROKE_WIDTH_400;
        semantic::BORDER_WIDTH_FOCUS => primitive::STROKE_WIDTH_300;
        semantic::BACKGROUND_APP_CONTROL => primitive::COLOR_GRAY_900;
        semantic::BACKGROUND_SURFACE_CANVAS => primitive::COLOR_GRAY_1000;
        semantic::BACKGROUND_SURFACE_RAISED => primitive::COLOR_GRAY_900;
        semantic::BACKGROUND_APP_CHILD => primitive::COLOR_GRAY_900;
        semantic::BACKGROUND_APP_FRAME => primitive::COLOR_GRAY_800;
    }
}
