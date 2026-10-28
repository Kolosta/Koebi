# Golden images of koebi-ui

Reference renders used by [`../regression.rs`](../regression.rs): every component in its main
states, drawn with the styles of Koebi's built-in themes. See
[testing.md](../../../../docs/development/testing.md#golden-images).

They fail when a component draws differently: its geometry, its states, or the tokens its
style reads. A change of a token value in `koebi-tokens` changes them too, on purpose: these
images show what the themes look like.

## The component sheet

`component_sheet()` draws, with the pointer over the plus button:

| Row | Components |
|---|---|
| 1 | a button, a hovered button with a plus icon, a toggle off and on, a checkbox unchecked and checked, three icon buttons (minus, cross, chevron up) |
| 2 | a slider at 30 %, a bounded number field at 60 % with its bar |
| 3 | a first-level panel holding a second-level panel, a separator and a third-level panel; three swatches: opaque, translucent over the checkerboard, selected |
| 4 | theme cards of the dark (selected) and light themes |

| File | Size | Active theme |
|---|---|---|
| `components_dark.png` | 320x184 | dark (the base theme) |
| `components_light.png` | 320x184 | light |

The theme cards always show the dark and light themes; only the cards' frames follow the
active theme.

## The lists

`list_scene()` draws a tree in the dark theme, with the embedded fonts: Scene holding Camera,
Lights (with Sun and a long-named fill light, cut with an ellipsis) and Props (folded,
showing the 1 row it hides), then Background and Notes with color marks. Camera is selected
and Sun active.

| File | Size | Shows |
|---|---|---|
| `tree.png` | 200x160 | the pointer over Lights: zebra rows, lines, fold arrows, the hovered, selected and active rows |
| `tree_drag_into.png` | 200x160 | Background dragged into Lights: the frame around Lights, the dragged row faded, its label near the pointer, the selection unchanged |
| `tree_drag_after.png` | 200x160 | the same drag below Fill, the pointer at Lights' level: the line after Lights |

## The text fields

`text_field_sheet()` draws six text fields, 184x22, in the dark theme: `Cube.001`, an empty
search field (placeholder and magnifier), centered text, `Snap to grid` editing with `grid`
selected (focus border, selection, caret), a long name cut with an ellipsis, and a disabled
field.

| File | Size | Shows |
|---|---|---|
| `text_fields_dark.png` | 200x190 | every state of a text field but hover |

## The number fields

`number_field_sheet()` draws number fields, 184x22: plain (`1.50`), a width in millimetres
with its label, opacity on a linear bar, frequency on a logarithmic bar, an integer, a
labelled bounded field, a field dragged by three steps (`2.30`), a vertical field on a bar (too
narrow for its value, which it leaves out), and a disabled field.

| File | Size | Shows |
|---|---|---|
| `number_fields_dark.png` | 232x236 | the pointer over the labelled bounded field: its step buttons, its text in place |
| `number_fields_light.png` | 232x236 | the same in the light theme |
| `number_fields_drag_dark.png` | 232x236 | the drag in progress, the button still down: drag background and border |
| `number_fields_error_dark.png` | 232x236 | the first field typed with `2 * (3`, kept open after Enter with the error border |

## The property group

`property_group_sheet()` draws a first-level panel holding a property group: Name (a text
field), Location X, Y, Z in millimetres as a vector, a checkbox in the input column, and a
disabled row.

| File | Size | Shows |
|---|---|---|
| `property_groups_dark.png` | 320x232 | labels on 35 % of the width, sub-groups, a vector, a disabled row |

## Comparison and updates

Same tolerance as the renderer's golden images: `Tolerance::new(3, 8)`. Never edit these
files by hand; after an intended visual change run `cargo xtask bless` and review the images
in the pull request.
