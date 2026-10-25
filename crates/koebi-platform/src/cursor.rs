//! Mouse cursors: the shapes every platform provides, and custom ones made of pixels.
//!
//! SDL cursor objects are created the first time they are shown, cached, and destroyed with
//! the [`crate::AppContext`]. Showing the cursor already shown costs nothing, so applications
//! can set the cursor they want every frame.

use std::ptr::NonNull;

use sdl3_sys::mouse::{
    SDL_CreateColorCursor, SDL_CreateSystemCursor, SDL_Cursor, SDL_DestroyCursor,
    SDL_SYSTEM_CURSOR_CROSSHAIR, SDL_SYSTEM_CURSOR_DEFAULT, SDL_SYSTEM_CURSOR_EW_RESIZE,
    SDL_SYSTEM_CURSOR_MOVE, SDL_SYSTEM_CURSOR_NESW_RESIZE, SDL_SYSTEM_CURSOR_NOT_ALLOWED,
    SDL_SYSTEM_CURSOR_NS_RESIZE, SDL_SYSTEM_CURSOR_NWSE_RESIZE, SDL_SYSTEM_CURSOR_POINTER,
    SDL_SYSTEM_CURSOR_PROGRESS, SDL_SYSTEM_CURSOR_TEXT, SDL_SYSTEM_CURSOR_WAIT, SDL_SetCursor,
    SDL_SystemCursor,
};
use sdl3_sys::pixels::SDL_PIXELFORMAT_RGBA32;
use sdl3_sys::surface::{SDL_CreateSurfaceFrom, SDL_DestroySurface};

use crate::error::PlatformError;

/// A cursor every platform provides.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SystemCursor {
    /// The default arrow.
    #[default]
    Default,
    /// Text insertion.
    Text,
    /// Busy.
    Wait,
    /// Precise selection.
    Crosshair,
    /// Busy, but still interactive.
    Progress,
    /// Resizing along the top-left to bottom-right diagonal.
    ResizeNwse,
    /// Resizing along the top-right to bottom-left diagonal.
    ResizeNesw,
    /// Resizing horizontally.
    ResizeHorizontal,
    /// Resizing vertically.
    ResizeVertical,
    /// Moving in any direction.
    Move,
    /// The action under the pointer is not possible.
    NotAllowed,
    /// A link or a clickable element.
    Pointer,
}

impl SystemCursor {
    /// Every system cursor.
    pub const ALL: [Self; 12] = [
        Self::Default,
        Self::Text,
        Self::Wait,
        Self::Crosshair,
        Self::Progress,
        Self::ResizeNwse,
        Self::ResizeNesw,
        Self::ResizeHorizontal,
        Self::ResizeVertical,
        Self::Move,
        Self::NotAllowed,
        Self::Pointer,
    ];

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|cursor| *cursor == self)
            .unwrap_or(0)
    }

    fn to_sdl(self) -> SDL_SystemCursor {
        match self {
            Self::Default => SDL_SYSTEM_CURSOR_DEFAULT,
            Self::Text => SDL_SYSTEM_CURSOR_TEXT,
            Self::Wait => SDL_SYSTEM_CURSOR_WAIT,
            Self::Crosshair => SDL_SYSTEM_CURSOR_CROSSHAIR,
            Self::Progress => SDL_SYSTEM_CURSOR_PROGRESS,
            Self::ResizeNwse => SDL_SYSTEM_CURSOR_NWSE_RESIZE,
            Self::ResizeNesw => SDL_SYSTEM_CURSOR_NESW_RESIZE,
            Self::ResizeHorizontal => SDL_SYSTEM_CURSOR_EW_RESIZE,
            Self::ResizeVertical => SDL_SYSTEM_CURSOR_NS_RESIZE,
            Self::Move => SDL_SYSTEM_CURSOR_MOVE,
            Self::NotAllowed => SDL_SYSTEM_CURSOR_NOT_ALLOWED,
            Self::Pointer => SDL_SYSTEM_CURSOR_POINTER,
        }
    }
}

/// The pixels of a custom cursor: straight-alpha RGBA, 8 bits per channel, row by row from the
/// top, and the hot spot (the pixel that points) from the top-left corner.
#[derive(Clone, Copy, Debug)]
pub struct CursorImage<'a> {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes.
    pub pixels: &'a [u8],
    /// The pixel that points, inside the image.
    pub hotspot: (u32, u32),
}

impl CursorImage<'_> {
    /// Images larger than this are refused: platforms cap cursors far below it.
    pub const MAX_SIZE: u32 = 256;

    /// Checks the size, the pixel count and the hot spot.
    fn validate(&self) -> Result<(), PlatformError> {
        let expected = u64::from(self.width) * u64::from(self.height) * 4;
        let valid = (1..=Self::MAX_SIZE).contains(&self.width)
            && (1..=Self::MAX_SIZE).contains(&self.height)
            && self.pixels.len() as u64 == expected
            && self.hotspot.0 < self.width
            && self.hotspot.1 < self.height;
        if valid {
            Ok(())
        } else {
            Err(PlatformError::new(
                "creating a cursor",
                format!(
                    "invalid {}x{} image of {} bytes with the hot spot at {:?}",
                    self.width,
                    self.height,
                    self.pixels.len(),
                    self.hotspot
                ),
            ))
        }
    }
}

/// A custom cursor created by [`crate::AppContext::create_cursor`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CursorId(usize);

/// The cursor to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cursor {
    /// A cursor of the platform.
    System(SystemCursor),
    /// A custom cursor.
    Custom(CursorId),
}

impl Default for Cursor {
    fn default() -> Self {
        Self::System(SystemCursor::Default)
    }
}

/// The SDL cursors created so far and the one shown.
#[derive(Debug, Default)]
pub(crate) struct Cursors {
    system: [Option<NonNull<SDL_Cursor>>; SystemCursor::ALL.len()],
    custom: Vec<NonNull<SDL_Cursor>>,
    shown: Option<Cursor>,
}

impl Cursors {
    /// Shows `cursor`, creating its SDL object the first time.
    pub(crate) fn show(&mut self, cursor: Cursor) -> Result<(), PlatformError> {
        if self.shown == Some(cursor) {
            return Ok(());
        }
        let raw = match cursor {
            Cursor::System(system) => {
                let slot = &mut self.system[system.index()];
                if let Some(raw) = slot {
                    *raw
                } else {
                    // SAFETY: SDL video is initialized while the application context exists;
                    // the identifier is a valid system cursor.
                    let created = unsafe { SDL_CreateSystemCursor(system.to_sdl()) };
                    let created = NonNull::new(created)
                        .ok_or_else(|| PlatformError::from_sdl("SDL_CreateSystemCursor"))?;
                    *slot = Some(created);
                    created
                }
            }
            Cursor::Custom(CursorId(index)) => *self.custom.get(index).ok_or_else(|| {
                PlatformError::new("showing a cursor", format!("unknown cursor {index}"))
            })?,
        };
        // SAFETY: `raw` is a live cursor owned by this cache.
        if !unsafe { SDL_SetCursor(raw.as_ptr()) } {
            return Err(PlatformError::from_sdl("SDL_SetCursor"));
        }
        self.shown = Some(cursor);
        Ok(())
    }

    /// Creates a custom cursor from `image`.
    pub(crate) fn create(&mut self, image: &CursorImage<'_>) -> Result<CursorId, PlatformError> {
        image.validate()?;
        let to_int = |value: u32| i32::try_from(value).unwrap_or(i32::MAX);
        // SAFETY: the pixels are `width * height * 4` bytes (validated) and outlive the
        // surface, which only reads them: it is destroyed below, after SDL copied the pixels
        // into the cursor.
        let surface = unsafe {
            SDL_CreateSurfaceFrom(
                to_int(image.width),
                to_int(image.height),
                SDL_PIXELFORMAT_RGBA32,
                image.pixels.as_ptr().cast_mut().cast(),
                to_int(image.width * 4),
            )
        };
        if surface.is_null() {
            return Err(PlatformError::from_sdl("SDL_CreateSurfaceFrom"));
        }
        // SAFETY: `surface` is a live surface; the hot spot lies inside it (validated).
        let cursor = unsafe {
            SDL_CreateColorCursor(surface, to_int(image.hotspot.0), to_int(image.hotspot.1))
        };
        // SAFETY: the surface was created above and is not used afterwards.
        unsafe { SDL_DestroySurface(surface) };
        let cursor =
            NonNull::new(cursor).ok_or_else(|| PlatformError::from_sdl("SDL_CreateColorCursor"))?;
        self.custom.push(cursor);
        Ok(CursorId(self.custom.len() - 1))
    }
}

impl Drop for Cursors {
    fn drop(&mut self) {
        for cursor in self.system.iter().flatten().chain(&self.custom) {
            // SAFETY: each cursor was created by SDL for this cache and is destroyed once,
            // before SDL shuts down (the context is dropped in `app_quit`).
            unsafe { SDL_DestroyCursor(cursor.as_ptr()) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_are_validated_before_reaching_sdl() {
        let pixels = [0_u8; 4 * 4 * 4];
        let image = |width, height, pixels, hotspot| CursorImage {
            width,
            height,
            pixels,
            hotspot,
        };
        assert!(image(4, 4, &pixels[..], (1, 2)).validate().is_ok());
        assert!(image(4, 4, &pixels[..60], (1, 2)).validate().is_err());
        assert!(image(4, 4, &pixels[..], (4, 0)).validate().is_err());
        assert!(image(0, 0, &pixels[..0], (0, 0)).validate().is_err());
    }

    #[test]
    fn system_cursors_have_distinct_slots() {
        for (index, cursor) in SystemCursor::ALL.iter().enumerate() {
            assert_eq!(cursor.index(), index);
        }
        assert_eq!(Cursor::default(), Cursor::System(SystemCursor::Default));
    }
}
