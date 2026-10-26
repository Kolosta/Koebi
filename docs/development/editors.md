# Editors

What each area of the workspace shows is an editor. The workspace view (`koebi-ui`) draws the
areas, their top bars and tab bars; the application draws the editors' content through its
`EditorHost` ([`apps/koebi/src/editors/`](../../apps/koebi/src/editors/)).

## Kinds and their state

The type dropdown at the start of a top bar switches an editor's kind: Gallery, Viewport,
Outliner, Info or Timeline. An editor keeps a state for every kind it has shown (the
viewport's pan, zoom and rulers, the outliner's folds, selection and scroll position, the
timeline's scrolling and zoom...), so switching away and back finds it as it was. A split copies the state of every kind into the new editor; a join
forgets the states of the editors it removes.

What editors show is shared: every outliner shows the same document, every Info editor the
same messages, every timeline the same current frame. Only the view (folds, selection,
scrolling, zoom) belongs to each editor.

## Templates

Templates exercise the editor framework until Koebi has real documents.

| Kind | Shows | Shortcuts (keymap section) |
|---|---|---|
| Viewport | A canvas with a page, a grid that keeps its spacing on screen, rulers on any side (toggles in the top bar); the middle button pans, the wheel zooms around the pointer | Viewport: frame all (Home), zoom in and out (keypad + and -), actual size (keypad 1), toggle rulers (Ctrl+R) |
| Outliner | A template document (groups holding colored layers) in the list and tree view: folding, selection, reordering by drag and drop, rows going into groups only | Outliner: delete (X, Delete), group (Ctrl+G), unfold all (keypad +), fold all (keypad -) |
| Info | The application's log messages from `info` up, marked by level | Info: delete selected (X, Delete), clear (Shift+Delete) |
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
second after the last change (one write per pause while a gutter drags); both files are
saved when the application goes to the background or quits, only when they changed. Each
write goes to a temporary file, flushed, then renamed over the file, whose previous version
stays as `.bak`; a damaged or missing file falls back to its backup.

Both files are text keyed by stable names: kinds (`gallery`, `viewport`, `outliner`, `info`,
`timeline`) and action identifiers (`viewport.view-all = F`). Lines a version of Koebi does
not know are skipped with a warning, so files survive code changes; a layout that does not
tile the window is refused and the default workspace opens. Editors' views (scrolling, folds,
zoom) are not saved yet.

## Messages

`messages::install` puts a log sink in front of the platform's (stderr, logcat): it keeps
records from `info` up, from any thread, and the interface collects them once per frame. The
Info editor shows the 500 latest.
