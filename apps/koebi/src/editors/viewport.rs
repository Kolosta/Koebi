//! The Viewport: the document on an unbounded canvas, drawn by `koebi-canvas`, over a grid of
//! dots that keeps its spacing on screen, with optional rulers on each side.
//!
//! - **Camera:** the document point at the middle of the canvas area and a zoom, in f64, with
//!   no limit but what f64 holds: screen and document map exactly at every zoom.
//! - **Navigation**, each an action with its shortcuts: the wheel and a pinch zoom around the
//!   pointer, by the `viewport.zoom-step` token a notch; Ctrl and Shift with the wheel, a
//!   sideways wheel and Ctrl with the keypad's arrows pan; the middle button drags the canvas.
//!   Fit Document, Frame Selected and Reset View move the camera in an eased motion of the
//!   `viewport.frame.*` tokens.
//! - **The canvas image:** each viewport asks its window for a view of the document
//!   ([`CanvasViews`]); the window's GPU draws it into a target of the area's physical pixels,
//!   which the area shows texel for pixel, cut at its rounded corners.
//! - **Overlays**, from the menu of the top bar: the grid and each ruler.

use std::time::Duration;

use koebi_core::input::{Key, ModifierKey, MouseButton};
use koebi_core::{Color, Extent, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_design::value::CubicBezier;
use koebi_geom::{Point, Vector};
use koebi_i18n::{n_, tr};
use koebi_input::{
    ActionDef, ActionGroup, ActionKind, ButtonTrigger, Context, Hint, PinchDirection, Shortcut,
    Trigger, WheelDirection,
};
use koebi_render::{DotGrid, ImagePrimitive, RectPrimitive};
use koebi_tokens::component;
use koebi_ui::{
    Align, Amount, BarLayout, Button, MenuItem, MenuState, Placement, PointerButton, Ui, WidgetId,
};
use koebi_workspace::EditorId;

use crate::canvas::CanvasViews;

/// The viewport's canvas: its keys apply where the pointer is over one.
pub(crate) const VIEWPORT: Context = Context::new("viewport");

/// The wheel turned `direction`, without a modifier.
const fn wheel(direction: WheelDirection) -> Shortcut {
    Shortcut::new(Trigger::Wheel(direction))
}

/// Frames the whole document.
pub(crate) static FIT_DOCUMENT: ActionDef = ActionDef {
    id: "viewport.fit-document",
    label: n_!("Fit document"),
    description: n_!("Zooms and pans so that the whole document shows."),
    context: VIEWPORT,
    defaults: &[
        Shortcut::key(Key::C).with(ModifierKey::Shift),
        Shortcut::key(Key::Home),
    ],
    repeats: false,
    when: None,
    hint: Some(Hint::new(ActionKind::View, 7)),
};

/// Frames the selection.
pub(crate) static FRAME_SELECTED: ActionDef = ActionDef {
    id: "viewport.frame-selected",
    label: n_!("Frame selected"),
    description: n_!("Zooms and pans so that the selection shows, or the document without one."),
    context: VIEWPORT,
    defaults: &[Shortcut::key(Key::KeypadPeriod)],
    repeats: false,
    when: None,
    hint: Some(Hint::new(ActionKind::View, 6)),
};

/// Shows the page at its real size, centered.
pub(crate) static RESET_VIEW: ActionDef = ActionDef {
    id: "viewport.reset-view",
    label: n_!("Reset view"),
    description: n_!("Shows the page at its real size, centered."),
    context: VIEWPORT,
    defaults: &[Shortcut::key(Key::Keypad0).primary()],
    repeats: false,
    when: None,
    hint: Some(Hint::new(ActionKind::View, 5)),
};

/// Zooms in: on the middle of the view from a key, around the pointer from the wheel or a
/// pinch.
pub(crate) static ZOOM_IN: ActionDef = ActionDef {
    id: "viewport.zoom-in",
    label: n_!("Zoom in"),
    description: n_!("Zooms in by one step, around the pointer from the wheel or a pinch."),
    context: VIEWPORT,
    defaults: &[
        Shortcut::key(Key::KeypadPlus),
        wheel(WheelDirection::Up),
        Shortcut::new(Trigger::Pinch(PinchDirection::In)),
    ],
    repeats: true,
    when: None,
    hint: Some(Hint::new(ActionKind::View, 6)),
};

/// Zooms out: from the middle of the view from a key, around the pointer from the wheel or a
/// pinch.
pub(crate) static ZOOM_OUT: ActionDef = ActionDef {
    id: "viewport.zoom-out",
    label: n_!("Zoom out"),
    description: n_!("Zooms out by one step, around the pointer from the wheel or a pinch."),
    context: VIEWPORT,
    defaults: &[
        Shortcut::key(Key::KeypadMinus),
        wheel(WheelDirection::Down),
        Shortcut::new(Trigger::Pinch(PinchDirection::Out)),
    ],
    repeats: true,
    when: None,
    hint: Some(Hint::new(ActionKind::View, 6)),
};

/// Shows the document at its real size.
pub(crate) static ACTUAL_SIZE: ActionDef = ActionDef {
    id: "viewport.actual-size",
    label: n_!("Actual size"),
    description: n_!("Zooms to 100 %: a millimetre of the document is one on the screen."),
    context: VIEWPORT,
    defaults: &[Shortcut::key(Key::Keypad1)],
    repeats: false,
    when: None,
    hint: Some(Hint::new(ActionKind::View, 4)),
};

/// Drags the canvas while the button or key that started it stays down.
pub(crate) static PAN: ActionDef = ActionDef {
    id: "viewport.pan",
    label: n_!("Pan"),
    description: n_!("Drags the canvas with the pointer while the button stays down."),
    context: VIEWPORT,
    defaults: &[Shortcut::button(MouseButton::Middle, ButtonTrigger::Press)],
    repeats: false,
    when: None,
    hint: Some(Hint::new(ActionKind::View, 7)),
};

/// Moves the view left.
pub(crate) static PAN_LEFT: ActionDef = ActionDef {
    id: "viewport.pan-left",
    label: n_!("Pan left"),
    description: n_!("Moves the view to the left."),
    context: VIEWPORT,
    defaults: &[
        wheel(WheelDirection::Left),
        wheel(WheelDirection::Up).with(ModifierKey::Ctrl),
        Shortcut::key(Key::Keypad4).with(ModifierKey::Ctrl),
    ],
    repeats: true,
    when: None,
    hint: Some(Hint::new(ActionKind::View, 3)),
};

/// Moves the view right.
pub(crate) static PAN_RIGHT: ActionDef = ActionDef {
    id: "viewport.pan-right",
    label: n_!("Pan right"),
    description: n_!("Moves the view to the right."),
    context: VIEWPORT,
    defaults: &[
        wheel(WheelDirection::Right),
        wheel(WheelDirection::Down).with(ModifierKey::Ctrl),
        Shortcut::key(Key::Keypad6).with(ModifierKey::Ctrl),
    ],
    repeats: true,
    when: None,
    hint: Some(Hint::new(ActionKind::View, 3)),
};

/// Moves the view up.
pub(crate) static PAN_UP: ActionDef = ActionDef {
    id: "viewport.pan-up",
    label: n_!("Pan up"),
    description: n_!("Moves the view up."),
    context: VIEWPORT,
    defaults: &[
        wheel(WheelDirection::Up).with(ModifierKey::Shift),
        Shortcut::key(Key::Keypad8).with(ModifierKey::Ctrl),
    ],
    repeats: true,
    when: None,
    hint: Some(Hint::new(ActionKind::View, 3)),
};

/// Moves the view down.
pub(crate) static PAN_DOWN: ActionDef = ActionDef {
    id: "viewport.pan-down",
    label: n_!("Pan down"),
    description: n_!("Moves the view down."),
    context: VIEWPORT,
    defaults: &[
        wheel(WheelDirection::Down).with(ModifierKey::Shift),
        Shortcut::key(Key::Keypad2).with(ModifierKey::Ctrl),
    ],
    repeats: true,
    when: None,
    hint: Some(Hint::new(ActionKind::View, 3)),
};

/// Shows or hides the rulers.
pub(crate) static TOGGLE_RULERS: ActionDef = ActionDef {
    id: "viewport.toggle-rulers",
    label: n_!("Toggle rulers"),
    description: n_!("Hides every ruler, or shows the top and left ones."),
    context: VIEWPORT,
    defaults: &[Shortcut::key(Key::R).primary()],
    repeats: false,
    when: None,
    hint: Some(Hint::new(ActionKind::View, 3)),
};

/// Shows or hides the grid.
pub(crate) static TOGGLE_GRID: ActionDef = ActionDef {
    id: "viewport.toggle-grid",
    label: n_!("Toggle grid"),
    description: n_!("Shows or hides the grid of dots behind the document."),
    context: VIEWPORT,
    defaults: &[],
    repeats: false,
    when: None,
    hint: Some(Hint::new(ActionKind::View, 2)),
};

/// The viewport's actions, for the keymap.
pub(crate) static ACTIONS: ActionGroup = ActionGroup {
    name: n_!("Viewport"),
    actions: &[
        &FIT_DOCUMENT,
        &FRAME_SELECTED,
        &RESET_VIEW,
        &ZOOM_IN,
        &ZOOM_OUT,
        &ACTUAL_SIZE,
        &PAN,
        &PAN_LEFT,
        &PAN_RIGHT,
        &PAN_UP,
        &PAN_DOWN,
        &TOGGLE_RULERS,
        &TOGGLE_GRID,
    ],
};

/// Logical pixels per document unit (a millimetre) at 100 %: a logical pixel is 1/96 inch.
pub(crate) const ACTUAL: f64 = 96.0 / 25.4;

/// Zoom limits, in logical pixels per document unit: far beyond any use, within what f64
/// keeps exact for a document a kilometre wide.
const ZOOM_RANGE: (f64, f64) = (1e-9, 1e12);

/// The sides a ruler may be on, in the order of [`View::rulers`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Top,
    Left,
    Bottom,
    Right,
}

const SIDES: [Side; 4] = [Side::Top, Side::Left, Side::Bottom, Side::Right];

/// Colors, metrics and motion of the viewport, from the `component.viewport.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Style {
    canvas: Color,
    /// The dots of the canvas grid.
    dots: DotGrid,
    /// The smallest space between grid steps on screen, which the rulers' ticks keep too.
    grid_spacing: f32,
    page_border: Color,
    origin: Color,
    ruler: f32,
    ruler_background: Color,
    ruler_tick: Color,
    ruler_label: Color,
    zoom_step: f64,
    pan_step: f64,
    frame_duration: Duration,
    frame_easing: CubicBezier,
}

impl Style {
    fn resolve(design: &DesignSystem) -> Self {
        Self {
            canvas: design.get(component::VIEWPORT_CANVAS_BACKGROUND),
            dots: DotGrid {
                step: 1.0,
                // A whole number from 2 to 10, as the token's range says.
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a small whole number"
                )]
                subdivisions: design
                    .get(component::VIEWPORT_GRID_SUBDIVISIONS)
                    .round()
                    .clamp(2.0, 10.0) as u32,
                min_distance: design.get(component::VIEWPORT_GRID_SPACING).max(1.0),
                size: design.get(component::VIEWPORT_GRID_DOT_SIZE).max(1.0),
                color: design.get(component::VIEWPORT_GRID_COLOR),
            },
            grid_spacing: design.get(component::VIEWPORT_GRID_SPACING).max(1.0),
            page_border: design.get(component::VIEWPORT_PAGE_BORDER),
            origin: design.get(component::VIEWPORT_ORIGIN_OUTLINE),
            ruler: design.get(component::VIEWPORT_RULER_SIZE),
            ruler_background: design.get(component::VIEWPORT_RULER_BACKGROUND),
            ruler_tick: design.get(component::VIEWPORT_RULER_TICK),
            ruler_label: design.get(component::VIEWPORT_RULER_LABEL),
            zoom_step: f64::from(design.get(component::VIEWPORT_ZOOM_STEP).max(1.01)),
            pan_step: f64::from(design.get(component::VIEWPORT_PAN_STEP).max(1.0)),
            frame_duration: design.get(component::VIEWPORT_FRAME_DURATION),
            frame_easing: design.get(component::VIEWPORT_FRAME_EASING),
        }
    }
}

/// A camera: the document point at the middle of the canvas area, and logical pixels per
/// document unit.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Camera {
    center: Point,
    zoom: f64,
}

/// A camera moving to another, eased.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Motion {
    from: Camera,
    to: Camera,
    /// When it started, on the interface's clock.
    start: Duration,
}

/// What holds the canvas while it is dragged: the buttons down and the key held when the drag
/// started, and where the pointer was in the last frame.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Grip {
    at: Vec2,
    buttons: [bool; 3],
    key: Option<Key>,
}

const BUTTONS: [PointerButton; 3] = [
    PointerButton::Primary,
    PointerButton::Secondary,
    PointerButton::Middle,
];

/// What a viewport shows: its camera, its overlays, and what moves or holds it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct View {
    camera: Camera,
    /// Rulers on the top, left, bottom and right sides.
    rulers: [bool; 4],
    /// The grid of dots.
    grid: bool,
    /// Fits the document on the first frame it is drawn.
    framed: bool,
    /// The drag in progress.
    grip: Option<Grip>,
    /// An eased move of the camera in progress.
    motion: Option<Motion>,
    /// The overlays menu of the top bar.
    overlays: MenuState,
    /// The canvas area of the last frame: the area without its rulers.
    area: Rect,
}

impl Default for View {
    fn default() -> Self {
        Self {
            camera: Camera {
                center: Point::ORIGIN,
                zoom: ACTUAL,
            },
            rulers: [true, true, false, false],
            grid: true,
            framed: false,
            grip: None,
            motion: None,
            overlays: MenuState::new(),
            area: Rect::ZERO,
        }
    }
}

/// `point` in f64.
fn wide(point: Vec2) -> Vector {
    Vector::new(f64::from(point.x), f64::from(point.y))
}

impl View {
    /// The screen point (logical pixels, f64) of document point `point` in a canvas area
    /// `area`.
    fn to_screen(&self, area: Rect, point: Point) -> [f64; 2] {
        let at = wide(area.center()) + (point - self.camera.center) * self.camera.zoom;
        [at.x, at.y]
    }

    /// The same, in f32, held to a few million pixels around the area: drawing clips it.
    fn to_screen_f32(&self, area: Rect, point: Point) -> Vec2 {
        let [x, y] = self.to_screen(area, point);
        #[allow(
            clippy::cast_possible_truncation,
            reason = "clamped to what f32 holds exactly enough for a clipped edge"
        )]
        Vec2::new(x.clamp(-1e7, 1e7) as f32, y.clamp(-1e7, 1e7) as f32)
    }

    /// The document point under screen point `point`.
    fn to_document(&self, area: Rect, point: Vec2) -> Point {
        self.camera.center + (wide(point) - wide(area.center())) * (1.0 / self.camera.zoom)
    }

    /// Zooms by `factor`, keeping the document point under `anchor` (a screen point) in place.
    fn zoom_at(&mut self, area: Rect, anchor: Vec2, factor: f64) {
        if !(factor.is_finite() && factor > 0.0) {
            return;
        }
        let held = self.to_document(area, anchor);
        self.camera.zoom = (self.camera.zoom * factor).clamp(ZOOM_RANGE.0, ZOOM_RANGE.1);
        self.camera.center = held - (wide(anchor) - wide(area.center())) * (1.0 / self.camera.zoom);
    }

    /// Moves the view by `delta` logical pixels on screen: the canvas moves the other way.
    fn pan(&mut self, delta: Vector) {
        self.camera.center += delta * (1.0 / self.camera.zoom);
    }

    /// The camera fitting `bounds` (document units) in `area` with a margin.
    fn fitted(area: Rect, bounds: koebi_geom::Rect) -> Option<Camera> {
        let fit = (f64::from(area.width()) / bounds.width())
            .min(f64::from(area.height()) / bounds.height())
            * 0.9;
        (fit.is_finite() && fit > 0.0).then(|| Camera {
            center: bounds.center(),
            zoom: fit.clamp(ZOOM_RANGE.0, ZOOM_RANGE.1),
        })
    }

    /// Starts an eased move to `to` at `now`.
    fn move_to(&mut self, to: Camera, now: Duration) {
        self.motion = Some(Motion {
            from: self.camera,
            to,
            start: now,
        });
    }

    /// Advances the move in progress to `now`; whether it goes on.
    fn advance(&mut self, now: Duration, duration: Duration, easing: &CubicBezier) -> bool {
        let Some(motion) = self.motion else {
            return false;
        };
        let elapsed = now.saturating_sub(motion.start);
        if duration.is_zero() || elapsed >= duration {
            self.camera = motion.to;
            self.motion = None;
            return false;
        }
        #[allow(clippy::cast_possible_truncation, reason = "a share from 0 to 1")]
        let t = (elapsed.as_secs_f64() / duration.as_secs_f64()) as f32;
        let eased = f64::from(easing.ease(t));
        // The zoom moves evenly in its logarithm: each step looks the same.
        let (from, to) = (motion.from.zoom.ln(), motion.to.zoom.ln());
        self.camera.zoom = (from + (to - from) * eased).exp();
        self.camera.center = motion.from.center + (motion.to.center - motion.from.center) * eased;
        true
    }

    /// The canvas area of `rect`: without the rulers shown, `ruler` thick on the top and
    /// bottom and `side` thick on the left and right (where numbers lie across them).
    fn canvas_area(&self, rect: Rect, ruler: f32, side: f32) -> Rect {
        let size = |index: usize, thickness: f32| if self.rulers[index] { thickness } else { 0.0 };
        let (top, left, bottom, right) =
            (size(0, ruler), size(1, side), size(2, ruler), size(3, side));
        Rect::from_min_max(
            rect.min + Vec2::new(left, top),
            Vec2::new(
                (rect.max.x - right).max(rect.min.x + left),
                (rect.max.y - bottom).max(rect.min.y + top),
            ),
        )
    }

    /// The canvas's view of `area` at `scale` physical pixels per logical one, and the area
    /// snapped to whole physical pixels, which its image fills texel for pixel.
    fn canvas(&self, area: Rect, scale: f32) -> Option<(koebi_canvas::View, Rect)> {
        let snap = |value: f32| (value * scale).round();
        let (x0, y0, x1, y1) = (
            snap(area.min.x),
            snap(area.min.y),
            snap(area.max.x),
            snap(area.max.y),
        );
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a whole number of physical pixels of a window"
        )]
        let extent = Extent::new((x1 - x0).max(0.0) as u32, (y1 - y0).max(0.0) as u32);
        let snapped = Rect::from_min_max(
            Vec2::new(x0 / scale, y0 / scale),
            Vec2::new(x1 / scale, y1 / scale),
        );
        // The middle of the whole pixels, in f64: the image's texels lie exactly on them.
        let middle = Vector::new(
            f64::midpoint(f64::from(x0), f64::from(x1)),
            f64::midpoint(f64::from(y0), f64::from(y1)),
        ) * (1.0 / f64::from(scale));
        let view = koebi_canvas::View {
            center: self.camera.center + (middle - wide(area.center())) * (1.0 / self.camera.zoom),
            zoom: self.camera.zoom * f64::from(scale),
            extent,
        };
        view.is_valid().then_some((view, snapped))
    }
}

/// The step between ruler ticks, in document units: 1, 2 or 5 times a power of ten, the
/// smallest at least `spacing` logical pixels apart at `zoom`.
fn step(zoom: f64, spacing: f64) -> f64 {
    let wanted = spacing / zoom;
    let mut step = 10.0_f64.powf(wanted.log10().floor());
    for factor in [1.0, 2.0, 5.0, 10.0] {
        if step * factor >= wanted {
            step *= factor;
            break;
        }
    }
    step
}

/// `value` with `decimals` decimals (at most 12), written into `digits`.
fn decimal(value: f64, decimals: u32, digits: &mut [u8; 32]) -> &str {
    let decimals = decimals.min(12);
    #[allow(
        clippy::cast_possible_truncation,
        reason = "ruler values times a power of ten stay far below i64's range"
    )]
    let scaled = (value * 10f64.powi(decimals.cast_signed())).round() as i64;
    let mut rest = scaled.unsigned_abs();
    let mut start = digits.len();
    let mut place = 0;
    loop {
        if place == decimals && decimals > 0 {
            start -= 1;
            digits[start] = b'.';
        }
        start -= 1;
        digits[start] = b"0123456789"[usize::try_from(rest % 10).unwrap_or(0)];
        rest /= 10;
        place += 1;
        if rest == 0 && place > decimals {
            break;
        }
    }
    if scaled < 0 {
        start -= 1;
        digits[start] = b'-';
    }
    core::str::from_utf8(&digits[start..]).expect("decimal digits are ASCII")
}

/// The decimals a ruler with ticks `step` apart writes.
fn decimals(step: f64) -> u32 {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a few decimals"
    )]
    let decimals = (-step.log10().floor()).max(0.0) as u32;
    decimals
}

/// The multiples of `step` from `from` to `to`.
fn multiples(from: f64, to: f64, step: f64) -> impl Iterator<Item = f64> {
    let first = (from / step).ceil();
    let count = ((to / step).floor() - first + 1.0).clamp(0.0, 4000.0);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to a few thousand lines"
    )]
    let count = count as u32;
    (0..count).map(move |index| (first + f64::from(index)) * step)
}

/// The overlays menu of a viewport's top bar, at its right end.
pub(crate) fn top_bar(
    ui: &mut Ui<'_>,
    id: WidgetId,
    view: &mut View,
    bar: &mut BarLayout,
    window: Rect,
) {
    let side = bar.rect().height();
    let rect = bar.right(side);
    Button::new(id.with(100))
        .icon(koebi_icons::view::BACKGROUND_DOT_SMALL)
        .menu(&mut view.overlays, Placement::BELOW)
        .show(ui, rect);
    ui.tooltip(id.with(100), tr("Overlays"), rect);
    let items = [
        MenuItem::Title(tr("Overlays")),
        MenuItem::action(tr("Grid")).checked(view.grid),
        MenuItem::Separator,
        MenuItem::Title(tr("Rulers")),
        MenuItem::action(tr("Top")).checked(view.rulers[0]),
        MenuItem::action(tr("Left")).checked(view.rulers[1]),
        MenuItem::action(tr("Bottom")).checked(view.rulers[2]),
        MenuItem::action(tr("Right")).checked(view.rulers[3]),
    ];
    match ui
        .menu(id.with(101), &mut view.overlays, &items, window)
        .map(|choice| choice.index())
    {
        Some(1) => view.grid = !view.grid,
        Some(index @ 4..=7) => view.rulers[index - 4] = !view.rulers[index - 4],
        _ => {}
    }
}

/// What a viewport shows besides its own state: the document's page, and the window's views.
pub(crate) struct Shown<'a> {
    /// The viewport.
    pub(crate) editor: EditorId,
    /// The document's first page, in document units, if it has one.
    pub(crate) page: Option<koebi_geom::Rect>,
    /// The window's views of the document.
    pub(crate) views: &'a mut CanvasViews,
}

/// Draws a viewport in `rect` and applies its input.
pub(crate) fn show(
    ui: &mut Ui<'_>,
    rect: Rect,
    design: &DesignSystem,
    view: &mut View,
    mut shown: Shown<'_>,
) {
    let style = Style::resolve(design);
    // Side rulers are as wide as four digits and a sign.
    let digits = ui.text_width("-8888", &ui.styles().text.detail);
    let area = view.canvas_area(rect, style.ruler, style.ruler.max(digits + 6.0));
    view.area = area;
    if !view.framed
        && let Some(camera) = shown.page.and_then(|page| View::fitted(area, page))
    {
        view.camera = camera;
        view.framed = true;
    }
    if view.advance(ui.input().time(), style.frame_duration, &style.frame_easing) {
        ui.request_redraw_at(ui.input().time());
    }
    ui.zone(rect, VIEWPORT, |ui| {
        handle_input(ui, rect, area, view, &style, shown.page);
        draw_canvas(ui, area, view, &style, &mut shown);
        draw_rulers(ui, rect, area, view, &style);
    });
}

/// The zoom factor of one trigger of Zoom In (`inward`) or Zoom Out.
fn zoom_factor(amount: Amount, step: f64, inward: bool) -> f64 {
    let notches = match amount {
        // A pinch zooms by its own scale, whichever way.
        Amount::Scale(scale) => return f64::from(scale),
        Amount::Once => 1.0,
        Amount::Notches(notches) => f64::from(notches),
    };
    step.powf(if inward { notches } else { -notches })
}

/// The logical pixels one trigger of a pan moves.
fn pan_length(amount: Amount, step: f64) -> f64 {
    match amount {
        Amount::Notches(notches) => f64::from(notches) * step,
        Amount::Once | Amount::Scale(_) => step,
    }
}

fn handle_input(
    ui: &mut Ui<'_>,
    rect: Rect,
    area: Rect,
    view: &mut View,
    style: &Style,
    page: Option<koebi_geom::Rect>,
) {
    let input = ui.input();
    let pointer = ui.pointer();
    let now = input.time();
    // A drag holds the canvas while a button or the key that started it stays down.
    if let Some(grip) = view.grip {
        let held = BUTTONS
            .iter()
            .zip(grip.buttons)
            .any(|(button, down)| down && input.is_button_down(*button))
            || grip
                .key
                .is_some_and(|key| input.keys_held().any(|held| held == key));
        match (held, pointer) {
            (true, Some(point)) => {
                view.pan(wide(grip.at) - wide(point));
                view.grip = Some(Grip { at: point, ..grip });
                ui.hold_pointer();
            }
            (true, None) => ui.hold_pointer(),
            (false, _) => view.grip = None,
        }
    }
    // During another action, these gestures are not the canvas's.
    let Some(point) = pointer.filter(|point| ui.pointer_is_free() && area.contains(*point)) else {
        return;
    };
    if view.grip.is_none() && ui.triggered(&PAN) {
        view.motion = None;
        let buttons = BUTTONS.map(|button| input.is_button_down(button));
        view.grip = Some(Grip {
            at: point,
            buttons,
            // A key starts the drag only when no button is down.
            key: if buttons.contains(&true) {
                None
            } else {
                input.keys_held().last()
            },
        });
        ui.hold_pointer();
    }
    for (action, inward) in [(&ZOOM_IN, true), (&ZOOM_OUT, false)] {
        for amount in ui.amounts(action) {
            view.motion = None;
            // From a key, on the middle of the view; from the wheel or a pinch, the pointer.
            let anchor = if amount == Amount::Once {
                area.center()
            } else {
                point
            };
            view.zoom_at(area, anchor, zoom_factor(amount, style.zoom_step, inward));
        }
    }
    for (action, direction) in [
        (&PAN_LEFT, Vector::new(-1.0, 0.0)),
        (&PAN_RIGHT, Vector::new(1.0, 0.0)),
        (&PAN_UP, Vector::new(0.0, -1.0)),
        (&PAN_DOWN, Vector::new(0.0, 1.0)),
    ] {
        for amount in ui.amounts(action) {
            view.motion = None;
            view.pan(direction * pan_length(amount, style.pan_step));
        }
    }
    if !rect.contains(point) {
        return;
    }
    if (ui.triggered(&FIT_DOCUMENT) || ui.triggered(&FRAME_SELECTED))
        // Without a selection yet, Frame Selected frames the document.
        && let Some(camera) = page.and_then(|page| View::fitted(area, page))
    {
        view.move_to(camera, now);
    }
    if ui.triggered(&RESET_VIEW)
        && let Some(page) = page
    {
        let camera = Camera {
            center: page.center(),
            zoom: ACTUAL,
        };
        view.move_to(camera, now);
    }
    if ui.triggered(&ACTUAL_SIZE) {
        view.motion = None;
        view.zoom_at(area, area.center(), ACTUAL / view.camera.zoom);
    }
    if ui.triggered(&TOGGLE_RULERS) {
        let shown = view.rulers.iter().any(|on| *on);
        view.rulers = [!shown, !shown, false, false];
    }
    if ui.triggered(&TOGGLE_GRID) {
        view.grid = !view.grid;
    }
}

fn draw_canvas(ui: &mut Ui<'_>, area: Rect, view: &View, style: &Style, shown: &mut Shown<'_>) {
    let scale = ui.scale_factor();
    let scene = ui.scene();
    scene.push_rect(&RectPrimitive::new(area, style.canvas));
    // The grid: dots every power of its subdivisions, the finer level fading in as it nears
    // the smallest space (ADR 0029).
    if view.grid
        && let Some(dots) =
            style
                .dots
                .fill(area, view.to_screen(area, Point::ORIGIN), view.camera.zoom)
    {
        scene.push_fill(&dots);
    }
    // The document, drawn by the window's GPU into an image of the area's pixels.
    if let Some((camera, snapped)) = view.canvas(area, scale)
        && let Some(image) = shown.views.show(shown.editor, camera)
    {
        scene.push_image(&ImagePrimitive::new(image, snapped));
    }
    // The page's edge, and a mark at the origin.
    if let Some(page) = shown.page {
        let page = Rect::from_min_max(
            view.to_screen_f32(area, page.min),
            view.to_screen_f32(area, page.max),
        );
        scene.push_rect(
            &RectPrimitive::new(page, Color::TRANSPARENT).with_border(1.0, style.page_border),
        );
    }
    let origin = view.to_screen_f32(area, Point::ORIGIN);
    scene.push_rect(
        &RectPrimitive::new(
            Rect::from_center_size(origin, Vec2::splat(9.0)),
            Color::TRANSPARENT,
        )
        .with_corner_radius(4.5)
        .with_border(1.5, style.origin),
    );
}

fn draw_rulers(ui: &mut Ui<'_>, rect: Rect, area: Rect, view: &View, style: &Style) {
    let font = ui.styles().text.detail.clone();
    let step = step(view.camera.zoom, f64::from(style.grid_spacing) * 3.0);
    let decimals = decimals(step);
    let (from, to) = (
        view.to_document(area, area.min),
        view.to_document(area, area.max),
    );
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a step's length on screen, a few dozen pixels"
    )]
    let step_on_screen = (step * view.camera.zoom) as f32;
    for (side, shown) in SIDES.into_iter().zip(view.rulers) {
        if !shown {
            continue;
        }
        let strip = match side {
            Side::Top => Rect::from_min_max(
                Vec2::new(area.min.x, rect.min.y),
                Vec2::new(area.max.x, area.min.y),
            ),
            Side::Bottom => Rect::from_min_max(
                Vec2::new(area.min.x, area.max.y),
                Vec2::new(area.max.x, rect.max.y),
            ),
            Side::Left => Rect::from_min_max(
                Vec2::new(rect.min.x, area.min.y),
                Vec2::new(area.min.x, area.max.y),
            ),
            Side::Right => Rect::from_min_max(
                Vec2::new(area.max.x, area.min.y),
                Vec2::new(rect.max.x, area.max.y),
            ),
        };
        ui.scene()
            .push_rect(&RectPrimitive::new(strip, style.ruler_background));
        let horizontal = matches!(side, Side::Top | Side::Bottom);
        let (start, end) = if horizontal {
            (from.x, to.x)
        } else {
            (from.y, to.y)
        };
        let mut digits = [0; 32];
        for value in multiples(start, end, step) {
            let text = decimal(value, decimals, &mut digits);
            if horizontal {
                let x = view.to_screen_f32(area, Point::new(value, 0.0)).x.round();
                ui.scene().push_rect(&RectPrimitive::new(
                    Rect::from_min_max(Vec2::new(x, strip.min.y), Vec2::new(x + 1.0, strip.max.y)),
                    style.ruler_tick,
                ));
                let label = Rect::from_min_max(
                    Vec2::new(x + 3.0, strip.min.y),
                    Vec2::new(x + step_on_screen, strip.max.y),
                );
                ui.label(text, label, &font, style.ruler_label, Align::Start);
            } else {
                let y = view.to_screen_f32(area, Point::new(0.0, value)).y.round();
                ui.scene().push_rect(&RectPrimitive::new(
                    Rect::from_min_max(Vec2::new(strip.min.x, y), Vec2::new(strip.max.x, y + 1.0)),
                    style.ruler_tick,
                ));
                let label = Rect::from_min_max(
                    Vec2::new(strip.min.x + 2.0, y),
                    Vec2::new(strip.max.x, y + style.ruler * 0.8),
                );
                ui.label(text, label, &font, style.ruler_label, Align::Start);
            }
        }
    }
    // The corner between two rulers.
    for (vertical, horizontal) in [(1, 0), (1, 2), (3, 0), (3, 2)] {
        if view.rulers[vertical] && view.rulers[horizontal] {
            let x = if vertical == 1 {
                (rect.min.x, area.min.x)
            } else {
                (area.max.x, rect.max.x)
            };
            let y = if horizontal == 0 {
                (rect.min.y, area.min.y)
            } else {
                (area.max.y, rect.max.y)
            };
            ui.scene().push_rect(&RectPrimitive::new(
                Rect::from_min_max(Vec2::new(x.0, y.0), Vec2::new(x.1, y.1)),
                style.ruler_background,
            ));
        }
    }
}

#[cfg(test)]
mod tests;
