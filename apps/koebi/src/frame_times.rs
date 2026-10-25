//! [`FrameTimes`]: what each frame costs, and how many frames per second the application
//! could draw at that cost.
//!
//! A frame costs CPU time (building the interface and recording the GPU commands) and GPU
//! time (executing them), measured without the waits for the display, which the processors
//! overlap: the frame rate the application could reach is limited by the slower of the two,
//! whatever the screen's refresh rate.

use core::time::Duration;
use std::time::Instant;

/// Displayed values are averages over this long.
const WINDOW: Duration = Duration::from_millis(500);

/// Averages over a window of frames.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Averages {
    /// Mean CPU time per frame.
    pub(crate) cpu: Duration,
    /// Mean GPU time per frame, when the GPU measures it.
    pub(crate) gpu: Option<Duration>,
}

impl Averages {
    /// Frames per second at this cost: one frame per CPU or GPU frame time, whichever is
    /// longer.
    pub(crate) fn frames_per_second(&self) -> f64 {
        let bottleneck = self.gpu.map_or(self.cpu, |gpu| gpu.max(self.cpu));
        if bottleneck.is_zero() {
            0.0
        } else {
            1.0 / bottleneck.as_secs_f64()
        }
    }
}

/// Running sums of one window.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Sums {
    frames: u32,
    cpu: Duration,
    gpu_frames: u32,
    gpu: Duration,
}

impl Sums {
    fn add(&mut self, cpu: Duration, gpu: Option<Duration>) {
        self.frames += 1;
        self.cpu += cpu;
        if let Some(gpu) = gpu {
            self.gpu_frames += 1;
            self.gpu += gpu;
        }
    }

    fn averages(&self) -> Option<Averages> {
        (self.frames > 0).then(|| Averages {
            cpu: self.cpu / self.frames,
            gpu: (self.gpu_frames > 0).then(|| self.gpu / self.gpu_frames),
        })
    }
}

/// Frame times, averaged over half a second for display and over the whole run for the
/// summary logged at exit. Recording a frame costs a few additions.
#[derive(Clone, Debug)]
pub(crate) struct FrameTimes {
    window: Sums,
    window_start: Option<Instant>,
    run: Sums,
    shown: Option<Averages>,
    /// Incremented when the shown averages change, so that their text is formatted once.
    shown_revision: u64,
}

impl FrameTimes {
    pub(crate) fn new() -> Self {
        Self {
            window: Sums::default(),
            window_start: None,
            run: Sums::default(),
            shown: None,
            shown_revision: 0,
        }
    }

    /// Records a presented frame: its CPU time, and the latest GPU frame time measured.
    pub(crate) fn record(&mut self, cpu: Duration, gpu: Option<Duration>, now: Instant) {
        self.run.add(cpu, gpu);
        self.window.add(cpu, gpu);
        let start = *self.window_start.get_or_insert(now);
        if now.saturating_duration_since(start) >= WINDOW {
            self.shown = self.window.averages();
            self.shown_revision += 1;
            self.window = Sums::default();
            self.window_start = Some(now);
        }
    }

    /// The averages of the last complete window, `None` before the first one ends.
    pub(crate) fn shown(&self) -> Option<Averages> {
        self.shown
    }

    /// Changes whenever [`Self::shown`] does.
    pub(crate) fn shown_revision(&self) -> u64 {
        self.shown_revision
    }

    /// The averages of the whole run, for the summary logged at exit.
    pub(crate) fn run(&self) -> Option<Averages> {
        self.run.averages()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn the_slower_processor_sets_the_frame_rate() {
        let gpu_bound = Averages {
            cpu: ms(1),
            gpu: Some(ms(4)),
        };
        assert!((gpu_bound.frames_per_second() - 250.0).abs() < 1e-9);
        let cpu_only = Averages {
            cpu: ms(2),
            gpu: None,
        };
        assert!((cpu_only.frames_per_second() - 500.0).abs() < 1e-9);
        assert!(Averages::default().frames_per_second().abs() < f64::EPSILON);
    }

    #[test]
    fn averages_refresh_every_half_second() {
        let mut times = FrameTimes::new();
        let start = Instant::now();
        times.record(ms(2), None, start);
        times.record(ms(4), Some(ms(1)), start + ms(100));
        assert_eq!(times.shown(), None);
        times.record(ms(6), Some(ms(3)), start + ms(500));
        let shown = times.shown().expect("a window ended");
        assert_eq!(shown.cpu, ms(4));
        assert_eq!(shown.gpu, Some(ms(2)));
        assert_eq!(times.shown_revision(), 1);
        // The run keeps everything; the next window starts empty.
        times.record(ms(10), None, start + ms(600));
        assert_eq!(times.shown_revision(), 1);
        assert_eq!(times.run().expect("frames recorded").cpu, ms(22) / 4);
    }
}
