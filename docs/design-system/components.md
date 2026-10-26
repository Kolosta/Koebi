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
| Menu | `Ui::menu(id, &mut state, &items, bounds)` returns the chosen item | `component.menu.*`, `component.separator.*` |
| Dropdown | `Ui::dropdown(id, rect, &mut state, current, &items, bounds)` | `component.dropdown.*`, then the menu's |
| Label | `Ui::label(text, rect, &font, color, align)`, `Ui::text_width(text, &font)` | `semantic.font.*`, `semantic.text.*` |
| Scroll area | `Ui::scroll_area(id, rect, &mut state, content_size, axes, add)` | `component.scrollbar.*` |
| List and tree | `Ui::list(id, rect, &mut state, &items, &options, &mut delegate)` returns a `ListResponse` | `component.list-row.*`, `component.outliner.*`, `component.table.row.*`, `component.drop-target.*` |

Line icons (`Icon::Check`, `Icon::Plus`, chevrons...) use `component.icon.*`. Labels use the
fonts given to `UiState::set_fonts`, in the roles of `Styles::text` (label, body, detail): one
line, cut with an ellipsis when too long ([ADR 0014](../adr/0014-text-basics.md)). Theme cards
still draw text as bars.

## Scroll areas

`Ui::scroll_area` clips its content to its rectangle and gives it the scrolled origin and the
viewport (`ScrollContent`); a `ScrollState` per area keeps the offset and the scrollbars'
animation. The wheel and trackpads scroll it when the pointer is over it, nothing else on top
(Shift turns the wheel sideways); one notch is `scrollbar.wheel-step`. Its scrollbars, rebuilt
from the previous application's, are overlays in the area's margin (`scrollbar.overlay.*`):
the margin is always kept, so content never shifts; a grab shows only when the content
overflows, thin at rest, growing and brightening as the pointer comes within
`scrollbar.overlay.proximity` (eased, the area asking for frames until it settles); dragging
the grab scrolls, a press on the track brings the grab under the pointer. Which axes scroll is
the content's choice (`ScrollAxes`): lists vertically, timelines both ways.

## Lists and trees

One component draws every list of the application (outliner, layers, color usage, messages);
`ListOptions` turns each behaviour on or off (`ListOptions::LIST` for a flat list,
`ListOptions::TREE` for everything).

- **Rows** come from the application in display order, depth first: a `ListItem` has the
  application's stable key, a depth (a row is the parent of the deeper rows after it), a label
  and an icon or a color. Rows are `list-row.height` high (one interface unit, or
  `ListOptions::row_height`), alternate `table.row.background.color.default` and `.alt`, and
  show per level a fold arrow slot `list-row.indent` wide, then the mark, the label cut with an
  ellipsis, and the items the `ListDelegate` draws at their end.
- **Folding**: a click on the arrow folds or unfolds a row, with Shift its whole subtree. The
  subtree slides in or out like a drawer (`list-row.expand.duration`,
  `list-row.collapse.duration`), and a folded row shows how many rows it hides
  (`list-row.count.color`). Lines run from parents along their children
  (`outliner.tree-line.*`).
- **Selection** lives in `ListState`, with the folds, the active row and the scroll position:
  a click selects a row and makes it active, Ctrl (Command on Apple platforms) adds or removes
  one, Shift selects the visible rows from the last row clicked, and a click on the empty
  space clears the selection. Double clicks and secondary presses come back in the
  `ListResponse` (`double_clicked`, `context_menu`). Over the list, the keyboard walks the
  rows: `list.next`, `list.previous`, `list.extend-next`, `list.extend-previous`,
  `list.unfold`, `list.fold` and `list.select-all` (arrows, Shift and arrows, Ctrl+A).
- **Drag and drop**: past `semantic.input.drag-threshold`, a press drags its row, or the
  topmost rows of the selection when the row was selected; the selection itself does not
  change. Where the pointer is in a row picks before it (top quarter), into it (middle half)
  or after it (bottom quarter); below an unfolded parent, rows go first inside it; below the
  last child of subtrees, the pointer's indentation picks the level. The list refuses places
  inside the dragged rows, `ListDelegate::can_drop` decides the rest, and a refused place
  gives way to the next one. A line (`list-row.drop-line.*`) or a frame (`drop-target.*`)
  previews the drop; the dragged rows fade and their label follows the pointer
  (`list-row.drag.*`); pressing against the top or bottom edge scrolls. Escape, Delete or a
  right click cancel (`list.cancel-drag`). The list moves nothing itself: when
  `ListResponse::dropped` says where, the application moves the rows `ListState::dragged`
  lists.

## Menus

- A `MenuState` per menu, kept by the application, opens it at a point (`open_at`, a context
  menu, usually on `Response::secondary_pressed`) or below a control (`open_below`, what a
  dropdown does). `Ui::menu` draws it on the floating layer, kept inside the window, and
  returns the item chosen this frame; items are actions with an optional shortcut text,
  separators and titles (`MenuItem`), disabled or marked as the current choice.
- While open, a menu takes the keyboard (`Ui::grab(MENU)`): `menu.next`, `menu.previous`,
  `menu.choose` and `menu.close` (arrows, Enter or Space, Escape by default) work wherever the
  pointer is. A press outside closes the menu and reaches nothing else; nothing under the menu
  receives its clicks.

[`crates/koebi-ui/tests/golden/`](../../crates/koebi-ui/tests/golden/) shows every component
in the dark and light themes, and the application shows them all in its
[gallery](gallery.md).

## The workspace

`Ui::workspace(&mut view, &mut workspace, window, &mut host)` draws a `koebi_workspace::Workspace`
([ADR 0015](../adr/0015-workspace-layout.md)) and edits it; `WorkspaceView` keeps what lasts
between frames, and the application's `EditorHost` draws each editor's content (clipped to its
area) and learns when a split copies an editor or a join removes some.

- **Areas** are rounded panels (`component.editor.*`) separated by gaps of
  `zone.separator.size`. Each has a top bar starting with the editor type dropdown
  (`EditorHost::kinds`, switching with `EditorHost::set_kind`), then the editor's own items
  (`EditorHost::top_bar`, placed with a `BarLayout`: groups from the left, from the right, and
  centred), and the editor's content below.
- **Tabs** (`component.zone-tab.*`): an area holding several editors shows a tab bar above its
  top bar; a click shows an editor, a drag along the bar reorders the tabs.
- **Docking**: dragging a tab or the background of a top bar (past `zone-tab.drag.threshold`)
  moves the editor: over another area's tab bar or top bar it goes into its tabs (a line shows
  where, and a single editor's area shows its tab bar meanwhile), in the middle of an area it
  joins its tabs, near an edge it docks beside the area, which splits in half (the half is
  previewed). The dragged tab follows the pointer; the area it leaves closes when empty, or
  the drop is refused if that area cannot close.
- **Gutters** show a resize cursor; dragging one moves every edge of its line, ten times finer
  with Shift and snapped to a 48th of the window with Ctrl, both as soon as they are pressed,
  never below `zone.min-size`. After `input.hover.delay`, a band with rounded ends shows the
  whole line that would move (`zone.separator.color.continuation` at its opacity), brighter
  between the two hovered areas (`zone.separator.color.default`).
- **Area menu**: a right click on a gutter offers to swap the two areas, join them, or split
  any area vertically or horizontally.
- **Corners** (`zone.corner.size`, crosshair cursor): dragged inwards, a corner splits its area
  along the axis the drag starts on (a horizontal drag makes a vertical line), where the pointer
  lets go; dragged into a neighbour, it joins that neighbour into its area. The previews are the
  same as the menu's.
- **Previews**: a split shows the two halves the area under the pointer would become, a gutter
  apart (`zone-overlay.split-preview`); a join shows the kept area lighter, the part of the
  removed area it takes darker, the parts neighbours take darker still, and the joined area
  outlined (`zone-overlay.join.*`), with an arrow cursor from the removed area to the kept one,
  or "not allowed" where nothing can join. A press applies the preview; a press where nothing
  can happen does nothing.
- **Cancel**: Escape, Delete or a right click (`workspace.cancel`) end any operation and restore
  the exact previous layout; Enter (`workspace.confirm`) applies a preview.

## One frame

```rust
use koebi_ui::{Input, StyleCache, Ui, UiState, WidgetId};

// Once per window.
let mut styles = StyleCache::new(&design);
let (mut state, mut input) = (UiState::new(), Input::new());

// Platform events feed the input: input.pointer_moved(p), input.button(button, down, p),
// input.scrolled(delta), input.set_modifiers(m); the keymap's actions: input.trigger(action).

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
