//! Scene description: what to draw this frame, in painter's order.

use core::ops::Range;

use koebi_core::{Color, Rect, Vec2};
use koebi_gpu::Pod;

/// A rectangle with optional rounded corners and an inner border.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RectPrimitive {
    /// Bounds in logical pixels.
    pub rect: Rect,
    /// Fill color.
    pub fill: Color,
    /// Corner radius in logical pixels (clamped to half the smallest side).
    pub corner_radius: f32,
    /// Border width in logical pixels, drawn inside the bounds. `0` disables the border.
    pub border_width: f32,
    /// Border color.
    pub border_color: Color,
}

impl RectPrimitive {
    /// A filled rectangle with square corners and no border.
    pub const fn new(rect: Rect, fill: Color) -> Self {
        Self {
            rect,
            fill,
            corner_radius: 0.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        }
    }

    /// Sets the corner radius.
    #[must_use]
    pub const fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Adds an inner border.
    #[must_use]
    pub const fn with_border(mut self, width: f32, color: Color) -> Self {
        self.border_width = width;
        self.border_color = color;
        self
    }
}

/// A straight segment with round caps: strokes of check marks, chevrons and icons.
///
/// Caps extend half the width beyond both ends, so segments sharing an end point join with a
/// round corner, and a zero-length segment is a dot. Where translucent segments overlap, the
/// overlap blends twice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinePrimitive {
    /// Start point in logical pixels.
    pub from: Vec2,
    /// End point in logical pixels.
    pub to: Vec2,
    /// Stroke width in logical pixels.
    pub width: f32,
    /// Color.
    pub color: Color,
}

impl LinePrimitive {
    /// A segment from `from` to `to`.
    pub const fn new(from: Vec2, to: Vec2, width: f32, color: Color) -> Self {
        Self {
            from,
            to,
            width,
            color,
        }
    }
}

/// GPU layout of one rectangle (vertex buffer, per-instance step).
///
/// Must match the inputs of `shaders/rect.vert`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub(crate) struct RectInstance {
    /// `x, y, width, height` in logical pixels.
    pub(crate) rect: [f32; 4],
    /// Premultiplied fill.
    pub(crate) fill: [f32; 4],
    /// Premultiplied border color.
    pub(crate) border_color: [f32; 4],
    /// `corner_radius, border_width, 0, 0` in logical pixels.
    pub(crate) params: [f32; 4],
}

// SAFETY: `repr(C)` struct of four `[f32; 4]` arrays: 64 bytes with no padding, and every bit
// pattern is a valid value.
unsafe impl Pod for RectInstance {}
const _: () = assert!(size_of::<RectInstance>() == 64);

impl RectInstance {
    pub(crate) const STRIDE: u32 = 64;

    fn from_primitive(primitive: &RectPrimitive) -> Self {
        let r = primitive.rect;
        Self {
            rect: [r.min.x, r.min.y, r.width(), r.height()],
            fill: primitive.fill.premultiplied(),
            border_color: primitive.border_color.premultiplied(),
            params: [
                primitive.corner_radius.max(0.0),
                primitive.border_width.max(0.0),
                0.0,
                0.0,
            ],
        }
    }
}

/// GPU layout of one segment (vertex buffer, per-instance step).
///
/// Must match the inputs of `shaders/line.vert`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub(crate) struct LineInstance {
    /// `from.x, from.y, to.x, to.y` in logical pixels.
    pub(crate) points: [f32; 4],
    /// Premultiplied color.
    pub(crate) color: [f32; 4],
    /// `half_width, 0, 0, 0` in logical pixels.
    pub(crate) params: [f32; 4],
}

// SAFETY: `repr(C)` struct of three `[f32; 4]` arrays: 48 bytes with no padding, and every bit
// pattern is a valid value.
unsafe impl Pod for LineInstance {}
const _: () = assert!(size_of::<LineInstance>() == 48);

impl LineInstance {
    pub(crate) const STRIDE: u32 = 48;

    fn from_primitive(primitive: &LinePrimitive) -> Self {
        let (from, to) = (primitive.from, primitive.to);
        Self {
            points: [from.x, from.y, to.x, to.y],
            color: primitive.color.premultiplied(),
            params: [primitive.width * 0.5, 0.0, 0.0, 0.0],
        }
    }
}

/// The primitive types, one pipeline each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PrimitiveKind {
    Rect,
    Line,
}

/// Consecutive primitives of one type: one instanced draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Batch {
    pub(crate) kind: PrimitiveKind,
    /// First instance, in the instances of `kind`.
    start: u32,
    /// One past the last instance.
    end: u32,
}

impl Batch {
    /// The instances of the batch, in the instances of its kind.
    pub(crate) fn instances(self) -> Range<u32> {
        self.start..self.end
    }
}

/// The 2D primitives of one frame, drawn in insertion order (later on top).
///
/// Reuse one scene across frames with [`Scene::clear`]: storage is kept, so steady-state
/// frames do not allocate.
#[derive(Clone, Debug, Default)]
pub struct Scene {
    clear_color: Color,
    rects: Vec<RectInstance>,
    lines: Vec<LineInstance>,
    /// Painter's order across types: runs of rectangles and segments.
    batches: Vec<Batch>,
}

impl Scene {
    /// An empty scene cleared to transparent black.
    pub fn new() -> Self {
        Self::default()
    }

    /// Removes every primitive, keeping allocated storage.
    pub fn clear(&mut self) {
        self.rects.clear();
        self.lines.clear();
        self.batches.clear();
    }

    /// Sets the color the target is cleared to before drawing.
    pub fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }

    /// The clear color.
    pub fn clear_color(&self) -> Color {
        self.clear_color
    }

    /// Appends a rectangle. Empty or fully transparent rectangles are culled.
    pub fn push_rect(&mut self, primitive: &RectPrimitive) {
        let invisible = primitive.fill.a <= 0.0
            && (primitive.border_width <= 0.0 || primitive.border_color.a <= 0.0);
        if primitive.rect.is_empty() || invisible {
            return;
        }
        self.rects.push(RectInstance::from_primitive(primitive));
        self.extend_batch(PrimitiveKind::Rect, self.rects.len());
    }

    /// Appends a segment. Segments without width or color are culled, and so are segments
    /// with a non-finite coordinate or width, which cannot be drawn.
    pub fn push_line(&mut self, primitive: &LinePrimitive) {
        let (from, to, width) = (primitive.from, primitive.to, primitive.width);
        let finite = [from.x, from.y, to.x, to.y, width]
            .iter()
            .all(|v| v.is_finite());
        if !finite || width <= 0.0 || primitive.color.a <= 0.0 {
            return;
        }
        self.lines.push(LineInstance::from_primitive(primitive));
        self.extend_batch(PrimitiveKind::Line, self.lines.len());
    }

    /// Records the instance just pushed (`count` instances of `kind` so far).
    fn extend_batch(&mut self, kind: PrimitiveKind, count: usize) {
        // More than 4 billion primitives cannot be drawn anyway: saturate.
        let end = u32::try_from(count).unwrap_or(u32::MAX);
        match self.batches.last_mut() {
            Some(batch) if batch.kind == kind => batch.end = end,
            _ => self.batches.push(Batch {
                kind,
                start: end - 1,
                end,
            }),
        }
    }

    /// Number of rectangles.
    pub fn rect_count(&self) -> usize {
        self.rects.len()
    }

    /// Number of segments.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Number of draw calls the scene needs: one per run of consecutive primitives of the
    /// same type.
    pub fn draw_count(&self) -> usize {
        self.batches.len()
    }

    /// `true` when nothing will be drawn (the target is still cleared).
    pub fn is_empty(&self) -> bool {
        self.batches.is_empty()
    }

    pub(crate) fn rect_instances(&self) -> &[RectInstance] {
        &self.rects
    }

    pub(crate) fn line_instances(&self) -> &[LineInstance] {
        &self.lines
    }

    pub(crate) fn batches(&self) -> &[Batch] {
        &self.batches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Rect {
        Rect::from_origin_size(Vec2::new(10.0, 20.0), Vec2::new(30.0, 40.0))
    }

    fn line() -> LinePrimitive {
        LinePrimitive::new(Vec2::new(1.0, 2.0), Vec2::new(3.0, 4.0), 2.0, Color::WHITE)
    }

    #[test]
    fn instances_are_premultiplied_and_use_origin_size() {
        let primitive = RectPrimitive::new(rect(), Color::new(1.0, 0.5, 0.0, 0.5))
            .with_corner_radius(4.0)
            .with_border(2.0, Color::WHITE);
        let instance = RectInstance::from_primitive(&primitive);
        assert_eq!(instance.rect, [10.0, 20.0, 30.0, 40.0]);
        assert_eq!(instance.fill, [0.5, 0.25, 0.0, 0.5]);
        assert_eq!(instance.border_color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(instance.params, [4.0, 2.0, 0.0, 0.0]);
    }

    #[test]
    fn line_instances_carry_end_points_and_half_width() {
        let primitive = LinePrimitive {
            color: Color::new(0.0, 1.0, 0.5, 0.5),
            ..line()
        };
        let instance = LineInstance::from_primitive(&primitive);
        assert_eq!(instance.points, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(instance.color, [0.0, 0.5, 0.25, 0.5]);
        assert_eq!(instance.params, [1.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn invisible_and_empty_rects_are_culled() {
        let mut scene = Scene::new();
        scene.push_rect(&RectPrimitive::new(Rect::ZERO, Color::WHITE));
        scene.push_rect(&RectPrimitive::new(rect(), Color::TRANSPARENT));
        assert!(scene.is_empty());
        scene.push_rect(
            &RectPrimitive::new(rect(), Color::TRANSPARENT).with_border(1.0, Color::BLACK),
        );
        assert_eq!(scene.rect_count(), 1);
    }

    #[test]
    fn invisible_lines_are_culled_but_dots_are_kept() {
        let mut scene = Scene::new();
        for invisible in [
            LinePrimitive {
                width: 0.0,
                ..line()
            },
            LinePrimitive {
                width: f32::NAN,
                ..line()
            },
            LinePrimitive {
                color: Color::TRANSPARENT,
                ..line()
            },
            LinePrimitive {
                to: Vec2::new(f32::INFINITY, 0.0),
                ..line()
            },
        ] {
            scene.push_line(&invisible);
        }
        assert!(scene.is_empty());
        let dot = Vec2::new(5.0, 5.0);
        scene.push_line(&LinePrimitive::new(dot, dot, 4.0, Color::WHITE));
        assert_eq!(scene.line_count(), 1);
    }

    #[test]
    fn consecutive_primitives_of_one_type_share_a_draw() {
        let mut scene = Scene::new();
        let rect = RectPrimitive::new(rect(), Color::WHITE);
        scene.push_rect(&rect);
        scene.push_rect(&rect);
        scene.push_line(&line());
        scene.push_line(&line());
        scene.push_line(&line());
        scene.push_rect(&rect);
        let batches: Vec<_> = scene
            .batches()
            .iter()
            .map(|batch| (batch.kind, batch.instances()))
            .collect();
        assert_eq!(
            batches,
            [
                (PrimitiveKind::Rect, 0..2),
                (PrimitiveKind::Line, 0..3),
                (PrimitiveKind::Rect, 2..3),
            ]
        );
        assert_eq!(scene.draw_count(), 3);
    }

    #[test]
    fn culled_primitives_do_not_split_batches() {
        let mut scene = Scene::new();
        scene.push_line(&line());
        scene.push_rect(&RectPrimitive::new(Rect::ZERO, Color::WHITE));
        scene.push_line(&line());
        assert_eq!(scene.draw_count(), 1);
    }

    #[test]
    fn clear_keeps_capacity_for_allocation_free_frames() {
        let mut scene = Scene::new();
        for _ in 0..100 {
            scene.push_rect(&RectPrimitive::new(rect(), Color::WHITE));
            scene.push_line(&line());
        }
        let capacities = (
            scene.rects.capacity(),
            scene.lines.capacity(),
            scene.batches.capacity(),
        );
        scene.clear();
        assert!(scene.is_empty());
        assert_eq!(scene.line_count(), 0);
        assert_eq!(
            (
                scene.rects.capacity(),
                scene.lines.capacity(),
                scene.batches.capacity()
            ),
            capacities
        );
    }
}
