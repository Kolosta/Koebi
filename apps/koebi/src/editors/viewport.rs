//! The Viewport template: a canvas that pans and zooms freely, a page on it, a grid that
//! keeps its spacing on screen, and optional rulers on each side.
//!
//! The middle button drags the canvas, the wheel zooms around the pointer; the top bar's
//! toggles show or hide each ruler.

use koebi_core::input::Key;
use koebi_core::{Color, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_input::{ActionDef, ActionGroup, Context, Shortcut};
use koebi_render::RectPrimitive;
use koebi_tokens::component;
use koebi_ui::{Align, BarLayout, Button, Icon, PointerButton, Ui, WidgetId};

/// The viewport's canvas: its keys apply where the pointer is over one.
pub(crate) const VIEWPORT: Context = Context::new("viewport");

/// Frames the page.
pub(crate) static VIEW_ALL: ActionDef = ActionDef {
    id: "viewport.view-all",
    label: "Frame all",
    description: "Zooms and pans so that the whole page shows.",
    context: VIEWPORT,
    defaults: &[Shortcut::key(Key::Home)],
};

/// Zooms in on the middle of the view.
pub(crate) static ZOOM_IN: ActionDef = ActionDef {
    id: "viewport.zoom-in",
    label: "Zoom in",
    description: "Zooms in by one step on the middle of the view.",
    context: VIEWPORT,
    defaults: &[Shortcut::key(Key::KeypadPlus)],
};

/// Zooms out from the middle of the view.
pub(crate) static ZOOM_OUT: ActionDef = ActionDef {
    id: "viewport.zoom-out",
    label: "Zoom out",
    description: "Zooms out by one step from the middle of the view.",
    context: VIEWPORT,
    defaults: &[Shortcut::key(Key::KeypadMinus)],
};

/// Shows the canvas at its real size.
pub(crate) static ACTUAL_SIZE: ActionDef = ActionDef {
    id: "viewport.actual-size",
    label: "Actual size",
    description: "Zooms to 100 %: one canvas unit per logical pixel.",
    context: VIEWPORT,
    defaults: &[Shortcut::key(Key::Keypad1)],
};

/// Shows or hides the rulers.
pub(crate) static TOGGLE_RULERS: ActionDef = ActionDef {
    id: "viewport.toggle-rulers",
    label: "Toggle rulers",
    description: "Hides every ruler, or shows the top and left ones.",
    context: VIEWPORT,
    defaults: &[Shortcut::key(Key::R).primary()],
};

/// The viewport's actions, for the keymap.
pub(crate) static ACTIONS: ActionGroup = ActionGroup {
    name: "Viewport",
    actions: &[&VIEW_ALL, &ZOOM_IN, &ZOOM_OUT, &ACTUAL_SIZE, &TOGGLE_RULERS],
};

/// The template page, in canvas units.
const PAGE: Rect = Rect::from_min_max(Vec2::new(0.0, 0.0), Vec2::new(800.0, 600.0));

/// Zoom limits.
const ZOOM_RANGE: (f32, f32) = (0.02, 64.0);

/// The sides a ruler may be on, in the order of [`View::rulers`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Top,
    Left,
    Bottom,
    Right,
}

const SIDES: [Side; 4] = [Side::Top, Side::Left, Side::Bottom, Side::Right];

/// Colors and metrics of the viewport, from the `component.viewport.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Style {
    canvas: Color,
    grid: Color,
    grid_spacing: f32,
    page: Color,
    page_border: Color,
    origin: Color,
    ruler: f32,
    ruler_background: Color,
    ruler_tick: Color,
    ruler_label: Color,
    zoom_step: f32,
}

impl Style {
    fn resolve(design: &DesignSystem) -> Self {
        Self {
            canvas: design.get(component::VIEWPORT_CANVAS_BACKGROUND),
            grid: design.get(component::VIEWPORT_GRID_COLOR),
            grid_spacing: design.get(component::VIEWPORT_GRID_SPACING).max(1.0),
            page: design.get(component::VIEWPORT_PAGE_BACKGROUND),
            page_border: design.get(component::VIEWPORT_PAGE_BORDER),
            origin: design.get(component::VIEWPORT_ORIGIN_OUTLINE),
            ruler: design.get(component::VIEWPORT_RULER_SIZE),
            ruler_background: design.get(component::VIEWPORT_RULER_BACKGROUND),
            ruler_tick: design.get(component::VIEWPORT_RULER_TICK),
            ruler_label: design.get(component::VIEWPORT_RULER_LABEL),
            zoom_step: design.get(component::VIEWPORT_ZOOM_STEP).max(1.01),
        }
    }
}

/// What a viewport shows: where its canvas is, how big, which rulers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct View {
    /// The canvas point in the middle of the canvas area.
    center: Vec2,
    /// Logical pixels per canvas unit.
    zoom: f32,
    /// Rulers on the top, left, bottom and right sides.
    rulers: [bool; 4],
    /// Frames the page on the first frame it is drawn.
    framed: bool,
    /// Where the pointer held the canvas in the last frame, while the middle button drags it.
    held: Option<Vec2>,
}

impl Default for View {
    fn default() -> Self {
        Self {
            center: PAGE.center(),
            zoom: 1.0,
            rulers: [true, true, false, false],
            framed: false,
            held: None,
        }
    }
}

impl View {
    /// The screen point of canvas point `point`, in a canvas area `area`.
    fn to_screen(self, area: Rect, point: Vec2) -> Vec2 {
        area.center() + (point - self.center) * self.zoom
    }

    /// The canvas point under screen point `point`.
    fn to_canvas(self, area: Rect, point: Vec2) -> Vec2 {
        self.center + (point - area.center()) * (1.0 / self.zoom)
    }

    /// Zooms by `factor`, keeping the canvas point under `anchor` (a screen point) in place.
    fn zoom_at(&mut self, area: Rect, anchor: Vec2, factor: f32) {
        let held = self.to_canvas(area, anchor);
        self.zoom = (self.zoom * factor).clamp(ZOOM_RANGE.0, ZOOM_RANGE.1);
        self.center = held - (anchor - area.center()) * (1.0 / self.zoom);
    }

    /// Fits `rect` (canvas units) in `area` with a margin.
    fn frame(&mut self, area: Rect, rect: Rect) {
        let fit = (area.width() / rect.width()).min(area.height() / rect.height()) * 0.9;
        if fit.is_finite() && fit > 0.0 {
            self.zoom = fit.clamp(ZOOM_RANGE.0, ZOOM_RANGE.1);
        }
        self.center = rect.center();
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
}

/// The step between grid lines or ruler ticks, in canvas units: 1, 2 or 5 times a power of
/// ten, the smallest at least `spacing` logical pixels apart at `zoom`.
fn step(zoom: f32, spacing: f32) -> f32 {
    let wanted = spacing / zoom;
    let mut step = 10.0_f32.powf(wanted.log10().floor());
    for factor in [1.0, 2.0, 5.0, 10.0] {
        if step * factor >= wanted {
            step *= factor;
            break;
        }
    }
    step
}

/// `value` in decimal, written into `digits`.
fn integer(value: i64, digits: &mut [u8; 21]) -> &str {
    let mut rest = value.unsigned_abs();
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = b"0123456789"[usize::try_from(rest % 10).unwrap_or(0)];
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    if value < 0 {
        start -= 1;
        digits[start] = b'-';
    }
    core::str::from_utf8(&digits[start..]).expect("decimal digits are ASCII")
}

/// The multiples of `step` from `from` to `to`, as whole numbers of steps.
fn multiples(from: f32, to: f32, step: f32) -> impl Iterator<Item = f32> {
    let first = (from / step).ceil();
    let count = ((to / step).floor() - first + 1.0).clamp(0.0, 4000.0);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to a few thousand lines"
    )]
    let count = count as u32;
    (0..count).map(move |index| (first + index as f32) * step)
}

/// The ruler toggles of a viewport's top bar, from the right.
pub(crate) fn top_bar(ui: &mut Ui<'_>, id: WidgetId, view: &mut View, bar: &mut BarLayout) {
    let icons = [
        Icon::ChevronUp,
        Icon::ChevronLeft,
        Icon::ChevronDown,
        Icon::ChevronRight,
    ];
    let side = bar.rect().height();
    for (index, icon) in icons.into_iter().enumerate().rev() {
        let rect = bar.right(side);
        Button::new(id.with(100 + index as u64))
            .icon(icon)
            .toggle(&mut view.rulers[index])
            .show(ui, rect);
    }
}

/// Draws a viewport in `rect` and applies its input.
pub(crate) fn show(ui: &mut Ui<'_>, rect: Rect, design: &DesignSystem, view: &mut View) {
    let style = Style::resolve(design);
    // Side rulers are as wide as four digits and a sign.
    let digits = ui.text_width("-8888", &ui.styles().text.detail);
    let area = view.canvas_area(rect, style.ruler, style.ruler.max(digits + 6.0));
    if !view.framed {
        view.frame(area, PAGE);
        view.framed = true;
    }
    ui.zone(rect, VIEWPORT, |ui| {
        handle_input(ui, rect, area, view, &style);
        draw_canvas(ui, area, view, &style);
        draw_rulers(ui, rect, area, view, &style);
    });
}

fn handle_input(ui: &mut Ui<'_>, rect: Rect, area: Rect, view: &mut View, style: &Style) {
    let input = ui.input();
    let pointer = input.pointer();
    let over = pointer.is_some_and(|point| area.contains(point));
    // The middle button holds the canvas from a press on it until its release.
    if input
        .button_pressed_at(PointerButton::Middle)
        .is_some_and(|point| area.contains(point))
    {
        view.held = pointer;
    }
    if !input.is_button_down(PointerButton::Middle) {
        view.held = None;
    }
    if let (Some(held), Some(point)) = (view.held, pointer) {
        view.center -= (point - held) * (1.0 / view.zoom);
        view.held = Some(point);
    }
    if over && let Some(point) = pointer {
        let notches = input.wheel().y;
        if notches != 0.0 {
            view.zoom_at(area, point, style.zoom_step.powf(notches));
        }
    }
    if !pointer.is_some_and(|point| rect.contains(point)) {
        return;
    }
    if ui.triggered(&VIEW_ALL) {
        view.frame(area, PAGE);
    }
    if ui.triggered(&ZOOM_IN) {
        view.zoom_at(area, area.center(), style.zoom_step);
    }
    if ui.triggered(&ZOOM_OUT) {
        view.zoom_at(area, area.center(), 1.0 / style.zoom_step);
    }
    if ui.triggered(&ACTUAL_SIZE) {
        view.zoom_at(area, area.center(), 1.0 / view.zoom);
    }
    if ui.triggered(&TOGGLE_RULERS) {
        let shown = view.rulers.iter().any(|on| *on);
        view.rulers = [!shown, !shown, false, false];
    }
}

fn draw_canvas(ui: &mut Ui<'_>, area: Rect, view: &View, style: &Style) {
    let scene = ui.scene();
    scene.push_rect(&RectPrimitive::new(area, style.canvas));
    // The grid, a line every step that keeps its lines apart on screen.
    let step = step(view.zoom, style.grid_spacing);
    let (from, to) = (
        view.to_canvas(area, area.min),
        view.to_canvas(area, area.max),
    );
    for x in multiples(from.x, to.x, step) {
        let screen = view.to_screen(area, Vec2::new(x, 0.0)).x.round();
        scene.push_rect(&RectPrimitive::new(
            Rect::from_min_max(
                Vec2::new(screen, area.min.y),
                Vec2::new(screen + 1.0, area.max.y),
            ),
            style.grid,
        ));
    }
    for y in multiples(from.y, to.y, step) {
        let screen = view.to_screen(area, Vec2::new(0.0, y)).y.round();
        scene.push_rect(&RectPrimitive::new(
            Rect::from_min_max(
                Vec2::new(area.min.x, screen),
                Vec2::new(area.max.x, screen + 1.0),
            ),
            style.grid,
        ));
    }
    // The page, and a mark at the origin.
    let page = Rect::from_min_max(
        view.to_screen(area, PAGE.min),
        view.to_screen(area, PAGE.max),
    );
    scene.push_rect(&RectPrimitive::new(page, style.page).with_border(1.0, style.page_border));
    let origin = view.to_screen(area, Vec2::ZERO);
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
    let step = step(view.zoom, style.grid_spacing * 3.0);
    let (from, to) = (
        view.to_canvas(area, area.min),
        view.to_canvas(area, area.max),
    );
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
        let mut digits = [0; 21];
        for value in multiples(start, end, step) {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "ruler values are whole multiples of steps far below i64's range"
            )]
            let number = value.round() as i64;
            let text = integer(number, &mut digits);
            if horizontal {
                let x = view.to_screen(area, Vec2::new(value, 0.0)).x.round();
                ui.scene().push_rect(&RectPrimitive::new(
                    Rect::from_min_max(Vec2::new(x, strip.min.y), Vec2::new(x + 1.0, strip.max.y)),
                    style.ruler_tick,
                ));
                let label = Rect::from_min_max(
                    Vec2::new(x + 3.0, strip.min.y),
                    Vec2::new(x + step * view.zoom, strip.max.y),
                );
                ui.label(text, label, &font, style.ruler_label, Align::Start);
            } else {
                let y = view.to_screen(area, Vec2::new(0.0, value)).y.round();
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
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect::from_origin_size(Vec2::new(100.0, 50.0), Vec2::new(400.0, 300.0))
    }

    #[test]
    fn zooming_keeps_the_point_under_the_pointer() {
        let mut view = View::default();
        let anchor = Vec2::new(150.0, 80.0);
        let before = view.to_canvas(area(), anchor);
        view.zoom_at(area(), anchor, 2.0);
        assert_eq!(view.zoom, 2.0);
        let after = view.to_canvas(area(), anchor);
        assert!((after - before).length() < 1e-3);
        // Zoom stays within its limits.
        view.zoom_at(area(), anchor, 1e9);
        assert_eq!(view.zoom, ZOOM_RANGE.1);
    }

    #[test]
    fn framing_fits_the_page_in_the_area() {
        let mut view = View::default();
        view.frame(area(), PAGE);
        assert_eq!(view.center, PAGE.center());
        let page = Rect::from_min_max(
            view.to_screen(area(), PAGE.min),
            view.to_screen(area(), PAGE.max),
        );
        assert!((page.width() - 360.0).abs() < 1e-3);
        assert!(page.height() <= area().height());
    }

    #[test]
    fn grid_steps_are_round_and_far_enough_apart() {
        assert_eq!(step(1.0, 24.0), 50.0);
        assert_eq!(step(2.0, 24.0), 20.0);
        assert_eq!(step(0.1, 24.0), 500.0);
        assert_eq!(step(10.0, 24.0), 5.0);
        let values: Vec<f32> = multiples(-12.0, 41.0, 10.0).collect();
        assert_eq!(values, [-10.0, 0.0, 10.0, 20.0, 30.0, 40.0]);
    }

    #[test]
    fn rulers_take_their_side_and_numbers_read_back() {
        let mut view = View::default();
        let rect = area();
        assert_eq!(
            view.canvas_area(rect, 20.0, 30.0).min,
            rect.min + Vec2::new(30.0, 20.0)
        );
        view.rulers = [false, false, true, true];
        assert_eq!(
            view.canvas_area(rect, 20.0, 30.0).max,
            rect.max - Vec2::new(30.0, 20.0)
        );
        let mut digits = [0; 21];
        assert_eq!(integer(-1250, &mut digits), "-1250");
        assert_eq!(integer(0, &mut digits), "0");
    }
}
