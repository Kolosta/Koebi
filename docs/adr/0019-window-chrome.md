# ADR 0019: Custom window chrome through the system's hit test

## Status

Accepted

## Context

Koebi draws its whole interface, and its windows should show it from their very top: a bar with
the menus, the title and the window buttons, in the interface's look. A window without a system
title bar loses what the bar did: moving, snapping to screen edges, maximizing by a drag to the
top, restoring under the pointer by a drag from maximized, the system menu, the animations.

The C++ application this one follows redid part of that by hand: it cancelled the system's
maximize to compute its own, and moved the window itself when dragged from maximized, because the
SDL version it used let a maximized borderless window cover the taskbar.

## Decision

The application draws the bar and tells the system what each point of the window is. The system
does the rest.

- `WindowChrome::Custom` makes a borderless window (a titled one on macOS, see below), with an SDL
  hit test answering from `WindowRegions`: the bar moves the window, edges and corners resize it,
  and what lies on the bar is the application's. A pure function (`chrome::HitRegions::hit`)
  answers on every platform, unit tested, and checked against the real window manager on Windows
  through `WM_NCHITTEST`.
- What lies on the bar is not declared by hand: `UiState::covering` reads it from the frame's
  regions, so that a new item on the bar, or a menu floating over it, takes the pointer without
  any further code.
- **Windows**: `HTCAPTION` gives every behaviour of a title bar for free, the double click, the
  drag from maximized and the animations included. SDL 3.4 keeps a maximized borderless window in
  the work area, so that nothing needs cancelling. `DwmExtendFrameIntoClientArea` brings the
  shadow back.
- **Linux**: the same through the window manager (`_NET_WM_MOVERESIZE` on X11, `xdg_toplevel` on
  Wayland). A double click on the bar reaches nobody: SDL reports the press on X11, and the
  platform layer counts two of them; **Wayland reports nothing, so that the double click does not
  work there** until SDL offers a way.
- **macOS**: the traffic lights stay the system's, in the system's look of each version: the
  window keeps its title bar, transparent and without a title, with the content under it
  (`NSWindowStyleMaskFullSizeContentView`). `ChromeInsets` tells the bar the room they take.
- Window buttons fire on the release, as the system's do. Their glyphs are rectangles on whole
  physical pixels (`UiState::set_scale_factor`), and the cross two segments ending on the corners
  of its box.

## Consequences

- Behaviours follow the system on each machine, including those nobody listed, and Koebi has no
  window geometry code to maintain.
- The macOS path cannot be exercised on Windows: it compiles in CI, and its behaviours are
  unverified until someone runs it on a Mac. It calls AppKit through the Objective-C runtime, which
  adds no dependency.
- The maximize button of Windows 11 shows its snap layouts only when the hit test answers
  `HTMAXBUTTON`, which SDL cannot answer: `win32.rs` subclasses the window procedure to answer it.
  The system then reports the pointer over the button as non-client messages, so that the
  application shows the button's state from what `Window::maximize_button` says, and the release
  sends the system's own command. Windows 10 goes the same way, without the layouts: it is where
  this path can be tried on the maintainer's machine; the layouts themselves are unverified.
