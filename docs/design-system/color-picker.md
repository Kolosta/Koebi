# Color fields and color pickers

`koebi-ui` edits colors with a **color field** (`ColorField`), one interface unit high, which
opens a **color picker** (`ColorPicker`) in a menu's panel: below the field as a dropdown,
their corners meeting, or as a panel of its own. The gallery's Colors panel shows both, with the
settings below to try.

## The field

- Outlined in a neutral gray of its own (`color-field.outline.*`): the outline marks the field
  against a background of a close color without taking the eye, and stays when the
  interface's borders go (its width is a value, not `semantic.border.width.thin`).
- Each half shows the color's transparency its own way (`Transparency`): opaque, over what lies
  behind, over white, over black, or over a checkerboard. By default the left half is opaque
  and the right half over the checkerboard.

## The picker

- **An area of two components**, its marker a white ring thinly outlined in black. Its real
  corners are reachable under the rounded ones: a drag past the frame clamps to the square,
  and the marker covers the corner.
- **Strips** of the third component and of the opacity (`ColorStrip`, vertical or horizontal),
  each marked by two white triangles outlined in black, pointing inward and passing the strip
  by 2 pixels (`color-picker.arrow.*`). The opacity strip lies over a checkerboard.
- **Swatches**: the current color, the color when the picker opened, and the original one (a
  token's own value, without the user's overrides); a click on the last two brings them back.
- **An eyedropper** below the swatches: a magnifier follows the pointer, a click picks the
  pixel under it, Escape or the secondary button gives up.
- **The switch of the model** the fields edit, as grouped buttons across the picker, then **the
  fields**, stacked and grouped (`FieldGroup`), each a slider of its component.
- **What the caller chooses**: a title, a palette of swatches, the order of the parts
  (`PickerSection`), the shape.

Dragging the area or a strip hides the pointer, which comes back at the mark on release
(`Ui::lock_pointer`); Shift makes the drag ten times finer (`color-picker.fine-factor`);
Escape brings back the color the drag started from. Every change shows at once: the color is
written on every frame something moves it.

## Shapes

| Shape | Area | Strip | Seen in |
| --- | --- | --- | --- |
| `SquareSv` (default) | saturation across, value up | hue | Photoshop, DaVinci Resolve |
| `SquareHs` | hue across, saturation up | value | Blender's "HS + V" |
| `WheelHsv` | hue around, saturation outward | value | Blender's HSV circle |
| `WheelHsl` | hue around, saturation outward | lightness | Blender's HSL circle |
| `WheelOklch` | hue around, chroma outward, at one OKLCH lightness | lightness | perceptual pickers |
| `RingTriangle` | hue on a ring, saturation and value in the triangle | none | GIMP, Krita |
| `RingSquare` | hue on a ring, saturation and value in the square | none | Krita |

Hue 0 (red) lies at the top of wheels and rings, growing clockwise.

## Display models

How an area or a strip is painted decides what the eye sees:

- **HSV** is a cylinder over gamma-encoded sRGB: every hue at full value and saturation is the
  brightest color of that hue. But those colors are not equally bright to the eye: yellow,
  cyan and magenta are much lighter than blue and red, so an HSV wheel shows bright **spokes**
  at the secondary colors. HSV is still what most people know, and the SV square is exactly
  one hue's slice.
- **HSL** puts every pure hue halfway between black and white; its spokes are the same.
- **OKLCH** is perceptual: one lightness, one chroma, every hue, and the colors look equally
  light all around. Its wheel has no spokes, which is what a picker made for the eye shows.
  At a high chroma, some hues leave the sRGB gamut: their channels clip (the edge of the OKLCH
  wheel, `OKLCH_CHROMA`, is chosen to keep most of them inside).

The fill shader (`crates/koebi-render/shaders/fill.frag`, `Fill`) and the picker's mapping of
positions (`widgets/color/shape.rs`) use the same formulas, so that the color under the
marker is the color picked.

## Models of the fields

RGB (0 to 255), HSV and HSL (hue in degrees, the rest in percent), OKLCH (lightness in
percent, chroma, hue in degrees), CMYK (the naive conversion, without an ink profile: a
reference, not a print workflow) and hex (`#RRGGBB`, or `#RRGGBBAA` with an opacity). The
opacity is a field of its own in percent, but in hex. Grays keep their hue when their
saturation reaches zero, so that raising it again comes back to the same hue.

## The eyedropper

It picks inside Koebi's windows: the picker asks for the pixels around the pointer
(`Ui::want_screen_pixels`), and the application copies them from the frame it presents
(`Presenter::request_pixels`): an 11 × 11 square, read back once the GPU finished that frame,
frames later and without waiting. The magnifier shows them; a press picks the pixel under
the pointer, or, when the pointer moved faster than the pixels came back, the nearest one read
(the one the magnifier outlines). A press always ends picking. Pixels are forgotten when
picking starts and ends, and the eyedropper does not show where the window's frames cannot be
read back (`Presenter::can_read_pixels`, `UiState::set_screen_readable`).

Picking anywhere on the screen, outside Koebi, needs each system's own capture API (the
desktop duplication of Windows, Screen Recording on macOS, which asks for a permission, the
screenshot portal on Wayland): it is not done yet.
