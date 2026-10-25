//! End-to-end tests: launch the real `koebi` binary.
//!
//! Desktop only: on Android and iOS the binary is packaged (APK, app bundle) and exercised by
//! the smoke tests of the CI mobile jobs instead.
#![cfg(not(any(target_os = "android", target_os = "ios")))]
//!
//! The main test opens a real window, presents frames through the swapchain and checks the
//! exit code, which is non-zero if anything failed, including Vulkan validation errors.
//! Without a display or a Vulkan driver the binary exits with `UNSUPPORTED`; the test then
//! skips, unless `KOEBI_REQUIRE_GPU=1` (CI) makes it fail.

use std::io::Read as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use koebi::exit_code;

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run_koebi(args: &[&str], timeout: Duration) -> Run {
    let mut child = Command::new(env!("CARGO_BIN_EXE_koebi"))
        .args(args)
        .env("KOEBI_LOG", "info")
        .env(
            "KOEBI_VALIDATION",
            std::env::var("KOEBI_VALIDATION").unwrap_or_else(|_| "1".into()),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to launch koebi");
    // Drain both pipes concurrently: a child blocked on a full pipe would never exit.
    let mut out_pipe = child.stdout.take().expect("stdout is piped");
    let mut err_pipe = child.stderr.take().expect("stderr is piped");
    let out_reader = std::thread::spawn(move || {
        let mut text = String::new();
        out_pipe.read_to_string(&mut text).map(|_| text)
    });
    let err_reader = std::thread::spawn(move || {
        let mut text = String::new();
        err_pipe.read_to_string(&mut text).map(|_| text)
    });

    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("cannot poll koebi") {
            break Some(status);
        }
        if started.elapsed() > timeout {
            // An error means the process exited in the meantime, which is fine.
            drop(child.kill());
            break None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let stdout = out_reader
        .join()
        .expect("stdout reader")
        .expect("read stdout");
    let stderr = err_reader
        .join()
        .expect("stderr reader")
        .expect("read stderr");
    assert!(
        status.is_some(),
        "koebi did not exit within {timeout:?}\nstderr:\n{stderr}"
    );
    Run {
        code: status.and_then(|s| s.code()),
        stdout,
        stderr,
    }
}

#[test]
fn renders_frames_in_a_window_and_exits_cleanly() {
    let run = run_koebi(&["--frames", "60"], Duration::from_secs(120));
    if run.code == Some(exit_code::UNSUPPORTED) && !koebi_testkit::require_gpu() {
        eprintln!("SKIPPED: no display or Vulkan driver\n{}", run.stderr);
        return;
    }
    assert_eq!(
        run.code,
        Some(exit_code::SUCCESS),
        "koebi failed:\n{}",
        run.stderr
    );
    assert!(
        run.stderr.contains("presented 60 frames"),
        "unexpected log:\n{}",
        run.stderr
    );
    assert!(
        run.stderr.contains("validation: 0 errors"),
        "validation errors:\n{}",
        run.stderr
    );
}

#[test]
fn help_prints_usage_and_succeeds() {
    let run = run_koebi(&["--help"], Duration::from_secs(60));
    if run.code == Some(exit_code::UNSUPPORTED) && !koebi_testkit::require_gpu() {
        eprintln!("SKIPPED: SDL video unavailable\n{}", run.stderr);
        return;
    }
    assert_eq!(run.code, Some(exit_code::SUCCESS), "{}", run.stderr);
    assert!(
        run.stdout.contains("--frames"),
        "usage missing:\n{}",
        run.stdout
    );
}

#[test]
fn invalid_arguments_exit_with_usage_code() {
    let run = run_koebi(&["--frames", "zero"], Duration::from_secs(60));
    if run.code == Some(exit_code::UNSUPPORTED) && !koebi_testkit::require_gpu() {
        eprintln!("SKIPPED: SDL video unavailable\n{}", run.stderr);
        return;
    }
    assert_eq!(run.code, Some(exit_code::USAGE), "{}", run.stderr);
    assert!(run.stderr.contains("invalid value"), "{}", run.stderr);
}

/// An empty directory for the design state of one test.
fn data_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    // Left over by a previous run, or absent: either way the test starts empty.
    std::fs::remove_dir_all(&dir).ok();
    dir
}

#[test]
fn design_changes_survive_a_restart() {
    let dir = data_dir("e2e-design-state");
    let dir_arg = dir.to_str().expect("the target directory has a UTF-8 path");
    let first = run_koebi(
        &[
            "--frames",
            "3",
            "--data-dir",
            dir_arg,
            "--theme",
            "light",
            "--override",
            "semantic.radius.control=7",
            "--override",
            "component.button.background.color.default=#ff8800",
            "--color-vision",
            "simulate-deutan",
        ],
        Duration::from_secs(120),
    );
    if first.code == Some(exit_code::UNSUPPORTED) && !koebi_testkit::require_gpu() {
        eprintln!(
            "SKIPPED: no display or Vulkan driver
{}",
            first.stderr
        );
        return;
    }
    assert_eq!(first.code, Some(exit_code::SUCCESS), "{}", first.stderr);
    assert!(first.stderr.contains("design saved to"), "{}", first.stderr);
    assert!(dir.join("design.kds").is_file());

    // A plain restart restores everything, and has nothing new to save.
    let second = run_koebi(
        &["--frames", "3", "--data-dir", dir_arg],
        Duration::from_secs(120),
    );
    assert_eq!(second.code, Some(exit_code::SUCCESS), "{}", second.stderr);
    let expected = concat!(
        "design: theme light, 2 global override(s), 0 theme override(s), ",
        "color vision simulate-deutan"
    );
    assert!(
        second.stderr.contains("design restored from"),
        "{}",
        second.stderr
    );
    assert!(second.stderr.contains(expected), "{}", second.stderr);
    assert!(
        !second.stderr.contains("design saved to"),
        "{}",
        second.stderr
    );
}

#[test]
fn refused_design_options_exit_with_usage_code_and_save_nothing() {
    let dir = data_dir("e2e-design-refused");
    let dir_arg = dir.to_str().expect("the target directory has a UTF-8 path");
    let run = run_koebi(
        &[
            "--frames",
            "1",
            "--data-dir",
            dir_arg,
            "--override",
            "semantic.radius.control=wide",
        ],
        Duration::from_secs(60),
    );
    if run.code == Some(exit_code::UNSUPPORTED) && !koebi_testkit::require_gpu() {
        eprintln!(
            "SKIPPED: SDL video unavailable
{}",
            run.stderr
        );
        return;
    }
    assert_eq!(run.code, Some(exit_code::USAGE), "{}", run.stderr);
    assert!(
        run.stderr
            .contains("--override semantic.radius.control=wide"),
        "{}",
        run.stderr
    );
    assert!(!dir.join("design.kds").exists());
}
