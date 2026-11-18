//! [`CellStore`]: the cells views draw, prepared once per outline and octave and kept from
//! render to render in one array of texels, which the GPU mirrors by range (ADR 0029).
//!
//! - **By outline:** an outline is known by its address, kept alive by the store so that the
//!   address names it. The evaluator gives an object it did not touch the very outline it had,
//!   a recolored one too (`koebi_doc::eval`): an edit prepares the outlines it changed and no
//!   other.
//! - **By octave:** cells follow the zoom's octave. An outline keeps the cells of its last
//!   [`OCTAVES`] octaves, so a zoom back and forth across an octave's edge prepares nothing
//!   again.
//! - **Around the view:** an outline is prepared whole when it lies within the view's area
//!   grown by the view's size on each side, else over that grown area. A pan prepares again
//!   only outlines too large for that, once the view leaves their grown area. Outlines outside
//!   the view are not prepared at all.
//! - **Records** are appended one after another. What no render drew for [`FORGET_AFTER`]
//!   renders is forgotten, by a sweep every [`FORGET_AFTER`] renders. Once forgotten records take more than half the store, the live
//!   ones are moved together: they name nothing outside themselves (`cells.rs`), so moving them
//!   changes no texel of theirs, and the GPU receives the store again.

use std::ops::Range;
use std::sync::Arc;

use koebi_core::hash::IdMap;
use koebi_geom::{Contours, Octave, Point, Rect, Vector, fill_cells};

use crate::cells::{DATA_WIDTH, encode};

/// The most rows of texels the store holds.
pub(crate) const ROWS: u32 = 4096;
/// The most texels the store holds: [`ROWS`] rows, 256 MiB. Record indices stay below 2²⁴,
/// exact in the `f32` of an instance.
pub(crate) const CAPACITY: usize = DATA_WIDTH as usize * ROWS as usize;
/// Renders after which what no render used is forgotten, and between two sweeps.
pub(crate) const FORGET_AFTER: u64 = 64;
/// Octaves an outline keeps cells for.
pub(crate) const OCTAVES: usize = 2;
/// Forgotten texels below which records are never moved: moving costs a whole upload.
const MOVE_AFTER: usize = 1 << 16;

/// One cell of an outline as views draw it: its origin in the document and its record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PreparedCell {
    /// Its corner of least x and y, in document units, exact.
    pub(crate) origin: Point,
    /// Where its record starts in the store.
    pub(crate) record: u32,
}

/// An outline's cells at one octave.
#[derive(Debug)]
struct Prepared {
    octave: Octave,
    /// The part of the document it was prepared over; `None` when the whole outline was.
    region: Option<Rect>,
    cells: Vec<PreparedCell>,
    /// Its records: texels `start..start + len`, one after another.
    start: usize,
    len: usize,
    /// The last render that drew it.
    used: u64,
}

/// What the store keeps of one outline.
#[derive(Debug)]
struct Kept {
    /// The outline: kept, so that its address names it.
    outline: Arc<Contours>,
    bounds: Rect,
    /// Its cells by octave, [`OCTAVES`] at most.
    prepared: Vec<Prepared>,
    /// The last render that showed it, drawn or culled.
    used: u64,
}

impl Kept {
    /// Keeps `prepared` in place of the cells of its octave over another region, else of the
    /// octave used longest ago once there are [`OCTAVES`], counting what it replaces in
    /// `wasted`; returns where it lies.
    fn keep(&mut self, prepared: Prepared, wasted: &mut usize) -> usize {
        let replaced = self
            .prepared
            .iter()
            .position(|other| other.octave == prepared.octave)
            .or_else(|| {
                (self.prepared.len() >= OCTAVES)
                    .then(|| {
                        self.prepared
                            .iter()
                            .enumerate()
                            .min_by_key(|(_, other)| other.used)
                            .map(|(at, _)| at)
                    })
                    .flatten()
            });
        if let Some(at) = replaced {
            *wasted += self.prepared[at].len;
            self.prepared[at] = prepared;
            at
        } else {
            self.prepared.push(prepared);
            self.prepared.len() - 1
        }
    }
}

/// What a canvas did since its stats were last reset: for tests and benchmarks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CanvasStats {
    /// Renders that recorded drawing; a steady view records none.
    pub rendered: u64,
    /// Outlines prepared at an octave.
    pub prepared: u64,
    /// Cells prepared.
    pub cells: u64,
    /// Strokes in pixels outlined at a zoom level.
    pub outlined: u64,
    /// Texels sent to the GPU, the store's header included.
    pub uploaded: u64,
    /// Times the live records were moved together.
    pub moves: u64,
}

/// The store is full: nothing was added.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Full;

/// The cells of the outlines views draw, and their records (see the module).
#[derive(Debug)]
pub(crate) struct CellStore {
    /// Texel 0 is the header, `0, length, 0, 0`: the shader reads no texel past the length.
    data: Vec<[f32; 4]>,
    kept: IdMap<usize, Kept>,
    /// Texels of records forgotten.
    wasted: usize,
    /// The texels the GPU holds as they are here: all but the header up to this one, unless
    /// `moved`.
    uploaded: usize,
    /// Records moved since the last upload: everything goes up again.
    moved: bool,
    /// Renders so far: the current one.
    clock: u64,
    /// The render that last forgot what was not used.
    swept: u64,
    pub(crate) stats: CanvasStats,
}

impl Default for CellStore {
    fn default() -> Self {
        Self {
            data: vec![[0.0, 1.0, 0.0, 0.0]],
            kept: IdMap::default(),
            wasted: 0,
            uploaded: 1,
            moved: true,
            clock: 0,
            swept: 0,
            stats: CanvasStats::default(),
        }
    }
}

/// `rect` grown by its own size on each side.
fn grown(rect: Rect) -> Rect {
    let size = Vector::new(rect.width(), rect.height());
    Rect::new(rect.min - size, rect.max + size)
}

impl CellStore {
    /// The texels, the header first.
    pub(crate) fn data(&self) -> &[[f32; 4]] {
        &self.data
    }

    /// Starts a render: what earlier renders prepared stays, what none used for
    /// [`FORGET_AFTER`] renders is forgotten.
    pub(crate) fn begin(&mut self) {
        self.clock += 1;
        if self.clock - self.swept >= FORGET_AFTER {
            self.forget_unused();
        }
        if self.wasted >= MOVE_AFTER && 2 * self.wasted > self.data.len() {
            self.move_together();
        }
    }

    /// The bounds of `outline` and its cells at `octave` that a view of `area` needs, prepared
    /// unless kept: none when the outline lies outside the area.
    ///
    /// Fails, adding nothing, when the store has no room for the outline's records.
    pub(crate) fn cells(
        &mut self,
        outline: &Arc<Contours>,
        octave: Octave,
        area: Rect,
    ) -> Result<(Rect, &[PreparedCell]), Full> {
        let Self {
            data,
            kept,
            wasted,
            clock,
            stats,
            ..
        } = self;
        let kept = kept
            .entry(Arc::as_ptr(outline) as usize)
            .or_insert_with(|| Kept {
                outline: Arc::clone(outline),
                bounds: outline.bounds(),
                prepared: Vec::new(),
                used: *clock,
            });
        debug_assert!(Arc::ptr_eq(&kept.outline, outline));
        kept.used = *clock;
        let bounds = kept.bounds;
        if !bounds.intersects(area) {
            return Ok((bounds, &[]));
        }
        let at = kept.prepared.iter().position(|prepared| {
            prepared.octave == octave
                && prepared
                    .region
                    .is_none_or(|region| region.contains_rect(area))
        });
        let at = if let Some(at) = at {
            at
        } else {
            let prepared = prepare(outline, bounds, octave, area, data)?;
            stats.prepared += 1;
            stats.cells += prepared.cells.len() as u64;
            kept.keep(prepared, wasted)
        };
        let prepared = &mut kept.prepared[at];
        prepared.used = *clock;
        Ok((bounds, &prepared.cells))
    }

    /// What the GPU must receive besides the header: the texels added since the last upload,
    /// or all of them once records moved.
    pub(crate) fn pending(&self) -> Range<usize> {
        if self.moved {
            1..self.data.len()
        } else {
            self.uploaded..self.data.len()
        }
    }

    /// Records that the GPU received the header and [`Self::pending`], or the whole store when
    /// `all` (a texture made anew).
    pub(crate) fn uploaded(&mut self, all: bool) {
        let sent = if all || self.moved {
            self.data.len()
        } else {
            1 + self.data.len() - self.uploaded
        };
        self.stats.uploaded += sent as u64;
        self.uploaded = self.data.len();
        self.moved = false;
    }

    /// Forgets every record: after [`Full`], the render prepares what it needs again.
    pub(crate) fn clear(&mut self) {
        self.kept.clear();
        self.data.truncate(1);
        self.data[0][1] = 1.0;
        self.wasted = 0;
        self.moved = true;
    }

    /// Forgets the outlines and octaves no render used for [`FORGET_AFTER`] renders.
    fn forget_unused(&mut self) {
        let clock = self.clock;
        let unused = |used: u64| used + FORGET_AFTER <= clock;
        let wasted = &mut self.wasted;
        self.kept.retain(|_, kept| {
            if unused(kept.used) {
                *wasted += kept
                    .prepared
                    .iter()
                    .map(|prepared| prepared.len)
                    .sum::<usize>();
                return false;
            }
            kept.prepared.retain(|prepared| {
                if unused(prepared.used) {
                    *wasted += prepared.len;
                    false
                } else {
                    true
                }
            });
            true
        });
        self.swept = clock;
    }

    /// Moves the live records together, dropping the forgotten ones.
    pub(crate) fn move_together(&mut self) {
        let mut data = Vec::with_capacity(self.data.len() - self.wasted);
        data.push(self.data[0]);
        for kept in self.kept.values_mut() {
            for prepared in &mut kept.prepared {
                let start = data.len();
                data.extend_from_slice(&self.data[prepared.start..prepared.start + prepared.len]);
                for cell in &mut prepared.cells {
                    #[allow(
                        clippy::cast_possible_truncation,
                        reason = "texel indices below CAPACITY, 2^24"
                    )]
                    let record = (cell.record as usize - prepared.start + start) as u32;
                    cell.record = record;
                }
                prepared.start = start;
            }
        }
        self.data = data;
        #[allow(clippy::cast_precision_loss, reason = "a length below 2^24")]
        {
            self.data[0][1] = self.data.len() as f32;
        }
        self.wasted = 0;
        self.moved = true;
        self.stats.moves += 1;
    }
}

/// The cells of `outline` (of `bounds`) at `octave` for a view of `area`, their records
/// appended to `data`: over the whole outline when it lies within the area grown by its size,
/// else over that grown area.
fn prepare(
    outline: &Contours,
    bounds: Rect,
    octave: Octave,
    area: Rect,
    data: &mut Vec<[f32; 4]>,
) -> Result<Prepared, Full> {
    let (size, margin) = (octave.cell_size(), octave.margin());
    let around = grown(area);
    let whole = bounds.inflated(margin);
    let (over, region) = if around.contains_rect(whole) {
        (whole, None)
    } else {
        (around.intersection(whole), Some(around))
    };
    let start = data.len();
    let cells: Vec<PreparedCell> = fill_cells(outline, octave, over)
        .iter()
        .map(|cell| PreparedCell {
            origin: cell.origin,
            #[allow(
                clippy::cast_possible_truncation,
                reason = "checked below against CAPACITY, 2^24"
            )]
            record: encode(cell, size, margin, data) as u32,
        })
        .collect();
    if data.len() > CAPACITY {
        data.truncate(start);
        return Err(Full);
    }
    #[allow(clippy::cast_precision_loss, reason = "a length below 2^24")]
    {
        data[0][1] = data.len() as f32;
    }
    Ok(Prepared {
        octave,
        region,
        cells,
        start,
        len: data.len() - start,
        used: 0,
    })
}

#[cfg(test)]
mod tests {
    use koebi_geom::{Path, Point};

    use super::*;

    fn square(x: f64, side: f64) -> Arc<Contours> {
        Arc::new(
            Path::rectangle(Rect::new(Point::new(x, 0.0), Point::new(x + side, side))).contours(),
        )
    }

    fn area(x0: f64, x1: f64) -> Rect {
        Rect::new(Point::new(x0, -10.0), Point::new(x1, 10.0))
    }

    #[test]
    fn an_outline_is_prepared_once_per_octave_and_kept() {
        let mut store = CellStore::default();
        let shape = square(0.0, 4.0);
        let octave = Octave(0);
        store.begin();
        let (bounds, cells) = store.cells(&shape, octave, area(-5.0, 5.0)).unwrap();
        assert_eq!(bounds, shape.bounds());
        let first = cells.to_vec();
        assert_ne!(first.len(), 0);
        // Panning, and another octave: prepared once each, then kept.
        store.begin();
        store.cells(&shape, octave, area(-2.0, 8.0)).unwrap();
        store.cells(&shape, Octave(3), area(-2.0, 8.0)).unwrap();
        store.cells(&shape, Octave(3), area(-2.0, 8.0)).unwrap();
        assert_eq!(store.stats.prepared, 2);
        assert_eq!(
            store.cells(&shape, octave, area(-1.0, 5.0)).unwrap().1,
            first
        );
        // A third octave pushes out the one used longest ago.
        store.begin();
        store.cells(&shape, Octave(3), area(-2.0, 8.0)).unwrap();
        store.cells(&shape, Octave(5), area(-2.0, 8.0)).unwrap();
        store.cells(&shape, Octave(3), area(-2.0, 8.0)).unwrap();
        assert_eq!(store.stats.prepared, 3);
        store.cells(&shape, octave, area(-2.0, 8.0)).unwrap();
        assert_eq!(store.stats.prepared, 4);
        // Outside the view: nothing prepared, the bounds known.
        let far = square(1000.0, 4.0);
        let (bounds, cells) = store.cells(&far, octave, area(-5.0, 5.0)).unwrap();
        assert!(cells.is_empty() && bounds == far.bounds());
        assert_eq!(store.stats.prepared, 4);
    }

    #[test]
    fn a_large_outline_is_prepared_around_the_view_and_again_once_left() {
        let mut store = CellStore::default();
        // 10 000 units wide, seen 100 at a time at octave 0 (cells of 1024).
        let wide = square(0.0, 10_000.0);
        let octave = Octave(0);
        store.begin();
        let cells = store
            .cells(&wide, octave, area(0.0, 100.0))
            .unwrap()
            .1
            .len();
        // The view grown by its size each way covers -100..200: the cells around the corner.
        assert!((1..=4).contains(&cells), "{cells}");
        // Within the grown area: kept; past it, prepared again.
        store.cells(&wide, octave, area(50.0, 150.0)).unwrap();
        assert_eq!(store.stats.prepared, 1);
        store.cells(&wide, octave, area(2000.0, 2100.0)).unwrap();
        assert_eq!(store.stats.prepared, 2);
        // The records replaced are forgotten.
        assert!(store.wasted > 0);
    }

    #[test]
    fn records_moved_together_keep_their_texels() {
        let mut store = CellStore::default();
        let shapes: Vec<_> = (0..40).map(|i| square(f64::from(i) * 10.0, 4.0)).collect();
        let octave = Octave(0);
        store.begin();
        let view = area(-10.0, 400.0);
        for shape in &shapes {
            store.cells(shape, octave, view).unwrap();
        }
        // A record whole: its header, its bands, the curves they name.
        let record_of = |store: &mut CellStore, shape: &Arc<Contours>| {
            let cell = store.cells(shape, octave, view).unwrap().1[0];
            let start = cell.record as usize;
            let head = store.data[start];
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "counts held in f32"
            )]
            let (bands, curves) = {
                let bands = (head[0] + head[1]) as usize;
                let curves: usize = store.data[start + 1..=start + bands]
                    .iter()
                    .map(|band| band[1] as usize)
                    .sum();
                (bands, curves)
            };
            store.data[start..start + 1 + bands + 2 * curves].to_vec()
        };
        let before = record_of(&mut store, &shapes[39]);
        // Half the shapes are no longer drawn: forgotten at the sweep after FORGET_AFTER renders.
        for _ in 0..2 * FORGET_AFTER {
            store.begin();
            for shape in &shapes[20..] {
                store.cells(shape, octave, view).unwrap();
            }
        }
        assert!(store.wasted > 0 && store.kept.len() == 20);
        let length = store.data.len();
        store.move_together();
        assert_eq!(store.pending(), 1..store.data.len());
        assert!(store.data.len() < length);
        assert_eq!(record_of(&mut store, &shapes[39]), before);
        assert_eq!(store.stats.prepared, 40);
        // Uploaded: nothing pending but what is added next.
        store.uploaded(false);
        assert!(store.pending().is_empty());
        store.cells(&shapes[0], octave, view).unwrap();
        assert_eq!(
            store.pending().start,
            store.data.len() - store.pending().len()
        );
        assert!(!store.pending().is_empty());
    }
}
