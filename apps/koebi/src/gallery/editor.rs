//! The token editor and the color vision modes.
//!
//! The editor writes overrides in the chosen scope: every theme ([`Scope::Global`]) or the
//! active theme only. It reads tokens without the color vision transformation
//! ([`Layers::DEFINITIONS`]), so that editing never bakes a simulation into a value.

use core::ops::RangeInclusive;

use koebi_core::{Color, Oklch, Rect, Vec2};
use koebi_design::kind::{Dimension, Kind};
use koebi_design::tier::Tier;
use koebi_design::{DesignSystem, Layers, Scope, Token};
use koebi_render::{LinePrimitive, RectPrimitive};
use koebi_tokens::{component, semantic};
use koebi_ui::{Icon, PanelLevel, Ui, WidgetId};

use super::{Gallery, Metrics, PALETTE, cut_left, cut_top};
use crate::design_options::COLOR_VISION_MODES;

/// Rows of the token editor: the scope, three accent channels, three dimensions.
pub(super) const ROWS: u16 = 7;

/// The hovered and pressed accent (`accent.color.600`) is this much lighter than the accent
/// (`accent.color.700`), in OKLCH lightness, as in the built-in themes.
const ACCENT_STATE_LIGHTNESS: f32 = 0.05;

/// The value of `token` in the active theme, without the color vision transformation.
fn definition<K: Kind, T: Tier>(design: &DesignSystem, token: Token<K, T>) -> K::Value {
    match design.view(design.active_theme(), Layers::DEFINITIONS) {
        Some(view) => view.get(token),
        None => design.get(token),
    }
}

/// Sets `token` to `value` in `scope`; the controls only produce valid values, so a refusal is
/// a bug worth a warning, not a crash.
fn set<K: Kind, T: Tier>(
    design: &mut DesignSystem,
    scope: Scope,
    token: Token<K, T>,
    value: K::Value,
) {
    if let Err(error) = design.set_value(scope, token, value) {
        koebi_core::warn!("cannot edit {}: {error}", token.id());
    }
}

/// Removes the override of `token` in `scope`.
fn reset(design: &mut DesignSystem, scope: Scope, token: impl Into<koebi_design::TokenId>) {
    if let Err(error) = design.reset(scope, token) {
        koebi_core::warn!("cannot reset a token: {error}");
    }
}

/// A channel of the accent color, edited by a slider beside a strip of its values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Channel {
    Hue,
    Chroma,
    Lightness,
}

impl Channel {
    const ALL: [Self; 3] = [Self::Hue, Self::Chroma, Self::Lightness];

    fn range(self) -> RangeInclusive<f32> {
        match self {
            Self::Hue => 0.0..=360.0,
            // Beyond 0.33, no sRGB color is left at most hues.
            Self::Chroma => 0.0..=0.33,
            Self::Lightness => 0.2..=0.95,
        }
    }

    fn value(self, accent: &mut Oklch) -> &mut f32 {
        match self {
            Self::Hue => &mut accent.h,
            Self::Chroma => &mut accent.c,
            Self::Lightness => &mut accent.l,
        }
    }

    /// Paints the channel's values from one end of its range to the other, the other two
    /// channels staying those of `accent`.
    fn paint(self, ui: &mut Ui<'_>, rect: Rect, accent: Oklch, frame: Color) {
        const STEPS: u8 = 8;
        let range = self.range();
        let width = rect.width() / f32::from(STEPS);
        for step in 0..STEPS {
            let t = (f32::from(step) + 0.5) / f32::from(STEPS);
            let mut sample = accent;
            *self.value(&mut sample) = range.start() + (range.end() - range.start()) * t;
            let cell = Rect::from_origin_size(
                rect.min + Vec2::new(width * f32::from(step), 0.0),
                Vec2::new(width, rect.height()),
            );
            ui.scene()
                .push_rect(&RectPrimitive::new(cell, sample.to_color()));
        }
        ui.scene()
            .push_rect(&RectPrimitive::new(rect, Color::TRANSPARENT).with_border(1.0, frame));
    }
}

/// The override marker at the start of a row: a dot when the row's token is overridden in
/// the editor's scope. Clicking the dot removes the override.
fn badge(ui: &mut Ui<'_>, id: WidgetId, area: Rect, overridden: bool, color: Color) -> bool {
    let response = ui.interact(id, area);
    if overridden {
        let size = area.height() * if response.hovered { 0.45 } else { 0.3 };
        let center = area.center();
        ui.scene()
            .push_line(&LinePrimitive::new(center, center, size, color));
    }
    overridden && response.clicked
}

/// The icon of a scope toggle: four squares for every theme, one for the active theme.
fn scope_icon(ui: &mut Ui<'_>, area: Rect, every_theme: bool, color: Color) {
    let side = (area.height() * 0.5).round();
    let icon = Rect::from_center_size(area.center(), Vec2::splat(side));
    let icon = Rect::from_origin_size(
        Vec2::new(icon.min.x.round(), icon.min.y.round()),
        icon.size(),
    );
    if every_theme {
        let cell = ((side - 2.0) * 0.5).max(1.0);
        for (column, row) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
            let min = icon.min + Vec2::new(column, row) * (cell + 2.0);
            ui.scene().push_rect(
                &RectPrimitive::new(Rect::from_origin_size(min, Vec2::splat(cell)), color)
                    .with_corner_radius(1.0),
            );
        }
    } else {
        ui.scene()
            .push_rect(&RectPrimitive::new(icon, color).with_corner_radius(2.0));
    }
}

/// What a dimension row's preview shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Preview {
    /// A rounded square with the edited radius.
    Radius,
    /// A square with the edited border width.
    Border,
    /// A checked checkbox of the edited size.
    Checkbox,
}

/// The editing step of a dimension within `range`.
fn step_of(range: &koebi_design::value::Range) -> f32 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "token ranges are small numbers of pixels, exact in f32"
    )]
    let (span, step) = ((range.max() - range.min()) as f32, range.step() as f32);
    if step > 0.0 {
        step
    } else if span >= 16.0 {
        1.0
    } else {
        0.25
    }
}

/// A row editing a dimension token with a drag value, beside its override marker and a
/// preview of the value.
fn dimension_row<T: Tier>(
    ui: &mut Ui<'_>,
    design: &mut DesignSystem,
    scope: Scope,
    mut line: Rect,
    token: Token<Dimension, T>,
    preview: Preview,
    metrics: Metrics,
) {
    let id = WidgetId::new("dimension").with(token.id().index() as u64);
    let (row, gap) = (metrics.row, metrics.row_gap());
    let marker = design.get(component::PANEL_OVERRIDE_BADGE_COLOR_DEFAULT);
    let overridden = design.override_of(scope, token).is_some();
    if badge(
        ui,
        id.with(0),
        cut_left(&mut line, row, gap),
        overridden,
        marker,
    ) {
        reset(design, scope, token);
    }
    let value = definition(design, token);
    let area = cut_left(&mut line, 3.0 * row, gap);
    let square = Rect::from_center_size(area.center(), Vec2::splat(row));
    let ink = design.get(semantic::TEXT_COLOR_SECONDARY_DEFAULT);
    let styles = ui.styles();
    match preview {
        Preview::Radius => ui.scene().push_rect(
            &RectPrimitive::new(square, Color::TRANSPARENT)
                .with_corner_radius(value)
                .with_border(styles.panel.border_width.max(1.0), ink),
        ),
        Preview::Border => ui
            .scene()
            .push_rect(&RectPrimitive::new(square, Color::TRANSPARENT).with_border(value, ink)),
        Preview::Checkbox => {
            let style = &styles.checkbox;
            let side = value.min(area.height()).max(0.0);
            let bounds = Rect::from_center_size(area.center(), Vec2::splat(side));
            ui.scene().push_rect(
                &RectPrimitive::new(bounds, style.background_checked.normal)
                    .with_corner_radius(style.corner_radius),
            );
            Icon::Check.draw(ui.scene(), bounds, style.mark, style.mark_width);
        }
    }
    let Some(range) = design.range(token) else {
        return;
    };
    #[allow(
        clippy::cast_possible_truncation,
        reason = "token ranges are small numbers of pixels, exact in f32"
    )]
    let bounds = range.min() as f32..=range.max() as f32;
    let mut edited = value;
    if ui
        .drag_value(id.with(1), line, &mut edited, bounds, step_of(&range))
        .changed
    {
        set(design, scope, token, edited);
    }
}

/// The token editor: the scope, the accent color in OKLCH, and three dimensions.
pub(super) fn tokens(
    gallery: &mut Gallery,
    ui: &mut Ui<'_>,
    design: &mut DesignSystem,
    rect: Rect,
    metrics: Metrics,
) {
    const ID: WidgetId = WidgetId::new("token-editor");
    let mut content = ui.panel(rect, PanelLevel::First);
    let (row, gap) = (metrics.row, metrics.row_gap());

    // The scope: every theme or the active theme, and a button removing its overrides.
    let mut line = cut_top(&mut content, row, gap);
    for (index, scope) in [Scope::Global, Scope::ActiveTheme].into_iter().enumerate() {
        let area = cut_left(&mut line, row, gap);
        let mut on = gallery.scope == scope;
        if ui
            .toggle(ID.with(index as u64), area, &mut on, None)
            .clicked
        {
            gallery.scope = scope;
        }
        let style = &ui.styles().toggle;
        let color = if gallery.scope == scope {
            style.label_selected
        } else {
            style.label
        };
        scope_icon(ui, area, scope == Scope::Global, color);
    }
    let clear = Rect::from_origin_size(Vec2::new(line.max.x - row, line.min.y), Vec2::splat(row));
    if ui.button(ID.with(2), clear, Some(Icon::Close)).clicked
        && let Err(error) = design.reset_scope(gallery.scope)
    {
        koebi_core::warn!("cannot reset the overrides: {error}");
    }

    accent_rows(gallery, ui, design, &mut content, metrics);

    let scope = gallery.scope;
    let line = cut_top(&mut content, row, gap);
    dimension_row(
        ui,
        design,
        scope,
        line,
        semantic::RADIUS_CONTROL,
        Preview::Radius,
        metrics,
    );
    let line = cut_top(&mut content, row, gap);
    dimension_row(
        ui,
        design,
        scope,
        line,
        semantic::BORDER_WIDTH_THIN,
        Preview::Border,
        metrics,
    );
    let line = cut_top(&mut content, row, gap);
    let size = component::CHECKBOX_BOX_SIZE_DEFAULT;
    dimension_row(ui, design, scope, line, size, Preview::Checkbox, metrics);
}

/// Three sliders editing the accent in OKLCH: hue, chroma and lightness. They write
/// `accent.color.700` and its lighter state color `accent.color.600`.
fn accent_rows(
    gallery: &mut Gallery,
    ui: &mut Ui<'_>,
    design: &mut DesignSystem,
    content: &mut Rect,
    metrics: Metrics,
) {
    const ID: WidgetId = WidgetId::new("accent");
    let (row, gap) = (metrics.row, metrics.row_gap());
    // Follow changes made elsewhere: a theme switch, a reset.
    let current = definition(design, semantic::ACCENT_COLOR_700);
    if gallery.accent_source != Some(current) {
        gallery.accent = current.to_oklch();
        gallery.accent_source = Some(current);
    }
    let scope = gallery.scope;
    let overridden = design
        .override_of(scope, semantic::ACCENT_COLOR_700)
        .is_some();
    let marker = design.get(component::PANEL_OVERRIDE_BADGE_COLOR_DEFAULT);
    let frame = design.get(semantic::BORDER_COLOR_DEFAULT);
    let mut accent = gallery.accent;
    let (mut changed, mut cleared) = (false, false);
    for (index, channel) in Channel::ALL.into_iter().enumerate() {
        let id = ID.with(index as u64);
        let mut line = cut_top(content, row, gap);
        cleared |= badge(
            ui,
            id.with(0),
            cut_left(&mut line, row, gap),
            overridden,
            marker,
        );
        channel.paint(ui, cut_left(&mut line, 3.0 * row, gap), accent, frame);
        changed |= ui
            .slider(
                id.with(1),
                line,
                channel.value(&mut accent),
                channel.range(),
            )
            .changed;
    }
    if cleared {
        reset(design, scope, semantic::ACCENT_COLOR_700);
        reset(design, scope, semantic::ACCENT_COLOR_600);
    } else if changed {
        gallery.accent = accent;
        let color = accent.to_color();
        let lighter = Oklch {
            l: (accent.l + ACCENT_STATE_LIGHTNESS).min(1.0),
            ..accent
        };
        set(design, scope, semantic::ACCENT_COLOR_700, color);
        set(
            design,
            scope,
            semantic::ACCENT_COLOR_600,
            lighter.to_color(),
        );
        gallery.accent_source = Some(color);
    }
}

/// The color vision modes: one toggle each, previewing the mode on four chart colors.
pub(super) fn color_vision(
    ui: &mut Ui<'_>,
    design: &mut DesignSystem,
    rect: Rect,
    metrics: Metrics,
) {
    const ID: WidgetId = WidgetId::new("color-vision");
    let mut content = ui.panel(rect, PanelLevel::First);
    let (row, gap) = (metrics.row, metrics.row_gap());
    let samples = [0, 1, 2, 3].map(|index| definition(design, PALETTE[index]));
    let current = design.color_vision();
    // Normal, then four simulations, then three corrections, with a wider gap between groups.
    let count = COLOR_VISION_MODES.len() as f32;
    let width = ((content.width() - gap * (count + 1.0)) / count)
        .clamp(0.0, 3.0 * row)
        .round();
    let mut chosen = None;
    for (index, (_, mode)) in COLOR_VISION_MODES.into_iter().enumerate() {
        if index == 1 || index == 5 {
            cut_left(&mut content, 0.0, gap);
        }
        let area = cut_left(&mut content, width, gap);
        let mut on = current == mode;
        if ui
            .toggle(ID.with(index as u64), area, &mut on, None)
            .clicked
        {
            chosen = Some(mode);
        }
        let strip = area.inset((row * 0.25).round());
        if strip.is_empty() {
            continue;
        }
        let matrix = mode.matrix();
        let cell = strip.width() / samples.len() as f32;
        for (position, color) in samples.into_iter().enumerate() {
            let min = strip.min + Vec2::new(cell * position as f32, 0.0);
            ui.scene().push_rect(&RectPrimitive::new(
                Rect::from_origin_size(min, Vec2::new(cell, strip.height())),
                matrix.apply(color),
            ));
        }
    }
    if let Some(mode) = chosen
        && let Err(error) = design.set_color_vision(mode)
    {
        koebi_core::warn!("cannot change the color vision setting: {error}");
    }
}
