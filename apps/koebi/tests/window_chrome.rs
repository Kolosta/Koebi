//! The custom title bar against the real window manager of Windows.
//!
//! These tests launch the real binary and ask Windows what its window is: `WM_NCHITTEST` answers
//! what the system would do with a press at a point, `WM_SYSCOMMAND` is what the system's own
//! maximize, minimize and restore send. No input is simulated: moving a window from the caption,
//! snapping it and maximizing it by double click are the system's own behaviours of `HTCAPTION`
//! and of the window styles checked here.
#![cfg(windows)]

use std::ffi::c_void;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use koebi::exit_code;

type Hwnd = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct WinRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct WinPoint {
    x: i32,
    y: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct MonitorInfo {
    size: u32,
    monitor: WinRect,
    work: WinRect,
    flags: u32,
}

/// The size Windows reads the structure by: its size, two rectangles and its flags.
const MONITOR_INFO_SIZE: u32 = 40;
const _: () = assert!(size_of::<MonitorInfo>() == MONITOR_INFO_SIZE as usize);

type EnumProc = unsafe extern "system" fn(Hwnd, isize) -> i32;

#[link(name = "user32")]
unsafe extern "system" {
    fn SetProcessDPIAware() -> i32;
    fn EnumWindows(callback: EnumProc, parameter: isize) -> i32;
    fn GetWindowThreadProcessId(window: Hwnd, process: *mut u32) -> u32;
    fn IsWindowVisible(window: Hwnd) -> i32;
    fn GetWindow(window: Hwnd, command: u32) -> Hwnd;
    fn IsZoomed(window: Hwnd) -> i32;
    fn IsIconic(window: Hwnd) -> i32;
    fn GetClientRect(window: Hwnd, rect: *mut WinRect) -> i32;
    fn ClientToScreen(window: Hwnd, point: *mut WinPoint) -> i32;
    fn GetWindowLongW(window: Hwnd, index: i32) -> i32;
    fn MonitorFromWindow(window: Hwnd, flags: u32) -> *mut c_void;
    fn GetMonitorInfoW(monitor: *mut c_void, info: *mut MonitorInfo) -> i32;
    fn PostMessageW(window: Hwnd, message: u32, wparam: usize, lparam: isize) -> i32;
    fn SendMessageTimeoutW(
        window: Hwnd,
        message: u32,
        wparam: usize,
        lparam: isize,
        flags: u32,
        timeout: u32,
        result: *mut isize,
    ) -> isize;
}

const WM_CLOSE: u32 = 0x0010;
const WM_NCHITTEST: u32 = 0x0084;
const WM_SYSCOMMAND: u32 = 0x0112;
const SC_MINIMIZE: usize = 0xF020;
const SC_MAXIMIZE: usize = 0xF030;
const SC_RESTORE: usize = 0xF120;
const SMTO_ABORTIFHUNG: u32 = 2;
const MONITOR_DEFAULTTONEAREST: u32 = 2;
const GW_OWNER: u32 = 4;
const GWL_STYLE: i32 = -16;
const WS_THICKFRAME: i32 = 0x0004_0000;
const WS_MAXIMIZEBOX: i32 = 0x0001_0000;
const WS_MINIMIZEBOX: i32 = 0x0002_0000;
const WS_CAPTION: i32 = 0x00C0_0000;
const WS_SYSMENU: i32 = 0x0008_0000;

const HTCLIENT: isize = 1;
const HTCAPTION: isize = 2;
const HTLEFT: isize = 10;
const HTRIGHT: isize = 11;
const HTTOP: isize = 12;
const HTTOPLEFT: isize = 13;
const HTTOPRIGHT: isize = 14;
const HTBOTTOM: isize = 15;
const HTBOTTOMLEFT: isize = 16;
const HTBOTTOMRIGHT: isize = 17;

/// Long enough for a software GPU to start, and for a window manager to answer.
const WAIT: Duration = Duration::from_secs(60);

/// The running application, killed if a test ends early.
struct App {
    child: Child,
    window: Hwnd,
    data: PathBuf,
}

impl Drop for App {
    fn drop(&mut self) {
        drop(self.child.kill());
        drop(self.child.wait());
        drop(std::fs::remove_dir_all(&self.data));
    }
}

struct Search {
    process: u32,
    found: Vec<Hwnd>,
}

unsafe extern "system" fn look_at(window: Hwnd, parameter: isize) -> i32 {
    let search = parameter as *mut Search;
    let mut process = 0;
    // SAFETY: `window` comes from `EnumWindows`; the out-pointer references a local; `parameter`
    // is the `Search` of `find_window`, alive during the enumeration.
    unsafe {
        GetWindowThreadProcessId(window, &raw mut process);
        if process == (*search).process && IsWindowVisible(window) != 0 {
            (*search).found.push(window);
        }
    }
    1
}

/// The visible windows of `process`.
fn find_windows(process: u32) -> Vec<Hwnd> {
    let mut search = Search {
        process,
        found: Vec::new(),
    };
    // SAFETY: the callback matches `WNDENUMPROC`, and `search` outlives the call.
    unsafe { EnumWindows(look_at, (&raw mut search) as isize) };
    search.found
}

/// Launches Koebi with `args`; `None` when the machine cannot run it (no GPU, outside CI).
fn launch(name: &str, args: &[&str]) -> Option<App> {
    // SAFETY: no precondition; coordinates are physical pixels from here on.
    unsafe { SetProcessDPIAware() };
    let data = std::env::temp_dir().join(format!("koebi-chrome-{name}-{}", std::process::id()));
    let child = Command::new(env!("CARGO_BIN_EXE_koebi"))
        .args(args)
        .arg("--data-dir")
        .arg(&data)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to launch koebi");
    let mut app = App {
        child,
        window: core::ptr::null_mut(),
        data,
    };
    let started = Instant::now();
    loop {
        if let Some(&window) = find_windows(app.child.id()).last() {
            app.window = window;
            app.wait_until("the bar to answer", |app| {
                // The first frame gives the system the regions.
                app.hit(app.rect().width() / 2, 10) != HTCLIENT || app.state_has_no_drag(args)
            });
            return Some(app);
        }
        if let Some(status) = app.child.try_wait().expect("cannot poll koebi") {
            let code = status.code().unwrap_or(-1);
            assert!(
                code == exit_code::UNSUPPORTED && std::env::var_os("KOEBI_REQUIRE_GPU").is_none(),
                "koebi exited with {code} before showing its window"
            );
            return None;
        }
        assert!(started.elapsed() < WAIT, "no window after {WAIT:?}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

impl WinRect {
    fn width(self) -> i32 {
        self.right - self.left
    }

    fn height(self) -> i32 {
        self.bottom - self.top
    }

    fn size(self) -> (i32, i32) {
        (self.width(), self.height())
    }
}

impl App {
    fn state_has_no_drag(&self, args: &[&str]) -> bool {
        args.contains(&"--fullscreen") && self.rect() == self.monitor().monitor
    }

    /// What the window shows, on the screen. A maximized window has an invisible frame around
    /// it, past the work area: the application draws inside it.
    fn rect(&self) -> WinRect {
        Self::rect_of(self.window)
    }

    fn rect_of(window: Hwnd) -> WinRect {
        let mut rect = WinRect::default();
        let mut origin = WinPoint::default();
        // SAFETY: a live window; the out-pointers reference locals.
        unsafe {
            GetClientRect(window, &raw mut rect);
            ClientToScreen(window, &raw mut origin);
        }
        WinRect {
            left: origin.x,
            top: origin.y,
            right: origin.x + rect.right,
            bottom: origin.y + rect.bottom,
        }
    }

    fn monitor(&self) -> MonitorInfo {
        let mut info = MonitorInfo {
            size: MONITOR_INFO_SIZE,
            ..MonitorInfo::default()
        };
        // SAFETY: a live window; `info` has its size set and outlives the call.
        unsafe {
            let monitor = MonitorFromWindow(self.window, MONITOR_DEFAULTTONEAREST);
            GetMonitorInfoW(monitor, &raw mut info);
        }
        info
    }

    /// What the system would do with a press at `(x, y)` from the window's corner.
    fn hit(&self, x: i32, y: i32) -> isize {
        let rect = self.rect();
        let (x, y) = (rect.left + x, rect.top + y);
        let lparam = ((y & 0xFFFF) << 16 | (x & 0xFFFF)) as isize;
        let mut result = 0;
        // SAFETY: a live window; WM_NCHITTEST carries no pointer; the out-pointer references a
        // local.
        let answered = unsafe {
            SendMessageTimeoutW(
                self.window,
                WM_NCHITTEST,
                0,
                lparam,
                SMTO_ABORTIFHUNG,
                10_000,
                &raw mut result,
            )
        };
        assert_ne!(answered, 0, "the window does not answer");
        result
    }

    /// Sends what the system's own window menu sends.
    fn command(&self, command: usize) {
        // SAFETY: a live window; the message carries no pointer.
        let posted = unsafe { PostMessageW(self.window, WM_SYSCOMMAND, command, 0) };
        assert_ne!(posted, 0);
    }

    /// What the window is, for a test that fails.
    fn describe(&self) -> String {
        let monitor = self.monitor();
        format!(
            "rect {:?}, maximized {}, minimized {}, work area {:?}, monitor {:?}",
            self.rect(),
            self.is_maximized(),
            self.is_minimized(),
            monitor.work,
            monitor.monitor
        )
    }

    fn is_maximized(&self) -> bool {
        // SAFETY: a live window.
        unsafe { IsZoomed(self.window) != 0 }
    }

    fn is_minimized(&self) -> bool {
        // SAFETY: a live window.
        unsafe { IsIconic(self.window) != 0 }
    }

    fn style(&self) -> i32 {
        // SAFETY: a live window.
        unsafe { GetWindowLongW(self.window, GWL_STYLE) }
    }

    #[track_caller]
    fn wait_until(&self, what: &str, done: impl Fn(&Self) -> bool) {
        let started = Instant::now();
        while !done(self) {
            assert!(
                started.elapsed() < WAIT,
                "waited too long for {what}: {}",
                self.describe()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

#[test]
fn every_part_of_the_window_answers_as_it_should() {
    let Some(app) = launch("parts", &[]) else {
        return;
    };
    let rect = app.rect();
    let (width, height) = (rect.width(), rect.height());
    // The bar, a few pixels down from the top edge, moves the window.
    let bar = 10;
    assert_eq!(app.hit(width / 2, bar), HTCAPTION);
    assert_eq!(app.hit(width / 3, bar), HTCAPTION);
    // Its menus and its window buttons are the application's.
    assert_eq!(app.hit(20, bar), HTCLIENT);
    assert_eq!(app.hit(width - 20, bar), HTCLIENT);
    assert_eq!(app.hit(width - 60, bar), HTCLIENT);
    assert_eq!(app.hit(width - 100, bar), HTCLIENT);
    // Below it too.
    assert_eq!(app.hit(width / 2, height / 2), HTCLIENT);
    // Edges and corners resize.
    assert_eq!(app.hit(width / 2, 0), HTTOP);
    assert_eq!(app.hit(width / 2, height - 1), HTBOTTOM);
    assert_eq!(app.hit(0, height / 2), HTLEFT);
    assert_eq!(app.hit(width - 1, height / 2), HTRIGHT);
    assert_eq!(app.hit(0, 0), HTTOPLEFT);
    assert_eq!(app.hit(width - 1, 0), HTTOPRIGHT);
    assert_eq!(app.hit(0, height - 1), HTBOTTOMLEFT);
    assert_eq!(app.hit(width - 1, height - 1), HTBOTTOMRIGHT);
}

#[test]
fn the_window_has_the_styles_the_system_behaviours_need() {
    let Some(app) = launch("styles", &[]) else {
        return;
    };
    // Snapping and the drag to the top need a thick frame and a maximize box; the animations and
    // the system menu a caption and a system menu, even if nothing of them is drawn.
    let style = app.style();
    for needed in [
        WS_THICKFRAME,
        WS_MAXIMIZEBOX,
        WS_MINIMIZEBOX,
        WS_CAPTION,
        WS_SYSMENU,
    ] {
        assert_eq!(style & needed, needed, "style {style:#x} lacks {needed:#x}");
    }
}

#[test]
fn maximizing_fills_the_work_area_and_restoring_brings_the_size_back() {
    let Some(app) = launch("maximize", &[]) else {
        return;
    };
    let before = app.rect();
    app.command(SC_MAXIMIZE);
    app.wait_until("the window to maximize", |app| {
        app.is_maximized() && app.rect() == app.monitor().work
    });
    // Filling the work area, the taskbar stays visible; the window has no edges any more, and it
    // still moves from its bar, the very top included.
    let rect = app.rect();
    app.wait_until("the regions to follow", |app| {
        app.hit(rect.width() / 2, 0) == HTCAPTION
    });
    assert_eq!(app.hit(0, rect.height() / 2), HTCLIENT);
    assert_eq!(app.hit(rect.width() - 1, rect.height() / 2), HTCLIENT);
    assert_eq!(app.hit(rect.width() / 2, rect.height() - 1), HTCLIENT);
    app.command(SC_RESTORE);
    app.wait_until("the window to come back", |app| {
        !app.is_maximized() && app.rect().size() == before.size()
    });
}

#[test]
fn minimizing_and_restoring_keep_the_window() {
    let Some(app) = launch("minimize", &[]) else {
        return;
    };
    let before = app.rect();
    app.command(SC_MINIMIZE);
    app.wait_until("the window to minimize", App::is_minimized);
    app.command(SC_RESTORE);
    app.wait_until("the window to come back", |app| {
        !app.is_minimized() && app.rect().size() == before.size()
    });
}

#[test]
fn a_window_opening_maximized_fills_the_work_area() {
    let Some(app) = launch("open-maximized", &["--maximized"]) else {
        return;
    };
    app.wait_until("the window to maximize", |app| {
        app.is_maximized() && app.rect() == app.monitor().work
    });
}

#[test]
fn a_fullscreen_window_covers_its_display_and_cannot_be_moved() {
    let Some(app) = launch("fullscreen", &["--fullscreen"]) else {
        return;
    };
    app.wait_until("the window to cover its display", |app| {
        app.rect() == app.monitor().monitor
    });
    let rect = app.rect();
    // The bar stays, as the application's: no caption, and no edge.
    for (x, y) in [
        (rect.width() / 2, 10),
        (rect.width() / 2, 0),
        (0, 0),
        (0, rect.height() / 2),
        (rect.width() - 1, rect.height() - 1),
    ] {
        assert_eq!(app.hit(x, y), HTCLIENT, "at ({x}, {y})");
    }
}

#[test]
fn the_system_closing_the_window_quits_cleanly() {
    let Some(mut app) = launch("close", &[]) else {
        return;
    };
    // SAFETY: a live window; the message carries no pointer.
    let posted = unsafe { PostMessageW(app.window, WM_CLOSE, 0, 0) };
    assert_ne!(posted, 0);
    let started = Instant::now();
    loop {
        if let Some(status) = app.child.try_wait().expect("cannot poll koebi") {
            assert_eq!(status.code(), Some(exit_code::SUCCESS));
            return;
        }
        assert!(started.elapsed() < WAIT, "koebi did not quit");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The application with a floating window beside its main one: the floating one is smaller.
fn with_floating(name: &str) -> Option<(App, App)> {
    let mut main = launch(name, &["--open-window", "floating"])?;
    main.wait_until("both windows", |app| {
        find_windows(app.child.id()).len() == 2
    });
    let windows = find_windows(main.child.id());
    let width = |window: Hwnd| App::rect_of(window).width();
    let (large, small) = if width(windows[0]) > width(windows[1]) {
        (windows[0], windows[1])
    } else {
        (windows[1], windows[0])
    };
    main.window = large;
    let floating = App {
        child: Command::new(env!("CARGO_BIN_EXE_koebi"))
            .arg("--version")
            .stdout(Stdio::null())
            .spawn()
            .expect("failed to launch koebi"),
        window: small,
        data: std::env::temp_dir().join(format!("koebi-chrome-none-{}", std::process::id())),
    };
    Some((main, floating))
}

#[test]
fn a_floating_window_is_on_its_own_and_of_the_same_application() {
    let Some((main, floating)) = with_floating("floating") else {
        return;
    };
    // No owner: the system neither minimizes it with the main window nor keeps it above it. Two
    // windows of one process share their icon on the taskbar.
    for window in [main.window, floating.window] {
        // SAFETY: a live window.
        assert!(unsafe { GetWindow(window, GW_OWNER) }.is_null());
    }
    // It has the same bar, with the same behaviours.
    floating.wait_until("its bar to answer", |app| {
        app.hit(app.rect().width() / 2, 10) == HTCAPTION
    });
    assert_eq!(floating.hit(0, 100), HTLEFT);
    // Minimizing the main window leaves it there.
    main.command(SC_MINIMIZE);
    main.wait_until("the main window to minimize", App::is_minimized);
    std::thread::sleep(Duration::from_millis(300));
    assert!(!floating.is_minimized());
    main.command(SC_RESTORE);
    main.wait_until("the main window to come back", |app| !app.is_minimized());
    // And the other way round.
    floating.command(SC_MINIMIZE);
    floating.wait_until("the floating window to minimize", App::is_minimized);
    assert!(!main.is_minimized());
}

#[test]
fn closing_a_floating_window_leaves_the_application_and_closing_the_main_one_quits() {
    let Some((mut main, floating)) = with_floating("closing") else {
        return;
    };
    // SAFETY: a live window; the message carries no pointer.
    assert_ne!(unsafe { PostMessageW(floating.window, WM_CLOSE, 0, 0) }, 0);
    main.wait_until("the floating window to go", |app| {
        find_windows(app.child.id()).len() == 1
    });
    assert!(main.child.try_wait().expect("cannot poll koebi").is_none());
    // SAFETY: a live window; the message carries no pointer.
    assert_ne!(unsafe { PostMessageW(main.window, WM_CLOSE, 0, 0) }, 0);
    let started = Instant::now();
    loop {
        if let Some(status) = main.child.try_wait().expect("cannot poll koebi") {
            assert_eq!(status.code(), Some(exit_code::SUCCESS));
            return;
        }
        assert!(started.elapsed() < WAIT, "koebi did not quit");
        std::thread::sleep(Duration::from_millis(20));
    }
}
