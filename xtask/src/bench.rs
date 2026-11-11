//! `cargo xtask bench [--save]`: the performance baseline (issue #30).
//!
//! Runs the application's benchmark in release (`apps/koebi/src/bench.rs`): the scenes of real
//! screens built on the CPU, text laid out, frames rendered offscreen. Then it measures the
//! start-up of the release binary to its first frame. The report goes to
//! `target/bench/latest.md`, and with `--save` to `docs/performance/baseline.md`, the baseline
//! later changes are compared with on the same machine.

use std::fmt::Write as _;
use std::fs;
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::util::{self, Result};

/// Start-ups measured.
const STARTS: usize = 5;

pub(crate) fn run(save: bool) -> Result<()> {
    util::step("bench: scenes, text and offscreen frames (release)");
    let output = util::cargo()
        .args([
            "test",
            "--release",
            "-p",
            "koebi",
            "--lib",
            "bench::baseline",
            "--",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("KOEBI_LOG", "info")
        .output()
        .map_err(|error| format!("cannot run the benchmark: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!("the benchmark failed:\n{stdout}\n{stderr}").into());
    }
    let table: Vec<&str> = stdout
        .lines()
        // libtest writes the test's name on the line where its output starts.
        .skip_while(|line| !line.contains("BENCH-BEGIN"))
        .skip(1)
        .take_while(|line| !line.contains("BENCH-END"))
        .collect();
    if table.is_empty() {
        return Err(format!("the benchmark printed no table:\n{stdout}").into());
    }
    let gpu = stderr
        .lines()
        .find_map(|line| line.split_once("GPU device: ").map(|(_, rest)| rest.trim()))
        .unwrap_or("none (GPU frames skipped)")
        .to_owned();

    util::step("bench: start-up to the first frame (release)");
    util::run(util::cargo().args(["build", "--release", "-p", "koebi"]))?;
    let startup = startup()?;

    let mut report = String::new();
    let _ = writeln!(report, "# Performance baseline\n");
    let _ = writeln!(
        report,
        "Taken with `cargo xtask bench` on {} ({} {}), CPU {}, GPU {gpu}.\n",
        today(),
        std::env::consts::OS,
        std::env::consts::ARCH,
        cpu()
    );
    let _ = writeln!(
        report,
        "Wall times, in microseconds, of release builds. Compare them on one machine only. \
         *CPU scene* builds the scene of a screen, as each frame does (warm, 200 frames). \
         *Text layout* lays a sample paragraph out at 600 px. *Offscreen frame* records the \
         frame, waits for the GPU and reads the image back (40 frames, after the two that \
         store glyphs and icons). See issue #30.\n"
    );
    for line in &table {
        let _ = writeln!(report, "{line}");
    }
    let _ = writeln!(report, "\n## Start-up\n");
    match startup {
        Some(times) => {
            let _ = writeln!(
                report,
                "`koebi --frames 1` to its exit, {STARTS} runs: median {} ms, fastest {} ms \
                 (window, Vulkan, pipelines, fonts and the first frame).",
                times.0.as_millis(),
                times.1.as_millis()
            );
        }
        None => {
            let _ = writeln!(
                report,
                "Not measured: the binary could not open a window here."
            );
        }
    }
    println!("\n{report}");
    let dir = util::workspace_root().join("target").join("bench");
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("latest.md"), &report)?;
    if save {
        let docs = util::workspace_root().join("docs").join("performance");
        fs::create_dir_all(&docs)?;
        fs::write(docs.join("baseline.md"), &report)?;
        println!("saved to docs/performance/baseline.md");
    }
    Ok(())
}

/// The median and fastest start-up of the release binary, `None` when it cannot start.
fn startup() -> Result<Option<(Duration, Duration)>> {
    let root = util::workspace_root();
    let binary = root
        .join("target")
        .join("release")
        .join(format!("koebi{}", std::env::consts::EXE_SUFFIX));
    let data = root.join("target").join("bench").join("data");
    let mut times = Vec::with_capacity(STARTS);
    for _ in 0..STARTS {
        let start = Instant::now();
        let status = Command::new(&binary)
            .args(["--frames", "1", "--data-dir"])
            .arg(&data)
            .output()
            .map_err(|error| format!("cannot start {}: {error}", binary.display()))?
            .status;
        if !status.success() {
            return Ok(None);
        }
        times.push(start.elapsed());
    }
    times.sort_unstable();
    Ok(Some((times[STARTS / 2], times[0])))
}

/// The processor, as the system names it, where it says.
fn cpu() -> String {
    if let Ok(name) = std::env::var("PROCESSOR_IDENTIFIER") {
        return name;
    }
    fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|info| {
            info.lines()
                .find_map(|line| line.strip_prefix("model name"))
                .map(|rest| rest.trim_start_matches([' ', '\t', ':']).to_owned())
        })
        .or_else(|| {
            util::output(Command::new("sysctl").args(["-n", "machdep.cpu.brand_string"]))
                .ok()
                .map(|name| name.trim().to_owned())
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

/// Today's date, `YYYY-MM-DD`, in UTC.
fn today() -> String {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() / 86_400);
    let (year, month, day) = civil_from_days(i64::try_from(days).unwrap_or(0));
    format!("{year:04}-{month:02}-{day:02}")
}

/// The calendar date of `days` since 1970-01-01 (Howard Hinnant's `civil_from_days`).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_counted_from_the_epoch() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
        assert_eq!(civil_from_days(20_768), (2026, 11, 11));
    }
}
