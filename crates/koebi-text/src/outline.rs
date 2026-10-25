//! Glyph outlines: TrueType contours as move, line and quadratic curve commands.

use koebi_core::{Rect, Vec2};

use crate::font::{Font, GlyphId};
use crate::read::{Reader, Tag, i16_at};
use crate::{Error, Result};

/// One command of an outline, in font units, y up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCommand {
    /// Starts a contour.
    MoveTo(Vec2),
    /// A straight segment to a point.
    LineTo(Vec2),
    /// A quadratic curve through a control point to an end point.
    QuadTo(Vec2, Vec2),
    /// Closes the contour (its last command ends on its first point).
    Close,
}

/// Composite glyphs nest at most this deep: deeper ones are refused, loops included.
const MAX_DEPTH: u8 = 8;
/// Points one glyph may total, components included: protects memory from hostile fonts.
const MAX_POINTS: usize = 1 << 16;

// Simple glyph flags.
const ON_CURVE: u8 = 0x01;
const X_SHORT: u8 = 0x02;
const Y_SHORT: u8 = 0x04;
const REPEAT: u8 = 0x08;
const X_SAME_OR_POSITIVE: u8 = 0x10;
const Y_SAME_OR_POSITIVE: u8 = 0x20;

// Composite glyph flags.
const ARGS_ARE_WORDS: u16 = 0x0001;
const ARGS_ARE_XY_VALUES: u16 = 0x0002;
const HAS_SCALE: u16 = 0x0008;
const MORE_COMPONENTS: u16 = 0x0020;
const HAS_X_AND_Y_SCALE: u16 = 0x0040;
const HAS_TWO_BY_TWO: u16 = 0x0080;
const SCALED_COMPONENT_OFFSET: u16 = 0x0800;

/// The outline of a glyph, in font units.
///
/// Reuse one across glyphs: [`Outline::load`] clears it and keeps its storage.
#[derive(Clone, Debug, Default)]
pub struct Outline {
    commands: Vec<PathCommand>,
    // Scratch storage for the simple glyph being read.
    points: Vec<(Vec2, bool)>,
    flags: Vec<u8>,
    ends: Vec<u16>,
}

/// An affine transform of a composite glyph's component.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Transform {
    xx: f32,
    xy: f32,
    yx: f32,
    yy: f32,
    dx: f32,
    dy: f32,
}

impl Transform {
    const IDENTITY: Self = Self {
        xx: 1.0,
        xy: 0.0,
        yx: 0.0,
        yy: 1.0,
        dx: 0.0,
        dy: 0.0,
    };

    fn apply(&self, point: Vec2) -> Vec2 {
        Vec2::new(
            self.xx * point.x + self.yx * point.y + self.dx,
            self.xy * point.x + self.yy * point.y + self.dy,
        )
    }

    /// `inner` first, then `self`.
    fn then(&self, inner: &Self) -> Self {
        let offset = self.apply(Vec2::new(inner.dx, inner.dy));
        Self {
            xx: self.xx * inner.xx + self.yx * inner.xy,
            xy: self.xy * inner.xx + self.yy * inner.xy,
            yx: self.xx * inner.yx + self.yx * inner.yy,
            yy: self.xy * inner.yx + self.yy * inner.yy,
            dx: offset.x,
            dy: offset.y,
        }
    }
}

impl Outline {
    /// An empty outline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the outline with `glyph`'s, components resolved.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownGlyph`] outside the font, [`Error::Malformed`] for broken glyph
    /// data and [`Error::TooComplex`] beyond the nesting and point limits. The outline is left
    /// empty on error.
    pub fn load(&mut self, font: &Font, glyph: GlyphId) -> Result<()> {
        self.commands.clear();
        let mut budget = MAX_POINTS;
        let loaded = self.load_glyph(font, glyph, &Transform::IDENTITY, 0, &mut budget);
        if loaded.is_err() {
            self.commands.clear();
        }
        loaded
    }

    /// The commands, contour after contour.
    pub fn commands(&self) -> &[PathCommand] {
        &self.commands
    }

    /// `true` for glyphs without outline (spaces).
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// The smallest rectangle holding every point, control points included, in font units.
    pub fn bounds(&self) -> Option<Rect> {
        let mut points = self.commands.iter().flat_map(|command| {
            let (first, second) = match *command {
                PathCommand::MoveTo(point) | PathCommand::LineTo(point) => (Some(point), None),
                PathCommand::QuadTo(control, point) => (Some(control), Some(point)),
                PathCommand::Close => (None, None),
            };
            first.into_iter().chain(second)
        });
        let first = points.next()?;
        let (min, max) = points.fold((first, first), |(min, max), point| {
            (min.min(point), max.max(point))
        });
        Some(Rect::from_min_max(min, max))
    }

    fn load_glyph(
        &mut self,
        font: &Font,
        glyph: GlyphId,
        transform: &Transform,
        depth: u8,
        budget: &mut usize,
    ) -> Result<()> {
        let data = font.glyph_data(glyph)?;
        if data.is_empty() {
            return Ok(());
        }
        let contours = i16_at(data, 0).ok_or(malformed("truncated glyph header"))?;
        match u16::try_from(contours) {
            Ok(contours) => self.load_simple(data, usize::from(contours), transform, budget),
            Err(_) => self.load_composite(font, data, transform, depth, budget),
        }
    }

    fn load_simple(
        &mut self,
        data: &[u8],
        contours: usize,
        transform: &Transform,
        budget: &mut usize,
    ) -> Result<()> {
        let truncated = || malformed("truncated simple glyph");
        let mut reader = Reader::at(data, 10);
        self.ends.clear();
        for _ in 0..contours {
            let end = reader.u16().ok_or_else(truncated)?;
            if self.ends.last().is_some_and(|previous| end <= *previous) {
                return Err(malformed("contour ends go backwards"));
            }
            self.ends.push(end);
        }
        let count = self.ends.last().map_or(0, |end| usize::from(*end) + 1);
        *budget = budget.checked_sub(count).ok_or(Error::TooComplex)?;
        let instructions = reader.u16().ok_or_else(truncated)?;
        reader
            .skip(usize::from(instructions))
            .ok_or_else(truncated)?;

        self.flags.clear();
        while self.flags.len() < count {
            let flag = reader.u8().ok_or_else(truncated)?;
            self.flags.push(flag);
            if flag & REPEAT != 0 {
                let repeat = usize::from(reader.u8().ok_or_else(truncated)?);
                let repeat = repeat.min(count - self.flags.len());
                self.flags.extend(std::iter::repeat_n(flag, repeat));
            }
        }
        self.points.clear();
        let mut x = 0_i32;
        for &flag in &self.flags {
            x = x.saturating_add(delta(&mut reader, flag, X_SHORT, X_SAME_OR_POSITIVE)?);
            self.points
                .push((Vec2::new(x as f32, 0.0), flag & ON_CURVE != 0));
        }
        let mut y = 0_i32;
        for (point, &flag) in self.points.iter_mut().zip(&self.flags) {
            y = y.saturating_add(delta(&mut reader, flag, Y_SHORT, Y_SAME_OR_POSITIVE)?);
            point.0 = transform.apply(Vec2::new(point.0.x, y as f32));
        }

        let mut start = 0;
        for &end in &self.ends {
            let end = usize::from(end);
            emit_contour(&mut self.commands, &self.points[start..=end]);
            start = end + 1;
        }
        Ok(())
    }

    fn load_composite(
        &mut self,
        font: &Font,
        data: &[u8],
        transform: &Transform,
        depth: u8,
        budget: &mut usize,
    ) -> Result<()> {
        if depth >= MAX_DEPTH {
            return Err(Error::TooComplex);
        }
        let truncated = || malformed("truncated composite glyph");
        let mut reader = Reader::at(data, 10);
        loop {
            let flags = reader.u16().ok_or_else(truncated)?;
            let glyph = GlyphId(reader.u16().ok_or_else(truncated)?);
            let (first, second) =
                match (flags & ARGS_ARE_WORDS != 0, flags & ARGS_ARE_XY_VALUES != 0) {
                    (true, _) => (
                        f32::from(reader.i16().ok_or_else(truncated)?),
                        f32::from(reader.i16().ok_or_else(truncated)?),
                    ),
                    (false, true) => (
                        f32::from(reader.i8().ok_or_else(truncated)?),
                        f32::from(reader.i8().ok_or_else(truncated)?),
                    ),
                    (false, false) => (
                        f32::from(reader.u8().ok_or_else(truncated)?),
                        f32::from(reader.u8().ok_or_else(truncated)?),
                    ),
                };
            let mut component = Transform::IDENTITY;
            let mut scale = || reader.i16().map(f2dot14).ok_or_else(truncated);
            if flags & HAS_SCALE != 0 {
                component.xx = scale()?;
                component.yy = component.xx;
            } else if flags & HAS_X_AND_Y_SCALE != 0 {
                component.xx = scale()?;
                component.yy = scale()?;
            } else if flags & HAS_TWO_BY_TWO != 0 {
                component.xx = scale()?;
                component.xy = scale()?;
                component.yx = scale()?;
                component.yy = scale()?;
            }
            // Components anchored on matching points (rare) are placed at their origin.
            if flags & ARGS_ARE_XY_VALUES != 0 {
                let offset = Vec2::new(first, second);
                let offset = if flags & SCALED_COMPONENT_OFFSET != 0 {
                    component.apply(offset)
                } else {
                    offset
                };
                component.dx = offset.x;
                component.dy = offset.y;
            }
            let combined = transform.then(&component);
            self.load_glyph(font, glyph, &combined, depth + 1, budget)?;
            if flags & MORE_COMPONENTS == 0 {
                return Ok(());
            }
        }
    }
}

fn malformed(reason: &'static str) -> Error {
    Error::Malformed {
        table: Tag::GLYF,
        reason,
    }
}

fn f2dot14(value: i16) -> f32 {
    f32::from(value) / 16384.0
}

/// One coordinate delta of a simple glyph.
fn delta(reader: &mut Reader<'_>, flag: u8, short: u8, same_or_positive: u8) -> Result<i32> {
    let truncated = || malformed("truncated coordinates");
    Ok(if flag & short != 0 {
        let magnitude = i32::from(reader.u8().ok_or_else(truncated)?);
        if flag & same_or_positive != 0 {
            magnitude
        } else {
            -magnitude
        }
    } else if flag & same_or_positive != 0 {
        0
    } else {
        i32::from(reader.i16().ok_or_else(truncated)?)
    })
}

/// Emits one TrueType contour: on-curve points joined by lines, off-curve points as quadratic
/// controls, with an implied on-curve point between two consecutive controls.
fn emit_contour(commands: &mut Vec<PathCommand>, points: &[(Vec2, bool)]) {
    let (Some(&(first, first_on)), Some(&(last, last_on))) = (points.first(), points.last()) else {
        return;
    };
    if points.len() < 2 {
        return;
    }
    let midpoint = |a: Vec2, b: Vec2| (a + b) * 0.5;
    let (start, rest) = if first_on {
        (first, &points[1..])
    } else if last_on {
        (last, &points[..points.len() - 1])
    } else {
        (midpoint(first, last), points)
    };
    commands.push(PathCommand::MoveTo(start));
    let mut control: Option<Vec2> = None;
    for &(point, on_curve) in rest {
        match (on_curve, control) {
            (true, Some(previous)) => {
                commands.push(PathCommand::QuadTo(previous, point));
                control = None;
            }
            (true, None) => commands.push(PathCommand::LineTo(point)),
            (false, Some(previous)) => {
                commands.push(PathCommand::QuadTo(previous, midpoint(previous, point)));
                control = Some(point);
            }
            (false, None) => control = Some(point),
        }
    }
    commands.push(match control {
        Some(previous) => PathCommand::QuadTo(previous, start),
        None => PathCommand::LineTo(start),
    });
    commands.push(PathCommand::Close);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on(x: f32, y: f32) -> (Vec2, bool) {
        (Vec2::new(x, y), true)
    }

    fn off(x: f32, y: f32) -> (Vec2, bool) {
        (Vec2::new(x, y), false)
    }

    #[test]
    fn contours_imply_on_curve_points_between_controls() {
        let mut commands = Vec::new();
        emit_contour(
            &mut commands,
            &[on(0.0, 0.0), off(10.0, 0.0), off(10.0, 10.0)],
        );
        let p = Vec2::new;
        assert_eq!(
            commands,
            [
                PathCommand::MoveTo(p(0.0, 0.0)),
                PathCommand::QuadTo(p(10.0, 0.0), p(10.0, 5.0)),
                PathCommand::QuadTo(p(10.0, 10.0), p(0.0, 0.0)),
                PathCommand::Close,
            ]
        );
    }

    #[test]
    fn contours_without_on_curve_points_start_between_the_last_and_first() {
        let mut commands = Vec::new();
        emit_contour(
            &mut commands,
            &[
                off(0.0, 0.0),
                off(10.0, 0.0),
                off(10.0, 10.0),
                off(0.0, 10.0),
            ],
        );
        let p = Vec2::new;
        assert_eq!(commands[0], PathCommand::MoveTo(p(0.0, 5.0)));
        assert_eq!(commands.len(), 6);
        assert_eq!(commands[4], PathCommand::QuadTo(p(0.0, 10.0), p(0.0, 5.0)));
    }

    #[test]
    fn single_points_draw_nothing() {
        let mut commands = Vec::new();
        emit_contour(&mut commands, &[on(1.0, 1.0)]);
        emit_contour(&mut commands, &[]);
        assert_eq!(commands, Vec::<PathCommand>::new());
    }

    #[test]
    fn transforms_compose_inner_first() {
        let scale = Transform {
            xx: 2.0,
            yy: 2.0,
            ..Transform::IDENTITY
        };
        let shift = Transform {
            dx: 10.0,
            ..Transform::IDENTITY
        };
        // Shift, then scale: the offset is scaled too.
        let point = scale.then(&shift).apply(Vec2::new(1.0, 1.0));
        assert_eq!(point, Vec2::new(22.0, 2.0));
        assert!((f2dot14(0x4000) - 1.0).abs() < f32::EPSILON);
        assert!((f2dot14(-0x2000) + 0.5).abs() < f32::EPSILON);
    }
}
