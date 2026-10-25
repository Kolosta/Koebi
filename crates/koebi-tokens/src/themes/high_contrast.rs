//! The high-contrast theme.

use crate::{primitive, semantic};

koebi_design::theme! {
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
