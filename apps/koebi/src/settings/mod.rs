//! The settings window: a column of pages in groups, and the page shown beside it.
//!
//! - **Theme** ([`theme`]) and **Accessibility** ([`accessibility`]) choose among what the design
//!   system offers.
//! - **Customisation** ([`customisation`]) edits every token, by feature or by component.
//! - **Dev** ([`dev`]), in development builds: what helps develop, the frame times first.
//! - **Inputs** ([`inputs`]) edits the distances and delays of interaction, which are tokens.
//! - The others wait for what they will hold. **Dev** shows only in development builds.
//!
//! Options are expandable panels ([`options`]): the header edits the value for the whole
//! application, and the panel opens on the same value for the application and for the current
//! theme, each with what brings it back, and on how the token gets its value.

mod accessibility;
mod cards;
mod colors;
mod customisation;
mod dev;
mod icons;
mod inputs;
mod keymaps;
mod language;
mod options;
mod theme;
mod values;

use koebi_core::{Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_i18n::tr;
use koebi_icons::IconId;

use crate::language::LanguageSetting;
use koebi_input::Keymap;
use koebi_tokens::{component, semantic};
use koebi_ui::{
    Align, Button, ButtonGroup, CollapsiblePanel, CollapsiblePanelResponse, GroupCell, PanelLevel,
    ScrollAxes, ScrollState, Ui, WidgetId,
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
    /// The icon of the page, in its button.
    const fn icon(self) -> IconId {
        use koebi_icons::settings as icons;
        match self {
            Self::General => icons::GENERAL,
            Self::Language => icons::LANGUAGE,
            Self::Theme => icons::THEME,
            Self::Customisation => icons::CUSTOMISATION,
            Self::Accessibility => icons::ACCESSIBILITY,
            Self::Inputs => icons::INPUTS,
            Self::Keymaps => icons::KEYMAPS,
            Self::Navigation => icons::NAVIGATION,
            Self::Icons => icons::ICONS,
            Self::Dev => icons::DEV,
        }
    }

    /// The title of the page, in its button.
    fn title(self) -> &'static str {
        match self {
            Self::General => tr("General"),
            Self::Language => tr("Language"),
            Self::Theme => tr("Theme"),
            Self::Customisation => tr("Customisation"),
            Self::Accessibility => tr("Accessibility"),
            Self::Inputs => tr("Inputs"),
            Self::Keymaps => tr("Keymaps"),
            Self::Navigation => tr("Navigation"),
            Self::Icons => tr("Icons"),
            Self::Dev => tr("Dev"),
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

    /// Where the content of the next container lies: inside its frame and padding, from the
    /// top of it down.
    pub(crate) fn content(&self, ui: &Ui<'_>) -> Rect {
        let panel = &ui.styles().panel;
        let inset = panel.border_width + panel.padding;
        Rect::from_min_max(
            Vec2::new(self.left + inset, self.y + inset),
            Vec2::new(
                (self.right - inset).max(self.left + inset),
                self.y + UNBOUNDED,
            ),
        )
    }

    /// Shows, below the previous one, a container that never folds, holding `height` of
    /// content (its sections, which `add` draws in the rectangle [`Self::content`] gives).
    pub(crate) fn container(
        &mut self,
        ui: &mut Ui<'_>,
        height: f32,
        add: impl FnOnce(&mut Ui<'_>, Rect),
    ) {
        let panel = &ui.styles().panel;
        let inset = panel.border_width + panel.padding;
        let rect = Rect::from_min_max(
            Vec2::new(self.left, self.y),
            Vec2::new(self.right, self.y + height + 2.0 * inset),
        );
        let content = ui.panel(rect, PanelLevel::First);
        add(ui, content);
        self.y += rect.height() + self.gap;
    }
}

/// The settings window's state.
#[derive(Debug)]
pub(crate) struct SettingsView {
    pub(crate) page: Page,
    theme: theme::ThemePage,
    customisation: customisation::CustomisationPage,
    accessibility: accessibility::AccessibilityPage,
    keymaps: keymaps::KeymapsPage,
    inputs: inputs::InputsPage,
    language_page: language::LanguagePage,
    icons: icons::IconsPage,
    /// How far each page is scrolled, and how high it was on the last frame.
    scroll: [ScrollState; PAGES],
    height: [f32; PAGES],
    /// The language the pages that keep text were built in (`koebi_i18n::revision`).
    language: u64,
}

impl SettingsView {
    /// The settings, on their theme page.
    pub(crate) fn new() -> Self {
        Self {
            page: Page::Theme,
            theme: theme::ThemePage::new(),
            customisation: customisation::CustomisationPage::new(),
            accessibility: accessibility::AccessibilityPage::new(),
            keymaps: keymaps::KeymapsPage::new(),
            inputs: inputs::InputsPage::new(),
            language_page: language::LanguagePage::new(),
            icons: icons::IconsPage::new(),
            scroll: [ScrollState::new(); PAGES],
            height: [0.0; PAGES],
            language: koebi_i18n::revision(),
        }
    }

    /// The page shown.
    #[cfg(test)]
    pub(crate) fn page(&self) -> Page {
        self.page
    }

    /// Draws the settings in `rect`, applying the changes to `design`, to `keymap`, and to
    /// whether the frame times show.
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui<'_>,
        rect: Rect,
        design: &mut DesignSystem,
        keymap: &mut Keymap,
        frame_times: &mut bool,
        language: &mut LanguageSetting,
    ) {
        if self.language != koebi_i18n::revision() {
            self.language = koebi_i18n::revision();
            self.inputs = inputs::InputsPage::new();
            self.keymaps.language_changed();
        }
        let margin = ui.whole_pixels(design.get(component::SIDE_PANEL_MARGIN_DEFAULT));
        // At least the token's width, as wide as the widest page's button: no name is cut in
        // any language. Half the window at most, the page keeping its room.
        let widest = shown_groups()
            .iter()
            .flat_map(|pages| pages.iter())
            .map(|page| {
                Button::new(ID)
                    .label(page.title())
                    .leading_icon(page.icon())
                    .align(Align::Start)
                    .size(ui)
                    .x
            })
            .fold(0.0_f32, f32::max);
        let width = design
            .get(component::SIDE_PANEL_WIDTH_DEFAULT)
            .max(ui.whole_pixels(widest + 2.0 * margin))
            .min((rect.width() * 0.5).max(0.0));
        let column = Rect::from_min_max(
            rect.min,
            Vec2::new((rect.min.x + width).min(rect.max.x), rect.max.y),
        );
        self.column(ui, column, design, margin);
        let content = Rect::from_min_max(Vec2::new(column.max.x, rect.min.y), rect.max);
        if !content.is_empty() {
            self.content(ui, content, design, keymap, frame_times, language, margin);
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
                let button = Button::new(id)
                    .label(page.map_or("", |page| page.title()))
                    .align(Align::Start)
                    .selected(page == Some(&self.page));
                GroupCell::new(match page {
                    Some(page) => button.leading_icon(page.icon()),
                    None => button,
                })
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
    #[allow(
        clippy::too_many_arguments,
        reason = "what the pages change, from the window"
    )]
    fn content(
        &mut self,
        ui: &mut Ui<'_>,
        rect: Rect,
        design: &mut DesignSystem,
        keymap: &mut Keymap,
        frame_times: &mut bool,
        language: &mut LanguageSetting,
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
                    Page::Customisation => {
                        self.customisation.show(ui, &mut stack, design, keymap);
                    }
                    Page::Accessibility => self.accessibility.show(ui, &mut stack, design),
                    Page::Inputs => self.inputs.show(ui, &mut stack, design),
                    Page::Keymaps => self.keymaps.show(ui, &mut stack, design, keymap),
                    Page::Dev => dev::show(ui, &mut stack, design, frame_times),
                    Page::Language => self.language_page.show(ui, &mut stack, language),
                    Page::Icons => self.icons.show(ui, &mut stack, design),
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
    ui.label(
        tr("Nothing to set here yet"),
        rect,
        &font,
        color,
        Align::Center,
    );
}

#[cfg(test)]
pub(crate) mod tests {
    use koebi_render::Scene;
    use koebi_text::Fonts;
    use koebi_ui::{Input, StyleCache, UiState};

    use super::*;

    /// Runs frames of the settings.
    pub(crate) struct Harness {
        pub(crate) design: DesignSystem,
        pub(super) keymap: Keymap,
        pub(crate) settings: SettingsView,
        pub(super) frame_times: bool,
        pub(super) language: LanguageSetting,
        pub(super) state: UiState,
        pub(super) input: Input,
        pub(super) styles: StyleCache,
        time: u64,
    }

    impl Harness {
        pub(crate) fn new() -> Self {
            let design = DesignSystem::new(&koebi_tokens::KOEBI).unwrap();
            let mut state = UiState::new();
            state.set_fonts(Fonts::with_embedded());
            Self {
                styles: StyleCache::new(&design),
                design,
                keymap: Keymap::new(&crate::actions::GROUPS).unwrap(),
                settings: SettingsView::new(),
                frame_times: false,
                language: LanguageSetting::new(None, Vec::new(), None),
                state,
                input: Input::new(),
                time: 0,
            }
        }

        /// The labels the last frame cut short.
        pub(crate) fn cut_labels(&self) -> Vec<String> {
            self.state.truncated_labels().map(str::to_owned).collect()
        }

        pub(super) fn window() -> Rect {
            Rect::from_origin_size(Vec2::ZERO, Vec2::new(960.0, 680.0))
        }

        pub(crate) fn frame(&mut self) -> Scene {
            let mut scene = Scene::new();
            self.frame_into(&mut scene);
            scene
        }

        /// A frame drawn into `scene`, cleared first: as the application does, keeping the
        /// scene's storage from frame to frame.
        pub(crate) fn frame_into(&mut self, scene: &mut Scene) {
            scene.clear();
            self.time += 16;
            self.input
                .set_time(core::time::Duration::from_millis(self.time));
            {
                let styles = self.styles.update(&self.design, self.state.scale_factor());
                let mut ui = Ui::new(scene, styles, &self.input, &mut self.state);
                self.settings.show(
                    &mut ui,
                    Self::window(),
                    &mut self.design,
                    &mut self.keymap,
                    &mut self.frame_times,
                    &mut self.language,
                );
            }
            self.input.end_frame();
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

        /// The middle of the Dev page's checkbox: below its section's title, in its container.
        pub(super) fn dev_option(&self) -> Vec2 {
            let styles = koebi_ui::Styles::resolve(&self.design);
            let row = self.design.get(semantic::SIZE_CONTROL_HEIGHT);
            let margin = self.design.get(component::SIDE_PANEL_MARGIN_DEFAULT);
            let left = self.design.get(component::SIDE_PANEL_WIDTH_DEFAULT);
            let inset = styles.panel.border_width + styles.panel.padding;
            let y = margin + inset + row + styles.property_group.row_gap + row * 0.5;
            Vec2::new(left + inset + 6.0, y)
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
    pub(super) fn page_image(
        gpu: &koebi_testkit::HeadlessGpu,
        harness: &mut Harness,
    ) -> koebi_testkit::Image {
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
        renderer
            .set_icons(koebi_icons::packed())
            .expect("a valid icon set");
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
    pub(super) fn assert_page(image: &koebi_testkit::Image, name: &str) {
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

    /// `page` drawn with the interface in `code`, on this thread only.
    fn translated_page(
        gpu: &koebi_testkit::HeadlessGpu,
        page: Page,
        code: &str,
    ) -> koebi_testkit::Image {
        let language = koebi_i18n::find(code).expect("the catalog ships with Koebi");
        koebi_i18n::with_language(language, || {
            let mut harness = Harness::new();
            harness.settings.page = page;
            // Key caps as on Windows, whatever the machine.
            harness.state.set_keyboard(koebi_ui::Keyboard::Windows);
            page_image(gpu, &mut harness)
        })
    }

    #[test]
    fn the_language_page_lists_every_language_in_french() {
        let Some(gpu) =
            koebi_testkit::gpu_or_skip("the_language_page_lists_every_language_in_french")
        else {
            return;
        };
        let image = translated_page(&gpu, Page::Language, "fr");
        assert_page(&image, "settings_language_fr");
        gpu.assert_no_validation_errors();
    }

    #[test]
    fn the_theme_page_reads_in_japanese() {
        // The theme page shows the same on every platform: the keymaps page shows each one's
        // default shortcuts.
        let Some(gpu) = koebi_testkit::gpu_or_skip("the_theme_page_reads_in_japanese") else {
            return;
        };
        let image = translated_page(&gpu, Page::Theme, "ja");
        assert_page(&image, "settings_theme_ja");
        gpu.assert_no_validation_errors();
    }

    #[test]
    fn the_page_titles_follow_the_language() {
        let french = koebi_i18n::find("fr").unwrap();
        let japanese = koebi_i18n::find("ja").unwrap();
        assert_eq!(
            koebi_i18n::with_language(french, || Page::Theme.title()),
            "Thème"
        );
        assert_eq!(
            koebi_i18n::with_language(japanese, || Page::Keymaps.title()),
            "キーマップ"
        );
        assert_eq!(Page::Theme.title(), "Theme");
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
        // Keys named the same on every platform, shortcuts without the primary modifier.
        harness.state.set_keyboard(koebi_ui::Keyboard::Windows);
        // Fullscreen with a second shortcut, the frame times', off for the conflict; its panel
        // open, and the search showing the two actions.
        let shortcut = |keymap: &Keymap, id: &str| keymap.shortcuts(id).unwrap()[0];
        let frame_times = shortcut(&harness.keymap, crate::actions::FRAME_TIMES.id);
        let fullscreen = shortcut(&harness.keymap, crate::actions::FULLSCREEN.id);
        harness
            .keymap
            .set_bindings(
                crate::actions::FULLSCREEN.id,
                &[
                    koebi_input::Binding::on(fullscreen),
                    koebi_input::Binding::off(frame_times),
                ],
            )
            .unwrap();
        harness
            .settings
            .keymaps
            .open_action(crate::actions::FULLSCREEN.id, &[1]);
        harness.settings.keymaps.search_for(frame_times);
        let image = page_image(&gpu, &mut harness);
        assert_page(&image, "settings_keymaps");
        gpu.assert_no_validation_errors();
    }

    #[test]
    fn the_keymaps_page_with_a_shared_shortcut_and_a_draft_matches_golden() {
        let Some(gpu) = koebi_testkit::gpu_or_skip("the_keymaps_page_with_a_draft") else {
            return;
        };
        let mut harness = Harness::new();
        harness.settings.page = Page::Keymaps;
        harness.state.set_keyboard(koebi_ui::Keyboard::Windows);
        // Fold all also on Unfold all's key: they never apply at once, so they share it.
        let fold = crate::editors::outliner::FOLD_ALL.id;
        let unfold = crate::editors::outliner::UNFOLD_ALL.id;
        let shortcut = |keymap: &Keymap, id: &str| keymap.shortcuts(id).unwrap()[0];
        let (minus, plus) = (
            shortcut(&harness.keymap, fold),
            shortcut(&harness.keymap, unfold),
        );
        harness
            .keymap
            .set_shortcuts(fold, &[minus, plus])
            .expect("shared, not taken");
        // Its shared shortcut open, and a shortcut being added.
        harness.settings.keymaps.open_action(fold, &[1]);
        harness.settings.keymaps.add_draft(fold);
        harness.settings.keymaps.search_for(plus);
        let image = page_image(&gpu, &mut harness);
        assert_page(&image, "settings_keymaps_shared");
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

    #[test]
    fn no_page_cuts_a_label_at_the_window_s_size_in_any_language() {
        // Widths come from what the labels measure, in every language: the window as it
        // opens cuts nothing (what a narrower one cuts shows whole in a tooltip).
        let pages = [
            Page::General,
            Page::Language,
            Page::Theme,
            Page::Customisation,
            Page::Accessibility,
            Page::Inputs,
            Page::Keymaps,
            Page::Navigation,
            Page::Icons,
            Page::Dev,
        ];
        for language in koebi_i18n::languages() {
            koebi_i18n::with_language(language, || {
                for page in pages {
                    let mut harness = Harness::new();
                    harness.settings.page = page;
                    harness.settle();
                    let cut = harness.cut_labels();
                    assert!(cut.is_empty(), "{} {page:?}: {cut:?}", language.code());
                }
            });
        }
    }

    /// Koebi's rule for hot paths (CLAUDE.md): once a page has settled, its frames allocate
    /// nothing. The test binary counts allocations (`koebi_testkit::CountingAllocator`), so a
    /// `format!`, a collected `Vec` or a cloned `String` in a frame fails here.
    #[test]
    fn no_page_allocates_in_a_steady_frame() {
        let mut failures = Vec::new();
        for &page in GROUPS.iter().flat_map(|pages| pages.iter()) {
            let mut harness = Harness::new();
            harness.settings.page = page;
            let mut scene = Scene::new();
            // Still, then with the pointer over the page, where hovers and tooltips show.
            for pointer in [None, Some(Vec2::new(600.0, 300.0))] {
                if let Some(at) = pointer {
                    harness.input.pointer_moved(at);
                }
                for _ in 0..10 {
                    harness.frame_into(&mut scene);
                }
                let (count, ()) = koebi_testkit::allocations(|| harness.frame_into(&mut scene));
                if count > 0 {
                    failures.push(format!("{page:?} (pointer {pointer:?}): {count}"));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "allocations in steady frames: {failures:#?}"
        );
    }
}
