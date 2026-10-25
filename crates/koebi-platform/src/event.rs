//! Input and lifecycle events, translated from SDL into plain Rust types.
//!
//! [`translate`] is a pure function of the SDL event (and of the modifier state SDL tracks,
//! passed in), unit-tested from zeroed `SDL_Event`s. Pointer positions come out in SDL window
//! coordinates; the event callback converts them to logical pixels with [`to_logical`].

use koebi_core::input::{Key, Modifiers, MouseButton};
use koebi_core::{Extent, Vec2};
use sdl3_sys::events::{
    SDL_EVENT_DID_ENTER_BACKGROUND, SDL_EVENT_DID_ENTER_FOREGROUND, SDL_EVENT_KEY_DOWN,
    SDL_EVENT_KEY_UP, SDL_EVENT_LOW_MEMORY, SDL_EVENT_MOUSE_BUTTON_DOWN, SDL_EVENT_MOUSE_BUTTON_UP,
    SDL_EVENT_MOUSE_MOTION, SDL_EVENT_MOUSE_WHEEL, SDL_EVENT_PINCH_BEGIN, SDL_EVENT_PINCH_END,
    SDL_EVENT_PINCH_UPDATE, SDL_EVENT_QUIT, SDL_EVENT_TERMINATING, SDL_EVENT_WILL_ENTER_BACKGROUND,
    SDL_EVENT_WILL_ENTER_FOREGROUND, SDL_EVENT_WINDOW_CLOSE_REQUESTED,
    SDL_EVENT_WINDOW_DISPLAY_SCALE_CHANGED, SDL_EVENT_WINDOW_EXPOSED,
    SDL_EVENT_WINDOW_FOCUS_GAINED, SDL_EVENT_WINDOW_FOCUS_LOST, SDL_EVENT_WINDOW_MINIMIZED,
    SDL_EVENT_WINDOW_MOUSE_ENTER, SDL_EVENT_WINDOW_MOUSE_LEAVE,
    SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED, SDL_EVENT_WINDOW_RESIZED, SDL_EVENT_WINDOW_RESTORED,
    SDL_Event, SDL_EventType,
};
use sdl3_sys::keycode::{
    SDL_KMOD_LALT, SDL_KMOD_LCTRL, SDL_KMOD_LGUI, SDL_KMOD_LSHIFT, SDL_KMOD_RALT, SDL_KMOD_RCTRL,
    SDL_KMOD_RGUI, SDL_KMOD_RSHIFT, SDL_Keycode, SDL_Keymod, SDLK_APOSTROPHE, SDLK_APPLICATION,
    SDLK_BACKSLASH, SDLK_BACKSPACE, SDLK_CAPSLOCK, SDLK_COMMA, SDLK_DELETE, SDLK_DOWN, SDLK_END,
    SDLK_EQUALS, SDLK_ESCAPE, SDLK_F1, SDLK_F2, SDLK_F3, SDLK_F4, SDLK_F5, SDLK_F6, SDLK_F7,
    SDLK_F8, SDLK_F9, SDLK_F10, SDLK_F11, SDLK_F12, SDLK_F13, SDLK_F14, SDLK_F15, SDLK_F16,
    SDLK_F17, SDLK_F18, SDLK_F19, SDLK_F20, SDLK_F21, SDLK_F22, SDLK_F23, SDLK_F24, SDLK_GRAVE,
    SDLK_HOME, SDLK_INSERT, SDLK_KP_0, SDLK_KP_1, SDLK_KP_2, SDLK_KP_3, SDLK_KP_4, SDLK_KP_5,
    SDLK_KP_6, SDLK_KP_7, SDLK_KP_8, SDLK_KP_9, SDLK_KP_DIVIDE, SDLK_KP_ENTER, SDLK_KP_MINUS,
    SDLK_KP_MULTIPLY, SDLK_KP_PERIOD, SDLK_KP_PLUS, SDLK_LALT, SDLK_LCTRL, SDLK_LEFT,
    SDLK_LEFTBRACKET, SDLK_LGUI, SDLK_LSHIFT, SDLK_MINUS, SDLK_NUMLOCKCLEAR, SDLK_PAGEDOWN,
    SDLK_PAGEUP, SDLK_PAUSE, SDLK_PERIOD, SDLK_PRINTSCREEN, SDLK_RALT, SDLK_RCTRL, SDLK_RETURN,
    SDLK_RGUI, SDLK_RIGHT, SDLK_RIGHTBRACKET, SDLK_RSHIFT, SDLK_SCANCODE_MASK, SDLK_SCROLLLOCK,
    SDLK_SEMICOLON, SDLK_SLASH, SDLK_SPACE, SDLK_TAB, SDLK_UP,
};
use sdl3_sys::mouse::SDL_MOUSEWHEEL_FLIPPED;

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
        /// The key, as the current layout labels it.
        key: Key,
        /// The key's physical position (a USB HID usage), whatever the layout.
        scancode: u32,
        /// `true` on press, `false` on release.
        down: bool,
        /// `true` for auto-repeated presses while the key stays down.
        repeat: bool,
        /// Modifier keys held, this key included once pressed.
        modifiers: Modifiers,
    },
    /// The mouse moved.
    MouseMotion {
        /// Window under the mouse.
        window: WindowId,
        /// Position in logical pixels, the unit of the window's scene.
        position: Vec2,
        /// Modifier keys held.
        modifiers: Modifiers,
    },
    /// A mouse button was pressed or released.
    MouseButton {
        /// Window under the mouse.
        window: WindowId,
        /// The button.
        button: MouseButton,
        /// `true` on press.
        down: bool,
        /// Consecutive clicks counted by the platform: 1 for a click, 2 for a double click.
        clicks: u8,
        /// Position in logical pixels.
        position: Vec2,
        /// Modifier keys held.
        modifiers: Modifiers,
    },
    /// The mouse wheel or the touchpad scrolled.
    MouseWheel {
        /// Window under the mouse.
        window: WindowId,
        /// Scroll amount in wheel notches, fractions for touchpads: positive `y` scrolls
        /// away from the user (up), positive `x` to the right, whatever the system's "natural
        /// scrolling" setting.
        delta: Vec2,
        /// Pointer position in logical pixels.
        position: Vec2,
        /// Modifier keys held.
        modifiers: Modifiers,
    },
    /// A two-finger pinch on a touchpad or touch screen.
    Pinch {
        /// Window under the fingers.
        window: WindowId,
        /// Where the gesture is.
        phase: GesturePhase,
        /// Scale change since the previous update: above 1 zooms in, below 1 zooms out
        /// (1 for the begin and end phases).
        scale: f32,
    },
}

/// The phase of a continuous gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GesturePhase {
    /// The gesture started.
    Begin,
    /// The gesture continues.
    Update,
    /// The gesture ended.
    End,
}

/// Window-specific events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WindowEvent {
    /// The close button was pressed.
    CloseRequested,
    /// New size in SDL window coordinates: pixels on Windows and most Linux setups, points on
    /// Apple platforms. Use [`crate::Window::size_in_pixels`] for the drawable size.
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
    /// The pointer entered the window.
    PointerEntered,
    /// The pointer left the window: nothing in it is under the pointer any more.
    PointerLeft,
    /// The window received the keyboard focus.
    FocusGained,
    /// The window lost the keyboard focus: keys held may be released elsewhere.
    FocusLost,
}

/// Every named key with its SDL key code; letters, digits and other characters are converted
/// from their code points.
const KEYS: &[(SDL_Keycode, Key)] = &[
    (SDLK_ESCAPE, Key::Escape),
    (SDLK_RETURN, Key::Enter),
    (SDLK_TAB, Key::Tab),
    (SDLK_BACKSPACE, Key::Backspace),
    (SDLK_DELETE, Key::Delete),
    (SDLK_INSERT, Key::Insert),
    (SDLK_HOME, Key::Home),
    (SDLK_END, Key::End),
    (SDLK_PAGEUP, Key::PageUp),
    (SDLK_PAGEDOWN, Key::PageDown),
    (SDLK_UP, Key::Up),
    (SDLK_DOWN, Key::Down),
    (SDLK_LEFT, Key::Left),
    (SDLK_RIGHT, Key::Right),
    (SDLK_SPACE, Key::Space),
    (SDLK_MINUS, Key::Minus),
    (SDLK_EQUALS, Key::Equals),
    (SDLK_LEFTBRACKET, Key::LeftBracket),
    (SDLK_RIGHTBRACKET, Key::RightBracket),
    (SDLK_BACKSLASH, Key::Backslash),
    (SDLK_SEMICOLON, Key::Semicolon),
    (SDLK_APOSTROPHE, Key::Apostrophe),
    (SDLK_GRAVE, Key::Grave),
    (SDLK_COMMA, Key::Comma),
    (SDLK_PERIOD, Key::Period),
    (SDLK_SLASH, Key::Slash),
    (SDLK_CAPSLOCK, Key::CapsLock),
    (SDLK_PRINTSCREEN, Key::PrintScreen),
    (SDLK_SCROLLLOCK, Key::ScrollLock),
    (SDLK_PAUSE, Key::Pause),
    (SDLK_APPLICATION, Key::Menu),
    (SDLK_NUMLOCKCLEAR, Key::NumLock),
    (SDLK_F1, Key::F1),
    (SDLK_F2, Key::F2),
    (SDLK_F3, Key::F3),
    (SDLK_F4, Key::F4),
    (SDLK_F5, Key::F5),
    (SDLK_F6, Key::F6),
    (SDLK_F7, Key::F7),
    (SDLK_F8, Key::F8),
    (SDLK_F9, Key::F9),
    (SDLK_F10, Key::F10),
    (SDLK_F11, Key::F11),
    (SDLK_F12, Key::F12),
    (SDLK_F13, Key::F13),
    (SDLK_F14, Key::F14),
    (SDLK_F15, Key::F15),
    (SDLK_F16, Key::F16),
    (SDLK_F17, Key::F17),
    (SDLK_F18, Key::F18),
    (SDLK_F19, Key::F19),
    (SDLK_F20, Key::F20),
    (SDLK_F21, Key::F21),
    (SDLK_F22, Key::F22),
    (SDLK_F23, Key::F23),
    (SDLK_F24, Key::F24),
    (SDLK_KP_0, Key::Keypad0),
    (SDLK_KP_1, Key::Keypad1),
    (SDLK_KP_2, Key::Keypad2),
    (SDLK_KP_3, Key::Keypad3),
    (SDLK_KP_4, Key::Keypad4),
    (SDLK_KP_5, Key::Keypad5),
    (SDLK_KP_6, Key::Keypad6),
    (SDLK_KP_7, Key::Keypad7),
    (SDLK_KP_8, Key::Keypad8),
    (SDLK_KP_9, Key::Keypad9),
    (SDLK_KP_DIVIDE, Key::KeypadDivide),
    (SDLK_KP_MULTIPLY, Key::KeypadMultiply),
    (SDLK_KP_MINUS, Key::KeypadMinus),
    (SDLK_KP_PLUS, Key::KeypadPlus),
    (SDLK_KP_ENTER, Key::KeypadEnter),
    (SDLK_KP_PERIOD, Key::KeypadPeriod),
    (SDLK_LSHIFT, Key::LeftShift),
    (SDLK_RSHIFT, Key::RightShift),
    (SDLK_LCTRL, Key::LeftCtrl),
    (SDLK_RCTRL, Key::RightCtrl),
    (SDLK_LALT, Key::LeftAlt),
    (SDLK_RALT, Key::RightAlt),
    (SDLK_LGUI, Key::LeftGui),
    (SDLK_RGUI, Key::RightGui),
];

/// Letters and digits in code point order.
const LETTERS: [Key; 26] = [
    Key::A,
    Key::B,
    Key::C,
    Key::D,
    Key::E,
    Key::F,
    Key::G,
    Key::H,
    Key::I,
    Key::J,
    Key::K,
    Key::L,
    Key::M,
    Key::N,
    Key::O,
    Key::P,
    Key::Q,
    Key::R,
    Key::S,
    Key::T,
    Key::U,
    Key::V,
    Key::W,
    Key::X,
    Key::Y,
    Key::Z,
];
const DIGITS: [Key; 10] = [
    Key::Digit0,
    Key::Digit1,
    Key::Digit2,
    Key::Digit3,
    Key::Digit4,
    Key::Digit5,
    Key::Digit6,
    Key::Digit7,
    Key::Digit8,
    Key::Digit9,
];

/// The [`Key`] of an SDL key code: named keys, letters and digits of any layout, other
/// printable characters, or [`Key::Unknown`].
pub(crate) fn key_from_sdl(keycode: SDL_Keycode) -> Key {
    if let Some((_, key)) = KEYS.iter().find(|(code, _)| *code == keycode) {
        return *key;
    }
    let code = keycode.0;
    if code & SDLK_SCANCODE_MASK.0 != 0 {
        return Key::Unknown(code);
    }
    match char::from_u32(code) {
        Some(letter @ 'a'..='z') => LETTERS[letter as usize - 'a' as usize],
        Some(letter @ 'A'..='Z') => LETTERS[letter as usize - 'A' as usize],
        Some(digit @ '0'..='9') => DIGITS[digit as usize - '0' as usize],
        Some(character) if !character.is_control() => Key::Character(character),
        _ => Key::Unknown(code),
    }
}

/// The [`Modifiers`] of an SDL modifier state; lock keys are ignored.
pub(crate) fn modifiers_from_sdl(state: SDL_Keymod) -> Modifiers {
    [
        (SDL_KMOD_LSHIFT, Modifiers::LEFT_SHIFT),
        (SDL_KMOD_RSHIFT, Modifiers::RIGHT_SHIFT),
        (SDL_KMOD_LCTRL, Modifiers::LEFT_CTRL),
        (SDL_KMOD_RCTRL, Modifiers::RIGHT_CTRL),
        (SDL_KMOD_LALT, Modifiers::LEFT_ALT),
        (SDL_KMOD_RALT, Modifiers::RIGHT_ALT),
        (SDL_KMOD_LGUI, Modifiers::LEFT_GUI),
        (SDL_KMOD_RGUI, Modifiers::RIGHT_GUI),
    ]
    .into_iter()
    .filter(|(sdl, _)| state.0 & sdl.0 != 0)
    .fold(Modifiers::NONE, |held, (_, modifier)| held.union(modifier))
}

fn to_extent(width: i32, height: i32) -> Extent {
    Extent::new(
        u32::try_from(width).unwrap_or(0),
        u32::try_from(height).unwrap_or(0),
    )
}

impl Event {
    /// The window of a pointer event that carries a position.
    pub(crate) fn pointer_window(&self) -> Option<WindowId> {
        match *self {
            Self::MouseMotion { window, .. }
            | Self::MouseButton { window, .. }
            | Self::MouseWheel { window, .. } => Some(window),
            _ => None,
        }
    }
}

/// Converts the position of a pointer event from SDL window coordinates to logical pixels,
/// `scale` being the logical pixels per window coordinate of the event's window
/// ([`crate::window::logical_per_window_coordinate`]). Other events are returned unchanged.
pub(crate) fn to_logical(event: Event, scale: f32) -> Event {
    match event {
        Event::MouseMotion {
            window,
            position,
            modifiers,
        } => Event::MouseMotion {
            window,
            position: position * scale,
            modifiers,
        },
        Event::MouseButton {
            window,
            button,
            down,
            clicks,
            position,
            modifiers,
        } => Event::MouseButton {
            window,
            button,
            down,
            clicks,
            position: position * scale,
            modifiers,
        },
        Event::MouseWheel {
            window,
            delta,
            position,
            modifiers,
        } => Event::MouseWheel {
            window,
            delta,
            position: position * scale,
            modifiers,
        },
        other => other,
    }
}

/// Translates an SDL event; returns `None` for events Koebi does not handle yet.
/// `modifiers` is the modifier state SDL tracks (`SDL_GetModState`), given to pointer events;
/// key events carry their own. Pointer positions stay in SDL window coordinates: see
/// [`to_logical`].
pub(crate) fn translate(event: &SDL_Event, modifiers: Modifiers) -> Option<Event> {
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
            key: key_from_sdl(key.key),
            scancode: u32::try_from(key.scancode.0).unwrap_or(0),
            down: key.down,
            repeat: key.repeat,
            modifiers: modifiers_from_sdl(key.r#mod),
        });
    }
    if kind == SDL_EVENT_MOUSE_MOTION {
        // SAFETY: motion events use the `motion` member.
        let motion = unsafe { event.motion };
        return Some(Event::MouseMotion {
            window: WindowId(motion.windowID.0),
            position: Vec2::new(motion.x, motion.y),
            modifiers,
        });
    }
    if kind == SDL_EVENT_MOUSE_BUTTON_DOWN || kind == SDL_EVENT_MOUSE_BUTTON_UP {
        // SAFETY: button events use the `button` member.
        let button = unsafe { event.button };
        return Some(Event::MouseButton {
            window: WindowId(button.windowID.0),
            button: MouseButton::from(button.button),
            down: button.down,
            clicks: button.clicks,
            position: Vec2::new(button.x, button.y),
            modifiers,
        });
    }
    if kind == SDL_EVENT_MOUSE_WHEEL {
        // SAFETY: wheel events use the `wheel` member.
        let wheel = unsafe { event.wheel };
        // With "natural scrolling", SDL reports flipped values: undo it, so that positive y
        // always means scrolling away from the user.
        let sign = if wheel.direction == SDL_MOUSEWHEEL_FLIPPED {
            -1.0
        } else {
            1.0
        };
        return Some(Event::MouseWheel {
            window: WindowId(wheel.windowID.0),
            delta: Vec2::new(wheel.x, wheel.y) * sign,
            position: Vec2::new(wheel.mouse_x, wheel.mouse_y),
            modifiers,
        });
    }
    let pinch = [
        (SDL_EVENT_PINCH_BEGIN, GesturePhase::Begin),
        (SDL_EVENT_PINCH_UPDATE, GesturePhase::Update),
        (SDL_EVENT_PINCH_END, GesturePhase::End),
    ];
    if let Some((_, phase)) = pinch.iter().find(|(sdl, _)| *sdl == kind) {
        // SAFETY: pinch events use the `pinch` member.
        let pinch = unsafe { event.pinch };
        let scale = if pinch.scale.is_finite() && pinch.scale > 0.0 {
            pinch.scale
        } else {
            1.0
        };
        return Some(Event::Pinch {
            window: WindowId(pinch.windowID.0),
            phase: *phase,
            scale,
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
        k if k == SDL_EVENT_WINDOW_MOUSE_ENTER => WindowEvent::PointerEntered,
        k if k == SDL_EVENT_WINDOW_MOUSE_LEAVE => WindowEvent::PointerLeft,
        k if k == SDL_EVENT_WINDOW_FOCUS_GAINED => WindowEvent::FocusGained,
        k if k == SDL_EVENT_WINDOW_FOCUS_LOST => WindowEvent::FocusLost,
        _ => return None,
    };
    Some(Event::Window {
        window: WindowId(window.windowID.0),
        event: window_event,
    })
}

#[cfg(test)]
mod tests {
    use sdl3_sys::keycode::SDLK_A;
    use sdl3_sys::scancode::SDL_SCANCODE_A;

    use super::*;

    fn blank() -> SDL_Event {
        // SAFETY: `SDL_Event` is a C union of plain data for which all-zero bytes are valid.
        unsafe { core::mem::zeroed() }
    }

    #[test]
    fn lifecycle_events_are_translated() {
        let mut event = blank();
        event.r#type = SDL_EVENT_QUIT.0;
        assert_eq!(translate(&event, Modifiers::NONE), Some(Event::Quit));
        event.r#type = SDL_EVENT_DID_ENTER_FOREGROUND.0;
        assert_eq!(
            translate(&event, Modifiers::NONE),
            Some(Event::DidEnterForeground)
        );
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
            translate(&event, Modifiers::NONE),
            Some(Event::Window {
                window: WindowId(7),
                event: WindowEvent::PixelSizeChanged(Extent::new(1600, 0)),
            })
        );
    }

    #[test]
    fn keys_carry_their_code_position_and_modifier_sides() {
        let mut event = blank();
        event.key.r#type = SDL_EVENT_KEY_DOWN;
        event.key.windowID = sdl3_sys::video::SDL_WindowID(1);
        event.key.key = SDLK_A;
        event.key.scancode = SDL_SCANCODE_A;
        event.key.r#mod = SDL_Keymod(SDL_KMOD_LCTRL.0 | SDL_KMOD_RSHIFT.0);
        event.key.down = true;
        event.key.repeat = true;
        assert_eq!(
            translate(&event, Modifiers::NONE),
            Some(Event::Key {
                window: WindowId(1),
                key: Key::A,
                scancode: 4,
                down: true,
                repeat: true,
                modifiers: Modifiers::LEFT_CTRL.union(Modifiers::RIGHT_SHIFT),
            })
        );
    }

    #[test]
    fn key_codes_map_to_keys_in_any_layout() {
        assert_eq!(key_from_sdl(SDLK_ESCAPE), Key::Escape);
        assert_eq!(key_from_sdl(SDLK_KP_PLUS), Key::KeypadPlus);
        assert_eq!(key_from_sdl(SDLK_F24), Key::F24);
        assert_eq!(key_from_sdl(SDLK_RGUI), Key::RightGui);
        assert_eq!(key_from_sdl(SDL_Keycode('z' as u32)), Key::Z);
        assert_eq!(key_from_sdl(SDL_Keycode('7' as u32)), Key::Digit7);
        // An AZERTY key producing é, and a key SDL only knows by scan code.
        assert_eq!(key_from_sdl(SDL_Keycode('é' as u32)), Key::Character('é'));
        let mute = SDL_Keycode(SDLK_SCANCODE_MASK.0 | 127);
        assert_eq!(key_from_sdl(mute), Key::Unknown(mute.0));
    }

    #[test]
    fn modifier_states_ignore_lock_keys() {
        let state =
            SDL_Keymod(SDL_KMOD_RALT.0 | SDL_KMOD_LGUI.0 | sdl3_sys::keycode::SDL_KMOD_CAPS.0);
        assert_eq!(
            modifiers_from_sdl(state),
            Modifiers::RIGHT_ALT.union(Modifiers::LEFT_GUI)
        );
    }

    #[test]
    fn mouse_buttons_carry_clicks_and_modifiers() {
        let mut event = blank();
        event.button.r#type = SDL_EVENT_MOUSE_BUTTON_DOWN;
        event.button.windowID = sdl3_sys::video::SDL_WindowID(2);
        event.button.button = 3;
        event.button.down = true;
        event.button.clicks = 2;
        event.button.x = 10.0;
        event.button.y = 20.0;
        assert_eq!(
            translate(&event, Modifiers::LEFT_ALT),
            Some(Event::MouseButton {
                window: WindowId(2),
                button: MouseButton::Right,
                down: true,
                clicks: 2,
                position: Vec2::new(10.0, 20.0),
                modifiers: Modifiers::LEFT_ALT,
            })
        );
    }

    #[test]
    fn natural_scrolling_is_undone() {
        let mut event = blank();
        event.wheel.r#type = SDL_EVENT_MOUSE_WHEEL;
        event.wheel.windowID = sdl3_sys::video::SDL_WindowID(1);
        event.wheel.x = 0.25;
        event.wheel.y = 1.0;
        event.wheel.direction = SDL_MOUSEWHEEL_FLIPPED;
        event.wheel.mouse_x = 5.0;
        event.wheel.mouse_y = 6.0;
        assert_eq!(
            translate(&event, Modifiers::NONE),
            Some(Event::MouseWheel {
                window: WindowId(1),
                delta: Vec2::new(-0.25, -1.0),
                position: Vec2::new(5.0, 6.0),
                modifiers: Modifiers::NONE,
            })
        );
    }

    #[test]
    fn pinches_report_their_phase_and_scale() {
        let mut event = blank();
        event.pinch.r#type = SDL_EVENT_PINCH_UPDATE;
        event.pinch.windowID = sdl3_sys::video::SDL_WindowID(1);
        event.pinch.scale = 1.25;
        assert_eq!(
            translate(&event, Modifiers::NONE),
            Some(Event::Pinch {
                window: WindowId(1),
                phase: GesturePhase::Update,
                scale: 1.25,
            })
        );
        event.pinch.r#type = SDL_EVENT_PINCH_BEGIN;
        event.pinch.scale = 0.0;
        assert!(matches!(
            translate(&event, Modifiers::NONE),
            Some(Event::Pinch {
                phase: GesturePhase::Begin,
                scale: 1.0,
                ..
            })
        ));
    }

    #[test]
    fn window_focus_and_pointer_crossing_are_translated() {
        let mut event = blank();
        event.window.windowID = sdl3_sys::video::SDL_WindowID(3);
        for (sdl, expected) in [
            (SDL_EVENT_WINDOW_MOUSE_ENTER, WindowEvent::PointerEntered),
            (SDL_EVENT_WINDOW_MOUSE_LEAVE, WindowEvent::PointerLeft),
            (SDL_EVENT_WINDOW_FOCUS_GAINED, WindowEvent::FocusGained),
            (SDL_EVENT_WINDOW_FOCUS_LOST, WindowEvent::FocusLost),
        ] {
            event.window.r#type = sdl;
            assert_eq!(
                translate(&event, Modifiers::NONE),
                Some(Event::Window {
                    window: WindowId(3),
                    event: expected,
                })
            );
        }
    }

    #[test]
    fn pointer_positions_are_converted_to_logical_pixels() {
        // Regression test for #65: on Windows at 125 %, window coordinates are pixels and a
        // logical pixel is 1.25 pixels, so a click at pixel (400, 200) is at (320, 160) in the
        // scene. Positions used to pass through unconverted.
        let window = WindowId(1);
        let click = Event::MouseButton {
            window,
            button: MouseButton::Left,
            down: true,
            clicks: 1,
            position: Vec2::new(400.0, 200.0),
            modifiers: Modifiers::NONE,
        };
        assert_eq!(click.pointer_window(), Some(window));
        assert!(matches!(
            to_logical(click, 1.0 / 1.25),
            Event::MouseButton { position, .. } if position == Vec2::new(320.0, 160.0)
        ));
        // On a Retina display, window coordinates are points: already logical pixels.
        let motion = Event::MouseMotion {
            window,
            position: Vec2::new(400.0, 200.0),
            modifiers: Modifiers::NONE,
        };
        assert_eq!(to_logical(motion, 1.0), motion);
        // Wheel events carry a position too; their delta is not a position.
        let wheel = Event::MouseWheel {
            window,
            delta: Vec2::new(0.0, 1.0),
            position: Vec2::new(100.0, 50.0),
            modifiers: Modifiers::NONE,
        };
        assert_eq!(
            to_logical(wheel, 0.5),
            Event::MouseWheel {
                window,
                delta: Vec2::new(0.0, 1.0),
                position: Vec2::new(50.0, 25.0),
                modifiers: Modifiers::NONE,
            }
        );
    }

    #[test]
    fn unknown_events_are_ignored() {
        let mut event = blank();
        event.r#type = sdl3_sys::events::SDL_EVENT_CLIPBOARD_UPDATE.0;
        assert_eq!(translate(&event, Modifiers::NONE), None);
    }
}
