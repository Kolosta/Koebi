# Components

`koebi-ui` draws Koebi's interface components from design tokens
([ADR 0011](../adr/0011-token-driven-components.md)). This page explains how a component
reads its tokens and how to write a new one.

## The components

| Component | Method | Tokens |
|---|---|---|
| Button | `Ui::button(id, rect, icon)` | `component.button.*` |
| Toggle button | `Ui::toggle(id, rect, &mut on, icon)` | `component.toggle.*` |
| Checkbox | `Ui::checkbox(id, rect, &mut checked)` | `component.checkbox.*` |
| Slider | `Ui::slider(id, rect, &mut value, range)` | `component.slider.*` |
| Drag value | `Ui::drag_value(id, rect, &mut value, range, step)` | `component.drag-value.*` |
| Panel | `Ui::panel(rect, level)` returns the content area | `component.panel.*` |
| Separator | `Ui::separator(rect)` | `component.separator.*` |
| Color swatch | `Ui::swatch(id, rect, color, selected)` | `component.color-swatch.*` |
| Theme card | `Ui::theme_card(id, rect, &preview, selected)` | `component.theme-card.*` |

Line icons (`Icon::Check`, `Icon::Plus`, chevrons...) use `component.icon.*`. Components
have no text yet: icons stand in for labels, and theme cards draw text as bars.

[`crates/koebi-ui/tests/golden/`](../../crates/koebi-ui/tests/golden/) shows every component
in the dark and light themes, and the application shows them all in its
[gallery](gallery.md).

## One frame

```rust
use koebi_ui::{Input, StyleCache, Ui, UiState, WidgetId};

// Once per window.
let mut styles = StyleCache::new(&design);
let (mut state, mut input) = (UiState::new(), Input::new());

// Platform events feed the input: input.pointer_moved(p), input.primary_button(down, p).

// Every frame.
scene.clear();
let styles = styles.update(&design); // resolves again only if the design system changed
scene.set_clear_color(styles.background);
let mut ui = Ui::new(&mut scene, styles, &input, &mut state);
if ui.button(WidgetId::new("apply"), apply_rect, Some(Icon::Check)).clicked {
    // ...
}
ui.slider(WidgetId::new("radius"), slider_rect, &mut radius, 0.0..=16.0);
drop(ui); // ends the frame for `state`
input.end_frame();
```

- **Ids** identify a widget across frames, so that a press or a drag survives redraws.
  Derive the ids of repeated widgets with `WidgetId::with(index)`.
- **Responses** say what happened this frame: `hovered`, `pressed`, `held`, `clicked`,
  `released`, `changed` and `drag_delta`.
- **Values** belong to the application: components take `&mut` references and report
  `changed`.
- **What is on top wins** ([ADR 0013](../adr/0013-layered-hit-testing.md)): each frame
  registers the rectangles of its components, and the next frame resolves the pointer
  against them: the highest layer (`Ui::on_layer`: base, floating, overlay), then the
  component drawn last. Nothing under it is hovered or clicked, so components may overlap.
- **Zones** (`Ui::zone`) group components under a shortcut context; the chain of zones under
  the pointer (`UiState::contexts_at`) is what the keymap resolves, innermost first.
  `Ui::interact_with_cursor` and `Ui::zone_with_cursor` choose the cursor
  (`UiState::cursor`).
- **Redraws**: hover and press states follow the pointer, so the application redraws after
  every input event, and after every change of the design system.

## How components read tokens

Each component has a style made of plain values, filled from its component tokens by a
`resolve` function:

```rust
pub struct ButtonStyle {
    pub background: StateColors, // normal, hovered, pressed
    pub label: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub corner_radius: f32,
}

impl ButtonStyle {
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            background: StateColors {
                normal: tokens.get(component::BUTTON_BACKGROUND_COLOR_DEFAULT),
                hovered: tokens.get(component::BUTTON_BACKGROUND_COLOR_HOVER),
                pressed: tokens.get(component::BUTTON_BACKGROUND_COLOR_PRESSED),
            },
            label: tokens.get(component::BUTTON_LABEL_COLOR_DEFAULT),
            border_color: tokens.get(component::BUTTON_BORDER_COLOR_DEFAULT),
            border_width: tokens.get(component::BUTTON_BORDER_WIDTH_DEFAULT),
            corner_radius: tokens.get(component::BUTTON_CORNER_RADIUS_DEFAULT),
        }
    }
}
```

`tokens.get` returns the token's Rust type (`Color`, `f32`...), so a dimension token cannot
land in a color field. `Resolve` is implemented by the `DesignSystem` (the active theme with
every layer) and by any `View`: `ThemePreview::of(&design, theme)` resolves another theme,
with its overrides and the color vision setting, for a theme card.

`Styles::resolve` resolves every component's style together. `StyleCache` keeps them in step
with the design system and resolves again only after a change: a slider that edits a token
every frame costs one resolution per frame, and frames without changes read no token.

## Writing a component

1. **Tokens.** List what the component needs: colors per state, sizes, radii, widths. Reuse
   the component's existing tokens; add the missing ones to its file in `koebi-tokens`
   (`src/component/<category>/<component>.rs`, created for a new component, see
   [defining tokens](README.md#defining-tokens)) under the component's name, aliasing
   semantic tokens (or holding a value when no semantic token fits). Avoid aliasing another
   component's tokens: two components that look alike today may diverge tomorrow.
2. **Style.** In `crates/koebi-ui/src/widgets/<component>.rs`, define `<Component>Style` with
   one field per token and its `resolve` function. Use `StateColors` for per-state colors.
   Add the style to `Styles` and to `Styles::resolve`.
3. **Drawing.** Add the method to `Ui` in the same file: call `self.interact(id, rect)` for
   interactive components, update the value and set `response.changed`, then push primitives
   with the style's values. Read `self.styles()` (it outlives the borrow of `self`), never a
   token. Keep edges on whole logical pixels (`widgets::snap`) and allocate nothing.
4. **Tests.**
   - Unit tests beside the code: interaction (clicks, drags, values, clamping) and the
     primitives drawn.
   - A pixel test in `crates/koebi-ui/tests/pixels.rs` proving that the component's tokens
     reach the screen: override a token, render offscreen, read the pixel.
   - Add the component to the sheet in `tests/regression.rs`, run `cargo xtask bless` and
     review the golden images.
5. **Docs.** Add the component to the table above and its new tokens to the
   [porting report](porting.md#added-after-the-port).
