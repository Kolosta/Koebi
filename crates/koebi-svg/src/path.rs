//! SVG path data (`d`), every command absolute or relative, implicit repetitions and the
//! compact number forms (`1.5.5`, `-1-2`, `1e-3`), arcs turned into cubics; and the basic
//! shapes (`rect` with its corners, `circle`, `ellipse`, `line`, `polyline`, `polygon`) as
//! outlines.

use core::f32::consts::{FRAC_PI_2, TAU};

use crate::error::{Error, Result};
use crate::geometry::{Outline, Point, Segment, Subpath};

/// The numbers of `text`, separated by spaces or commas, in the compact forms SVG allows.
pub(crate) fn numbers(text: &str) -> Option<Vec<f32>> {
    let mut scanner = Scanner::new(text);
    let mut out = Vec::new();
    loop {
        scanner.skip_separators();
        if scanner.done() {
            return Some(out);
        }
        out.push(scanner.number()?);
    }
}

struct Scanner<'a> {
    bytes: &'a [u8],
    text: &'a str,
    at: usize,
}

impl<'a> Scanner<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            bytes: text.as_bytes(),
            text,
            at: 0,
        }
    }

    fn done(&self) -> bool {
        self.at >= self.bytes.len()
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn skip_separators(&mut self) {
        while let Some(byte) = self.peek() {
            if byte.is_ascii_whitespace() || byte == b',' {
                self.at += 1;
            } else {
                break;
            }
        }
    }

    /// A number: a sign, digits, a point and digits, an exponent.
    fn number(&mut self) -> Option<f32> {
        let start = self.at;
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.at += 1;
        }
        let mut digits = false;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.at += 1;
            digits = true;
        }
        if self.peek() == Some(b'.') {
            self.at += 1;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.at += 1;
                digits = true;
            }
        }
        if !digits {
            self.at = start;
            return None;
        }
        if matches!(self.peek(), Some(b'e' | b'E'))
            && matches!(self.bytes.get(self.at + 1), Some(b'0'..=b'9' | b'+' | b'-'))
        {
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.at += 1;
            }
        }
        self.text[start..self.at].parse().ok()
    }

    /// An arc's flag: `0` or `1`, which may touch what follows (`a1 1 0 01 2 3`).
    fn flag(&mut self) -> Option<bool> {
        self.skip_separators();
        match self.peek()? {
            b'0' => {
                self.at += 1;
                Some(false)
            }
            b'1' => {
                self.at += 1;
                Some(true)
            }
            _ => None,
        }
    }
}

/// Reads path data into an outline.
pub(crate) fn parse_path(data: &str, line: usize) -> Result<Outline> {
    let fail = |what: &str| Error::at(line, format!("malformed path data: {what}"));
    let mut scanner = Scanner::new(data);
    let mut outline = Outline::default();
    let mut current = Subpath::default();
    let mut start = Point::default();
    let mut at = Point::default();
    // The reflections of the last control points, for S and T.
    let mut last_cubic: Option<Point> = None;
    let mut last_quad: Option<Point> = None;
    let mut command: Option<u8> = None;
    let mut moved = false;
    let finish = |outline: &mut Outline, current: &mut Subpath| {
        if current.segments.is_empty() {
            current.closed = false;
        } else {
            outline.subpaths.push(core::mem::take(current));
        }
    };
    loop {
        scanner.skip_separators();
        let Some(byte) = scanner.peek() else {
            break;
        };
        if byte.is_ascii_alphabetic() {
            scanner.at += 1;
            command = Some(byte);
        } else if command.is_none() {
            return Err(fail("it does not start with a command"));
        }
        let name = command.ok_or_else(|| fail("no command"))?;
        if !moved && !matches!(name, b'M' | b'm') {
            return Err(fail("it must start with a move (M)"));
        }
        moved = true;
        let relative = name.is_ascii_lowercase();
        let origin = if relative { at } else { Point::default() };
        let mut number = || {
            scanner.skip_separators();
            scanner.number().ok_or_else(|| fail("a number is missing"))
        };
        match name.to_ascii_uppercase() {
            b'M' => {
                let point = origin.add(Point::new(number()?, number()?));
                finish(&mut outline, &mut current);
                start = point;
                at = point;
                // Pairs after a move are lines.
                command = Some(if relative { b'l' } else { b'L' });
                last_cubic = None;
                last_quad = None;
            }
            b'Z' => {
                if !current.segments.is_empty() {
                    current.closed = true;
                }
                finish(&mut outline, &mut current);
                at = start;
                last_cubic = None;
                last_quad = None;
                // Z takes no numbers: a number after it starts nothing.
                command = None;
            }
            b'L' => {
                let point = origin.add(Point::new(number()?, number()?));
                current.segments.push(Segment::Line(at, point));
                at = point;
                last_cubic = None;
                last_quad = None;
            }
            b'H' => {
                let x = if relative {
                    at.x + number()?
                } else {
                    number()?
                };
                let point = Point::new(x, at.y);
                current.segments.push(Segment::Line(at, point));
                at = point;
                last_cubic = None;
                last_quad = None;
            }
            b'V' => {
                let y = if relative {
                    at.y + number()?
                } else {
                    number()?
                };
                let point = Point::new(at.x, y);
                current.segments.push(Segment::Line(at, point));
                at = point;
                last_cubic = None;
                last_quad = None;
            }
            b'C' => {
                let c1 = origin.add(Point::new(number()?, number()?));
                let c2 = origin.add(Point::new(number()?, number()?));
                let end = origin.add(Point::new(number()?, number()?));
                current.segments.push(Segment::Cubic(at, c1, c2, end));
                last_cubic = Some(c2);
                last_quad = None;
                at = end;
            }
            b'S' => {
                let c1 = last_cubic.map_or(at, |c| at.add(at.sub(c)));
                let c2 = origin.add(Point::new(number()?, number()?));
                let end = origin.add(Point::new(number()?, number()?));
                current.segments.push(Segment::Cubic(at, c1, c2, end));
                last_cubic = Some(c2);
                last_quad = None;
                at = end;
            }
            b'Q' => {
                let control = origin.add(Point::new(number()?, number()?));
                let end = origin.add(Point::new(number()?, number()?));
                current.segments.push(Segment::Quad(at, control, end));
                last_quad = Some(control);
                last_cubic = None;
                at = end;
            }
            b'T' => {
                let control = last_quad.map_or(at, |c| at.add(at.sub(c)));
                let end = origin.add(Point::new(number()?, number()?));
                current.segments.push(Segment::Quad(at, control, end));
                last_quad = Some(control);
                last_cubic = None;
                at = end;
            }
            b'A' => {
                let rx = number()?;
                let ry = number()?;
                let rotation = number()?;
                let large = scanner
                    .flag()
                    .ok_or_else(|| fail("an arc flag is missing"))?;
                let sweep = scanner
                    .flag()
                    .ok_or_else(|| fail("an arc flag is missing"))?;
                let mut number = || {
                    scanner.skip_separators();
                    scanner.number().ok_or_else(|| fail("a number is missing"))
                };
                let end = origin.add(Point::new(number()?, number()?));
                arc(
                    at,
                    rx,
                    ry,
                    rotation,
                    large,
                    sweep,
                    end,
                    &mut current.segments,
                );
                at = end;
                last_cubic = None;
                last_quad = None;
            }
            other => {
                return Err(fail(&format!("unknown command {:?}", char::from(other))));
            }
        }
    }
    finish(&mut outline, &mut current);
    Ok(outline)
}

/// An elliptical arc from `from` to `to`, as SVG's arc command, added as cubics.
#[allow(
    clippy::too_many_arguments,
    reason = "the parameters of SVG's arc command"
)]
fn arc(
    from: Point,
    rx: f32,
    ry: f32,
    rotation: f32,
    large: bool,
    sweep: bool,
    to: Point,
    out: &mut Vec<Segment>,
) {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if (from.sub(to)).length() < 1e-9 {
        return;
    }
    if rx < 1e-9 || ry < 1e-9 {
        out.push(Segment::Line(from, to));
        return;
    }
    // The endpoint to center conversion of the SVG specification (appendix B.2.4).
    let (sin, cos) = rotation.to_radians().sin_cos();
    let half = from.sub(to).scale(0.5);
    let p = Point::new(cos * half.x + sin * half.y, -sin * half.x + cos * half.y);
    let lambda = (p.x * p.x) / (rx * rx) + (p.y * p.y) / (ry * ry);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let numerator = (rx * rx * ry * ry - rx * rx * p.y * p.y - ry * ry * p.x * p.x).max(0.0);
    let denominator = rx * rx * p.y * p.y + ry * ry * p.x * p.x;
    let mut factor = (numerator / denominator.max(1e-12)).sqrt();
    if large == sweep {
        factor = -factor;
    }
    let center_rotated = Point::new(factor * rx * p.y / ry, -factor * ry * p.x / rx);
    let middle = from.add(to).scale(0.5);
    let center = Point::new(
        cos * center_rotated.x - sin * center_rotated.y + middle.x,
        sin * center_rotated.x + cos * center_rotated.y + middle.y,
    );
    let angle = |u: Point, v: Point| {
        let value = u.cross(v).atan2(u.dot(v));
        if value.is_finite() { value } else { 0.0 }
    };
    let start_vector = Point::new((p.x - center_rotated.x) / rx, (p.y - center_rotated.y) / ry);
    let end_vector = Point::new(
        (-p.x - center_rotated.x) / rx,
        (-p.y - center_rotated.y) / ry,
    );
    let start_angle = angle(Point::new(1.0, 0.0), start_vector);
    let mut delta = angle(start_vector, end_vector) % TAU;
    if !sweep && delta > 0.0 {
        delta -= TAU;
    } else if sweep && delta < 0.0 {
        delta += TAU;
    }
    // Quarter turns at most, each a cubic.
    let pieces = (delta.abs() / FRAC_PI_2).ceil().max(1.0);
    let step = delta / pieces;
    let kappa = 4.0 / 3.0 * (step / 4.0).tan();
    let point_at = |theta: f32| {
        let (s, c) = theta.sin_cos();
        let x = rx * c;
        let y = ry * s;
        Point::new(cos * x - sin * y + center.x, sin * x + cos * y + center.y)
    };
    let tangent_at = |theta: f32| {
        let (s, c) = theta.sin_cos();
        let x = -rx * s;
        let y = ry * c;
        Point::new(cos * x - sin * y, sin * x + cos * y)
    };
    let mut theta = start_angle;
    let mut previous = from;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "four pieces at most"
    )]
    let count = pieces as usize;
    for index in 0..count {
        let next = theta + step;
        let end = if index + 1 == count {
            to
        } else {
            point_at(next)
        };
        let c1 = previous.add(tangent_at(theta).scale(kappa));
        let c2 = end.sub(tangent_at(next).scale(kappa));
        out.push(Segment::Cubic(previous, c1, c2, end));
        previous = end;
        theta = next;
    }
}

/// A rectangle, its corners rounded by `rx` and `ry` (each defaulting to the other).
pub(crate) fn rect(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    rx: Option<f32>,
    ry: Option<f32>,
) -> Outline {
    let rx = rx.or(ry).unwrap_or(0.0).clamp(0.0, width * 0.5);
    let ry = ry.or(Some(rx)).unwrap_or(0.0).clamp(0.0, height * 0.5);
    let mut segments = Vec::new();
    if rx <= 0.0 || ry <= 0.0 {
        let corners = [
            Point::new(x, y),
            Point::new(x + width, y),
            Point::new(x + width, y + height),
            Point::new(x, y + height),
        ];
        for index in 0..4 {
            segments.push(Segment::Line(corners[index], corners[(index + 1) % 4]));
        }
    } else {
        let at = |px: f32, py: f32| Point::new(px, py);
        let corner = |from: Point, to: Point, rx: f32, ry: f32, segments: &mut Vec<Segment>| {
            arc(from, rx, ry, 0.0, false, true, to, segments);
        };
        segments.push(Segment::Line(at(x + rx, y), at(x + width - rx, y)));
        corner(
            at(x + width - rx, y),
            at(x + width, y + ry),
            rx,
            ry,
            &mut segments,
        );
        segments.push(Segment::Line(
            at(x + width, y + ry),
            at(x + width, y + height - ry),
        ));
        corner(
            at(x + width, y + height - ry),
            at(x + width - rx, y + height),
            rx,
            ry,
            &mut segments,
        );
        segments.push(Segment::Line(
            at(x + width - rx, y + height),
            at(x + rx, y + height),
        ));
        corner(
            at(x + rx, y + height),
            at(x, y + height - ry),
            rx,
            ry,
            &mut segments,
        );
        segments.push(Segment::Line(at(x, y + height - ry), at(x, y + ry)));
        corner(at(x, y + ry), at(x + rx, y), rx, ry, &mut segments);
    }
    Outline {
        subpaths: vec![Subpath {
            segments,
            closed: true,
        }],
    }
}

/// An ellipse centered on `(cx, cy)`.
pub(crate) fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Outline {
    let mut segments = Vec::new();
    let right = Point::new(cx + rx, cy);
    let left = Point::new(cx - rx, cy);
    arc(right, rx, ry, 0.0, false, true, left, &mut segments);
    arc(left, rx, ry, 0.0, false, true, right, &mut segments);
    Outline {
        subpaths: vec![Subpath {
            segments,
            closed: true,
        }],
    }
}

/// A polyline through `points`, closed for a polygon.
pub(crate) fn polyline(points: &[f32], closed: bool, line: usize) -> Result<Outline> {
    if !points.len().is_multiple_of(2) {
        return Err(Error::at(line, "points come in pairs"));
    }
    let points: Vec<Point> = points
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&[x, y]| Point::new(x, y))
        .collect();
    let segments = points
        .windows(2)
        .map(|pair| Segment::Line(pair[0], pair[1]))
        .collect();
    Ok(Outline {
        subpaths: vec![Subpath { segments, closed }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_in_their_compact_forms() {
        assert_eq!(
            numbers("1.5.5-2,3e1 -.5"),
            Some(vec![1.5, 0.5, -2.0, 30.0, -0.5])
        );
        assert_eq!(numbers("1 x"), None);
    }

    #[test]
    fn every_command_reads_absolute_and_relative() {
        let outline = parse_path(
            "M10 10h10v10H10zm5 5l2 2 2-2c0 1 1 1 1 0s1 1 1 0q1 1 2 0t2 0a1 1 0 0 1 2 0",
            1,
        )
        .unwrap();
        assert_eq!(outline.subpaths.len(), 2);
        assert!(outline.subpaths[0].closed);
        assert_eq!(outline.subpaths[0].segments.len(), 3);
        let second = &outline.subpaths[1];
        assert!(!second.closed);
        // The relative move after Z starts from the subpath's start.
        assert_eq!(second.segments[0].start(), Point::new(15.0, 15.0));
        assert_eq!(second.segments[1].end(), Point::new(19.0, 15.0));
        // The arc ends where it should.
        let end = second.segments.last().unwrap().end();
        assert!(
            (end.x - 27.0).abs() < 1e-3 && (end.y - 15.0).abs() < 1e-3,
            "{end:?}"
        );
    }

    #[test]
    fn arcs_pass_through_the_ellipse() {
        let mut segments = Vec::new();
        arc(
            Point::new(0.0, 0.0),
            1.0,
            1.0,
            0.0,
            false,
            true,
            Point::new(2.0, 0.0),
            &mut segments,
        );
        // A half circle above or below the chord, of radius 1 about (1, 0).
        for segment in &segments {
            let end = segment.end();
            assert!(((end.sub(Point::new(1.0, 0.0))).length() - 1.0).abs() < 1e-3);
        }
        assert!(parse_path("L 1 1", 1).is_err());
        assert!(parse_path("M 1", 1).is_err());
        assert!(parse_path("M1 1 X 2", 1).is_err());
    }

    #[test]
    fn shapes_become_closed_outlines() {
        assert_eq!(
            rect(0.0, 0.0, 4.0, 2.0, None, None).subpaths[0]
                .segments
                .len(),
            4
        );
        // Rounded corners: four sides and four arcs.
        assert!(
            rect(0.0, 0.0, 4.0, 2.0, Some(1.0), None).subpaths[0]
                .segments
                .len()
                >= 8
        );
        assert!(ellipse(0.0, 0.0, 2.0, 1.0).subpaths[0].closed);
        assert!(polyline(&[0.0, 0.0, 1.0], false, 1).is_err());
        assert_eq!(
            polyline(&[0.0, 0.0, 1.0, 0.0, 1.0, 1.0], true, 1)
                .unwrap()
                .subpaths[0]
                .segments
                .len(),
            2
        );
    }
}
