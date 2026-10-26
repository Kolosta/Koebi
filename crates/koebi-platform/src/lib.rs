//! Koebi platform layer: the only crate that talks to SDL3.
//!
//! SDL provides what the operating system must provide and nothing more: application
//! lifecycle, windows, input events and the Vulkan surface. Everything visible is drawn by
//! `koebi-render` through `koebi-gpu`; SDL's own renderer and GPU API are compiled out.
//!
//! # Application model
//!
//! Applications implement [`App`] and call [`run`] (desktop, iOS) or [`run_sdl_main`]
//! (Android's `SDL_main`). This uses SDL3's *main callbacks*: SDL owns the loop and calls
//! `init`, `event`, `iterate` and `quit`. It is the model SDL recommends for mobile, where
//! the OS drives the frame loop, and it works identically on desktop. Idle applications can
//! sleep until the next event with [`IterationRate::WaitEvent`], which keeps a UI at zero CPU
//! usage when nothing changes.
//!
//! SDL types never appear in this crate's public API, so callers stay independent of SDL.
//! Vulkan types (`ash::vk`) do, because surfaces are created here.

mod app;
mod cursor;
mod error;
mod event;
mod vulkan;
mod window;

pub use app::{App, AppContext, AppControl, AppError, IterationRate, exit_code, run, run_sdl_main};
pub use cursor::{Cursor, CursorId, CursorImage, SystemCursor};
pub use error::PlatformError;
pub use event::{Event, GesturePhase, TextChunk, WindowEvent};
pub use koebi_core::input::{Key, Modifiers, MouseButton};
pub use vulkan::VulkanSupport;
pub use window::{Window, WindowDesc, WindowId};
