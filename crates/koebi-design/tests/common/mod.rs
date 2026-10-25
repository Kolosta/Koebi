//! A small schema exercising every kind of reference, shared by the integration tests.
//!
//! ```text
//! component::BUTTON_BACKGROUND_HOVER ─► component::BUTTON_BACKGROUND ─► semantic::BACKGROUND_ACCENT
//!   ─► semantic::ACCENT_COLOR_700 ─► primitive::COLOR_BLUE_700   (green theme: COLOR_GREEN_700)
//! component::DEEP ─► semantic::CHAIN_8 ─► ... ─► semantic::CHAIN_1 ─► primitive::COLOR_RED_700
//! component::LOGO ─► semantic::BRAND_LOGO ─► primitive::COLOR_PURPLE_500
//! ```

use koebi_design::schema::{Schema, ThemeInfo};

pub(crate) mod primitive {
    use koebi_design::define::*;

    koebi_design::tokens! {
        group PRIMITIVE: Primitive, start = 0;
        /// Blue 700.
        COLOR_BLUE_700 = "color.blue.700": Color = hex("#5D89FF");
        /// Green 700.
        COLOR_GREEN_700 = "color.green.700": Color = hex("#139E5C");
        /// Red 700.
        COLOR_RED_700 = "color.red.700": Color = hex("#E3403C");
        /// Purple 500, the brand color.
        COLOR_PURPLE_500 = "color.purple.500": Color = hex("#9B59F0");
        /// Gray 50.
        COLOR_GRAY_50 = "color.gray.50": Color = hex("#F8F8F8");
        /// Gray 500.
        COLOR_GRAY_500 = "color.gray.500": Color = hex("#808080");
        /// Gray 900.
        COLOR_GRAY_900 = "color.gray.900": Color = hex("#131313");
        /// Radius 100.
        RADIUS_100 = "radius.100": Dimension = px(2.0), range(0.0, 32.0);
        /// Radius 200.
        RADIUS_200 = "radius.200": Dimension = px(4.0), range(0.0, 32.0);
        /// Spacing 100.
        SPACING_100 = "spacing.100": Dimension = px(4.0), range(0.0, 256.0);
        /// 50 % opacity.
        OPACITY_500 = "opacity.500": Number = 0.5, range(0.0, 1.0);
        /// Middle stacking layer.
        LAYER_MID = "layer.mid": Integer = 100, range(0.0, 10_000.0);
        /// Fast animation.
        DURATION_FAST = "duration.fast": Duration = ms(100), range(0.0, 2000.0);
        /// Standard easing.
        EASING_EASE = "easing.ease": CubicBezier = bezier(0.25, 0.1, 0.25, 1.0);
        /// Sans-serif family.
        FONT_FAMILY_SANS = "font.family.sans": FontFamily = family("Noto Sans");
        /// Body size.
        FONT_SIZE_60 = "font.size.60": Dimension = px(15.0), range(6.0, 200.0);
        /// Regular weight.
        FONT_WEIGHT_REGULAR = "font.weight.regular": FontWeight = weight(400);
        /// Bold weight.
        FONT_WEIGHT_BOLD = "font.weight.bold": FontWeight = weight(700);
        /// Normal line height.
        FONT_LINE_HEIGHT_NORMAL = "font.line-height.normal": Number = 1.5;
        /// Normal letter spacing.
        FONT_TRACKING_NORMAL = "font.tracking.normal": Number = 0.0;
    }
}

pub(crate) mod semantic {
    use super::primitive;

    koebi_design::tokens! {
        group SEMANTIC: Semantic, start = primitive::PRIMITIVE::END;
        /// Accent, step 700.
        ACCENT_COLOR_700 = "accent.color.700": Color -> primitive::COLOR_BLUE_700;
        /// Background of accent elements (semantic to semantic).
        BACKGROUND_ACCENT = "background.accent": Color -> ACCENT_COLOR_700;
        /// Base background.
        BACKGROUND_BASE = "background.base": Color -> primitive::COLOR_GRAY_900;
        /// Primary text.
        TEXT_PRIMARY = "text.primary": Color -> primitive::COLOR_GRAY_50;
        /// Neutral border.
        BORDER_NEUTRAL = "border.neutral": Color -> primitive::COLOR_GRAY_500;
        /// The brand logo.
        BRAND_LOGO = "brand.logo": Color -> primitive::COLOR_PURPLE_500;
        /// Control radius.
        RADIUS_CONTROL = "radius.control": Dimension -> primitive::RADIUS_100;
        /// Body font weight.
        FONT_WEIGHT_BODY = "font.weight.body": FontWeight -> primitive::FONT_WEIGHT_REGULAR;
        /// Chain, link 1.
        CHAIN_1 = "chain.1": Color -> primitive::COLOR_RED_700;
        /// Chain, link 2.
        CHAIN_2 = "chain.2": Color -> CHAIN_1;
        /// Chain, link 3.
        CHAIN_3 = "chain.3": Color -> CHAIN_2;
        /// Chain, link 4.
        CHAIN_4 = "chain.4": Color -> CHAIN_3;
        /// Chain, link 5.
        CHAIN_5 = "chain.5": Color -> CHAIN_4;
        /// Chain, link 6.
        CHAIN_6 = "chain.6": Color -> CHAIN_5;
        /// Chain, link 7.
        CHAIN_7 = "chain.7": Color -> CHAIN_6;
        /// Chain, link 8.
        CHAIN_8 = "chain.8": Color -> CHAIN_7;
    }

    koebi_design::typography! {
        group TEXT: Semantic, start = SEMANTIC::END;
        /// Body text.
        TEXT_BODY = "text.body" {
            font_family: primitive::FONT_FAMILY_SANS,
            font_size: primitive::FONT_SIZE_60,
            font_weight: FONT_WEIGHT_BODY,
            line_height: primitive::FONT_LINE_HEIGHT_NORMAL,
            letter_spacing: primitive::FONT_TRACKING_NORMAL,
        }
    }
}

pub(crate) mod component {
    use super::semantic;
    use koebi_design::define::*;

    koebi_design::tokens! {
        group COMPONENT: Component, start = semantic::TEXT::END;
        /// Button background (component to semantic).
        BUTTON_BACKGROUND = "button.background": Color -> semantic::BACKGROUND_ACCENT;
        /// Hovered button background (component to component).
        BUTTON_BACKGROUND_HOVER = "button.background-hover": Color -> BUTTON_BACKGROUND;
        /// Button label.
        BUTTON_LABEL = "button.label": Color -> semantic::TEXT_PRIMARY;
        /// Button border.
        BUTTON_BORDER = "button.border": Color -> semantic::BORDER_NEUTRAL;
        /// Button corner radius.
        BUTTON_RADIUS = "button.radius": Dimension -> semantic::RADIUS_CONTROL;
        /// Logo color.
        LOGO = "logo": Color -> semantic::BRAND_LOGO;
        /// End of a long chain.
        DEEP = "deep": Color -> semantic::CHAIN_8;
        /// Scrollbar thickness: a component token holding its own value.
        SCROLLBAR_SIZE = "scrollbar.size": Dimension = px(14.0), range(1.0, 32.0);
    }
}

pub(crate) mod themes {
    use super::{primitive, semantic};

    koebi_design::theme! {
        /// Light surfaces.
        LIGHT = "light", "Light" {
            semantic::BACKGROUND_BASE => primitive::COLOR_GRAY_50;
            semantic::TEXT_PRIMARY => primitive::COLOR_GRAY_900;
        }

        /// Green accent.
        GREEN = "green", "Green" {
            semantic::ACCENT_COLOR_700 => primitive::COLOR_GREEN_700;
        }
    }
}

pub(crate) const SCHEMA: Schema = Schema::new(
    "fixture",
    ThemeInfo::new("dark", "Dark", "Base theme.", &[]),
    &[
        primitive::PRIMITIVE::GROUP,
        semantic::SEMANTIC::GROUP,
        semantic::TEXT::GROUP,
        component::COMPONENT::GROUP,
    ],
    &[themes::LIGHT, themes::GREEN],
);

const _: () = SCHEMA.assert_valid();

pub(crate) fn hex(rgb: u32) -> koebi_core::Color {
    koebi_core::Color::from_hex(rgb)
}
