//! The Properties template: the properties of the document's object, edited with the
//! components in real use. Property groups in collapsible panels (a nested one included),
//! text fields, vectors of number fields in units, checkboxes, a bounded number field on a bar,
//! a dropdown, an integer field and a button group. Every Properties editor edits the same
//! object; each keeps its own panels' folds, scroll position and menus.

use koebi_core::expr::{DEGREE, MILLIMETER, Unit};
use koebi_core::{Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_tokens::component;
use koebi_ui::{
    BarMapping, Button, ButtonGroup, Checkbox, CollapsiblePanel, CollapsiblePanelState, Dropdown,
    GroupCell, MenuItem, MenuState, NumberField, PanelLevel, PropertyGroup, ScrollAxes,
    ScrollState, TextField, Track, Ui, WidgetId,
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
    object: CollapsiblePanelState,
    transform: CollapsiblePanelState,
    delta: CollapsiblePanelState,
    visibility: CollapsiblePanelState,
    shading: CollapsiblePanelState,
    scroll: ScrollState,
    height: f32,
    shading_menu: MenuState,
}

impl Default for View {
    fn default() -> Self {
        Self {
            object: CollapsiblePanelState::new(true),
            transform: CollapsiblePanelState::new(true),
            delta: CollapsiblePanelState::new(false),
            visibility: CollapsiblePanelState::new(true),
            shading: CollapsiblePanelState::new(true),
            scroll: ScrollState::new(),
            height: 0.0,
            shading_menu: MenuState::new(),
        }
    }
}

/// The rows of a property group inset by the panels' padding in `rect`.
fn group<'a>(ui: &Ui<'a>, rect: Rect) -> PropertyGroup<'a> {
    let padding = ui.styles().panel.padding;
    let inset = Rect::from_min_max(
        Vec2::new(rect.min.x + padding, rect.min.y),
        Vec2::new((rect.max.x - padding).max(rect.min.x + padding), rect.max.y),
    );
    PropertyGroup::new(ui, inset)
}

/// Shows the three components of a vector as number fields in `unit` (plain numbers without
/// one) with `decimals` decimals, one per row.
fn vector(
    ui: &mut Ui<'_>,
    group: &mut PropertyGroup<'_>,
    id: WidgetId,
    label: &str,
    values: &mut [f64; 3],
    unit: Option<&'static Unit>,
    decimals: usize,
) {
    let rows = group.vector(ui, label, ["X", "Y", "Z"]);
    for ((value, rect), key) in values.iter_mut().zip(rows).zip(0_u64..) {
        let field = NumberField::new(id.with(key), value).decimals(decimals);
        match unit {
            Some(unit) => field.unit(unit),
            None => field,
        }
        .show(ui, rect);
    }
}

/// The height a panel lays its content out in: the content measures what it uses.
const UNBOUNDED: f32 = 1.0e6;

/// Panels stacked down a column, `gap` apart.
struct Stack {
    left: f32,
    right: f32,
    y: f32,
    gap: f32,
}

impl Stack {
    /// Shows `panel` below the previous one, `add` drawing its content.
    fn panel(
        &mut self,
        ui: &mut Ui<'_>,
        panel: CollapsiblePanel<'_>,
        add: impl FnOnce(&mut Ui<'_>, Rect) -> f32,
    ) {
        let at = Rect::from_min_max(
            Vec2::new(self.left, self.y),
            Vec2::new(self.right, self.y + UNBOUNDED),
        );
        self.y += panel.show(ui, at, add).height + self.gap;
    }
}

/// Draws a Properties editor in `rect`, editing `object`; menus open inside `window`.
pub(crate) fn show(
    ui: &mut Ui<'_>,
    id: WidgetId,
    rect: Rect,
    window: Rect,
    design: &DesignSystem,
    object: &mut Object,
    view: &mut View,
) {
    let margin = design.get(component::SIDE_PANEL_MARGIN_DEFAULT);
    let content = Vec2::new(rect.width() - ui.styles().scroll.margin, view.height);
    let mut scroll = view.scroll;
    let height = ui.scroll_area(
        id.with(0),
        rect,
        &mut scroll,
        content,
        ScrollAxes::Vertical,
        |ui, area| {
            let top = area.origin.y;
            let mut stack = Stack {
                left: area.origin.x + margin,
                right: area.viewport.max.x - margin,
                y: top + margin,
                gap: ui.styles().collapsible_panel.gap,
            };
            stack.panel(
                ui,
                CollapsiblePanel::new(id.with(1), &mut view.object, "Object"),
                |ui, rect| {
                    let mut rows = group(ui, rect);
                    let name = rows.row(ui, "Name");
                    TextField::new(id.with(10), &mut object.name).show(ui, name);
                    let description = rows.row(ui, "Notes");
                    TextField::new(id.with(11), &mut object.description)
                        .placeholder("What the object is for")
                        .show(ui, description);
                    rows.height()
                },
            );
            stack.panel(
                ui,
                CollapsiblePanel::new(id.with(2), &mut view.transform, "Transform"),
                |ui, rect| {
                    let mut rows = group(ui, rect);
                    vector(
                        ui,
                        &mut rows,
                        id.with(20),
                        "Location",
                        &mut object.location,
                        Some(&MILLIMETER),
                        2,
                    );
                    vector(
                        ui,
                        &mut rows,
                        id.with(21),
                        "Rotation",
                        &mut object.rotation,
                        Some(&DEGREE),
                        1,
                    );
                    vector(
                        ui,
                        &mut rows,
                        id.with(22),
                        "Scale",
                        &mut object.scale,
                        None,
                        3,
                    );
                    let buttons = rows.wide_row();
                    let mut cells = [
                        GroupCell::new(Button::new(id.with(23)).label("Reset")),
                        GroupCell::new(Button::new(id.with(24)).label("Apply delta")),
                    ];
                    let clicked = ButtonGroup::horizontal()
                        .columns(&[Track::Share(1.0), Track::Share(1.0)])
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
                                    2,
                                );
                                rows.height()
                            });
                    below + delta.height - rect.min.y
                },
            );
            stack.panel(
                ui,
                CollapsiblePanel::new(id.with(3), &mut view.visibility, "Visibility"),
                |ui, rect| {
                    let mut rows = group(ui, rect);
                    let boxes = [
                        (&mut object.in_viewport, "Show in viewport"),
                        (&mut object.in_renders, "Show in renders"),
                        (&mut object.selectable, "Selectable"),
                    ];
                    for ((checked, label), key) in boxes.into_iter().zip(30_u64..) {
                        Checkbox::new(id.with(key), checked)
                            .label(label)
                            .show(ui, rows.unlabeled_row());
                    }
                    rows.start_group();
                    let opacity = rows.row(ui, "Opacity");
                    NumberField::new(id.with(33), &mut object.opacity)
                        .range(0.0, 1.0)
                        .bar(BarMapping::Linear)
                        .show(ui, opacity);
                    rows.height()
                },
            );
            stack.panel(
                ui,
                CollapsiblePanel::new(id.with(4), &mut view.shading, "Shading"),
                |ui, rect| {
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
            // The same margin below the last panel as above the first.
            stack.y - stack.gap + margin - top
        },
    );
    view.scroll = scroll;
    if (height - view.height).abs() > 0.01 {
        // The scroll area took the last frame's height: the next frame scrolls the new one.
        ui.request_redraw_at(ui.input().time());
    }
    view.height = height;
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;
    use koebi_text::Fonts;
    use koebi_ui::{Input, Styles, UiState};

    use super::*;

    const ID: WidgetId = WidgetId::new("properties");

    /// An editor shorter than its content, away from the window's corner.
    fn short() -> Rect {
        Rect::from_origin_size(Vec2::new(40.0, 60.0), Vec2::new(320.0, 200.0))
    }

    /// An editor tall enough for all its content.
    fn tall() -> Rect {
        Rect::from_origin_size(Vec2::new(40.0, 60.0), Vec2::new(320.0, 1000.0))
    }

    fn assert_near(value: f32, expected: f32) {
        assert!((value - expected).abs() < 0.01, "{value} != {expected}");
    }

    /// A Properties editor, its UI state and input, one frame at a time.
    struct Harness {
        design: DesignSystem,
        state: UiState,
        input: Input,
        styles: Styles,
        object: Object,
        view: View,
        area: Rect,
    }

    impl Harness {
        fn new(area: Rect) -> Self {
            let design = DesignSystem::new(&koebi_tokens::KOEBI).expect("Koebi's schema is valid");
            let mut state = UiState::new();
            state.set_fonts(Fonts::with_embedded());
            Self {
                styles: Styles::resolve(&design),
                design,
                state,
                input: Input::new(),
                object: Object::template(),
                view: View::default(),
                area,
            }
        }

        fn frame(&mut self) -> Scene {
            let mut scene = Scene::new();
            {
                let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
                show(
                    &mut ui,
                    ID,
                    self.area,
                    self.area,
                    &self.design,
                    &mut self.object,
                    &mut self.view,
                );
            }
            self.input.end_frame();
            scene
        }

        /// Draws frames as the application does, only when the last one asks for another,
        /// until none does. A frame that measures another height than the one it scrolled
        /// must ask for the next.
        fn settle(&mut self) {
            for _ in 0..100 {
                let before = self.view.height;
                self.frame();
                let Some(at) = self.state.redraw_at() else {
                    assert_near(self.view.height, before);
                    return;
                };
                self.input.set_time(at.max(self.input.time()));
            }
            panic!("the editor never settles");
        }

        /// The height of `rows` property rows in `groups` sub-groups.
        fn rows(&self, rows: usize, groups: usize) -> f32 {
            let mut scene = Scene::new();
            let mut state = UiState::new();
            let ui = Ui::new(&mut scene, &self.styles, &self.input, &mut state);
            PropertyGroup::new(&ui, self.area).measure(rows, groups)
        }

        /// The height of a panel showing `content` (closed: `None`).
        fn panel(&self, content: Option<f32>) -> f32 {
            let style = &self.styles.collapsible_panel;
            style.header_height
                + content.map_or(0.0, |content| {
                    (content + 2.0 * style.content_padding).round()
                })
        }

        fn margin(&self) -> f32 {
            self.design.get(component::SIDE_PANEL_MARGIN_DEFAULT)
        }

        /// The height of the transform panel's content.
        fn transform(&self, delta: bool) -> f32 {
            let delta = self.panel(delta.then(|| self.rows(3, 1)));
            self.rows(10, 4) + self.styles.property_group.group_gap + delta
        }

        /// The height of the editor's content, the transform panel and its delta transform
        /// open or not.
        fn expected(&self, transform: bool, delta: bool) -> f32 {
            let panels = self.panel(Some(self.rows(2, 1)))
                + self.panel(transform.then(|| self.transform(delta)))
                + self.panel(Some(self.rows(4, 2)))
                + self.panel(Some(self.rows(2, 1)));
            2.0 * self.margin() + panels + 3.0 * self.styles.collapsible_panel.gap
        }

        /// Where to click the transform panel's buttons, Reset then Apply delta.
        fn buttons(&self) -> [Vec2; 2] {
            let styles = &self.styles;
            let inset = self.margin() + styles.panel.border_width + styles.panel.padding;
            let left = self.area.min.x + inset;
            let right = self.area.max.x - styles.scroll.margin - inset;
            let content = self.area.min.y
                + self.margin()
                + self.panel(Some(self.rows(2, 1)))
                + styles.collapsible_panel.gap
                + styles.collapsible_panel.header_height
                + styles.collapsible_panel.content_padding;
            let y = content + self.rows(10, 4) - styles.property_group.row_height * 0.5;
            [0.25, 0.75].map(|share| Vec2::new(left + (right - left) * share, y))
        }

        fn click(&mut self, at: Vec2) {
            self.input.pointer_moved(at);
            self.frame();
            self.input.primary_button(true, at);
            self.input.primary_button(false, at);
            self.frame();
        }
    }

    #[test]
    fn the_editor_measures_its_panels_and_folds_them() {
        let mut harness = Harness::new(short());
        harness.settle();
        assert_near(harness.view.height, harness.expected(true, false));
        assert!(harness.frame().glyph_count() > 100);
        // Opening the delta transform adds its content to the transform panel's.
        harness.view.delta.set_open(true);
        harness.settle();
        assert_near(harness.view.height, harness.expected(true, true));
        assert_near(
            harness.expected(true, true) - harness.expected(true, false),
            harness.panel(Some(harness.rows(3, 1))) - harness.panel(None),
        );
        // Folding the transform panel takes its content away, the delta transform's too.
        harness.view.transform.set_open(false);
        harness.settle();
        assert_near(harness.view.height, harness.expected(false, true));
    }

    #[test]
    fn a_new_height_is_scrolled_on_the_next_frame() {
        let mut harness = Harness::new(short());
        // `settle` checks that each change of height asks for another frame.
        harness.settle();
        let height = harness.view.height;
        assert!(height > short().height(), "{height}");
        // Scrolled to the end, the last panel ends its margin above the bottom.
        harness.input.pointer_moved(short().center());
        harness.frame();
        harness.input.scrolled(Vec2::new(0.0, -100.0));
        harness.frame();
        assert_near(harness.view.scroll.offset().y, height - short().height());
        // Nothing moves any more: no frame is asked for.
        harness.frame();
        assert_eq!(harness.state.redraw_at(), None);
        assert_near(harness.view.height, height);
    }

    #[test]
    fn the_name_is_edited_in_its_text_field() {
        let mut harness = Harness::new(short());
        harness.settle();
        let styles = &harness.styles;
        // The first row of the first panel, near the end of its input column.
        let y = short().min.y
            + harness.margin()
            + styles.collapsible_panel.header_height
            + styles.collapsible_panel.content_padding
            + styles.property_group.row_height * 0.5;
        let x = short().max.x - styles.scroll.margin - 3.0 * harness.margin();
        harness.click(Vec2::new(x, y));
        harness.input.typed("Sphere");
        harness.frame();
        assert_eq!(harness.object.name, "Sphere");
    }

    #[test]
    fn the_buttons_reset_the_transform_and_apply_the_delta() {
        let mut harness = Harness::new(tall());
        harness.object.location = [5.0, 6.0, 7.0];
        harness.object.delta = [1.0, 2.0, 3.0];
        harness.object.rotation = [10.0, 20.0, 30.0];
        harness.settle();
        let [reset, apply] = harness.buttons();
        harness.click(apply);
        assert_eq!(harness.object.location, [6.0, 8.0, 10.0]);
        assert_eq!(harness.object.delta, [0.0; 3]);
        assert_eq!(harness.object.rotation, [10.0, 20.0, 30.0]);
        harness.click(reset);
        let template = Object::template();
        assert_eq!(harness.object.location, template.location);
        assert_eq!(harness.object.rotation, template.rotation);
    }

    #[test]
    fn transforms_reset_and_deltas_apply() {
        let mut object = Object::template();
        object.location = [5.0, 6.0, 7.0];
        object.delta = [1.0, 2.0, 3.0];
        object.apply_delta();
        assert_eq!(object.location, [6.0, 8.0, 10.0]);
        assert_eq!(object.delta, [0.0; 3]);
        // Resetting touches the transform only.
        object.name = "Sphere".to_owned();
        object.rotation = [10.0, 20.0, 30.0];
        object.scale = [2.0; 3];
        object.delta = [1.0; 3];
        object.opacity = 0.5;
        object.reset_transform();
        let expected = Object {
            name: "Sphere".to_owned(),
            opacity: 0.5,
            ..Object::template()
        };
        assert_eq!(object, expected);
    }
}
