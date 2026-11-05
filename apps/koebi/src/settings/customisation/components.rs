//! Customisation by component: every component of the interface by kind, as its documentation
//! for users. Each page tells what the component is for and how it behaves, pictures it in
//! each of its states (forced: they look exactly as they do for real, and react to nothing),
//! offers one to try, lists its shortcuts, then its tokens: what it does first (motion, delays,
//! distances, factors: what a picture cannot show), then all of them, each editable for the
//! application and the current theme with its resolution chain, as by feature.

use koebi_core::{Color, Rect, Vec2};
use koebi_design::kind::KindId;
use koebi_design::schema::Group;
use koebi_design::value::{CubicBezier, Curve, CurveKey};
use koebi_design::{DesignSystem, TokenId};
use koebi_input::{ActionGroup, Keymap};
use koebi_tokens::component;
use koebi_ui::{
    Align, BarMapping, Button, Checkbox, CollapsiblePanel, CollapsiblePanelState, ColorField,
    ColorFieldState, ControlState, CurveEditor, CurveEditorState, Dropdown, MenuItem, MenuState,
    NumberField, TextField, Ui, WidgetId,
};

use super::super::options::{self, OptionDesc, OptionRow};
use super::super::{Stack, UNBOUNDED};

/// What a component's page lets the user try, and pictures in its states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Preview {
    Button,
    Checkbox,
    NumberField,
    TextField,
    Dropdown,
    ColorField,
    CurveEditor,
    Tooltip,
    Panel,
    /// Nothing to try: the page shows its shortcuts and tokens.
    None,
}

/// The documentation of a component.
#[derive(Debug)]
struct Doc {
    title: &'static str,
    /// What it is for and how it behaves, for users.
    text: &'static str,
    preview: Preview,
    /// The token groups that style it.
    groups: &'static [Group],
    /// The actions it answers, shown with their shortcuts.
    actions: Option<&'static ActionGroup>,
}

/// The components by kind.
static KINDS: [(&str, &[Doc]); 4] = [
    (
        "Actions",
        &[
            Doc {
                title: "Button",
                text: "A button runs an action when it is clicked: the pointer pressed and \
                    released over it. Moving away before releasing cancels the click. A \
                    toggle button stays selected until clicked again; a selected button that \
                    is pressed shows both. Disabled buttons show dimmed and ignore the pointer, \
                    but still keep it from what lies below.",
                preview: Preview::Button,
                groups: &[component::BUTTON::GROUP, component::ICON_BUTTON::GROUP],
                actions: None,
            },
            Doc {
                title: "Checkbox",
                text: "A checkbox turns an option on or off. A click anywhere on it, its label \
                    included, flips it; a mixed box (some of several options on) becomes \
                    checked.",
                preview: Preview::Checkbox,
                groups: &[component::CHECKBOX::GROUP, component::TOGGLE::GROUP],
                actions: None,
            },
        ],
    ),
    (
        "Values",
        &[
            Doc {
                title: "Number field",
                text: "A number field shows a value and changes it. Drag it sideways: the \
                    pointer hides and the value follows, finer with Shift, coarser with Ctrl; \
                    Escape or the secondary button brings back the value the drag started \
                    from. A click without moving types the value: expressions and units \
                    (\"2 * 3 mm\") are understood. The step buttons, and Ctrl with the wheel, \
                    step it. A bounded field fills a bar up to its value.",
                preview: Preview::NumberField,
                groups: &[component::NUMBER_FIELD::GROUP],
                actions: Some(&koebi_ui::NUMBER_ACTIONS),
            },
            Doc {
                title: "Text field",
                text: "A text field edits one line of text. A click places the caret, a drag \
                    selects, a double click selects a word; the usual shortcuts copy, cut, \
                    paste and undo. Enter keeps the text, Escape brings back what it was.",
                preview: Preview::TextField,
                groups: &[component::TEXT_FIELD::GROUP],
                actions: Some(&koebi_ui::TEXT_ACTIONS),
            },
            Doc {
                title: "Dropdown",
                text: "A dropdown shows the current choice and opens the list of the others \
                    below it, its corners meeting the list's. The arrows move in the list, \
                    Enter chooses, Escape closes it without changing anything.",
                preview: Preview::Dropdown,
                groups: &[component::DROPDOWN::GROUP, component::MENU::GROUP],
                actions: Some(&koebi_ui::MENU_ACTIONS),
            },
            Doc {
                title: "Color field and picker",
                text: "A color field shows a color, each half showing its transparency its own \
                    way, outlined in a gray of its own. A click opens the picker: an area of \
                    two components, strips for the third and the opacity, the colors when it \
                    opened and originally, an eyedropper, and the fields of a model (RGB, \
                    HSV, HSL, OKLCH, CMYK, hex). Dragging hides the pointer; Shift is finer; \
                    Escape brings the color back.",
                preview: Preview::ColorField,
                groups: &[
                    component::COLOR_FIELD::GROUP,
                    component::COLOR_PICKER::GROUP,
                ],
                actions: None,
            },
            Doc {
                title: "Curve editor",
                text: "A curve editor shapes how something eases over time. Drag a key to move \
                    it (Escape brings it back), double click the graph to add one, right click \
                    a key to remove it; the fields type the selected key's place and the \
                    easing of the segment leaving it. It is temporary: curves get a fuller \
                    editor later.",
                preview: Preview::CurveEditor,
                groups: &[component::CURVE_EDITOR::GROUP],
                actions: None,
            },
        ],
    ),
    (
        "Containers",
        &[
            Doc {
                title: "Panel",
                text: "Panels gather what belongs together. An expandable panel opens and \
                    closes with a click on its header; nested panels span their parent's width \
                    and indent what they hold. A dot at the end of a header tells something \
                    below was changed, and brings it back.",
                preview: Preview::Panel,
                groups: &[component::PANEL::GROUP, component::PROPERTY_GROUP::GROUP],
                actions: None,
            },
            Doc {
                title: "Tooltip",
                text: "A tooltip names what lies under the pointer once it rests there for a \
                    moment; moving to a neighbour shows the neighbour's at once, for a little \
                    while.",
                preview: Preview::Tooltip,
                groups: &[component::TOOLTIP::GROUP],
                actions: None,
            },
            Doc {
                title: "Status bar",
                text: "The status bar, at the bottom of workspace windows, shows the shortcuts \
                    that apply where the pointer is, the most useful first, and the version on \
                    the right.",
                preview: Preview::None,
                groups: &[component::STATUS_BAR::GROUP],
                actions: None,
            },
        ],
    ),
    (
        "Shortcuts",
        &[Doc {
            title: "Key caps",
            text: "Key caps show shortcuts as the keys to press: the modifiers first, each \
                    side of the keyboard as its own key when it matters, then the key or the \
                    mouse button.",
            preview: Preview::None,
            groups: &[component::KEY_CAP::GROUP, component::CAPTURE_FIELD::GROUP],
            actions: Some(&koebi_ui::MENU_ACTIONS),
        }],
    ),
];

/// The width of a control in the previews.
const CONTROL: f32 = 120.0;

/// The states a button shows, each with its name.
const BUTTON_STATES: [(ControlState, &str); 9] = [
    (ControlState::Normal, "Normal"),
    (ControlState::Hovered, "Hovered"),
    (ControlState::Pressed, "Pressed"),
    (ControlState::Selected, "Selected"),
    (ControlState::SelectedHovered, "Selected, hovered"),
    (ControlState::SelectedPressed, "Selected, pressed"),
    (ControlState::Active, "Active"),
    (ControlState::Disabled, "Disabled"),
    (ControlState::SelectedDisabled, "Selected, disabled"),
];

/// The states a checkbox shows (checked or not is its value).
const CHECKBOX_STATES: [(ControlState, &str); 4] = [
    (ControlState::Normal, "Normal"),
    (ControlState::Hovered, "Hovered"),
    (ControlState::Pressed, "Pressed"),
    (ControlState::Disabled, "Disabled"),
];

/// The states a number field shows.
const FIELD_STATES: [(ControlState, &str); 5] = [
    (ControlState::Normal, "Normal"),
    (ControlState::Hovered, "Hovered"),
    (ControlState::Pressed, "Pressed"),
    (ControlState::Active, "Dragged"),
    (ControlState::Disabled, "Disabled"),
];

/// The choices of the dropdown to try.
const CHOICE_NAMES: [&str; 3] = ["Linear", "Logarithmic", "Stepped"];
const CHOICES: [MenuItem<'static>; 3] = [
    MenuItem::action(CHOICE_NAMES[0]),
    MenuItem::action(CHOICE_NAMES[1]),
    MenuItem::action(CHOICE_NAMES[2]),
];

/// What the components to try keep.
#[derive(Debug)]
struct Live {
    toggled: bool,
    checked: bool,
    number: f64,
    text: String,
    menu: MenuState,
    choice: usize,
    color: Color,
    color_state: ColorFieldState,
    curve: Curve,
    curve_state: CurveEditorState,
    nested: CollapsiblePanelState,
}

impl Live {
    fn new() -> Self {
        Self {
            toggled: false,
            checked: true,
            number: 50.0,
            text: String::from("Untitled"),
            menu: MenuState::new(),
            choice: 0,
            color: Color::new(0.24, 0.53, 0.88, 0.8),
            color_state: ColorFieldState::new(),
            curve: Curve::new(&[
                CurveKey::new(0.0, 0.0, CubicBezier::new(0.3, 0.0, 0.6, 1.0)),
                CurveKey::new(0.6, 1.1, CubicBezier::new(0.4, 0.0, 0.6, 1.0)),
                CurveKey::new(1.0, 1.0, CubicBezier::LINEAR),
            ]),
            curve_state: CurveEditorState::new(),
            nested: CollapsiblePanelState::new(true),
        }
    }
}

/// A component's page and what it keeps.
#[derive(Debug)]
struct Page {
    doc: &'static Doc,
    panel: CollapsiblePanelState,
    menu: MenuState,
    /// Every token of its groups, for the header's dot.
    tokens: Vec<TokenId>,
    /// What it does: motion, delays, distances, factors.
    behaviour: Vec<OptionRow>,
    /// Every token.
    all: CollapsiblePanelState,
    rows: Vec<OptionRow>,
}

impl Page {
    fn new(design: &DesignSystem, doc: &'static Doc) -> Self {
        let mut page = Self {
            doc,
            panel: CollapsiblePanelState::new(false),
            menu: MenuState::new(),
            tokens: Vec::new(),
            behaviour: Vec::new(),
            all: CollapsiblePanelState::new(false),
            rows: Vec::new(),
        };
        for group in doc.groups {
            for (info, index) in group.tokens().iter().zip(group.start()..) {
                let token = TokenId::from_index(index);
                page.tokens.push(token);
                let desc = OptionDesc::of(design, token, info.path());
                if is_behaviour(info.kind(), info.path()) {
                    page.behaviour.push(OptionRow::new(desc));
                }
                page.rows.push(OptionRow::new(desc));
            }
        }
        page
    }
}

/// Whether a token decides what a component does rather than how it looks: motion, delays,
/// distances and factors of the pointer.
fn is_behaviour(kind: KindId, path: &str) -> bool {
    const INPUT: [&str; 8] = [
        "drag",
        "delay",
        "distance",
        "threshold",
        "factor",
        "repeat",
        "grace",
        "speed",
    ];
    matches!(kind, KindId::Duration | KindId::Curve | KindId::CubicBezier)
        || INPUT.iter().any(|word| path.contains(word))
}

/// A kind of components: its panel and its pages.
#[derive(Debug)]
struct Kind {
    title: &'static str,
    panel: CollapsiblePanelState,
    menu: MenuState,
    tokens: Vec<TokenId>,
    pages: Vec<Page>,
}

/// Every component by kind.
#[derive(Debug)]
pub(super) struct ComponentPages {
    kinds: Vec<Kind>,
    live: Live,
}

impl ComponentPages {
    pub(super) fn new(design: &DesignSystem) -> Self {
        let kinds = KINDS
            .iter()
            .map(|(title, docs)| {
                let pages: Vec<Page> = docs.iter().map(|doc| Page::new(design, doc)).collect();
                Kind {
                    title,
                    panel: CollapsiblePanelState::new(true),
                    menu: MenuState::new(),
                    tokens: pages
                        .iter()
                        .flat_map(|page| page.tokens.iter().copied())
                        .collect(),
                    pages,
                }
            })
            .collect();
        Self {
            kinds,
            live: Live::new(),
        }
    }

    /// Shows the kinds below what `stack` holds.
    pub(super) fn show(
        &mut self,
        ui: &mut Ui<'_>,
        stack: &mut Stack,
        design: &mut DesignSystem,
        keymap: &Keymap,
    ) {
        const ID: WidgetId = WidgetId::new("customisation-components");
        let bounds = stack.bounds;
        let slot = options::reset_slot(design);
        let Self { kinds, live } = self;
        for (index, kind) in kinds.iter_mut().enumerate() {
            let id = ID.with(index as u64);
            let Kind {
                title,
                panel,
                menu,
                tokens,
                pages,
            } = kind;
            let area = Rect::from_min_max(
                Vec2::new(stack.left, stack.y),
                Vec2::new(stack.right, stack.y + UNBOUNDED),
            );
            let shown = CollapsiblePanel::new(id, panel, title).trailing(slot).show(
                ui,
                area,
                |ui, rect| {
                    let mut y = rect.min.y;
                    for (number, page) in pages.iter_mut().enumerate() {
                        let at = Rect::from_min_max(Vec2::new(rect.min.x, y), rect.max);
                        let key = id.with(number as u64 + 1);
                        y += show_page(ui, at, key, design, keymap, page, live, bounds);
                    }
                    y - rect.min.y
                },
            );
            if let Some(trailing) = shown.trailing {
                options::header_dot(ui, trailing, id.with(0), design, tokens, menu, bounds);
            }
            stack.y += shown.height + stack.gap;
        }
    }

    /// Opens the page titled `title`, closing the others.
    #[cfg(test)]
    pub(super) fn open(&mut self, title: &str) {
        for kind in &mut self.kinds {
            for page in &mut kind.pages {
                page.panel.set_open(page.doc.title == title);
            }
        }
    }

    /// The titles of every page, in their order.
    #[cfg(test)]
    pub(super) fn titles(&self) -> Vec<&'static str> {
        self.kinds
            .iter()
            .flat_map(|kind| kind.pages.iter().map(|page| page.doc.title))
            .collect()
    }

    /// The behaviour options of the page titled `title`.
    #[cfg(test)]
    pub(super) fn behaviour(&self, title: &str) -> Vec<TokenId> {
        self.kinds
            .iter()
            .flat_map(|kind| &kind.pages)
            .filter(|page| page.doc.title == title)
            .flat_map(|page| page.behaviour.iter().map(|row| row.desc().token))
            .collect()
    }
}

/// Shows a component's page at the top of `area`. Returns its height.
#[allow(clippy::too_many_arguments, reason = "the parts of one page")]
fn show_page(
    ui: &mut Ui<'_>,
    area: Rect,
    id: WidgetId,
    design: &mut DesignSystem,
    keymap: &Keymap,
    page: &mut Page,
    live: &mut Live,
    bounds: Rect,
) -> f32 {
    let slot = options::reset_slot(design);
    let Page {
        doc,
        panel,
        menu,
        tokens,
        behaviour,
        all,
        rows,
    } = page;
    let shown = CollapsiblePanel::new(id, panel, doc.title)
        .trailing(slot)
        .show(ui, area, |ui, rect| {
            let padding = ui.styles().panel.padding;
            let row = ui.styles().button.height;
            let gap = ui.styles().property_group.row_gap;
            let inner = Rect::from_min_max(
                Vec2::new(rect.min.x + padding, rect.min.y),
                Vec2::new((rect.max.x - padding).max(rect.min.x + padding), rect.max.y),
            );
            let mut y = inner.min.y;
            let font = ui.styles().text.body.clone();
            let color = ui.styles().text.primary;
            let text = Rect::from_min_max(Vec2::new(inner.min.x, y), inner.max);
            y += ui.paragraph(doc.text, text, &font, color) + gap;
            let section = |ui: &mut Ui<'_>, y: &mut f32, title: &str| {
                let at = Rect::from_min_max(
                    Vec2::new(inner.min.x, *y),
                    Vec2::new(inner.max.x, *y + row),
                );
                ui.section_title(at, title);
                *y += row + gap;
            };
            if has_states(doc.preview) {
                section(ui, &mut y, "States");
                y += states(
                    ui,
                    Vec2::new(inner.min.x, y),
                    inner.max.x,
                    id.with(1),
                    doc.preview,
                ) + gap;
            }
            if doc.preview != Preview::None {
                section(ui, &mut y, "Try it");
                let at = Rect::from_min_max(Vec2::new(inner.min.x, y), inner.max);
                y += try_it(ui, at, id.with(2), doc.preview, live, bounds) + gap;
            }
            if let Some(actions) = doc.actions {
                section(ui, &mut y, "Shortcuts");
                y += shortcuts(ui, Vec2::new(inner.min.x, y), inner.max.x, actions, keymap) + gap;
            }
            if !behaviour.is_empty() {
                section(ui, &mut y, "What it does");
                for (index, option) in behaviour.iter_mut().enumerate() {
                    let at = Rect::from_min_max(Vec2::new(rect.min.x, y), rect.max);
                    let key = id.with(100 + index as u64);
                    y += options::show_option(ui, at, key, design, option, bounds);
                }
                y += gap;
            }
            let at = Rect::from_min_max(Vec2::new(rect.min.x, y), rect.max);
            let tokens_panel =
                CollapsiblePanel::new(id.with(3), all, "Every token").show(ui, at, |ui, inner| {
                    let mut y = inner.min.y;
                    for (index, option) in rows.iter_mut().enumerate() {
                        let at = Rect::from_min_max(Vec2::new(inner.min.x, y), inner.max);
                        let key = id.with(1000 + index as u64);
                        y += options::show_option(ui, at, key, design, option, bounds);
                    }
                    y - inner.min.y
                });
            y += tokens_panel.height;
            y - rect.min.y
        });
    if let Some(trailing) = shown.trailing {
        options::header_dot(ui, trailing, id.with(0), design, tokens, menu, bounds);
    }
    shown.height
}

/// Whether `preview` pictures its states.
const fn has_states(preview: Preview) -> bool {
    matches!(
        preview,
        Preview::Button | Preview::Checkbox | Preview::NumberField
    )
}

/// The component of `preview` in each of its states, forced, named below, in rows from
/// `origin` up to `right`. Returns the height used.
fn states(ui: &mut Ui<'_>, origin: Vec2, right: f32, id: WidgetId, preview: Preview) -> f32 {
    let row = ui.styles().button.height;
    let gap = ui.styles().property_group.column_gap;
    let detail = ui.styles().text.detail.clone();
    let secondary = ui.styles().text.secondary;
    let caption = ui.line_height(&detail).ceil();
    let cell = Vec2::new(CONTROL, row + caption + gap);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a few cells across a page, never negative once floored at zero"
    )]
    let per_row = (((right - origin.x + gap) / (cell.x + gap)).floor().max(1.0)) as usize;
    let list: &[(ControlState, &str)] = match preview {
        Preview::Button => &BUTTON_STATES,
        Preview::Checkbox => &CHECKBOX_STATES,
        _ => &FIELD_STATES,
    };
    for (index, &(state, name)) in list.iter().enumerate() {
        let (column, line) = (index % per_row, index / per_row);
        let at = origin + Vec2::new(column as f32 * (cell.x + gap), line as f32 * (cell.y + gap));
        let control = Rect::from_origin_size(at, Vec2::new(cell.x, row));
        let key = id.with(index as u64);
        match preview {
            Preview::Button => {
                Button::new(key)
                    .label("Save")
                    .forced(state)
                    .show(ui, control);
            }
            Preview::Checkbox => {
                let mut checked = index % 2 == 0;
                Checkbox::new(key, &mut checked)
                    .label("Snap")
                    .forced(state)
                    .show(ui, control);
            }
            _ => {
                let mut value = 0.4_f64;
                NumberField::new(key, &mut value)
                    .range(0.0, 1.0)
                    .decimals(2)
                    .label("Mix")
                    .bar(BarMapping::Linear)
                    .forced(state)
                    .show(ui, control);
            }
        }
        let name_at =
            Rect::from_origin_size(Vec2::new(at.x, at.y + row), Vec2::new(cell.x, caption));
        ui.label(name, name_at, &detail, secondary, Align::Start);
    }
    let lines = list.len().div_ceil(per_row) as f32;
    lines * cell.y + (lines - 1.0).max(0.0) * gap
}

/// The component of `preview` to try, at the top of `area`. Returns the height used.
fn try_it(
    ui: &mut Ui<'_>,
    area: Rect,
    id: WidgetId,
    preview: Preview,
    live: &mut Live,
    bounds: Rect,
) -> f32 {
    let row = ui.styles().button.height;
    let control = Rect::from_origin_size(area.min, Vec2::new(CONTROL * 1.5, row));
    match preview {
        Preview::Button => {
            Button::new(id)
                .label(if live.toggled { "On" } else { "Off" })
                .toggle(&mut live.toggled)
                .show(ui, control);
            row
        }
        Preview::Checkbox => {
            Checkbox::new(id, &mut live.checked)
                .label("Snap to grid")
                .show(ui, control);
            row
        }
        Preview::NumberField => {
            NumberField::new(id, &mut live.number)
                .range(0.0, 100.0)
                .decimals(1)
                .label("Opacity")
                .bar(BarMapping::Linear)
                .show(ui, control);
            row
        }
        Preview::TextField => {
            TextField::new(id, &mut live.text)
                .placeholder("Name")
                .show(ui, control);
            row
        }
        Preview::Dropdown => {
            let label = CHOICE_NAMES.get(live.choice).copied().unwrap_or_default();
            if let Some(choice) = Dropdown::new(id, &mut live.menu, &CHOICES)
                .label(label)
                .show(ui, control, bounds)
            {
                live.choice = choice.index();
            }
            row
        }
        Preview::ColorField => {
            ColorField::new(id, &mut live.color, &mut live.color_state)
                .dropdown(true)
                .show(ui, control, bounds);
            row
        }
        Preview::CurveEditor => {
            let height = CurveEditor::height(ui);
            let at = Rect::from_origin_size(area.min, Vec2::new(CONTROL * 3.0, height));
            CurveEditor::new(id, &mut live.curve, &mut live.curve_state).show(ui, at);
            height
        }
        Preview::Tooltip => {
            Button::new(id).label("Rest here").show(ui, control);
            ui.tooltip(id, "A tooltip names what lies under the pointer", control);
            row
        }
        Preview::Panel => {
            let at = Rect::from_min_max(area.min, Vec2::new(area.max.x, area.max.y));
            let shown = CollapsiblePanel::new(id, &mut live.nested, "A nested panel").show(
                ui,
                at,
                |ui, inner| {
                    let font = ui.styles().text.label.clone();
                    let color = ui.styles().text.secondary;
                    let line = Rect::from_origin_size(inner.min, Vec2::new(inner.width(), row));
                    ui.label("What it holds", line, &font, color, Align::Start);
                    row
                },
            );
            shown.height
        }
        Preview::None => 0.0,
    }
}

/// The actions of `group`, each with its first shortcut in `keymap`, one per row from `origin`
/// up to `right`. Returns the height used.
fn shortcuts(
    ui: &mut Ui<'_>,
    origin: Vec2,
    right: f32,
    group: &ActionGroup,
    keymap: &Keymap,
) -> f32 {
    let row = ui.styles().button.height;
    let font = ui.styles().text.label.clone();
    let color = ui.styles().text.primary;
    let width = (right - origin.x).min(CONTROL * 4.0);
    let mut y = origin.y;
    for action in group.actions {
        let line = Rect::from_origin_size(Vec2::new(origin.x, y), Vec2::new(width, row));
        ui.label(action.label, line, &font, color, Align::Start);
        if let Some(first) = keymap
            .shortcuts(action.id)
            .and_then(|shortcuts| shortcuts.first())
        {
            ui.key_caps(line, first, Align::End);
        }
        y += row;
    }
    y - origin.y
}

#[cfg(test)]
mod tests {
    use koebi_tokens::KOEBI;

    use super::*;

    #[test]
    fn every_page_has_its_tokens_and_what_it_does() {
        let design = DesignSystem::new(&KOEBI).unwrap();
        let pages = ComponentPages::new(&design);
        assert!(pages.titles().len() >= 10, "{:?}", pages.titles());
        // The number field's drag distance and factors are what it does.
        let behaviour = pages.behaviour("Number field");
        assert!(behaviour.contains(&component::NUMBER_FIELD_DRAG_STEP_DISTANCE.id()));
        assert!(behaviour.contains(&component::NUMBER_FIELD_DRAG_FINE_FACTOR.id()));
        // Its colors are not.
        assert!(!behaviour.contains(&component::NUMBER_FIELD_BACKGROUND_COLOR_DEFAULT.id()));
        // Panels open and close with a curve.
        assert!(
            pages
                .behaviour("Panel")
                .contains(&component::PANEL_EXPAND_CURVE.id())
        );
    }

    #[test]
    fn behaviour_is_motion_and_the_pointer() {
        assert!(is_behaviour(KindId::Duration, "tooltip.delay.default"));
        assert!(is_behaviour(
            KindId::Dimension,
            "number-field.drag.step-distance"
        ));
        assert!(!is_behaviour(
            KindId::Color,
            "button.background.color.default"
        ));
        assert!(!is_behaviour(
            KindId::Dimension,
            "button.border.width.default"
        ));
    }
}
