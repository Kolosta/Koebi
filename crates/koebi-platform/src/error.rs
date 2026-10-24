//! Platform errors.

use std::ffi::CStr;
use std::fmt;

/// A failing SDL call, with SDL's error message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformError {
    context: &'static str,
    message: String,
}

impl PlatformError {
    /// Creates an error from a context (usually the SDL function) and a message.
    pub fn new(context: &'static str, message: impl Into<String>) -> Self {
        Self {
            context,
            message: message.into(),
        }
    }

    /// Creates an error carrying SDL's current error string.
    pub(crate) fn from_sdl(context: &'static str) -> Self {
        Self::new(context, sdl_error())
    }

    /// The failing operation.
    pub fn context(&self) -> &'static str {
        self.context
    }

    /// The error message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.context, self.message)
    }
}

impl std::error::Error for PlatformError {}

/// SDL's last error message for the calling thread.
pub(crate) fn sdl_error() -> String {
    // SAFETY: SDL_GetError always returns a valid, NUL-terminated (possibly empty) string.
    let message = unsafe { CStr::from_ptr(sdl3_sys::error::SDL_GetError()) };
    let message = message.to_string_lossy();
    if message.is_empty() {
        String::from("unknown SDL error")
    } else {
        message.into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_combines_context_and_message() {
        let error = PlatformError::new("SDL_CreateWindow", "no video device");
        assert_eq!(error.to_string(), "SDL_CreateWindow: no video device");
        assert_eq!(error.context(), "SDL_CreateWindow");
    }
}
