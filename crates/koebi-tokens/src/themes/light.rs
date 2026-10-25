//! The light theme.

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
}
