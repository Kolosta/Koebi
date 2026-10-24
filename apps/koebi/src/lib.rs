//! The Koebi application: the composition root of every layer.
//!
//! - `koebi-platform` provides the window, events and lifecycle (SDL3 main callbacks);
//! - `koebi-gpu` provides the Vulkan device and frame presentation;
//! - `koebi-render` draws the 2D scene.
//!
//! Today the scene is a single themed rounded rectangle. Future UI crates (widgets, layout,
//! design system) plug in here without touching the lower layers.

mod app;
mod options;
mod scene;
mod theme;

pub use koebi_platform::exit_code;
pub use options::{Options, ParseError};

/// Runs Koebi and returns the process exit code (see [`exit_code`]).
///
/// Entry point for desktop and iOS (`src/main.rs`).
pub fn run() -> i32 {
    koebi_platform::run::<app::KoebiApp>()
}

/// Android entry point: SDL's Java activity loads `libkoebi.so` and calls this function.
///
/// # Safety
///
/// Must only be called by SDL, with `argc` valid C strings in `argv`.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SDL_main(
    argc: std::ffi::c_int,
    argv: *mut *mut std::ffi::c_char,
) -> std::ffi::c_int {
    // SAFETY: SDL's Java glue passes valid arguments (contract of this function).
    unsafe { koebi_platform::run_sdl_main::<app::KoebiApp>(argc, argv) }
}
