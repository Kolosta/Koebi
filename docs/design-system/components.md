# Components

`koebi-ui` draws Koebi's interface components from design tokens
([ADR 0011](../adr/0011-token-driven-components.md)), as a library whose components take
their defaults from the tokens and every property from the call
([ADR 0017](../adr/0017-component-library.md)). This page explains how a component reads its
tokens and how to write a new one.

## The interface unit

A control is one interface unit high (`ui-unit`, through `semantic.size.control-height`). What
holds controls in a row is sized from it and leaves the same inset above and below them,
`semantic.spacing.control-inset` (2 pixels), which its own padding token aliases:

| What | Height | Rule |
|---|---|---|
| Button, dropdown, text field, number field, checkbox | one unit | fixed; `height` overrides it for one control |
| Top bar | one unit + `editor-top-bar.padding` on each side | fixed, and its items are one unit high |
| Tab bar | one unit + `zone-tab.padding.vertical` on each side | fixed; a tab reaches down to the top bar below it, its label one unit high |
| Application bar | one unit + `title-bar.padding` on each side | fixed; its menus and window buttons are one unit high |
| Menu item, list row | one unit, highlight included | fixed; the highlight lies inside and the pointer selects the whole row |
| Collapsible panel header | one unit (`panel.header.height`) + `panel.header.inset` on each side | fixed; the controls at its end are one unit high |
| Property group row | one unit | fixed; `row_with_height` asks for a taller one |
| Menu, list, panel content, property group | what they hold | grow with their content, then scroll or stop at a maximum |

Bars have a fixed height along their axis (a vertical bar, a fixed width); what lists things
grows. An item's interactive zone is always its whole row, margins included: margins only move
what shows.

## The components

| Component | Call | Tokens |
|---|---|---|
| Button, toggle button | `Button::new(id)` + options, then `.show(ui, rect)` | `component.button.*` |
| Button group | `ButtonGroup::horizontal()` or `::vertical()` + options, then `.show(ui, rect, &mut cells)` | `component.button.*`, `component.button.group.*` |
| Checkbox | `Checkbox::new(id, &mut checked)` + options, then `.show(ui, rect)` | `component.checkbox.*` |
| Number field | `NumberField::new(id, &mut value)` + options, then `.show(ui, rect)` returns a `NumberFieldResponse` | `component.number-field.*` |
| Property group | `PropertyGroup::new(ui, area)`, then `.row(ui, label)`, `.vector(ui, label, ["X", "Y", "Z"])`, `.unlabeled_row()`, `.wide_row()` return the inputs' rectangles | `component.property-group.*` |
| Text field | `TextField::new(id, &mut text)` + options, then `.show(ui, rect)` returns a `TextFieldResponse` | `component.text-field.*` |
| Panel | `Ui::panel(rect, level)` returns the content area | `component.panel.*` |
| Collapsible panel | `CollapsiblePanel::new(id, &mut state, title)` + options, then `.show(ui, area, add)` returns a `CollapsiblePanelResponse` | `component.panel.*` |
| Separator | `Ui::separator(rect)`; `Ui::section_title(rect, title)` heads a section of a container that never folds: the title, then a line to the end of the row | `component.separator.*` |
| Color field | `ColorField::new(id, &mut color, &mut state).show(ui, rect, bounds)`: the color one unit high, outlined in its own gray, each half showing transparency its way (`.halves(left, right)`), opening its picker as a dropdown (`.dropdown(true)`, the default) or a panel; `.shape`, `.title`, `.palette`, `.original`, `.sections`, `.eyedropper` set up the picker ([Color pickers](color-picker.md)) | `component.color-field.*` |
| Color picker | `ColorPicker::new(id, &mut state, &mut color).show(ui, rect)`, in place or opened by a field: an area, strips, swatches, an eyedropper, the switch of models and their fields, a palette | `component.color-picker.*` |
| Color strip | `ColorStrip::new(id, &mut state, &mut value, fill)`, vertical or horizontal (`.vertical(false)`), over a checkerboard (`.checker(true)`): one component, marked by two triangles | `component.color-picker.*` |
| Curve editor | `CurveEditor::new(id, &mut curve, &mut state).show(ui, rect)`, `CurveEditor::height(ui)`: the graph of an easing curve (`koebi_design::value::Curve`) over a grid; keys selected and dragged (Escape brings a dragged key back), added by a double click or "Add key", removed by the secondary button or "Remove key"; the selected key's place and its segment's easing typed in grouped fields. **Temporary**, until curves get a fuller editor; the curve stays valid (ends fixed, keys in time order, values within `Curve::LIMIT`, eight keys at most) | `component.curve-editor.*` |
| Field group | `FieldGroup::horizontal(rect, count, radius, border)` or `::vertical`: fields sharing their borders, only the group's outer corners rounded (`.cell(index)` gives each field's rectangle and corners) | the fields' |
| Color swatch | `Ui::swatch(id, rect, color, selected)` | `component.color-swatch.*` |
| Theme card | `Ui::theme_card(id, rect, &preview, selected)`: a miniature of shapes painted from a `ThemePreview` (surfaces, text, accent, the colors of states and of a chart), resolved from a theme's view or a color vision mode's | `component.theme-card.*` |
| Card | `Ui::card(id, rect, selected)` returns a `CardResponse` with the room inside its frame | `component.theme-card.*` |
| Reset dot | `Ui::reset_dot(id, area, hint)`: shown beside a changed value, `clicked` brings it back | `component.panel.override-badge.*` |
| Key caps | `Ui::key_caps(rect, &shortcut, align)` draws a shortcut as keys joined by `+`, each cap as wide as its label, with icons for Shift, Tab, Command, the Windows key and mouse buttons, named for the keyboard `UiState::set_keyboard` gives (`Keyboard::CURRENT` by default); `ShortcutCaps::of` gives the caps, `ShortcutCaps::of_modifiers` those of modifiers alone, `Ui::draw_caps` draws any; menu items show them with `MenuItem::caps(shortcut)` | `component.key-cap.*` |
| Capture field | `CaptureField::new(id, &mut state, shortcut)` + `.capture(Capture::Keyboard)`, then `.show(ui, rect)` records a shortcut: the modifiers held show as they come and go, the first other key, mouse button (any of them, `Input::mouse_buttons_pressed`) or wheel turn ends it with the modifiers held then; it takes the keyboard meanwhile (`CAPTURE`), and only Escape cancels (Delete and the right button are shortcuts too). It looks like a field, active while recording | `component.capture-field.*` |
| Preview | `Ui::preview(&styles, &mut state, add)` draws real components in other styles, inert | the styles given |
| Menu | `Ui::menu(id, &mut state, &items, bounds)` returns the chosen item (`MenuChoice`) | `component.menu.*`, `component.separator.*` |
| Popup | `Ui::popup(id, &mut state, size, bounds, add)` draws custom content | `component.menu.*` |
| Dropdown | `Dropdown::new(id, &mut state, &items)` + options, then `.show(ui, rect, bounds)` | `component.dropdown.*`, then the menu's |
| Tooltip | `Ui::tooltip(id, text, bounds)` after the widget | `component.tooltip.*` |
| Status bar | `Ui::status_bar(rect, &entries, end)`: items one interface unit high, the bar's inset above and below; `StatusEntry`s (key caps and a label) from the left, thin lines between them, as many as fit, the highest priorities kept and the order given kept; `end` on the right | `component.status-bar.*` |
| Label | `Ui::label(text, rect, &font, color, align)`, `Ui::text_width(text, &font)` | `semantic.font.*`, `semantic.text.*` |
| Scroll area | `Ui::scroll_area(id, rect, &mut state, content_size, axes, add)` | `component.scrollbar.*` |
| Scrollbar | `Scrollbar::new(id, &mut state, axis, &mut offset, visible, content)`, then `.show(ui, area)` | `component.scrollbar.*` |
| List and tree | `Ui::list(id, rect, &mut state, &items, &options, &mut delegate)` returns a `ListResponse` | `component.list-row.*`, `component.outliner.*`, `component.table.row.*`, `component.drop-target.*` |

Line icons (`Icon::Check`, `Icon::Plus`, chevrons...) use `component.icon.*`: polylines
whose segments are drawn by pairs joined at their corner, so that a chevron's tip or a check
mark's corner is covered once and keeps the weight of its arms. The chevrons that fold and open
(fold arrows, panel headers, dropdowns) take the finer `icon.stroke-width.chevron`.
`Ui::pixel_span` and `Ui::centered_on_line` place what must line up on the physical pixel grid:
a line covering whole pixels, and a shape centered on the middle of that line. Labels use the
fonts given to `UiState::set_fonts`, in the roles of `Styles::text` (label, body, detail): one
line, cut with an ellipsis when too long ([ADR 0014](../adr/0014-text-basics.md)). Theme cards
still draw text as bars.

## Buttons and checkboxes

Builders take their options in any order, then `size(ui)` measures the control and
`show(ui, rect)` draws it and returns its `Response`:

```rust
let save = Button::new(WidgetId::new("save"))
    .leading_icon(Icon::Check)
    .label("Save")
    .tone(Tone::ACCENT);
let size = save.size(ui);
if save.show(ui, Rect::from_origin_size(origin, size)).clicked {
    // ...
}
```

- **Tones**: a `Tone` is a `Variant` (default, accent, positive, negative, info, neutral,
  notice: `button.*`, then `button.<variant>.*`) and an optional `Hue` replacing its color
  (`Tone::POSITIVE.with_hue(Hue::Orange)`). A family has three colors (`button.hue.<hue>.fill`,
  `.subtle` and `.text`): filled variants take its fill, the default variant its subtle
  background and its text color. Hovered and pressed shades add `button.shade.hover.lightness`
  and `button.shade.pressed.lightness` to the OKLCH lightness (a selected filled variant,
  hovered, is shaded the same way); disabled ones keep `button.hue.disabled.tint` of the
  family over the disabled colors. A toggle of the default tone has the look of toggles
  (`toggle.*`: colors, corner radius and border width), like Blender's toggle theme.
- **States** (`ControlState`): normal, hovered, pressed, selected (a toggle switched on, the
  current choice), selected and hovered, selected and pressed (darker than selected, as
  pressed is darker than at rest: a toggle switched on never turns gray while pressed),
  active, disabled, selected and disabled (a toggle switched on, then disabled). **Selected**
  and **active** have their own tokens: selected is switched on or chosen and lasts, active
  holds the input for a while (a field being edited or dragged, a shortcut being recorded, a
  menu open: `semantic.background.app.active` and `semantic.border.color.active` for fields,
  `button.*.active` for a button's open menu). A disabled control ignores the pointer: it is
  never hovered nor
  pressed, but a selected one still shows it is on, its disabled colors keeping
  `button.selected.disabled.tint` of its selected ones (`checkbox.selected.disabled.tint`
  for a checked box). `States<T>` holds a value per state. Disabled controls and fields share
  `semantic.background.app.disabled`, between the surfaces and the fields.
- **Overrides**: every property takes a `Prop`, a token or a value:
  `.corner_radius(0.0)`, `.padding(semantic::SPACING_EDITOR_INSET)`,
  `.background(ControlState::Hovered, Color::from_hex(0xE0_30_30))`; `.corner_radii(...)`
  takes one radius per corner (`CornerRadii`), and the last of the two calls applies.
- **Content** (`Content`): a leading icon, a label or an icon, a status dot and a trailing
  icon, placed by `align`; a label too long is cut with an ellipsis, icons that do not fit
  within the paddings take them, as much on each side (an icon stays centered in a small
  square button), and content that still cannot fit is clipped to the button.
- **Size**: one interface unit high (`button.height`), never lower than the content (an icon,
  or the line box of the label's font); as wide as the content, paddings and borders within
  `button.min-width` and `.max_width(...)`, or `.width(...)` for a fixed width. A button
  showing an icon alone is square.
- **Behaviour**: the action fires on release (moving away first cancels), on press, or on the
  second press of a double click (`Activation`: within `semantic.input.double-click.time` of
  the first press and `semantic.input.drag-threshold` of its place, as platforms count
  them); `.repeat(true)` fires on press then every `semantic.input.repeat.interval` after
  `semantic.input.repeat.delay` while held under the pointer, pausing while the pointer is
  off the button, the frames asked for in advance; `.toggle(&mut on)` flips a value and shows
  the button selected while it is on. A disabled button ignores the pointer but still covers
  what lies under it.
- **Groups** (`ButtonGroup`): cells (`GroupCell::new(button).at(column, row).span(columns,
  rows)`) tile a grid, the Ctrl key above L and R for example; columns and rows fit their
  buttons, or take a fixed size or a share of the room (`Track`); cells without a place take
  the next free one along the group's direction, and a group in a rectangle smaller than its
  size shrinks its fitting tracks first. Neighbours share one border
  (`button.group.border.width`) whose color is the one of the cell of highest state priority
  (active, pressed, hovered, selected, rest, selected and disabled, disabled); only the
  group's outer
  corners are rounded. The
  `GroupResponse` says which cell fired, was double-clicked, pressed with the secondary
  button or hovered.
- **Checkboxes**: the box, a label beside it and an optional description below
  (`checkbox.description.color`), all of it clickable; `.mixed(true)` shows the mixed mark of a
  partly checked set, which a click checks. The box's colors (`background`,
  `checked_background`, `border_color`, `checked_border_color`), size, radius and border, the
  mark and the label colors take overrides like buttons; the first line grows to hold a larger
  box or font.

## Application bar and menu bars

`AppBar::new(id, title)` fills the top of a window: the caller's items from the start, the title in
the middle when it fits, and the window buttons at the end, flush with the corner and as high as
the bar. It is one interface unit high, and `title-bar.padding` above and below. The system moves the window
from the bar's background, and whatever lies over it takes the pointer
(`UiState::covering`): [ADR 0019](../adr/0019-window-chrome.md) tells the rest. Window buttons act
on the release (`AppBarResponse::command`). `.window_buttons(false)` and `.leading_inset(width)`
leave the room to the system's on macOS. Their glyphs (`Ui::window_glyph`) are rectangles on whole
physical pixels, and the cross two segments ending on the corners of its box
(`title-bar.glyph.*`).

`Ui::menu_bar` shows titles along a bar. A press opens a menu; while one is open, moving over
another title opens that one, and the left and right arrows go from menu to menu.

## Number fields

Blender's number button. `NumberField::new(id, &mut value)` edits any `Numeric`: `f32`,
`f64`, or integers (no decimals, whole steps). Options: `.range(min, max)` (or `.min`,
`.max`), `.step(step)` (1 for integers; for floats, about a hundredth of a bounded range, or 0.1), `.decimals(count)` (2 for floats),
`.unit(&unit)` (from `koebi_core::expr`, shown after the value and used for typed
expressions), `.label(text)`, `.bar(mapping)`, `.vertical()`, `.step_buttons(shown)`,
`.fine_mode(mode)`, `.fine_factor(factor)`, `.enabled(false)`, and the metrics and colors per state as tokens or values.

- **Showing**: the value rounded to its decimals, its unit in `number-field.unit.color`,
  centred, or after its label at the start. A bounded field with `.bar(BarMapping::Linear)`,
  `Logarithmic` or `Custom { to_fraction, from_fraction }` fills a bar to where its value
  lies. A vertical field fills from the bottom and leaves out a value too wide for it.
- **Step buttons** appear on hover at both ends, by default on a field without a bar only. Their
  hover zone lies inside the border, follows the rounded end of the field and is translucent
  (`number-field.step-button.background.opacity`), so that a bar shows through. A click adds a
  step; Shift a finer one (`number-field.drag.fine-factor`, or `.bar-fine-factor` on a bar); Ctrl
  goes to the next round unit (`number-field.drag.coarse-factor` steps, finer with Shift); Alt
  moves by a unit and keeps the hidden digits.
- **What a drag sets**: the value it shows, never digits hidden below it: `245.15874` dragged
  until it shows `245.20` is exactly `245.2`, and only typing goes finer. A drag that does not
  reach another shown value leaves the value as it was. Alt pressed before that keeps the hidden
  digits (`246.15874`); pressed later, there are none left, and its units count from what shows.
  Step buttons and the wheel follow the same rule.
- **The wheel**: Ctrl and the wheel over a field change its value, a step per notch, and what
  scrolls around the field stays still. Shift makes it finer and goes on and off at any time, as
  in a drag and with the same `FineMode`; it stops being finer where a notch would no longer
  show. It lasts as long as Ctrl stays held over the field.
- **Dragging**: a press locks the pointer at once (`Ui::lock_pointer`), and the field takes
  the keyboard (`NUMBER` context). Past the drag threshold, the travel the locked pointer
  reports (`Input::motion`) changes the value: a step per `number-field.drag.step-distance`
  for an unbounded field, the whole range along the bar for a bounded one. The left Shift does the field's
  `FineMode`, the right Shift the other one: `FromStart`, the default with a bar, makes the value
  follow the share of the drag done, as if the whole drag had been finer, and returns when Shift
  is released, as many times as wanted; `FromHere`, the default without a bar, goes on finer from
  the value reached, without a jump either way. Ctrl goes to round units and Alt moves by units
  keeping the hidden digits (`1.23`, `2.23`, `3.23`), both finer with Shift; without a bar, a unit
  takes the travel of a step, so that they go faster than plain steps. Escape or Delete
  (`number.cancel`) or a right click cancel at any time and restore the value (a right click that
  cancels asks for no context menu); Enter (`number.confirm`) or the release keep it. The pointer
  comes back where the press was, or at the end of the bar of a bounded field, also when the
  application draws the frame again with settled input. Travel past a bound does not count:
  coming back moves the value at once. Integers step by whole numbers, at least 1, Shift
  included.
- **Typing**: a release before the threshold types the value in a text field, with every
  decimal that matters and the unit. Keeping the edit evaluates the text as an expression
  with units in the field's unit (`2*3 + 1cm`, see `koebi_core::expr`). Enter on an invalid
  one keeps the edit open, its text and caret as they are, framed in
  `number-field.border.color.error`; Escape, or leaving an invalid edit (a press outside,
  Tab), leaves the value. A vertical field types in a box across it. Disabling a field drops
  what was typed; a quick click on another field ends this field's edit first.
- **Response**: `changed`, `dragging`, `editing`, and `ended` (`EditEnd::Confirmed` or
  `Cancelled`, also `confirmed()` and `cancelled()`) for a drag or an edit that ended this
  frame.

## Property groups

Labeled inputs in two columns, as in Blender's properties. `PropertyGroup::new(ui, area)`
lays rows out from the top of `area`; each call draws a label and returns the rectangle the
caller shows its input in:

- `row(ui, label)`: the label right-aligned at the end of the first column
  (`property-group.label.width-ratio` of the width, 35 % by default, minus
  `property-group.column.gap`), the input filling the rest; `row_with_height` for taller
  inputs (a list).
- `vector(ui, "Location", ["X", "Y", "Z"])`: a sub-group of its own whose label goes with its
  first component only ("Location X", then "Y", "Z"), drawn without allocating. A label too
  long for its column is cut part by part, so that the axis letter stays.
- `unlabeled_row()`: the input column alone (a checkbox, whose label follows its box);
  `wide_row()`: both columns (a button, a message).
- `start_group()` starts a sub-group: rows of a sub-group are `property-group.row.gap` apart,
  sub-groups `property-group.gap.size`. `set_enabled(false)` dims the labels that follow, like
  the disabled inputs beside them.
- `height()` is the height used so far; `measure(rows, groups)` what to reserve with the
  group's own metrics (a vector is a sub-group, and so is what follows it). `label_ratio`,
  `row_height` and `gaps` take tokens or values.

## Text fields

- **Editing** starts with a press on the field, which selects the whole text by default
  (`SelectOnFocus::All`; `End` puts the caret at the end, `Pointer` where the press is), with
  Tab from the previous field, or when the application asks (`Ui::edit_text_field(id)`: a
  name to type at once; a request no field answers within a frame is dropped). One field
  edits at a time. While it does, it takes the whole keyboard (`Ui::grab` with the
  `TEXT` context): no shortcut of the contexts under the pointer fires, not even one the text
  context does not bind. It also asks the platform for text input over itself
  (`Ui::text_input`: the on-screen keyboard of touch devices, input method windows).
- **Pointer**: a press places the caret (Shift extends the selection), a drag selects, a
  double press selects a word; a drag from the press that selected everything selects from
  the press instead, past the drag threshold. Presses count where they happened, even when
  the pointer moved or left before the frame. The text scrolls to keep the caret in view.
- **Keys** (`TEXT_ACTIONS`, rebindable like every action): moves by character, by word and
  to the ends, each with Shift to select; deletion by character or word; select all; undo
  and redo (by word for typing, one step per paste); copy, cut and paste. Defaults follow
  the platform: Command on Apple platforms, Ctrl elsewhere, Option on Apple platforms and
  Ctrl elsewhere for words. Keys and typed text between two frames all apply in the order
  they came (each action remembers how much text was typed before it:
  `Input::actions_in_text`). Typed text goes to one field a frame, and the field consumes
  the actions it uses.
- **Ending**: Enter keeps the text; Escape restores the text the edit started with; Tab and
  Shift+Tab keep it and edit the next or previous enabled field in the order the last frame
  showed them, wrapping around; a press outside keeps it (`OutsidePress::Confirm`) or
  restores it (`OutsidePress::Cancel`), also when the press lands on another field drawn
  before it. Another field starting to edit without a press, the field disabled or no longer
  shown keep the text. An edit that ended away from its field tells the field in its next
  frame, then is forgotten.
- **The text** changes as the user types: `TextFieldResponse::changed` says when, and
  `ended` (`EditEnd::Confirmed` or `EditEnd::Cancelled`) how the edit ended. A change the
  application makes to the text during an edit (an undo of the document) carries on in the
  edit, which can undo it.
- **Constraints**: `.filter(fn(char) -> bool)` keeps only some characters, typed or pasted,
  and `.max_chars(count)` limits the length. Pasted line breaks become spaces.
- **Clipboard**: the interface never reaches the platform. `Ui::copy_text` leaves the text
  in `UiState::copied_text` for the application to put on the clipboard after the frame;
  `Ui::request_paste` sets `UiState::paste_requested`, and the application answers with
  `UiState::set_pasted_text(text)` before the next frame, where the editing field inserts
  it. A paste the filter empties changes nothing.
- **Look**: a box of the field's colors per state (active is editing, with the
  `text-field.border.color.editing` focus border), an optional icon (`Icon::Search` for a
  search field), the text cut with an ellipsis while idle or a placeholder while empty, and
  while editing the selection (`text-field.selection.color`) and the caret
  (`text-field.caret.width`, blinking every `text-field.caret.blink-interval` from 50 ms up,
  which asks for the frames that turn it; shorter intervals keep it shown). Every property takes a token or a value at
  the call, as for buttons.

## Collapsible panels

Boxes holding trees of sections that fold, as in Blender's properties.
`CollapsiblePanel::new(id, &mut state, title)` keeps its fold in a `CollapsiblePanelState` the caller
owns (open or closed, the openness animating between, and the height of the content measured
on the last frame).

- **Header**: `panel.header.height` high and `panel.header.inset` above and below, a chevron
  (`panel.chevron.*`, centered on a pixel) pointing right while
  closed and turning down as the panel opens, an optional `.icon(icon)`, the title, and room
  for the caller's buttons (`.trailing(width)`, returned as `CollapsiblePanelResponse::trailing`;
  draw them after the panel so that they take the pointer). Under the pointer, the header
  lightens by `panel.header.hover.lightness`.
- **Content**: `show(ui, area, add)` calls `add(ui, rect)` with the panel's width inside the
  frame, indented as its header, from the top of its content down to the bottom of `area`
  (the room there is); `add` draws and returns the height it used (a nested panel's
  `height`). Rows inset themselves by `panel.padding`; nested panels take the rectangle as it
  is. The content draws while
  the panel is open or opening, clipped to the share shown, and the frame is drawn over it. A
  panel never measured shows its content from the frame after it first drew it (it asks for
  that frame at once); nested levels follow one frame apart, while they settle and while they
  fold.
- **Levels**: a panel shown in another's content is one level deeper on its own. Its
  backgrounds span its parent's whole content; only what it holds is indented, by
  `CollapsiblePanelStyle::indent` (the chevron's room and the header gap): its chevron lies
  below its parent's title, its content one indent further. `.level(level)` gives a panel
  shown on its own the colors of a deeper level. A first-level panel is framed and rounded,
  its header in `panel.header.background.color`; a nested panel's header always takes its
  parent's content color, and its content the next level's
  (`panel.body.background.color.level-*`): levels 1, 2 and 3, then 2 and 3 alternating, so
  that every content differs from the one holding it however deep. The response's
  `header_rect` is the header as wide as its backgrounds. `.status(icon, color)` shows an
  icon at the end of
  the title (a conflict inside, carried up to every panel holding it) and `.tint(color)` lays
  a translucent color over the header.
- **Folding**: a press on the header opens or closes it; the content is revealed along
  `panel.expand.duration` and `panel.expand.curve` (or `panel.collapse.*`), and the panel asks
  for frames while it moves. The response's `height` is what to advance by, then
  `panel.gap.size` before the next panel.

## Scroll areas

`Ui::scroll_area` clips its content to its rectangle and gives it the scrolled origin and the
viewport (`ScrollContent`); a `ScrollState` per area keeps the offset and the scrollbars'
animation. The wheel and trackpads scroll it when the pointer is over it, nothing else on top
(Shift turns the wheel sideways); one notch is `scrollbar.wheel-step`. Its scrollbars, rebuilt
from the previous application's, are overlays at the edges of the area (`scrollbar.overlay.*`):
they lie over the content and never take room from it, on the zebra rows of a list too, so
content never shifts. Content that must stay clear of them keeps a margin of its own. A grab
shows only when the content
overflows, thin at rest, growing and brightening as the pointer comes within
`scrollbar.overlay.proximity` (eased, the area asking for frames until it settles); dragging
the grab scrolls, a press on the track brings the grab under the pointer. Which axes scroll is
the content's choice (`ScrollAxes`): lists vertically, timelines both ways.

The scrollbar is a component of its own, for what scrolls its own way (a canvas, a timeline):
`Scrollbar::new(id, &mut state, ScrollAxis::Vertical, &mut offset, visible, content)` shows
in the margin at the end of the area passed to `show` (right for vertical, bottom for
horizontal) and changes `offset`, from 0 to `content - visible`. Its `ScrollbarState` keeps
the grab's growth (`growth()`, eased along `scrollbar.overlay.grow.duration` and `.curve`)
and its drag (`is_dragging()`). `.near(false)` keeps it at rest while the pointer is not over
what it scrolls; `ScrollbarResponse` says whether it showed and changed the offset. Scroll
areas use one per axis.

## Lists and trees

A row's hover, selected and active states show in a highlight inside it, with rounded corners
(`list-row.highlight.*`), while the zebra keeps the whole row and the pointer reaches all of it.
In a tree, the line along the children of a row (`outliner.tree-line.*`) starts just below the
top of its first child and ends just above the bottom of its last one, below the tip of the
row's chevron: both are placed on the same physical pixels (`Ui::pixel_span`,
`Ui::centered_on_line`) at every scale factor. Each row chooses its look
(`ListItem::with_guide`): `TreeGuide::Line`,
`TreeGuide::Color(color)` or `TreeGuide::Dotted`.

One component draws every list of the application (outliner, layers, color usage, messages);
`ListOptions` turns each behaviour on or off (`ListOptions::LIST` for a flat list,
`ListOptions::TREE` for everything).

- **Rows** come from the application in display order, depth first: a `ListItem` has the
  application's stable key, a depth (a row is the parent of the deeper rows after it), a label
  and an icon or a color. Rows are `list-row.height` high (one interface unit, or
  `ListOptions::row_height`), alternate `table.row.background.color.default` and `.alt`, and
  show per level a fold arrow slot `list-row.indent` wide, then the mark, the label cut with an
  ellipsis, and the items the `ListDelegate` draws at their end.
- **Folding**: a press on the arrow folds or unfolds a row, with Shift its whole subtree. The
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

- **Items** (`MenuItem`): actions with an icon, a label, a shortcut hint, a check or radio
  mark (`MenuMark`, set with `.checked(on)` or `.radio(on)`), disabled or marked as the
  current choice (`.selected(true)`); submenus (`MenuItem::submenu(label, &items)`), eight
  levels deep at most; separators and titles.
  Columns line up across a menu: marks, icons, labels, shortcuts, submenu arrows.
- **Rows**: an item is one interface unit high, and items lie against each other. Its highlight
  is smaller: `menu.item-gap` between two highlights and `menu.item.margin.horizontal` from the
  panel's sides, while `menu.padding.vertical` counts to the highlights of the first and last
  items. The pointer selects an item over its whole row. The current choice has its own color
  under the pointer (`menu.item.background.color.selected-hover`).
- **Opening**: a `MenuState` per menu, kept by the application, opens it around the pointer
  (`open_at_pointer`, for context menus, usually on `Response::secondary_pressed`), at a point
  (`open_at`) or beside a control (`open_for(rect, placement)`). Around the pointer, the pointer
  lies over the item last chosen, or the first one that can be chosen, at
  `menu.open.pointer-share` of its width; a menu that would leave the window stays inside it, and
  the pointer then lies elsewhere. The choice is remembered for as long as the `MenuState` lives. `Ui::menu` draws it and its open submenus on the floating
  layer, kept inside the window, and returns the item chosen this frame: `MenuChoice::index`
  in its own list, `MenuChoice::path` from the root (`[2, 0]`: the first item of the submenu
  of the root's third item).
- **Placement** (`Placement`): a side of the control (`Side`: below, above, right, left) and
  an alignment along it (`PanelAlign`: start, end); the menu flips to the other side or alignment when the
  window has no room, unless `flip` is off. A menu below or above its control is at least as
  wide; where the two meet, their corners are square (`Placed::control_corners`,
  `Placed::panel_corners`). An open menu follows its control when the layout moves
  (`MenuState::follow`).
- **Submenus** open beside their item when the pointer rests on it for
  `semantic.input.submenu.delay` or at a click, cascading the way the previous level went
  (a level that flipped left keeps the next ones on the left). While the pointer heads from the item
  towards the open submenu, the items it crosses wait for `semantic.input.submenu.grace`
  before taking over.
- **Closing**: choosing closes every menu; a press outside closes them and reaches nothing
  else; the pointer going farther than `semantic.input.menu.close-distance` from the menus and
  their control closes them, once it came near them (0 turns this off). Leaving a menu closes
  nothing by itself.
- **Keyboard**, while open, wherever the pointer is: `menu.next`, `menu.previous`,
  `menu.open-submenu`, `menu.close-submenu`, `menu.choose` and `menu.close` (arrows, Enter or
  Space, Escape by default); Escape closes the innermost menu. The pointer takes the
  highlight back only when it moves, so the keyboard keeps it while a long menu scrolls under
  a still pointer.
- **Long menus** stop at `menu.max-height` (or the window), scroll with the wheel
  (`menu.wheel-step`) and follow the keyboard's highlight.
- **Menu buttons**: `Button::menu(&mut state, placement)` opens the menu beside the button,
  shows the button selected while it is open and squares the corners where they meet, also
  inside a `ButtonGroup` (buttons fused around a menu button). Show the menu with `Ui::menu`
  after the button or the group.
- **Dropdowns** (`Dropdown`): a menu button with the dropdowns' look (`dropdown.*`, darker
  than buttons) and a chevron (`dropdown.chevron.size`) pointing where the menu opens, turned
  back while it is open. `.placement(Placement::RIGHT)` makes a vertical dropdown; every
  button option (content, sizes, colors per state) applies, and a tone colors the control
  while its metrics and chevron stay the dropdowns'. `Dropdown::measure(ui, label)` gives
  the size of a dropdown showing `label`, to reserve the width of the widest choice.
- **Popups** (`Ui::popup`): a panel of custom content that opens, places itself and closes
  like a menu (but never by moving away, so that its content can be dragged). Escape closes
  a menu open inside the popup first: a component that uses an action this frame marks it
  used (`Ui::consume`), and the components asking after it no longer see it triggered.

## Tooltips

`Ui::tooltip(id, text, bounds)`, called after the widget `id`, shows `text` near the pointer
(`tooltip.offset`, flipping above near the bottom of the window) on the overlay layer once the
pointer has rested on the widget for `semantic.input.tooltip.delay`, or for
`semantic.input.tooltip.delay-short` right after another tooltip. A press hides it until the
pointer moves to another widget; tooltips never take the pointer.

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
  moves the editor: over a tab bar it goes into its tabs (a line shows where), in the middle of
  an area (40 % of its size) it joins its tabs, elsewhere it docks beside the area, on the
  nearest side, which splits in half. The preview glides from one place to the next with the
  time constant `zone-tab.drop-preview.anim-duration` (exponential smoothing), and jumps to
  another area. A lone editor has no tab bar: when the drag stays over its top for
  `zone-tab.reveal.delay`, the bar slides in, and it slides out `zone-tab.reveal.grace` after the
  drag leaves. The dragged tab follows the pointer; the area it leaves closes when empty, or the
  drop is refused if that area cannot close.
- **Gutters** show a resize cursor; dragging one moves every edge of its line, ten times finer
  with Shift and snapped to a 48th of the window with Ctrl, both as soon as they are pressed,
  never below `zone.min-size`. After `input.hover.delay`, a band with rounded ends shows the
  whole line that would move (`zone.separator.color.continuation` at its opacity), brighter
  between the two hovered areas (`zone.separator.color.default`).
- **Area menu**: a right click on a gutter offers to swap the two areas, join them, or split
  any area vertically or horizontally.
- **Corners** (`zone.corner.size`, crosshair cursor): dragged inwards, a corner splits its area
  along the axis the drag starts on (a horizontal drag makes a vertical line), where the pointer
  lets go; dragged into a neighbour, it joins that neighbour into its area, even when their
  edges only partly meet ([ADR 0016](../adr/0016-partial-joins.md)). Once the drag has
  started, a place where releasing would do nothing shows "not allowed". The previews are the
  same as the menu's.
- **Previews**: a split shows the two halves the area under the pointer would become, a gutter
  apart (`zone-overlay.split-preview`); a join shows the part of the kept area that stays
  lighter, the part of the removed area it takes darker, the parts of both that neighbours take
  darker still, and the joined area outlined (`zone-overlay.join.*`). Everything around the
  future area that goes is dark: the gutters next to it and the outsides of the rounded corners,
  on its side and on the adjacent side of what stays. An arrow cursor points the way the join
  goes (from the kept area towards the removed one), or "not allowed"
  where nothing can join. A press applies the preview; a press where nothing can happen does
  nothing.
- **Cancel**: Escape, Delete or a right click (`workspace.cancel`) end any operation and restore
  the exact previous layout; Enter (`workspace.confirm`) applies a preview.

## One frame

```rust
use koebi_ui::{Button, Icon, Input, StyleCache, Ui, UiState, WidgetId};

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
if Button::new(WidgetId::new("apply"))
    .icon(Icon::Check)
    .show(&mut ui, apply_rect)
    .clicked
{
    // ...
}
NumberField::new(WidgetId::new("radius"), &mut radius)
    .range(0.0, 16.0)
    .bar(BarMapping::Linear)
    .show(&mut ui, radius_rect);
drop(ui); // ends the frame for `state`
input.end_frame();
```

- **Ids** identify a widget across frames, so that a press or a drag survives redraws.
  Derive the ids of repeated widgets with `WidgetId::with(index)`.
- **Responses** say what happened this frame: `hovered`, `pressed`, `held`, `clicked`,
  `double_clicked`, `secondary_pressed`, `released`, `changed` and `drag_delta`.
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
pub struct CheckboxStyle {
    pub background: States<Color>, // one color per ControlState
    pub mark: Color,
    pub box_size: f32,
    pub corner_radius: f32,
    // ...
}

impl CheckboxStyle {
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            background: States {
                normal: tokens.get(component::CHECKBOX_BACKGROUND_COLOR_DEFAULT),
                hovered: tokens.get(component::CHECKBOX_BACKGROUND_COLOR_HOVER),
                // ...
            },
            mark: tokens.get(component::CHECKBOX_MARK_COLOR_DEFAULT),
            box_size: tokens.get(component::CHECKBOX_BOX_SIZE_DEFAULT),
            corner_radius: tokens.get(component::CHECKBOX_CORNER_RADIUS_DEFAULT),
            // ...
        }
    }
}
```

`tokens.get` returns the token's Rust type (`Color`, `f32`...), so a dimension token cannot
land in a color field. `Resolve` is implemented by the `DesignSystem` (the active theme with
every layer) and by any `View`: `ThemePreview::of(&design, theme)` resolves another theme,
with its overrides and the color vision setting, for a theme card.

`Styles::resolve` resolves every component's style together. `StyleCache` keeps them in step
with the design system and resolves again only after a change: a field that edits a token
every frame costs one resolution per frame, and frames without changes read no token.
`Styles::tokens` keeps a snapshot of every token's value, which resolves the tokens calls give
(`Prop::Token`) without a lookup in the design system.

## Writing a component

1. **Tokens.** List what the component needs: colors per state, sizes, radii, widths. Reuse
   the component's existing tokens; add the missing ones to its file in `koebi-tokens`
   (`src/component/<category>/<component>.rs`, created for a new component, see
   [defining tokens](README.md#defining-tokens)) under the component's name, aliasing
   semantic tokens (or holding a value when no semantic token fits). Avoid aliasing another
   component's tokens: two components that look alike today may diverge tomorrow.
   Give the component every property even when the current themes leave it at zero (a border
   width, the border colors of every state), so that a theme can use it.
2. **Style.** In `crates/koebi-ui/src/widgets/<component>.rs` (or a folder for a component
   with parts), define `<Component>Style` with one field per token and its `resolve`
   function. Use `States<T>` for values per state. Add the style to `Styles` and to
   `Styles::resolve`.
3. **Builder.** `<Component>::new(id, ...)` takes what the component cannot do without (its
   identifier, the value it edits); every other option has a method returning `Self`
   (`#[must_use]`), properties as `impl Into<Prop<K>>`. `size(ui)` measures the component,
   `show(self, ui, rect)` calls `ui.interact(id, rect)`, updates the value, sets
   `response.changed`, then draws with the style's values and the call's overrides. Read
   `ui.styles()` (it outlives the borrow of `ui`), never the design system. Shared parts live
   in `widgets/parts` (`ControlState`, `Tone`, `Content`). Keep edges on whole logical pixels
   (`widgets::snap`) and allocate nothing.
4. **Tests.**
   - Unit tests beside the code: interaction (clicks, drags, values, clamping), options,
     overrides and sizes.
   - A pixel test proving that the component's tokens reach the screen in every state
     (`crates/koebi-ui/tests/buttons.rs` shows how to put a component in each state).
   - The component at every numeric token's extremes stays inside its rectangle (the same
     file's `components_stay_inside_their_rectangles_at_every_token_extreme`).
   - A golden image of the component in its states and variants (`assert_golden!`); run
     `cargo xtask bless` and review the images.
5. **Docs.** Add the component to the table above and its new tokens to the
   [porting report](porting.md#added-after-the-port).
