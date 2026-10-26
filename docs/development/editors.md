# Editors

What each area of the workspace shows is an editor. The workspace view (`koebi-ui`) draws the
areas, their top bars and tab bars; the application draws the editors' content through its
`EditorHost` ([`apps/koebi/src/editors/`](../../apps/koebi/src/editors/)).

## Kinds and their state

The type dropdown at the start of a top bar switches an editor's kind: Gallery, Viewport,
Outliner, Info or Timeline. An editor keeps a state for every kind it has shown (the
outliner's folds, selection and scroll position, the Info editor's...), so switching away and
back finds it as it was. A split copies the state of every kind into the new editor; a join
forgets the states of the editors it removes.

What editors show is shared: every outliner shows the same document, every Info editor the
same messages. Only the view (folds, selection, scrolling) belongs to each editor.

## Templates

Templates exercise the editor framework until Koebi has real documents.

| Kind | Shows | Shortcuts (keymap section) |
|---|---|---|
| Outliner | A template document (groups holding colored layers) in the list and tree view: folding, selection, reordering by drag and drop, rows going into groups only | Outliner: delete (X, Delete), group (Ctrl+G), unfold all (keypad +), fold all (keypad -) |
| Info | The application's log messages from `info` up, marked by level | Info: delete selected (X, Delete), clear (Shift+Delete) |
| Viewport, Timeline | Placeholders | |

Each template's shortcuts apply where the pointer is over one of its editors: its content is a
zone of the template's context (`outliner`, `info`), inside the list's own context, which
gives the list's keys (arrows, Ctrl+A) to every list.

## Messages

`messages::install` puts a log sink in front of the platform's (stderr, logcat): it keeps
records from `info` up, from any thread, and the interface collects them once per frame. The
Info editor shows the 500 latest.
