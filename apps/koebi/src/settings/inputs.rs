//! The Inputs page: the distances and delays of interaction, which are tokens, by theme.

use koebi_core::expr::{MILLISECOND, PIXEL};
use koebi_design::{DesignSystem, TokenId};
use koebi_tokens::{component, semantic};
use koebi_ui::{Ui, WidgetId};

use super::Stack;
use super::options::{OptionDesc, OptionGroup};

const ID: WidgetId = WidgetId::new("settings-inputs");

const fn pixels(token: TokenId, label: &'static str) -> OptionDesc {
    OptionDesc {
        token,
        label,
        unit: Some(&PIXEL),
        decimals: 0,
    }
}

const fn millis(token: TokenId, label: &'static str) -> OptionDesc {
    OptionDesc {
        token,
        label,
        unit: Some(&MILLISECOND),
        decimals: 0,
    }
}

const fn factor(token: TokenId, label: &'static str) -> OptionDesc {
    OptionDesc {
        token,
        label,
        unit: None,
        decimals: 3,
    }
}

/// The Inputs page's state.
#[derive(Debug)]
pub(super) struct InputsPage {
    groups: Vec<OptionGroup>,
}

impl InputsPage {
    pub(super) fn new() -> Self {
        use component as c;
        use semantic as s;
        let groups = vec![
            OptionGroup::new(
                "Pointer",
                &[
                    pixels(s::INPUT_DRAG_THRESHOLD.id(), "Drag start distance"),
                    millis(s::INPUT_DOUBLE_CLICK_TIME.id(), "Double click time"),
                ],
            ),
            OptionGroup::new(
                "Hovering",
                &[
                    millis(s::INPUT_TOOLTIP_DELAY_SHORT.id(), "Tooltip delay, short"),
                    millis(s::INPUT_TOOLTIP_DELAY.id(), "Tooltip delay, normal"),
                    millis(
                        s::INPUT_TOOLTIP_DELAY_STATIONARY.id(),
                        "Tooltip delay, long",
                    ),
                    millis(s::INPUT_HOVER_DELAY.id(), "Hover delay"),
                ],
            ),
            OptionGroup::new(
                "Holding",
                &[
                    millis(s::INPUT_REPEAT_DELAY.id(), "Repeat delay"),
                    millis(s::INPUT_REPEAT_INTERVAL.id(), "Repeat interval"),
                ],
            ),
            OptionGroup::new(
                "Number fields",
                &[
                    pixels(
                        c::NUMBER_FIELD_DRAG_STEP_DISTANCE.id(),
                        "Drag distance of a step",
                    ),
                    factor(
                        c::NUMBER_FIELD_DRAG_FINE_FACTOR.id(),
                        "Shift, without a bar",
                    ),
                    factor(
                        c::NUMBER_FIELD_DRAG_BAR_FINE_FACTOR.id(),
                        "Shift, with a bar",
                    ),
                    factor(c::NUMBER_FIELD_DRAG_COARSE_FACTOR.id(), "Steps in a unit"),
                ],
            ),
            OptionGroup::new(
                "Menus",
                &[
                    millis(s::INPUT_SUBMENU_DELAY.id(), "Submenu delay"),
                    millis(s::INPUT_SUBMENU_GRACE.id(), "Submenu grace"),
                    pixels(s::INPUT_MENU_CLOSE_DISTANCE.id(), "Closing distance"),
                ],
            ),
            OptionGroup::new(
                "Scrolling",
                &[pixels(c::SCROLLBAR_WHEEL_STEP_DEFAULT.id(), "Wheel step")],
            ),
            OptionGroup::new(
                "Docking",
                &[
                    pixels(
                        c::ZONE_TAB_DRAG_THRESHOLD.id(),
                        "Drag start distance of a tab",
                    ),
                    millis(c::ZONE_TAB_REVEAL_DELAY.id(), "Tab bar delay"),
                    millis(c::ZONE_TAB_REVEAL_GRACE.id(), "Tab bar grace"),
                    millis(
                        c::ZONE_TAB_DROP_PREVIEW_ANIM_DURATION.id(),
                        "Drop preview glide",
                    ),
                ],
            ),
        ];
        Self { groups }
    }

    /// Opens the option at `option` of the group at `group`, and its resolution chain.
    #[cfg(test)]
    pub(super) fn open_option(&mut self, group: usize, option: usize, chain: bool) {
        self.groups[group].open_option(option, chain);
    }

    pub(super) fn show(&mut self, ui: &mut Ui<'_>, stack: &mut Stack, design: &mut DesignSystem) {
        for (index, group) in self.groups.iter_mut().enumerate() {
            group.show(ui, stack, ID.with(index as u64), design);
        }
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::Vec2;
    use koebi_design::{Layers, Scope};

    use super::super::Page;
    use super::super::tests::Harness;
    use super::*;

    #[test]
    fn every_option_is_a_number_with_a_range() {
        let design = DesignSystem::new(&koebi_tokens::KOEBI).unwrap();
        let view = design.view(design.active_theme(), Layers::ALL).unwrap();
        let page = InputsPage::new();
        let options: Vec<_> = page
            .groups
            .iter()
            .flat_map(OptionGroup::descriptions)
            .collect();
        assert!(options.len() >= 15);
        for option in options {
            assert!(design.range(option.token).is_some(), "{}", option.label);
            assert!(view.value(option.token).is_some(), "{}", option.label);
        }
    }

    #[test]
    fn the_input_of_a_closed_option_edits_the_application_and_the_icon_brings_it_back() {
        let mut harness = Harness::new();
        let at = harness.page_button(Page::Inputs);
        harness.click(at);
        harness.settle();
        let token = semantic::INPUT_DRAG_THRESHOLD;
        let before = harness.design.get(token);
        assert!(harness.design.override_of(Scope::Global, token).is_none());
        // The first option's input lies at the end of its header, below the group's header.
        let (input, reset) = header_input(&harness, 0);
        harness.input.pointer_moved(input);
        harness.frame();
        harness.input.primary_button(true, input);
        harness.frame();
        harness.input.pointer_moved_by(Vec2::new(30.0, 0.0));
        harness.frame();
        harness.input.primary_button(false, input);
        harness.frame();
        assert!(harness.design.override_of(Scope::Global, token).is_some());
        assert!(harness.design.get(token) > before);
        harness.settle();
        harness.click(reset);
        harness.settle();
        assert!(harness.design.override_of(Scope::Global, token).is_none());
        assert!((harness.design.get(token) - before).abs() < 1e-3);
    }

    /// Opens the Inputs page and lets it settle.
    fn inputs_page() -> Harness {
        let mut harness = Harness::new();
        let at = harness.page_button(Page::Inputs);
        harness.click(at);
        harness.settle();
        harness
    }

    /// Where the dot of the first group's header lies.
    fn group_dot(harness: &Harness) -> Vec2 {
        let styles = koebi_ui::Styles::resolve(&harness.design);
        let panel = styles.collapsible_panel;
        let margin = harness.design.get(component::SIDE_PANEL_MARGIN_DEFAULT);
        let row = harness.design.get(semantic::SIZE_CONTROL_HEIGHT);
        let right = Harness::window().max.x - margin;
        Vec2::new(
            right - panel.header_padding - row * 0.5,
            margin + panel.header_height * 0.5,
        )
    }

    #[test]
    fn a_group_shows_a_dot_while_an_option_is_changed_and_it_brings_every_change_back() {
        let mut harness = inputs_page();
        let (drag, click) = (
            semantic::INPUT_DRAG_THRESHOLD,
            semantic::INPUT_DOUBLE_CLICK_TIME,
        );
        let base = (harness.design.get(drag), harness.design.get(click));
        // Nothing changed: no dot, a click on its place changes nothing.
        let dots = |harness: &mut Harness| {
            harness.settle();
            harness.frame().rect_count()
        };
        let none = dots(&mut harness);
        harness
            .design
            .set_value(Scope::Global, drag, base.0 + 2.0)
            .unwrap();
        harness
            .design
            .set_value(
                Scope::ActiveTheme,
                click,
                base.1 + core::time::Duration::from_millis(50),
            )
            .unwrap();
        // Changed: the group and both options show their dot.
        assert!(dots(&mut harness) >= none + 3);
        harness.click(group_dot(&harness));
        harness.settle();
        for scope in [Scope::Global, Scope::ActiveTheme] {
            assert!(harness.design.override_of(scope, drag).is_none());
            assert!(harness.design.override_of(scope, click).is_none());
        }
        assert_eq!(dots(&mut harness), none);
    }

    #[test]
    fn the_secondary_button_on_a_group_dot_brings_back_one_scope_only() {
        let mut harness = inputs_page();
        let drag = semantic::INPUT_DRAG_THRESHOLD;
        let base = harness.design.get(drag);
        harness
            .design
            .set_value(Scope::Global, drag, base + 2.0)
            .unwrap();
        harness
            .design
            .set_value(Scope::ActiveTheme, drag, base + 1.0)
            .unwrap();
        harness.settle();
        let dot = group_dot(&harness);
        harness.input.pointer_moved(dot);
        harness.frame();
        harness
            .input
            .button(koebi_ui::PointerButton::Secondary, true, dot);
        harness.frame();
        harness
            .input
            .button(koebi_ui::PointerButton::Secondary, false, dot);
        harness.settle();
        // The menu opens at the pointer, within the page; its first item brings back the
        // application's change.
        let menu = harness.settings.inputs.groups[0]
            .menu()
            .rect()
            .expect("the menu is open");
        let row = harness.design.get(component::MENU_ITEM_HEIGHT_DEFAULT);
        let padding = harness.design.get(component::MENU_PADDING_VERTICAL_DEFAULT);
        let first = Vec2::new(menu.center().x, menu.min.y + padding + row * 0.5);
        harness.click(first);
        harness.settle();
        assert!(harness.design.override_of(Scope::Global, drag).is_none());
        assert!(
            harness
                .design
                .override_of(Scope::ActiveTheme, drag)
                .is_some()
        );
    }

    #[test]
    fn the_dot_beside_an_input_brings_back_its_scope_only() {
        let mut harness = inputs_page();
        harness.settings.inputs.open_option(0, 0, false);
        let drag = semantic::INPUT_DRAG_THRESHOLD;
        let base = harness.design.get(drag);
        harness
            .design
            .set_value(Scope::Global, drag, base + 2.0)
            .unwrap();
        harness
            .design
            .set_value(Scope::ActiveTheme, drag, base + 1.0)
            .unwrap();
        harness.settle();
        // The option's body: the application's column, then the current theme's, each with its
        // dot at its end, below their titles.
        let styles = koebi_ui::Styles::resolve(&harness.design);
        let panel = styles.collapsible_panel;
        let margin = harness.design.get(component::SIDE_PANEL_MARGIN_DEFAULT);
        let row = harness.design.get(semantic::SIZE_CONTROL_HEIGHT);
        let right = Harness::window().max.x - margin - styles.panel.border_width;
        let y = margin
            + panel.header_height
            + panel.content_padding
            + panel.header_height
            + panel.content_padding
            + row * 1.5;
        let theme_dot = Vec2::new(right - styles.panel.padding - row * 0.5, y);
        harness.click(theme_dot);
        harness.settle();
        assert!(
            harness
                .design
                .override_of(Scope::ActiveTheme, drag)
                .is_none()
        );
        assert!(harness.design.override_of(Scope::Global, drag).is_some());
    }

    /// Where the input and the reset button of the option at `index` of the first group lie.
    fn header_input(harness: &Harness, index: usize) -> (Vec2, Vec2) {
        let styles = koebi_ui::Styles::resolve(&harness.design);
        let panel = styles.collapsible_panel;
        let margin = harness.design.get(component::SIDE_PANEL_MARGIN_DEFAULT);
        let row = harness.design.get(semantic::SIZE_CONTROL_HEIGHT);
        let window = Harness::window();
        let right = window.max.x - margin - styles.panel.border_width;
        let y = margin
            + panel.header_height
            + panel.content_padding
            + panel.header_height * (index as f32 + 0.5);
        let reset = Vec2::new(right - panel.header_padding - row * 0.5, y);
        let input = Vec2::new(right - panel.header_padding - row * 3.0, y);
        (input, reset)
    }
}
