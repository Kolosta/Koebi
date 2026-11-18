//! [`Frame`]: what one view of a program draws, built on the CPU for the GPU and for the
//! reference alike: the layers of the program, kept while its revision is the same, and one
//! instance per cell the view shows, its records in the [`CellStore`]. Strokes in pixels are
//! outlined at the view's zoom level by [`ViewStrokes`].

use std::sync::Arc;

use koebi_doc::eval::{Draw, Program, Step, StepId, ViewStroke};
use koebi_geom::{Contours, FillRule, Path, Point, Rect, StrokeStyle, Vector};
use koebi_gpu::Pod;

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
    /// The layers, bottom first.
    layers: Vec<Layer>,
    /// The zoom level its strokes in pixels were last outlined at.
    level: Option<Level>,
    /// The outline of the page's background, and the size it was made for.
    page: Option<(Vector, Arc<Contours>)>,
    /// One instance per cell shown, in drawing order.
    pub(crate) instances: Vec<CellInstance>,
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
        if !view.is_valid() {
            return Ok(());
        }
        let octave = view.octave();
        let size = octave.cell_size();
        let area = view.area();
        let pixel = 1.0 / view.zoom;
        let level = Level::follow(self.level, view.zoom);
        self.level = Some(level);
        for layer in &self.layers {
            let stroked;
            let (outline, rule, color) = match layer {
                Layer::Fill {
                    outline,
                    rule,
                    color,
                } => (outline, *rule, color),
                Layer::Stroke {
                    geometry,
                    style,
                    color,
                } => {
                    let Some(outline) = strokes.outline(geometry, style, level, area) else {
                        continue;
                    };
                    stroked = outline;
                    (&stroked, FillRule::NonZero, color)
                }
            };
            let (bounds, cells) = store.cells(outline, octave, area)?;
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
        }
        Ok(())
    }

    /// Gathers the layers of `program`: its page's background, then its root's layers.
    fn gather(&mut self, program: &Program) {
        self.layers.clear();
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
            });
        }
        if let Some(root) = program.root() {
            gather(program, root, 1.0, &mut self.layers, 0);
        }
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
    },
    /// A geometry stroked in a style whose lengths count the view's pixels.
    Stroke {
        geometry: Arc<Contours>,
        style: Arc<StrokeStyle>,
        color: [f32; 4],
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

/// Appends the layers of the step `id` of `program`, bottom first, at `opacity`.
fn gather(program: &Program, id: StepId, opacity: f64, layers: &mut Vec<Layer>, depth: usize) {
    let Some(step) = program.steps().get(id.0) else {
        return;
    };
    if depth > MAX_DEPTH || opacity <= 0.0 {
        return;
    }
    match step {
        Step::Draw(Draw {
            outline,
            rule,
            color,
            opacity: own,
        }) => {
            layers.push(Layer::Fill {
                outline: Arc::clone(outline),
                rule: *rule,
                color: premultiplied(*color, opacity * own),
            });
        }
        Step::ViewStroke(ViewStroke {
            geometry,
            style,
            color,
            opacity: own,
        }) => {
            layers.push(Layer::Stroke {
                geometry: Arc::clone(geometry),
                style: Arc::clone(style),
                color: premultiplied(*color, opacity * own),
            });
        }
        Step::Merge(inputs) => {
            for input in inputs {
                gather(
                    program,
                    input.step,
                    opacity * input.opacity,
                    layers,
                    depth + 1,
                );
            }
        }
        Step::Blend {
            backdrop,
            source,
            opacity: own,
            ..
        } => {
            gather(program, *backdrop, opacity, layers, depth + 1);
            gather(program, *source, opacity * own, layers, depth + 1);
        }
        Step::Mask { content, .. } => gather(program, *content, opacity, layers, depth + 1),
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
            outline: Arc::new(Contours::default()),
            rule: FillRule::NonZero,
            color: LinearRgba::new(1.0, 0.5, 0.25, alpha),
            opacity: 0.5,
        })
    }

    #[test]
    fn layers_come_bottom_first_at_the_opacity_of_what_holds_them() {
        let steps = vec![
            draw(1.0),
            draw(0.8),
            Step::Merge(vec![
                MergeInput {
                    step: StepId(0),
                    blend: BlendMode::Normal,
                    opacity: 1.0,
                },
                MergeInput {
                    step: StepId(1),
                    blend: BlendMode::Multiply,
                    opacity: 0.5,
                },
            ]),
            // A cycle, as a corrupt program might hold: the walk stops.
            Step::Merge(vec![MergeInput {
                step: StepId(3),
                blend: BlendMode::Normal,
                opacity: 1.0,
            }]),
        ];
        let program = Program::new(
            koebi_doc::NodeId(0),
            Vector::new(10.0, 10.0),
            LinearRgba::new(0.0, 0.0, 0.0, 0.0),
            steps,
            Some(StepId(2)),
        );
        let mut layers = Vec::new();
        gather(&program, StepId(2), 1.0, &mut layers, 0);
        let alphas: Vec<f32> = layers.iter().map(|layer| layer.color()[3]).collect();
        assert_eq!(alphas, [0.5, 0.2]);
        // Premultiplied.
        assert!((layers[1].color()[1] - 0.1).abs() < 1e-6);
        layers.clear();
        gather(&program, StepId(3), 1.0, &mut layers, 0);
        assert_eq!(layers.len(), 0);
    }
}
