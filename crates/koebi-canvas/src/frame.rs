//! [`Frame`]: what one view of a program draws, built on the CPU for the GPU and for the
//! reference alike.
//!
//! - **The script**, kept while the program's revision is the same: the program's steps walked
//!   into layers to draw and the groups that composite them ([`Op`]). A merge that blends one
//!   of its layers in another mode than Normal draws in a group of its own, so that a blend
//!   reaches only what lies below it in its merge; a layer drawn at an opacity, or in another
//!   mode, gets a group too, unless it is a single draw at an opacity, which its color takes. A
//!   mask draws its content and its coverage in two groups, the content then shown where the
//!   coverage is.
//! - **The instances**: one per cell the view shows, layer after layer, their records in the
//!   [`CellStore`]. Strokes in pixels are outlined at the view's zoom level by
//!   [`ViewStrokes`].

use std::sync::Arc;

use std::ops::Range;

use koebi_core::Extent;
use koebi_doc::eval::{Draw, Program, Step, StepCoverage, StepId, ViewStroke};
use koebi_doc::{BlendMode, NodeId};
use koebi_geom::{Contours, FillRule, Path, Point, Rect, StrokeStyle, Vector};
use koebi_gpu::{Pod, TextureRegion};

use crate::pick::{Drawn, PickIndex};
use crate::store::{CellStore, Full};
use crate::strokes::{Level, ViewStrokes};
use crate::view::View;

/// GPU layout of one cell's instance. Must match the inputs of `shaders/canvas.vert`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub(crate) struct CellInstance {
    /// The part of the cell's square the outline may cover, on the target: `x0, y0, x1, y1`,
    /// physical pixels.
    pub(crate) rect: [f32; 4],
    /// The cell's origin on the target, the pixels per side of the cell, and its record.
    pub(crate) map: [f32; 4],
    /// Its paint, linear light, premultiplied.
    pub(crate) color: [f32; 4],
    /// 1 for the even-odd rule, the margin over the cell's size, the cell's sides per pixel
    /// (the inverse of `map[2]`, from f64), unused.
    pub(crate) params: [f32; 4],
}

// SAFETY: `repr(C)` struct of four `[f32; 4]` arrays: 64 bytes with no padding, and every bit
// pattern is a valid value.
unsafe impl Pod for CellInstance {}
const _: () = assert!(size_of::<CellInstance>() == 64);

/// The layers and instances of one view of a program. Kept from render to render: its storage
/// stays, and its layers while the program's revision does.
#[derive(Debug, Default)]
pub(crate) struct Frame {
    /// The revision of the program the layers come from.
    revision: Option<u64>,
    /// The layers, in the order the script draws them.
    layers: Vec<Layer>,
    /// What the view draws: layers and groups, in order.
    pub(crate) script: Vec<Op>,
    /// The instances of each layer: those of layer `i` are `instances[ranges[i]]`.
    pub(crate) ranges: Vec<Range<u32>>,
    /// The pixels each group may draw, group `k` being the `k`-th [`Op::Push`]: what its target
    /// clears and composites, nothing outside it.
    pub(crate) areas: Vec<TextureRegion>,
    /// The groups open while the areas are found, with what they drew so far.
    open: Vec<(usize, TextureRegion)>,
    /// The zoom level its strokes in pixels were last outlined at.
    level: Option<Level>,
    /// The outline of the page's background, and the size it was made for.
    page: Option<(Vector, Arc<Contours>)>,
    /// One instance per cell shown, in drawing order.
    pub(crate) instances: Vec<CellInstance>,
    /// The view of the last build.
    view: Option<View>,
    /// The layers in sight that paint an object, as drawn: what picking tests.
    drawn: Vec<Drawn>,
    /// Their bounds over the view, filed at the first pick after a build.
    index: PickIndex,
}

impl Frame {
    /// The instances of `program` seen by `view`, their cells taken from `store` or prepared
    /// into it, the outlines of strokes in pixels from `strokes`. Each instance covers the part
    /// of its cell's square that lies within the view and within a pixel of its outline's
    /// bounds: past those, the outline covers nothing.
    ///
    /// Fails when the store has no room for the cells; the instances are then incomplete.
    pub(crate) fn build(
        &mut self,
        program: &Program,
        view: &View,
        store: &mut CellStore,
        strokes: &mut ViewStrokes,
    ) -> Result<(), Full> {
        if self.revision != Some(program.revision()) {
            self.gather(program);
            self.revision = Some(program.revision());
        }
        self.instances.clear();
        self.ranges.clear();
        self.drawn.clear();
        self.index.invalidate();
        self.view = Some(*view);
        if !view.is_valid() {
            self.ranges.resize(self.layers.len(), 0..0);
            return Ok(());
        }
        let octave = view.octave();
        let size = octave.cell_size();
        let area = view.area();
        let pixel = 1.0 / view.zoom;
        let level = Level::follow(self.level, view.zoom);
        self.level = Some(level);
        let count =
            |instances: &Vec<CellInstance>| u32::try_from(instances.len()).unwrap_or(u32::MAX);
        for layer in &self.layers {
            let start = count(&self.instances);
            let stroked;
            let (outline, rule, color, object) = match layer {
                Layer::Fill {
                    outline,
                    rule,
                    color,
                    object,
                } => (outline, *rule, color, *object),
                Layer::Stroke {
                    geometry,
                    style,
                    color,
                    object,
                } => {
                    let Some(outline) = strokes.outline(geometry, style, level, area) else {
                        self.ranges.push(start..start);
                        continue;
                    };
                    stroked = outline;
                    (&stroked, FillRule::NonZero, color, *object)
                }
            };
            let (bounds, cells) = store.cells(outline, octave, area)?;
            if let Some(object) = object
                && !cells.is_empty()
            {
                self.drawn.push(Drawn {
                    outline: Arc::clone(outline),
                    rule,
                    bounds,
                    object,
                });
            }
            let reach = bounds.inflated(pixel).intersection(area);
            for cell in cells {
                let square = Rect::new(cell.origin, cell.origin + Vector::new(size, size));
                let shown = square.intersection(reach);
                if shown.width() <= 0.0 || shown.height() <= 0.0 {
                    continue;
                }
                let (low, high) = (view.to_target(shown.min), view.to_target(shown.max));
                let origin = view.to_target(cell.origin);
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_precision_loss,
                    reason = "positions near the target in pixels, and a record below 2^24"
                )]
                self.instances.push(CellInstance {
                    rect: [low[0] as f32, low[1] as f32, high[0] as f32, high[1] as f32],
                    map: [
                        origin[0] as f32,
                        origin[1] as f32,
                        (view.zoom * size) as f32,
                        cell.record as f32,
                    ],
                    color: *color,
                    params: [
                        f32::from(u8::from(rule == FillRule::EvenOdd)),
                        crate::cells::MARGIN,
                        (1.0 / (view.zoom * size)) as f32,
                        0.0,
                    ],
                });
            }
            self.ranges.push(start..count(&self.instances));
        }
        self.find_areas(view.extent);
        Ok(())
    }

    /// The pixels layer `layer` draws: the bounds of its instances.
    fn layer_area(&self, layer: usize, extent: Extent) -> TextureRegion {
        let range = self.ranges.get(layer).cloned().unwrap_or(0..0);
        let instances = &self.instances[range.start as usize..range.end as usize];
        let bounds = instances
            .iter()
            .fold(None, |bounds: Option<[f32; 4]>, instance| {
                let [x0, y0, x1, y1] = instance.rect;
                Some(match bounds {
                    Some([a, b, c, d]) => [a.min(x0), b.min(y0), c.max(x1), d.max(y1)],
                    None => instance.rect,
                })
            });
        bounds.map_or(EMPTY, |bounds| pixels_of(bounds, extent))
    }

    /// The area of every group: what its layers and its groups draw.
    fn find_areas(&mut self, extent: Extent) {
        self.areas.clear();
        self.open.clear();
        for at in 0..self.script.len() {
            let drawn = match self.script[at] {
                Op::Draw(layer) => self.layer_area(layer, extent),
                Op::Push => {
                    self.areas.push(EMPTY);
                    self.open.push((self.areas.len() - 1, EMPTY));
                    continue;
                }
                Op::Pop { .. } => match self.open.pop() {
                    Some((group, area)) => {
                        self.areas[group] = area;
                        area
                    }
                    None => EMPTY,
                },
                Op::PopMask { invert } => {
                    let (Some((mask, covered)), Some((content, drawn))) =
                        (self.open.pop(), self.open.pop())
                    else {
                        continue;
                    };
                    self.areas[content] = drawn;
                    // Inverted, the coverage is read wherever the content is: cleared there.
                    if invert {
                        self.areas[mask] = union(covered, drawn);
                        drawn
                    } else {
                        self.areas[mask] = covered;
                        intersection(covered, drawn)
                    }
                }
            };
            if let Some((_, area)) = self.open.last_mut() {
                *area = union(*area, drawn);
            }
        }
        // Groups a corrupt script left open keep what they drew.
        while let Some((group, area)) = self.open.pop() {
            self.areas[group] = area;
        }
    }

    /// The topmost object drawn at `pixel` of the last build's target, within `tolerance`
    /// pixels of its edges ([`crate::pick`]).
    pub(crate) fn pick(&mut self, pixel: [f64; 2], tolerance: f64) -> Option<NodeId> {
        let view = self.view.filter(|view| view.is_valid())?;
        self.index.pick(&self.drawn, &view, pixel, tolerance)
    }

    /// Walks `program` into layers and the script that draws them: its page's background, then
    /// its root.
    fn gather(&mut self, program: &Program) {
        self.layers.clear();
        self.script.clear();
        let background = program.background();
        if background.a > 0.0 {
            let size = program.size();
            let page = match &self.page {
                Some((made_for, page)) if *made_for == size => Arc::clone(page),
                _ => {
                    let page = Arc::new(
                        Path::rectangle(Rect::new(
                            Point::new(0.0, 0.0),
                            Point::new(size.x, size.y),
                        ))
                        .contours(),
                    );
                    self.page = Some((size, Arc::clone(&page)));
                    page
                }
            };
            self.layers.push(Layer::Fill {
                outline: page,
                rule: FillRule::NonZero,
                color: premultiplied(background, 1.0),
                object: None,
            });
            self.script.push(Op::Draw(0));
        }
        if let Some(root) = program.root() {
            Walk {
                program,
                layers: &mut self.layers,
                script: &mut self.script,
            }
            .step(root, 0, false);
        }
    }
}

/// One operation of a view's script, run in order over a stack of targets: the view's own at
/// the bottom, groups above it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Op {
    /// Draws layer `n` over the current target.
    Draw(usize),
    /// Starts a group: a new target, transparent, becomes the current one.
    Push,
    /// Ends a group: its target is composited onto the one below in `mode` at `opacity`, which
    /// becomes the current one again.
    Pop { mode: BlendMode, opacity: f32 },
    /// Ends a mask: the current target holds the coverage, the one below the content. The
    /// content is composited over the target below both where the coverage's alpha is (or is
    /// not, when inverted).
    PopMask { invert: bool },
}

/// No pixel.
pub(crate) const EMPTY: TextureRegion = TextureRegion::new(0, 0, 0, 0);

/// The pixels whose centers may lie in `bounds` (`x0, y0, x1, y1`, physical pixels), within a
/// target of `extent`.
fn pixels_of(bounds: [f32; 4], extent: Extent) -> TextureRegion {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "pixel coordinates clamped to the target"
    )]
    let clamp = |value: f32, side: u32| value.clamp(0.0, side as f32) as u32;
    #[allow(clippy::cast_precision_loss, reason = "a target's size")]
    let (x0, y0, x1, y1) = (
        clamp(bounds[0].floor(), extent.width),
        clamp(bounds[1].floor(), extent.height),
        clamp(bounds[2].ceil(), extent.width),
        clamp(bounds[3].ceil(), extent.height),
    );
    if x1 > x0 && y1 > y0 {
        TextureRegion::new(x0, y0, x1 - x0, y1 - y0)
    } else {
        EMPTY
    }
}

/// The pixels of either.
pub(crate) fn union(a: TextureRegion, b: TextureRegion) -> TextureRegion {
    if a.is_empty() {
        return b;
    }
    if b.is_empty() {
        return a;
    }
    let (x0, y0) = (a.x.min(b.x), a.y.min(b.y));
    let (x1, y1) = (
        (a.x + a.width).max(b.x + b.width),
        (a.y + a.height).max(b.y + b.height),
    );
    TextureRegion::new(x0, y0, x1 - x0, y1 - y0)
}

/// The pixels of both.
pub(crate) fn intersection(a: TextureRegion, b: TextureRegion) -> TextureRegion {
    let (x0, y0) = (a.x.max(b.x), a.y.max(b.y));
    let (x1, y1) = (
        (a.x + a.width).min(b.x + b.width),
        (a.y + a.height).min(b.y + b.height),
    );
    if x1 > x0 && y1 > y0 {
        TextureRegion::new(x0, y0, x1 - x0, y1 - y0)
    } else {
        EMPTY
    }
}

/// The premultiplied linear color of `color` at `opacity`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "an opacity between 0 and 1"
)]
fn premultiplied(color: koebi_core::LinearRgba, opacity: f64) -> [f32; 4] {
    let alpha = (f64::from(color.a) * opacity).clamp(0.0, 1.0) as f32;
    [color.r * alpha, color.g * alpha, color.b * alpha, alpha]
}

/// One layer of a frame, its paint linear and premultiplied.
#[derive(Clone, Debug)]
enum Layer {
    /// An outline filled under a rule.
    Fill {
        outline: Arc<Contours>,
        rule: FillRule,
        color: [f32; 4],
        object: Option<NodeId>,
    },
    /// A geometry stroked in a style whose lengths count the view's pixels.
    Stroke {
        geometry: Arc<Contours>,
        style: Arc<StrokeStyle>,
        color: [f32; 4],
        object: Option<NodeId>,
    },
}

impl Layer {
    /// Its paint.
    #[cfg(test)]
    fn color(&self) -> [f32; 4] {
        match self {
            Self::Fill { color, .. } | Self::Stroke { color, .. } => *color,
        }
    }
}

/// The most nested steps the program's walk follows, against cycles in a corrupt program.
const MAX_DEPTH: usize = 256;

/// The walk of a program's steps into layers and a script.
struct Walk<'a> {
    program: &'a Program,
    layers: &'a mut Vec<Layer>,
    script: &'a mut Vec<Op>,
}

impl Walk<'_> {
    /// Draws step `id` over the current target; `fresh` when that target is a group of the
    /// step's own, started for it and transparent, which a merge or a blend needs no other.
    fn step(&mut self, id: StepId, depth: usize, fresh: bool) {
        let Some(step) = self.program.steps().get(id.0) else {
            return;
        };
        if depth > MAX_DEPTH {
            return;
        }
        match step {
            Step::Draw(_) | Step::ViewStroke(_) => self.draw(step, 1.0),
            Step::Merge(inputs) => {
                // A blend reaches only what lies below it in its own merge.
                let isolated =
                    !fresh && inputs.iter().any(|input| input.blend != BlendMode::Normal);
                if isolated {
                    self.script.push(Op::Push);
                }
                for input in inputs {
                    self.input(input.step, input.blend, input.opacity, false, depth + 1);
                }
                if isolated {
                    self.pop(BlendMode::Normal, 1.0);
                }
            }
            Step::Blend {
                backdrop,
                source,
                mode,
                opacity,
                isolate,
            } => {
                let isolated = !fresh && *mode != BlendMode::Normal;
                if isolated {
                    self.script.push(Op::Push);
                }
                self.step(*backdrop, depth + 1, false);
                self.input(*source, *mode, *opacity, *isolate, depth + 1);
                if isolated {
                    self.pop(BlendMode::Normal, 1.0);
                }
            }
            Step::Mask {
                content,
                coverage,
                invert,
            } => {
                self.script.push(Op::Push);
                self.step(*content, depth + 1, true);
                self.script.push(Op::Push);
                match coverage {
                    StepCoverage::Outline { outline, rule } => {
                        self.layers.push(Layer::Fill {
                            outline: Arc::clone(outline),
                            rule: *rule,
                            color: [1.0; 4],
                            object: None,
                        });
                        self.script.push(Op::Draw(self.layers.len() - 1));
                    }
                    StepCoverage::Layer(layer) => self.step(*layer, depth + 1, true),
                }
                self.script.push(Op::PopMask { invert: *invert });
            }
        }
    }

    /// Draws step `id` over the current target in `mode` at `opacity`: in place when Normal
    /// and opaque, by its color when a single draw at an opacity, else in a group of its own
    /// (always when `isolated`).
    fn input(&mut self, id: StepId, mode: BlendMode, opacity: f64, isolated: bool, depth: usize) {
        if opacity <= 0.0 || depth > MAX_DEPTH {
            return;
        }
        let normal = mode == BlendMode::Normal;
        let opaque = (opacity - 1.0).abs() < f64::EPSILON;
        if normal && opaque && !isolated {
            return self.step(id, depth, false);
        }
        if let Some(step @ (Step::Draw(_) | Step::ViewStroke(_))) = self.program.steps().get(id.0)
            && normal
        {
            return self.draw(step, opacity);
        }
        self.script.push(Op::Push);
        self.step(id, depth, true);
        #[allow(
            clippy::cast_possible_truncation,
            reason = "an opacity between 0 and 1"
        )]
        self.pop(mode, opacity.clamp(0.0, 1.0) as f32);
    }

    /// Ends a group, or drops it when it drew nothing.
    fn pop(&mut self, mode: BlendMode, opacity: f32) {
        if self.script.last() == Some(&Op::Push) {
            self.script.pop();
        } else {
            self.script.push(Op::Pop { mode, opacity });
        }
    }

    /// The layer of a draw step at `opacity`, drawn over the current target.
    fn draw(&mut self, step: &Step, opacity: f64) {
        let layer = match step {
            Step::Draw(Draw {
                object,
                outline,
                rule,
                color,
                opacity: own,
            }) => Layer::Fill {
                outline: Arc::clone(outline),
                rule: *rule,
                color: premultiplied(*color, opacity * own),
                object: *object,
            },
            Step::ViewStroke(ViewStroke {
                object,
                geometry,
                style,
                color,
                opacity: own,
            }) => Layer::Stroke {
                geometry: Arc::clone(geometry),
                style: Arc::clone(style),
                color: premultiplied(*color, opacity * own),
                object: *object,
            },
            _ => return,
        };
        self.layers.push(layer);
        self.script.push(Op::Draw(self.layers.len() - 1));
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::LinearRgba;
    use koebi_doc::BlendMode;
    use koebi_doc::eval::MergeInput;

    use super::*;

    fn draw(alpha: f32) -> Step {
        Step::Draw(Draw {
            object: None,
            outline: Arc::new(Contours::default()),
            rule: FillRule::NonZero,
            color: LinearRgba::new(1.0, 0.5, 0.25, alpha),
            opacity: 0.5,
        })
    }

    fn script_of(program: &Program) -> (Vec<Op>, Vec<f32>) {
        let mut frame = Frame::default();
        frame.gather(program);
        let alphas = frame.layers.iter().map(|layer| layer.color()[3]).collect();
        (frame.script, alphas)
    }

    fn merge(inputs: &[(usize, BlendMode, f64)]) -> Step {
        Step::Merge(
            inputs
                .iter()
                .map(|&(step, blend, opacity)| MergeInput {
                    step: StepId(step),
                    blend,
                    opacity,
                })
                .collect(),
        )
    }

    fn program(steps: Vec<Step>, root: usize) -> Program {
        Program::new(
            NodeId(0),
            Vector::new(10.0, 10.0),
            LinearRgba::new(0.0, 0.0, 0.0, 0.0),
            steps,
            Some(StepId(root)),
        )
    }

    #[test]
    fn a_merge_that_blends_draws_in_a_group_of_its_own() {
        // A draw, then one multiplied at half opacity: the merge is a group, the multiplied
        // draw a group within it, composited at its opacity.
        let blended = program(
            vec![
                draw(1.0),
                draw(0.8),
                merge(&[(0, BlendMode::Normal, 1.0), (1, BlendMode::Multiply, 0.5)]),
            ],
            2,
        );
        let (script, alphas) = script_of(&blended);
        assert_eq!(
            script,
            [
                Op::Push,
                Op::Draw(0),
                Op::Push,
                Op::Draw(1),
                Op::Pop {
                    mode: BlendMode::Multiply,
                    opacity: 0.5
                },
                Op::Pop {
                    mode: BlendMode::Normal,
                    opacity: 1.0
                },
            ]
        );
        // Each draw at its own opacity, premultiplied.
        assert_eq!(alphas, [0.5, 0.4]);
        // Normal at an opacity: in place, the opacity in the draw's color, no group at all.
        let faded = program(
            vec![
                draw(1.0),
                draw(0.8),
                merge(&[(0, BlendMode::Normal, 1.0), (1, BlendMode::Normal, 0.5)]),
            ],
            2,
        );
        let (script, alphas) = script_of(&faded);
        assert_eq!(script, [Op::Draw(0), Op::Draw(1)]);
        assert_eq!(alphas, [0.5, 0.2]);
        // A merge at an opacity is a group; a cycle, as a corrupt program might hold, stops.
        let nested = program(
            vec![
                draw(1.0),
                draw(1.0),
                merge(&[(0, BlendMode::Normal, 1.0), (1, BlendMode::Normal, 1.0)]),
                merge(&[(2, BlendMode::Normal, 0.25)]),
            ],
            3,
        );
        let (script, _) = script_of(&nested);
        assert_eq!(
            script,
            [
                Op::Push,
                Op::Draw(0),
                Op::Draw(1),
                Op::Pop {
                    mode: BlendMode::Normal,
                    opacity: 0.25
                },
            ]
        );
        // A blended merge at an opacity: its group is the merge's own, not a second one.
        let faded_blend = program(
            vec![
                draw(1.0),
                draw(1.0),
                merge(&[(0, BlendMode::Normal, 1.0), (1, BlendMode::Screen, 1.0)]),
                merge(&[(2, BlendMode::Normal, 0.5)]),
            ],
            3,
        );
        let (script, _) = script_of(&faded_blend);
        assert_eq!(
            script,
            [
                Op::Push,
                Op::Draw(0),
                Op::Push,
                Op::Draw(1),
                Op::Pop {
                    mode: BlendMode::Screen,
                    opacity: 1.0
                },
                Op::Pop {
                    mode: BlendMode::Normal,
                    opacity: 0.5
                },
            ]
        );
        let cycle = program(vec![merge(&[(0, BlendMode::Multiply, 1.0)])], 0);
        assert_eq!(script_of(&cycle).0, []);
    }

    #[test]
    fn a_mask_draws_its_content_and_its_coverage_in_two_groups() {
        let masked = program(
            vec![
                draw(1.0),
                Step::Mask {
                    content: StepId(0),
                    coverage: StepCoverage::Outline {
                        outline: Arc::new(Contours::default()),
                        rule: FillRule::EvenOdd,
                    },
                    invert: true,
                },
            ],
            1,
        );
        let (script, alphas) = script_of(&masked);
        assert_eq!(
            script,
            [
                Op::Push,
                Op::Draw(0),
                Op::Push,
                Op::Draw(1),
                Op::PopMask { invert: true },
            ]
        );
        // The coverage drawn opaque, white.
        assert_eq!(alphas, [0.5, 1.0]);
    }
}
