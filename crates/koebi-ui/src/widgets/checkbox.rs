//! Checkboxes: a box, its label beside it and an optional description below, all of it
//! reacting to the pointer; checked, unchecked or mixed (a group of options partly checked).

use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_design::kind::{self, Kind};
use koebi_render::RectPrimitive;
use koebi_tokens::component;

use crate::icon::Icon;
use crate::id::WidgetId;
use crate::style::Prop;
use crate::text::Align;
use crate::ui::{Response, Ui};
use crate::widgets::parts::{ControlState, States};
use crate::widgets::snap;

/// Colors and metrics of checkboxes, from the `component.checkbox.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckboxStyle {
    /// Box background per state, unchecked.
    pub background: States<Color>,
    /// Box background per state, checked or mixed.
    pub background_checked: States<Color>,
    /// Box border per state, unchecked.
    pub border: States<Color>,
    /// Box border per state, checked or mixed.
    pub border_checked: States<Color>,
    /// Mark color.
    pub mark: Color,
    /// Mark color when disabled.
    pub mark_disabled: Color,
    /// Mark stroke width.
    pub mark_width: f32,
    /// Side of the box.
    pub box_size: f32,
    /// Corner radius of the box.
    pub corner_radius: f32,
    /// Border width of the box.
    pub border_width: f32,
    /// Label color.
    pub label: Color,
    /// Label color when disabled.
    pub label_disabled: Color,
    /// Description color.
    pub description: Color,
    /// Description color when disabled.
    pub description_disabled: Color,
    /// Space between the box and the label.
    pub gap: f32,
    /// Height of a line of label or description.
    pub line_height: f32,
}

impl CheckboxStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        let get = |token| tokens.get(token);
        let disabled_background = get(c::CHECKBOX_BACKGROUND_COLOR_DISABLED);
        let disabled_border = get(c::CHECKBOX_BORDER_COLOR_DISABLED);
        Self {
            background: States {
                normal: get(c::CHECKBOX_BACKGROUND_COLOR_DEFAULT),
                hovered: get(c::CHECKBOX_BACKGROUND_COLOR_HOVER),
                pressed: get(c::CHECKBOX_BACKGROUND_COLOR_PRESSED),
                selected: get(c::CHECKBOX_BACKGROUND_COLOR_DEFAULT),
                selected_hovered: get(c::CHECKBOX_BACKGROUND_COLOR_HOVER),
                disabled: disabled_background,
            },
            background_checked: States {
                normal: get(c::CHECKBOX_BACKGROUND_COLOR_SELECTED),
                hovered: get(c::CHECKBOX_BACKGROUND_COLOR_SELECTED_HOVER),
                pressed: get(c::CHECKBOX_BACKGROUND_COLOR_SELECTED_PRESSED),
                selected: get(c::CHECKBOX_BACKGROUND_COLOR_SELECTED),
                selected_hovered: get(c::CHECKBOX_BACKGROUND_COLOR_SELECTED_HOVER),
                disabled: disabled_background,
            },
            border: States {
                hovered: get(c::CHECKBOX_BORDER_COLOR_HOVER),
                selected_hovered: get(c::CHECKBOX_BORDER_COLOR_HOVER),
                disabled: disabled_border,
                ..States::all(get(c::CHECKBOX_BORDER_COLOR_DEFAULT))
            },
            border_checked: States {
                disabled: disabled_border,
                ..States::all(get(c::CHECKBOX_BORDER_COLOR_SELECTED))
            },
            mark: get(c::CHECKBOX_MARK_COLOR_DEFAULT),
            mark_disabled: get(c::CHECKBOX_MARK_COLOR_DISABLED),
            mark_width: tokens.get(c::CHECKBOX_MARK_WIDTH_DEFAULT),
            box_size: tokens.get(c::CHECKBOX_BOX_SIZE_DEFAULT),
            corner_radius: tokens.get(c::CHECKBOX_CORNER_RADIUS_DEFAULT),
            border_width: tokens.get(c::CHECKBOX_BORDER_WIDTH_DEFAULT),
            label: get(c::CHECKBOX_LABEL_COLOR_DEFAULT),
            label_disabled: get(c::CHECKBOX_LABEL_COLOR_DISABLED),
            description: get(c::CHECKBOX_DESCRIPTION_COLOR_DEFAULT),
            description_disabled: get(c::CHECKBOX_DESCRIPTION_COLOR_DISABLED),
            gap: tokens.get(c::CHECKBOX_GAP_DEFAULT),
            line_height: tokens.get(c::CHECKBOX_LINE_HEIGHT_DEFAULT),
        }
    }

    /// The box of a checkbox whose first line is `line`: at its left edge, centered
    /// vertically, on whole pixels.
    pub fn box_rect(&self, line: Rect) -> Rect {
        box_rect(self.box_size, line)
    }
}

/// A box of side `side` (at most the line's height and width) at the left edge of `line`,
/// centered vertically, on whole pixels.
fn box_rect(side: f32, line: Rect) -> Rect {
    let side = side.min(line.width()).min(line.height()).max(0.0);
    let min = Vec2::new(line.min.x, line.center().y - side * 0.5);
    snap(Rect::from_origin_size(min, Vec2::splat(side)))
}

/// Color overrides of a call: per part (background, border), unchecked or checked, per state.
type BoxColors = [[[Option<Prop<kind::Color>>; 6]; 2]; 2];

const BACKGROUND: usize = 0;
const BORDER: usize = 1;

/// A checkbox, configured then shown with [`Checkbox::show`]. Every property comes from the
/// checkbox tokens unless the call gives another token or a value.
#[derive(Debug)]
pub struct Checkbox<'a> {
    id: WidgetId,
    checked: &'a mut bool,
    label: Option<&'a str>,
    description: Option<&'a str>,
    mixed: bool,
    enabled: bool,
    box_size: Option<Prop<kind::Dimension>>,
    corner_radius: Option<Prop<kind::Dimension>>,
    border_width: Option<Prop<kind::Dimension>>,
    colors: BoxColors,
    mark: Option<Prop<kind::Color>>,
    label_color: Option<Prop<kind::Color>>,
}

impl<'a> Checkbox<'a> {
    /// A checkbox identified by `id` showing and flipping `checked`.
    pub fn new(id: WidgetId, checked: &'a mut bool) -> Self {
        Self {
            id,
            checked,
            label: None,
            description: None,
            mixed: false,
            enabled: true,
            box_size: None,
            corner_radius: None,
            border_width: None,
            colors: BoxColors::default(),
            mark: None,
            label_color: None,
        }
    }

    /// The label beside the box.
    #[must_use]
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// A description below the label, in a quieter color.
    #[must_use]
    pub fn description(mut self, description: &'a str) -> Self {
        self.description = Some(description);
        self
    }

    /// Shows the mixed mark: the options it stands for are partly checked. A click checks it.
    #[must_use]
    pub fn mixed(mut self, mixed: bool) -> Self {
        self.mixed = mixed;
        self
    }

    /// Makes the checkbox available or not.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The side of the box; the first line grows to hold a box taller than it.
    #[must_use]
    pub fn box_size(mut self, size: impl Into<Prop<kind::Dimension>>) -> Self {
        self.box_size = Some(size.into());
        self
    }

    /// The corner radius of the box.
    #[must_use]
    pub fn corner_radius(mut self, radius: impl Into<Prop<kind::Dimension>>) -> Self {
        self.corner_radius = Some(radius.into());
        self
    }

    /// The border width of the box.
    #[must_use]
    pub fn border_width(mut self, width: impl Into<Prop<kind::Dimension>>) -> Self {
        self.border_width = Some(width.into());
        self
    }

    /// The background of the unchecked box in `state`. A checkbox is normal, hovered,
    /// pressed or disabled; checked, it takes [`Checkbox::checked_background`].
    #[must_use]
    pub fn background(mut self, state: ControlState, color: impl Into<Prop<kind::Color>>) -> Self {
        self.colors[BACKGROUND][0][state.index()] = Some(color.into());
        self
    }

    /// The background of the checked (or mixed) box in `state`.
    #[must_use]
    pub fn checked_background(
        mut self,
        state: ControlState,
        color: impl Into<Prop<kind::Color>>,
    ) -> Self {
        self.colors[BACKGROUND][1][state.index()] = Some(color.into());
        self
    }

    /// The border color of the unchecked box in `state`.
    #[must_use]
    pub fn border_color(
        mut self,
        state: ControlState,
        color: impl Into<Prop<kind::Color>>,
    ) -> Self {
        self.colors[BORDER][0][state.index()] = Some(color.into());
        self
    }

    /// The border color of the checked (or mixed) box in `state`.
    #[must_use]
    pub fn checked_border_color(
        mut self,
        state: ControlState,
        color: impl Into<Prop<kind::Color>>,
    ) -> Self {
        self.colors[BORDER][1][state.index()] = Some(color.into());
        self
    }

    /// The color of the mark of an enabled checkbox.
    #[must_use]
    pub fn mark_color(mut self, color: impl Into<Prop<kind::Color>>) -> Self {
        self.mark = Some(color.into());
        self
    }

    /// The color of the label of an enabled checkbox.
    #[must_use]
    pub fn label_color(mut self, color: impl Into<Prop<kind::Color>>) -> Self {
        self.label_color = Some(color.into());
        self
    }

    /// Resolves a property given at the call, or takes the style's value.
    fn prop<K: Kind>(ui: &Ui<'_>, prop: Option<&Prop<K>>, fallback: K::Value) -> K::Value {
        prop.map_or(fallback, |prop| prop.resolve(&ui.styles().tokens))
    }

    /// The side of the box.
    fn side(&self, ui: &Ui<'_>) -> f32 {
        Self::prop(ui, self.box_size.as_ref(), ui.styles().checkbox.box_size)
    }

    /// The heights of the first line (box and label) and of the description's line: the
    /// line height, or more for a taller box or text.
    fn lines(&self, ui: &mut Ui<'_>) -> (f32, f32) {
        let styles = ui.styles();
        let line = styles.checkbox.line_height;
        let label = ui.line_height(&styles.text.label);
        let detail = ui.line_height(&styles.text.detail);
        let first = line.max(self.side(ui)).max(label).ceil();
        (first, line.max(detail).ceil())
    }

    /// The color of `part` of a box checked or not, in `state`: the call's, or the style's.
    fn box_color(&self, ui: &Ui<'_>, part: usize, checked: bool, state: ControlState) -> Color {
        let style = &ui.styles().checkbox;
        let fallback = match (part, checked) {
            (BACKGROUND, false) => style.background,
            (BACKGROUND, true) => style.background_checked,
            (_, false) => style.border,
            (_, true) => style.border_checked,
        }
        .get(state);
        Self::prop(
            ui,
            self.colors[part][usize::from(checked)][state.index()].as_ref(),
            fallback,
        )
    }

    /// The size the checkbox asks for: the box, the gap and the longest of the label and the
    /// description; one line high, two with a description.
    pub fn size(&self, ui: &mut Ui<'_>) -> Vec2 {
        let styles = ui.styles();
        let label = self
            .label
            .map_or(0.0, |label| ui.text_width(label, &styles.text.label));
        let description = self
            .description
            .map_or(0.0, |text| ui.text_width(text, &styles.text.detail));
        let text = label.max(description);
        let gap = if text > 0.0 {
            styles.checkbox.gap + text
        } else {
            0.0
        };
        let (first, second) = self.lines(ui);
        let height = first
            + if self.description.is_some() {
                second
            } else {
                0.0
            };
        Vec2::new((self.side(ui) + gap).ceil(), height)
    }

    /// Shows the checkbox in `rect`: a click anywhere in it flips the value (a mixed box
    /// becomes checked) and reports `changed`.
    pub fn show(self, ui: &mut Ui<'_>, rect: Rect) -> Response {
        let mut response = ui.interact(self.id, rect);
        if !self.enabled {
            response = Response::default();
        }
        if response.clicked {
            *self.checked = self.mixed || !*self.checked;
            response.changed = true;
        }
        let styles = ui.styles();
        let style = &styles.checkbox;
        let marked = *self.checked || (self.mixed && !response.changed);
        // Checked or not picks the colors; the state is the pointer's.
        let state = ControlState::of(&response, false, self.enabled);
        let (first, second) = self.lines(ui);
        let line = Rect::from_min_max(
            rect.min,
            Vec2::new(rect.max.x, rect.min.y + first.min(rect.height())),
        );
        let bounds = box_rect(self.side(ui), line);
        let radius = Self::prop(ui, self.corner_radius.as_ref(), style.corner_radius);
        let border = Self::prop(ui, self.border_width.as_ref(), style.border_width);
        let background = self.box_color(ui, BACKGROUND, marked, state);
        let border_color = self.box_color(ui, BORDER, marked, state);
        ui.scene().push_rect(
            &RectPrimitive::new(bounds, background)
                .with_corner_radius(radius)
                .with_border(border, border_color),
        );
        if marked {
            let mark = if self.enabled {
                Self::prop(ui, self.mark.as_ref(), style.mark)
            } else {
                style.mark_disabled
            };
            let icon = if self.mixed && !response.changed {
                Icon::Minus
            } else {
                Icon::Check
            };
            icon.draw(ui.scene(), bounds, mark, style.mark_width);
        }
        let text_left = bounds.max.x + style.gap;
        if let Some(label) = self.label {
            let color = if self.enabled {
                Self::prop(ui, self.label_color.as_ref(), style.label)
            } else {
                style.label_disabled
            };
            let slot = Rect::from_min_max(
                Vec2::new(text_left, line.min.y),
                Vec2::new(rect.max.x, line.max.y),
            );
            ui.label(label, slot, &styles.text.label, color, Align::Start);
        }
        if let Some(description) = self.description {
            let slot = Rect::from_min_max(
                Vec2::new(text_left, line.max.y),
                Vec2::new(rect.max.x, (line.max.y + second).min(rect.max.y)),
            );
            ui.label(
                description,
                slot,
                &styles.text.detail,
                if self.enabled {
                    style.description
                } else {
                    style.description_disabled
                },
                Align::Start,
            );
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;
    use koebi_text::Fonts;

    use super::*;
    use crate::{Input, UiState};

    fn rect() -> Rect {
        Rect::from_origin_size(Vec2::ZERO, Vec2::new(160.0, 40.0))
    }

    /// One frame of a checkbox over `rect()`; returns the response and the scene.
    fn frame(
        state: &mut UiState,
        input: &mut Input,
        checked: &mut bool,
        configure: impl FnOnce(Checkbox<'_>) -> Checkbox<'_>,
    ) -> (Response, Scene) {
        let styles = crate::test_styles();
        let mut scene = Scene::new();
        let response = {
            let mut ui = Ui::new(&mut scene, &styles, input, state);
            configure(Checkbox::new(WidgetId::new("snap"), checked)).show(&mut ui, rect())
        };
        input.end_frame();
        (response, scene)
    }

    #[test]
    fn a_click_on_the_label_flips_the_value() {
        let (mut state, mut input, mut checked) = (UiState::new(), Input::new(), false);
        let (_, scene) = frame(&mut state, &mut input, &mut checked, |c| c.label("Snap"));
        assert_eq!((scene.rect_count(), scene.line_count()), (1, 0));
        // Far from the box, on the label.
        input.primary_button(true, Vec2::new(120.0, 10.0));
        input.primary_button(false, Vec2::new(120.0, 10.0));
        let (response, scene) = frame(&mut state, &mut input, &mut checked, |c| c.label("Snap"));
        assert!(checked && response.changed);
        assert_eq!(scene.line_count(), 2);
    }

    #[test]
    fn mixed_boxes_show_a_line_and_check_on_click() {
        let (mut state, mut input, mut checked) = (UiState::new(), Input::new(), false);
        let (_, scene) = frame(&mut state, &mut input, &mut checked, |c| c.mixed(true));
        // The minus mark: one stroke.
        assert_eq!(scene.line_count(), 1);
        input.primary_button(true, Vec2::new(8.0, 10.0));
        input.primary_button(false, Vec2::new(8.0, 10.0));
        let (response, _) = frame(&mut state, &mut input, &mut checked, |c| c.mixed(true));
        assert!(checked && response.changed);
    }

    #[test]
    fn disabled_boxes_ignore_clicks() {
        let (mut state, mut input, mut checked) = (UiState::new(), Input::new(), false);
        frame(&mut state, &mut input, &mut checked, |c| c.enabled(false));
        input.primary_button(true, Vec2::new(8.0, 10.0));
        input.primary_button(false, Vec2::new(8.0, 10.0));
        let (response, _) = frame(&mut state, &mut input, &mut checked, |c| c.enabled(false));
        assert!(!checked && !response.changed && !response.clicked);
    }

    #[test]
    fn checkboxes_measure_their_box_and_text() {
        let styles = crate::test_styles();
        let style = &styles.checkbox;
        let (mut scene, mut state, input) = (Scene::new(), UiState::new(), Input::new());
        state.set_fonts(Fonts::with_embedded());
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        let mut checked = false;
        let label = ui.text_width("Snap", &styles.text.label);
        let size = Checkbox::new(WidgetId::new("snap"), &mut checked)
            .label("Snap")
            .size(&mut ui);
        assert_eq!(
            size,
            Vec2::new(
                (style.box_size + style.gap + label).ceil(),
                style.line_height
            )
        );
        let size = Checkbox::new(WidgetId::new("snap"), &mut checked)
            .label("Snap")
            .description("Snap moved items to the grid")
            .size(&mut ui);
        assert_eq!(size.y, 2.0 * style.line_height);
    }

    #[test]
    fn the_box_is_centered_on_the_first_line_on_whole_pixels() {
        let style = crate::test_styles().checkbox;
        let line = Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::new(100.0, 25.0));
        let bounds = style.box_rect(line);
        assert_eq!(bounds.size(), Vec2::splat(16.0));
        assert_eq!(bounds.min, Vec2::new(10.0, 15.0));
    }

    #[test]
    fn calls_override_the_box_with_tokens_or_values() {
        let styles = crate::test_styles();
        let (mut scene, mut state, input) = (Scene::new(), UiState::new(), Input::new());
        state.set_fonts(Fonts::with_embedded());
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        let mut checked = true;
        let red = Color::from_hex(0xE0_30_30);
        let big = Checkbox::new(WidgetId::new("big"), &mut checked)
            .box_size(32.0)
            .checked_background(ControlState::Normal, red)
            .mark_color(koebi_tokens::semantic::TEXT_COLOR_PRIMARY_DEFAULT);
        // The first line grows to hold the box.
        assert_eq!(big.size(&mut ui), Vec2::new(32.0, 32.0));
        assert_eq!(
            big.box_color(&ui, BACKGROUND, true, ControlState::Normal),
            red
        );
        // Unchecked colors and other states keep the style's.
        assert_eq!(
            big.box_color(&ui, BACKGROUND, false, ControlState::Normal),
            styles.checkbox.background.normal
        );
        assert_eq!(
            big.box_color(&ui, BACKGROUND, true, ControlState::Hovered),
            styles.checkbox.background_checked.hovered
        );
        let mark = Checkbox::prop(&ui, big.mark.as_ref(), styles.checkbox.mark);
        assert_eq!(
            mark,
            styles
                .tokens
                .get(koebi_tokens::semantic::TEXT_COLOR_PRIMARY_DEFAULT)
        );
    }
}
