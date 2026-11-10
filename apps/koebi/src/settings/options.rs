//! Options of the settings: expandable panels editing a token, whatever its kind
//! ([`super::values`]: numbers, colors, curves, cubic Béziers, font families).
//!
//! The header shows the option's name and, at its end, its value for the whole application (a
//! picture of it for a curve). The panel opens on the value for the application and for the
//! current theme, side by side, then on how the token gets its value. Beside every input lies a place for the dot bringing the value
//! back ([`Ui::reset_dot`]): it stays empty until the value is changed, and takes no other room
//! then.
//!
//! Headers show a dot too, when anything they hold is changed: the option's for its two scopes,
//! the group's for all its options. A click brings back every change below, for the application
//! and the current theme; the secondary button opens a menu bringing back those of one scope.

use core::fmt::Write as _;
use koebi_i18n::tr;

use koebi_core::expr::{MILLISECOND, PIXEL, Unit};
use koebi_core::{Color, Rect, Vec2};
use koebi_design::kind::KindId;
use koebi_design::value::Value;
use koebi_design::{ChainRole, ChainStep, DesignSystem, Layers, Origin, Scope, TokenId};
use koebi_render::RectPrimitive;
use koebi_tokens::semantic;
use koebi_ui::{
    Align, CollapsiblePanel, CollapsiblePanelState, MenuItem, MenuState, PointerButton, Ui,
    WidgetId,
};

use super::Stack;
use super::values::{self, Editors, Shape, Slot};

/// The width of an input in the header of an option.
const HEADER_INPUT: f32 = 140.0;

/// The space between the columns of an option.
const COLUMN_GAP: f32 = 12.0;

/// The keys of a column's dot and editor under the column's key: apart, so that nothing the
/// editor holds shares the dot's identity.
const DOT: u64 = 1;
const EDITOR: u64 = 2;

/// What an option edits and shows.
#[derive(Clone, Copy, Debug)]
pub(super) struct OptionDesc {
    /// The token.
    pub(super) token: TokenId,
    /// Its name for the user.
    pub(super) label: &'static str,
    /// The unit its value shows in.
    pub(super) unit: Option<&'static Unit>,
    /// The decimals its value shows with.
    pub(super) decimals: usize,
}

impl OptionDesc {
    /// `token`, named `label`, its unit and decimals those of its kind: pixels with one decimal,
    /// milliseconds, numbers with two decimals, integers and weights without.
    pub(super) fn of(design: &DesignSystem, token: TokenId, label: &'static str) -> Self {
        let kind = design
            .info(token)
            .map(koebi_design::schema::TokenInfo::kind);
        let (unit, decimals) = match kind {
            Some(KindId::Dimension) => (Some(&PIXEL), 1),
            Some(KindId::Duration) => (Some(&MILLISECOND), 0),
            Some(KindId::Number) => (None, 2),
            _ => (None, 0),
        };
        Self {
            token,
            label,
            unit,
            decimals,
        }
    }
}

/// An option and what it keeps between frames.
#[derive(Debug)]
pub(super) struct OptionRow {
    desc: OptionDesc,
    panel: CollapsiblePanelState,
    chain: CollapsiblePanelState,
    /// The menu of the header's dot.
    menu: MenuState,
    /// The steps of the resolution chain, and the text of a value: reused from frame to frame.
    steps: Vec<ChainStep>,
    text: String,
    /// What its editors keep, made the first time it shows.
    editors: Option<Box<Editors>>,
}

impl OptionRow {
    pub(super) fn new(desc: OptionDesc) -> Self {
        Self {
            desc,
            panel: CollapsiblePanelState::new(false),
            chain: CollapsiblePanelState::new(false),
            menu: MenuState::new(),
            steps: Vec::new(),
            text: String::new(),
            editors: None,
        }
    }

    /// What the option edits.
    #[cfg(test)]
    pub(super) fn desc(&self) -> OptionDesc {
        self.desc
    }

    /// Opens or closes the option, and its resolution chain.
    #[cfg(test)]
    pub(super) fn set_open(&mut self, open: bool, chain: bool) {
        self.panel.set_open(open);
        self.chain.set_open(chain);
    }
}

/// A group of options: an expandable panel, open by default, whose options are closed.
#[derive(Debug)]
pub(super) struct OptionGroup {
    title: &'static str,
    panel: CollapsiblePanelState,
    rows: Vec<OptionRow>,
    /// What the options edit, for the header's dot, and its menu.
    tokens: Vec<TokenId>,
    menu: MenuState,
}

impl OptionGroup {
    pub(super) fn new(title: &'static str, options: &[OptionDesc]) -> Self {
        Self {
            title,
            panel: CollapsiblePanelState::new(true),
            rows: options.iter().copied().map(OptionRow::new).collect(),
            tokens: options.iter().map(|option| option.token).collect(),
            menu: MenuState::new(),
        }
    }

    /// What the group's options edit.
    #[cfg(test)]
    pub(super) fn descriptions(&self) -> impl Iterator<Item = OptionDesc> + '_ {
        self.rows.iter().map(|row| row.desc)
    }

    /// The menu of the header's dot.
    #[cfg(test)]
    pub(super) fn menu(&self) -> &MenuState {
        &self.menu
    }

    /// Opens or closes the option at `index`, and its resolution chain.
    #[cfg(test)]
    pub(super) fn open_option(&mut self, index: usize, chain: bool) {
        if let Some(row) = self.rows.get_mut(index) {
            row.panel.set_open(true);
            row.chain.set_open(chain);
        }
    }

    /// Shows the group below what `stack` holds, identified by `id`.
    pub(super) fn show(
        &mut self,
        ui: &mut Ui<'_>,
        stack: &mut Stack,
        id: WidgetId,
        design: &mut DesignSystem,
    ) {
        let Self {
            title,
            panel,
            rows,
            tokens,
            menu,
        } = self;
        let bounds = stack.bounds;
        let group = CollapsiblePanel::new(id, panel, title).trailing(reset_slot(design));
        let shown = stack.panel(ui, group, |ui, rect| {
            let mut y = rect.min.y;
            for (index, row) in rows.iter_mut().enumerate() {
                let at = Rect::from_min_max(Vec2::new(rect.min.x, y), rect.max);
                y += show_option(ui, at, id.with(index as u64 + 1), design, row, bounds);
            }
            y - rect.min.y
        });
        if let Some(trailing) = shown.trailing {
            header_dot(ui, trailing, id.with(0), design, tokens, menu, bounds);
        }
    }
}

/// The side of the place for the dot bringing a value back.
pub(super) fn reset_slot(design: &DesignSystem) -> f32 {
    design.get(semantic::SIZE_CONTROL_HEIGHT)
}

/// What a header's dot brings back: the changes of one scope, or of both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reset {
    /// The changes for the whole application.
    Application,
    /// The changes for the current theme.
    CurrentTheme,
    /// Both.
    Both,
}

impl Reset {
    /// The scopes it brings back.
    const fn scopes(self) -> &'static [Scope] {
        match self {
            Self::Application => &[Scope::Global],
            Self::CurrentTheme => &[Scope::ActiveTheme],
            Self::Both => &[Scope::Global, Scope::ActiveTheme],
        }
    }

    /// The choice of the item at `index` of the dot's menu ([`reset_items`]).
    const fn of_item(index: usize) -> Self {
        match index {
            0 => Self::Application,
            1 => Self::CurrentTheme,
            _ => Self::Both,
        }
    }
}

/// The items of a header's dot's menu, those with nothing to bring back disabled.
fn reset_items(application: bool, theme: bool) -> [MenuItem<'static>; 4] {
    [
        MenuItem::action(tr("Reset for the application")).enabled(application),
        MenuItem::action(tr("Reset for the current theme")).enabled(theme),
        MenuItem::Separator,
        MenuItem::action(tr("Reset both")).enabled(application || theme),
    ]
}

/// Which of `tokens` hold a change: for the application, for the current theme.
pub(super) fn changes(design: &DesignSystem, tokens: &[TokenId]) -> (bool, bool) {
    let changed = |scope| {
        tokens
            .iter()
            .any(|token| design.override_of(scope, *token).is_some())
    };
    (changed(Scope::Global), changed(Scope::ActiveTheme))
}

/// Brings back the changes of `tokens` that `reset` names.
fn reset_tokens(design: &mut DesignSystem, tokens: &[TokenId], reset: Reset) {
    for &scope in reset.scopes() {
        for &token in tokens {
            if design.override_of(scope, token).is_some()
                && let Err(error) = design.reset(scope, token)
            {
                koebi_core::warn!("cannot bring {token} back: {error}");
            }
        }
    }
}

/// The dot at the end of a header whose options edit `tokens`, shown in `area` while any of
/// them holds a change: a click brings back every change, the secondary button opens `menu`,
/// which brings back those of one scope or of both. Menus lie within `bounds`.
pub(super) fn header_dot(
    ui: &mut Ui<'_>,
    area: Rect,
    id: WidgetId,
    design: &mut DesignSystem,
    tokens: &[TokenId],
    menu: &mut MenuState,
    bounds: Rect,
) {
    let (application, theme) = changes(design, tokens);
    let mut chosen = None;
    if application || theme {
        let hint = tr("Reset what changed below; right-click to choose for what");
        let response = ui.reset_dot(id, area, hint);
        if response.clicked {
            chosen = Some(Reset::Both);
        }
        if response.secondary_pressed
            && let Some(point) = ui.input().button_pressed_at(PointerButton::Secondary)
        {
            menu.open_at_pointer(point);
        }
    }
    let items = reset_items(application, theme);
    if let Some(choice) = ui.menu(id.with(1), menu, &items, bounds) {
        chosen = Some(Reset::of_item(choice.index()));
    }
    if let Some(reset) = chosen {
        reset_tokens(design, tokens, reset);
    }
}

/// Shows an option at the top of `area`, its menus within `bounds`. Returns the height it took.
pub(super) fn show_option(
    ui: &mut Ui<'_>,
    area: Rect,
    id: WidgetId,
    design: &mut DesignSystem,
    row: &mut OptionRow,
    bounds: Rect,
) -> f32 {
    let desc = row.desc;
    let slot = reset_slot(design);
    let gap = design.get(semantic::SPACING_EDITOR_INSET) * 0.5;
    let OptionRow {
        panel,
        chain,
        menu,
        steps,
        text,
        editors,
        ..
    } = row;
    let editors = editors.get_or_insert_with(Box::default);
    let shown = CollapsiblePanel::new(id, panel, desc.label)
        .trailing(HEADER_INPUT + gap + slot)
        .show(ui, area, |ui, rect| {
            let mut parts = Parts {
                chain,
                steps,
                text,
                editors,
            };
            option_body(ui, rect, id, design, desc, &mut parts, bounds)
        });
    // Over the header, which would take the press otherwise: the application's value, then
    // the dot bringing back the option's changes.
    if let Some(trailing) = shown.trailing {
        let (field, dot) = field_and_dot(trailing, slot, gap);
        let key = id.with(10).with(EDITOR);
        values::editor(
            ui,
            field,
            key,
            design,
            desc,
            Slot::Header,
            editors,
            text,
            bounds,
        );
        header_dot(ui, dot, id.with(11), design, &[desc.token], menu, bounds);
    }
    shown.height
}

/// What an option's panel keeps: its resolution chain's panel and steps, a text reused from
/// frame to frame, and the state of its editors.
struct Parts<'a> {
    chain: &'a mut CollapsiblePanelState,
    steps: &'a mut Vec<ChainStep>,
    text: &'a mut String,
    editors: &'a mut Editors,
}

/// `rect` cut into the room of a field and, at its end, the place of a dot `slot` wide, `gap`
/// apart.
fn field_and_dot(rect: Rect, slot: f32, gap: f32) -> (Rect, Rect) {
    let dot = Rect::from_min_max(
        Vec2::new((rect.max.x - slot).max(rect.min.x), rect.min.y),
        rect.max,
    );
    let field = Rect::from_min_max(
        rect.min,
        Vec2::new((dot.min.x - gap).max(rect.min.x), rect.max.y),
    );
    (field, dot)
}

/// The two columns of an option, and how its token gets its value. Returns the height used.
fn option_body(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    design: &mut DesignSystem,
    desc: OptionDesc,
    parts: &mut Parts<'_>,
    bounds: Rect,
) -> f32 {
    let Parts {
        chain,
        steps,
        text,
        editors,
    } = parts;
    let padding = ui.styles().panel.padding;
    let row = design.get(semantic::SIZE_CONTROL_HEIGHT);
    let slot = reset_slot(design);
    let inset = design.get(semantic::SPACING_EDITOR_INSET) * 0.5;
    let font = ui.styles().text.label.clone();
    let secondary = design.get(semantic::TEXT_COLOR_SECONDARY_DEFAULT);
    let (left, right) = (rect.min.x + padding, rect.max.x - padding);
    let width = ((right - left - COLUMN_GAP) * 0.5).max(0.0);
    let columns = [
        (left, Scope::Global, None),
        (left + width + COLUMN_GAP, Scope::ActiveTheme, Some(())),
    ];
    let top = rect.min.y;
    let height = values::column_height(ui, design, Shape::of(design, desc.token));
    for (index, (x, scope, theme)) in columns.into_iter().enumerate() {
        let title = Rect::from_origin_size(Vec2::new(x, top), Vec2::new(width, row));
        text.clear();
        if theme.is_some() {
            let name = design.theme(design.active_theme()).map_or("", |theme| {
                koebi_tokens::themes::built_in_name(theme.key()).map_or(theme.name(), tr)
            });
            koebi_i18n::fill_into(text, tr("Current theme: {name}"), &[("name", &name)]);
        } else {
            text.push_str(tr("Application"));
        }
        ui.label(text, title, &font, secondary, Align::Start);
        let input = Rect::from_origin_size(Vec2::new(x, top + row), Vec2::new(width, height));
        let (field, dot) = field_and_dot(input, slot, inset);
        let dot = Rect::from_min_max(dot.min, Vec2::new(dot.max.x, dot.min.y + row));
        // The editor and the dot each under their own key: what an editor holds lies under
        // its key (a curve's graph, a number's step buttons), never the dot's.
        let key = id.with(20 + index as u64);
        let slot = Slot::Column(scope);
        values::editor(
            ui,
            field,
            key.with(EDITOR),
            design,
            desc,
            slot,
            editors,
            text,
            bounds,
        );
        scope_dot(ui, dot, key.with(DOT), design, desc, scope);
    }
    let gap = ui.styles().collapsible_panel.gap;
    let below = top + row + height + gap;
    let panel = Rect::from_min_max(Vec2::new(rect.min.x, below), rect.max);
    let shown = CollapsiblePanel::new(id.with(30), chain, tr("Resolution chain")).show(
        ui,
        panel,
        |ui, inner| {
            if let Some(view) = design.view(design.active_theme(), Layers::ALL) {
                view.chain(desc.token, steps);
            }
            chain_rows(ui, inner, design, desc, steps, text)
        },
    );
    below + shown.height - top
}

/// The dot bringing back the option's value for `scope`, in `area` once `scope` changes it.
fn scope_dot(
    ui: &mut Ui<'_>,
    area: Rect,
    id: WidgetId,
    design: &mut DesignSystem,
    desc: OptionDesc,
    scope: Scope,
) {
    if design.override_of(scope, desc.token).is_none() {
        return;
    }
    let hint = match scope {
        Scope::Global => tr("Reset for the application"),
        Scope::ActiveTheme | Scope::Theme(_) => tr("Reset for the current theme"),
    };
    if ui.reset_dot(id, area, hint).clicked
        && let Err(error) = design.reset(scope, desc.token)
    {
        koebi_core::warn!("cannot bring {} back: {error}", desc.label);
    }
}

/// `value` as the option shows it.
fn write_value(text: &mut String, value: Option<&Value>, desc: OptionDesc) {
    text.clear();
    let Some(value) = value else {
        return;
    };
    let Some(number) = values::number_of(value) else {
        // Colors in hex, easings and curves as their definition, families by name.
        // Writing into a `String` cannot fail.
        let _infallible = write!(text, "{value}");
        return;
    };
    let decimals = desc.decimals;
    // Writing into a `String` cannot fail.
    let _infallible = write!(text, "{number:.decimals$}");
    if let Some(unit) = desc.unit {
        let _infallible = write!(text, " {}", unit.symbol);
    }
}

/// What the origin of a token's definition is called, where it is not the token tables.
fn origin_label(origin: Origin) -> Option<&'static str> {
    match origin {
        Origin::Base => None,
        Origin::Theme => Some(tr("Theme")),
        Origin::ThemeOverride => Some(tr("Current theme")),
        Origin::GlobalOverride => Some(tr("Application")),
    }
}

/// The tokens of a resolution chain, one per row, joined by a line down their marks: what gives
/// the value stands out, what lies past it is grayed. Returns the height used.
fn chain_rows(
    ui: &mut Ui<'_>,
    rect: Rect,
    design: &DesignSystem,
    desc: OptionDesc,
    steps: &[ChainStep],
    text: &mut String,
) -> f32 {
    let padding = ui.styles().panel.padding;
    let row = design.get(semantic::SIZE_CONTROL_HEIGHT);
    let primary = design.get(semantic::TEXT_COLOR_PRIMARY_DEFAULT);
    let secondary = design.get(semantic::TEXT_COLOR_SECONDARY_DEFAULT);
    let disabled = design.get(semantic::TEXT_COLOR_DISABLED_DEFAULT);
    let accent = design.get(semantic::BACKGROUND_ACCENT_DEFAULT);
    let line = design.get(semantic::BORDER_COLOR_DEFAULT);
    let font = ui.styles().text.label.clone();
    let detail = ui.styles().text.detail.clone();
    let mark = (row * 0.4).round();
    let (left, right) = (rect.min.x + padding, rect.max.x - padding);
    let mut y = rect.min.y;
    for (index, step) in steps.iter().enumerate() {
        let band = Rect::from_min_max(Vec2::new(left, y), Vec2::new(right, y + row));
        let (ink, fill) = match step.role {
            ChainRole::Source => (primary, accent),
            ChainRole::Passes => (secondary, secondary),
            ChainRole::Unused => (disabled, disabled),
        };
        if step.role == ChainRole::Source {
            let highlight = accent.with_alpha(0.15);
            ui.scene()
                .push_rect(&RectPrimitive::new(band, highlight).with_corner_radius(row * 0.2));
        }
        // The line down to the next token.
        let center = Vec2::new(left + row * 0.5, y + row * 0.5);
        if index + 1 < steps.len() {
            let stroke = Rect::from_center_size(Vec2::new(center.x, y + row), Vec2::new(1.0, row));
            ui.scene().push_rect(&RectPrimitive::new(stroke, line));
        }
        ui.scene().push_rect(
            &RectPrimitive::new(Rect::from_center_size(center, Vec2::splat(mark)), fill)
                .with_corner_radius(mark * 0.5),
        );
        let start = left + row;
        write_value(text, step.value.as_ref(), desc);
        let value_width = ui.text_width(text, &detail) + padding;
        let value_rect = Rect::from_min_max(
            Vec2::new((right - value_width).max(start), y),
            Vec2::new(right - padding * 0.5, y + row),
        );
        ui.label(text, value_rect, &detail, ink, Align::End);
        let mut end = value_rect.min.x;
        if let Some(origin) = origin_label(step.origin) {
            end = origin_badge(ui, origin, end, y, row, step.role);
        }
        if let Some(info) = design.info(step.token) {
            text.clear();
            // Writing into a `String` cannot fail.
            let _infallible = write!(text, "{}", info.qualified());
            let path = Rect::from_min_max(Vec2::new(start, y), Vec2::new(end.max(start), y + row));
            ui.label(text, path, &font, ink, Align::Start);
        }
        y += row;
    }
    y - rect.min.y
}

/// A small label ending at `end`, telling who gave a token its definition. Returns where it
/// starts.
fn origin_badge(ui: &mut Ui<'_>, label: &str, end: f32, y: f32, row: f32, role: ChainRole) -> f32 {
    let detail = ui.styles().text.detail.clone();
    let padding = (row * 0.3).round();
    let width = ui.text_width(label, &detail) + 2.0 * padding;
    let margin = row * 0.15;
    let badge = Rect::from_min_max(
        Vec2::new(end - width - padding, y + margin),
        Vec2::new(end - padding, y + row - margin),
    );
    let tone = ui.styles().button.colors(koebi_ui::Tone::DEFAULT);
    let color: Color = tone.background.normal;
    let ink = if role == ChainRole::Unused {
        tone.content.disabled
    } else {
        tone.content.normal
    };
    ui.scene()
        .push_rect(&RectPrimitive::new(badge, color).with_corner_radius(badge.height() * 0.5));
    ui.label(label, badge, &detail, ink, Align::Center);
    badge.min.x
}

#[cfg(test)]
mod tests {
    use koebi_tokens::{primitive, semantic};

    use super::*;

    fn design() -> DesignSystem {
        DesignSystem::new(&koebi_tokens::KOEBI).unwrap()
    }

    #[test]
    fn scopes_read_what_they_see() {
        let mut design = design();
        let token = semantic::INPUT_DRAG_THRESHOLD;
        let base = design.get(token);
        let read = |design: &DesignSystem, scope| {
            let value = values::value_in(design, token.id(), scope).unwrap();
            values::number_of(&value).unwrap() - f64::from(base)
        };
        // The application's value goes over the theme's, which keeps its own.
        design.set_value(Scope::Global, token, base + 3.0).unwrap();
        assert!((read(&design, Scope::Global) - 3.0).abs() < 1e-3);
        assert!(read(&design, Scope::ActiveTheme).abs() < 1e-3);
        design
            .set_value(Scope::ActiveTheme, token, base + 1.0)
            .unwrap();
        assert!((read(&design, Scope::ActiveTheme) - 1.0).abs() < 1e-3);
        assert!((read(&design, Scope::Global) - 3.0).abs() < 1e-3);
        design.reset(Scope::Global, token).unwrap();
        assert!((read(&design, Scope::Global) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn values_show_with_their_unit() {
        let desc = OptionDesc {
            token: primitive::DURATION_FAST.id(),
            label: "Fast",
            unit: Some(&MILLISECOND),
            decimals: 0,
        };
        let mut text = String::new();
        write_value(
            &mut text,
            Some(&Value::Duration(core::time::Duration::from_millis(100))),
            desc,
        );
        assert_eq!(text, "100 ms");
        write_value(&mut text, None, desc);
        assert_eq!(text, "");
    }

    /// Frames of one option alone in a window, open, its panel at the top left.
    struct OptionHarness {
        design: DesignSystem,
        state: koebi_ui::UiState,
        input: koebi_ui::Input,
        row: OptionRow,
        time: u64,
    }

    impl OptionHarness {
        const AREA: Rect = Rect::from_min_max(Vec2::new(0.0, 0.0), Vec2::new(700.0, 600.0));

        fn new(token: TokenId) -> Self {
            let design = design();
            let mut state = koebi_ui::UiState::new();
            state.set_fonts(koebi_text::Fonts::with_embedded());
            let mut row = OptionRow::new(OptionDesc::of(&design, token, "Option"));
            row.set_open(true, false);
            Self {
                design,
                state,
                input: koebi_ui::Input::new(),
                row,
                time: 0,
            }
        }

        fn frame(&mut self) {
            let mut scene = koebi_render::Scene::new();
            self.time += 16;
            self.input
                .set_time(core::time::Duration::from_millis(self.time));
            {
                let styles = koebi_ui::Styles::resolve(&self.design);
                let mut ui = Ui::new(&mut scene, &styles, &self.input, &mut self.state);
                let id = WidgetId::new("option");
                show_option(
                    &mut ui,
                    Self::AREA,
                    id,
                    &mut self.design,
                    &mut self.row,
                    Self::AREA,
                );
            }
            self.input.end_frame();
        }

        fn settle(&mut self) {
            for _ in 0..30 {
                self.frame();
            }
        }

        fn click(&mut self, at: Vec2) {
            self.input.pointer_moved(at);
            self.frame();
            self.input.primary_button(true, at);
            self.frame();
            self.input.primary_button(false, at);
            self.frame();
        }

        /// The middle of the dot of the column of `scope`, beside its editor.
        fn column_dot(&self, scope: Scope) -> Vec2 {
            let styles = koebi_ui::Styles::resolve(&self.design);
            let panel = styles.collapsible_panel;
            let padding = styles.panel.padding;
            let row = self.design.get(semantic::SIZE_CONTROL_HEIGHT);
            let slot = reset_slot(&self.design);
            let inner = Self::AREA.max.x - styles.panel.border_width;
            let (left, right) = (styles.panel.border_width + padding, inner - padding);
            let width = ((right - left - COLUMN_GAP) * 0.5).max(0.0);
            let x = match scope {
                Scope::Global => left,
                _ => left + width + COLUMN_GAP,
            };
            let top = panel.header_height + panel.content_padding;
            Vec2::new(x + width - slot * 0.5, top + row * 1.5)
        }
    }

    #[test]
    fn the_dots_beside_every_kind_of_editor_bring_their_value_back() {
        // A curve (its editor's graph), a number (its step buttons), a color.
        let cases = [
            koebi_tokens::component::PANEL_EXPAND_CURVE.id(),
            semantic::INPUT_DRAG_THRESHOLD.id(),
            semantic::BACKGROUND_ACCENT_DEFAULT.id(),
        ];
        for token in cases {
            for scope in [Scope::Global, Scope::ActiveTheme] {
                let mut harness = OptionHarness::new(token);
                let value = harness.design.value(token).cloned().unwrap();
                let changed = match value {
                    Value::Curve(_) => {
                        Value::Curve(std::sync::Arc::new(koebi_design::value::Curve::LINEAR))
                    }
                    Value::Color(_) => Value::Color(Color::from_hex(0x12_34_56)),
                    other => values::value_like(&other, values::number_of(&other).unwrap() + 2.0)
                        .unwrap(),
                };
                harness
                    .design
                    .set_override(
                        scope,
                        token,
                        koebi_design::value::Definition::Value(changed),
                    )
                    .unwrap();
                harness.settle();
                let at = harness.column_dot(scope);
                harness.click(at);
                harness.settle();
                let path = harness.design.info(token).unwrap().qualified();
                assert!(
                    harness.design.override_of(scope, token).is_none(),
                    "{path} for {scope:?}"
                );
            }
        }
    }
}
