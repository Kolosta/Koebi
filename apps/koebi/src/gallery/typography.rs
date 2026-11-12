//! The typography of the gallery: text drawn from its curves, every setting live. A family of
//! the fonts loaded (Noto Sans, Noto Sans JP, the system's Korean and Chinese fallbacks), or any
//! TrueType font of the system once listed; its weight, size and turn; letter spacing, line
//! height, width and alignment; kerning and ligatures on or off; a sample in Latin, Japanese,
//! Korean, Chinese, or all of them in one paragraph. The block is laid out and drawn again on
//! every change, from the same glyph curves at any size and angle.

use koebi_core::{Rect, Vec2};
use koebi_i18n::n_;
use koebi_i18n::tr;
use koebi_render::RectPrimitive;
use koebi_text::system::SystemFont;
use koebi_text::{Align as TextAlign, Features, Paragraph, TextBlock, TextStyle};
use koebi_ui::{
    Align, BarMapping, Button, ButtonGroup, Dropdown, GroupCell, MenuItem, MenuState, NumberField,
    Overflow, PanelLevel, Track, Ui, WidgetId,
};

use super::{Metrics, cut_left, cut_top};

/// Rows of the panel: three of settings, then the preview.
const SETTING_ROWS: f32 = 3.0;
const PREVIEW_ROWS: f32 = 11.0;

/// The samples: their name and their text.
pub(crate) const SAMPLES: [(&str, &str); 5] = [
    (
        n_!("Latin"),
        "Typography: kerning in AV, To and Wa, ligatures in office, affluent and fjord. The quick \
         brown fox jumps over the lazy dog. Été, Жук, 12,5 €.",
    ),
    (
        n_!("Japanese"),
        "吾輩は猫である。名前はまだ無い。どこで生れたかとんと見当がつかぬ。何でも薄暗いじめじめした所で\
         ニャーニャー泣いていた事だけは記憶している。",
    ),
    (
        n_!("Korean"),
        "모든 인간은 태어날 때부터 자유로우며 그 존엄과 권리에 있어 동등하다. 인간은 천부적으로 이성과 \
         양심을 부여받았으며 서로 형제애의 정신으로 행동하여야 한다.",
    ),
    (
        n_!("Chinese"),
        "人人生而自由，在尊严和权利上一律平等。他们赋有理性和良心，并应以兄弟关系的精神相对待。",
    ),
    (
        n_!("Mixed"),
        "Koebi は日本語、English, 한국어 and 中文 in one paragraph: each character takes the font \
         of its script.",
    ),
];

/// What the panel keeps.
#[derive(Debug)]
pub(crate) struct Typography {
    family: String,
    weight: f32,
    size: f32,
    rotation: f32,
    tracking: f32,
    line_height: f32,
    width: f32,
    align: TextAlign,
    kerning: bool,
    ligatures: bool,
    sample: usize,
    /// The system's fonts, once listed.
    system: Option<Vec<SystemFont>>,
    /// A font of the system to load, for the application to take.
    request: Option<SystemFont>,
    family_menu: MenuState,
    sample_menu: MenuState,
    block: TextBlock,
}

impl Typography {
    pub(super) fn new() -> Self {
        Self {
            family: koebi_text::embedded::NOTO_SANS.to_owned(),
            weight: 400.0,
            size: 22.0,
            rotation: 0.0,
            tracking: 0.0,
            line_height: 1.5,
            width: 520.0,
            align: TextAlign::Start,
            kerning: true,
            ligatures: true,
            sample: 0,
            system: None,
            request: None,
            family_menu: MenuState::new(),
            sample_menu: MenuState::new(),
            block: TextBlock::new(),
        }
    }

    /// The font of the system to load, once: the application loads it and adds it to the
    /// fonts of every window.
    pub(crate) fn take_request(&mut self) -> Option<SystemFont> {
        self.request.take()
    }

    /// The height of the panel.
    pub(super) fn height(ui: &Ui<'_>, metrics: Metrics) -> f32 {
        let panel = &ui.styles().panel;
        (SETTING_ROWS + PREVIEW_ROWS) * metrics.row
            + SETTING_ROWS * metrics.row_gap()
            + 2.0 * (panel.padding + panel.border_width)
    }

    /// The families the dropdown offers: those of the loaded fonts, then the system's listed.
    fn families(&self, ui: &Ui<'_>) -> Vec<String> {
        let mut families: Vec<String> = ui
            .fonts()
            .families()
            .into_iter()
            .map(str::to_owned)
            .collect();
        for font in self.system.iter().flatten() {
            if !families.contains(&font.family) {
                families.push(font.family.clone());
            }
        }
        families
    }

    /// Draws the panel in `rect`, its menus inside `window`.
    pub(super) fn show(&mut self, ui: &mut Ui<'_>, rect: Rect, window: Rect, metrics: Metrics) {
        const ID: WidgetId = WidgetId::new("typography");
        let mut content = ui.panel(rect, PanelLevel::First);
        let (row, gap) = (metrics.row, metrics.row_gap());

        // The font: family, system fonts, weight, size, turn.
        let mut line = cut_top(&mut content, row, gap);
        let families = self.families(ui);
        let items: Vec<MenuItem<'_>> = families
            .iter()
            .map(|family| MenuItem::action(family).radio(*family == self.family))
            .collect();
        let chosen = Dropdown::new(ID.with(1), &mut self.family_menu, &items)
            .label(&self.family)
            .show(ui, cut_left(&mut line, 9.0 * row, gap), window)
            .and_then(|choice| families.get(choice.index()).cloned());
        if let Some(family) = chosen {
            self.choose(ui, family);
        }
        let listed = self.system.as_ref().map_or(0, Vec::len);
        let label = if self.system.is_some() {
            tr("System fonts listed")
        } else {
            tr("List system fonts")
        };
        let button = Button::new(ID.with(2))
            .label(label)
            .enabled(self.system.is_none());
        let width = button.size(ui).x;
        if button.show(ui, cut_left(&mut line, width, gap)).clicked {
            self.system = Some(koebi_text::system::scan());
        }
        ui.tooltip(
            ID.with(2),
            if listed > 0 {
                tr("TrueType fonts of the system: pick one in the family menu to load it")
            } else {
                tr("Reads the headers of the system's fonts; fonts with CFF outlines are not read yet")
            },
            rect,
        );
        let fields = [
            (3, tr("Weight"), &mut self.weight, (100.0, 900.0), 100.0, 0),
            (4, tr("Size"), &mut self.size, (4.0, 240.0), 1.0, 1),
            (5, tr("Turn"), &mut self.rotation, (-180.0, 180.0), 1.0, 0),
        ];
        let width = ((line.width() - 2.0 * gap) / 3.0).max(0.0);
        for (index, name, value, (low, high), step, decimals) in fields {
            NumberField::new(ID.with(index), value)
                .label(name)
                .range(low, high)
                .step(step)
                .decimals(decimals)
                .bar(BarMapping::Linear)
                .show(ui, cut_left(&mut line, width, gap));
        }

        // The block: letter spacing, line height, width, alignment.
        let mut line = cut_top(&mut content, row, gap);
        let fields = [
            (
                6,
                tr("Letter spacing"),
                &mut self.tracking,
                (-0.1, 0.5),
                0.01,
                2,
            ),
            (
                7,
                tr("Line height"),
                &mut self.line_height,
                (0.8, 3.0),
                0.05,
                2,
            ),
            (8, tr("Width"), &mut self.width, (40.0, 1200.0), 1.0, 0),
        ];
        let width = ((line.width() - 9.0 * row - 3.0 * gap) / 3.0).max(0.0);
        for (index, name, value, (low, high), step, decimals) in fields {
            NumberField::new(ID.with(index), value)
                .label(name)
                .range(low, high)
                .step(step)
                .decimals(decimals)
                .bar(BarMapping::Linear)
                .show(ui, cut_left(&mut line, width, gap));
        }
        let aligns = [TextAlign::Start, TextAlign::Center, TextAlign::End];
        let align = |index: usize| self.align == aligns[index];
        let mut cells = [
            GroupCell::new(
                Button::new(ID.with(21))
                    .label(tr("Start"))
                    .selected(align(0)),
            )
            .at(0, 0),
            GroupCell::new(
                Button::new(ID.with(22))
                    .label(tr("Center"))
                    .selected(align(1)),
            )
            .at(1, 0),
            GroupCell::new(Button::new(ID.with(23)).label(tr("End")).selected(align(2))).at(2, 0),
        ];
        let columns = [Track::Share(1.0); 3];
        if let Some(index) = ButtonGroup::horizontal()
            .columns(&columns)
            .show(ui, line, &mut cells)
            .clicked
        {
            self.align = aligns[index];
        }

        // Features and the sample.
        let mut line = cut_top(&mut content, row, gap);
        for (index, label, on) in [
            (9, tr("Kerning"), &mut self.kerning),
            (10, tr("Ligatures"), &mut self.ligatures),
        ] {
            if Button::new(ID.with(index))
                .label(label)
                .selected(*on)
                .show(ui, cut_left(&mut line, 5.0 * row, gap))
                .clicked
            {
                *on = !*on;
            }
        }
        let items: Vec<MenuItem<'_>> = SAMPLES
            .iter()
            .enumerate()
            .map(|(index, (name, _))| MenuItem::action(tr(name)).radio(index == self.sample))
            .collect();
        if let Some(choice) = Dropdown::new(ID.with(11), &mut self.sample_menu, &items)
            .label(tr(SAMPLES[self.sample].0))
            .show(ui, cut_left(&mut line, 7.0 * row, gap), window)
        {
            self.sample = choice.index().min(SAMPLES.len() - 1);
        }
        let secondary = ui.styles().text.secondary;
        let detail = ui.styles().text.detail.clone();
        ui.label_with(
            tr("Drawn from the glyphs' curves: any size, any turn."),
            line,
            &detail,
            secondary,
            Align::End,
            Overflow::Shrink {
                min_size: detail.size * 0.8,
            },
        );

        // The preview: the block on the panel's background, cut at its edges.
        self.preview(ui, content);
    }

    /// Switches to `family`, asking for it to be loaded when it is a system font not loaded yet.
    fn choose(&mut self, ui: &Ui<'_>, family: String) {
        let loaded = ui.fonts().families().contains(&family.as_str());
        if !loaded
            && let Some(font) = self
                .system
                .iter()
                .flatten()
                .find(|font| font.family == family)
        {
            self.request = Some(font.clone());
        }
        self.family = family;
    }

    fn preview(&mut self, ui: &mut Ui<'_>, area: Rect) {
        let background = ui.styles().background;
        let ink = ui.styles().text.primary;
        ui.scene()
            .push_rect(&RectPrimitive::new(area, background).with_corner_radius(4.0));
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "100 to 900"
        )]
        let weight = self.weight.round().clamp(1.0, 1000.0) as u16;
        let fonts = ui.fonts().clone();
        let Some(font) = fonts
            .select(&self.family, weight)
            .or_else(|| fonts.select(koebi_text::embedded::NOTO_SANS, weight))
        else {
            return;
        };
        let style = TextStyle {
            font,
            size: self.size,
            tracking: self.tracking,
            features: Features {
                kerning: self.kerning,
                ligatures: self.ligatures,
            },
        };
        let paragraph = Paragraph {
            width: Some(self.width),
            line_height: self.line_height,
            align: self.align,
        };
        self.block
            .layout(&fonts, SAMPLES[self.sample].1, &style, &paragraph);
        // Upright, from the top left; turned, about the middle of the preview.
        let turn = self.rotation.to_radians();
        let origin = if turn == 0.0 {
            area.min + Vec2::splat(12.0)
        } else {
            let half = Vec2::new(self.block.width(), self.block.height()) * 0.5;
            let (sin, cos) = turn.sin_cos();
            area.center() - Vec2::new(half.x * cos - half.y * sin, half.x * sin + half.y * cos)
        };
        let scene = ui.scene();
        scene.push_clip(area);
        scene.push_block(&self.block, origin, ink, turn);
        scene.pop_clip();
    }
}

#[cfg(test)]
mod tests {
    use koebi_design::DesignSystem;
    use koebi_render::Scene;
    use koebi_text::Fonts;
    use koebi_tokens::KOEBI;
    use koebi_ui::{Input, StyleCache, UiState};

    use super::*;

    fn frame(typography: &mut Typography) -> Scene {
        let design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
        let metrics = Metrics::resolve(&design);
        let mut scene = Scene::new();
        let mut state = UiState::new();
        state.set_fonts(Fonts::with_embedded());
        let cache = StyleCache::new(&design);
        let input = Input::new();
        let window = Rect::from_origin_size(Vec2::ZERO, Vec2::new(900.0, 700.0));
        let mut ui = Ui::new(&mut scene, cache.styles(), &input, &mut state);
        let height = Typography::height(&ui, metrics);
        let panel = Rect::from_origin_size(Vec2::ZERO, Vec2::new(900.0, height));
        typography.show(&mut ui, panel, window, metrics);
        drop(ui);
        scene
    }

    #[test]
    fn every_sample_draws_in_the_fonts_of_its_scripts() {
        let mut typography = Typography::new();
        for (sample, (name, _)) in SAMPLES.iter().enumerate() {
            typography.sample = sample;
            let scene = frame(&mut typography);
            assert!(scene.glyph_count() > 20, "{name}");
            assert!(typography.block.lines().len() >= 2, "{name} wraps");
        }
    }

    #[test]
    fn settings_change_the_block() {
        let mut typography = Typography::new();
        frame(&mut typography);
        let lines = typography.block.lines().len();
        // Narrower: more lines; larger: more lines still.
        typography.width = 200.0;
        frame(&mut typography);
        assert!(typography.block.lines().len() > lines);
        let narrow = typography.block.lines().len();
        typography.size = 40.0;
        frame(&mut typography);
        assert!(typography.block.lines().len() > narrow);
        // Ligatures join "ffi" into one glyph: off, more glyphs.
        typography.width = 2000.0;
        typography.size = 20.0;
        frame(&mut typography);
        let joined = typography.block.glyphs().len();
        typography.ligatures = false;
        frame(&mut typography);
        assert!(typography.block.glyphs().len() > joined);
        // A system font asked for once.
        assert!(typography.take_request().is_none());
    }

    #[test]
    fn mixed_scripts_turned_match_golden() {
        let Some(gpu) = koebi_testkit::gpu_or_skip("mixed_scripts_turned_match_golden") else {
            return;
        };
        let render = |typography: &mut Typography, name: &str| {
            let mut scene = frame(typography);
            scene.set_clear_color(koebi_core::Color::from_hex(0x13_13_13));
            let format = koebi_gpu::TextureFormat::Rgba8Unorm;
            let mut renderer =
                koebi_render::Renderer2D::new(&gpu.device, format).expect("renderer creation");
            renderer.set_fonts(Fonts::with_embedded());
            renderer
                .set_icons(koebi_icons::packed())
                .expect("a valid icon set");
            let image = gpu
                .render_offscreen(
                    koebi_core::Extent::new(900, 340),
                    format,
                    |encoder, target| {
                        renderer
                            .render(encoder, target, &scene, 0, 1.0)
                            .expect("render");
                    },
                )
                .expect("offscreen rendering");
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
            koebi_testkit::assert_golden_at(
                &root.join("tests").join("golden"),
                &std::env::temp_dir().join("koebi-golden-failures"),
                name,
                &image,
                koebi_testkit::Tolerance::new(3, 8),
            );
        };
        let mut typography = Typography::new();
        typography.sample = 4;
        typography.width = 600.0;
        render(&mut typography, "gallery_typography");
        typography.rotation = 12.0;
        typography.size = 30.0;
        typography.sample = 1;
        render(&mut typography, "gallery_typography_turned");
        gpu.assert_no_validation_errors();
    }
}
