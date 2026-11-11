//! Settings > Icons: every icon of `koebi-icons` by category, and a preview beside them of the
//! one picked, large and small, in each state and tone the interface gives icons, in its own
//! colors, two-toned or not, its zones recolored one by one for the preview alone. Below, the
//! icon tokens, editable like any other.

use koebi_core::{Color, Rect, Vec2};
use koebi_design::{DesignSystem, TokenId};
use koebi_i18n::{fill, tr};
use koebi_icons::{IconId, Role};
use koebi_render::{ICON_ZONES, IconColors, IconPrimitive, RectPrimitive};
use koebi_tokens::{component, primitive, semantic};
use koebi_ui::{
    Align, Button, ButtonGroup, ColorField, ColorFieldState, ControlState, GroupCell, PanelLevel,
    Ui, WidgetId, icon_bounds,
};

use super::Stack;
use super::options::{self, OptionDesc, OptionRow};

const ID: WidgetId = WidgetId::new("settings-icons");

/// Side of an icon in the grid, and the space around it in its cell, in logical pixels.
const GRID_ICON: f32 = 24.0;
const GRID_PADDING: f32 = 8.0;
/// Side of the large icon of the preview.
const LARGE: f32 = 96.0;
/// The sizes the preview also shows the icon at.
const SMALL: [f32; 3] = [16.0, 24.0, 32.0];
/// Width of the preview beside the grid, at most.
const PREVIEW_WIDTH: f32 = 340.0;

/// How the preview paints the icon: in a state of a control, in a tone, or in its own colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Look {
    State(ControlState),
    Accent,
    Negative,
    Notice,
    Neutral,
    Yellow,
    Original,
}

/// Every look, in the order of the preview's buttons.
const LOOKS: [Look; 11] = [
    Look::State(ControlState::Normal),
    Look::State(ControlState::Hovered),
    Look::State(ControlState::Pressed),
    Look::State(ControlState::Active),
    Look::State(ControlState::Disabled),
    Look::Accent,
    Look::Negative,
    Look::Notice,
    Look::Neutral,
    Look::Yellow,
    Look::Original,
];

impl Look {
    fn label(self) -> &'static str {
        match self {
            Self::State(ControlState::Hovered) => tr("Hovered"),
            Self::State(ControlState::Pressed) => tr("Pressed"),
            Self::State(ControlState::Active) => tr("Active"),
            Self::State(ControlState::Disabled) => tr("Disabled"),
            Self::State(_) => tr("Normal"),
            Self::Accent => tr("Accent"),
            Self::Negative => tr("Negative"),
            Self::Notice => tr("Notice"),
            Self::Neutral => tr("Neutral"),
            Self::Yellow => tr("Yellow"),
            Self::Original => tr("Original colors"),
        }
    }

    /// The main color of the icon in this look; `None` in its own colors.
    fn color(self, ui: &Ui<'_>, design: &DesignSystem) -> Option<Color> {
        Some(match self {
            Self::State(state) => ui.styles().button.variants[0].icon.get(state),
            Self::Accent => design.get(semantic::ACCENT_COLOR_1300),
            Self::Negative => design.get(semantic::NEGATIVE_COLOR_1300),
            Self::Notice => design.get(semantic::NOTICE_COLOR_1300),
            Self::Neutral => design.get(semantic::TEXT_COLOR_SECONDARY_DEFAULT),
            Self::Yellow => design.get(primitive::COLOR_YELLOW_1300),
            Self::Original => return None,
        })
    }
}

/// The name of a category of icons, as the page titles it.
fn category_title(category: &str) -> &str {
    match category {
        "actions" => tr("Actions"),
        "app" => tr("Application"),
        "cursors" => tr("Cursors"),
        "editors" => tr("Editors"),
        "input" => tr("Input"),
        "interface" => tr("Interface"),
        "navigation" => tr("Navigation"),
        "objects" => tr("Objects"),
        "settings" => tr("Settings"),
        "shapes" => tr("Shapes"),
        "text" => tr("Text"),
        "tools" => tr("Tools"),
        "transform" => tr("Transform"),
        "view" => tr("View"),
        other => other,
    }
}

/// The name of a zone's role.
fn role_name(role: Role) -> &'static str {
    match role {
        Role::Primary => tr("Main ink"),
        Role::Secondary => tr("Secondary ink"),
        Role::Tertiary => tr("Third ink"),
        Role::Accent => tr("Accent ink"),
    }
}

/// The Icons page's state: the icon picked and how its preview shows it.
#[derive(Debug)]
pub(super) struct IconsPage {
    selected: Option<IconId>,
    look: Look,
    two_tones: bool,
    /// The colors given to the zones of the picked icon, for the preview alone.
    zones: [Option<Color>; ICON_ZONES],
    /// The colors its file gives its zones.
    original: [Color; ICON_ZONES],
    fields: [ColorFieldState; ICON_ZONES],
    /// The icon tokens, built once.
    tokens: Option<Vec<OptionRow>>,
}

impl IconsPage {
    pub(super) fn new() -> Self {
        Self {
            selected: None,
            look: Look::State(ControlState::Normal),
            two_tones: true,
            zones: [None; ICON_ZONES],
            original: [Color::BLACK; ICON_ZONES],
            fields: core::array::from_fn(|_| ColorFieldState::new()),
            tokens: None,
        }
    }

    /// Picks `icon`: its zones' colors are its own again.
    pub(super) fn select(&mut self, icon: IconId) {
        self.selected = Some(icon);
        self.zones = [None; ICON_ZONES];
        self.original = [Color::BLACK; ICON_ZONES];
        for (slot, zone) in self.original.iter_mut().zip(icon.compile().zones) {
            let [r, g, b, a] = zone.color.0;
            *slot = Color::new(r, g, b, a);
        }
    }

    /// The colors the preview paints the picked icon's zones in.
    fn colors(&self, icon: IconId, ui: &Ui<'_>, design: &DesignSystem) -> IconColors {
        let mut colors = [None; ICON_ZONES];
        if let Some(main) = self.look.color(ui, design) {
            let style = ui.styles().icon;
            let ink = if self.two_tones {
                style.ink(main)
            } else {
                style.one_tone(main)
            };
            for (zone, &role) in colors.iter_mut().zip(icon.roles()) {
                *zone = Some(ink.of(role));
            }
        }
        for (zone, chosen) in colors.iter_mut().zip(self.zones) {
            if chosen.is_some() {
                *zone = chosen;
            }
        }
        IconColors::Zones(colors)
    }

    pub(super) fn show(&mut self, ui: &mut Ui<'_>, stack: &mut Stack, design: &mut DesignSystem) {
        let row = ui.styles().button.height;
        let gap = stack.gap;
        let title = Rect::from_min_max(
            Vec2::new(stack.left, stack.y),
            Vec2::new(stack.right, stack.y + row),
        );
        ui.section_title(title, tr("Icons"));
        stack.y += row + gap;
        let width = stack.right - stack.left;
        let preview_width = if self.selected.is_some() {
            PREVIEW_WIDTH.min(width * 0.45)
        } else {
            0.0
        };
        let grid_right = if preview_width > 0.0 {
            stack.right - preview_width - gap
        } else {
            stack.right
        };
        let top = stack.y;
        let grid_bottom = self.grid(
            ui,
            Rect::from_min_max(Vec2::new(stack.left, top), Vec2::new(grid_right, top)),
            gap,
        );
        let preview_bottom = match self.selected {
            Some(icon) if preview_width > 0.0 => {
                let area = Rect::from_min_max(
                    Vec2::new(stack.right - preview_width, top),
                    Vec2::new(stack.right, top),
                );
                self.preview(ui, area, icon, design, stack.bounds)
            }
            _ => top,
        };
        stack.y = grid_bottom.max(preview_bottom) + gap;
        self.tokens_section(ui, stack, design);
    }

    /// The icons by category from the top of `area`, as wide as it is; returns the y below
    /// them.
    fn grid(&mut self, ui: &mut Ui<'_>, area: Rect, gap: f32) -> f32 {
        let row = ui.styles().button.height;
        let font = ui.styles().text.label.clone();
        let (primary, secondary) = (ui.styles().text.primary, ui.styles().text.secondary);
        let selection = ui.styles().list.selected;
        let accent = ui.styles().icon.accent;
        let cell = GRID_ICON + 2.0 * GRID_PADDING;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a few dozen cells across a page, floored at one"
        )]
        let columns = ((area.width() / cell).floor() as usize).max(1);
        let mut y = area.min.y;
        let mut category = "";
        let mut index = 0_usize;
        for icon in IconId::all() {
            if icon.category() != category {
                if !category.is_empty() {
                    y += index.div_ceil(columns) as f32 * cell + gap;
                }
                category = icon.category();
                index = 0;
                let title =
                    Rect::from_min_max(Vec2::new(area.min.x, y), Vec2::new(area.max.x, y + row));
                ui.label(
                    category_title(category),
                    title,
                    &font,
                    secondary,
                    Align::Start,
                );
                y += row;
            }
            let at = Vec2::new(
                area.min.x + (index % columns) as f32 * cell,
                y + (index / columns) as f32 * cell,
            );
            index += 1;
            let bounds = Rect::from_origin_size(at, Vec2::splat(cell));
            let id = ID.with(1000 + u64::from(icon.index()));
            let response = ui.interact(id, bounds);
            if self.selected == Some(icon) {
                ui.scene()
                    .push_rect(&RectPrimitive::new(bounds, selection).with_corner_radius(4.0));
            }
            let color = if response.hovered { accent } else { primary };
            ui.icon(icon_bounds(bounds, GRID_ICON), icon, color);
            ui.tooltip(id, icon.name(), bounds);
            if response.clicked {
                self.select(icon);
            }
        }
        y + index.div_ceil(columns) as f32 * cell
    }

    /// The preview of `icon` from the top of `area`; returns the y below it.
    fn preview(
        &mut self,
        ui: &mut Ui<'_>,
        area: Rect,
        icon: IconId,
        design: &DesignSystem,
        bounds: Rect,
    ) -> f32 {
        let row = ui.styles().button.height;
        let gap = ui.styles().property_group.row_gap;
        let panel = &ui.styles().panel;
        let inset = panel.padding + panel.border_width;
        let roles = icon.roles();
        let looks = LOOKS.len().div_ceil(2);
        let height = 2.0 * inset
            + LARGE
            + gap
            + 2.0 * row
            + gap
            + SMALL[2]
            + gap
            + looks as f32 * row
            + gap
            + row
            + gap
            + (roles.len() as f32) * (row + gap)
            + row;
        let rect = Rect::from_min_max(area.min, Vec2::new(area.max.x, area.min.y + height));
        let content = ui.panel(rect, PanelLevel::First);
        let colors = self.colors(icon, ui, design);
        let font = ui.styles().text.label.clone();
        let detail = ui.styles().text.detail.clone();
        let (primary, secondary) = (ui.styles().text.primary, ui.styles().text.secondary);
        let mut y = content.min.y;

        // The icon, large, then its name and where developers find it.
        let large = Rect::from_origin_size(
            Vec2::new(content.center().x - LARGE * 0.5, y),
            Vec2::splat(LARGE),
        );
        push_icon(ui, large, icon, colors);
        y += LARGE + gap;
        let line = |y: f32| {
            Rect::from_min_max(
                Vec2::new(content.min.x, y),
                Vec2::new(content.max.x, y + row),
            )
        };
        ui.label(icon.name(), line(y), &font, primary, Align::Center);
        let constant = constant_of(icon);
        ui.label(&constant, line(y + row), &detail, secondary, Align::Center);
        y += 2.0 * row + gap;

        // The same at the sizes of the interface, side by side.
        let total: f32 = SMALL.iter().sum::<f32>() + 2.0 * gap;
        let mut x = content.center().x - total * 0.5;
        for size in SMALL {
            let at = Rect::from_origin_size(Vec2::new(x, y + SMALL[2] - size), Vec2::splat(size));
            push_icon(ui, at, icon, colors);
            x += size + gap;
        }
        y += SMALL[2] + gap;

        // The looks, two by row.
        let grid = Rect::from_min_max(
            Vec2::new(content.min.x, y),
            Vec2::new(content.max.x, y + looks as f32 * row),
        );
        let mut cells: [GroupCell<'_>; LOOKS.len()] = core::array::from_fn(|index| {
            GroupCell::new(
                Button::new(ID.with(10 + index as u64))
                    .label(LOOKS[index].label())
                    .selected(self.look == LOOKS[index]),
            )
            .at(
                u16::from(index % 2 == 1),
                u16::try_from(index / 2).unwrap_or(0),
            )
        });
        if let Some(index) = ButtonGroup::horizontal().show(ui, grid, &mut cells).clicked
            && let Some(&look) = LOOKS.get(index)
        {
            self.look = look;
        }
        y += looks as f32 * row + gap;
        let tones = line(y);
        if Button::new(ID.with(30))
            .label(tr("Two tones"))
            .selected(self.two_tones)
            .enabled(self.look != Look::Original)
            .show(ui, tones)
            .clicked
        {
            self.two_tones = !self.two_tones;
        }
        y += row + gap;

        // Its zones, each recolored for the preview alone.
        let painted = match colors {
            IconColors::Zones(zones) => zones,
            IconColors::Original => [None; ICON_ZONES],
        };
        for (zone, &role) in roles.iter().enumerate().take(ICON_ZONES) {
            let at = line(y);
            let label = Rect::from_min_max(at.min, Vec2::new(at.center().x, at.max.y));
            let field = Rect::from_min_max(Vec2::new(at.center().x, at.min.y), at.max);
            let name = fill(
                tr("Zone {number}: {role}"),
                &[("number", &(zone + 1)), ("role", &role_name(role))],
            );
            ui.label(&name, label, &font, secondary, Align::Start);
            let mut color = self.zones[zone]
                .or(painted[zone])
                .unwrap_or(self.original[zone]);
            if ColorField::new(
                ID.with(40 + zone as u64),
                &mut color,
                &mut self.fields[zone],
            )
            .original(self.original[zone])
            .show(ui, field, bounds)
            .changed
            {
                self.zones[zone] = Some(color);
            }
            y += row + gap;
        }
        if Button::new(ID.with(60))
            .label(tr("Colors of the look"))
            .enabled(self.zones.iter().any(Option::is_some))
            .show(ui, line(y))
            .clicked
        {
            self.zones = [None; ICON_ZONES];
        }
        rect.max.y
    }

    /// The icon tokens: their values for the application and the current theme.
    fn tokens_section(&mut self, ui: &mut Ui<'_>, stack: &mut Stack, design: &mut DesignSystem) {
        let row = ui.styles().button.height;
        let title = Rect::from_min_max(
            Vec2::new(stack.left, stack.y),
            Vec2::new(stack.right, stack.y + row),
        );
        ui.section_title(title, tr("Icon tokens"));
        stack.y += row + stack.gap;
        let rows = self.tokens.get_or_insert_with(|| {
            let group = component::ICON::GROUP;
            group
                .tokens()
                .iter()
                .zip(group.start()..)
                .map(|(info, index)| {
                    let token = TokenId::from_index(index);
                    OptionRow::new(OptionDesc::of(design, token, info.path()))
                })
                .collect()
        });
        for (index, option) in rows.iter_mut().enumerate() {
            let at = Rect::from_min_max(
                Vec2::new(stack.left, stack.y),
                Vec2::new(stack.right, stack.y + 1.0e6),
            );
            let key = ID.with(2000 + index as u64);
            stack.y += options::show_option(ui, at, key, design, option, stack.bounds) + stack.gap;
        }
    }
}

/// Paints `icon` in `bounds` in `colors`.
fn push_icon(ui: &mut Ui<'_>, bounds: Rect, icon: IconId, colors: IconColors) {
    ui.scene().push_icon(&IconPrimitive {
        rect: bounds,
        icon: icon.index(),
        colors,
        opacity: 1.0,
        rotation: 0.0,
    });
}

/// The constant developers name `icon` by: `koebi_icons::settings::GENERAL`.
fn constant_of(icon: IconId) -> String {
    let name = icon.name();
    let short = name.rsplit('/').next().unwrap_or(name);
    format!(
        "koebi_icons::{}::{}",
        icon.category(),
        short.replace('-', "_").to_uppercase()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_shows_every_icon_and_the_preview_of_the_one_picked() {
        let Some(gpu) = koebi_testkit::gpu_or_skip("the_page_shows_every_icon_and_the_preview")
        else {
            return;
        };
        let mut harness = crate::settings::tests::Harness::new();
        harness.settings.page = crate::settings::Page::Icons;
        let page = &mut harness.settings.icons;
        page.select(koebi_icons::actions::SAVE_AS);
        page.look = Look::Accent;
        // The secondary ink recolored, for the preview alone.
        page.zones[1] = Some(Color::from_hex(0xE0_4F_5F));
        let image = crate::settings::tests::page_image(&gpu, &mut harness);
        crate::settings::tests::assert_page(&image, "settings_icons");
        gpu.assert_no_validation_errors();
    }

    #[test]
    fn looks_paint_the_zones_by_role_and_the_zones_given_win() {
        let design = DesignSystem::new(&koebi_tokens::KOEBI).unwrap();
        let mut harness = crate::settings::tests::Harness::new();
        let mut page = IconsPage::new();
        page.select(koebi_icons::actions::SAVE_AS);
        let mut scene = koebi_render::Scene::new();
        let styles = harness.styles.update(&harness.design, 1.0);
        let ui = Ui::new(&mut scene, styles, &harness.input, &mut harness.state);
        let accent = design.get(semantic::ACCENT_COLOR_1300);
        page.look = Look::Accent;
        let IconColors::Zones(zones) = page.colors(koebi_icons::actions::SAVE_AS, &ui, &design)
        else {
            panic!("an interface look recolors every zone");
        };
        // The main ink in the accent, the secondary one fainter.
        assert_eq!(zones[0], Some(accent));
        assert_ne!(zones[1], Some(accent));
        page.zones[0] = Some(Color::WHITE);
        page.look = Look::Original;
        let IconColors::Zones(zones) = page.colors(koebi_icons::actions::SAVE_AS, &ui, &design)
        else {
            panic!("zones given are painted");
        };
        assert_eq!(zones[0], Some(Color::WHITE));
        assert_eq!(zones[1], None);
    }

    #[test]
    fn a_constant_is_named_after_the_file() {
        let icon = koebi_icons::settings::GENERAL;
        assert_eq!(constant_of(icon), "koebi_icons::settings::GENERAL");
        assert_eq!(
            constant_of(koebi_icons::navigation::ARROW_DOWNWARD_ALT),
            "koebi_icons::navigation::ARROW_DOWNWARD_ALT"
        );
    }

    #[test]
    fn picking_an_icon_reads_its_own_colors_and_forgets_the_zones_given() {
        let mut page = IconsPage::new();
        page.select(koebi_icons::actions::SAVE);
        page.zones[0] = Some(Color::WHITE);
        page.select(koebi_icons::actions::SAVE_AS);
        assert_eq!(page.zones, [None; ICON_ZONES]);
        // Save As has a main and a secondary ink, of two grays.
        assert_ne!(page.original[0], page.original[1]);
    }
}
