# Components

`koebi-ui` draws Koebi's interface components from design tokens
([ADR 0011](../adr/0011-token-driven-components.md)), as a library whose components take
their defaults from the tokens and every property from the call
([ADR 0017](../adr/0017-component-library.md)). This page explains how a component reads its
tokens and how to write a new one.

## The components

| Component | Call | Tokens |
|---|---|---|
| Button, toggle button | `Button::new(id)` + options, then `.show(ui, rect)` | `component.button.*` |
| Button group | `ButtonGroup::horizontal()` or `::vertical()` + options, then `.show(ui, rect, &mut cells)` | `component.button.*`, `component.button.group.*` |
| Checkbox | `Checkbox::new(id, &mut checked)` + options, then `.show(ui, rect)` | `component.checkbox.*` |
| Slider | `Ui::slider(id, rect, &mut value, range)` | `component.slider.*` |
| Number field | `NumberField::new(id, &mut value)` + options, then `.show(ui, rect)` returns a `NumberFieldResponse` | `component.number-field.*` |
| Property group | `PropertyGroup::new(ui, area)`, then `.row(ui, label)`, `.vector(ui, label, ["X", "Y", "Z"])`, `.unlabeled_row()`, `.wide_row()` return the inputs' rectangles | `component.property-group.*` |
| Text field | `TextField::new(id, &mut text)` + options, then `.show(ui, rect)` returns a `TextFieldResponse` | `component.text-field.*` |
| Panel | `Ui::panel(rect, level)` returns the content area | `component.panel.*` |
| Collapsible panel | `CollapsiblePanel::new(id, &mut state, title)` + options, then `.show(ui, area, add)` returns a `CollapsiblePanelResponse` | `component.panel.*` |
| Separator | `Ui::separator(rect)` | `component.separator.*` |
| Color swatch | `Ui::swatch(id, rect, color, selected)` | `component.color-swatch.*` |
| Theme card | `Ui::theme_card(id, rect, &preview, selected)` | `component.theme-card.*` |
| Menu | `Ui::menu(id, &mut state, &items, bounds)` returns the chosen item (`MenuChoice`) | `component.menu.*`, `component.separator.*` |
| Popup | `Ui::popup(id, &mut state, size, bounds, add)` draws custom content | `component.menu.*` |
| Dropdown | `Dropdown::new(id, &mut state, &items)` + options, then `.show(ui, rect, bounds)` | `component.dropdown.*`, then the menu's |
| Tooltip | `Ui::tooltip(id, text, bounds)` after the widget | `component.tooltip.*` |
| Label | `Ui::label(text, rect, &font, color, align)`, `Ui::text_width(text, &font)` | `semantic.font.*`, `semantic.text.*` |
| Scroll area | `Ui::scroll_area(id, rect, &mut state, content_size, axes, add)` | `component.scrollbar.*` |
| Scrollbar | `Scrollbar::new(id, &mut state, axis, &mut offset, visible, content)`, then `.show(ui, area)` | `component.scrollbar.*` |
| List and tree | `Ui::list(id, rect, &mut state, &items, &options, &mut delegate)` returns a `ListResponse` | `component.list-row.*`, `component.outliner.*`, `component.table.row.*`, `component.drop-target.*` |

Line icons (`Icon::Check`, `Icon::Plus`, chevrons...) use `component.icon.*`. Labels use the
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
  current choice), selected and hovered, disabled. `States<T>` holds a value per state.
- **Overrides**: every property takes a `Prop`, a token or a value:
  `.corner_radius(0.0)`, `.padding(semantic::SPACING_EDITOR_INSET)`,
  `.background(ControlState::Hovered, Color::from_hex(0xE0_30_30))`; `.corner_radii(...)`
  takes one radius per corner (`CornerRadii`), and the last of the two calls applies.
- **Content** (`Content`): a leading icon, a label or an icon, a status dot and a trailing
  icon, placed by `align`; a label too long is cut with an ellipsis, and content that cannot
  fit is clipped to the button.
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
  (pressed, hovered, selected, rest, disabled); only the group's outer corners are rounded. The
  `GroupResponse` says which cell fired, was double-clicked, pressed with the secondary
  button or hovered.
- **Checkboxes**: the box, a label beside it and an optional description below
  (`checkbox.description.color`), all of it clickable; `.mixed(true)` shows the mixed mark of a
  partly checked set, which a click checks. The box's colors (`background`,
  `checked_background`, `border_color`, `checked_border_color`), size, radius and border, the
  mark and the label colors take overrides like buttons; the first line grows to hold a larger
  box or font.

## Number fields

Blender's number button. `NumberField::new(id, &mut value)` edits any `Numeric`: `f32`,
`f64`, or integers (no decimals, whole steps). Options: `.range(min, max)` (or `.min`,
`.max`), `.step(step)` (0.1 for floats, 1 for integers), `.decimals(count)` (2 for floats),
`.unit(&unit)` (from `koebi_core::expr`, shown after the value and used for typed
expressions), `.label(text)`, `.bar(mapping)`, `.vertical()`, `.step_buttons(false)`,
`.enabled(false)`, and the metrics and colors per state as tokens or values.

- **Showing**: the value rounded to its decimals, its unit in `number-field.unit.color`,
  centred, or after its label at the start. A bounded field with `.bar(BarMapping::Linear)`,
  `Logarithmic` or `Custom { to_fraction, from_fraction }` fills a bar to where its value
  lies. A vertical field fills from the bottom and leaves out a value too wide for it.
- **Step buttons** appear on hover at both ends: a step; Shift a finer one
  (`number-field.drag.fine-factor`); Ctrl to the next round step (a multiple of the step);
  Alt by the value's last decimal (1.25 steps by 0.01).
- **Dragging**: a press locks the pointer at once (`Ui::lock_pointer`), and the field takes
  the keyboard (`NUMBER` context). Past the drag threshold, the travel the locked pointer
  reports (`Input::motion`) changes the value: a step per `number-field.drag.step-distance`
  for an unbounded field, the whole range along the bar for a bounded one. Left Shift makes
  the whole drag finer from its start, right Shift finer from the current value; Ctrl rounds
  to steps; Alt keeps the start's last decimal. Escape or Delete (`number.cancel`) or a right
  click cancel at any time and restore the value (a right click that cancels asks for no
  context menu); Enter (`number.confirm`) or the release keep it. The pointer comes back where
  the press was, or at the end of the bar of a bounded field, also when the application draws
  the frame again with settled input. Travel past a bound does not count: coming back moves
  the value at once. Integers step by whole numbers, at least 1, Shift included; Ctrl with
  Shift rounds to the finer step.
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
  edits at a time. While it does, it takes the whole keyboard (`Ui::grab_keyboard` with the
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
- **Look**: a box of the field's colors per state (selected is editing, with the
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

- **Header**: `panel.header.height` high, a chevron (`panel.chevron.*`) pointing right while
  closed and turning down as the panel opens, an optional `.icon(icon)`, the title, and room
  for the caller's buttons (`.trailing(width)`, returned as `CollapsiblePanelResponse::trailing`;
  draw them after the panel so that they take the pointer). Under the pointer, the header
  lightens by `panel.header.hover.lightness`.
- **Content**: `show(ui, area, add)` calls `add(ui, rect)` with the full width of the panel
  inside the frame, from the top of its content down to the bottom of `area` (the room
  there is); `add` draws and returns the height it used (a nested panel's `height`). Rows
  inset themselves by `panel.padding`; nested panels fill the width. The content draws while
  the panel is open or opening, clipped to the share shown, and the frame is drawn over it. A
  panel never measured shows its content from the frame after it first drew it (it asks for
  that frame at once); nested levels follow one frame apart, while they settle and while they
  fold.
- **Levels**: `.level(PanelLevel::Second)` and `Third` for panels inside a panel's content.
  A first-level panel is framed and rounded, its header in `panel.header.background.color`;
  a nested panel's header takes its parent's content color and its content the next level's
  (`panel.body.background.color.level-*`).
- **Folding**: a press on the header opens or closes it; the content is revealed along
  `panel.expand.duration` and `panel.expand.curve` (or `panel.collapse.*`), and the panel asks
  for frames while it moves. The response's `height` is what to advance by, then
  `panel.gap.size` before the next panel.

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

The scrollbar is a component of its own, for what scrolls its own way (a canvas, a timeline):
`Scrollbar::new(id, &mut state, ScrollAxis::Vertical, &mut offset, visible, content)` shows
in the margin at the end of the area passed to `show` (right for vertical, bottom for
horizontal) and changes `offset`, from 0 to `content - visible`. Its `ScrollbarState` keeps
the grab's growth (`growth()`, eased along `scrollbar.overlay.grow.duration` and `.curve`)
and its drag (`is_dragging()`). `.near(false)` keeps it at rest while the pointer is not over
what it scrolls; `ScrollbarResponse` says whether it showed and changed the offset. Scroll
areas use one per axis.

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

- **Items** (`MenuItem`): actions with an icon, a label, a shortcut hint, a check or radio
  mark (`MenuMark`, set with `.checked(on)` or `.radio(on)`), disabled or marked as the
  current choice (`.selected(true)`); submenus (`MenuItem::submenu(label, &items)`), eight
  levels deep at most; separators and titles.
  Columns line up across a menu: marks, icons, labels, shortcuts, submenu arrows.
- **Opening**: a `MenuState` per menu, kept by the application, opens it at a point
  (`open_at`, a context menu, usually on `Response::secondary_pressed`) or beside a control
  (`open_for(rect, placement)`). `Ui::menu` draws it and its open submenus on the floating
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
  lets go; dragged into a neighbour, it joins that neighbour into its area, even when their
  edges only partly meet ([ADR 0016](../adr/0016-partial-joins.md)). Once the drag has
  started, a place where releasing would do nothing shows "not allowed". The previews are the
  same as the menu's.
- **Previews**: a split shows the two halves the area under the pointer would become, a gutter
  apart (`zone-overlay.split-preview`); a join shows the part of the kept area that stays
  lighter, the part of the removed area it takes darker, the parts of both that neighbours take
  darker still, and the joined area outlined (`zone-overlay.join.*`), with an arrow cursor
  pointing the way the join goes (from the kept area towards the removed one), or "not allowed"
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
ui.slider(WidgetId::new("radius"), slider_rect, &mut radius, 0.0..=16.0);
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
with the design system and resolves again only after a change: a slider that edits a token
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
