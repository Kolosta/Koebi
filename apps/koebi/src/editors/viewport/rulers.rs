//! The viewport's rulers, on any of its four sides.
//!
//! - **Units:** the numbers count in the viewport's display units ([`Units`]): the
//!   document's own, metric (millimetres, centimetres or metres, whichever keeps the numbers
//!   short), imperial (inches), typographic (points) or pixels (96 an inch). The corner between
//!   two rulers shows the unit and cycles through them.
//! - **Ticks:** a major tick every 1, 2 or 5 times a power of ten of the unit, the smallest
//!   step that keeps the ticks three grid spaces apart, each step cut in five by minor ticks.
//! - **The pointer:** a mark on every ruler shows where it is.

use koebi_core::expr::{CENTIMETER, INCH, METER, MILLIMETER, PIXEL, POINT, Unit};
use koebi_core::{Color, Rect, Vec2};
use koebi_geom::Point;
use koebi_i18n::{n_, tr};
use koebi_render::RectPrimitive;
use koebi_ui::{Align, Button, Ui, WidgetId};

use super::View;

/// What a viewport's rulers count in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Units {
    /// The document's unit.
    #[default]
    Document,
    /// Millimetres, centimetres or metres, whichever keeps the numbers short.
    Metric,
    /// Inches.
    Imperial,
    /// Points.
    Typographic,
    /// Pixels, 96 an inch.
    Pixel,
}

impl Units {
    /// Every choice, in the order the corner cycles through them.
    const ALL: [Self; 5] = [
        Self::Document,
        Self::Metric,
        Self::Imperial,
        Self::Typographic,
        Self::Pixel,
    ];

    /// The next choice of the corner.
    pub(crate) fn next(self) -> Self {
        let at = Self::ALL
            .iter()
            .position(|units| *units == self)
            .unwrap_or(0);
        Self::ALL[(at + 1) % Self::ALL.len()]
    }

    /// Its name, translated.
    fn name(self) -> &'static str {
        tr(match self {
            Self::Document => n_!("Rulers in the document's unit"),
            Self::Metric => n_!("Rulers in metric units"),
            Self::Imperial => n_!("Rulers in inches"),
            Self::Typographic => n_!("Rulers in points"),
            Self::Pixel => n_!("Rulers in pixels"),
        })
    }
}

/// Document units (millimetres) in one `unit`.
fn millimetres(unit: &Unit) -> f64 {
    unit.base(96.0) / MILLIMETER.base(96.0)
}

/// The step between major ticks, in some unit: 1, 2 or 5 times a power of ten, the smallest
/// at least `wanted` long.
fn nice(wanted: f64) -> f64 {
    let power = 10.0_f64.powf(wanted.log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|factor| factor * power)
        .find(|step| *step >= wanted * (1.0 - 1e-12))
        .unwrap_or(10.0 * power)
}

/// How a ruler counts at a zoom: its unit, the document units in one of them, the step
/// between major ticks in that unit, and the decimals its numbers need.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Scale {
    pub(crate) unit: &'static Unit,
    pub(crate) per_unit: f64,
    pub(crate) step: f64,
    pub(crate) decimals: u32,
}

impl Scale {
    /// The scale of `units` at `zoom` logical pixels per document unit, its major ticks at
    /// least `spacing` logical pixels apart.
    pub(crate) fn new(units: Units, zoom: f64, spacing: f64) -> Self {
        let unit = match units {
            // Documents count in millimetres.
            Units::Document => &MILLIMETER,
            Units::Metric => {
                let step = nice(spacing / zoom);
                if step >= 1000.0 {
                    &METER
                } else if step >= 10.0 {
                    &CENTIMETER
                } else {
                    &MILLIMETER
                }
            }
            Units::Imperial => &INCH,
            Units::Typographic => &POINT,
            Units::Pixel => &PIXEL,
        };
        let per_unit = millimetres(unit);
        let step = nice(spacing / (zoom * per_unit));
        Self {
            unit,
            per_unit,
            step,
            decimals: decimals(step),
        }
    }
}

/// `value` with `decimals` decimals (at most 12), written into `digits`.
pub(crate) fn decimal(value: f64, decimals: u32, digits: &mut [u8; 32]) -> &str {
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

/// The decimals the numbers of ticks `step` apart need.
fn decimals(step: f64) -> u32 {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a few decimals"
    )]
    let decimals = (-(step.log10() + 1e-9).floor()).max(0.0) as u32;
    decimals
}

/// The whole multiples of `step` from `from` to `to`, as their indices.
fn multiples(from: f64, to: f64, step: f64) -> impl Iterator<Item = f64> {
    let first = (from / step).ceil();
    let count = ((to / step).floor() - first + 1.0).clamp(0.0, 4000.0);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to a few thousand lines"
    )]
    let count = count as u32;
    (0..count).map(move |index| first + f64::from(index))
}

/// The minor ticks of a major step.
const MINORS: u32 = 5;

/// The sides a ruler may be on, in the order of [`View::rulers`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Top,
    Left,
    Bottom,
    Right,
}

const SIDES: [Side; 4] = [Side::Top, Side::Left, Side::Bottom, Side::Right];

/// Colors and metrics of the rulers, from the `component.viewport.ruler.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RulerStyle {
    pub(crate) size: f32,
    pub(crate) background: Color,
    pub(crate) tick: Color,
    pub(crate) label: Color,
    pub(crate) cursor: Color,
    /// The minor ticks' length, a share of `size`.
    pub(crate) minor: f32,
    /// The least space between major ticks.
    pub(crate) spacing: f32,
}

/// The strip of `side`'s ruler around the canvas area `area` of `rect`.
fn strip(side: Side, rect: Rect, area: Rect) -> Rect {
    match side {
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
    }
}

/// A tick of `strip` at `at` (x for a horizontal ruler, y otherwise), `length` long from the
/// edge facing the canvas.
fn tick(side: Side, strip: Rect, at: f32, length: f32) -> Rect {
    match side {
        Side::Top => Rect::from_min_max(
            Vec2::new(at, strip.max.y - length),
            Vec2::new(at + 1.0, strip.max.y),
        ),
        Side::Bottom => Rect::from_min_max(
            Vec2::new(at, strip.min.y),
            Vec2::new(at + 1.0, strip.min.y + length),
        ),
        Side::Left => Rect::from_min_max(
            Vec2::new(strip.max.x - length, at),
            Vec2::new(strip.max.x, at + 1.0),
        ),
        Side::Right => Rect::from_min_max(
            Vec2::new(strip.min.x, at),
            Vec2::new(strip.min.x + length, at + 1.0),
        ),
    }
}

/// Draws the rulers `view` shows around its canvas area `area` of `rect`, and the corner
/// that cycles their units.
pub(crate) fn draw(
    ui: &mut Ui<'_>,
    id: WidgetId,
    rect: Rect,
    area: Rect,
    view: &mut View,
    style: &RulerStyle,
) {
    let font = ui.styles().text.detail.clone();
    let scale = Scale::new(view.units, view.camera.zoom, f64::from(style.spacing));
    let step = scale.step * scale.per_unit;
    let (from, to) = (
        view.to_document(area, area.min),
        view.to_document(area, area.max),
    );
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a step's length on screen, a few dozen pixels"
    )]
    let step_on_screen = (step * view.camera.zoom) as f32;
    let pointer = ui.pointer().filter(|point| rect.contains(*point));
    let mut digits = [0; 32];
    for (side, shown) in SIDES.into_iter().zip(view.rulers) {
        if !shown {
            continue;
        }
        let strip = strip(side, rect, area);
        ui.scene()
            .push_rect(&RectPrimitive::new(strip, style.background));
        let horizontal = matches!(side, Side::Top | Side::Bottom);
        let (start, end) = if horizontal {
            (from.x, to.x)
        } else {
            (from.y, to.y)
        };
        let thickness = if horizontal {
            strip.height()
        } else {
            strip.width()
        };
        let on_screen = |value: f64| {
            let point = if horizontal {
                Point::new(value, 0.0)
            } else {
                Point::new(0.0, value)
            };
            let screen = view.to_screen_f32(area, point);
            if horizontal { screen.x } else { screen.y }.round()
        };
        // Minor ticks first, from the step before the first major one shown.
        let minor = step / f64::from(MINORS);
        for index in multiples(start, end, minor) {
            if index.rem_euclid(f64::from(MINORS)) != 0.0 {
                let at = on_screen(index * minor);
                ui.scene().push_rect(&RectPrimitive::new(
                    tick(side, strip, at, thickness * style.minor),
                    style.tick,
                ));
            }
        }
        for index in multiples(start, end, step) {
            let at = on_screen(index * step);
            ui.scene().push_rect(&RectPrimitive::new(
                tick(side, strip, at, thickness),
                style.tick,
            ));
            let text = decimal(index * scale.step, scale.decimals, &mut digits);
            let label = if horizontal {
                Rect::from_min_max(
                    Vec2::new(at + 3.0, strip.min.y),
                    Vec2::new(at + step_on_screen, strip.max.y),
                )
            } else {
                Rect::from_min_max(
                    Vec2::new(strip.min.x + 2.0, at),
                    Vec2::new(strip.max.x, at + style.size * 0.8),
                )
            };
            ui.label(text, label, &font, style.label, Align::Start);
        }
        // Where the pointer is, along this ruler.
        if let Some(point) = pointer {
            let along = if horizontal { point.x } else { point.y };
            let (low, high) = if horizontal {
                (area.min.x, area.max.x)
            } else {
                (area.min.y, area.max.y)
            };
            if (low..high).contains(&along) {
                ui.scene().push_rect(&RectPrimitive::new(
                    tick(side, strip, along.floor(), thickness),
                    style.cursor,
                ));
            }
        }
    }
    corners(ui, id, rect, area, view, style.background);
}

/// The corners between two rulers; the first one shows the unit and cycles the units.
fn corners(
    ui: &mut Ui<'_>,
    id: WidgetId,
    rect: Rect,
    area: Rect,
    view: &mut View,
    background: Color,
) {
    let mut first = true;
    for (vertical, horizontal) in [(1, 0), (1, 2), (3, 0), (3, 2)] {
        if !(view.rulers[vertical] && view.rulers[horizontal]) {
            continue;
        }
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
        let corner = Rect::from_min_max(Vec2::new(x.0, y.0), Vec2::new(x.1, y.1));
        if first {
            first = false;
            let scale = Scale::new(view.units, view.camera.zoom, 1.0);
            if Button::new(id.with(200))
                .label(scale.unit.symbol)
                .show(ui, corner)
                .clicked
            {
                view.units = view.units.next();
            }
            ui.tooltip(id.with(200), view.units.name(), corner);
        } else {
            ui.scene()
                .push_rect(&RectPrimitive::new(corner, background));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn assert_near(got: f64, want: f64) {
        assert!(
            (got - want).abs() <= 1e-9 * want.abs().max(1.0),
            "{got} is not {want}"
        );
    }

    #[test]
    fn steps_are_round_in_the_display_unit() {
        assert_near(nice(24.0), 50.0);
        assert_near(nice(20.0), 20.0);
        assert_near(nice(0.0013), 0.002);
        assert_near(nice(5e-5), 5e-5);
        // 72 pixels apart at 100 %: 20 mm in millimetres, 1 inch... in inches, 50 points.
        let zoom = super::super::ACTUAL;
        let mm = Scale::new(Units::Document, zoom, 72.0);
        assert_eq!(mm.unit.symbol, "mm");
        assert_near(mm.step, 20.0);
        let inches = Scale::new(Units::Imperial, zoom, 72.0);
        assert_eq!(inches.unit.symbol, "in");
        assert_near(inches.step, 1.0);
        assert_near(inches.per_unit, 25.4);
        let points = Scale::new(Units::Typographic, zoom, 72.0);
        assert_near(points.step, 100.0);
        let pixels = Scale::new(Units::Pixel, zoom, 72.0);
        assert_near(pixels.step, 100.0);
        assert_near(pixels.per_unit, 25.4 / 96.0);
    }

    #[test]
    fn metric_rulers_pick_the_unit_that_keeps_numbers_short() {
        let zoom = super::super::ACTUAL;
        // 2 mm apart close in, 2 cm at 100 %, 5 m far out.
        let close = Scale::new(Units::Metric, zoom * 10.0, 72.0);
        assert_eq!((close.unit.symbol, close.step), ("mm", 2.0));
        let actual = Scale::new(Units::Metric, zoom, 72.0);
        assert_eq!(actual.unit.symbol, "cm");
        assert_near(actual.step, 2.0);
        let far = Scale::new(Units::Metric, zoom / 200.0, 72.0);
        assert_eq!(far.unit.symbol, "m");
        assert_near(far.step, 5.0);
        // The document's unit stays millimetres.
        assert_eq!(
            Scale::new(Units::Document, zoom / 200.0, 72.0).unit.symbol,
            "mm"
        );
    }

    #[test]
    fn numbers_read_back_with_the_decimals_their_step_needs() {
        let mut digits = [0; 32];
        assert_eq!(decimal(-1250.0, 0, &mut digits), "-1250");
        assert_eq!(decimal(0.0, 0, &mut digits), "0");
        assert_eq!(decimal(0.25, 2, &mut digits), "0.25");
        assert_eq!(decimal(-0.05, 2, &mut digits), "-0.05");
        assert_eq!(decimal(12.5, 1, &mut digits), "12.5");
        assert_eq!(decimals(50.0), 0);
        assert_eq!(decimals(1.0), 0);
        assert_eq!(decimals(0.2), 1);
        assert_eq!(decimals(0.1), 1);
        assert_eq!(decimals(5e-5), 5);
        let values: Vec<f64> = multiples(-12.0, 41.0, 10.0).collect();
        assert_eq!(values, [-1.0, 0.0, 1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn the_corner_cycles_every_unit() {
        let mut units = Units::default();
        let mut seen = Vec::new();
        for _ in 0..Units::ALL.len() {
            seen.push(units);
            units = units.next();
        }
        assert_eq!(seen, Units::ALL);
        assert_eq!(units, Units::Document);
    }
}
