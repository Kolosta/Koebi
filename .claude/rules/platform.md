---
paths:
  - "crates/koebi-platform/**"
  - "apps/koebi/src/**"
  - "platform/**"
---

# Platform layer and application lifecycle

- SDL types (`SDL_*`, `sdl3_sys::*`) never leak out of `koebi-platform`'s public API; translate
  to Koebi types (`Event`, `WindowId`, `Key`...). Event translation stays a pure function with
  unit tests built from zeroed `SDL_Event`s.
- All SDL calls happen on the main thread, inside the `App` callbacks (`AppContext` is the
  capability that proves it). Never block in `iterate`/`event`.
- Mobile lifecycle: on `WillEnterBackground` drop the presenter (swapchain + surface) before the
  OS destroys the native window; recreate it on `DidEnterForeground`. Handle
  `koebi_gpu::Error::SurfaceLost` by recreating the surface.
- UI apps render on demand: `IterationRate::WaitEvent`, redraw only when `needs_redraw` is set
  (input, resize, expose, animation). `--frames N` switches to continuous rendering for tests.
- Exit codes come from `koebi_platform::exit_code` (CI and e2e tests depend on them).
- Android: `KoebiActivity` loads `SDL3` then `koebi`; SDL's Java glue is copied from the locked
  `sdl3-src` by `cargo xtask android`, never edited or vendored. Keep `minSdk` 26.
- iOS/macOS: bundles are assembled by `cargo xtask ios|package` from the `Info.plist`
  templates; MoltenVK is static on iOS and bundled in `Contents/Frameworks` on macOS.
