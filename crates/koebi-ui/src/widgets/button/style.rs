//! [`ButtonStyle`]: the colors of every variant and color family in every state, the look of
//! toggle buttons, and the metrics of buttons, read from the `component.button.*` and
//! `component.toggle.*` tokens.

use koebi_core::Color;
use koebi_design::Resolve;
use koebi_tokens::component;

use crate::widgets::parts::{HueColors, Shades, States, Tone, ToneColors, Variant, hued, shade};

/// Colors and metrics of buttons, from the `component.button.*` tokens.
///
/// Not `Copy`: it holds the colors of every tone (several kilobytes); components read it by
/// reference.
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonStyle {
    /// Height.
    pub height: f32,
    /// Smallest width of a button fitting its content.
    pub min_width: f32,
    /// Space between the sides and the content.
    pub padding: f32,
    /// Space between the parts of the content.
    pub gap: f32,
    /// Corner radius.
    pub corner_radius: f32,
    /// Border width.
    pub border_width: f32,
    /// Width of the borders around and between grouped buttons.
    pub group_border_width: f32,
    /// Side of the status dot.
    pub dot_size: f32,
    /// Side of icons.
    pub icon_size: f32,
    /// Stroke width of icons.
    pub stroke_width: f32,
    /// Colors of each variant, in the order of [`Variant::ALL`].
    pub variants: [ToneColors; 7],
    /// Colors of each variant in each color family: `hues[hue][variant]`.
    pub hues: [[ToneColors; 7]; 8],
    /// Colors of toggle buttons of the default tone (`component.toggle.*`).
    pub toggle: ToneColors,
    /// Corner radius of toggle buttons.
    pub toggle_corner_radius: f32,
    /// Border width of toggle buttons.
    pub toggle_border_width: f32,
}

impl ButtonStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        let get = |token| tokens.get(token);
        let disabled = (
            get(c::BUTTON_BACKGROUND_COLOR_DISABLED),
            get(c::BUTTON_BORDER_COLOR_DISABLED),
            get(c::BUTTON_LABEL_COLOR_DISABLED),
        );
        let label = States {
            selected: get(c::BUTTON_LABEL_COLOR_SELECTED),
            selected_hovered: get(c::BUTTON_LABEL_COLOR_SELECTED),
            disabled: disabled.2,
            ..States::all(get(c::BUTTON_LABEL_COLOR_DEFAULT))
        };
        let default = ToneColors {
            background: States {
                normal: get(c::BUTTON_BACKGROUND_COLOR_DEFAULT),
                hovered: get(c::BUTTON_BACKGROUND_COLOR_HOVER),
                pressed: get(c::BUTTON_BACKGROUND_COLOR_PRESSED),
                selected: get(c::BUTTON_BACKGROUND_COLOR_SELECTED),
                selected_hovered: get(c::BUTTON_BACKGROUND_COLOR_SELECTED_HOVER),
                disabled: disabled.0,
            },
            border: States {
                normal: get(c::BUTTON_BORDER_COLOR_DEFAULT),
                hovered: get(c::BUTTON_BORDER_COLOR_HOVER),
                pressed: get(c::BUTTON_BORDER_COLOR_PRESSED),
                selected: get(c::BUTTON_BORDER_COLOR_SELECTED),
                selected_hovered: get(c::BUTTON_BORDER_COLOR_SELECTED),
                disabled: disabled.1,
            },
            content: label,
            icon: label,
        };
        let shades = Shades {
            hover: tokens.get(c::BUTTON_SHADE_HOVER_LIGHTNESS),
            pressed: tokens.get(c::BUTTON_SHADE_PRESSED_LIGHTNESS),
            disabled_tint: tokens.get(c::BUTTON_HUE_DISABLED_TINT),
        };
        // A filled variant: its own colors, pressed when selected (lighter when hovered), the
        // shared disabled ones.
        let filled = |background: [Color; 3], label: Color, border: Color| ToneColors {
            background: States {
                normal: background[0],
                hovered: background[1],
                pressed: background[2],
                selected: background[2],
                selected_hovered: shade(background[2], shades.hover),
                disabled: disabled.0,
            },
            border: States {
                disabled: disabled.1,
                ..States::all(border)
            },
            content: States {
                disabled: disabled.2,
                ..States::all(label)
            },
            icon: States {
                disabled: disabled.2,
                ..States::all(label)
            },
        };
        let variants = [
            default,
            filled(
                [
                    get(c::BUTTON_ACCENT_BACKGROUND_COLOR_DEFAULT),
                    get(c::BUTTON_ACCENT_BACKGROUND_COLOR_HOVER),
                    get(c::BUTTON_ACCENT_BACKGROUND_COLOR_PRESSED),
                ],
                get(c::BUTTON_ACCENT_LABEL_COLOR_DEFAULT),
                get(c::BUTTON_ACCENT_BORDER_COLOR_DEFAULT),
            ),
            filled(
                [
                    get(c::BUTTON_POSITIVE_BACKGROUND_COLOR_DEFAULT),
                    get(c::BUTTON_POSITIVE_BACKGROUND_COLOR_HOVER),
                    get(c::BUTTON_POSITIVE_BACKGROUND_COLOR_PRESSED),
                ],
                get(c::BUTTON_POSITIVE_LABEL_COLOR_DEFAULT),
                get(c::BUTTON_POSITIVE_BORDER_COLOR_DEFAULT),
            ),
            filled(
                [
                    get(c::BUTTON_NEGATIVE_BACKGROUND_COLOR_DEFAULT),
                    get(c::BUTTON_NEGATIVE_BACKGROUND_COLOR_HOVER),
                    get(c::BUTTON_NEGATIVE_BACKGROUND_COLOR_PRESSED),
                ],
                get(c::BUTTON_NEGATIVE_LABEL_COLOR_DEFAULT),
                get(c::BUTTON_NEGATIVE_BORDER_COLOR_DEFAULT),
            ),
            filled(
                [
                    get(c::BUTTON_INFO_BACKGROUND_COLOR_DEFAULT),
                    get(c::BUTTON_INFO_BACKGROUND_COLOR_HOVER),
                    get(c::BUTTON_INFO_BACKGROUND_COLOR_PRESSED),
                ],
                get(c::BUTTON_INFO_LABEL_COLOR_DEFAULT),
                get(c::BUTTON_INFO_BORDER_COLOR_DEFAULT),
            ),
            filled(
                [
                    get(c::BUTTON_NEUTRAL_BACKGROUND_COLOR_DEFAULT),
                    get(c::BUTTON_NEUTRAL_BACKGROUND_COLOR_HOVER),
                    get(c::BUTTON_NEUTRAL_BACKGROUND_COLOR_PRESSED),
                ],
                get(c::BUTTON_NEUTRAL_LABEL_COLOR_DEFAULT),
                get(c::BUTTON_NEUTRAL_BORDER_COLOR_DEFAULT),
            ),
            filled(
                [
                    get(c::BUTTON_NOTICE_BACKGROUND_COLOR_DEFAULT),
                    get(c::BUTTON_NOTICE_BACKGROUND_COLOR_HOVER),
                    get(c::BUTTON_NOTICE_BACKGROUND_COLOR_PRESSED),
                ],
                get(c::BUTTON_NOTICE_LABEL_COLOR_DEFAULT),
                get(c::BUTTON_NOTICE_BORDER_COLOR_DEFAULT),
            ),
        ];
        let families = [
            (
                c::BUTTON_HUE_BLUE_FILL,
                c::BUTTON_HUE_BLUE_SUBTLE,
                c::BUTTON_HUE_BLUE_TEXT,
            ),
            (
                c::BUTTON_HUE_CYAN_FILL,
                c::BUTTON_HUE_CYAN_SUBTLE,
                c::BUTTON_HUE_CYAN_TEXT,
            ),
            (
                c::BUTTON_HUE_GREEN_FILL,
                c::BUTTON_HUE_GREEN_SUBTLE,
                c::BUTTON_HUE_GREEN_TEXT,
            ),
            (
                c::BUTTON_HUE_YELLOW_FILL,
                c::BUTTON_HUE_YELLOW_SUBTLE,
                c::BUTTON_HUE_YELLOW_TEXT,
            ),
            (
                c::BUTTON_HUE_ORANGE_FILL,
                c::BUTTON_HUE_ORANGE_SUBTLE,
                c::BUTTON_HUE_ORANGE_TEXT,
            ),
            (
                c::BUTTON_HUE_RED_FILL,
                c::BUTTON_HUE_RED_SUBTLE,
                c::BUTTON_HUE_RED_TEXT,
            ),
            (
                c::BUTTON_HUE_MAGENTA_FILL,
                c::BUTTON_HUE_MAGENTA_SUBTLE,
                c::BUTTON_HUE_MAGENTA_TEXT,
            ),
            (
                c::BUTTON_HUE_PURPLE_FILL,
                c::BUTTON_HUE_PURPLE_SUBTLE,
                c::BUTTON_HUE_PURPLE_TEXT,
            ),
        ];
        let hues = families.map(|(fill, subtle, text)| {
            let family = HueColors {
                fill: get(fill),
                subtle: get(subtle),
                text: get(text),
            };
            Variant::ALL.map(|variant| hued(variant, &variants[variant.index()], family, shades))
        });
        let toggle_label = States {
            normal: get(c::TOGGLE_LABEL_COLOR_DEFAULT),
            hovered: get(c::TOGGLE_LABEL_COLOR_HOVER),
            pressed: get(c::TOGGLE_LABEL_COLOR_HOVER),
            selected: get(c::TOGGLE_LABEL_COLOR_SELECTED),
            selected_hovered: get(c::TOGGLE_LABEL_COLOR_SELECTED),
            disabled: get(c::TOGGLE_LABEL_COLOR_DISABLED),
        };
        let toggle = ToneColors {
            background: States {
                normal: get(c::TOGGLE_BACKGROUND_COLOR_DEFAULT),
                hovered: get(c::TOGGLE_BACKGROUND_COLOR_HOVER),
                pressed: get(c::TOGGLE_BACKGROUND_COLOR_PRESSED),
                selected: get(c::TOGGLE_BACKGROUND_COLOR_SELECTED),
                selected_hovered: get(c::TOGGLE_BACKGROUND_COLOR_SELECTED_HOVER),
                disabled: get(c::TOGGLE_BACKGROUND_COLOR_DISABLED),
            },
            border: States {
                normal: get(c::TOGGLE_BORDER_COLOR_DEFAULT),
                hovered: get(c::TOGGLE_BORDER_COLOR_HOVER),
                pressed: get(c::TOGGLE_BORDER_COLOR_HOVER),
                selected: get(c::TOGGLE_BORDER_COLOR_SELECTED),
                selected_hovered: get(c::TOGGLE_BORDER_COLOR_SELECTED),
                disabled: get(c::TOGGLE_BORDER_COLOR_DISABLED),
            },
            content: toggle_label,
            icon: toggle_label,
        };
        Self {
            height: tokens.get(c::BUTTON_HEIGHT_DEFAULT),
            min_width: tokens.get(c::BUTTON_MIN_WIDTH_DEFAULT),
            padding: tokens.get(c::BUTTON_PADDING_HORIZONTAL_DEFAULT),
            gap: tokens.get(c::BUTTON_GAP_DEFAULT),
            corner_radius: tokens.get(c::BUTTON_CORNER_RADIUS_DEFAULT),
            border_width: tokens.get(c::BUTTON_BORDER_WIDTH_DEFAULT),
            group_border_width: tokens.get(c::BUTTON_GROUP_BORDER_WIDTH_DEFAULT),
            dot_size: tokens.get(c::BUTTON_STATUS_DOT_SIZE_DEFAULT),
            icon_size: tokens.get(c::ICON_SIZE_DEFAULT),
            stroke_width: tokens.get(c::ICON_STROKE_WIDTH_DEFAULT),
            variants,
            hues,
            toggle,
            toggle_corner_radius: tokens.get(c::TOGGLE_CORNER_RADIUS_DEFAULT),
            toggle_border_width: tokens.get(c::TOGGLE_BORDER_WIDTH_DEFAULT),
        }
    }

    /// The colors of `tone` in every state.
    pub fn colors(&self, tone: Tone) -> &ToneColors {
        match tone.hue {
            Some(hue) => &self.hues[hue.index()][tone.variant.index()],
            None => &self.variants[tone.variant.index()],
        }
    }
}
