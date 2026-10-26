//! The Koebi application: the composition root of every layer.
//!
//! - `koebi-platform` provides the window, events and lifecycle (SDL3 main callbacks);
//! - `koebi-gpu` provides the Vulkan device and frame presentation;
//! - `koebi-render` draws the 2D scene;
//! - `koebi-design` and `koebi-tokens` hold the design system, saved in the user's preferences;
//! - `koebi-ui` draws the components.
//!
//! Until Koebi has editors, the window shows the design system gallery: theme cards, every
//! component, a token editor, the color vision modes and a grid of components.

mod actions;
mod app;
mod cursors;
mod design_options;
mod editors;
mod frame_times;
mod gallery;
mod messages;
mod options;
mod overlay;

pub use koebi_platform::exit_code;
pub use options::{Options, ParseError};

/// Runs Koebi and returns the process exit code (see [`exit_code`]).
///
/// Entry point for desktop and iOS (`src/main.rs`).
pub fn run() -> i32 {
    messages::install();
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
    messages::install();
    // SAFETY: SDL's Java glue passes valid arguments (contract of this function).
    unsafe { koebi_platform::run_sdl_main::<app::KoebiApp>(argc, argv) }
}
