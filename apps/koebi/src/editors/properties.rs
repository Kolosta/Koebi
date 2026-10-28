//! The Properties template: the properties of the document's object, edited with the
//! components in real use. Property groups in collapsible panels (a nested one included), a
//! text field, vectors of number fields in units, checkboxes, a bounded number field on a bar,
//! a dropdown, an integer field and a button group. Every Properties editor edits the same
//! object; each keeps its own panels' folds, scroll position and menus.

use koebi_core::expr::{DEGREE, MILLIMETER};
use koebi_core::{Rect, Vec2};
use koebi_ui::{
    BarMapping, Button, ButtonGroup, Checkbox, CollapsiblePanel, Dropdown, GroupCell, MenuItem,
    MenuState, NumberField, PanelLevel, PanelState, PropertyGroup, ScrollAxes, ScrollState,
    TextField, Ui, WidgetId,
};

/// The shading modes of the object, for its dropdown.
const SHADINGS: [&str; 3] = ["Smooth", "Flat", "Auto smooth"];

/// The object the Properties editors edit: a template document's.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Object {
    name: String,
    description: String,
    location: [f64; 3],
    rotation: [f64; 3],
    scale: [f64; 3],
    delta: [f64; 3],
    in_viewport: bool,
    in_renders: bool,
    selectable: bool,
    opacity: f64,
    shading: usize,
    samples: i32,
}

impl Object {
    /// The template's object.
    pub(crate) fn template() -> Self {
        Self {
            name: "Cube".to_owned(),
            description: String::new(),
            location: [0.0, 0.0, 1000.0],
            rotation: [0.0, 0.0, 45.0],
            scale: [1.0; 3],
            delta: [0.0; 3],
            in_viewport: true,
            in_renders: true,
            selectable: true,
            opacity: 1.0,
            shading: 0,
            samples: 16,
        }
    }

    /// Back to the transform it starts with.
    fn reset_transform(&mut self) {
        let template = Self::template();
        self.location = template.location;
        self.rotation = template.rotation;
        self.scale = template.scale;
        self.delta = template.delta;
    }

    /// The delta transform added to the transform, cleared.
    fn apply_delta(&mut self) {
        for (location, delta) in self.location.iter_mut().zip(&mut self.delta) {
            *location += *delta;
            *delta = 0.0;
        }
    }
}

/// One Properties editor's own state: its panels' folds, its scroll position and the height
/// of its content on the last frame, its menu.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct View {
    object: PanelState,
    transform: PanelState,
    delta: PanelState,
    visibility: PanelState,
    shading: PanelState,
    scroll: ScrollState,
    height: f32,
    shading_menu: MenuState,
}

impl Default for View {
    fn default() -> Self {
        Self {
            object: PanelState::new(true),
            transform: PanelState::new(true),
            delta: PanelState::new(false),
            visibility: PanelState::new(true),
            shading: PanelState::new(true),
            scroll: ScrollState::new(),
            height: 0.0,
            shading_menu: MenuState::new(),
        }
    }
}

/// The rows of a property group inset by the panels' padding in `rect`.
fn group(ui: &Ui<'_>, rect: Rect) -> PropertyGroup {
    let padding = ui.styles().panel.padding;
    let inset = Rect::from_min_max(
        Vec2::new(rect.min.x + padding, rect.min.y),
        Vec2::new((rect.max.x - padding).max(rect.min.x + padding), rect.max.y),
    );
    PropertyGroup::new(ui, inset)
}

/// Shows the three components of a vector as number fields in `unit`, one per row.
fn vector(
    ui: &mut Ui<'_>,
    group: &mut PropertyGroup,
    id: WidgetId,
    label: &str,
    values: &mut [f64; 3],
    unit: Option<&'static koebi_core::expr::Unit>,
) {
    let rows = group.vector(ui, label, ["X", "Y", "Z"]);
    for ((value, rect), key) in values.iter_mut().zip(rows).zip(0_u64..) {
        let field = NumberField::new(id.with(key), value);
        let field = match unit {
            Some(unit) => field.unit(unit),
            None => field.decimals(3),
        };
        field.show(ui, rect);
    }
}

/// Draws a Properties editor in `rect`, editing `object`; menus open inside `window`.
pub(crate) fn show(
    ui: &mut Ui<'_>,
    id: WidgetId,
    rect: Rect,
    window: Rect,
    object: &mut Object,
    view: &mut View,
) {
    let content = Vec2::new(rect.width() - ui.styles().scroll.margin, view.height);
    let mut scroll = view.scroll;
    let height = ui.scroll_area(
        id.with(0),
        rect,
        &mut scroll,
        content,
        ScrollAxes::Vertical,
        |ui, area| {
            let margin = ui.styles().panel.padding;
            let gap = ui.styles().collapsible_panel.gap;
            let top = area.origin.y;
            let mut y = top + margin;
            let left = area.origin.x + margin;
            let right = area.viewport.max.x - margin;
            let mut next =
                |ui: &mut Ui<'_>,
                 panel: CollapsiblePanel<'_>,
                 add: &mut dyn FnMut(&mut Ui<'_>, Rect) -> f32| {
                    let at = Rect::from_min_max(Vec2::new(left, y), Vec2::new(right, y + 1.0e6));
                    let shown = panel.show(ui, at, |ui, rect| add(ui, rect));
                    y += shown.height + gap;
                };
            next(
                ui,
                CollapsiblePanel::new(id.with(1), &mut view.object, "Object"),
                &mut |ui, rect| {
                    let mut rows = group(ui, rect);
                    let name = rows.row(ui, "Name");
                    TextField::new(id.with(10), &mut object.name).show(ui, name);
                    let description = rows.row(ui, "Description");
                    TextField::new(id.with(11), &mut object.description)
                        .placeholder("What the object is for")
                        .show(ui, description);
                    rows.height()
                },
            );
            next(
                ui,
                CollapsiblePanel::new(id.with(2), &mut view.transform, "Transform"),
                &mut |ui, rect| {
                    let mut rows = group(ui, rect);
                    vector(
                        ui,
                        &mut rows,
                        id.with(20),
                        "Location",
                        &mut object.location,
                        Some(&MILLIMETER),
                    );
                    vector(
                        ui,
                        &mut rows,
                        id.with(21),
                        "Rotation",
                        &mut object.rotation,
                        Some(&DEGREE),
                    );
                    vector(ui, &mut rows, id.with(22), "Scale", &mut object.scale, None);
                    let buttons = rows.wide();
                    let mut cells = [
                        GroupCell::new(Button::new(id.with(23)).label("Reset")),
                        GroupCell::new(Button::new(id.with(24)).label("Apply delta")),
                    ];
                    let clicked = ButtonGroup::horizontal()
                        .show(ui, buttons, &mut cells)
                        .clicked;
                    match clicked {
                        Some(0) => object.reset_transform(),
                        Some(1) => object.apply_delta(),
                        _ => {}
                    }
                    // The delta transform: a second-level panel across the whole width.
                    let gap = ui.styles().property_group.group_gap;
                    let below = rect.min.y + rows.height() + gap;
                    let nested = Rect::from_min_max(Vec2::new(rect.min.x, below), rect.max);
                    let delta =
                        CollapsiblePanel::new(id.with(25), &mut view.delta, "Delta transform")
                            .level(PanelLevel::Second)
                            .show(ui, nested, |ui, rect| {
                                let mut rows = group(ui, rect);
                                vector(
                                    ui,
                                    &mut rows,
                                    id.with(26),
                                    "Location",
                                    &mut object.delta,
                                    Some(&MILLIMETER),
                                );
                                rows.height()
                            });
                    below + delta.height - rect.min.y
                },
            );
            next(
                ui,
                CollapsiblePanel::new(id.with(3), &mut view.visibility, "Visibility"),
                &mut |ui, rect| {
                    let mut rows = group(ui, rect);
                    let boxes = [
                        (&mut object.in_viewport, "Show in viewport"),
                        (&mut object.in_renders, "Show in renders"),
                        (&mut object.selectable, "Selectable"),
                    ];
                    for ((checked, label), key) in boxes.into_iter().zip(30_u64..) {
                        Checkbox::new(id.with(key), checked)
                            .label(label)
                            .show(ui, rows.unlabeled());
                    }
                    rows.group();
                    let opacity = rows.row(ui, "Opacity");
                    NumberField::new(id.with(33), &mut object.opacity)
                        .range(0.0, 1.0)
                        .bar(BarMapping::Linear)
                        .show(ui, opacity);
                    rows.height()
                },
            );
            next(
                ui,
                CollapsiblePanel::new(id.with(4), &mut view.shading, "Shading"),
                &mut |ui, rect| {
                    let mut rows = group(ui, rect);
                    let current = object.shading;
                    let mut index = 0;
                    let marked = SHADINGS.map(|label| {
                        let item = MenuItem::action(label).selected(index == current);
                        index += 1;
                        item
                    });
                    let shading = rows.row(ui, "Shading");
                    if let Some(choice) =
                        Dropdown::new(id.with(40), &mut view.shading_menu, &marked)
                            .label(SHADINGS[object.shading.min(SHADINGS.len() - 1)])
                            .show(ui, shading, window)
                    {
                        object.shading = choice.index();
                    }
                    let samples = rows.row(ui, "Samples");
                    NumberField::new(id.with(41), &mut object.samples)
                        .range(1, 1024)
                        .show(ui, samples);
                    rows.height()
                },
            );
            y - top
        },
    );
    view.scroll = scroll;
    view.height = height;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transforms_reset_and_deltas_apply() {
        let mut object = Object::template();
        object.location = [5.0, 6.0, 7.0];
        object.delta = [1.0, 2.0, 3.0];
        object.apply_delta();
        assert_eq!(object.location, [6.0, 8.0, 10.0]);
        assert_eq!(object.delta, [0.0; 3]);
        object.reset_transform();
        assert_eq!(object.location, Object::template().location);
    }
}
