# Windows

How Koebi draws its own title bar, and how several windows share one application.

## The bar

Every window has an application bar: menus, title and window buttons. The application draws it and
the system keeps its behaviours: see [ADR 0019](../adr/0019-window-chrome.md).

| Behaviour | Windows | Linux X11 | Linux Wayland | macOS |
|---|---|---|---|---|
| Window buttons | Koebi's, right | Koebi's, right | Koebi's, right | the system's traffic lights, left |
| Move from the bar | system | window manager | compositor | system |
| Snap to screen edges, drag to the top | system | window manager | compositor | system |
| Drag from maximized restores under the pointer | system | window manager | compositor | none: no such state |
| Snap layouts over the maximize button | system, on Windows 11 | none | none | none |
| Double click on the bar | system | counted by Koebi | **not available** | system setting, read by Koebi |
| Right click on the bar | system menu | window manager's menu | compositor's menu | none |
| Resize from edges and corners | system | window manager | compositor | system |
| F11 fullscreen, no moving or resizing | Koebi and system | Koebi and manager | Koebi and compositor | Koebi and system (its own space) |

The hit test answers from `WindowRegions`, updated after each frame. It lies in
`crates/koebi-platform/src/chrome.rs`, a pure function tested on every platform. On Windows,
`apps/koebi/tests/window_chrome.rs` asks the real window manager what each part of the window is,
without simulating any input.

## Several windows

| Shared, in `Interface` and `Gpu` | Per window, in `AppWindow` |
|---|---|
| Vulkan instance and device | surface, presenter and renderer |
| design system and its saved state | `UiState`, `Input`, `Scene` |
| keymap | application bar and its menus |
| what editors show: document, gallery, messages | workspace and the state of its editors |

- **One renderer per window**: its streaming buffers follow the frames in flight of one presenter.
- **Windows are independent for the system**: no window owns another, so that minimizing a main
  window leaves the others, and one process gives them one icon on the taskbar, in the dock or
  their equivalents.
- **Main windows and floating windows** (`Role`), as in Blender: `Window > New Main Window` opens
  another main window, on a copy of the workspace it is chosen from; `Window > New Window` opens a
  floating window, which belongs to the main window it is chosen from (or that of the floating
  window it is chosen from). Closing a main window closes its floating windows with it; closing the
  last main window quits. The settings window belongs to no main window: it stays while one does.
  The workspace saved is the one of the oldest main window open, and main windows show the frame
  times.
- **Another instance**: `Window > New Instance` starts another process, as VS Code's new window
  does, with `--untitled`: it shares the data directory, so the design state and the shortcuts,
  but its workspace is its own, neither restored nor saved. Closing one instance leaves the other.
- **Redraws**: each window redraws when it asks to, and when the design system has changed since
  its last frame, wherever the change came from.
- **Editors between windows**: an editor dropped outside its window goes to the window under the
  pointer, as a tab of the area there, or to a new floating window with its state. A floating window
  left empty closes; it belongs to the main window of the window the editor came from.
- **Mobile platforms** have one window: the settings show inside it, and the Window menu opens
  nothing.

Floating windows are not restored at the next start yet, unlike the main window's workspace.

## The settings window

`Edit > Settings` opens it, or brings it to the front. A column of pages in groups lies beside the
page shown. Options are expandable panels: the header edits the value for the whole application;
the panel opens on the value for the application (`Scope::Global`) and for the current theme
(`Scope::ActiveTheme`), each with a place for the icon that brings it back, then on how the token
gets its value (`View::chain`): the tokens it refers to, down to the one holding the value, and,
grayed, what that one would refer to without its override.
