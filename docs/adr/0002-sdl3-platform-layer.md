# 0002. SDL3 platform layer with main callbacks

- Status: Accepted
- Date: 2026-10-24
- Deciders: @Kolosta

## Context

Koebi needs windows, input, high-DPI information, application lifecycle events and a Vulkan
surface on five platforms. Writing that natively means Win32, X11, Wayland, Cocoa, UIKit and
the Android activity model: a large surface to maintain. Mobile operating systems also own the
event loop: on iOS and Android the application cannot run its own `loop {}`.

## Decision

`koebi-platform` wraps SDL3 through the raw `sdl3-sys` bindings, and exposes a small safe API.

- SDL3 is built from source by `sdl3-sys` (CMake) and linked statically, except on Android
  where `SDLActivity` loads `libSDL3.so`.
- The application implements the `App` trait (`init`, `event`, `iterate`, `quit`), driven by
  SDL's *main callbacks* (`SDL_EnterAppMainCallbacks`). The same code runs on desktop and
  mobile.
- The default iteration rate is "wait for events": an idle window uses no CPU.
- Only the Vulkan library loading and `SDL_Vulkan_CreateSurface` cross into the GPU layer, as
  raw handles; SDL never sees the renderer.
- SDL subsystems Koebi does not use are disabled at build time (camera, GPU, 2D renderer).

## Consequences

- One code path for all platforms, including Android and iOS lifecycles.
- SDL brings mature input, IME, gamepad and display handling for later features.
- Building needs CMake and a C compiler; the SDL Java glue for Android must match the native
  version exactly, so `cargo xtask android` copies it from the locked `sdl3-src` crate.
- Inversion of control: application state lives in the `App` implementation, not on the stack
  of `main`.

## Alternatives considered

- **winit**: pure Rust, but many dependencies and a weaker mobile lifecycle story; still needs
  a separate audio and gamepad solution later.
- **The `sdl3` safe wrapper crate**: another dependency whose abstractions we would not control;
  the wrapper we need is small.
- **Native code per platform**: maximal control, but months of work before the first feature.
- **GLFW**: no mobile support.
