//! A scope profiler for development: how long each named part of a frame takes, on average and
//! at worst, without a dependency and at almost no cost while off.
//!
//! [`scope`] (or the [`profile_scope!`](crate::profile_scope) macro) times the code until the
//! guard it returns drops; [`end_frame`] closes a frame. Timings accumulate per thread and per
//! name: [`report`] lists them by time per frame, the slowest first. Scopes nest: each one's time
//! includes its children's.
//!
//! Profiling is off unless [`set_enabled`] turns it on (the application does with
//! `KOEBI_PROFILE=1`); while off, a scope costs one relaxed atomic load.

use core::cell::RefCell;
use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration;
use std::time::Instant;

static ENABLED: AtomicBool = AtomicBool::new(false);

/// Most distinct scope names a thread records; past them, scopes are not recorded.
const MAX_SCOPES: usize = 128;

/// What one scope name took over the frames recorded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopeStats {
    /// The scope's name.
    pub name: &'static str,
    /// Time over every frame.
    pub total: Duration,
    /// How many times the scope ran.
    pub calls: u64,
    /// The longest single frame of it: its time summed within one frame.
    pub worst_frame: Duration,
    /// Its time in the frame being recorded.
    frame: Duration,
}

#[derive(Debug)]
struct Recorder {
    scopes: Vec<ScopeStats>,
    frames: u64,
}

std::thread_local! {
    #[allow(
        clippy::missing_const_for_thread_local,
        reason = "the initializer is const: clippy misreads the macro's expansion on Android"
    )]
    static RECORDER: RefCell<Recorder> = const {
        RefCell::new(Recorder {
            scopes: Vec::new(),
            frames: 0,
        })
    };
}

/// Turns profiling on or off for every thread.
pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

/// Whether profiling is on.
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Times the code until the guard drops, under `name`; nothing while profiling is off.
#[must_use = "the scope ends when the guard drops"]
pub fn scope(name: &'static str) -> Scope {
    Scope {
        name,
        start: enabled().then(Instant::now),
    }
}

/// A scope being timed ([`scope`]).
#[derive(Debug)]
pub struct Scope {
    name: &'static str,
    start: Option<Instant>,
}

impl Drop for Scope {
    fn drop(&mut self) {
        let Some(start) = self.start else {
            return;
        };
        let elapsed = start.elapsed();
        RECORDER.with(|recorder| {
            let mut recorder = recorder.borrow_mut();
            let at = match recorder.scopes.iter().position(|s| s.name == self.name) {
                Some(at) => at,
                None if recorder.scopes.len() < MAX_SCOPES => {
                    // The first time a name runs: allocated once per name.
                    recorder.scopes.push(ScopeStats {
                        name: self.name,
                        total: Duration::ZERO,
                        calls: 0,
                        worst_frame: Duration::ZERO,
                        frame: Duration::ZERO,
                    });
                    recorder.scopes.len() - 1
                }
                None => return,
            };
            let stats = &mut recorder.scopes[at];
            stats.total += elapsed;
            stats.calls += 1;
            stats.frame += elapsed;
        });
    }
}

/// Closes the frame of this thread: each scope's time in it counts toward its worst frame.
pub fn end_frame() {
    if !enabled() {
        return;
    }
    RECORDER.with(|recorder| {
        let mut recorder = recorder.borrow_mut();
        recorder.frames += 1;
        for stats in &mut recorder.scopes {
            stats.worst_frame = stats.worst_frame.max(stats.frame);
            stats.frame = Duration::ZERO;
        }
    });
}

/// The scopes this thread recorded, the most time per frame first, and how many frames.
pub fn report() -> (Vec<ScopeStats>, u64) {
    RECORDER.with(|recorder| {
        let recorder = recorder.borrow();
        let mut scopes = recorder.scopes.clone();
        scopes.sort_by_key(|stats| core::cmp::Reverse(stats.total));
        (scopes, recorder.frames)
    })
}

/// The report as text: one line per scope, its time per frame, per call and at worst.
pub fn report_text() -> String {
    use core::fmt::Write as _;
    let (scopes, frames) = report();
    let frames_f = frames.max(1) as f64;
    let mut text = format!("{frames} frames\n");
    let _ = writeln!(
        text,
        "{:<34}{:>12}{:>10}{:>12}{:>12}",
        "scope", "ms/frame", "calls/f", "µs/call", "worst ms"
    );
    for stats in scopes {
        let total = stats.total.as_secs_f64() * 1e3;
        let calls = stats.calls.max(1) as f64;
        let _ = writeln!(
            text,
            "{:<34}{:>12.3}{:>10.1}{:>12.2}{:>12.3}",
            stats.name,
            total / frames_f,
            calls / frames_f,
            total * 1e3 / calls,
            stats.worst_frame.as_secs_f64() * 1e3
        );
    }
    text
}

/// Times the rest of the enclosing block under a name: `profile_scope!("ui.build");`.
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        let _profile_scope = $crate::profile::scope($name);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scopes_add_up_per_frame_and_keep_the_worst() {
        set_enabled(true);
        for pause in [1_u64, 3] {
            {
                profile_scope!("test.outer");
                std::thread::sleep(Duration::from_millis(pause));
            }
            end_frame();
        }
        let (scopes, frames) = report();
        let outer = scopes.iter().find(|s| s.name == "test.outer").unwrap();
        assert_eq!((outer.calls, frames), (2, 2));
        assert!(outer.worst_frame >= Duration::from_millis(3));
        assert!(outer.total >= Duration::from_millis(4));
        assert!(report_text().contains("test.outer"));
        set_enabled(false);
    }
}
