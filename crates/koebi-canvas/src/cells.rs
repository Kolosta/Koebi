//! The cells of an outline encoded for the GPU: one record per cell in a data texture of
//! `vec4` texels, read by `shaders/canvas.frag` and by the CPU reference ([`crate::reference`]).
//!
//! A record, at texel `r`:
//! - `r`: `rows, columns, curves, 0`: its bands across and down, and the most curves a band
//!   holds;
//! - `r + 1 ..`: its horizontal bands, then its vertical bands, each `first, count, 0, 0`: where
//!   the band's curves start, counted from `r`, and how many there are;
//! - the bands' curves, two texels each: `a.x, a.y, b.x, b.y` then `c.x, c.y, weight, 0`.
//!
//! A record names no texel outside itself: it reads the same wherever it lies, so the store
//! moves records without rewriting them ([`crate::store`]).
//!
//! Coordinates are the cell's own: relative to its origin, over its size, so that the cell
//! spans 0 to 1 whatever the zoom and every number the shader handles stays near 1. Sizes are
//! powers of two: the scaling is exact. The margin is then 2⁻¹⁰ at every octave.
//!
//! Bands split the cell grown by its margin into equal stretches. A horizontal band holds the
//! curves a ray going right may cross in it, with the edges of the right side; a vertical band
//! those a ray going down may cross, with the edges of the bottom. An edge is a straight line
//! along its side whose weight is the winding it stands for ([`koebi_geom::Edge`]); a curve
//! weighs 1.

use koebi_geom::FillCell;

/// Texels per row of the data texture: a power of two, `WIDTH_SHIFT` in the shader.
pub(crate) const DATA_WIDTH: u32 = 1 << WIDTH_SHIFT;
/// Its binary logarithm.
pub(crate) const WIDTH_SHIFT: u32 = 12;
/// The most bands a cell takes each way, `MAX_BANDS` in the shader.
pub(crate) const MAX_BANDS: usize = 32;
/// A cell's margin over its size ([`koebi_geom::Octave::margin`] over its cell size): 2⁻¹⁰ at
/// every octave.
pub(crate) const MARGIN: f32 = 1.0 / 1024.0;

/// A curve as a band stores it: its three points and its weight.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Stored {
    points: [[f32; 2]; 3],
    weight: f32,
}

/// Appends the record of `cell` to `data`, and returns where it starts. `size` and `margin` are the cell's, in document units: its coordinates are divided by
/// `size` ([`MARGIN`] is then the margin).
pub(crate) fn encode(cell: &FillCell, size: f64, margin: f64, data: &mut Vec<[f32; 4]>) -> usize {
    debug_assert!((margin / size - f64::from(MARGIN)).abs() < 1e-12);
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the inverse of a power of two, which an f32 holds exactly within its range"
    )]
    let unit = (1.0 / size) as f32;
    let normal = |value: f32| value * unit;
    let (side, margin) = (1.0 + MARGIN, MARGIN);
    let curves = cell.curves.iter().map(|&points| Stored {
        points: points.map(|[x, y]| [normal(x), normal(y)]),
        weight: 1.0,
    });
    // Along the grown right side, up or down as their winding says; along the bottom, a ray
    // going down counts a line going left: the opposite weight.
    #[allow(clippy::cast_precision_loss, reason = "a small winding number")]
    let right = cell.right.iter().map(|edge| Stored {
        points: [
            [side, normal(edge.from)],
            [side, normal(f32::midpoint(edge.from, edge.to))],
            [side, normal(edge.to)],
        ],
        weight: edge.winding as f32,
    });
    #[allow(clippy::cast_precision_loss, reason = "a small winding number")]
    let bottom = cell.bottom.iter().map(|edge| Stored {
        points: [
            [normal(edge.from), side],
            [normal(f32::midpoint(edge.from, edge.to)), side],
            [normal(edge.to), side],
        ],
        weight: -(edge.winding as f32),
    });
    let across: Vec<Stored> = curves.clone().chain(right).collect();
    let down: Vec<Stored> = curves.chain(bottom).collect();
    let bands = |count: usize| {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss,
            reason = "the square root of a count of curves"
        )]
        let bands = (count as f64).sqrt().round() as usize;
        bands.clamp(1, MAX_BANDS)
    };
    let (rows, columns) = (bands(across.len()), bands(down.len()));
    let extent = side + margin;
    let record = data.len();
    data.push([0.0; 4]);
    let table = data.len();
    data.resize(table + rows + columns, [0.0; 4]);
    let mut most = 0;
    // Horizontal bands over y for the ray going right, then vertical bands over x for the ray
    // going down.
    let layouts = [(rows, 1, &across), (columns, 0, &down)];
    let mut slot = table;
    for (count, axis, curves) in layouts {
        for band in 0..count {
            #[allow(
                clippy::cast_precision_loss,
                reason = "a band index and count below 64"
            )]
            let (low, high) = (
                -margin + extent * band as f32 / count as f32,
                -margin + extent * (band + 1) as f32 / count as f32,
            );
            let first = data.len();
            let mut held = 0_usize;
            for curve in curves {
                let (a, c) = (curve.points[0][axis], curve.points[2][axis]);
                // The band's own stretch, ends included: a curve ending on its line counts in
                // both bands.
                if a.max(c) >= low && a.min(c) <= high {
                    let [p0, p1, p2] = curve.points;
                    data.push([p0[0], p0[1], p1[0], p1[1]]);
                    data.push([p2[0], p2[1], curve.weight, 0.0]);
                    held += 1;
                }
            }
            most = most.max(held);
            #[allow(clippy::cast_precision_loss, reason = "texel counts below 2^24")]
            {
                data[slot] = [(first - record) as f32, held as f32, 0.0, 0.0];
            }
            slot += 1;
        }
    }
    #[allow(clippy::cast_precision_loss, reason = "counts below 2^24")]
    {
        data[record] = [rows as f32, columns as f32, most as f32, 0.0];
    }
    record
}

#[cfg(test)]
mod tests {
    use koebi_geom::{Edge, Point};

    use super::*;

    #[test]
    fn records_hold_their_bands_curves_and_edges_in_the_cells_coordinates() {
        // A cell of 4 units: a curve inside, an edge on the right of winding 2, one at the
        // bottom of winding -1.
        let cell = FillCell {
            column: 0,
            row: 0,
            origin: Point::new(0.0, 0.0),
            curves: vec![[[1.0, 1.0], [2.0, 1.5], [3.0, 3.0]]],
            right: vec![Edge {
                from: 0.0,
                to: 2.0,
                winding: 2,
            }],
            bottom: vec![Edge {
                from: 1.0,
                to: 3.0,
                winding: -1,
            }],
        };
        // After another record's texels: the record names its curves from its own start.
        let mut data = vec![[9.0; 4]];
        let record = encode(&cell, 4.0, 4.0 / 1024.0, &mut data);
        assert_eq!(record, 1);
        // Two curves each way: one band each.
        assert_eq!(data[1], [1.0, 1.0, 2.0, 0.0]);
        let across = data[2];
        let down = data[3];
        assert_eq!(across[1], 2.0);
        assert_eq!(down[1], 2.0);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "an index"
        )]
        let curve = |band: [f32; 4], k: usize| {
            let first = record + band[0] as usize + 2 * k;
            (data[first], data[first + 1])
        };
        // The curve, over the cell's size.
        assert_eq!(
            curve(across, 0),
            ([0.25, 0.25, 0.5, 0.375], [0.75, 0.75, 1.0, 0.0])
        );
        // The right edge on the grown side, upward, weighing its winding.
        let side = 1.0 + MARGIN;
        assert_eq!(
            curve(across, 1),
            ([side, 0.0, side, 0.25], [side, 0.5, 2.0, 0.0])
        );
        // The bottom edge going right, weighing the opposite: a ray going down counts lines
        // going left.
        assert_eq!(
            curve(down, 1),
            ([0.25, side, 0.5, side], [0.75, side, 1.0, 0.0])
        );
    }
}
