//! [`CurveEditor`]: a simple editor of easing curves ([`Curve`]).
//!
//! The curve shows over a grid. Its keys are pressed to select them and dragged (Escape brings a
//! dragged key back), added by a double click on the graph or the "Add key" button, and
//! removed by the secondary button or the "Remove key" button. Below the graph, the selected
//! key's place and the easing of the segment leaving it are typed in grouped fields.
//!
//! **Temporary**: curves get a fuller editor later (handles on the graph, presets, snapping,
//! zoom). This one keeps to what curve tokens hold: the first key at `(0, 0)` and the last at
//! `(1, 1)`, keys following each other in time, values within [`Curve::LIMIT`], at most
//! [`Curve::MAX_KEYS`] keys. An edit breaking a rule is refused; the curve is never invalid.

use koebi_core::input::CursorShape;
use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_design::value::{CubicBezier, Curve, CurveKey};
use koebi_i18n::n_;
use koebi_i18n::tr;
use koebi_render::{LinePrimitive, RectPrimitive};
use koebi_tokens::component;

use crate::id::WidgetId;
use crate::operation::OperationEnd;
use crate::ui::Ui;
use crate::widgets::button::{Button, ButtonGroup, GroupCell};
use crate::widgets::field_group::FieldGroup;
use crate::widgets::number_field::NumberField;

/// What the easing's fields set: the control points of the segment leaving the selected key.
const EASING_TIPS: [&str; 4] = [
    n_!("The easing's first control point: its time"),
    n_!("The easing's first control point: its value"),
    n_!("The easing's second control point: its time"),
    n_!("The easing's second control point: its value"),
];

/// How many segments draw the curve.
const SAMPLES: usize = 48;

/// The smallest distance in time between two keys.
const MIN_GAP: f32 = 1e-3;

/// Colors and metrics of curve editors, from the `component.curve-editor.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveEditorStyle {
    /// Height of the graph.
    pub graph_height: f32,
    /// Space between the graph's frame and the square the curve spans.
    pub padding: f32,
    /// Background of the graph.
    pub background: Color,
    /// Border of the graph.
    pub border: Color,
    /// Width of that border.
    pub border_width: f32,
    /// Corner radius of the graph.
    pub corner_radius: f32,
    /// Lines of the grid.
    pub grid: Color,
    /// Color of the curve.
    pub curve: Color,
    /// Width of the curve.
    pub curve_width: f32,
    /// Side of a key.
    pub key_size: f32,
    /// Side of the selected key, and of a key under the pointer.
    pub key_size_selected: f32,
    /// Fill of a key.
    pub key: Color,
    /// Fill of the selected key.
    pub key_selected: Color,
    /// Fill of the keys that cannot move: the first and the last.
    pub key_fixed: Color,
    /// Outline of the keys.
    pub key_outline: Color,
    /// Width of that outline.
    pub key_outline_width: f32,
    /// How far from a key a press takes it.
    pub pick_distance: f32,
}

impl CurveEditorStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        Self {
            graph_height: tokens.get(c::CURVE_EDITOR_GRAPH_HEIGHT_DEFAULT),
            padding: tokens.get(c::CURVE_EDITOR_PADDING_DEFAULT),
            background: tokens.get(c::CURVE_EDITOR_BACKGROUND_COLOR_DEFAULT),
            border: tokens.get(c::CURVE_EDITOR_BORDER_COLOR_DEFAULT),
            border_width: tokens.get(c::CURVE_EDITOR_BORDER_WIDTH_DEFAULT),
            corner_radius: tokens.get(c::CURVE_EDITOR_CORNER_RADIUS_DEFAULT),
            grid: tokens.get(c::CURVE_EDITOR_GRID_COLOR_DEFAULT),
            curve: tokens.get(c::CURVE_EDITOR_CURVE_COLOR_DEFAULT),
            curve_width: tokens.get(c::CURVE_EDITOR_CURVE_WIDTH_DEFAULT),
            key_size: tokens.get(c::CURVE_EDITOR_KEY_SIZE_DEFAULT),
            key_size_selected: tokens.get(c::CURVE_EDITOR_KEY_SIZE_SELECTED),
            key: tokens.get(c::CURVE_EDITOR_KEY_COLOR_DEFAULT),
            key_selected: tokens.get(c::CURVE_EDITOR_KEY_COLOR_SELECTED),
            key_fixed: tokens.get(c::CURVE_EDITOR_KEY_COLOR_FIXED),
            key_outline: tokens.get(c::CURVE_EDITOR_KEY_OUTLINE_COLOR_DEFAULT),
            key_outline_width: tokens.get(c::CURVE_EDITOR_KEY_OUTLINE_WIDTH_DEFAULT),
            pick_distance: tokens.get(c::CURVE_EDITOR_PICK_DISTANCE_DEFAULT),
        }
    }
}

/// A drag of a key: which one, where it was (Escape brings it back), and the values the graph
/// showed when it started, kept while it lasts so that the graph never moves under the pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Drag {
    index: usize,
    from: CurveKey,
    range: (f32, f32),
}

/// What a curve editor keeps between frames: the key selected and the drag in progress.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CurveEditorState {
    selected: Option<usize>,
    drag: Option<Drag>,
}

impl CurveEditorState {
    /// Nothing selected.
    pub const fn new() -> Self {
        Self {
            selected: None,
            drag: None,
        }
    }

    /// The index of the key selected.
    pub const fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Selects the key at `index`, or none.
    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
    }
}

/// A curve editor of `curve`, configured then shown with [`CurveEditor::show`].
#[derive(Debug)]
pub struct CurveEditor<'a> {
    id: WidgetId,
    curve: &'a mut Curve,
    state: &'a mut CurveEditorState,
}

impl<'a> CurveEditor<'a> {
    /// An editor of `curve`, identified by `id`, its selection kept in `state`.
    pub fn new(id: WidgetId, curve: &'a mut Curve, state: &'a mut CurveEditorState) -> Self {
        Self { id, curve, state }
    }

    /// The height the editor takes: the graph, then two rows of fields.
    pub fn height(ui: &Ui<'_>) -> f32 {
        let (row, gap) = rows(ui);
        ui.styles().curve_editor.graph_height + 2.0 * (row + gap)
    }

    /// Shows the editor in `rect`, the graph above and the fields below, as wide as `rect`.
    /// Returns whether the curve changed.
    pub fn show(self, ui: &mut Ui<'_>, rect: Rect) -> bool {
        let Self { id, curve, state } = self;
        let (row, gap) = rows(ui);
        let graph = Rect::from_min_max(
            rect.min,
            Vec2::new(rect.max.x, (rect.max.y - 2.0 * (row + gap)).max(rect.min.y)),
        );
        let mut keys = Keys::of(curve);
        if state.selected.is_some_and(|index| index >= keys.len) {
            state.selected = None;
        }
        let mut changed = show_graph(ui, id, graph, curve, &mut keys, state);
        let first = Rect::from_min_max(
            Vec2::new(rect.min.x, graph.max.y + gap),
            Vec2::new(rect.max.x, graph.max.y + gap + row),
        );
        changed |= key_fields(ui, id, first, curve, &mut keys, state);
        let second = Rect::from_min_max(
            Vec2::new(rect.min.x, first.max.y + gap),
            Vec2::new(rect.max.x, first.max.y + gap + row),
        );
        changed |= easing_fields(ui, id, second, &mut keys, state);
        if !changed {
            return false;
        }
        let next = Curve::new(keys.as_slice());
        if !next.is_valid() || next == *curve {
            return false;
        }
        *curve = next;
        true
    }
}

/// The height of a row of fields and the space between rows.
fn rows(ui: &Ui<'_>) -> (f32, f32) {
    (
        ui.styles().number_field.height,
        ui.styles().property_group.row_gap,
    )
}

/// The keys of a curve being edited, without allocating: a copy of its keys.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Keys {
    keys: [CurveKey; Curve::MAX_KEYS],
    len: usize,
}

impl Keys {
    fn of(curve: &Curve) -> Self {
        let mut keys = [CurveKey::new(1.0, 1.0, CubicBezier::LINEAR); Curve::MAX_KEYS];
        let source = curve.keys();
        keys[..source.len()].copy_from_slice(source);
        Self {
            keys,
            len: source.len(),
        }
    }

    fn as_slice(&self) -> &[CurveKey] {
        &self.keys[..self.len]
    }

    /// Whether the key at `index` lies between the ends: the ends never move.
    const fn is_inner(&self, index: usize) -> bool {
        index > 0 && index + 1 < self.len
    }

    /// Adds a key at `(x, y)` between the keys around it, its segment linear; the segment it
    /// splits keeps its easing. Returns its index, or `None` when the curve holds
    /// [`Curve::MAX_KEYS`] or the key would lie on another or past the ends.
    fn add(&mut self, x: f32, y: f32) -> Option<usize> {
        if self.len >= Curve::MAX_KEYS || !x.is_finite() {
            return None;
        }
        let index = self.as_slice().iter().position(|key| key.x > x)?;
        if index == 0 || x - self.keys[index - 1].x < MIN_GAP || self.keys[index].x - x < MIN_GAP {
            return None;
        }
        self.keys.copy_within(index..self.len, index + 1);
        self.keys[index] =
            CurveKey::new(x, y.clamp(-Curve::LIMIT, Curve::LIMIT), CubicBezier::LINEAR);
        self.len += 1;
        Some(index)
    }

    /// Removes the inner key at `index`; the segment before it now reaches the next key.
    fn remove(&mut self, index: usize) -> bool {
        if !self.is_inner(index) {
            return false;
        }
        self.keys.copy_within(index + 1..self.len, index);
        self.len -= 1;
        true
    }

    /// Moves the inner key at `index` to `(x, y)`, kept between its neighbours in time and
    /// within [`Curve::LIMIT`]. Returns whether it moved.
    fn move_to(&mut self, index: usize, x: f32, y: f32) -> bool {
        if !self.is_inner(index) || !x.is_finite() || !y.is_finite() {
            return false;
        }
        let (low, high) = (
            self.keys[index - 1].x + MIN_GAP,
            self.keys[index + 1].x - MIN_GAP,
        );
        if low > high {
            return false;
        }
        let key = &mut self.keys[index];
        let (x, y) = (x.clamp(low, high), y.clamp(-Curve::LIMIT, Curve::LIMIT));
        let moved = (key.x - x).abs() > f32::EPSILON || (key.y - y).abs() > f32::EPSILON;
        key.x = x;
        key.y = y;
        moved
    }

    /// The middle of the widest segment, where "Add key" puts a key.
    fn widest_middle(&self) -> Option<f32> {
        self.as_slice()
            .windows(2)
            .max_by(|a, b| (a[1].x - a[0].x).total_cmp(&(b[1].x - b[0].x)))
            .map(|pair| f32::midpoint(pair[0].x, pair[1].x))
    }
}

/// Where the curve lies in the graph: the square from time 0 to 1, values from `low` to
/// `high`.
#[derive(Clone, Copy, Debug)]
struct Plot {
    rect: Rect,
    low: f32,
    high: f32,
}

impl Plot {
    fn point_of(self, x: f32, y: f32) -> Vec2 {
        let t = (y - self.low) / (self.high - self.low).max(1e-3);
        Vec2::new(
            self.rect.min.x + x * self.rect.width(),
            self.rect.max.y - t * self.rect.height(),
        )
    }

    fn place_at(self, point: Vec2) -> (f32, f32) {
        let x = ((point.x - self.rect.min.x) / self.rect.width().max(1.0)).clamp(0.0, 1.0);
        let t = (self.rect.max.y - point.y) / self.rect.height().max(1.0);
        (x, self.low + t * (self.high - self.low))
    }
}

/// The values the graph shows: 0 to 1, widened to the curve's overshoots, with a margin.
fn value_range(curve: &Curve) -> (f32, f32) {
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for step in 0..=SAMPLES {
        let y = curve.ease(step as f32 / SAMPLES as f32);
        low = low.min(y);
        high = high.max(y);
    }
    for key in curve.keys() {
        low = low.min(key.y);
        high = high.max(key.y);
    }
    let margin = (high - low) * 0.08;
    (low - margin, high + margin)
}

/// The graph: frame, grid, curve and keys. Presses select keys and drag the inner ones, a
/// double click adds one, the secondary button removes one. Returns whether `keys` changed.
fn show_graph(
    ui: &mut Ui<'_>,
    id: WidgetId,
    rect: Rect,
    curve: &Curve,
    keys: &mut Keys,
    state: &mut CurveEditorState,
) -> bool {
    let style = ui.styles().curve_editor;
    ui.scene().push_rect(
        &RectPrimitive::new(rect, style.background)
            .with_corner_radius(style.corner_radius)
            .with_border(style.border_width, style.border),
    );
    let range = state
        .drag
        .map_or_else(|| value_range(curve), |drag| drag.range);
    let plot = Plot {
        rect: rect.inset(style.padding),
        low: range.0,
        high: range.1,
    };
    // The grid: the start, the middle and the end in time; the levels 0 and 1.
    let line = ui.whole_pixels(1.0);
    for x in [0.0, 0.5, 1.0] {
        let (top, bottom) = (plot.point_of(x, range.1), plot.point_of(x, range.0));
        let stroke = Rect::from_min_max(
            Vec2::new(top.x - line * 0.5, top.y),
            Vec2::new(top.x + line * 0.5, bottom.y),
        );
        ui.scene()
            .push_rect(&RectPrimitive::new(stroke, style.grid));
    }
    for y in [0.0, 1.0] {
        let (left, right) = (plot.point_of(0.0, y), plot.point_of(1.0, y));
        let stroke = Rect::from_min_max(
            Vec2::new(left.x, left.y - line * 0.5),
            Vec2::new(right.x, left.y + line * 0.5),
        );
        ui.scene()
            .push_rect(&RectPrimitive::new(stroke, style.grid));
    }

    // Presses: the key under the pointer, if any.
    let pointer = ui.input().pointer();
    let near = pointer.and_then(|pointer| {
        keys.as_slice()
            .iter()
            .enumerate()
            .map(|(index, key)| (index, (plot.point_of(key.x, key.y) - pointer).length()))
            .filter(|&(_, distance)| distance <= style.pick_distance)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
    });
    let movable = state.drag.is_some() || near.is_some_and(|index| keys.is_inner(index));
    let response = if movable {
        ui.interact_with_cursor(id.with(1), rect, CursorShape::Move)
    } else {
        ui.interact(id.with(1), rect)
    };
    let mut changed = false;
    if response.double_clicked && near.is_none() {
        if let Some(pointer) = pointer {
            let (x, y) = plot.place_at(pointer);
            if let Some(index) = keys.add(x, y) {
                state.selected = Some(index);
                changed = true;
            }
        }
    } else if response.pressed {
        state.selected = near;
        state.drag = near
            .filter(|&index| keys.is_inner(index))
            .map(|index| Drag {
                index,
                from: keys.keys[index],
                range,
            });
    } else if response.secondary_pressed
        && state.drag.is_none()
        && let Some(index) = near
        && keys.remove(index)
    {
        state.selected = None;
        changed = true;
    }
    if let Some(drag) = state.drag {
        // The operation's keys: its cancel brings the key back, its confirm keeps it.
        let end = ui.operation();
        if end == Some(OperationEnd::Cancelled) {
            changed |= keys.move_to(drag.index, drag.from.x, drag.from.y);
            state.drag = None;
        } else if end == Some(OperationEnd::Confirmed) {
            state.drag = None;
        } else if response.held
            && let Some(pointer) = pointer
        {
            let (x, y) = plot.place_at(pointer);
            changed |= keys.move_to(drag.index, x, y);
        } else {
            state.drag = None;
        }
    }

    // The curve as `keys` make it now, then its keys, the selected one and the one under the
    // pointer larger.
    let shown = Curve::new(keys.as_slice());
    let shown = if shown.is_valid() { shown } else { *curve };
    let mut previous = plot.point_of(0.0, shown.ease(0.0));
    let mut step = 1;
    while step <= SAMPLES {
        let at = |step: usize| {
            let x = step as f32 / SAMPLES as f32;
            plot.point_of(x, shown.ease(x))
        };
        let next = at(step);
        let mut stroke = LinePrimitive::new(previous, next, style.curve_width, style.curve);
        previous = next;
        if step < SAMPLES {
            // Two segments per primitive: their joint keeps the stroke's weight.
            let after = at(step + 1);
            stroke = stroke.then(after);
            previous = after;
        }
        ui.scene().push_line(&stroke);
        step += 2;
    }
    for (index, key) in shown.keys().iter().enumerate() {
        let selected = state.selected == Some(index);
        let large = selected || near == Some(index) || state.drag.is_some_and(|d| d.index == index);
        let side = if large {
            style.key_size_selected
        } else {
            style.key_size
        };
        let fill = if selected {
            style.key_selected
        } else if keys.is_inner(index) {
            style.key
        } else {
            style.key_fixed
        };
        let at = Rect::from_center_size(plot.point_of(key.x, key.y), Vec2::splat(side));
        ui.scene().push_rect(
            &RectPrimitive::new(at, fill)
                .with_corner_radius(side * 0.5)
                .with_border(style.key_outline_width, style.key_outline),
        );
    }
    changed
}

/// The buttons adding and removing keys, then the selected key's place. Returns whether
/// `keys` changed.
fn key_fields(
    ui: &mut Ui<'_>,
    id: WidgetId,
    rect: Rect,
    curve: &Curve,
    keys: &mut Keys,
    state: &mut CurveEditorState,
) -> bool {
    let gap = ui.styles().property_group.column_gap;
    let selected = state.selected;
    let removable = selected.is_some_and(|index| keys.is_inner(index));
    let mut cells = [
        GroupCell::new(
            Button::new(id.with(10))
                .icon(koebi_icons::interface::PLUS)
                .enabled(keys.len < Curve::MAX_KEYS),
        ),
        GroupCell::new(
            Button::new(id.with(11))
                .icon(koebi_icons::interface::MINUS)
                .enabled(removable),
        ),
    ];
    // Two square buttons, their names in tooltips; the fields take the rest of the row.
    let width = (2.0 * rect.height()).min(rect.width());
    let buttons = Rect::from_min_max(rect.min, Vec2::new(rect.min.x + width, rect.max.y));
    let fields = Rect::from_min_max(Vec2::new(buttons.max.x + gap, rect.min.y), rect.max);
    let half = Rect::from_min_max(
        buttons.min,
        Vec2::new(buttons.min.x + width * 0.5, buttons.max.y),
    );
    ui.tooltip(id.with(10), tr("Add key"), half);
    ui.tooltip(
        id.with(11),
        tr("Remove key"),
        Rect::from_min_max(Vec2::new(half.max.x, buttons.min.y), buttons.max),
    );
    let mut changed = false;
    match ButtonGroup::horizontal()
        .show(ui, buttons, &mut cells)
        .clicked
    {
        Some(0) => {
            if let Some(x) = keys.widest_middle()
                && let Some(index) = keys.add(x, curve.ease(x))
            {
                state.selected = Some(index);
                changed = true;
            }
        }
        Some(1) => {
            if let Some(index) = selected
                && keys.remove(index)
            {
                state.selected = None;
                changed = true;
            }
        }
        _ => {}
    }
    // The place of the selected key; the ends' stay put.
    let style = &ui.styles().number_field;
    let group = FieldGroup::horizontal(fields, 2, style.corner_radius, style.border_width);
    let key = selected.and_then(|index| keys.as_slice().get(index).copied());
    let editable = selected.filter(|_| removable);
    let mut x = key.map_or(0.0, |key| key.x);
    let mut y = key.map_or(0.0, |key| key.y);
    let (cell, corners) = group.cell(0);
    let moved_x = NumberField::new(id.with(12), &mut x)
        .label("X")
        .range(0.0, 1.0)
        .decimals(3)
        .corner_radii(corners)
        .step_buttons(false)
        .enabled(editable.is_some())
        .show(ui, cell)
        .changed;
    let (cell, corners) = group.cell(1);
    let moved_y = NumberField::new(id.with(13), &mut y)
        .label("Y")
        .range(-Curve::LIMIT, Curve::LIMIT)
        .decimals(3)
        .corner_radii(corners)
        .step_buttons(false)
        .enabled(editable.is_some())
        .show(ui, cell)
        .changed;
    ui.tooltip(id.with(12), tr("The selected key's time"), fields);
    ui.tooltip(id.with(13), tr("The selected key's value"), fields);
    if (moved_x || moved_y)
        && let Some(index) = editable
    {
        changed |= keys.move_to(index, x, y);
    }
    changed
}

/// The easing of the segment leaving the selected key: its cubic Bézier's control points.
/// Returns whether `keys` changed.
fn easing_fields(
    ui: &mut Ui<'_>,
    id: WidgetId,
    rect: Rect,
    keys: &mut Keys,
    state: &CurveEditorState,
) -> bool {
    let style = &ui.styles().number_field;
    let group = FieldGroup::horizontal(rect, 4, style.corner_radius, style.border_width);
    let index = state.selected.filter(|&index| index + 1 < keys.len);
    let easing = index.map_or(CubicBezier::LINEAR, |index| keys.keys[index].easing);
    let mut values = [easing.x1, easing.y1, easing.x2, easing.y2];
    let labels = ["X1", "Y1", "X2", "Y2"];
    let mut changed = false;
    for (part, value) in values.iter_mut().enumerate() {
        let (cell, corners) = group.cell(part);
        // The control points' x stay within the segment; their y overshoot within the limit.
        let (low, high) = if part % 2 == 0 {
            (0.0, 1.0)
        } else {
            (-Curve::LIMIT, Curve::LIMIT)
        };
        let key = id.with(20 + part as u64);
        changed |= NumberField::new(key, value)
            .label(labels[part])
            .range(low, high)
            .decimals(2)
            .corner_radii(corners)
            .step_buttons(false)
            .enabled(index.is_some())
            .show(ui, cell)
            .changed;
        ui.tooltip(key, tr(EASING_TIPS[part]), cell);
    }
    let Some(index) = index.filter(|_| changed) else {
        return false;
    };
    let [x1, y1, x2, y2] = values;
    let next = CubicBezier::new(x1, y1, x2, y2);
    if !next.is_valid() || next == keys.keys[index].easing {
        return false;
    }
    keys.keys[index].easing = next;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linear() -> Keys {
        Keys::of(&Curve::LINEAR)
    }

    #[test]
    fn keys_go_between_their_neighbours_and_never_past_eight() {
        let mut keys = linear();
        assert_eq!(keys.add(0.5, 0.8), Some(1));
        assert_eq!((keys.keys[1].x, keys.keys[1].y), (0.5, 0.8));
        assert!(Curve::new(keys.as_slice()).is_valid());
        // Not on another key, nor past the ends.
        assert_eq!(keys.add(0.5, 0.2), None);
        assert_eq!(keys.add(1.5, 0.2), None);
        assert_eq!(keys.add(f32::NAN, 0.2), None);
        // Values past the limit are brought back to it.
        assert_eq!(keys.add(0.25, 9.0), Some(1));
        assert_eq!(keys.keys[1].y, Curve::LIMIT);
        for step in 0..10 {
            keys.add(0.05 + step as f32 * 0.09, 0.5);
        }
        assert_eq!(keys.len, Curve::MAX_KEYS);
        assert!(Curve::new(keys.as_slice()).is_valid());
    }

    #[test]
    fn inner_keys_move_between_their_neighbours_and_the_ends_stay() {
        let mut keys = linear();
        keys.add(0.3, 0.3);
        keys.add(0.6, 0.6);
        // Past its right neighbour: kept before it.
        assert!(keys.move_to(1, 0.9, 5.0));
        assert!(keys.keys[1].x < keys.keys[2].x);
        assert_eq!(keys.keys[1].y, Curve::LIMIT);
        // The ends never move, nor go.
        assert!(!keys.move_to(0, 0.5, 0.5));
        assert!(!keys.move_to(3, 0.5, 0.5));
        assert!(!keys.remove(0));
        assert!(!keys.remove(3));
        assert!(keys.remove(1));
        assert_eq!(keys.len, 3);
        assert!(Curve::new(keys.as_slice()).is_valid());
    }

    #[test]
    fn a_new_key_splits_the_widest_segment() {
        let mut keys = linear();
        keys.add(0.2, 0.2);
        assert_eq!(keys.widest_middle(), Some(0.6));
    }

    #[test]
    fn the_graph_shows_overshoots() {
        let overshoot = Curve::new(&[
            CurveKey::new(0.0, 0.0, CubicBezier::LINEAR),
            CurveKey::new(0.5, 1.5, CubicBezier::LINEAR),
            CurveKey::new(1.0, 1.0, CubicBezier::LINEAR),
        ]);
        let (low, high) = value_range(&overshoot);
        assert!(low < 0.0 && high > 1.5);
        let (low, high) = value_range(&Curve::LINEAR);
        assert!(low < 0.0 && low > -0.2 && high > 1.0 && high < 1.2);
    }

    #[test]
    fn places_map_to_the_graph_and_back() {
        let plot = Plot {
            rect: Rect::from_origin_size(Vec2::new(10.0, 20.0), Vec2::new(200.0, 100.0)),
            low: -0.5,
            high: 1.5,
        };
        let point = plot.point_of(0.25, 1.0);
        assert!((point.x - 60.0).abs() < 1e-4 && (point.y - 45.0).abs() < 1e-4);
        let (x, y) = plot.place_at(point);
        assert!((x - 0.25).abs() < 1e-5 && (y - 1.0).abs() < 1e-5);
        // Past the sides: time stays from 0 to 1.
        assert_eq!(plot.place_at(Vec2::new(-50.0, 45.0)).0, 0.0);
    }
}
