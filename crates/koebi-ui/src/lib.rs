//! Koebi's interface components, drawn from design tokens.
//!
//! - **Styles**: each component draws with a typed style ([`ButtonStyle`], [`SliderStyle`]...)
//!   whose fields come from its `component.*` tokens. [`Styles::resolve`] reads every style
//!   from the active theme or from any other theme ([`koebi_design::Resolve`]), and
//!   [`StyleCache`] resolves again only when the design system changes: frames read plain
//!   fields, however many components they draw.
//! - **Immediate mode**: a [`Ui`] lives for one frame. Components are its methods: they react
//!   to the [`Input`], draw into a [`koebi_render::Scene`] and return a [`Response`]
//!   (hovered, pressed, clicked, dragged, changed). [`UiState`] keeps what outlives a frame:
//!   the widget that holds the pointer.
//! - **Layout**: callers give every component its rectangle, in logical pixels. A layout
//!   engine comes with the widget tree.
//! - **Text**: components have no labels until Koebi renders text; line [`Icon`]s stand in.
//!
//! # Example
//!
//! ```
//! use koebi_core::{Rect, Vec2};
//! use koebi_design::DesignSystem;
//! use koebi_render::Scene;
//! use koebi_ui::{Icon, Input, StyleCache, Ui, UiState, WidgetId};
//!
//! let design = DesignSystem::new(&koebi_tokens::KOEBI)?;
//! let mut styles = StyleCache::new(&design);
//! let (mut scene, mut state, mut input) = (Scene::new(), UiState::new(), Input::new());
//! let toggle = Rect::from_origin_size(Vec2::ZERO, Vec2::new(24.0, 24.0));
//! let mut grid = false;
//!
//! // One frame: components draw, and react to the input against the previous frame.
//! let mut frame = |input: &mut Input, grid: &mut bool| {
//!     scene.clear();
//!     let mut ui = Ui::new(&mut scene, styles.update(&design), input, &mut state);
//!     let response = ui.toggle(WidgetId::new("grid"), toggle, grid, Some(Icon::Plus));
//!     drop(ui);
//!     input.end_frame();
//!     response
//! };
//!
//! frame(&mut input, &mut grid);
//! // A click on the toggle, between two frames.
//! input.primary_button(true, Vec2::new(10.0, 10.0));
//! input.primary_button(false, Vec2::new(10.0, 10.0));
//! let response = frame(&mut input, &mut grid);
//!
//! assert!(response.changed && grid);
//! # Ok::<(), koebi_design::Error>(())
//! ```

mod icon;
mod id;
mod input;
mod routing;
mod style;
mod ui;
mod widgets;

pub use icon::Icon;
pub use id::WidgetId;
pub use input::Input;
pub use routing::Layer;
pub use style::{IconStyle, StateColors, StyleCache, Styles};
pub use ui::{Response, Ui, UiState, WidgetState};
pub use widgets::{
    ButtonStyle, CheckboxStyle, DragValueStyle, PanelLevel, PanelStyle, SeparatorStyle,
    SliderStyle, SwatchStyle, ThemeCardStyle, ThemePreview, ToggleStyle,
};

/// The styles of Koebi's base theme, for unit tests.
#[cfg(test)]
pub(crate) fn test_styles() -> Styles {
    let design =
        koebi_design::DesignSystem::new(&koebi_tokens::KOEBI).expect("Koebi's schema is valid");
    Styles::resolve(&design)
}
