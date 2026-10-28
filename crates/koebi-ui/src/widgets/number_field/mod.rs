//! Number fields: Blender's number button, rebuilt.
//!
//! - **Showing**: one control high, the value rounded to its decimals with its unit, centred
//!   (or after its label); a bounded field can fill a bar to where its value lies
//!   ([`BarMapping`]: linear, logarithmic or the caller's). A vertical variant stands up.
//! - **Stepping**: on hover, step buttons at both ends; Shift steps finer, Ctrl to the next
//!   round step, Alt by the value's last decimal ([`math`]).
//! - **Dragging** ([`interact`]): a press locks the pointer at once. Released before the drag
//!   threshold, it types the value; past it, travel changes the value at the speed of the
//!   system's pointer: a step per `number-field.drag.step-distance`, or the bar's length for
//!   the whole range of a bounded field. Left Shift makes the whole drag finer from its start,
//!   right Shift finer from the current value, Ctrl rounds to steps, Alt keeps the start's last
//!   decimal. Escape, Delete or a right click cancel at any time; Enter or the release keep
//!   the value. The pointer comes back where the press was, or at the end of a bounded field's
//!   bar.
//! - **Typing**: the value as text in a [`TextField`](crate::TextField), evaluated as an
//!   expression with units ([`koebi_core::expr`]) in the field's unit; an invalid expression
//!   keeps the edit open, framed as an error.
//! - **Numbers** ([`Numeric`]): floats, or integers without decimals and with whole steps.

mod draw;
mod format;
mod interact;
mod math;
mod state;
mod style;

#[cfg(test)]
mod tests;

pub use math::BarMapping;
pub use style::NumberFieldStyle;

pub(crate) use state::NumberState;

use koebi_core::expr::Unit;
use koebi_core::input::Key;
use koebi_core::{Rect, Vec2};
use koebi_design::kind::{self, Kind};
use koebi_input::{ActionDef, ActionGroup, Context, Shortcut};

use self::math::Scale;
use crate::id::WidgetId;
use crate::style::{CornerRadii, Prop};
use crate::ui::{Response, Ui};
use crate::widgets::parts::ControlState;
use crate::widgets::text_field::EditEnd;

/// A number field being dragged: its keys apply wherever the pointer is.
pub const NUMBER: Context = Context::new("number");

/// Cancels the drag of a number field, restoring the value it started from.
pub static NUMBER_CANCEL: ActionDef = ActionDef {
    id: "number.cancel",
    label: "Cancel drag",
    description: "Cancels the drag of a number field and restores its value.",
    context: NUMBER,
    defaults: &[Shortcut::key(Key::Escape), Shortcut::key(Key::Delete)],
};

/// Ends the drag of a number field, keeping the value.
pub static NUMBER_CONFIRM: ActionDef = ActionDef {
    id: "number.confirm",
    label: "Confirm drag",
    description: "Ends the drag of a number field and keeps its value.",
    context: NUMBER,
    defaults: &[Shortcut::key(Key::Enter), Shortcut::key(Key::KeypadEnter)],
};

/// The actions of number fields, for the keymap.
pub static NUMBER_ACTIONS: ActionGroup = ActionGroup {
    name: "Number fields",
    actions: &[&NUMBER_CANCEL, &NUMBER_CONFIRM],
};

/// A number a [`NumberField`] edits: floats, or integers that show no decimals and step by
/// whole numbers. Fields compute in `f64`: integers beyond 2⁵³ lose their last digits.
pub trait Numeric: Copy + PartialEq {
    /// `true` for integers.
    const INTEGER: bool;

    /// The number as a float.
    fn to_f64(self) -> f64;

    /// The number nearest to `value`: integers saturate at their bounds and give 0 for NaN;
    /// floats keep NaN and infinities.
    fn from_f64(value: f64) -> Self;
}

impl Numeric for f64 {
    const INTEGER: bool = false;

    fn to_f64(self) -> f64 {
        self
    }

    fn from_f64(value: f64) -> Self {
        value
    }
}

impl Numeric for f32 {
    const INTEGER: bool = false;

    fn to_f64(self) -> f64 {
        f64::from(self)
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "a field of f32 keeps the precision of f32, within its range"
    )]
    fn from_f64(value: f64) -> Self {
        let max = f64::from(f32::MAX);
        if value.is_finite() {
            value.max(-max).min(max) as f32
        } else {
            value as f32
        }
    }
}

macro_rules! numeric_integer {
    ($($integer:ty),*) => {
        $(
            impl Numeric for $integer {
                const INTEGER: bool = true;

                #[allow(
                    clippy::cast_precision_loss,
                    clippy::cast_lossless,
                    reason = "fields edit integers below 2^53, which floats hold exactly"
                )]
                fn to_f64(self) -> f64 {
                    self as f64
                }

                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "float to integer casts saturate, and NaN gives 0"
                )]
                fn from_f64(value: f64) -> Self {
                    value.round() as $integer
                }
            }
        )*
    };
}

numeric_integer!(i32, i64, u8, u16, u32, u64, usize);

/// What happened to a number field this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NumberFieldResponse {
    /// The pointer's interaction with the field's body.
    pub response: Response,
    /// The value changed this frame (dragged, stepped, typed, or restored by a cancel).
    pub changed: bool,
    /// The field is being dragged past the threshold.
    pub dragging: bool,
    /// The value is being typed.
    pub editing: bool,
    /// How a drag or an edit ended this frame: kept, or cancelled and restored.
    pub ended: Option<EditEnd>,
}

impl NumberFieldResponse {
    /// `true` when a drag or an edit ended keeping the value.
    pub fn confirmed(&self) -> bool {
        self.ended == Some(EditEnd::Confirmed)
    }

    /// `true` when a drag or an edit ended restoring the value it started from.
    pub fn cancelled(&self) -> bool {
        self.ended == Some(EditEnd::Cancelled)
    }
}

/// The parts whose colors the call can set per state.
type FieldColors = [[Option<Prop<kind::Color>>; 6]; 3];

const BACKGROUND: usize = 0;
const BORDER: usize = 1;
const FILL: usize = 2;

/// The widget key of the text field that types the value.
const EDIT_KEY: u64 = u64::MAX;

/// A number field, configured then shown with [`NumberField::show`]. Every property comes
/// from the number field tokens (`component.number-field.*`) unless the call gives another
/// token or a value.
#[derive(Debug)]
pub struct NumberField<'a, N: Numeric> {
    id: WidgetId,
    value: &'a mut N,
    min: f64,
    max: f64,
    step: Option<f64>,
    decimals: Option<usize>,
    unit: Option<&'a Unit>,
    label: Option<&'a str>,
    bar: Option<BarMapping>,
    vertical: bool,
    enabled: bool,
    step_buttons: bool,
    height: Option<Prop<kind::Dimension>>,
    min_width: Option<Prop<kind::Dimension>>,
    radii: Option<CornerRadii>,
    border_width: Option<Prop<kind::Dimension>>,
    colors: FieldColors,
}

impl<'a, N: Numeric> NumberField<'a, N> {
    /// A field identified by `id` showing and changing `value`, unbounded.
    pub fn new(id: WidgetId, value: &'a mut N) -> Self {
        Self {
            id,
            value,
            min: f64::NEG_INFINITY,
            max: f64::INFINITY,
            step: None,
            decimals: None,
            unit: None,
            label: None,
            bar: None,
            vertical: false,
            enabled: true,
            step_buttons: true,
            height: None,
            min_width: None,
            radii: None,
            border_width: None,
            colors: FieldColors::default(),
        }
    }

    /// The field's identity.
    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Keeps the value from `min` to `max`.
    #[must_use]
    pub fn range(mut self, min: N, max: N) -> Self {
        self.min = min.to_f64();
        self.max = max.to_f64();
        self
    }

    /// Keeps the value at or above `min`.
    #[must_use]
    pub fn min(mut self, min: N) -> Self {
        self.min = min.to_f64();
        self
    }

    /// Keeps the value at or below `max`.
    #[must_use]
    pub fn max(mut self, max: N) -> Self {
        self.max = max.to_f64();
        self
    }

    /// What a step button adds and round steps are multiples of, and what an unbounded drag
    /// adds per `number-field.drag.step-distance`: 1 for integers (whole steps, at least 1),
    /// 0.1 for floats by default.
    #[must_use]
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    /// Decimals shown: none for integers, 2 for floats by default.
    #[must_use]
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = Some(decimals);
        self
    }

    /// The unit the value is in: shown after it, and the unit typed expressions convert to.
    #[must_use]
    pub fn unit(mut self, unit: &'a Unit) -> Self {
        self.unit = Some(unit);
        self
    }

    /// Shows `label` at the start, the value at the end.
    #[must_use]
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Fills a bar to where the value lies in its range, mapped by `mapping`; a bounded drag
    /// then runs the whole range along the bar. Only bounded fields show one.
    #[must_use]
    pub fn bar(mut self, mapping: BarMapping) -> Self {
        self.bar = Some(mapping);
        self
    }

    /// Stands the field up: dragging up increases the value, the step buttons are at the
    /// top and bottom, the bar fills from the bottom.
    #[must_use]
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    /// Makes the field available or not.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows step buttons on hover (the default) or not.
    #[must_use]
    pub fn step_buttons(mut self, shown: bool) -> Self {
        self.step_buttons = shown;
        self
    }

    /// The height (the width of a vertical field) [`NumberField::size`] measures.
    #[must_use]
    pub fn height(mut self, height: impl Into<Prop<kind::Dimension>>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// The width (the height of a vertical field) [`NumberField::size`] measures.
    #[must_use]
    pub fn min_width(mut self, width: impl Into<Prop<kind::Dimension>>) -> Self {
        self.min_width = Some(width.into());
        self
    }

    /// The same radius for every corner.
    #[must_use]
    pub fn corner_radius(mut self, radius: impl Into<Prop<kind::Dimension>>) -> Self {
        self.radii = Some(CornerRadii::all(radius));
        self
    }

    /// A radius per corner.
    #[must_use]
    pub fn corner_radii(mut self, radii: impl Into<CornerRadii>) -> Self {
        self.radii = Some(radii.into());
        self
    }

    /// The border width.
    #[must_use]
    pub fn border_width(mut self, width: impl Into<Prop<kind::Dimension>>) -> Self {
        self.border_width = Some(width.into());
        self
    }

    /// The background in `state` (pressed: a press that has not moved; selected: a drag).
    #[must_use]
    pub fn background(mut self, state: ControlState, color: impl Into<Prop<kind::Color>>) -> Self {
        self.colors[BACKGROUND][state.index()] = Some(color.into());
        self
    }

    /// The border color in `state` (selected: a drag).
    #[must_use]
    pub fn border_color(
        mut self,
        state: ControlState,
        color: impl Into<Prop<kind::Color>>,
    ) -> Self {
        self.colors[BORDER][state.index()] = Some(color.into());
        self
    }

    /// The bar color in `state`.
    #[must_use]
    pub fn fill_color(mut self, state: ControlState, color: impl Into<Prop<kind::Color>>) -> Self {
        self.colors[FILL][state.index()] = Some(color.into());
        self
    }

    /// Resolves a property given at the call, or takes the style's value.
    fn prop<K: Kind>(ui: &Ui<'_>, prop: Option<&Prop<K>>, fallback: K::Value) -> K::Value {
        prop.map_or(fallback, |prop| prop.resolve(&ui.styles().tokens))
    }

    /// The size the field asks for: its smallest width and one control high (turned for a
    /// vertical field), or a line of text if taller.
    pub fn size(&self, ui: &mut Ui<'_>) -> Vec2 {
        let style = &ui.styles().number_field;
        let width = Self::prop(ui, self.min_width.as_ref(), style.min_width);
        let line = ui.line_height(&ui.styles().text.body);
        let height = Self::prop(ui, self.height.as_ref(), style.height).max(line);
        if self.vertical {
            Vec2::new(height.ceil(), width.ceil())
        } else {
            Vec2::new(width.ceil(), height.ceil())
        }
    }

    /// The step: the call's, or 1 for integers and 0.1 for floats; whole and at least 1 for
    /// integers.
    fn step_value(&self) -> f64 {
        let step = self
            .step
            .filter(|step| step.is_finite() && *step > 0.0)
            .unwrap_or(if N::INTEGER { 1.0 } else { 0.1 });
        if N::INTEGER {
            step.round().max(1.0)
        } else {
            step
        }
    }

    /// The decimals shown: none for integers, the call's or 2 for floats.
    fn decimals_shown(&self) -> usize {
        if N::INTEGER {
            0
        } else {
            self.decimals.unwrap_or(2).min(12)
        }
    }

    /// `true` when both bounds are finite: the field can show a bar.
    fn is_bounded(&self) -> bool {
        self.min.is_finite() && self.max.is_finite() && self.max > self.min
    }

    /// The bar's mapping, when the field shows one.
    fn shown_bar(&self) -> Option<BarMapping> {
        self.bar.filter(|_| self.is_bounded())
    }

    /// The track of the bar: inside the border, across the whole field. The bar is drawn on
    /// it, a bounded drag runs the range along it, and the pointer comes back on it.
    fn track(&self, ui: &Ui<'_>, rect: Rect) -> Rect {
        let style = &ui.styles().number_field;
        rect.inset(Self::prop(
            ui,
            self.border_width.as_ref(),
            style.border_width,
        ))
    }

    /// How travel along `track` turns into values.
    fn scale(&self, ui: &Ui<'_>, track: Rect) -> Scale {
        let style = &ui.styles().number_field;
        let length = if self.vertical {
            track.height()
        } else {
            track.width()
        };
        Scale {
            step: self.step_value(),
            step_distance: style.step_distance,
            fine: f64::from(style.fine_factor),
            decimals: self.decimals_shown(),
            min: self.min,
            max: self.max,
            bar: self.shown_bar().map(|mapping| (mapping, length)),
            min_step: if N::INTEGER { 1.0 } else { 0.0 },
        }
    }

    /// `value` within the bounds, as the field's number type.
    fn bounded(&self, value: f64) -> N {
        N::from_f64(value.max(self.min).min(self.max))
    }

    /// The draggable body and the step buttons at both ends (left then right, or bottom then
    /// top), when the field is long enough for them.
    fn parts(&self, ui: &Ui<'_>, rect: Rect) -> (Rect, Option<[Rect; 2]>) {
        let side = ui.styles().number_field.step_width;
        let length = if self.vertical {
            rect.height()
        } else {
            rect.width()
        };
        if !self.step_buttons || side <= 0.0 || length < 3.0 * side {
            return (rect, None);
        }
        if self.vertical {
            let bottom = Rect::from_min_max(Vec2::new(rect.min.x, rect.max.y - side), rect.max);
            let top = Rect::from_min_max(rect.min, Vec2::new(rect.max.x, rect.min.y + side));
            let body = Rect::from_min_max(
                Vec2::new(rect.min.x, top.max.y),
                Vec2::new(rect.max.x, bottom.min.y),
            );
            (body, Some([bottom, top]))
        } else {
            let left = Rect::from_min_max(rect.min, Vec2::new(rect.min.x + side, rect.max.y));
            let right = Rect::from_min_max(Vec2::new(rect.max.x - side, rect.min.y), rect.max);
            let body = Rect::from_min_max(
                Vec2::new(left.max.x, rect.min.y),
                Vec2::new(right.min.x, rect.max.y),
            );
            (body, Some([left, right]))
        }
    }

    /// Shows the field in `rect`: dragged, stepped or typed when it should be.
    pub fn show(self, ui: &mut Ui<'_>, rect: Rect) -> NumberFieldResponse {
        let number = &mut ui.state.number;
        let typed = number.edit.as_ref().is_some_and(|edit| edit.id == self.id);
        if typed && self.enabled {
            return self.show_edit(ui, rect);
        }
        // A field disabled while its value is typed drops what was typed.
        let dropped = typed && number.edit.take().is_some();
        let mut out = self.show_value(ui, rect);
        if dropped {
            out.ended = Some(EditEnd::Cancelled);
        }
        out
    }
}
