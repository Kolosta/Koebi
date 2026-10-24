//! Input and lifecycle events, translated from SDL into plain Rust types.

use koebi_core::{Extent, Vec2};
use sdl3_sys::events::{
    SDL_EVENT_DID_ENTER_BACKGROUND, SDL_EVENT_DID_ENTER_FOREGROUND, SDL_EVENT_KEY_DOWN,
    SDL_EVENT_KEY_UP, SDL_EVENT_LOW_MEMORY, SDL_EVENT_MOUSE_BUTTON_DOWN, SDL_EVENT_MOUSE_BUTTON_UP,
    SDL_EVENT_MOUSE_MOTION, SDL_EVENT_MOUSE_WHEEL, SDL_EVENT_QUIT, SDL_EVENT_TERMINATING,
    SDL_EVENT_WILL_ENTER_BACKGROUND, SDL_EVENT_WILL_ENTER_FOREGROUND,
    SDL_EVENT_WINDOW_CLOSE_REQUESTED, SDL_EVENT_WINDOW_DISPLAY_SCALE_CHANGED,
    SDL_EVENT_WINDOW_EXPOSED, SDL_EVENT_WINDOW_MINIMIZED, SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED,
    SDL_EVENT_WINDOW_RESIZED, SDL_EVENT_WINDOW_RESTORED, SDL_Event, SDL_EventType,
};

use crate::window::WindowId;

/// An event delivered to [`crate::App::event`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// The user or the OS asked the application to quit.
    Quit,
    /// The OS is terminating the application (mobile). Save state now.
    Terminating,
    /// The OS is low on memory (mobile). Free caches.
    LowMemory,
    /// The application is about to go to the background (mobile). Stop rendering.
    WillEnterBackground,
    /// The application is in the background; its window surface is gone.
    DidEnterBackground,
    /// The application is about to return to the foreground.
    WillEnterForeground,
    /// The application is in the foreground again; recreate the window surface.
    DidEnterForeground,
    /// Something happened to a window.
    Window {
        /// The window concerned.
        window: WindowId,
        /// What happened.
        event: WindowEvent,
    },
    /// A key was pressed or released.
    Key {
        /// Window with keyboard focus.
        window: WindowId,
        /// Layout-dependent key code.
        key: Key,
        /// `true` on press, `false` on release.
        down: bool,
        /// `true` for auto-repeated presses.
        repeat: bool,
    },
    /// The mouse moved.
    MouseMotion {
        /// Window under the mouse.
        window: WindowId,
        /// Position in logical pixels.
        position: Vec2,
    },
    /// A mouse button was pressed or released.
    MouseButton {
        /// Window under the mouse.
        window: WindowId,
        /// The button.
        button: MouseButton,
        /// `true` on press.
        down: bool,
        /// Position in logical pixels.
        position: Vec2,
    },
    /// The mouse wheel or touchpad scrolled.
    MouseWheel {
        /// Window under the mouse.
        window: WindowId,
        /// Scroll amount (positive `y` scrolls away from the user).
        delta: Vec2,
    },
}

/// Window-specific events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WindowEvent {
    /// The close button was pressed.
    CloseRequested,
    /// New size in logical units.
    Resized {
        /// Width.
        width: i32,
        /// Height.
        height: i32,
    },
    /// New framebuffer size in physical pixels: resize the swapchain.
    PixelSizeChanged(Extent),
    /// The window must be redrawn.
    Exposed,
    /// The window was minimized.
    Minimized,
    /// The window was restored.
    Restored,
    /// The display scale changed (moved to another monitor, OS setting).
    DisplayScaleChanged,
}

/// A layout-dependent key code (an SDL keycode).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key(pub u32);

impl Key {
    /// Escape.
    pub const ESCAPE: Self = Self(sdl3_sys::keycode::SDLK_ESCAPE.0);
    /// Return / Enter.
    pub const RETURN: Self = Self(sdl3_sys::keycode::SDLK_RETURN.0);
    /// Space bar.
    pub const SPACE: Self = Self(sdl3_sys::keycode::SDLK_SPACE.0);
    /// F11.
    pub const F11: Self = Self(sdl3_sys::keycode::SDLK_F11.0);
}

/// A mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Primary button.
    Left,
    /// Wheel button.
    Middle,
    /// Secondary button.
    Right,
    /// Any other button, by SDL index.
    Other(u8),
}

impl From<u8> for MouseButton {
    fn from(index: u8) -> Self {
        match index {
            1 => Self::Left,
            2 => Self::Middle,
            3 => Self::Right,
            other => Self::Other(other),
        }
    }
}

fn to_extent(width: i32, height: i32) -> Extent {
    Extent::new(
        u32::try_from(width).unwrap_or(0),
        u32::try_from(height).unwrap_or(0),
    )
}

/// Translates an SDL event; returns `None` for events Koebi does not handle yet.
pub(crate) fn translate(event: &SDL_Event) -> Option<Event> {
    // SAFETY: `type` is the common first member of every event variant.
    let kind = SDL_EventType(unsafe { event.r#type });
    let lifecycle = [
        (SDL_EVENT_QUIT, Event::Quit),
        (SDL_EVENT_TERMINATING, Event::Terminating),
        (SDL_EVENT_LOW_MEMORY, Event::LowMemory),
        (SDL_EVENT_WILL_ENTER_BACKGROUND, Event::WillEnterBackground),
        (SDL_EVENT_DID_ENTER_BACKGROUND, Event::DidEnterBackground),
        (SDL_EVENT_WILL_ENTER_FOREGROUND, Event::WillEnterForeground),
        (SDL_EVENT_DID_ENTER_FOREGROUND, Event::DidEnterForeground),
    ];
    if let Some((_, translated)) = lifecycle.iter().find(|(sdl, _)| *sdl == kind) {
        return Some(*translated);
    }

    if kind == SDL_EVENT_KEY_DOWN || kind == SDL_EVENT_KEY_UP {
        // SAFETY: keyboard events use the `key` member.
        let key = unsafe { event.key };
        return Some(Event::Key {
            window: WindowId(key.windowID.0),
            key: Key(key.key.0),
            down: key.down,
            repeat: key.repeat,
        });
    }
    if kind == SDL_EVENT_MOUSE_MOTION {
        // SAFETY: motion events use the `motion` member.
        let motion = unsafe { event.motion };
        return Some(Event::MouseMotion {
            window: WindowId(motion.windowID.0),
            position: Vec2::new(motion.x, motion.y),
        });
    }
    if kind == SDL_EVENT_MOUSE_BUTTON_DOWN || kind == SDL_EVENT_MOUSE_BUTTON_UP {
        // SAFETY: button events use the `button` member.
        let button = unsafe { event.button };
        return Some(Event::MouseButton {
            window: WindowId(button.windowID.0),
            button: MouseButton::from(button.button),
            down: button.down,
            position: Vec2::new(button.x, button.y),
        });
    }
    if kind == SDL_EVENT_MOUSE_WHEEL {
        // SAFETY: wheel events use the `wheel` member.
        let wheel = unsafe { event.wheel };
        return Some(Event::MouseWheel {
            window: WindowId(wheel.windowID.0),
            delta: Vec2::new(wheel.x, wheel.y),
        });
    }

    // SAFETY: every remaining event handled below is a window event using `window`.
    let window = unsafe { event.window };
    let window_event = match kind {
        k if k == SDL_EVENT_WINDOW_CLOSE_REQUESTED => WindowEvent::CloseRequested,
        k if k == SDL_EVENT_WINDOW_RESIZED => WindowEvent::Resized {
            width: window.data1,
            height: window.data2,
        },
        k if k == SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED => {
            WindowEvent::PixelSizeChanged(to_extent(window.data1, window.data2))
        }
        k if k == SDL_EVENT_WINDOW_EXPOSED => WindowEvent::Exposed,
        k if k == SDL_EVENT_WINDOW_MINIMIZED => WindowEvent::Minimized,
        k if k == SDL_EVENT_WINDOW_RESTORED => WindowEvent::Restored,
        k if k == SDL_EVENT_WINDOW_DISPLAY_SCALE_CHANGED => WindowEvent::DisplayScaleChanged,
        _ => return None,
    };
    Some(Event::Window {
        window: WindowId(window.windowID.0),
        event: window_event,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank() -> SDL_Event {
        // SAFETY: `SDL_Event` is a C union of plain data for which all-zero bytes are valid.
        unsafe { core::mem::zeroed() }
    }

    #[test]
    fn lifecycle_events_are_translated() {
        let mut event = blank();
        event.r#type = SDL_EVENT_QUIT.0;
        assert_eq!(translate(&event), Some(Event::Quit));
        event.r#type = SDL_EVENT_DID_ENTER_FOREGROUND.0;
        assert_eq!(translate(&event), Some(Event::DidEnterForeground));
    }

    #[test]
    fn pixel_size_changes_carry_the_new_extent() {
        let mut event = blank();
        // Writing union fields is safe: it cannot produce an invalid value.
        event.window.r#type = SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED;
        event.window.windowID = sdl3_sys::video::SDL_WindowID(7);
        event.window.data1 = 1600;
        event.window.data2 = -1;
        assert_eq!(
            translate(&event),
            Some(Event::Window {
                window: WindowId(7),
                event: WindowEvent::PixelSizeChanged(Extent::new(1600, 0)),
            })
        );
    }

    #[test]
    fn keys_and_mouse_buttons_are_translated() {
        let mut event = blank();
        event.key.r#type = SDL_EVENT_KEY_DOWN;
        event.key.windowID = sdl3_sys::video::SDL_WindowID(1);
        event.key.key = sdl3_sys::keycode::SDLK_ESCAPE;
        event.key.down = true;
        assert_eq!(
            translate(&event),
            Some(Event::Key {
                window: WindowId(1),
                key: Key::ESCAPE,
                down: true,
                repeat: false
            })
        );
        assert_eq!(MouseButton::from(3), MouseButton::Right);
        assert_eq!(MouseButton::from(9), MouseButton::Other(9));
    }

    #[test]
    fn unknown_events_are_ignored() {
        let mut event = blank();
        event.r#type = sdl3_sys::events::SDL_EVENT_CLIPBOARD_UPDATE.0;
        assert_eq!(translate(&event), None);
    }
}
