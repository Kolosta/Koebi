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
| 2 | a slider at 30 %, a drag value at 60 % with its step buttons |
| 3 | a first-level panel holding a second-level panel, a separator and a third-level panel; three swatches: opaque, translucent over the checkerboard, selected |
| 4 | theme cards of the dark (selected) and light themes |

| File | Size | Active theme |
|---|---|---|
| `components_dark.png` | 320x184 | dark (the base theme) |
| `components_light.png` | 320x184 | light |

The theme cards always show the dark and light themes; only the cards' frames follow the
active theme.

## Comparison and updates

Same tolerance as the renderer's golden images: `Tolerance::new(3, 8)`. Never edit these
files by hand; after an intended visual change run `cargo xtask bless` and review the images
in the pull request.
