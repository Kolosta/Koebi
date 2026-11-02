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
- **Windows are independent**: no window owns another, so that minimizing the main one leaves the
  others, and one process gives them one icon on the taskbar, in the dock or their equivalents.
- **The main window** is the first one: closing it quits, and only it is saved and shows the frame
  times. Closing another one closes it alone.
- **Redraws**: each window redraws when it asks to, and when the design system has changed since
  its last frame, wherever the change came from.
- **Editors between windows**: an editor dropped outside its window goes to the window under the
  pointer, as a tab of the area there, or to a new floating window with its state. A floating window
  left empty closes. `Window > New Window` opens one.
- **Mobile platforms** have one window: the settings show inside it.

Floating windows are not restored at the next start yet, unlike the main window's workspace.

## The settings window

`Edit > Settings` opens it, or brings it to the front. A column of pages in groups lies beside the
page shown. Options are expandable panels: the header edits the value for the whole application;
the panel opens on the value for the application (`Scope::Global`) and for the current theme
(`Scope::ActiveTheme`), each with a place for the icon that brings it back, then on how the token
gets its value (`View::chain`): the tokens it refers to, down to the one holding the value, and,
grayed, what that one would refer to without its override.
