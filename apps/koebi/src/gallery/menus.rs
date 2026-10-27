//! The menus of the gallery: dropdowns opening below and beside, a menu button fused in a
//! group, a context menu with submenus, check and radio items, and tooltips.

use koebi_core::Rect;
use koebi_ui::{
    Align, Button, ButtonGroup, Dropdown, GroupCell, Icon, MenuChoice, MenuItem, MenuState,
    PanelLevel, Placement, PointerButton, Ui, WidgetId,
};

use super::{Metrics, cut_left, cut_top};

/// Rows of the panel: the controls, then the area that opens the context menu.
pub(super) const ROWS: u16 = 2;

const SHADINGS: [&str; 3] = ["Solid", "Wireframe", "Rendered"];
const MODES: [&str; 3] = ["Object", "Edit", "Sculpt"];
const PIVOTS: [&str; 3] = ["Median point", "Cursor", "Active element"];

/// The values the panel edits.
#[derive(Debug)]
pub(super) struct Menus {
    shading: usize,
    mode: usize,
    pivot: usize,
    snap: bool,
    /// The last action chosen in a menu.
    last: &'static str,
    shading_menu: MenuState,
    side_menu: MenuState,
    mode_menu: MenuState,
    context: MenuState,
}

impl Menus {
    pub(super) fn new() -> Self {
        Self {
            shading: 0,
            mode: 0,
            pivot: 0,
            snap: true,
            last: "Nothing chosen yet",
            shading_menu: MenuState::new(),
            side_menu: MenuState::new(),
            mode_menu: MenuState::new(),
            context: MenuState::new(),
        }
    }

    /// Draws the panel in `rect`; its menus open inside `window`.
    pub(super) fn show(&mut self, ui: &mut Ui<'_>, rect: Rect, window: Rect, metrics: Metrics) {
        const ID: WidgetId = WidgetId::new("menus");
        let mut content = ui.panel(rect, PanelLevel::First);
        let (row, gap) = (metrics.row, metrics.row_gap());

        let mut line = cut_top(&mut content, row, gap);
        let shadings = SHADINGS.map(MenuItem::action);
        let shadings = mark(shadings, self.shading);
        let shading = Dropdown::new(ID.with(0), &mut self.shading_menu, &shadings)
            .label(SHADINGS[self.shading]);
        let width = shading.size(ui).x.max(6.0 * row);
        if let Some(choice) = shading.show(ui, cut_left(&mut line, width, gap), window) {
            self.shading = choice.index();
        }
        ui.tooltip(ID.with(0), "How the viewport draws the scene", window);

        // A vertical dropdown: its menu opens beside it.
        let side = Dropdown::new(ID.with(1), &mut self.side_menu, &shadings)
            .icon(Icon::Plus)
            .placement(Placement::RIGHT);
        let width = side.size(ui).x;
        if let Some(choice) = side.show(ui, cut_left(&mut line, width, gap), window) {
            self.shading = choice.index();
        }

        // A menu button between two step buttons, fused in a group.
        let modes = mark(MODES.map(MenuItem::action), self.mode);
        let mut cells = [
            GroupCell::new(Button::new(ID.with(2)).icon(Icon::ChevronLeft)),
            GroupCell::new(
                Button::new(ID.with(3))
                    .label(MODES[self.mode])
                    .trailing_icon(Icon::ChevronDown)
                    .align(Align::Start)
                    .menu(&mut self.mode_menu, Placement::BELOW),
            ),
            GroupCell::new(Button::new(ID.with(4)).icon(Icon::ChevronRight)),
        ];
        let group = ButtonGroup::horizontal();
        let width = group.size(ui, &cells).x.max(7.0 * row);
        let stepped = group
            .show(ui, cut_left(&mut line, width, gap), &mut cells)
            .clicked;
        ui.tooltip(ID.with(2), "Previous mode", window);
        ui.tooltip(ID.with(4), "Next mode", window);
        match stepped {
            Some(0) => self.mode = (self.mode + MODES.len() - 1) % MODES.len(),
            Some(2) => self.mode = (self.mode + 1) % MODES.len(),
            _ => {}
        }
        if let Some(choice) = ui.menu(ID.with(5), &mut self.mode_menu, &modes, window) {
            self.mode = choice.index();
        }

        let styles = ui.styles();
        ui.label(
            self.last,
            line,
            &styles.text.label,
            styles.text.secondary,
            Align::Start,
        );

        // The area of the context menu.
        let area = cut_top(&mut content, row, gap);
        let response = ui.interact(ID.with(6), area);
        let styles = ui.styles();
        ui.label(
            "Right-click here for a context menu with submenus",
            area,
            &styles.text.label,
            styles.text.secondary,
            Align::Center,
        );
        if response.secondary_pressed
            && let Some(point) = ui.input().button_pressed_at(PointerButton::Secondary)
        {
            self.context.open_at(point);
        }
        self.context_menu(ui, window);
    }

    /// The context menu: actions, check and radio items, and nested submenus.
    fn context_menu(&mut self, ui: &mut Ui<'_>, window: Rect) {
        const MESHES: [MenuItem<'static>; 3] = [
            MenuItem::action("Cube"),
            MenuItem::action("Sphere"),
            MenuItem::action("Plane"),
        ];
        const LIGHTS: [MenuItem<'static>; 2] = [MenuItem::action("Point"), MenuItem::action("Sun")];
        const ADD: [MenuItem<'static>; 3] = [
            MenuItem::submenu("Mesh", &MESHES),
            MenuItem::submenu("Light", &LIGHTS),
            MenuItem::action("Camera"),
        ];
        let pivots: [MenuItem<'_>; 3] = core::array::from_fn(|index| {
            MenuItem::action(PIVOTS[index]).radio(index == self.pivot)
        });
        let items = [
            MenuItem::Title("Object"),
            MenuItem::submenu("Add", &ADD).icon(Icon::Plus),
            MenuItem::action("Duplicate").shortcut("Shift+D"),
            MenuItem::action("Delete").shortcut("X"),
            MenuItem::action("Join").enabled(false),
            MenuItem::Separator,
            MenuItem::action("Snap").checked(self.snap),
            MenuItem::submenu("Pivot point", &pivots),
        ];
        let Some(choice) = ui.menu(
            WidgetId::new("menus-context"),
            &mut self.context,
            &items,
            window,
        ) else {
            return;
        };
        self.last = describe(choice);
        match choice.path() {
            [6] => self.snap = !self.snap,
            [7, index] => self.pivot = *index,
            _ => {}
        }
    }
}

/// `items` with the item at `current` marked as the current choice.
fn mark<const N: usize>(items: [MenuItem<'static>; N], current: usize) -> [MenuItem<'static>; N] {
    let mut index = 0;
    items.map(|item| {
        let item = item.selected(index == current);
        index += 1;
        item
    })
}

/// What a choice of the context menu did, for the panel's status line.
fn describe(choice: MenuChoice) -> &'static str {
    match choice.path() {
        [1, 0, _] => "Added a mesh",
        [1, 1, _] => "Added a light",
        [1, 2] => "Added a camera",
        [2] => "Duplicated",
        [3] => "Deleted",
        [6] => "Toggled snapping",
        [7, _] => "Changed the pivot point",
        _ => "Chose an item",
    }
}
