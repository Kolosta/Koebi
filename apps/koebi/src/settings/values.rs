//! How the settings edit a token's value, by its kind: numbers in number fields bounded by the
//! token's range, colors in color fields opening their picker as a dropdown, curves in the curve
//! editor, cubic Béziers in four grouped fields, font families shown as they are (Koebi ships
//! one family).
//!
//! Every edit goes through [`DesignSystem::set_override`], which checks it against the token's
//! range: a value it refuses is logged and the token keeps its value.

use std::sync::Arc;

use koebi_core::{Rect, Vec2};
use koebi_design::kind::KindId;
use koebi_design::value::{CubicBezier, Curve, Definition, FontWeight, Value};
use koebi_design::{DesignSystem, Layers, Scope, TokenId};
use koebi_render::LinePrimitive;
use koebi_tokens::semantic;
use koebi_ui::{
    Align, BarMapping, ColorField, ColorFieldState, CurveEditor, CurveEditorState, FieldGroup,
    NumberField, Ui, WidgetId,
};

use super::options::OptionDesc;

/// How a token's value is edited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Shape {
    /// A number field: dimensions, numbers, integers, durations, font weights.
    Number,
    /// A color field.
    Color,
    /// Four grouped fields: the control points of a cubic Bézier.
    Bezier,
    /// The curve editor, in the option's panel.
    Curve,
    /// Shown, not edited: a font family.
    Text,
}

impl Shape {
    /// How `token` is edited.
    pub(super) fn of(design: &DesignSystem, token: TokenId) -> Self {
        match design
            .info(token)
            .map(koebi_design::schema::TokenInfo::kind)
        {
            Some(KindId::Color) => Self::Color,
            Some(KindId::CubicBezier) => Self::Bezier,
            Some(KindId::Curve) => Self::Curve,
            Some(KindId::FontFamily) => Self::Text,
            _ => Self::Number,
        }
    }
}

/// What the editors of one option keep between frames: the color fields (the header's, the
/// application's, the current theme's) and the curve editors (the application's, the theme's).
#[derive(Debug)]
pub(super) struct Editors {
    colors: [ColorFieldState; 3],
    curves: [CurveEditorState; 2],
}

impl Default for Editors {
    fn default() -> Self {
        Self {
            colors: core::array::from_fn(|_| ColorFieldState::new()),
            curves: [CurveEditorState::new(); 2],
        }
    }
}

/// Where an editor lies: the option's header, or one of the two columns of its panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Slot {
    /// The header: the application's value.
    Header,
    /// The panel's column of `scope`.
    Column(Scope),
}

impl Slot {
    /// The scope the slot edits.
    const fn scope(self) -> Scope {
        match self {
            Self::Header => Scope::Global,
            Self::Column(scope) => scope,
        }
    }

    /// Which of an option's color fields the slot uses.
    const fn color(self) -> usize {
        match self {
            Self::Header => 0,
            Self::Column(Scope::Global) => 1,
            Self::Column(_) => 2,
        }
    }

    /// Which of an option's curve editors the slot uses.
    const fn curve(self) -> usize {
        match self {
            Self::Column(Scope::Global) | Self::Header => 0,
            Self::Column(_) => 1,
        }
    }
}

/// The value of `token`, as far as `scope` sees it.
pub(super) fn value_in(design: &DesignSystem, token: TokenId, scope: Scope) -> Option<Value> {
    let layers = match scope {
        Scope::Global => Layers::ALL,
        Scope::ActiveTheme | Scope::Theme(_) => Layers::THEME.union(Layers::THEME_OVERRIDES),
    };
    design.view(design.active_theme(), layers)?.value(token)
}

/// The value of `token` in the current theme, without anyone's overrides: what pickers offer
/// as the original.
fn original(design: &DesignSystem, token: TokenId) -> Option<Value> {
    design
        .view(design.active_theme(), Layers::THEME)?
        .value(token)
}

/// A value as a number: pixels, milliseconds, a plain number, a weight.
pub(super) fn number_of(value: &Value) -> Option<f64> {
    match value {
        Value::Dimension(number) | Value::Number(number) => Some(f64::from(*number)),
        Value::Duration(duration) => Some(duration.as_secs_f64() * 1000.0),
        Value::Integer(number) => Some(f64::from(*number)),
        Value::FontWeight(weight) => Some(f64::from(weight.get())),
        _ => None,
    }
}

/// `number` as a value of the kind of `like`.
pub(super) fn value_like(like: &Value, number: f64) -> Option<Value> {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "token values, which their ranges keep inside f32, i32 and u16"
    )]
    match like {
        Value::Dimension(_) => Some(Value::Dimension(number as f32)),
        Value::Number(_) => Some(Value::Number(number as f32)),
        Value::Duration(_) => Some(Value::Duration(core::time::Duration::from_secs_f64(
            number.max(0.0) / 1000.0,
        ))),
        Value::Integer(_) => Some(Value::Integer(number.round() as i32)),
        Value::FontWeight(_) => {
            FontWeight::new(number.round().clamp(1.0, 1000.0) as u16).map(Value::FontWeight)
        }
        _ => None,
    }
}

/// Gives `token` the value `value` for `scope`, logging a refusal (outside its range).
fn set(design: &mut DesignSystem, scope: Scope, token: TokenId, value: Value, label: &str) {
    if let Err(error) = design.set_override(scope, token, Definition::Value(value)) {
        koebi_core::warn!("cannot change {label}: {error}");
    }
}

/// The height of the editor of a value of `shape` in an option's panel.
pub(super) fn column_height(ui: &Ui<'_>, design: &DesignSystem, shape: Shape) -> f32 {
    match shape {
        Shape::Curve => CurveEditor::height(ui),
        _ => design.get(semantic::SIZE_CONTROL_HEIGHT),
    }
}

/// Shows the editor of `desc`'s value for `slot` in `rect`, its menus within `bounds`, and
/// applies what it changes. `text` is reused for what shows as text.
#[allow(clippy::too_many_arguments, reason = "the parts of one editor")]
pub(super) fn editor(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    design: &mut DesignSystem,
    desc: OptionDesc,
    slot: Slot,
    editors: &mut Editors,
    text: &mut String,
    bounds: Rect,
) {
    let scope = slot.scope();
    let Some(value) = value_in(design, desc.token, scope) else {
        return;
    };
    match (&value, Shape::of(design, desc.token)) {
        (_, Shape::Number) => number(ui, rect, id, design, desc, scope, &value),
        (Value::Color(color), Shape::Color) => {
            let mut color = *color;
            let base = original(design, desc.token)
                .and_then(|value| match value {
                    Value::Color(color) => Some(color),
                    _ => None,
                })
                .unwrap_or(color);
            let changed = ColorField::new(id, &mut color, &mut editors.colors[slot.color()])
                .dropdown(true)
                .original(base)
                .title(desc.label)
                .show(ui, rect, bounds)
                .changed;
            if changed {
                // The token's range bounds the opacity (opaque tokens refuse any other).
                let alpha = design
                    .range(desc.token)
                    .map_or(f64::from(color.a), |range| range.limit(f64::from(color.a)));
                #[allow(clippy::cast_possible_truncation, reason = "an opacity")]
                let color = color.with_alpha(alpha as f32);
                set(design, scope, desc.token, Value::Color(color), desc.label);
            }
        }
        (Value::CubicBezier(easing), Shape::Bezier) => {
            if let Some(next) = bezier(ui, rect, id, *easing, slot == Slot::Header, text) {
                set(
                    design,
                    scope,
                    desc.token,
                    Value::CubicBezier(next),
                    desc.label,
                );
            }
        }
        (Value::Curve(curve), Shape::Curve) => {
            if slot == Slot::Header {
                curve_preview(ui, rect, curve);
                return;
            }
            let mut edited: Curve = **curve;
            let state = &mut editors.curves[slot.curve()];
            if CurveEditor::new(id, &mut edited, state).show(ui, rect) {
                set(
                    design,
                    scope,
                    desc.token,
                    Value::Curve(Arc::new(edited)),
                    desc.label,
                );
            }
        }
        (Value::FontFamily(family), _) => {
            // Koebi ships one family: there is nothing to choose yet.
            let font = ui.styles().text.label.clone();
            let color = design.get(semantic::TEXT_COLOR_SECONDARY_DEFAULT);
            ui.label(family, rect, &font, color, Align::End);
        }
        _ => {}
    }
}

/// A number field of `value`, bounded by the token's range, on a bar when the range is closed.
fn number(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    design: &mut DesignSystem,
    desc: OptionDesc,
    scope: Scope,
    value: &Value,
) {
    let Some(mut number) = number_of(value) else {
        return;
    };
    let mut field = NumberField::new(id, &mut number).decimals(desc.decimals);
    if let Some(unit) = desc.unit {
        field = field.unit(unit);
    }
    if let Some(range) = design.range(desc.token) {
        field = field.range(range.min(), range.max());
        if range.min().is_finite() && range.max().is_finite() {
            field = field.bar(BarMapping::Linear);
        }
        if range.step() > 0.0 {
            field = field.step(range.step());
        }
    }
    if field.show(ui, rect).changed
        && let Some(next) = value_like(value, number)
    {
        set(design, scope, desc.token, next, desc.label);
    }
}

/// The four control points of `easing` in grouped fields; in a header, its text. Returns the
/// easing typed, when valid and changed.
fn bezier(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    easing: CubicBezier,
    header: bool,
    text: &mut String,
) -> Option<CubicBezier> {
    if header {
        let font = ui.styles().text.detail.clone();
        let color = ui.styles().text.secondary;
        text.clear();
        // Writing into a `String` cannot fail.
        let _infallible = core::fmt::Write::write_fmt(text, format_args!("{easing}"));
        ui.label(text, rect, &font, color, Align::End);
        return None;
    }
    let style = &ui.styles().number_field;
    let group = FieldGroup::horizontal(rect, 4, style.corner_radius, style.border_width);
    let mut values = [easing.x1, easing.y1, easing.x2, easing.y2];
    let labels = ["X1", "Y1", "X2", "Y2"];
    let mut changed = false;
    for (part, value) in values.iter_mut().enumerate() {
        let (cell, corners) = group.cell(part);
        // The control points' x stay within the segment; their y may overshoot.
        let (low, high) = if part % 2 == 0 {
            (0.0, 1.0)
        } else {
            (-Curve::LIMIT, Curve::LIMIT)
        };
        changed |= NumberField::new(id.with(part as u64), value)
            .label(labels[part])
            .range(low, high)
            .decimals(2)
            .corner_radii(corners)
            .step_buttons(false)
            .show(ui, cell)
            .changed;
    }
    let [x1, y1, x2, y2] = values;
    let next = CubicBezier::new(x1, y1, x2, y2);
    (changed && next.is_valid() && next != easing).then_some(next)
}

/// A small picture of `curve` in `rect`: what a header shows of a curve.
fn curve_preview(ui: &mut Ui<'_>, rect: Rect, curve: &Curve) {
    const SAMPLES: usize = 24;
    let style = ui.styles().curve_editor;
    let inset = (rect.height() * 0.2).round();
    let area = Rect::from_min_max(
        Vec2::new(rect.max.x - 3.0 * rect.height(), rect.min.y + inset),
        Vec2::new(rect.max.x - inset, rect.max.y - inset),
    );
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for key in curve.keys() {
        low = low.min(key.y);
        high = high.max(key.y);
    }
    let point = |t: f32| {
        let y = (curve.ease(t) - low) / (high - low).max(1e-3);
        Vec2::new(
            area.min.x + t * area.width(),
            area.max.y - y * area.height(),
        )
    };
    let mut previous = point(0.0);
    for step in 1..=SAMPLES {
        let next = point(step as f32 / SAMPLES as f32);
        ui.scene()
            .push_line(&LinePrimitive::new(previous, next, 1.5, style.curve));
        previous = next;
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use koebi_core::Color;

    use super::*;

    #[test]
    fn values_turn_into_numbers_and_back_by_kind() {
        let cases = [
            (Value::Dimension(4.0), 4.0),
            (Value::Number(0.5), 0.5),
            (Value::Duration(Duration::from_millis(350)), 350.0),
            (Value::Integer(3), 3.0),
            (Value::FontWeight(FontWeight::BOLD), 700.0),
        ];
        for (value, number) in cases {
            assert!((number_of(&value).unwrap() - number).abs() < 1e-3);
            assert_eq!(value_like(&value, number), Some(value));
        }
        assert_eq!(number_of(&Value::Color(Color::WHITE)), None);
        assert_eq!(value_like(&Value::Color(Color::WHITE), 1.0), None);
    }

    #[test]
    fn tokens_are_edited_by_their_kind() {
        let design = DesignSystem::new(&koebi_tokens::KOEBI).unwrap();
        let shape = |token: TokenId| Shape::of(&design, token);
        assert_eq!(
            shape(semantic::BACKGROUND_ACCENT_DEFAULT.id()),
            Shape::Color
        );
        assert_eq!(
            shape(koebi_tokens::component::PANEL_EXPAND_CURVE.id()),
            Shape::Curve
        );
        assert_eq!(shape(semantic::FONT_FAMILY_BODY.id()), Shape::Text);
        assert_eq!(shape(semantic::SIZE_CONTROL_HEIGHT.id()), Shape::Number);
    }

    #[test]
    fn values_a_token_refuses_change_nothing() {
        let mut design = DesignSystem::new(&koebi_tokens::KOEBI).unwrap();
        let token = koebi_tokens::component::CHECKBOX_BOX_SIZE_DEFAULT;
        let before = design.get(token);
        // Zero would hide the box: refused, logged, the token keeps its value.
        set(
            &mut design,
            Scope::Global,
            token.id(),
            Value::Dimension(0.0),
            "Box size",
        );
        assert!(design.override_of(Scope::Global, token).is_none());
        assert!((design.get(token) - before).abs() < f32::EPSILON);
        // Inside the range: taken.
        set(
            &mut design,
            Scope::Global,
            token.id(),
            Value::Dimension(20.0),
            "Box size",
        );
        assert!((design.get(token) - 20.0).abs() < f32::EPSILON);
    }
}
