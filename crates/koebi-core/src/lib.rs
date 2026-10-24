//! Foundation layer of Koebi.
//!
//! `koebi-core` has no dependencies and no platform code. Every other crate builds on it:
//!
//! - [`math`]: small `#[repr(C)]` geometry types and the unit conventions used everywhere
//!   (logical pixels for layout, physical pixels for framebuffers).
//! - [`color`]: color science for the design system: gamma-encoded sRGB (what designers
//!   pick), linear light, perceptual OKLab/OKLCH, gamut mapping and WCAG contrast.
//! - [`log`]: a minimal logging facade with a pluggable sink (stderr by default, logcat on
//!   Android).
//!
//! Because nothing here touches the GPU or the windowing system, the whole crate is
//! unit-testable on every target, including the Android emulator and the iOS simulator.

pub mod color;
pub mod log;
pub mod math;

pub use color::{Color, LinearRgba, Oklab, Oklch, ParseColorError};
pub use math::{Extent, Rect, Vec2};
