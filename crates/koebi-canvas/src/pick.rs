//! Picking on the CPU (ADR 0029): what a view drew, layer by layer, with the object each layer
//! paints ([`Drawn`]), and an index of their bounds over the view ([`PickIndex`]) that leaves a
//! pick a few exact tests.
//!
//! A pick tests the layers under the point, topmost first: the outline drawn (a stroke in pixels
//! at the view's zoom level), by its fill rule, or within a tolerance of its edges. It returns the
//! object of the first layer hit. Compositing does not count: a masked or erased layer is picked
//! where its outline is.

use std::sync::Arc;

use koebi_core::Extent;
use koebi_doc::NodeId;
use koebi_geom::{Contours, FillRule, Point, Rect};

use crate::view::View;

/// The largest tolerance of a pick, in pixels: how far past their bounds the index files layers.
pub(crate) const MAX_TOLERANCE: f64 = 8.0;
/// Bins of the index along each side of the view.
const GRID: usize = 32;

/// A layer as a view drew it.
#[derive(Clone, Debug)]
pub(crate) struct Drawn {
    /// The outline drawn.
    pub(crate) outline: Arc<Contours>,
    pub(crate) rule: FillRule,
    pub(crate) bounds: Rect,
    /// The object it paints.
    pub(crate) object: NodeId,
}

impl Drawn {
    /// Whether `point` lies in the outline, or within `reach` of its edges (document units).
    fn hit(&self, point: Point, reach: f64) -> bool {
        self.bounds.inflated(reach).contains(point)
            && (self.outline.contains(point, self.rule)
                || (reach > 0.0
                    && self
                        .outline
                        .nearest(point)
                        .is_some_and(|nearest| nearest.distance <= reach)))
    }
}

/// The layers a view drew, filed by the bins of a grid over the view that their bounds, grown
/// by [`MAX_TOLERANCE`], cross. Stored flat: the layers of bin `b` are
/// `entries[starts[b]..starts[b + 1]]`, in drawing order. Its storage stays from build to build.
#[derive(Debug, Default)]
pub(crate) struct PickIndex {
    /// The view it was built for, `None` until built.
    view: Option<View>,
    starts: Vec<u32>,
    entries: Vec<u32>,
    /// Where each bin's next entry goes, while building.
    cursor: Vec<u32>,
}

impl PickIndex {
    /// Forgets the index: the layers changed.
    pub(crate) fn invalidate(&mut self) {
        self.view = None;
    }

    /// The topmost object of `drawn` (seen through `view`) at `pixel` of the target, within
    /// `tolerance` pixels (at most [`MAX_TOLERANCE`]); the index is built first if needed.
    pub(crate) fn pick(
        &mut self,
        drawn: &[Drawn],
        view: &View,
        pixel: [f64; 2],
        tolerance: f64,
    ) -> Option<NodeId> {
        if self.view != Some(*view) {
            self.build(drawn, view);
        }
        let point = view.to_document(pixel);
        let reach = tolerance.clamp(0.0, MAX_TOLERANCE) / view.zoom;
        let hit = |layer: &&Drawn| layer.hit(point, reach);
        match bin_of(view.extent, pixel) {
            Some(bin) => {
                let (start, end) = (self.starts[bin] as usize, self.starts[bin + 1] as usize);
                self.entries[start..end]
                    .iter()
                    .rev()
                    .map(|&at| &drawn[at as usize])
                    .find(hit)
            }
            // Outside the view: every layer, topmost first.
            None => drawn.iter().rev().find(hit),
        }
        .map(|layer| layer.object)
    }

    /// Files `drawn` by the bins their grown bounds cross.
    fn build(&mut self, drawn: &[Drawn], view: &View) {
        let bins = GRID * GRID;
        self.starts.clear();
        self.starts.resize(bins + 1, 0);
        let reach = MAX_TOLERANCE / view.zoom;
        let spans = |layer: &Drawn| bins_of(view, layer.bounds.inflated(reach));
        // Count, then lay the bins out one after another, then fill them in drawing order.
        for layer in drawn {
            if let Some((columns, rows)) = spans(layer) {
                for row in rows {
                    for column in columns.clone() {
                        self.starts[row * GRID + column + 1] += 1;
                    }
                }
            }
        }
        for bin in 0..bins {
            self.starts[bin + 1] += self.starts[bin];
        }
        self.entries.clear();
        self.entries.resize(self.starts[bins] as usize, 0);
        self.cursor.clear();
        self.cursor.extend_from_slice(&self.starts[..bins]);
        for (at, layer) in drawn.iter().enumerate() {
            if let Some((columns, rows)) = spans(layer) {
                for row in rows {
                    for column in columns.clone() {
                        let slot = &mut self.cursor[row * GRID + column];
                        #[allow(
                            clippy::cast_possible_truncation,
                            reason = "a layer index, below 2^32"
                        )]
                        {
                            self.entries[*slot as usize] = at as u32;
                        }
                        *slot += 1;
                    }
                }
            }
        }
        self.view = Some(*view);
    }
}

/// The bin of `pixel` in a target of `extent`; `None` outside it.
fn bin_of(extent: Extent, pixel: [f64; 2]) -> Option<usize> {
    let (width, height) = (f64::from(extent.width), f64::from(extent.height));
    if !(0.0..width).contains(&pixel[0]) || !(0.0..height).contains(&pixel[1]) {
        return None;
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a bin index below GRID"
    )]
    let at = |value: f64, side: f64| ((value / side * GRID as f64) as usize).min(GRID - 1);
    Some(at(pixel[1], height) * GRID + at(pixel[0], width))
}

/// The columns and rows of bins `bounds` (document units) crosses in `view`; `None` when it
/// misses the view.
fn bins_of(view: &View, bounds: Rect) -> Option<(std::ops::Range<usize>, std::ops::Range<usize>)> {
    let (low, high) = (view.to_target(bounds.min), view.to_target(bounds.max));
    let (width, height) = (f64::from(view.extent.width), f64::from(view.extent.height));
    if bounds.is_empty() || high[0] < 0.0 || high[1] < 0.0 || low[0] >= width || low[1] >= height {
        return None;
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a bin index within 0..=GRID"
    )]
    let at = |value: f64, side: f64| {
        ((value / side * GRID as f64).floor().max(0.0) as usize).min(GRID - 1)
    };
    Some((
        at(low[0], width)..at(high[0], width) + 1,
        at(low[1], height)..at(high[1], height) + 1,
    ))
}

#[cfg(test)]
mod tests {
    use koebi_geom::{Path, Vector};

    use super::*;

    fn square(x: f64, y: f64, side: f64, object: u64) -> Drawn {
        let bounds = Rect::new(Point::new(x, y), Point::new(x + side, y + side));
        Drawn {
            outline: Arc::new(Path::rectangle(bounds).contours()),
            rule: FillRule::NonZero,
            bounds,
            object: NodeId(object),
        }
    }

    #[test]
    fn picks_find_the_topmost_layer_under_the_point() {
        let view = View {
            center: Point::new(50.0, 50.0),
            zoom: 2.0,
            extent: Extent::new(200, 200),
        };
        // Two squares overlapping, a third far away, a ring with a hole.
        let ring = {
            let mut path = Path::ellipse(Point::new(20.0, 80.0), Vector::new(10.0, 10.0));
            path.subpaths
                .extend(Path::ellipse(Point::new(20.0, 80.0), Vector::new(5.0, 5.0)).subpaths);
            Drawn {
                outline: Arc::new(path.contours()),
                rule: FillRule::EvenOdd,
                bounds: Rect::new(Point::new(10.0, 70.0), Point::new(30.0, 90.0)),
                object: NodeId(4),
            }
        };
        let drawn = vec![
            square(10.0, 10.0, 30.0, 1),
            square(30.0, 30.0, 30.0, 2),
            square(500.0, 500.0, 10.0, 3),
            ring,
        ];
        let mut index = PickIndex::default();
        let pixel = |x: f64, y: f64| view.to_target(Point::new(x, y));
        assert_eq!(
            index.pick(&drawn, &view, pixel(35.0, 35.0), 0.0),
            Some(NodeId(2))
        );
        assert_eq!(
            index.pick(&drawn, &view, pixel(15.0, 15.0), 0.0),
            Some(NodeId(1))
        );
        assert_eq!(index.pick(&drawn, &view, pixel(65.0, 35.0), 0.0), None);
        // Within the tolerance of an edge: 4 pixels are 2 units at a zoom of 2.
        assert_eq!(
            index.pick(&drawn, &view, pixel(61.5, 35.0), 4.0),
            Some(NodeId(2))
        );
        assert_eq!(index.pick(&drawn, &view, pixel(62.5, 35.0), 4.0), None);
        // The ring's hole is not the ring.
        assert_eq!(index.pick(&drawn, &view, pixel(20.0, 80.0), 0.0), None);
        assert_eq!(
            index.pick(&drawn, &view, pixel(27.0, 80.0), 0.0),
            Some(NodeId(4))
        );
        // Outside the view, every layer is tested.
        assert_eq!(
            index.pick(&drawn, &view, pixel(505.0, 505.0), 0.0),
            Some(NodeId(3))
        );
        // The index files each layer in the bins it crosses, once each.
        assert_eq!(
            index.entries.len(),
            index
                .starts
                .windows(2)
                .map(|pair| (pair[1] - pair[0]) as usize)
                .sum::<usize>()
        );
        assert!(!index.entries.contains(&2));
    }
}
