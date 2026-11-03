//! Options of the settings: expandable panels editing a numeric token.
//!
//! The header shows the option's name and, at its end, its value for the whole application. The
//! panel opens on the value for the application and for the current theme, side by side, then on
//! how the token gets its value. Beside every input lies a place for the dot bringing the value
//! back ([`Ui::reset_dot`]): it stays empty until the value is changed, and takes no other room
//! then.
//!
//! Headers show a dot too, when anything they hold is changed: the option's for its two scopes,
//! the group's for all its options. A click brings back every change below, for the application
//! and the current theme; the secondary button opens a menu bringing back those of one scope.

use core::fmt::Write as _;
use core::time::Duration;

use koebi_core::expr::Unit;
use koebi_core::{Color, Rect, Vec2};
use koebi_design::value::Definition;
use koebi_design::value::Value;
use koebi_design::{ChainRole, ChainStep, DesignSystem, Layers, Origin, Scope, TokenId};
use koebi_render::RectPrimitive;
use koebi_tokens::semantic;
use koebi_ui::{
    Align, BarMapping, CollapsiblePanel, CollapsiblePanelState, MenuItem, MenuState, NumberField,
    PointerButton, Ui, WidgetId,
};

use super::Stack;

/// The width of an input in the header of an option.
const HEADER_INPUT: f32 = 140.0;

/// The space between the columns of an option.
const COLUMN_GAP: f32 = 12.0;

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
}

impl OptionRow {
    fn new(desc: OptionDesc) -> Self {
        Self {
            desc,
            panel: CollapsiblePanelState::new(false),
            chain: CollapsiblePanelState::new(false),
            menu: MenuState::new(),
            steps: Vec::new(),
            text: String::new(),
        }
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
fn reset_slot(design: &DesignSystem) -> f32 {
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
const fn reset_items(application: bool, theme: bool) -> [MenuItem<'static>; 4] {
    [
        MenuItem::action("Reset for the application").enabled(application),
        MenuItem::action("Reset for the current theme").enabled(theme),
        MenuItem::Separator,
        MenuItem::action("Reset both").enabled(application || theme),
    ]
}

/// Which of `tokens` hold a change: for the application, for the current theme.
fn changes(design: &DesignSystem, tokens: &[TokenId]) -> (bool, bool) {
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
fn header_dot(
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
        let hint = "Reset what changed below; right-click to choose for what";
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
fn show_option(
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
        ..
    } = row;
    let shown = CollapsiblePanel::new(id, panel, desc.label)
        .trailing(HEADER_INPUT + gap + slot)
        .show(ui, area, |ui, rect| {
            option_body(ui, rect, id, design, desc, chain, steps, text)
        });
    // Over the header, which would take the press otherwise: the application's value, then
    // the dot bringing back the option's changes.
    if let Some(trailing) = shown.trailing {
        let (field, dot) = field_and_dot(trailing, slot, gap);
        scoped_field(ui, field, id.with(10), design, desc, Scope::Global);
        header_dot(ui, dot, id.with(11), design, &[desc.token], menu, bounds);
    }
    shown.height
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
#[allow(clippy::too_many_arguments, reason = "the parts of one option")]
fn option_body(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    design: &mut DesignSystem,
    desc: OptionDesc,
    chain: &mut CollapsiblePanelState,
    steps: &mut Vec<ChainStep>,
    text: &mut String,
) -> f32 {
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
    for (index, (x, scope, theme)) in columns.into_iter().enumerate() {
        let title = Rect::from_origin_size(Vec2::new(x, top), Vec2::new(width, row));
        text.clear();
        if theme.is_some() {
            let name = design
                .theme(design.active_theme())
                .map_or("", koebi_design::Theme::name);
            // Writing into a `String` cannot fail.
            let _infallible = write!(text, "Current theme: {name}");
        } else {
            text.push_str("Application");
        }
        ui.label(text, title, &font, secondary, Align::Start);
        let input = Rect::from_origin_size(Vec2::new(x, top + row), Vec2::new(width, row));
        let (field, dot) = field_and_dot(input, slot, inset);
        let key = id.with(20 + index as u64);
        scoped_field(ui, field, key, design, desc, scope);
        scope_dot(ui, dot, key.with(1), design, desc, scope);
    }
    let gap = ui.styles().collapsible_panel.gap;
    let below = top + 2.0 * row + gap;
    let panel = Rect::from_min_max(Vec2::new(rect.min.x, below), rect.max);
    let shown = CollapsiblePanel::new(id.with(30), chain, "Resolution chain").show(
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

/// The value of `token`, as far as `scope` sees it, as a number.
fn value_in(design: &DesignSystem, token: TokenId, scope: Scope) -> Option<f64> {
    let layers = match scope {
        Scope::Global => Layers::ALL,
        Scope::ActiveTheme | Scope::Theme(_) => Layers::THEME.union(Layers::THEME_OVERRIDES),
    };
    let view = design.view(design.active_theme(), layers)?;
    number_of(&view.value(token)?)
}

/// A value as a number: pixels, milliseconds or a plain number.
fn number_of(value: &Value) -> Option<f64> {
    match value {
        Value::Dimension(number) | Value::Number(number) => Some(f64::from(*number)),
        Value::Duration(duration) => Some(duration.as_secs_f64() * 1000.0),
        Value::Integer(number) => Some(f64::from(*number)),
        _ => None,
    }
}

/// `number` as a value of the kind of `like`.
fn value_like(like: &Value, number: f64) -> Option<Value> {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "token values, which their ranges keep inside f32 and i32"
    )]
    match like {
        Value::Dimension(_) => Some(Value::Dimension(number as f32)),
        Value::Number(_) => Some(Value::Number(number as f32)),
        Value::Duration(_) => Some(Value::Duration(Duration::from_secs_f64(
            number.max(0.0) / 1000.0,
        ))),
        Value::Integer(_) => Some(Value::Integer(number.round() as i32)),
        _ => None,
    }
}

/// The field of the option's value for `scope` in `rect`: a number field, on a bar when the
/// token has a range.
fn scoped_field(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    design: &mut DesignSystem,
    desc: OptionDesc,
    scope: Scope,
) {
    let Some(mut number) = value_in(design, desc.token, scope) else {
        return;
    };
    let mut field = NumberField::new(id, &mut number).decimals(desc.decimals);
    if let Some(unit) = desc.unit {
        field = field.unit(unit);
    }
    if let Some(range) = design.range(desc.token) {
        field = field
            .range(range.min(), range.max())
            .bar(BarMapping::Linear);
    }
    if field.show(ui, rect).changed {
        let like = design
            .view(design.active_theme(), Layers::ALL)
            .and_then(|view| view.value(desc.token));
        if let Some(value) = like.and_then(|like| value_like(&like, number))
            && let Err(error) = design.set_override(scope, desc.token, Definition::Value(value))
        {
            koebi_core::warn!("cannot change {}: {error}", desc.label);
        }
    }
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
        Scope::Global => "Reset for the application",
        Scope::ActiveTheme | Scope::Theme(_) => "Reset for the current theme",
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
    let Some(number) = value.and_then(number_of) else {
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
const fn origin_label(origin: Origin) -> Option<&'static str> {
    match origin {
        Origin::Base => None,
        Origin::Theme => Some("Theme"),
        Origin::ThemeOverride => Some("Current theme"),
        Origin::GlobalOverride => Some("Application"),
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
    fn values_turn_into_numbers_and_back_by_kind() {
        let cases = [
            (Value::Dimension(4.0), 4.0),
            (Value::Number(0.5), 0.5),
            (Value::Duration(Duration::from_millis(350)), 350.0),
            (Value::Integer(3), 3.0),
        ];
        for (value, number) in cases {
            assert!((number_of(&value).unwrap() - number).abs() < 1e-3);
            assert_eq!(value_like(&value, number), Some(value));
        }
        assert_eq!(number_of(&Value::Color(Color::WHITE)), None);
        assert_eq!(value_like(&Value::Color(Color::WHITE), 1.0), None);
    }

    #[test]
    fn scopes_read_what_they_see() {
        let mut design = design();
        let token = semantic::INPUT_DRAG_THRESHOLD;
        let base = design.get(token);
        let read = |design: &DesignSystem, scope| {
            value_in(design, token.id(), scope).unwrap() - f64::from(base)
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
            unit: Some(&koebi_core::expr::MILLISECOND),
            decimals: 0,
        };
        let mut text = String::new();
        write_value(
            &mut text,
            Some(&Value::Duration(Duration::from_millis(100))),
            desc,
        );
        assert_eq!(text, "100 ms");
        write_value(&mut text, None, desc);
        assert_eq!(text, "");
    }
}
