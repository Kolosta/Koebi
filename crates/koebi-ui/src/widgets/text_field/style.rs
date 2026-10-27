//! [`TextFieldStyle`]: the colors and metrics of text fields, from the `component.text-field.*`
//! tokens.

use core::time::Duration;

use koebi_core::Color;
use koebi_design::Resolve;
use koebi_tokens::component;

use crate::widgets::parts::States;

/// Colors and metrics of text fields, from the `component.text-field.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextFieldStyle {
    /// Background per state; selected is editing.
    pub background: States<Color>,
    /// Border color per state; selected is editing.
    pub border: States<Color>,
    /// Text color.
    pub text: Color,
    /// Text color when disabled.
    pub text_disabled: Color,
    /// Color of the placeholder shown while the text is empty.
    pub placeholder: Color,
    /// Icon color.
    pub icon: Color,
    /// Icon color when disabled.
    pub icon_disabled: Color,
    /// Background of the selected text.
    pub selection: Color,
    /// Caret color.
    pub caret: Color,
    /// Caret width.
    pub caret_width: f32,
    /// Time the caret stays shown, then hidden; zero keeps it shown.
    pub caret_blink: Duration,
    /// Height.
    pub height: f32,
    /// Smallest width.
    pub min_width: f32,
    /// Space between the sides and the text.
    pub padding: f32,
    /// Space between the icon and the text.
    pub gap: f32,
    /// Side of the icon.
    pub icon_size: f32,
    /// Corner radius.
    pub corner_radius: f32,
    /// Border width.
    pub border_width: f32,
}

impl TextFieldStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        let get = |token| tokens.get(token);
        let editing = get(c::TEXT_FIELD_BACKGROUND_COLOR_EDITING);
        let border_hover = get(c::TEXT_FIELD_BORDER_COLOR_HOVER);
        let border_editing = get(c::TEXT_FIELD_BORDER_COLOR_EDITING);
        Self {
            background: States {
                normal: get(c::TEXT_FIELD_BACKGROUND_COLOR_DEFAULT),
                hovered: get(c::TEXT_FIELD_BACKGROUND_COLOR_HOVER),
                pressed: get(c::TEXT_FIELD_BACKGROUND_COLOR_HOVER),
                selected: editing,
                selected_hovered: editing,
                disabled: get(c::TEXT_FIELD_BACKGROUND_COLOR_DISABLED),
            },
            border: States {
                normal: get(c::TEXT_FIELD_BORDER_COLOR_DEFAULT),
                hovered: border_hover,
                pressed: border_hover,
                selected: border_editing,
                selected_hovered: border_editing,
                disabled: get(c::TEXT_FIELD_BORDER_COLOR_DISABLED),
            },
            text: get(c::TEXT_FIELD_TEXT_COLOR_DEFAULT),
            text_disabled: get(c::TEXT_FIELD_TEXT_COLOR_DISABLED),
            placeholder: get(c::TEXT_FIELD_PLACEHOLDER_COLOR_DEFAULT),
            icon: get(c::TEXT_FIELD_ICON_COLOR_DEFAULT),
            icon_disabled: get(c::TEXT_FIELD_ICON_COLOR_DISABLED),
            selection: get(c::TEXT_FIELD_SELECTION_COLOR_DEFAULT),
            caret: get(c::TEXT_FIELD_CARET_COLOR_DEFAULT),
            caret_width: tokens.get(c::TEXT_FIELD_CARET_WIDTH),
            caret_blink: tokens.get(c::TEXT_FIELD_CARET_BLINK_INTERVAL),
            height: tokens.get(c::TEXT_FIELD_SIZE_HEIGHT),
            min_width: tokens.get(c::TEXT_FIELD_MIN_WIDTH_DEFAULT),
            padding: tokens.get(c::TEXT_FIELD_PADDING_HORIZONTAL_DEFAULT),
            gap: tokens.get(c::TEXT_FIELD_GAP_DEFAULT),
            icon_size: tokens.get(c::TEXT_FIELD_ICON_SIZE),
            corner_radius: tokens.get(c::TEXT_FIELD_CORNER_RADIUS_DEFAULT),
            border_width: tokens.get(c::TEXT_FIELD_BORDER_WIDTH_DEFAULT),
        }
    }
}
