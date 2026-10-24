//! Desktop and iOS entry point. Android uses `SDL_main` from the library instead.

// Release builds on Windows are GUI applications without a console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    std::process::exit(koebi::run());
}
