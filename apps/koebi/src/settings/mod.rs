//! The settings window: a column of pages in groups, and the page shown beside it.
//!
//! - **Theme** ([`theme`]) and **Accessibility** ([`accessibility`]) choose among what the design
//!   system offers.
//! - **Inputs** ([`inputs`]) edits the distances and delays of interaction, which are tokens.
//! - The others wait for what they will hold. **Dev** shows only in development builds.
//!
//! Options are expandable panels ([`options`]): the header edits the value for the whole
//! application, and the panel opens on the same value for the application and for the current
//! theme, each with what brings it back, and on how the token gets its value.

mod accessibility;
mod inputs;
mod keymaps;
mod options;
mod theme;

use koebi_core::{Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_input::Keymap;
use koebi_tokens::{component, semantic};
use koebi_ui::{
    Align, Button, ButtonGroup, CollapsiblePanel, CollapsiblePanelResponse, GroupCell, ScrollAxes,
    ScrollState, Ui, WidgetId,
};

const ID: WidgetId = WidgetId::new("settings");

/// The height a panel lays its content out in: the content measures what it uses.
const UNBOUNDED: f32 = 1.0e6;

/// A page of the settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Page {
    General,
    Language,
    Theme,
    Customisation,
    Accessibility,
    Inputs,
    Keymaps,
    Navigation,
    Icons,
    Dev,
}

impl Page {
    /// The title of the page, in its button.
    const fn title(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Language => "Language",
            Self::Theme => "Theme",
            Self::Customisation => "Customisation",
            Self::Accessibility => "Accessibility",
            Self::Inputs => "Inputs",
            Self::Keymaps => "Keymaps",
            Self::Navigation => "Navigation",
            Self::Icons => "Icons",
            Self::Dev => "Dev",
        }
    }
}

/// How many pages there are.
const PAGES: usize = 10;

/// The pages of the column, group by group; the last group shows in development builds only.
const GROUPS: [&[Page]; 5] = [
    &[Page::General, Page::Language],
    &[Page::Theme, Page::Customisation, Page::Accessibility],
    &[Page::Inputs, Page::Keymaps, Page::Navigation],
    &[Page::Icons],
    &[Page::Dev],
];

/// The groups of the column in this build.
fn shown_groups() -> &'static [&'static [Page]] {
    if cfg!(debug_assertions) {
        &GROUPS
    } else {
        &GROUPS[..GROUPS.len() - 1]
    }
}

/// Panels stacked down a column, `gap` apart.
pub(crate) struct Stack {
    pub(crate) left: f32,
    pub(crate) right: f32,
    pub(crate) y: f32,
    pub(crate) gap: f32,
    /// Where menus opened from the page may lie: the room the page shows in.
    pub(crate) bounds: Rect,
}

impl Stack {
    /// Shows `panel` below the previous one, `add` drawing its content. Returns what happened
    /// to it.
    pub(crate) fn panel(
        &mut self,
        ui: &mut Ui<'_>,
        panel: CollapsiblePanel<'_>,
        add: impl FnOnce(&mut Ui<'_>, Rect) -> f32,
    ) -> CollapsiblePanelResponse {
        let at = Rect::from_min_max(
            Vec2::new(self.left, self.y),
            Vec2::new(self.right, self.y + UNBOUNDED),
        );
        let response = panel.show(ui, at, add);
        self.y += response.height + self.gap;
        response
    }
}

/// The settings window's state.
#[derive(Debug)]
pub(crate) struct SettingsView {
    page: Page,
    theme: theme::ThemePage,
    accessibility: accessibility::AccessibilityPage,
    keymaps: keymaps::KeymapsPage,
    inputs: inputs::InputsPage,
    /// How far each page is scrolled, and how high it was on the last frame.
    scroll: [ScrollState; PAGES],
    height: [f32; PAGES],
}

impl SettingsView {
    /// The settings, on their theme page.
    pub(crate) fn new() -> Self {
        Self {
            page: Page::Theme,
            theme: theme::ThemePage::new(),
            accessibility: accessibility::AccessibilityPage::new(),
            keymaps: keymaps::KeymapsPage::new(),
            inputs: inputs::InputsPage::new(),
            scroll: [ScrollState::new(); PAGES],
            height: [0.0; PAGES],
        }
    }

    /// The page shown.
    #[cfg(test)]
    pub(crate) fn page(&self) -> Page {
        self.page
    }

    /// Draws the settings in `rect`, applying the changes to `design`.
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui<'_>,
        rect: Rect,
        design: &mut DesignSystem,
        keymap: &mut Keymap,
    ) {
        let width = design.get(component::SIDE_PANEL_WIDTH_DEFAULT);
        let margin = design.get(component::SIDE_PANEL_MARGIN_DEFAULT);
        let column = Rect::from_min_max(
            rect.min,
            Vec2::new((rect.min.x + width).min(rect.max.x), rect.max.y),
        );
        self.column(ui, column, design, margin);
        let content = Rect::from_min_max(Vec2::new(column.max.x, rect.min.y), rect.max);
        if !content.is_empty() {
            self.content(ui, content, design, keymap, margin);
        }
    }

    /// The column of pages, in groups.
    fn column(&mut self, ui: &mut Ui<'_>, column: Rect, design: &DesignSystem, margin: f32) {
        let row = design.get(semantic::SIZE_CONTROL_HEIGHT);
        let gap = design.get(semantic::SPACING_EDITOR_INSET);
        let (left, right) = (column.min.x + margin, column.max.x - margin);
        let mut y = column.min.y + margin;
        let mut key = 0_u64;
        for pages in shown_groups() {
            // No group holds more than this; the cells past its pages are not shown.
            let mut cells: [GroupCell<'_>; 3] = core::array::from_fn(|index| {
                let id = ID.with(1).with(key + index as u64);
                let page = pages.get(index);
                GroupCell::new(
                    Button::new(id)
                        .label(page.map_or("", |page| page.title()))
                        .align(Align::Start)
                        .selected(page == Some(&self.page)),
                )
            });
            let cells = &mut cells[..pages.len()];
            let height = row * pages.len() as f32;
            let group = Rect::from_min_max(Vec2::new(left, y), Vec2::new(right, y + height));
            let clicked = ButtonGroup::vertical().show(ui, group, cells).clicked;
            if let Some(page) = clicked.and_then(|index| pages.get(index)) {
                self.page = *page;
            }
            y += height + gap;
            key += 8;
        }
    }

    /// The page shown, scrolling when it is too high for its room.
    fn content(
        &mut self,
        ui: &mut Ui<'_>,
        rect: Rect,
        design: &mut DesignSystem,
        keymap: &mut Keymap,
        margin: f32,
    ) {
        let index = self.page as usize;
        let size = Vec2::new(rect.width(), self.height[index]);
        let mut scroll = self.scroll[index];
        let height = ui.scroll_area(
            ID.with(2).with(index as u64),
            rect,
            &mut scroll,
            size,
            ScrollAxes::Vertical,
            |ui, area| {
                let top = area.origin.y;
                let mut stack = Stack {
                    left: area.origin.x,
                    right: area.viewport.max.x - margin,
                    y: top + margin,
                    gap: ui.styles().collapsible_panel.gap,
                    bounds: area.viewport,
                };
                match self.page {
                    Page::Theme => self.theme.show(ui, &mut stack, design),
                    Page::Accessibility => self.accessibility.show(ui, &mut stack, design),
                    Page::Inputs => self.inputs.show(ui, &mut stack, design),
                    Page::Keymaps => self.keymaps.show(ui, &mut stack, design, keymap),
                    _ => empty_page(ui, area.viewport, design),
                }
                stack.y - top + margin
            },
        );
        self.scroll[index] = scroll;
        if (height - self.height[index]).abs() > 0.01 {
            self.height[index] = height;
            ui.request_redraw_after(core::time::Duration::ZERO);
        }
    }
}

/// What a page without settings yet shows.
fn empty_page(ui: &mut Ui<'_>, rect: Rect, design: &DesignSystem) {
    let font = ui.styles().text.label.clone();
    let color = design.get(semantic::TEXT_COLOR_SECONDARY_DEFAULT);
    ui.label("Nothing to set here yet", rect, &font, color, Align::Center);
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;
    use koebi_text::Fonts;
    use koebi_ui::{Input, StyleCache, UiState};

    use super::*;

    /// Runs frames of the settings.
    pub(super) struct Harness {
        pub(super) design: DesignSystem,
        pub(super) keymap: Keymap,
        pub(super) settings: SettingsView,
        pub(super) state: UiState,
        pub(super) input: Input,
        styles: StyleCache,
        time: u64,
    }

    impl Harness {
        pub(super) fn new() -> Self {
            let design = DesignSystem::new(&koebi_tokens::KOEBI).unwrap();
            let mut state = UiState::new();
            state.set_fonts(Fonts::with_embedded());
            Self {
                styles: StyleCache::new(&design),
                design,
                keymap: Keymap::new(&crate::actions::GROUPS).unwrap(),
                settings: SettingsView::new(),
                state,
                input: Input::new(),
                time: 0,
            }
        }

        pub(super) fn window() -> Rect {
            Rect::from_origin_size(Vec2::ZERO, Vec2::new(960.0, 680.0))
        }

        pub(super) fn frame(&mut self) -> Scene {
            let mut scene = Scene::new();
            self.time += 16;
            self.input
                .set_time(core::time::Duration::from_millis(self.time));
            {
                let styles = self.styles.update(&self.design);
                let mut ui = Ui::new(&mut scene, styles, &self.input, &mut self.state);
                self.settings
                    .show(&mut ui, Self::window(), &mut self.design, &mut self.keymap);
            }
            self.input.end_frame();
            scene
        }

        /// Frames until what changed has settled: heights and animations.
        pub(super) fn settle(&mut self) {
            for _ in 0..30 {
                self.frame();
            }
        }

        pub(super) fn click(&mut self, at: Vec2) {
            self.input.pointer_moved(at);
            self.frame();
            self.input.primary_button(true, at);
            self.frame();
            self.input.primary_button(false, at);
            self.frame();
        }

        /// The middle of the button of `page` in the column.
        pub(super) fn page_button(&self, page: Page) -> Vec2 {
            let row = self.design.get(semantic::SIZE_CONTROL_HEIGHT);
            let gap = self.design.get(semantic::SPACING_EDITOR_INSET);
            let margin = self.design.get(component::SIDE_PANEL_MARGIN_DEFAULT);
            let mut y = margin;
            for pages in shown_groups() {
                for (index, candidate) in pages.iter().enumerate() {
                    if *candidate == page {
                        return Vec2::new(margin + 20.0, y + row * (index as f32 + 0.5));
                    }
                }
                y += row * pages.len() as f32 + gap;
            }
            panic!("no button for {page:?}");
        }
    }

    /// The page as it shows, rendered offscreen.
    fn page_image(gpu: &koebi_testkit::HeadlessGpu, harness: &mut Harness) -> koebi_testkit::Image {
        harness.settle();
        let mut scene = harness.frame();
        let background = harness
            .design
            .get(component::WINDOW_BACKGROUND_COLOR_DEFAULT);
        scene.set_clear_color(background);
        let format = koebi_gpu::TextureFormat::Rgba8Unorm;
        let mut renderer =
            koebi_render::Renderer2D::new(&gpu.device, format).expect("renderer creation");
        renderer.set_fonts(Fonts::with_embedded());
        gpu.render_offscreen(
            koebi_core::Extent::new(960, 680),
            format,
            |encoder, target| {
                renderer
                    .render(encoder, target, &scene, 0, 1.0)
                    .expect("render");
            },
        )
        .expect("offscreen rendering")
    }

    /// Compares `image` with `tests/golden/<name>.png` of the application.
    #[track_caller]
    fn assert_page(image: &koebi_testkit::Image, name: &str) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        koebi_testkit::assert_golden_at(
            &root.join("tests").join("golden"),
            &std::env::temp_dir().join("koebi-golden-failures"),
            name,
            image,
            koebi_testkit::Tolerance::new(3, 8),
        );
    }

    #[test]
    fn the_theme_page_matches_golden() {
        let Some(gpu) = koebi_testkit::gpu_or_skip("the_theme_page_matches_golden") else {
            return;
        };
        let mut harness = Harness::new();
        let image = page_image(&gpu, &mut harness);
        assert_page(&image, "settings_theme");
        gpu.assert_no_validation_errors();
    }

    #[test]
    fn the_accessibility_page_matches_golden() {
        let Some(gpu) = koebi_testkit::gpu_or_skip("the_accessibility_page_matches_golden") else {
            return;
        };
        let mut harness = Harness::new();
        harness.settings.page = Page::Accessibility;
        let image = page_image(&gpu, &mut harness);
        assert_page(&image, "settings_accessibility");
        gpu.assert_no_validation_errors();
    }

    #[test]
    fn the_keymaps_page_with_a_conflict_matches_golden() {
        let Some(gpu) = koebi_testkit::gpu_or_skip("the_keymaps_page_matches_golden") else {
            return;
        };
        let mut harness = Harness::new();
        harness.settings.page = Page::Keymaps;
        // Fullscreen with a second shortcut, Quit's, off for the conflict; its panel open.
        let quit = harness.keymap.shortcuts(crate::actions::QUIT.id).unwrap()[0];
        let fullscreen = harness
            .keymap
            .shortcuts(crate::actions::FULLSCREEN.id)
            .unwrap()[0];
        harness
            .keymap
            .set_bindings(
                crate::actions::FULLSCREEN.id,
                &[
                    koebi_input::Binding::on(fullscreen),
                    koebi_input::Binding::off(quit),
                ],
            )
            .unwrap();
        harness
            .settings
            .keymaps
            .open_action(crate::actions::FULLSCREEN.id, &[1]);
        let image = page_image(&gpu, &mut harness);
        assert_page(&image, "settings_keymaps");
        gpu.assert_no_validation_errors();
    }

    #[test]
    fn the_inputs_page_with_an_option_open_matches_golden() {
        let Some(gpu) = koebi_testkit::gpu_or_skip("the_inputs_page_matches_golden") else {
            return;
        };
        let mut harness = Harness::new();
        harness.settings.page = Page::Inputs;
        // An override along the chain: it gives the value, and what lies past it is grayed.
        harness
            .design
            .set_value(
                koebi_design::Scope::ActiveTheme,
                semantic::INPUT_DRAG_THRESHOLD,
                6.0,
            )
            .unwrap();
        harness.settings.inputs.open_option(0, 0, true);
        let image = page_image(&gpu, &mut harness);
        assert_page(&image, "settings_inputs");
        gpu.assert_no_validation_errors();
    }

    #[test]
    fn the_column_holds_every_page_in_its_groups() {
        let titles: Vec<Vec<&str>> = GROUPS
            .iter()
            .map(|pages| pages.iter().map(|page| page.title()).collect())
            .collect();
        assert_eq!(
            titles,
            [
                vec!["General", "Language"],
                vec!["Theme", "Customisation", "Accessibility"],
                vec!["Inputs", "Keymaps", "Navigation"],
                vec!["Icons"],
                vec!["Dev"],
            ]
        );
    }

    #[test]
    fn each_page_keeps_its_own_scroll_and_height() {
        // Regression test: they shared one, and a shorter page opened blank, scrolled past its end.
        let mut harness = Harness::new();
        let inputs = harness.page_button(Page::Inputs);
        harness.click(inputs);
        // With a few options open, it is higher than the window.
        for group in 0..3 {
            harness.settings.inputs.open_option(group, 0, true);
        }
        harness.settle();
        let tall = harness.settings.height[Page::Inputs as usize];
        assert!(tall > Harness::window().height());
        // Over the page, the wheel scrolls it.
        harness.input.pointer_moved(Vec2::new(500.0, 300.0));
        harness.frame();
        harness.input.scrolled(Vec2::new(0.0, -10.0));
        harness.settle();
        let scrolled = harness.settings.scroll[Page::Inputs as usize];
        assert!(scrolled.offset().y > 0.0);
        let theme = harness.page_button(Page::Theme);
        harness.click(theme);
        harness.settle();
        assert_eq!(
            harness.settings.scroll[Page::Theme as usize].offset().y,
            0.0
        );
        assert!(harness.settings.height[Page::Theme as usize] < tall);
        // Coming back, it is where it was.
        harness.click(inputs);
        harness.settle();
        assert_eq!(harness.settings.scroll[Page::Inputs as usize], scrolled);
    }

    #[test]
    fn dev_shows_in_development_builds_only() {
        let has_dev = shown_groups()
            .iter()
            .any(|pages| pages.contains(&Page::Dev));
        assert_eq!(has_dev, cfg!(debug_assertions));
    }

    #[test]
    fn the_buttons_of_the_column_change_the_page() {
        let mut harness = Harness::new();
        harness.settle();
        assert_eq!(harness.settings.page(), Page::Theme);
        for page in [
            Page::Inputs,
            Page::Accessibility,
            Page::General,
            Page::Icons,
        ] {
            let at = harness.page_button(page);
            harness.click(at);
            assert_eq!(harness.settings.page(), page);
            harness.settle();
        }
    }
}
