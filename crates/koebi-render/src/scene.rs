//! Scene description: what to draw this frame, layer by layer, each layer in painter's order.

use koebi_core::{Color, Rect, Vec2};
use koebi_gpu::Pod;
use koebi_text::{FontId, GlyphId, TextLine};

/// Per-corner values of a rectangle: top-left, top-right, bottom-right, bottom-left.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Corners {
    /// Top-left corner.
    pub top_left: f32,
    /// Top-right corner.
    pub top_right: f32,
    /// Bottom-right corner.
    pub bottom_right: f32,
    /// Bottom-left corner.
    pub bottom_left: f32,
}

impl Corners {
    /// Every corner square.
    pub const ZERO: Self = Self::all(0.0);

    /// The same value at every corner.
    pub const fn all(value: f32) -> Self {
        Self {
            top_left: value,
            top_right: value,
            bottom_right: value,
            bottom_left: value,
        }
    }

    /// The values in shader order: top-left, top-right, bottom-right, bottom-left.
    pub const fn to_array(self) -> [f32; 4] {
        [
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        ]
    }

    /// The largest value.
    pub fn max(self) -> f32 {
        self.top_left
            .max(self.top_right)
            .max(self.bottom_right)
            .max(self.bottom_left)
    }
}

/// Per-side values of a rectangle: top, right, bottom, left.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sides {
    /// Top side.
    pub top: f32,
    /// Right side.
    pub right: f32,
    /// Bottom side.
    pub bottom: f32,
    /// Left side.
    pub left: f32,
}

impl Sides {
    /// Nothing on any side.
    pub const ZERO: Self = Self::all(0.0);

    /// The same value on every side.
    pub const fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// The values in shader order: top, right, bottom, left.
    pub const fn to_array(self) -> [f32; 4] {
        [self.top, self.right, self.bottom, self.left]
    }

    /// The largest value.
    pub fn max(self) -> f32 {
        self.top.max(self.right).max(self.bottom).max(self.left)
    }
}

/// A rectangle with a fill, a radius per corner and an inner border with a width per side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RectPrimitive {
    /// Bounds in logical pixels.
    pub rect: Rect,
    /// Fill color.
    pub fill: Color,
    /// Corner radii in logical pixels, each clamped to half the smallest side.
    pub corner_radii: Corners,
    /// Border widths in logical pixels, drawn inside the bounds; zero on every side disables
    /// the border.
    pub border_widths: Sides,
    /// Border color.
    pub border_color: Color,
}

impl RectPrimitive {
    /// A filled rectangle with square corners and no border.
    pub const fn new(rect: Rect, fill: Color) -> Self {
        Self {
            rect,
            fill,
            corner_radii: Corners::ZERO,
            border_widths: Sides::ZERO,
            border_color: Color::TRANSPARENT,
        }
    }

    /// The same radius at every corner.
    #[must_use]
    pub const fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radii = Corners::all(radius);
        self
    }

    /// A radius per corner.
    #[must_use]
    pub const fn with_corner_radii(mut self, radii: Corners) -> Self {
        self.corner_radii = radii;
        self
    }

    /// An inner border of the same width on every side.
    #[must_use]
    pub const fn with_border(mut self, width: f32, color: Color) -> Self {
        self.border_widths = Sides::all(width);
        self.border_color = color;
        self
    }

    /// An inner border with a width per side: a header's bottom border, the shared edge of
    /// grouped buttons drawn by one of them.
    #[must_use]
    pub const fn with_border_widths(mut self, widths: Sides, color: Color) -> Self {
        self.border_widths = widths;
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

/// One glyph, drawn from the renderer's glyph atlas at its physical size.
///
/// The renderer rasterizes it from its fonts ([`crate::Renderer2D::set_fonts`]) and places its
/// bitmap on whole physical pixels, its pen position rounded to a quarter pixel horizontally
/// and its baseline to a whole pixel, so text stays crisp at every scale factor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphPrimitive {
    /// The glyph's font, in the renderer's fonts.
    pub font: FontId,
    /// The glyph.
    pub glyph: GlyphId,
    /// Size in logical pixels per em.
    pub size: f32,
    /// Pen position on the baseline, in logical pixels.
    pub origin: Vec2,
    /// Color.
    pub color: Color,
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
    /// Corner radii in logical pixels: top-left, top-right, bottom-right, bottom-left.
    pub(crate) radii: [f32; 4],
    /// Border widths in logical pixels: top, right, bottom, left.
    pub(crate) borders: [f32; 4],
}

// SAFETY: `repr(C)` struct of five `[f32; 4]` arrays: 80 bytes with no padding, and every bit
// pattern is a valid value.
unsafe impl Pod for RectInstance {}
const _: () = assert!(size_of::<RectInstance>() == 80);

impl RectInstance {
    pub(crate) const STRIDE: u32 = 80;

    fn from_primitive(primitive: &RectPrimitive) -> Self {
        let r = primitive.rect;
        let positive = |values: [f32; 4]| values.map(|value| value.max(0.0));
        Self {
            rect: [r.min.x, r.min.y, r.width(), r.height()],
            fill: primitive.fill.premultiplied(),
            border_color: primitive.border_color.premultiplied(),
            radii: positive(primitive.corner_radii.to_array()),
            borders: positive(primitive.border_widths.to_array()),
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
    Glyph,
}

/// Consecutive primitives of one type and clip, on one layer: one instanced draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Batch {
    pub(crate) kind: PrimitiveKind,
    /// First instance, in the instances of `kind`.
    start: u32,
    /// One past the last instance.
    end: u32,
    /// The rectangle its primitives are clipped to, in logical pixels.
    pub(crate) clip: Option<Rect>,
}

impl Batch {
    /// How to draw the batch from a buffer of instances of `stride` bytes: the byte offset of
    /// its first instance, where to bind the buffer, and its number of instances.
    ///
    /// Draws always start at instance zero: MoltenVK rejects non-zero base instances on
    /// GPUs without them, such as the iOS simulator's.
    pub(crate) fn window(self, stride: u32) -> (u64, u32) {
        (
            u64::from(self.start) * u64::from(stride),
            self.end - self.start,
        )
    }
}

/// The 2D primitives of one frame.
///
/// - **Layers**: primitives go to the current layer ([`Scene::push_layer`]); layers draw in
///   increasing order, each in insertion order (later on top). A menu pushed on a higher layer
///   covers what the base layer draws after it.
/// - **Clips**: primitives pushed between [`Scene::push_clip`] and [`Scene::pop_clip`] are
///   clipped to the rectangle (intersected with the clips around it): an editor's content stays
///   in its area, a scrolled list in its viewport. A layer pushed inside clips starts unclipped:
///   a menu is not limited to the area that opened it.
///
/// Reuse one scene across frames with [`Scene::clear`]: storage is kept, so steady-state
/// frames do not allocate.
#[derive(Clone, Debug, Default)]
pub struct Scene {
    clear_color: Color,
    rects: Vec<RectInstance>,
    lines: Vec<LineInstance>,
    /// Glyphs stay descriptions: their instances depend on the atlas and the scale factor.
    glyphs: Vec<GlyphPrimitive>,
    /// The runs of each layer, by layer number, each in painter's order.
    layers: Vec<Vec<Batch>>,
    /// The layer primitives go to.
    layer: u8,
    /// The clips in force, each intersected with the one before it in its layer context.
    clips: Vec<Rect>,
    /// Where the clips of the current layer context start in `clips`.
    clip_base: usize,
    /// The layer and clip base of each enclosing layer context.
    layer_stack: Vec<(u8, usize)>,
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
        self.glyphs.clear();
        for layer in &mut self.layers {
            layer.clear();
        }
        self.layer = 0;
        self.clips.clear();
        self.clip_base = 0;
        self.layer_stack.clear();
    }

    /// Pushes the primitives that follow to `layer`, without the clips in force, until the
    /// matching [`Scene::pop_layer`]. Layers draw in increasing order: 0 is the base.
    pub fn push_layer(&mut self, layer: u8) {
        self.layer_stack.push((self.layer, self.clip_base));
        self.layer = layer;
        self.clip_base = self.clips.len();
    }

    /// Returns to the layer and clips of before the matching [`Scene::push_layer`].
    pub fn pop_layer(&mut self) {
        self.clips.truncate(self.clip_base);
        if let Some((layer, clip_base)) = self.layer_stack.pop() {
            self.layer = layer;
            self.clip_base = clip_base;
        }
    }

    /// The layer primitives are pushed to.
    pub fn layer(&self) -> u8 {
        self.layer
    }

    /// Clips the primitives pushed from now on to `rect` (logical pixels), within the current
    /// clip, until the matching [`Scene::pop_clip`].
    pub fn push_clip(&mut self, rect: Rect) {
        let clip = match self.clip() {
            Some(outer) => outer.intersection(&rect).unwrap_or(Rect::ZERO),
            None => rect,
        };
        self.clips.push(clip);
    }

    /// Ends the innermost clip of the current layer context.
    pub fn pop_clip(&mut self) {
        if self.clips.len() > self.clip_base {
            self.clips.pop();
        }
    }

    /// The clip primitives are pushed with, if any.
    pub fn clip(&self) -> Option<Rect> {
        self.clips[self.clip_base..].last().copied()
    }

    /// `false` when `bounds` lie entirely outside the current clip.
    fn visible(&self, bounds: Rect) -> bool {
        self.clip()
            .is_none_or(|clip| bounds.intersection(&clip).is_some())
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
            && (primitive.border_widths.max() <= 0.0 || primitive.border_color.a <= 0.0);
        if primitive.rect.is_empty() || invisible || !self.visible(primitive.rect) {
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
        let reach = Vec2::splat(width * 0.5);
        let bounds = Rect::from_min_max(from.min(to) - reach, from.max(to) + reach);
        if !self.visible(bounds) {
            return;
        }
        self.lines.push(LineInstance::from_primitive(primitive));
        self.extend_batch(PrimitiveKind::Line, self.lines.len());
    }

    /// Appends a glyph. Glyphs without size or color, or with a non-finite position, are
    /// culled.
    pub fn push_glyph(&mut self, glyph: &GlyphPrimitive) {
        let finite = [glyph.origin.x, glyph.origin.y, glyph.size]
            .iter()
            .all(|v| v.is_finite());
        if !finite || glyph.size <= 0.0 || glyph.color.a <= 0.0 {
            return;
        }
        self.glyphs.push(*glyph);
        self.extend_batch(PrimitiveKind::Glyph, self.glyphs.len());
    }

    /// Appends the glyphs of `line`, its baseline starting at `origin` (logical pixels), at
    /// the size it was laid out at.
    pub fn push_text(&mut self, line: &TextLine, origin: Vec2, color: Color) {
        for placed in line.glyphs() {
            self.push_glyph(&GlyphPrimitive {
                font: placed.font,
                glyph: placed.glyph,
                size: line.size(),
                origin: Vec2::new(origin.x + placed.x, origin.y),
                color,
            });
        }
    }

    /// Records the instance just pushed (`count` instances of `kind` so far) in the current
    /// layer.
    fn extend_batch(&mut self, kind: PrimitiveKind, count: usize) {
        // More than 4 billion primitives cannot be drawn anyway: saturate.
        let end = u32::try_from(count).unwrap_or(u32::MAX);
        let clip = self.clip();
        let index = usize::from(self.layer);
        if self.layers.len() <= index {
            // A new layer number: grows once, then reused frame after frame.
            self.layers.resize_with(index + 1, Vec::new);
        }
        let batches = &mut self.layers[index];
        match batches.last_mut() {
            Some(batch) if batch.kind == kind && batch.clip == clip => batch.end = end,
            _ => batches.push(Batch {
                kind,
                start: end - 1,
                end,
                clip,
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

    /// Number of glyphs.
    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }

    /// Number of draw calls the scene needs: one per run of consecutive primitives of the
    /// same type and clip on one layer.
    pub fn draw_count(&self) -> usize {
        self.layers.iter().map(Vec::len).sum()
    }

    /// `true` when nothing will be drawn (the target is still cleared).
    pub fn is_empty(&self) -> bool {
        self.layers.iter().all(Vec::is_empty)
    }

    pub(crate) fn rect_instances(&self) -> &[RectInstance] {
        &self.rects
    }

    pub(crate) fn line_instances(&self) -> &[LineInstance] {
        &self.lines
    }

    pub(crate) fn glyphs(&self) -> &[GlyphPrimitive] {
        &self.glyphs
    }

    /// The runs in drawing order: layer by layer, each in painter's order.
    pub(crate) fn batches(&self) -> impl Iterator<Item = &Batch> + '_ {
        self.layers.iter().flatten()
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
        assert_eq!(instance.radii, [4.0; 4]);
        assert_eq!(instance.borders, [2.0; 4]);
        // Per corner and per side, in shader order; negative values become zero.
        let primitive = primitive
            .with_corner_radii(Corners {
                top_left: 1.0,
                top_right: 2.0,
                bottom_right: 3.0,
                bottom_left: -4.0,
            })
            .with_border_widths(
                Sides {
                    top: 0.0,
                    right: 1.0,
                    bottom: 2.0,
                    left: 3.0,
                },
                Color::BLACK,
            );
        let instance = RectInstance::from_primitive(&primitive);
        assert_eq!(instance.radii, [1.0, 2.0, 3.0, 0.0]);
        assert_eq!(instance.borders, [0.0, 1.0, 2.0, 3.0]);
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
        // With a stride of 1, a window is (first instance, instance count).
        let batches: Vec<_> = scene
            .batches()
            .map(|batch| (batch.kind, batch.window(1)))
            .collect();
        assert_eq!(
            batches,
            [
                (PrimitiveKind::Rect, (0, 2)),
                (PrimitiveKind::Line, (0, 3)),
                (PrimitiveKind::Rect, (2, 1)),
            ]
        );
        assert_eq!(scene.draw_count(), 3);
    }

    #[test]
    fn batches_draw_from_instance_zero_at_a_buffer_offset() {
        // Regression test for #61: MoltenVK on the iOS simulator rejects draws whose first
        // instance is not zero, so a run's start must move into the vertex buffer offset.
        let mut scene = Scene::new();
        let rect = RectPrimitive::new(rect(), Color::WHITE);
        scene.push_rect(&rect);
        scene.push_rect(&rect);
        scene.push_line(&line());
        scene.push_rect(&rect);
        let windows: Vec<_> = scene
            .batches()
            .map(|batch| batch.window(RectInstance::STRIDE))
            .collect();
        assert_eq!(windows, [(0, 2), (0, 1), (160, 1)]);
    }

    fn glyph() -> GlyphPrimitive {
        GlyphPrimitive {
            font: FontId::default(),
            glyph: GlyphId(36),
            size: 14.0,
            origin: Vec2::new(4.0, 12.0),
            color: Color::WHITE,
        }
    }

    #[test]
    fn invisible_glyphs_are_culled_and_runs_batch() {
        let mut scene = Scene::new();
        for invisible in [
            GlyphPrimitive {
                size: 0.0,
                ..glyph()
            },
            GlyphPrimitive {
                color: Color::TRANSPARENT,
                ..glyph()
            },
            GlyphPrimitive {
                origin: Vec2::new(f32::NAN, 0.0),
                ..glyph()
            },
        ] {
            scene.push_glyph(&invisible);
        }
        assert!(scene.is_empty());
        scene.push_glyph(&glyph());
        scene.push_glyph(&glyph());
        scene.push_rect(&RectPrimitive::new(rect(), Color::WHITE));
        scene.push_glyph(&glyph());
        let batches: Vec<_> = scene
            .batches()
            .map(|batch| (batch.kind, batch.window(1)))
            .collect();
        assert_eq!(
            batches,
            [
                (PrimitiveKind::Glyph, (0, 2)),
                (PrimitiveKind::Rect, (0, 1)),
                (PrimitiveKind::Glyph, (2, 1)),
            ]
        );
        assert_eq!(scene.glyph_count(), 3);
        scene.clear();
        assert_eq!(scene.glyph_count(), 0);
    }

    #[test]
    fn clips_split_batches_and_nest() {
        let mut scene = Scene::new();
        let rect = RectPrimitive::new(rect(), Color::WHITE);
        scene.push_rect(&rect);
        let area = Rect::from_origin_size(Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0));
        scene.push_clip(area);
        scene.push_rect(&rect);
        scene.push_rect(&rect);
        // A nested clip intersects with the one around it.
        scene.push_clip(Rect::from_origin_size(
            Vec2::new(20.0, 30.0),
            Vec2::new(200.0, 10.0),
        ));
        let inner = Rect::from_min_max(Vec2::new(20.0, 30.0), Vec2::new(100.0, 40.0));
        assert_eq!(scene.clip(), Some(inner));
        scene.push_rect(&rect);
        scene.pop_clip();
        scene.push_rect(&rect);
        scene.pop_clip();
        scene.push_rect(&rect);
        let clips: Vec<_> = scene.batches().map(|batch| batch.clip).collect();
        assert_eq!(clips, [None, Some(area), Some(inner), Some(area), None]);
        scene.push_clip(area);
        scene.clear();
        assert_eq!(scene.clip(), None);
    }

    #[test]
    fn primitives_outside_the_clip_are_culled() {
        let mut scene = Scene::new();
        scene.push_clip(Rect::from_origin_size(Vec2::ZERO, Vec2::new(5.0, 5.0)));
        // The rectangle starts at (10, 20); the segment ends one pixel away.
        scene.push_rect(&RectPrimitive::new(rect(), Color::WHITE));
        scene.push_line(&LinePrimitive::new(
            Vec2::new(8.0, 8.0),
            Vec2::new(9.0, 9.0),
            2.0,
            Color::WHITE,
        ));
        assert!(scene.is_empty());
        // A segment whose cap reaches into the clip is kept.
        scene.push_line(&LinePrimitive::new(
            Vec2::new(6.0, 2.0),
            Vec2::new(9.0, 2.0),
            4.0,
            Color::WHITE,
        ));
        assert_eq!(scene.line_count(), 1);
        // An empty clip hides everything.
        scene.push_clip(Rect::from_origin_size(
            Vec2::new(50.0, 50.0),
            Vec2::new(5.0, 5.0),
        ));
        assert_eq!(scene.clip(), Some(Rect::ZERO));
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
            scene.layers[0].capacity(),
        );
        scene.clear();
        assert!(scene.is_empty());
        assert_eq!(scene.line_count(), 0);
        assert_eq!(
            (
                scene.rects.capacity(),
                scene.lines.capacity(),
                scene.layers[0].capacity()
            ),
            capacities
        );
    }

    #[test]
    fn higher_layers_draw_after_the_base_whatever_the_order() {
        let mut scene = Scene::new();
        let rect = RectPrimitive::new(rect(), Color::WHITE);
        scene.push_rect(&rect);
        scene.push_layer(2);
        scene.push_line(&line());
        scene.pop_layer();
        // Pushed after the line, but on the base layer: drawn before it.
        scene.push_rect(&rect);
        let batches: Vec<_> = scene
            .batches()
            .map(|batch| (batch.kind, batch.window(1)))
            .collect();
        assert_eq!(
            batches,
            [(PrimitiveKind::Rect, (0, 2)), (PrimitiveKind::Line, (0, 1)),]
        );
        assert_eq!(scene.layer(), 0);
    }

    #[test]
    fn layers_start_outside_the_clips_and_restore_them() {
        let mut scene = Scene::new();
        let area = Rect::from_origin_size(Vec2::ZERO, Vec2::new(100.0, 100.0));
        scene.push_clip(area);
        scene.push_layer(1);
        // A menu opened inside a clipped area is not clipped by it.
        assert_eq!(scene.clip(), None);
        let menu = Rect::from_origin_size(Vec2::new(50.0, 50.0), Vec2::new(200.0, 10.0));
        scene.push_clip(menu);
        assert_eq!(scene.clip(), Some(menu));
        // Left unbalanced on purpose: popping the layer drops its clips.
        scene.pop_layer();
        assert_eq!(scene.clip(), Some(area));
        scene.pop_clip();
        assert_eq!(scene.clip(), None);
        // Popping more clips than pushed in a layer context leaves the outer ones.
        scene.push_clip(area);
        scene.push_layer(1);
        scene.pop_clip();
        scene.pop_layer();
        assert_eq!(scene.clip(), Some(area));
    }
}
