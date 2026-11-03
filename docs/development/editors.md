# Editors

What each area of the workspace shows is an editor. The workspace view (`koebi-ui`) draws the
areas, their top bars and tab bars; the application draws the editors' content through its
`EditorHost` ([`apps/koebi/src/editors/`](../../apps/koebi/src/editors/)).

## Kinds and their state

The type dropdown at the start of a top bar switches an editor's kind: Gallery, Viewport,
Outliner, Properties, Info or Timeline. An editor keeps a state for every kind it has shown (the
viewport's pan, zoom and rulers, the outliner's folds, selection and scroll position, the
timeline's scrolling and zoom...), so switching away and back finds it as it was. A split copies the state of every kind into the new editor; a join
forgets the states of the editors it removes.

What editors show is shared: every outliner shows the same document, every Info editor the
same messages, every timeline the same current frame, every Properties editor the same object. Only the view (folds, selection,
scrolling, zoom) belongs to each editor.

## Templates

Templates exercise the editor framework until Koebi has real documents.

| Kind | Shows | Shortcuts (keymap section) |
|---|---|---|
| Viewport | A canvas with a page, a grid that keeps its spacing on screen, rulers on any side (toggles in the top bar); the middle button pans, the wheel zooms around the pointer | Viewport: frame all (Home), zoom in and out (keypad + and -), actual size (keypad 1), toggle rulers (Ctrl+R) |
| Outliner | A template document (groups holding colored layers) in the list and tree view: folding, selection, reordering by drag and drop, rows going into groups only | Outliner: delete (X, Delete), group (Ctrl+G), unfold all (keypad +), fold all (keypad -) |
| Info | The application's log messages from `info` up, marked by level | Info: delete selected (X, Delete), clear (Shift+Delete) |
| Properties | The components in real use, editing the template's object: property groups in collapsible panels (Object, Transform with a nested Delta transform, Visibility, Shading), text fields, vectors of number fields in millimetres and degrees, checkboxes, an opacity on a bar, a dropdown, an integer field and a button group (reset the transform, apply the delta); the panels scroll when they do not fit | None of its own: the components' keys (text, number fields, menus) |
| Timeline | Tracks of keyframes along the scene's frames (dimmed outside its range), scrolling both ways; a press or a drag on the frame ruler sets the current frame | Timeline: next and previous frame (Right, Left, repeating), first and last frame (Shift+Left, Shift+Right), zoom in and out (keypad + and -), frame all (Home) |

Each template's shortcuts apply where the pointer is over one of its editors: its content is a
zone of the template's context (`viewport`, `outliner`, `info`, `timeline`). Lists add their
own context inside it, which gives the list's keys (arrows, Ctrl+A) to every list; the same
key may mean something else in another template (keypad + zooms a viewport, unfolds an
outliner). The ctrl shortcuts use Command on Apple platforms.

## Saved workspace and shortcuts

The areas, the editors of each area and each editor's kind are saved in `workspace.txt`, and
the shortcuts that differ from their defaults in `shortcuts.txt`, next to `design.kds` in the
data directory ([Gallery](../design-system/gallery.md#saved-state)). The workspace is saved a
second after the last change (one write per pause while a gutter drags), the shortcuts as soon
as the shortcut editor changes them; both files are saved when the application goes to the
background or quits, only when they changed. A shortcut turned off keeps a `!` before it
(`app.quit = Ctrl+Q, !Ctrl+W`), a key held as a modifier comes after the modifiers
(`Ctrl+Space+G`), and a double click with a time of its own carries it
(`LeftMouse double 300ms`). Each
write goes to a temporary file, flushed, then renamed over the file, whose previous version
stays as `.bak`; a damaged or missing file falls back to its backup.

Both files are text keyed by stable names: kinds (`gallery`, `viewport`, `outliner`,
`properties`, `info`, `timeline`) and action identifiers (`viewport.view-all = F`). Lines a version of Koebi does
not know are skipped with a warning, so files survive code changes; a layout that does not
tile the window is refused and the default workspace opens. Editors' views (scrolling, folds,
zoom) are not saved yet.

## Shortcut rules

The keymap (`koebi-input`) and the shortcut editor apply these rules, the same for the
defaults and for what users make; nothing else decides whether two shortcuts get along.

- **One zone, one input**: two actions of one context never share a shortcut that is on (same
  key or button, same event, modifiers some keys satisfy at once). The keymap refuses it; the
  editor keeps the newcomer off, in red, until one of them changes.
- **Unless they never apply at once**: an action may declare the condition it applies under
  (`ActionDef::when`, a `Condition` on a subject): conditions on one subject exclude each
  other. Two actions whose conditions exclude each other may share a shortcut in one context:
  it triggers both, and each acts only when its condition holds (Fold all when a group is
  unfolded, Unfold all when every group is folded: one key folds, then unfolds). The editor
  says so, as information, not as a conflict. Similar shortcuts of such actions stay
  conflicts: the first may change what the second reads.
- **Zones inside zones** (soft, orange): a shortcut of a zone lying inside another's wins
  there; the editor tells both actions.
- **Similar shortcuts** (soft, orange): the same key or button on another event (press and
  release, click and double click, press and drag) in one zone or zones inside each other: one
  movement triggers both, or one instead of the other.
- **Zones apart never conflict**: two editors, a menu and a text field being edited.
- **Defaults**: they follow the same rules, checked when the keymap is built (a default
  taking another's input is a programming error, as a default repeating for an action that
  does not repeat). Soft conflicts between two defaults are not shown: the application
  designed them together.
- **Repeats**: only actions that repeat (`ActionDef::repeats`: moving through a list,
  zooming) take shortcuts repeating while their key is held; the keymap drops the repeat of
  the others' (folding, quitting), and the editor offers it to those actions only, on a key
  press.
- **Events by device**: a key is pressed or released; a mouse button is also clicked, double
  clicked or dragged; the wheel, a pinch and a pointer move only happen.
- **Double clicks**: a second press of a button near the first (within the drag threshold),
  within the double-click time: the application's (`semantic.input.double-click.time`) or
  the shortcut's own. A third press starts again.
- **Shortcuts being added** have no key yet: they trigger nothing and conflict with nothing
  until they get one.

## Messages

`messages::install` puts a log sink in front of the platform's (stderr, logcat): it keeps
records from `info` up, from any thread, and the interface collects them once per frame. The
Info editor shows the 500 latest.
