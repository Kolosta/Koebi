//! Settings > Customisation: every token of the design system, for the application and for the
//! current theme, through two views chosen at the top.
//!
//! - **By feature** ([`features`]): nested panels from the parts of the interface down to each
//!   token; every header's dot tells what changed below and brings it back.
//! - **By component** ([`components`]): each component documented for users, pictured in its
//!   states, to try, with its shortcuts and its tokens.
//!
//! A third view, the graph of how tokens alias each other, comes once the interface has graph
//! and node components: it is not offered yet.
//!
//! Number fields take their bounds from the tokens' ranges; values a token refuses are
//! reported and change nothing ([`super::values`]).

mod components;
mod features;

use koebi_core::{Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_input::Keymap;
use koebi_ui::{Align, Button, ButtonGroup, GroupCell, Ui, WidgetId};

use super::Stack;

const ID: WidgetId = WidgetId::new("customisation");

/// The views of the page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum View {
    /// Tokens by feature, in nested panels.
    #[default]
    ByFeature,
    /// Components by kind, each documented with its tokens.
    ByComponent,
}

impl View {
    const ALL: [Self; 2] = [Self::ByFeature, Self::ByComponent];

    const fn title(self) -> &'static str {
        match self {
            Self::ByFeature => "By feature",
            Self::ByComponent => "By component",
        }
    }

    const fn description(self) -> &'static str {
        match self {
            Self::ByFeature => {
                "Every token, from the parts of the interface down. A dot marks what changed."
            }
            Self::ByComponent => {
                "Each component: what it does, its states, one to try, its shortcuts and tokens."
            }
        }
    }
}

/// The page: the view shown, and each view's panels, made the first time they show.
#[derive(Debug, Default)]
pub(super) struct CustomisationPage {
    view: View,
    features: Option<features::FeatureTree>,
    components: Option<components::ComponentPages>,
}

impl CustomisationPage {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// The view shown.
    #[cfg(test)]
    fn view(&self) -> View {
        self.view
    }

    /// Shows the page below what `stack` holds.
    pub(super) fn show(
        &mut self,
        ui: &mut Ui<'_>,
        stack: &mut Stack,
        design: &mut DesignSystem,
        keymap: &Keymap,
    ) {
        let row = ui.styles().button.height;
        let gap = ui.styles().property_group.row_gap;
        // The views, side by side, then what the one shown holds.
        let width = (stack.right - stack.left).min(2.0 * 9.0 * row);
        let tabs = Rect::from_min_max(
            Vec2::new(stack.left, stack.y),
            Vec2::new(stack.left + width, stack.y + row),
        );
        let mut cells: [GroupCell<'_>; 2] = core::array::from_fn(|index| {
            let view = View::ALL[index];
            GroupCell::new(
                Button::new(ID.with(index as u64))
                    .label(view.title())
                    .selected(view == self.view),
            )
        });
        if let Some(index) = ButtonGroup::horizontal().show(ui, tabs, &mut cells).clicked
            && let Some(&view) = View::ALL.get(index)
        {
            self.view = view;
        }
        let font = ui.styles().text.body.clone();
        let color = ui.styles().text.secondary;
        let line = Rect::from_min_max(
            Vec2::new(stack.left, tabs.max.y + gap),
            Vec2::new(stack.right, tabs.max.y + gap + row),
        );
        ui.label(self.view.description(), line, &font, color, Align::Start);
        stack.y = line.max.y + stack.gap;
        match self.view {
            View::ByFeature => self
                .features
                .get_or_insert_with(|| features::FeatureTree::new(design))
                .show(ui, stack, design),
            View::ByComponent => self
                .components
                .get_or_insert_with(|| components::ComponentPages::new(design))
                .show(ui, stack, design, keymap),
        }
    }

    /// Shows `view`.
    #[cfg(test)]
    fn set_view(&mut self, view: View) {
        self.view = view;
    }

    /// The tree of the first view, once shown.
    #[cfg(test)]
    fn features(&mut self) -> Option<&mut features::FeatureTree> {
        self.features.as_mut()
    }

    /// The pages of the second view, once shown.
    #[cfg(test)]
    fn components(&mut self) -> Option<&mut components::ComponentPages> {
        self.components.as_mut()
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::Vec2;
    use koebi_design::Scope;
    use koebi_tokens::{component, semantic};

    use super::super::tests::{Harness, assert_page, page_image};
    use super::super::{Page, options};
    use super::*;

    /// The Customisation page, shown and settled.
    fn page() -> Harness {
        let mut harness = Harness::new();
        harness.settings.page = Page::Customisation;
        harness.settle();
        harness
    }

    /// The middle of the button of the view at `index`, at the top of the page.
    fn tab(harness: &Harness, index: usize) -> Vec2 {
        let row = harness.design.get(semantic::SIZE_CONTROL_HEIGHT);
        let margin = harness.design.get(component::SIDE_PANEL_MARGIN_DEFAULT);
        let left = harness.design.get(component::SIDE_PANEL_WIDTH_DEFAULT);
        let right = Harness::window().max.x - margin;
        let width = (right - left).min(2.0 * 9.0 * row);
        Vec2::new(
            left + width * (index as f32 + 0.5) / 2.0,
            margin + row * 0.5,
        )
    }

    /// The middle of the dot at the end of the first feature's header.
    fn first_dot(harness: &Harness) -> Vec2 {
        let styles = koebi_ui::Styles::resolve(&harness.design);
        let row = harness.design.get(semantic::SIZE_CONTROL_HEIGHT);
        let margin = harness.design.get(component::SIDE_PANEL_MARGIN_DEFAULT);
        let panel = styles.collapsible_panel;
        let top = margin + row + styles.property_group.row_gap + row + panel.gap;
        let right = Harness::window().max.x - margin - styles.panel.border_width;
        Vec2::new(
            right - panel.header_padding - row * 0.5,
            top + panel.header_height * 0.5,
        )
    }

    #[test]
    fn the_views_switch_from_their_buttons_and_each_builds_when_first_shown() {
        let mut harness = Harness::new();
        let at = harness.page_button(Page::Customisation);
        harness.click(at);
        harness.settle();
        assert_eq!(harness.settings.page(), Page::Customisation);
        let page = &mut harness.settings.customisation;
        assert_eq!(page.view(), View::ByFeature);
        assert!(page.features().is_some());
        assert!(page.components().is_none());
        let at = tab(&harness, 1);
        harness.click(at);
        harness.settle();
        let page = &mut harness.settings.customisation;
        assert_eq!(page.view(), View::ByComponent);
        assert!(page.components().is_some());
        let at = tab(&harness, 0);
        harness.click(at);
        harness.settle();
        assert_eq!(harness.settings.customisation.view(), View::ByFeature);
    }

    #[test]
    fn a_change_shows_on_every_header_above_it_and_their_dot_brings_it_back() {
        let mut harness = page();
        let token = component::COLOR_PICKER_GAP_DEFAULT;
        harness.design.set_value(Scope::Global, token, 9.0).unwrap();
        harness
            .design
            .set_value(Scope::ActiveTheme, token, 8.0)
            .unwrap();
        let design = &harness.design;
        let tree = harness.settings.customisation.features().unwrap();
        let controls = tree.feature("Controls").unwrap();
        assert_eq!(
            options::changes(design, controls.tokens_below()),
            (true, true)
        );
        let picker = controls.child("Color picker").unwrap();
        assert_eq!(
            options::changes(design, picker.tokens_below()),
            (true, true)
        );
        let controls = tree.feature("Controls").unwrap();
        let button = controls.child("Button").unwrap();
        assert_eq!(
            options::changes(design, button.tokens_below()),
            (false, false)
        );
        let surfaces = tree.feature("Surfaces").unwrap();
        assert_eq!(
            options::changes(design, surfaces.tokens_below()),
            (false, false)
        );
        // The dot of the closed feature's header brings back both.
        harness.settle();
        let at = first_dot(&harness);
        harness.click(at);
        harness.settle();
        assert!(harness.design.override_of(Scope::Global, token).is_none());
        assert!(
            harness
                .design
                .override_of(Scope::ActiveTheme, token)
                .is_none()
        );
    }

    #[test]
    fn the_page_by_feature_matches_golden() {
        let Some(gpu) = koebi_testkit::gpu_or_skip("the_page_by_feature_matches_golden") else {
            return;
        };
        let mut harness = page();
        // The intents, their backgrounds, the accent's and its default open with its chain;
        // the accent changed for the application.
        harness
            .design
            .set_value(
                Scope::Global,
                semantic::BACKGROUND_ACCENT_DEFAULT,
                koebi_core::Color::from_hex(0xE0_7A_30),
            )
            .unwrap();
        let tree = harness.settings.customisation.features().unwrap();
        let intents = tree.feature("Intents").unwrap();
        intents.set_open(true);
        let background = intents.child("Background").unwrap();
        background.set_open(true);
        let accent = background.child("accent").unwrap();
        accent.set_open(true);
        accent
            .row(semantic::BACKGROUND_ACCENT_DEFAULT.id())
            .unwrap()
            .set_open(true, true);
        let image = page_image(&gpu, &mut harness);
        assert_page(&image, "settings_customisation_features");
        gpu.assert_no_validation_errors();
    }

    #[test]
    fn the_page_by_component_matches_golden() {
        let Some(gpu) = koebi_testkit::gpu_or_skip("the_page_by_component_matches_golden") else {
            return;
        };
        let mut harness = page();
        harness.settings.customisation.set_view(View::ByComponent);
        harness.settle();
        harness
            .settings
            .customisation
            .components()
            .unwrap()
            .open("Button");
        let image = page_image(&gpu, &mut harness);
        assert_page(&image, "settings_customisation_components");
        gpu.assert_no_validation_errors();
    }
}
