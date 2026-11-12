//! [`ButtonStyle`]: the colors of every variant and color family in every state, the look of
//! toggle buttons, and the metrics of buttons, read from the `component.button.*` and
//! `component.toggle.*` tokens.

use core::cell::OnceCell;

use koebi_core::Color;
use koebi_design::Resolve;
use koebi_tokens::component;

use crate::widgets::parts::{
    HueColors, HueLadders, Shades, States, Tone, ToneColors, Variant, shade,
};

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
    /// Colors of each variant, in the order of [`Variant::ALL`].
    pub variants: [ToneColors; 7],
    /// The colors of each color family, by [`Hue::index`](crate::Hue), and how their states
    /// derive from them.
    families: [HueColors; 8],
    shades: Shades,
    /// The colors of each variant in each family, derived on first use ([`Self::colors`]).
    hues: HueTones,
    /// Colors of toggle buttons of the default tone (`component.toggle.*`).
    pub toggle: ToneColors,
    /// Share of its selected colors a selected button keeps once disabled.
    pub selected_disabled_tint: f32,
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
            selected_pressed: get(c::BUTTON_LABEL_COLOR_SELECTED),
            active: get(c::BUTTON_LABEL_COLOR_ACTIVE),
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
                selected_pressed: get(c::BUTTON_BACKGROUND_COLOR_SELECTED_PRESSED),
                active: get(c::BUTTON_BACKGROUND_COLOR_ACTIVE),
                disabled: disabled.0,
                selected_disabled: disabled.0,
            },
            border: States {
                normal: get(c::BUTTON_BORDER_COLOR_DEFAULT),
                hovered: get(c::BUTTON_BORDER_COLOR_HOVER),
                pressed: get(c::BUTTON_BORDER_COLOR_PRESSED),
                selected: get(c::BUTTON_BORDER_COLOR_SELECTED),
                selected_hovered: get(c::BUTTON_BORDER_COLOR_SELECTED),
                selected_pressed: get(c::BUTTON_BORDER_COLOR_SELECTED),
                active: get(c::BUTTON_BORDER_COLOR_ACTIVE),
                disabled: disabled.1,
                selected_disabled: disabled.1,
            },
            content: label,
            icon: label,
        };
        let shades = Shades {
            hover: tokens.get(c::BUTTON_SHADE_HOVER_LIGHTNESS),
            pressed: tokens.get(c::BUTTON_SHADE_PRESSED_LIGHTNESS),
            disabled_tint: tokens.get(c::BUTTON_HUE_DISABLED_TINT),
        };
        // A filled variant: its own colors, pressed when selected (lighter when hovered,
        // darker when pressed again), active as selected, the shared disabled ones.
        let filled = |background: [Color; 3], label: Color, border: Color| ToneColors {
            background: States {
                normal: background[0],
                hovered: background[1],
                pressed: background[2],
                selected: background[2],
                selected_hovered: shade(background[2], shades.hover),
                selected_pressed: shade(background[2], shades.pressed),
                active: background[2],
                disabled: disabled.0,
                selected_disabled: disabled.0,
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
        let families = families.map(|(fill, subtle, text)| HueColors {
            fill: get(fill),
            subtle: get(subtle),
            text: get(text),
        });
        let toggle_label = States {
            normal: get(c::TOGGLE_LABEL_COLOR_DEFAULT),
            hovered: get(c::TOGGLE_LABEL_COLOR_HOVER),
            pressed: get(c::TOGGLE_LABEL_COLOR_HOVER),
            selected: get(c::TOGGLE_LABEL_COLOR_SELECTED),
            selected_hovered: get(c::TOGGLE_LABEL_COLOR_SELECTED),
            selected_pressed: get(c::TOGGLE_LABEL_COLOR_SELECTED),
            active: get(c::TOGGLE_LABEL_COLOR_SELECTED),
            disabled: get(c::TOGGLE_LABEL_COLOR_DISABLED),
            selected_disabled: get(c::TOGGLE_LABEL_COLOR_DISABLED),
        };
        let toggle = ToneColors {
            background: States {
                normal: get(c::TOGGLE_BACKGROUND_COLOR_DEFAULT),
                hovered: get(c::TOGGLE_BACKGROUND_COLOR_HOVER),
                pressed: get(c::TOGGLE_BACKGROUND_COLOR_PRESSED),
                selected: get(c::TOGGLE_BACKGROUND_COLOR_SELECTED),
                selected_hovered: get(c::TOGGLE_BACKGROUND_COLOR_SELECTED_HOVER),
                selected_pressed: get(c::TOGGLE_BACKGROUND_COLOR_SELECTED_PRESSED),
                active: get(c::TOGGLE_BACKGROUND_COLOR_SELECTED),
                disabled: get(c::TOGGLE_BACKGROUND_COLOR_DISABLED),
                selected_disabled: get(c::TOGGLE_BACKGROUND_COLOR_DISABLED),
            },
            border: States {
                normal: get(c::TOGGLE_BORDER_COLOR_DEFAULT),
                hovered: get(c::TOGGLE_BORDER_COLOR_HOVER),
                pressed: get(c::TOGGLE_BORDER_COLOR_HOVER),
                selected: get(c::TOGGLE_BORDER_COLOR_SELECTED),
                selected_hovered: get(c::TOGGLE_BORDER_COLOR_SELECTED),
                selected_pressed: get(c::TOGGLE_BORDER_COLOR_SELECTED),
                active: get(c::TOGGLE_BORDER_COLOR_SELECTED),
                disabled: get(c::TOGGLE_BORDER_COLOR_DISABLED),
                selected_disabled: get(c::TOGGLE_BORDER_COLOR_DISABLED),
            },
            content: toggle_label,
            icon: toggle_label,
        };
        // Switched on, then disabled: the disabled colors, tinted with the selected ones.
        let tint = tokens.get(c::BUTTON_SELECTED_DISABLED_TINT);
        let variants = variants.map(|colors| colors.with_selected_disabled(tint));
        let toggle = toggle.with_selected_disabled(tint);
        Self {
            height: tokens.get(c::BUTTON_HEIGHT_DEFAULT),
            min_width: tokens.get(c::BUTTON_MIN_WIDTH_DEFAULT),
            padding: tokens.get(c::BUTTON_PADDING_HORIZONTAL_DEFAULT),
            gap: tokens.get(c::BUTTON_GAP_DEFAULT),
            corner_radius: tokens.get(c::BUTTON_CORNER_RADIUS_DEFAULT),
            border_width: tokens.get(c::BUTTON_BORDER_WIDTH_DEFAULT),
            group_border_width: tokens.get(c::BUTTON_GROUP_BORDER_WIDTH_DEFAULT),
            dot_size: tokens.get(c::BUTTON_STATUS_DOT_SIZE_DEFAULT),
            icon_size: crate::style::icon_size(tokens),
            variants,
            families,
            shades,
            hues: HueTones::default(),
            toggle,
            selected_disabled_tint: tint,
            toggle_corner_radius: tokens.get(c::TOGGLE_CORNER_RADIUS_DEFAULT),
            toggle_border_width: tokens.get(c::TOGGLE_BORDER_WIDTH_DEFAULT),
        }
    }

    /// The colors of `tone` in every state. Those of a color family are derived the first time
    /// one of its buttons is drawn: each of its colors is a shade or a mix in OKLab, and most
    /// screens show no colored button.
    pub fn colors(&self, tone: Tone) -> &ToneColors {
        match tone.hue {
            Some(hue) => {
                let tones = self.hues.0[hue.index()].get_or_init(|| {
                    let family = HueLadders::new(self.families[hue.index()], self.shades);
                    // The variants' own selected and disabled colors, derived already, are
                    // derived again for the family: only the others come from the variant.
                    Variant::ALL.map(|variant| {
                        family
                            .tone(variant, &self.variants[variant.index()])
                            .with_selected_disabled(self.selected_disabled_tint)
                    })
                });
                &tones[tone.variant.index()]
            }
            None => &self.variants[tone.variant.index()],
        }
    }
}

/// The colors of each variant in each color family, `[hue][variant]`, derived on first use: a
/// cache. They follow from the rest of the style, so that two styles equal in the rest are
/// equal whatever their caches hold.
#[derive(Clone, Debug, Default)]
struct HueTones([OnceCell<[ToneColors; 7]>; 8]);

impl PartialEq for HueTones {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
