//! [`Frame`]: what one view of a program draws, built on the CPU for the GPU and for the
//! reference alike: the layers of the program, the records of their cells and an instance per
//! cell.

use std::sync::Arc;

use koebi_doc::eval::{Draw, Program, Step, StepId};
use koebi_geom::{Contours, FillRule, Path, Point, Rect, Vector, fill_cells};
use koebi_gpu::Pod;

use crate::cells::encode;
use crate::view::View;

/// GPU layout of one cell's instance. Must match the inputs of `shaders/canvas.vert`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub(crate) struct CellInstance {
    /// The cell's square on the target: `x0, y0, x1, y1`, physical pixels.
    pub(crate) rect: [f32; 4],
    /// Its origin on the target, the pixels per side of the cell, and its record.
    pub(crate) map: [f32; 4],
    /// Its paint, linear light, premultiplied.
    pub(crate) color: [f32; 4],
    /// 1 for the even-odd rule, the margin over the cell's size, unused.
    pub(crate) params: [f32; 4],
}

// SAFETY: `repr(C)` struct of four `[f32; 4]` arrays: 64 bytes with no padding, and every bit
// pattern is a valid value.
unsafe impl Pod for CellInstance {}
const _: () = assert!(size_of::<CellInstance>() == 64);

/// The layers, records and instances of one view of a program. Kept from frame to frame: its
/// storage stays.
#[derive(Debug, Default)]
pub(crate) struct Frame {
    /// The records of the cells, texel 0 their header ([`crate::cells`]).
    pub(crate) data: Vec<[f32; 4]>,
    /// One instance per cell, in drawing order.
    pub(crate) instances: Vec<CellInstance>,
    /// The layers, bottom first: outline, rule, premultiplied linear color.
    layers: Vec<Layer>,
    /// The outline of the page's background, and the size it was made for.
    page: Option<(Vector, Arc<Contours>)>,
}

impl Frame {
    /// Gathers the program's layers and builds the records and instances of their cells.
    pub(crate) fn build(&mut self, program: &Program, view: &View) {
        self.data.clear();
        // The header, filled once the records are known.
        self.data.push([0.0; 4]);
        self.instances.clear();
        self.layers.clear();
        if !view.is_valid() {
            return;
        }
        // The page's background, under everything.
        let background = program.background;
        if background.a > 0.0 {
            let page = match &self.page {
                Some((size, page)) if *size == program.size => Arc::clone(page),
                _ => {
                    let page = Arc::new(
                        Path::rectangle(Rect::new(
                            Point::new(0.0, 0.0),
                            Point::new(program.size.x, program.size.y),
                        ))
                        .contours(),
                    );
                    self.page = Some((program.size, Arc::clone(&page)));
                    page
                }
            };
            self.layers
                .push((page, FillRule::NonZero, premultiplied(background, 1.0)));
        }
        if let Some(root) = program.root {
            gather(program, root, 1.0, &mut self.layers, 0);
        }
        let octave = view.octave();
        let (size, margin) = (octave.cell_size(), octave.margin());
        let area = view.area();
        for (outline, rule, color) in &self.layers {
            for cell in fill_cells(outline, octave, area) {
                let record = encode(&cell, size, margin, &mut self.data);
                let low = view.to_target(cell.origin);
                let high = view.to_target(cell.origin + Vector::new(size, size));
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_precision_loss,
                    reason = "positions near the target in pixels, and a record below 2^24"
                )]
                self.instances.push(CellInstance {
                    rect: [low[0] as f32, low[1] as f32, high[0] as f32, high[1] as f32],
                    map: [
                        low[0] as f32,
                        low[1] as f32,
                        (view.zoom * size) as f32,
                        record as f32,
                    ],
                    color: *color,
                    params: [
                        f32::from(u8::from(*rule == FillRule::EvenOdd)),
                        crate::cells::MARGIN,
                        0.0,
                        0.0,
                    ],
                });
            }
        }
        #[allow(clippy::cast_precision_loss, reason = "counts below 2^24")]
        {
            self.data[0] = [
                self.instances.len() as f32,
                self.data.len() as f32,
                0.0,
                0.0,
            ];
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

/// One layer of a frame: an outline, its rule and its paint, linear and premultiplied.
type Layer = (Arc<Contours>, FillRule, [f32; 4]);

/// The most nested steps the program's walk follows, against cycles in a corrupt program.
const MAX_DEPTH: usize = 256;

/// Appends the layers of the step `id` of `program`, bottom first, at `opacity`.
fn gather(program: &Program, id: StepId, opacity: f64, layers: &mut Vec<Layer>, depth: usize) {
    let Some(step) = program.steps.get(id.0) else {
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
            layers.push((
                Arc::clone(outline),
                *rule,
                premultiplied(*color, opacity * own),
            ));
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
        let program = Program {
            page: koebi_doc::NodeId(0),
            size: Vector::new(10.0, 10.0),
            background: LinearRgba::new(0.0, 0.0, 0.0, 0.0),
            steps,
            root: Some(StepId(2)),
        };
        let mut layers = Vec::new();
        gather(&program, StepId(2), 1.0, &mut layers, 0);
        let alphas: Vec<f32> = layers.iter().map(|layer| layer.2[3]).collect();
        assert_eq!(alphas, [0.5, 0.2]);
        // Premultiplied.
        assert!((layers[1].2[1] - 0.1).abs() < 1e-6);
        layers.clear();
        gather(&program, StepId(3), 1.0, &mut layers, 0);
        assert_eq!(layers.len(), 0);
    }
}
