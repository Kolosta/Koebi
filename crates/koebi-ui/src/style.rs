//! [`Styles`]: what every component draws with, resolved from design tokens.
//!
//! Each component has a typed style (`ButtonStyle`, `SliderStyle`...) whose fields are read
//! from its `component.*` tokens by a `resolve` function. Resolving reads a few hundred
//! values, so it happens when the design system changes, not per component: drawing reads
//! plain fields.

use koebi_core::Color;
use koebi_design::{DesignSystem, Resolve};
use koebi_tokens::component;

use crate::ui::WidgetState;
use crate::widgets::{
    ButtonStyle, CheckboxStyle, DragValueStyle, PanelStyle, SeparatorStyle, SliderStyle,
    SwatchStyle, ThemeCardStyle, ToggleStyle,
};

/// One color per interaction state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StateColors {
    /// At rest.
    pub normal: Color,
    /// Under the pointer.
    pub hovered: Color,
    /// Held down.
    pub pressed: Color,
}

impl StateColors {
    /// The color of `state`.
    pub fn pick(&self, state: WidgetState) -> Color {
        match state {
            WidgetState::Normal => self.normal,
            WidgetState::Hovered => self.hovered,
            WidgetState::Pressed => self.pressed,
        }
    }
}

/// Size and stroke of line icons, from the `component.icon.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconStyle {
    /// Width and height of an icon, in logical pixels.
    pub size: f32,
    /// Stroke width.
    pub stroke_width: f32,
}

impl IconStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            size: tokens.get(component::ICON_SIZE_DEFAULT),
            stroke_width: tokens.get(component::ICON_STROKE_WIDTH_DEFAULT),
        }
    }
}

/// The style of every component, resolved together.
#[derive(Clone, Debug, PartialEq)]
pub struct Styles {
    /// Window background: the clear color of the scene.
    pub background: Color,
    /// Line icons.
    pub icon: IconStyle,
    /// [`Ui::button`](crate::Ui::button).
    pub button: ButtonStyle,
    /// [`Ui::toggle`](crate::Ui::toggle).
    pub toggle: ToggleStyle,
    /// [`Ui::checkbox`](crate::Ui::checkbox).
    pub checkbox: CheckboxStyle,
    /// [`Ui::slider`](crate::Ui::slider).
    pub slider: SliderStyle,
    /// [`Ui::drag_value`](crate::Ui::drag_value).
    pub drag_value: DragValueStyle,
    /// [`Ui::panel`](crate::Ui::panel).
    pub panel: PanelStyle,
    /// [`Ui::separator`](crate::Ui::separator).
    pub separator: SeparatorStyle,
    /// [`Ui::swatch`](crate::Ui::swatch).
    pub swatch: SwatchStyle,
    /// [`Ui::theme_card`](crate::Ui::theme_card).
    pub theme_card: ThemeCardStyle,
}

impl Styles {
    /// Reads every style from `tokens`: the active theme ([`DesignSystem`]) or a
    /// [`View`](koebi_design::View) of any theme.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            background: tokens.get(component::WINDOW_BACKGROUND_COLOR_DEFAULT),
            icon: IconStyle::resolve(tokens),
            button: ButtonStyle::resolve(tokens),
            toggle: ToggleStyle::resolve(tokens),
            checkbox: CheckboxStyle::resolve(tokens),
            slider: SliderStyle::resolve(tokens),
            drag_value: DragValueStyle::resolve(tokens),
            panel: PanelStyle::resolve(tokens),
            separator: SeparatorStyle::resolve(tokens),
            swatch: SwatchStyle::resolve(tokens),
            theme_card: ThemeCardStyle::resolve(tokens),
        }
    }
}

/// [`Styles`] that follow a design system's active theme, resolved again only when the
/// design system changed: every theme switch, override or color vision change shows on the
/// next frame, and frames without changes read no token at all.
#[derive(Clone, Debug)]
pub struct StyleCache {
    revision: u64,
    styles: Styles,
}

impl StyleCache {
    /// Resolves the styles of the active theme.
    pub fn new(design: &DesignSystem) -> Self {
        Self {
            revision: design.revision(),
            styles: Styles::resolve(design),
        }
    }

    /// The styles of the active theme, resolved again if `design` changed since the last
    /// call.
    pub fn update(&mut self, design: &DesignSystem) -> &Styles {
        if design.revision() != self.revision {
            self.revision = design.revision();
            self.styles = Styles::resolve(design);
        }
        &self.styles
    }

    /// The styles as last resolved.
    pub fn styles(&self) -> &Styles {
        &self.styles
    }
}

#[cfg(test)]
mod tests {
    use koebi_design::{Layers, Scope};
    use koebi_tokens::{KOEBI, semantic};

    use super::*;

    fn design() -> DesignSystem {
        DesignSystem::new(&KOEBI).expect("Koebi's schema is valid")
    }

    #[test]
    fn styles_read_component_tokens() {
        let design = design();
        let styles = Styles::resolve(&design);
        assert_eq!(
            styles.button.background.normal,
            design.get(component::BUTTON_BACKGROUND_COLOR_DEFAULT)
        );
        assert_eq!(
            styles.slider.grab.pressed,
            design.get(component::SLIDER_GRAB_COLOR_PRESSED)
        );
        assert_eq!(styles.checkbox.box_size, 16.0);
        assert_eq!(
            styles.icon,
            IconStyle {
                size: 16.0,
                stroke_width: 1.5
            }
        );
    }

    #[test]
    fn styles_of_another_theme_come_from_its_view() {
        let design = design();
        let light = design.find_theme("light").expect("built-in theme");
        let view = design.view(light, Layers::ALL).expect("live theme");
        let styles = Styles::resolve(&view);
        assert_eq!(
            styles.background,
            view.get(component::WINDOW_BACKGROUND_COLOR_DEFAULT)
        );
        assert_ne!(styles.background, Styles::resolve(&design).background);
    }

    #[test]
    fn the_cache_resolves_again_after_a_change_only() {
        let mut design = design();
        let mut cache = StyleCache::new(&design);
        let before = cache.styles().button.background.normal;
        assert_eq!(cache.update(&design).button.background.normal, before);

        let red = Color::from_hex(0xFF_00_00);
        design
            .set_value(Scope::Global, semantic::BACKGROUND_ACCENT_DEFAULT, red)
            .unwrap();
        // The button aliases the semantic token: the change flows to the component style.
        assert_eq!(cache.update(&design).button.background.normal, red);
    }
}
