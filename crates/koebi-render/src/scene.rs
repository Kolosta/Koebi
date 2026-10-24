//! Scene description: what to draw this frame, in painter's order.

use koebi_core::{Color, Rect};
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

/// The 2D primitives of one frame, drawn in insertion order (later on top).
///
/// Reuse one scene across frames with [`Scene::clear`]: storage is kept, so steady-state
/// frames do not allocate.
#[derive(Clone, Debug, Default)]
pub struct Scene {
    clear_color: Color,
    rects: Vec<RectInstance>,
}

impl Scene {
    /// An empty scene cleared to transparent black.
    pub fn new() -> Self {
        Self::default()
    }

    /// Removes every primitive, keeping allocated storage.
    pub fn clear(&mut self) {
        self.rects.clear();
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
    }

    /// Number of rectangles.
    pub fn rect_count(&self) -> usize {
        self.rects.len()
    }

    /// `true` when nothing will be drawn (the target is still cleared).
    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }

    pub(crate) fn rect_instances(&self) -> &[RectInstance] {
        &self.rects
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::Vec2;

    use super::*;

    fn rect() -> Rect {
        Rect::from_origin_size(Vec2::new(10.0, 20.0), Vec2::new(30.0, 40.0))
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
    fn clear_keeps_capacity_for_allocation_free_frames() {
        let mut scene = Scene::new();
        for _ in 0..100 {
            scene.push_rect(&RectPrimitive::new(rect(), Color::WHITE));
        }
        let capacity = scene.rects.capacity();
        scene.clear();
        assert!(scene.is_empty());
        assert_eq!(scene.rects.capacity(), capacity);
    }
}
