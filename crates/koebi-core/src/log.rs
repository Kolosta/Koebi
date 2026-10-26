//! Minimal, dependency-free logging facade.
//!
//! Libraries log through [`error!`](crate::error), [`warn!`](crate::warn),
//! [`info!`](crate::info), [`debug!`](crate::debug) and [`trace!`](crate::trace). The
//! maximum level comes from the `KOEBI_LOG` environment variable (`off`, `error`, `warn`,
//! `info`, `debug`, `trace`; default `info`) and can be changed at runtime with
//! [`set_max_level`].
//!
//! Records go to one process-wide [`Sink`]: stderr by default, logcat on Android. An
//! application may install its own sink once, early, with [`set_sink`].
//!
//! A disabled level costs one relaxed atomic load; its format arguments are not evaluated.

use core::fmt;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

/// Severity of a log record, from most to least severe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Level {
    /// Something failed; the operation could not complete.
    Error = 1,
    /// Something unexpected happened but the program recovered.
    Warn = 2,
    /// High-level progress useful to every user (startup, adapter selection...).
    Info = 3,
    /// Detailed information for developers.
    Debug = 4,
    /// Very verbose, per-frame information.
    Trace = 5,
}

impl Level {
    /// Upper-case name, as printed by the default sink.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
            Self::Debug => "DEBUG",
            Self::Trace => "TRACE",
        }
    }

    const fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Error),
            2 => Some(Self::Warn),
            3 => Some(Self::Info),
            4 => Some(Self::Debug),
            5 => Some(Self::Trace),
            _ => None,
        }
    }
}

/// Parses a level filter as accepted by `KOEBI_LOG`: `off` gives `Some(None)`, a level name
/// gives `Some(Some(level))`, anything else `None`. Case-insensitive.
#[allow(
    clippy::option_option,
    reason = "outer None means unparsable, inner None means logging is off"
)]
pub fn parse_filter(text: &str) -> Option<Option<Level>> {
    match text.trim().to_ascii_lowercase().as_str() {
        "off" | "none" | "0" => Some(None),
        "error" => Some(Some(Level::Error)),
        "warn" | "warning" => Some(Some(Level::Warn)),
        "info" => Some(Some(Level::Info)),
        "debug" => Some(Some(Level::Debug)),
        "trace" => Some(Some(Level::Trace)),
        _ => None,
    }
}

/// A log record handed to the active [`Sink`].
#[derive(Debug)]
pub struct Record<'a> {
    level: Level,
    target: &'a str,
    args: fmt::Arguments<'a>,
    elapsed: Duration,
}

impl<'a> Record<'a> {
    /// Severity.
    pub fn level(&self) -> Level {
        self.level
    }

    /// Module path that emitted the record.
    pub fn target(&self) -> &'a str {
        self.target
    }

    /// The formatted message.
    pub fn args(&self) -> &fmt::Arguments<'a> {
        &self.args
    }

    /// Time since the first record of the process.
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

/// A function receiving every enabled record.
pub type Sink = fn(&Record<'_>);

/// Returned by [`set_sink`] when a sink is already installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SinkAlreadySet;

impl fmt::Display for SinkAlreadySet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a log sink is already installed")
    }
}

impl std::error::Error for SinkAlreadySet {}

const UNINITIALIZED: u8 = u8::MAX;
const OFF: u8 = 0;

static MAX_LEVEL: AtomicU8 = AtomicU8::new(UNINITIALIZED);
static SINK: OnceLock<Sink> = OnceLock::new();
static START: OnceLock<Instant> = OnceLock::new();

/// Returns the most verbose enabled level, or `None` when logging is off.
pub fn max_level() -> Option<Level> {
    let mut raw = MAX_LEVEL.load(Ordering::Relaxed);
    if raw == UNINITIALIZED {
        raw = std::env::var("KOEBI_LOG")
            .ok()
            .and_then(|value| parse_filter(&value))
            .map_or(Level::Info as u8, |filter| {
                filter.map_or(OFF, |level| level as u8)
            });
        MAX_LEVEL.store(raw, Ordering::Relaxed);
    }
    Level::from_u8(raw)
}

/// Sets the most verbose enabled level (`None` disables logging).
pub fn set_max_level(level: Option<Level>) {
    MAX_LEVEL.store(level.map_or(OFF, |level| level as u8), Ordering::Relaxed);
}

/// Returns `true` if records of `level` are currently emitted.
#[inline]
pub fn enabled(level: Level) -> bool {
    max_level().is_some_and(|max| level <= max)
}

/// Installs the process-wide sink. Call it once, before the first record.
///
/// # Errors
///
/// Returns [`SinkAlreadySet`] if a sink was installed before.
pub fn set_sink(sink: Sink) -> Result<(), SinkAlreadySet> {
    SINK.set(sink).map_err(|_| SinkAlreadySet)
}

/// Implementation detail of the logging macros.
#[doc(hidden)]
pub fn __private_log(level: Level, target: &str, args: fmt::Arguments<'_>) {
    let start = *START.get_or_init(Instant::now);
    let record = Record {
        level,
        target,
        args,
        elapsed: start.elapsed(),
    };
    let sink = SINK.get().copied().unwrap_or(platform_sink);
    sink(&record);
}

/// Formats a record the way the stderr sink prints it.
pub fn format_record(record: &Record<'_>) -> String {
    format!(
        "[{:>9.3}s {:<5} {}] {}",
        record.elapsed.as_secs_f64(),
        record.level.as_str(),
        record.target,
        record.args
    )
}

/// Writes records to stderr. Default sink on every platform except Android.
pub fn stderr_sink(record: &Record<'_>) {
    use std::io::Write as _;
    let line = format_record(record);
    // Logging must never fail the caller: a closed stderr is silently ignored.
    if writeln!(std::io::stderr().lock(), "{line}").is_err() {}
}

/// The platform's sink, used when no other is installed: stderr, logcat on Android. A sink
/// that keeps records (to show them in the interface) passes them on to it.
#[cfg(not(target_os = "android"))]
pub fn platform_sink(record: &Record<'_>) {
    stderr_sink(record);
}

/// The platform's sink, used when no other is installed: stderr, logcat on Android. A sink
/// that keeps records (to show them in the interface) passes them on to it.
#[cfg(target_os = "android")]
pub fn platform_sink(record: &Record<'_>) {
    android::logcat_sink(record);
}

#[cfg(target_os = "android")]
mod android {
    use std::ffi::{CString, c_char, c_int};

    #[link(name = "log")]
    unsafe extern "C" {
        fn __android_log_write(priority: c_int, tag: *const c_char, text: *const c_char) -> c_int;
    }

    /// Writes records to logcat under the `koebi` tag (stdout/stderr go nowhere on Android).
    pub(super) fn logcat_sink(record: &super::Record<'_>) {
        // android/log.h priorities: VERBOSE=2, DEBUG=3, INFO=4, WARN=5, ERROR=6.
        let priority = match record.level() {
            super::Level::Error => 6,
            super::Level::Warn => 5,
            super::Level::Info => 4,
            super::Level::Debug => 3,
            super::Level::Trace => 2,
        };
        let text = format!("[{}] {}", record.target(), record.args()).replace('\0', "\u{fffd}");
        let Ok(text) = CString::new(text) else { return };
        // SAFETY: both pointers are valid NUL-terminated strings for the duration of the call.
        unsafe {
            __android_log_write(priority, c"koebi".as_ptr(), text.as_ptr());
        }
    }
}

/// Logs at an explicit [`Level`]: `log_at!(Level::Info, "x = {x}")`.
#[macro_export]
macro_rules! log_at {
    ($level:expr, $($arg:tt)+) => {{
        let level = $level;
        if $crate::log::enabled(level) {
            $crate::log::__private_log(level, ::core::module_path!(), ::core::format_args!($($arg)+));
        }
    }};
}

/// Logs an error: an operation failed.
#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => { $crate::log_at!($crate::log::Level::Error, $($arg)+) };
}

/// Logs a warning: something unexpected happened and was recovered from.
#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => { $crate::log_at!($crate::log::Level::Warn, $($arg)+) };
}

/// Logs high-level progress.
#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => { $crate::log_at!($crate::log::Level::Info, $($arg)+) };
}

/// Logs developer details.
#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => { $crate::log_at!($crate::log::Level::Debug, $($arg)+) };
}

/// Logs very verbose, per-frame details.
#[macro_export]
macro_rules! trace {
    ($($arg:tt)+) => { $crate::log_at!($crate::log::Level::Trace, $($arg)+) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_ordered_by_verbosity() {
        assert!(Level::Error < Level::Warn);
        assert!(Level::Debug < Level::Trace);
        assert_eq!(Level::Warn.as_str(), "WARN");
    }

    #[test]
    fn parse_filter_accepts_names_and_off() {
        assert_eq!(parse_filter("off"), Some(None));
        assert_eq!(parse_filter(" Debug "), Some(Some(Level::Debug)));
        assert_eq!(parse_filter("warning"), Some(Some(Level::Warn)));
        assert_eq!(parse_filter("verbose"), None);
    }

    #[test]
    fn max_level_controls_enabled_levels() {
        set_max_level(Some(Level::Warn));
        assert!(enabled(Level::Error));
        assert!(enabled(Level::Warn));
        assert!(!enabled(Level::Info));
        set_max_level(None);
        assert!(!enabled(Level::Error));
        set_max_level(Some(Level::Info));
    }

    #[test]
    fn records_are_formatted_with_time_level_and_target() {
        let record = Record {
            level: Level::Info,
            target: "koebi_gpu::device",
            args: format_args!("adapter {}", "lavapipe"),
            elapsed: Duration::from_millis(1_500),
        };
        assert_eq!(
            format_record(&record),
            "[    1.500s INFO  koebi_gpu::device] adapter lavapipe"
        );
    }
}
