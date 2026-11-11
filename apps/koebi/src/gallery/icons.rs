//! The icons of the gallery: every vector icon of `koebi-icons`, drawn from its curves at the
//! size the field sets, in the interface's colors (zones recolored by role; a hovered icon takes
//! the accent, a pressed one the secondary text color) or in the colors of its file. Icons with
//! a secondary ink show it fainter (two tones) or in the main color (one tone). Its name shows
//! in a tooltip.

use koebi_core::{Rect, Vec2};
use koebi_i18n::tr;
use koebi_icons::IconId;
use koebi_ui::{
    Align, BarMapping, Button, IconPaint, NumberField, PanelLevel, Ui, WidgetId, paint_icon,
};

use super::{Metrics, cut_left, cut_top, fit};

/// The sizes the field offers, in logical pixels.
const SIZES: (f32, f32) = (8.0, 96.0);

/// Space around an icon in its cell, in logical pixels.
const PADDING: f32 = 6.0;

/// What the panel keeps: the size icons are drawn at and how they are colored.
#[derive(Debug)]
pub(super) struct Icons {
    size: f32,
    original: bool,
    /// Secondary inks fainter than the main one (two tones), or in it (one tone).
    two_tones: bool,
}

impl Icons {
    pub(super) fn new() -> Self {
        Self {
            size: 24.0,
            original: false,
            two_tones: true,
        }
    }

    fn cell(&self) -> f32 {
        self.size + 2.0 * PADDING
    }

    /// How many icons a row of `width` holds.
    fn columns(&self, width: f32) -> usize {
        // At least one: a narrow window shows a column.
        fit(width, self.cell(), 0.0).max(1)
    }

    /// The height of the panel `width` wide: a row of settings, then the icons.
    pub(super) fn height(&self, ui: &Ui<'_>, width: f32, metrics: Metrics) -> f32 {
        let panel = &ui.styles().panel;
        let inset = 2.0 * (panel.padding + panel.border_width);
        let rows = IconId::all().count().div_ceil(self.columns(width - inset));
        metrics.row + metrics.row_gap() + rows as f32 * self.cell() + inset
    }

    /// Draws the panel in `rect`.
    pub(super) fn show(&mut self, ui: &mut Ui<'_>, rect: Rect, metrics: Metrics) {
        const ID: WidgetId = WidgetId::new("icons");
        let mut content = ui.panel(rect, PanelLevel::First);
        let (row, gap) = (metrics.row, metrics.row_gap());
        let font = ui.styles().text.label.clone();
        let (primary, secondary) = (ui.styles().text.primary, ui.styles().text.secondary);

        let mut line = cut_top(&mut content, row, gap);
        let title = cut_left(&mut line, 8.0 * row, gap);
        ui.label(tr("Vector icons"), title, &font, secondary, Align::Start);
        NumberField::new(ID.with(1), &mut self.size)
            .label(tr("Size"))
            .range(SIZES.0, SIZES.1)
            .step(1.0)
            .bar(BarMapping::Linear)
            .show(ui, cut_left(&mut line, 8.0 * row, gap));
        if Button::new(ID.with(2))
            .label(tr("Original colors"))
            .selected(self.original)
            .show(ui, cut_left(&mut line, 6.0 * row, gap))
            .clicked
        {
            self.original = !self.original;
        }
        let tones = cut_left(&mut line, 5.0 * row, gap);
        if Button::new(ID.with(3))
            .label(tr("Two tones"))
            .selected(self.two_tones)
            .enabled(!self.original)
            .show(ui, tones)
            .clicked
        {
            self.two_tones = !self.two_tones;
        }
        ui.tooltip(
            ID.with(3),
            tr(
                "Icons with a secondary part (what lies behind, inside, or what an action \
                applies to) show it fainter; off, in the main color",
            ),
            tones,
        );

        let style = ui.styles().icon;
        let columns = self.columns(content.width());
        let cell = self.cell();
        for (index, icon) in IconId::all().enumerate() {
            let at = Vec2::new(
                content.min.x + (index % columns) as f32 * cell,
                content.min.y + (index / columns) as f32 * cell,
            );
            let bounds = Rect::from_origin_size(at, Vec2::splat(cell));
            let id = ID.with(100 + u64::from(icon.index()));
            let response = ui.interact(id, bounds);
            let color = if response.held {
                secondary
            } else if response.hovered {
                style.accent
            } else {
                primary
            };
            let paint = if self.original {
                IconPaint::Original
            } else if self.two_tones {
                IconPaint::Ink(style.ink(color))
            } else {
                IconPaint::Ink(style.one_tone(color))
            };
            let square = Rect::from_origin_size(at + Vec2::splat(PADDING), Vec2::splat(self.size));
            paint_icon(ui.scene(), square, icon, paint, 0.0);
            ui.tooltip(id, icon.name(), bounds);
        }
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::{Color, Extent};
    use koebi_design::DesignSystem;
    use koebi_render::{Renderer2D, Scene};
    use koebi_text::Fonts;
    use koebi_tokens::KOEBI;
    use koebi_ui::{Input, StyleCache, UiState};

    use super::*;

    const AREA: Vec2 = Vec2::new(720.0, 600.0);

    /// Two frames of the panel filling [`AREA`], `input` on the second (hit testing uses the
    /// frame before), and the design they were drawn with.
    fn frame(icons: &mut Icons, input: &Input) -> (Scene, DesignSystem) {
        let design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
        let metrics = Metrics::resolve(&design);
        let mut scene = Scene::new();
        let mut state = UiState::new();
        state.set_fonts(Fonts::with_embedded());
        let cache = StyleCache::new(&design);
        for input in [input.settled(), *input] {
            scene.clear();
            let mut ui = Ui::new(&mut scene, cache.styles(), &input, &mut state);
            icons.show(&mut ui, Rect::from_origin_size(Vec2::ZERO, AREA), metrics);
        }
        (scene, design)
    }

    /// Where the panel's first icon cell lies: after the panel's inset and the row of settings.
    fn first_cell(design: &DesignSystem) -> Vec2 {
        let cache = StyleCache::new(design);
        let panel = &cache.styles().panel;
        let metrics = Metrics::resolve(design);
        let inset = panel.padding + panel.border_width;
        Vec2::new(inset, inset + metrics.row + metrics.row_gap())
    }

    #[test]
    fn every_icon_is_drawn_once() {
        let mut icons = Icons::new();
        let (scene, _) = frame(&mut icons, &Input::new());
        assert_eq!(scene.icon_count(), koebi_icons::COUNT);
    }

    #[test]
    fn the_hovered_icon_takes_the_accent() {
        let Some(gpu) = koebi_testkit::gpu_or_skip("the_hovered_icon_takes_the_accent") else {
            return;
        };
        let mut icons = Icons::new();
        icons.size = 48.0;
        let close = koebi_icons::actions::CLOSE.index() as usize;
        let (_, design) = frame(&mut icons, &Input::new());
        let columns = icons.columns(AREA.x - 2.0 * first_cell(&design).x);
        let cell = icons.cell();
        let center = first_cell(&design)
            + Vec2::new(
                (close % columns) as f32 * cell + cell * 0.5,
                (close / columns) as f32 * cell + cell * 0.5,
            );
        let render = |input: &Input, icons: &mut Icons| {
            let (scene, _) = frame(icons, input);
            let format = koebi_gpu::TextureFormat::Rgba8Unorm;
            let mut renderer = Renderer2D::new(&gpu.device, format).expect("renderer creation");
            renderer.set_fonts(Fonts::with_embedded());
            renderer
                .set_icons(koebi_icons::packed())
                .expect("a valid icon set");
            gpu.render_offscreen(Extent::new(720, 600), format, |encoder, target| {
                renderer
                    .render(encoder, target, &scene, 0, 1.0)
                    .expect("render");
            })
            .expect("offscreen rendering")
        };
        let (x, y) = (center.x.floor(), center.y.floor());
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a pixel inside the image"
        )]
        let (x, y) = (x as u32, y as u32);
        let at = |image: &koebi_testkit::Image| image.pixel(x, y);
        let resting = render(&Input::new(), &mut icons);
        let text: Color = design.get(koebi_tokens::semantic::TEXT_COLOR_PRIMARY_DEFAULT);
        let accent: Color = design.get(koebi_tokens::component::ICON_ZONE_ACCENT_COLOR);
        let close_enough = |pixel: [u8; 4], color: Color| {
            pixel
                .iter()
                .zip(color.to_rgba8())
                .all(|(a, b)| a.abs_diff(b) <= 2)
        };
        // The cross's center, in the color of text.
        assert!(close_enough(at(&resting), text), "{:?}", at(&resting));
        let mut input = Input::new();
        input.pointer_moved(center);
        let hovered = render(&input, &mut icons);
        assert!(close_enough(at(&hovered), accent), "{:?}", at(&hovered));
        // In its own colors: the file's blue.
        icons.original = true;
        let original = render(&Input::new(), &mut icons);
        assert!(
            close_enough(at(&original), Color::from_hex(0x001a_73e8)),
            "{:?}",
            at(&original)
        );
        gpu.assert_no_validation_errors();
    }

    #[test]
    fn two_tones_show_the_secondary_part_fainter_and_one_tone_in_the_main_color() {
        let Some(gpu) = koebi_testkit::gpu_or_skip(
            "two_tones_show_the_secondary_part_fainter_and_one_tone_in_the_main_color",
        ) else {
            return;
        };
        // Small enough for every row to fit the image.
        let mut icons = Icons::new();
        icons.size = 24.0;
        let shadow = koebi_icons::view::SHADOW.index() as usize;
        let (_, design) = frame(&mut icons, &Input::new());
        let columns = icons.columns(AREA.x - 2.0 * first_cell(&design).x);
        let cell = icons.cell();
        let square = first_cell(&design)
            + Vec2::new(
                (shadow % columns) as f32 * cell + PADDING,
                (shadow / columns) as f32 * cell + PADDING,
            );
        // The middle of the shadow, the strip left of the square: x 80..240, y 240..720 of 960.
        let point = square + Vec2::new(icons.size * 160.0 / 960.0, icons.size * 0.5);
        let render = |icons: &mut Icons| {
            let (scene, _) = frame(icons, &Input::new());
            let format = koebi_gpu::TextureFormat::Rgba8Unorm;
            let mut renderer = Renderer2D::new(&gpu.device, format).expect("renderer creation");
            renderer.set_fonts(Fonts::with_embedded());
            renderer
                .set_icons(koebi_icons::packed())
                .expect("a valid icon set");
            gpu.render_offscreen(Extent::new(720, 600), format, |encoder, target| {
                renderer
                    .render(encoder, target, &scene, 0, 1.0)
                    .expect("render");
            })
            .expect("offscreen rendering")
        };
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a pixel inside the image"
        )]
        let (x, y) = (point.x.floor() as u32, point.y.floor() as u32);
        let text: Color = design.get(koebi_tokens::semantic::TEXT_COLOR_PRIMARY_DEFAULT);
        let close_enough = |pixel: [u8; 4], color: Color| {
            pixel
                .iter()
                .zip(color.to_rgba8())
                .all(|(a, b)| a.abs_diff(b) <= 2)
        };
        let two = render(&mut icons).pixel(x, y);
        assert!(
            !close_enough(two, text),
            "two tones: the shadow is fainter, {two:?}"
        );
        icons.two_tones = false;
        let one = render(&mut icons).pixel(x, y);
        assert!(
            close_enough(one, text),
            "one tone: the shadow in the text color, {one:?}"
        );
        // Fainter means darker than the text on the dark panel.
        assert!(two[0] < one[0], "{two:?} {one:?}");
    }
}
