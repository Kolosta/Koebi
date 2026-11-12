//! Batching: the primitives of a scene gathered into instanced draws, one pipeline each.
//!
//! A primitive joins the latest batch of its type on its layer unless a batch drawn after that
//! one painted where it paints: what overlaps keeps its painter's order, what does not draws
//! together, in whatever order it was pushed. A label and the background of the next control
//! overlap nothing of each other's, so a whole interface draws in a few batches per layer:
//! backgrounds, then labels, then icons, then what lies over them.
//!
//! Overlaps are found in a grid of [`CELL`]-pixel cells kept for the frame: each cell holds,
//! for each batch that painted in it, the box of what it painted there. Finding the batch to
//! join costs the cells a primitive covers, whatever the number of batches; a box shared by
//! two primitives of a batch in one cell may only see an overlap where there is none, which
//! costs a batch, never the order. Clips never split a batch: each instance carries its own.

use koebi_core::{Rect, Vec2};

use crate::scene::PrimitiveKind;

/// Primitives of one type, on one layer: one instanced draw. Its instances lie together once
/// gathered ([`DrawOrder`]), in the order they were pushed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Batch {
    pub(crate) kind: PrimitiveKind,
    /// The layer it draws on.
    layer: u8,
    /// How many primitives it holds.
    count: u32,
}

/// The side of a cell of the grid, in logical pixels: about a line of text, so that the boxes a
/// cell keeps stay close to what was painted.
const CELL: f32 = 32.0;
/// Columns of the grid: 8192 logical pixels; what lies past them falls in the last column.
const COLUMNS: usize = 256;
/// Rows the grid may grow to, as primitives reach them; what lies past falls in the last row.
const MAX_ROWS: usize = 256;

/// What a batch painted in a cell.
#[derive(Clone, Copy, Debug)]
struct Mark {
    batch: u32,
    layer: u8,
    /// The box around what it painted there, inside the cell.
    area: Rect,
}

/// The grid of what each batch painted where, for the frame.
#[derive(Clone, Debug, Default)]
struct Grid {
    /// The marks of each cell, row after row: their storage stays from frame to frame.
    cells: Vec<Vec<Mark>>,
    /// The cells marked this frame: only they need clearing.
    marked: Vec<u32>,
}

impl Grid {
    fn clear(&mut self) {
        for &cell in &self.marked {
            self.cells[cell as usize].clear();
        }
        self.marked.clear();
    }

    /// The columns and rows of the cells `bounds` covers.
    fn span(bounds: Rect) -> ((usize, usize), (usize, usize)) {
        // Saturating casts: below zero (and NaN) is the first cell, beyond the grid its last.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "saturating float to integer casts, clamped to the grid"
        )]
        let cell = |value: f32, count: usize| ((value / CELL) as usize).min(count - 1);
        (
            (cell(bounds.min.x, COLUMNS), cell(bounds.max.x, COLUMNS)),
            (cell(bounds.min.y, MAX_ROWS), cell(bounds.max.y, MAX_ROWS)),
        )
    }

    /// The latest batch of `layer` that painted something `bounds` overlaps.
    fn latest_under(&self, bounds: Rect, layer: u8) -> Option<u32> {
        let ((x0, x1), (y0, y1)) = Self::span(bounds);
        let mut latest = None;
        for row in y0..=y1 {
            let Some(cells) = self.cells.get(row * COLUMNS + x0..=row * COLUMNS + x1) else {
                // Rows past those marked so far: nothing painted there.
                break;
            };
            for mark in cells.iter().flatten() {
                if mark.layer == layer
                    && latest.is_none_or(|latest| mark.batch > latest)
                    && mark.area.intersection(&bounds).is_some()
                {
                    latest = Some(mark.batch);
                }
            }
        }
        latest
    }

    /// Records that `batch`, of `layer`, painted inside `bounds`.
    fn mark(&mut self, bounds: Rect, batch: u32, layer: u8) {
        let ((x0, x1), (y0, y1)) = Self::span(bounds);
        let needed = (y1 + 1) * COLUMNS;
        if self.cells.len() < needed {
            // Rows reached for the first time: grows during the first frames, then stays.
            self.cells.resize_with(needed, Vec::new);
        }
        for row in y0..=y1 {
            for column in x0..=x1 {
                let index = row * COLUMNS + column;
                // The cell's own box; the first and last columns and rows reach as far as what
                // falls in them.
                let edge = |at: usize, count: usize| {
                    let low = if at == 0 { f32::MIN } else { at as f32 * CELL };
                    let high = if at + 1 < count {
                        (at + 1) as f32 * CELL
                    } else {
                        f32::MAX
                    };
                    (low, high)
                };
                let ((left, right), (top, bottom)) = (edge(column, COLUMNS), edge(row, MAX_ROWS));
                let (low, high) = (Vec2::new(left, top), Vec2::new(right, bottom));
                let area = Rect::from_min_max(low.max(bounds.min), high.min(bounds.max));
                let cell = &mut self.cells[index];
                if cell.is_empty() {
                    // At most COLUMNS × MAX_ROWS cells: fits.
                    #[allow(clippy::cast_possible_truncation, reason = "below 65536")]
                    self.marked.push(index as u32);
                }
                match cell.last_mut() {
                    Some(last) if last.batch == batch => last.area = last.area.union(&area),
                    _ => cell.push(Mark { batch, layer, area }),
                }
            }
        }
    }
}

/// The batches of a frame, layer by layer, and what each painted where. Kept by the scene from
/// frame to frame: steady frames do not allocate.
#[derive(Clone, Debug, Default)]
pub(crate) struct Batches {
    /// Every batch, numbered in the order they were made: in each layer, its painter's order.
    all: Vec<Batch>,
    /// The batches of each layer, by layer number.
    layers: Vec<Vec<u32>>,
    /// The latest batch of each type, by layer number.
    latest: Vec<[Option<u32>; PrimitiveKind::COUNT]>,
    grid: Grid,
}

impl Batches {
    pub(crate) fn clear(&mut self) {
        self.all.clear();
        for layer in &mut self.layers {
            layer.clear();
        }
        self.latest.fill([None; PrimitiveKind::COUNT]);
        self.grid.clear();
    }

    /// Files a primitive of `kind` on `layer`, painting inside `bounds` (logical pixels, its
    /// clip applied): in the latest batch of its type on the layer when no batch drawn after
    /// that one painted where it paints, else in a new batch. Returns its batch.
    pub(crate) fn join(&mut self, kind: PrimitiveKind, layer: u8, bounds: Rect) -> u32 {
        let index = usize::from(layer);
        if self.layers.len() <= index {
            // A new layer number: grows once, then reused frame after frame.
            self.layers.resize_with(index + 1, Vec::new);
            self.latest.resize(index + 1, [None; PrimitiveKind::COUNT]);
        }
        let under = self.grid.latest_under(bounds, layer);
        // In a layer, batches are numbered in the order they draw.
        let joined = self.latest[index][kind as usize]
            .filter(|&latest| under.is_none_or(|under| latest >= under));
        let id = if let Some(id) = joined {
            self.all[id as usize].count += 1;
            id
        } else {
            // More than 4 billion batches cannot be drawn anyway: saturate.
            let id = u32::try_from(self.all.len()).unwrap_or(u32::MAX);
            self.all.push(Batch {
                kind,
                layer,
                count: 1,
            });
            self.layers[index].push(id);
            self.latest[index][kind as usize] = Some(id);
            id
        };
        self.grid.mark(bounds, id, layer);
        id
    }

    pub(crate) fn len(&self) -> usize {
        self.all.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.all.is_empty()
    }

    /// The batches in drawing order, with their numbers: layer by layer, each in painter's
    /// order.
    pub(crate) fn in_order(&self) -> impl Iterator<Item = (u32, &Batch)> + '_ {
        self.layers
            .iter()
            .flatten()
            .map(|&id| (id, &self.all[id as usize]))
    }

    /// The capacity of the base layer's list, for the tests of allocation-free frames.
    #[cfg(test)]
    pub(crate) fn base_capacity(&self) -> usize {
        self.layers.first().map_or(0, Vec::capacity)
    }
}

/// Where each batch's instances lie once gathered, and the gathering, kept by the renderer from
/// frame to frame: steady frames do not allocate.
#[derive(Clone, Debug, Default)]
pub(crate) struct DrawOrder {
    /// The first instance of each batch, by batch number, among the instances of its type.
    starts: Vec<u32>,
    /// Where the next instance of each batch goes while gathering.
    cursors: Vec<u32>,
}

impl DrawOrder {
    /// The batches laid out: each type's instances batch after batch, in drawing order.
    pub(crate) fn compute(&mut self, batches: &Batches) {
        self.starts.clear();
        self.starts.resize(batches.len(), 0);
        let mut next = [0_u32; PrimitiveKind::COUNT];
        for (id, batch) in batches.in_order() {
            let kind = batch.kind as usize;
            self.starts[id as usize] = next[kind];
            next[kind] += batch.count;
        }
    }

    /// The instances of `items`, pushed in the order of `owners` (the batch of each), gathered
    /// batch after batch into `out`. Call [`DrawOrder::compute`] for the same batches first.
    pub(crate) fn gather<T: Copy>(&mut self, items: &[T], owners: &[u32], out: &mut Vec<T>) {
        out.clear();
        out.extend_from_slice(items);
        self.cursors.clear();
        self.cursors.extend_from_slice(&self.starts);
        for (item, &owner) in items.iter().zip(owners) {
            let cursor = &mut self.cursors[owner as usize];
            out[*cursor as usize] = *item;
            *cursor += 1;
        }
    }

    /// How to draw `batch` from a buffer of elements of `stride` bytes: the byte offset of its
    /// first instance, where to bind the buffer, and its number of instances.
    ///
    /// Draws always start at instance zero: MoltenVK rejects non-zero base instances on
    /// GPUs without them, such as the iOS simulator's.
    pub(crate) fn window(&self, id: u32, batch: Batch, stride: u32) -> (u64, u32) {
        let start = self.starts.get(id as usize).copied().unwrap_or(0);
        (u64::from(start) * u64::from(stride), batch.count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect::from_origin_size(Vec2::new(x, y), Vec2::new(width, height))
    }

    fn kinds(batches: &Batches) -> Vec<(PrimitiveKind, u32)> {
        batches
            .in_order()
            .map(|(_, batch)| (batch.kind, batch.count))
            .collect()
    }

    #[test]
    fn controls_in_columns_draw_backgrounds_then_labels() {
        let mut batches = Batches::default();
        // Two columns of fields, the second drawn after the whole first one.
        for column in 0..2_u8 {
            for row in 0..10_u8 {
                let (x, y) = (f32::from(column) * 300.0, f32::from(row) * 30.0);
                batches.join(PrimitiveKind::Rect, 0, rect(x, y, 200.0, 24.0));
                batches.join(PrimitiveKind::Glyph, 0, rect(x + 8.0, y + 6.0, 60.0, 12.0));
                batches.join(PrimitiveKind::Icon, 0, rect(x + 180.0, y + 4.0, 16.0, 16.0));
            }
        }
        assert_eq!(
            kinds(&batches),
            [
                (PrimitiveKind::Rect, 20),
                (PrimitiveKind::Glyph, 20),
                (PrimitiveKind::Icon, 20),
            ]
        );
    }

    #[test]
    fn what_overlaps_keeps_its_order() {
        let mut batches = Batches::default();
        let label = rect(10.0, 10.0, 50.0, 12.0);
        batches.join(PrimitiveKind::Rect, 0, rect(0.0, 0.0, 100.0, 30.0));
        batches.join(PrimitiveKind::Glyph, 0, label);
        // A highlight over the label: after it.
        batches.join(PrimitiveKind::Rect, 0, rect(20.0, 8.0, 10.0, 16.0));
        // Text over the highlight, then text beside everything: the latest glyphs take both.
        batches.join(PrimitiveKind::Glyph, 0, rect(20.0, 10.0, 8.0, 12.0));
        batches.join(PrimitiveKind::Glyph, 0, rect(500.0, 10.0, 8.0, 12.0));
        assert_eq!(
            kinds(&batches),
            [
                (PrimitiveKind::Rect, 1),
                (PrimitiveKind::Glyph, 1),
                (PrimitiveKind::Rect, 1),
                (PrimitiveKind::Glyph, 2),
            ]
        );
    }

    #[test]
    fn a_primitive_joins_its_type_past_batches_it_does_not_overlap() {
        let mut batches = Batches::default();
        batches.join(PrimitiveKind::Rect, 0, rect(0.0, 0.0, 10.0, 10.0));
        batches.join(PrimitiveKind::Line, 0, rect(100.0, 0.0, 10.0, 10.0));
        batches.join(PrimitiveKind::Glyph, 0, rect(200.0, 0.0, 10.0, 10.0));
        batches.join(PrimitiveKind::Icon, 0, rect(300.0, 0.0, 10.0, 10.0));
        // Under none of the three batches after the first.
        batches.join(PrimitiveKind::Rect, 0, rect(0.0, 100.0, 10.0, 10.0));
        assert_eq!(batches.len(), 4);
        // Under the segment: a batch of its own, after it.
        batches.join(PrimitiveKind::Rect, 0, rect(105.0, 5.0, 10.0, 10.0));
        assert_eq!(batches.len(), 5);
    }

    #[test]
    fn layers_batch_apart() {
        let mut batches = Batches::default();
        let area = rect(0.0, 0.0, 50.0, 50.0);
        batches.join(PrimitiveKind::Rect, 0, area);
        batches.join(PrimitiveKind::Rect, 1, area);
        // Over the menu of layer 1, but on the base layer: with the base's rectangles.
        batches.join(PrimitiveKind::Glyph, 1, area);
        batches.join(PrimitiveKind::Rect, 0, area);
        let layers: Vec<_> = batches
            .in_order()
            .map(|(_, batch)| (batch.layer, batch.kind, batch.count))
            .collect();
        assert_eq!(
            layers,
            [
                (0, PrimitiveKind::Rect, 2),
                (1, PrimitiveKind::Rect, 1),
                (1, PrimitiveKind::Glyph, 1),
            ]
        );
    }

    #[test]
    fn far_and_negative_coordinates_stay_in_the_grid() {
        let mut batches = Batches::default();
        batches.join(PrimitiveKind::Rect, 0, rect(-500.0, -500.0, 10.0, 10.0));
        batches.join(PrimitiveKind::Glyph, 0, rect(1.0e7, 1.0e7, 10.0, 10.0));
        batches.join(PrimitiveKind::Rect, 0, rect(1.0e7, 1.0e7, 10.0, 10.0));
        // The last one lies over the glyph, far as it is.
        assert_eq!(batches.len(), 3);
        batches.clear();
        assert!(batches.is_empty());
        assert_eq!(batches.grid.marked, []);
    }

    #[test]
    fn instances_are_gathered_batch_after_batch() {
        let mut batches = Batches::default();
        let owners = [
            batches.join(PrimitiveKind::Rect, 0, rect(0.0, 0.0, 10.0, 10.0)),
            batches.join(PrimitiveKind::Glyph, 0, rect(0.0, 0.0, 10.0, 10.0)),
            batches.join(PrimitiveKind::Rect, 0, rect(5.0, 5.0, 10.0, 10.0)),
            batches.join(PrimitiveKind::Rect, 0, rect(100.0, 0.0, 10.0, 10.0)),
        ];
        let mut order = DrawOrder::default();
        order.compute(&batches);
        // The rectangles' own owners: 0, 2 (over the glyph), 2 (beside it, latest of its type).
        let rect_owners = [owners[0], owners[2], owners[3]];
        let mut gathered = Vec::new();
        order.gather(&['a', 'b', 'c'], &rect_owners, &mut gathered);
        assert_eq!(gathered, ['a', 'b', 'c']);
        let windows: Vec<_> = batches
            .in_order()
            .map(|(id, batch)| order.window(id, *batch, 16))
            .collect();
        assert_eq!(windows, [(0, 1), (0, 1), (16, 2)]);
    }
}
