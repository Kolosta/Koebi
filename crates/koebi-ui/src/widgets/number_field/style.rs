//! [`NumberFieldStyle`]: the colors and metrics of number fields, from the
//! `component.number-field.*` tokens.

use koebi_core::Color;
use koebi_design::Resolve;
use koebi_tokens::component;

use crate::widgets::parts::States;

/// Colors and metrics of number fields, from the `component.number-field.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumberFieldStyle {
    /// Background per state; pressed is a press that has not moved yet, selected a drag.
    pub background: States<Color>,
    /// Border color per state; selected is a drag.
    pub border: States<Color>,
    /// Border of a typed expression that is not valid.
    pub error: Color,
    /// Bar per state; hovered also while dragging.
    pub fill: States<Color>,
    /// Value color.
    pub text: Color,
    /// Value color when disabled.
    pub text_disabled: Color,
    /// Label color.
    pub label: Color,
    /// Unit color.
    pub unit: Color,
    /// Step button chevrons, and hovered.
    pub step: Color,
    /// See `step`.
    pub step_hovered: Color,
    /// Step button background, hovered.
    pub step_background: Color,
    /// Height.
    pub height: f32,
    /// Smallest width.
    pub min_width: f32,
    /// Space between the step buttons (or the sides) and the text.
    pub padding: f32,
    /// Width of a step button (height in vertical fields).
    pub step_width: f32,
    /// Side of a step button's chevron.
    pub step_icon_size: f32,
    /// Corner radius.
    pub corner_radius: f32,
    /// Border width.
    pub border_width: f32,
    /// Pointer travel that changes an unbounded value by one step.
    pub step_distance: f32,
    /// How much finer Shift makes a drag or a step.
    pub fine_factor: f32,
}

impl NumberFieldStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        let get = |token| tokens.get(token);
        let drag = get(c::NUMBER_FIELD_BACKGROUND_COLOR_DRAG);
        let border_drag = get(c::NUMBER_FIELD_BORDER_COLOR_DRAG);
        let border_hover = get(c::NUMBER_FIELD_BORDER_COLOR_HOVER);
        let fill_hover = get(c::NUMBER_FIELD_FILL_COLOR_HOVER);
        Self {
            background: States {
                normal: get(c::NUMBER_FIELD_BACKGROUND_COLOR_DEFAULT),
                hovered: get(c::NUMBER_FIELD_BACKGROUND_COLOR_HOVER),
                pressed: get(c::NUMBER_FIELD_BACKGROUND_COLOR_PRESSED),
                selected: drag,
                selected_hovered: drag,
                disabled: get(c::NUMBER_FIELD_BACKGROUND_COLOR_DISABLED),
            },
            border: States {
                normal: get(c::NUMBER_FIELD_BORDER_COLOR_DEFAULT),
                hovered: border_hover,
                pressed: border_hover,
                selected: border_drag,
                selected_hovered: border_drag,
                disabled: get(c::NUMBER_FIELD_BORDER_COLOR_DISABLED),
            },
            error: get(c::NUMBER_FIELD_BORDER_COLOR_ERROR),
            fill: States {
                normal: get(c::NUMBER_FIELD_FILL_COLOR_DEFAULT),
                hovered: fill_hover,
                pressed: fill_hover,
                selected: fill_hover,
                selected_hovered: fill_hover,
                disabled: get(c::NUMBER_FIELD_FILL_COLOR_DISABLED),
            },
            text: get(c::NUMBER_FIELD_TEXT_COLOR_DEFAULT),
            text_disabled: get(c::NUMBER_FIELD_TEXT_COLOR_DISABLED),
            label: get(c::NUMBER_FIELD_LABEL_COLOR_DEFAULT),
            unit: get(c::NUMBER_FIELD_UNIT_COLOR_DEFAULT),
            step: get(c::NUMBER_FIELD_STEP_BUTTON_COLOR_DEFAULT),
            step_hovered: get(c::NUMBER_FIELD_STEP_BUTTON_COLOR_HOVER),
            step_background: get(c::NUMBER_FIELD_STEP_BUTTON_BACKGROUND_COLOR_HOVER),
            height: tokens.get(c::NUMBER_FIELD_SIZE_HEIGHT),
            min_width: tokens.get(c::NUMBER_FIELD_MIN_WIDTH_DEFAULT),
            padding: tokens.get(c::NUMBER_FIELD_PADDING_HORIZONTAL_DEFAULT),
            step_width: tokens.get(c::NUMBER_FIELD_STEP_BUTTON_WIDTH),
            step_icon_size: tokens.get(c::NUMBER_FIELD_STEP_BUTTON_ICON_SIZE),
            corner_radius: tokens.get(c::NUMBER_FIELD_CORNER_RADIUS_DEFAULT),
            border_width: tokens.get(c::NUMBER_FIELD_BORDER_WIDTH_DEFAULT),
            step_distance: tokens.get(c::NUMBER_FIELD_DRAG_STEP_DISTANCE),
            fine_factor: tokens.get(c::NUMBER_FIELD_DRAG_FINE_FACTOR),
        }
    }
}
