//! Dash layout: where the dashes of a subpath lie, as intervals of arc length.
//!
//! - Without anchors, the pattern runs from the subpath's start, shifted by the dash offset, as
//!   in SVG.
//! - With anchors, each one centers a dash or a gap of its own size, a whole number of patterns
//!   fits between two anchors, stretched by the [`DashFit`], and the pattern runs unscaled
//!   beyond the outer anchors. On a closed subpath the anchors go round: the last one is fitted
//!   to the first across the start.
//! - On a closed subpath, a dash running over the start is one dash.

use super::{DashCenter, DashFit, Dashes};

/// More dashes than this on one subpath draw it solid: a pattern far too fine to see.
const MAX_DASHES: f64 = 1_048_576.0;

/// The scratch of a layout, kept between subpaths so that dashing allocates once.
#[derive(Clone, Debug, Default)]
pub(super) struct Layout {
    /// The pattern, made even.
    pattern: Vec<f64>,
    /// The anchors of the subpath: position, element (0: the first dash, 1: the first gap),
    /// size.
    anchors: Vec<(f64, usize, f64)>,
    /// The dashes: intervals `(start, end)` of arc length, ascending.
    pub(super) dashes: Vec<(f64, f64)>,
}

impl Layout {
    /// Lays out the dashes of contour `contour`, of length `total`, in [`Self::dashes`]. On a
    /// closed subpath an interval may run over the start, with `end > total`. Returns whether
    /// the subpath is dashed: false without a pattern, or with one too fine or degenerate.
    pub(super) fn run(
        &mut self,
        dashes: &Dashes,
        contour: usize,
        total: f64,
        closed: bool,
    ) -> bool {
        self.dashes.clear();
        self.pattern.clear();
        if !(total.is_finite() && total > 0.0) {
            return false;
        }
        if dashes
            .pattern
            .iter()
            .any(|length| !(length.is_finite() && *length >= 0.0))
        {
            return false;
        }
        self.pattern.extend_from_slice(&dashes.pattern);
        if self.pattern.len() % 2 == 1 {
            self.pattern.extend_from_within(..);
        }
        let period: f64 = self.pattern.iter().sum();
        if self.pattern.is_empty() || period <= total * 1e-12 || total / period > MAX_DASHES {
            return false;
        }

        self.anchors.clear();
        for anchor in &dashes.anchors {
            if anchor.contour != contour || !anchor.at.is_finite() {
                continue;
            }
            let element = match anchor.center {
                DashCenter::Dash => 0,
                DashCenter::Gap => 1,
            };
            let size = anchor
                .size
                .filter(|size| size.is_finite() && *size >= 0.0)
                .unwrap_or(self.pattern[element]);
            self.anchors
                .push((anchor.at.clamp(0.0, 1.0) * total, element, size));
        }
        self.anchors.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));

        let anchored = !self.anchors.is_empty();
        if anchored {
            self.anchored(dashes.fit, period, total, closed);
        } else {
            self.plain(dashes.offset, period, total, closed);
        }
        self.normalize(total, closed, anchored);
        true
    }

    /// The pattern from the start, `offset` into it.
    fn plain(&mut self, offset: f64, period: f64, total: f64, closed: bool) {
        let phase = if offset.is_finite() {
            offset.rem_euclid(period)
        } else {
            0.0
        };
        let mut position = -phase;
        let mut element = 0;
        // On a closed subpath the end is the start: a dot there would be drawn twice.
        while position < total || (!closed && position <= total) {
            let length = self.pattern[element];
            // A dash of some length draws where it overlaps the subpath; a dot, where it is.
            let draws = if length > 0.0 {
                position + length > 0.0 && position < total
            } else {
                position >= 0.0
            };
            if element % 2 == 0 && draws {
                self.dashes.push((position, position + length));
            }
            position += length;
            element = (element + 1) % self.pattern.len();
        }
    }

    /// The pattern fitted between anchors.
    fn anchored(&mut self, fit: DashFit, period: f64, total: f64, closed: bool) {
        let feature =
            |(at, element, size): (f64, usize, f64)| (at - size * 0.5, at + size * 0.5, element);
        let first = feature(self.anchors[0]);
        let last = feature(self.anchors[self.anchors.len() - 1]);
        for index in 1..self.anchors.len() {
            let (a, b) = (
                feature(self.anchors[index - 1]),
                feature(self.anchors[index]),
            );
            self.fill_between(a.1, b.0, (a.2, b.2), fit, period);
        }
        if closed {
            // Round the start: from the last anchor to the first one, a lap further.
            self.fill_between(last.1, first.0 + total, (last.2, first.2), fit, period);
        } else {
            self.fill_before(first.0, first.2);
            self.fill_after(last.1, last.2, total);
        }
        for index in 0..self.anchors.len() {
            let (start, end, element) = feature(self.anchors[index]);
            if element == 0 {
                self.dashes.push((start, end));
            }
        }
    }

    /// From the element after `elements.0` to the one before `elements.1` (cyclic), plus as
    /// many whole patterns as fit best, stretched to span `start..end` exactly.
    fn fill_between(
        &mut self,
        start: f64,
        end: f64,
        elements: (usize, usize),
        fit: DashFit,
        period: f64,
    ) {
        let span = end - start;
        if span <= 0.0 {
            return;
        }
        let count = self.pattern.len();
        let element = |k: usize| (elements.0 + 1 + k) % count;
        // The elements between the two anchored ones, without a whole pattern.
        let base = (elements.1 + count - elements.0 - 1) % count;
        let base_length: f64 = (0..base).map(|k| self.pattern[element(k)]).sum();
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a count of patterns, at most the subpath's length over the period, which \
                      MAX_DASHES bounds"
        )]
        let mut repeats = ((span - base_length) / period).round().max(0.0) as usize;
        if base == 0 && repeats == 0 {
            // Two anchored elements that follow each other in the pattern, apart: a whole
            // pattern goes between them, stretched.
            repeats = 1;
        }
        let elements = base + repeats * count;
        let (mut dash_total, mut gap_total) = (0.0, 0.0);
        for k in 0..elements {
            let index = element(k);
            if index % 2 == 0 {
                dash_total += self.pattern[index];
            } else {
                gap_total += self.pattern[index];
            }
        }
        let natural = dash_total + gap_total;
        if natural <= 0.0 {
            return;
        }
        let both = span / natural;
        let (dash_scale, gap_scale) = match fit {
            DashFit::AdaptDashes if dash_total > 0.0 && span >= gap_total => {
                ((span - gap_total) / dash_total, 1.0)
            }
            DashFit::AdaptGaps if gap_total > 0.0 && span >= dash_total => {
                (1.0, (span - dash_total) / gap_total)
            }
            _ => (both, both),
        };
        let mut position = start;
        for k in 0..elements {
            let index = element(k);
            if index % 2 == 0 {
                let length = self.pattern[index] * dash_scale;
                self.dashes.push((position, position + length));
                position += length;
            } else {
                position += self.pattern[index] * gap_scale;
            }
        }
    }

    /// The unscaled pattern backward from `end`, the start of the anchored `element`, down to
    /// the subpath's start.
    fn fill_before(&mut self, end: f64, element: usize) {
        let count = self.pattern.len();
        let (mut position, mut index) = (end, element);
        while position > 0.0 {
            index = (index + count - 1) % count;
            let length = self.pattern[index];
            if index % 2 == 0 {
                self.dashes.push((position - length, position));
            }
            position -= length;
        }
    }

    /// The unscaled pattern forward from `start`, the end of the anchored `element`, up to the
    /// subpath's end.
    fn fill_after(&mut self, start: f64, element: usize, total: f64) {
        let count = self.pattern.len();
        let (mut position, mut index) = (start, element);
        while position < total {
            index = (index + 1) % count;
            let length = self.pattern[index];
            if index % 2 == 0 {
                self.dashes.push((position, position + length));
            }
            position += length;
        }
    }

    /// Clips the intervals to the subpath (anchored ones on a closed subpath wrap round it
    /// instead), sorts them, unites those that overlap or touch, and makes one dash of those on
    /// either side of a closed subpath's start.
    fn normalize(&mut self, total: f64, closed: bool, anchored: bool) {
        let dashes = &mut self.dashes;
        if closed && anchored {
            // Positions are cyclic: bring every start into the lap.
            for interval in dashes.iter_mut() {
                let shift = (interval.0 / total).floor() * total;
                interval.0 -= shift;
                interval.1 -= shift;
            }
        } else {
            dashes.retain_mut(|interval| {
                interval.0 = interval.0.max(0.0);
                interval.1 = interval.1.min(total);
                interval.1 >= interval.0
            });
        }
        dashes.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
        let mut kept = 0;
        for index in 0..dashes.len() {
            let interval = dashes[index];
            if kept > 0 && interval.0 <= dashes[kept - 1].1 {
                dashes[kept - 1].1 = dashes[kept - 1].1.max(interval.1);
            } else {
                dashes[kept] = interval;
                kept += 1;
            }
        }
        dashes.truncate(kept);
        if closed && kept > 1 {
            // The last dash runs over the start: it takes in the first ones it reaches.
            let reach = dashes[kept - 1].1 - total;
            let covered = dashes
                .iter()
                .take_while(|interval| interval.0 <= reach)
                .count();
            if covered > 0 {
                dashes[kept - 1].1 = dashes[covered - 1].1.max(reach) + total;
                dashes.drain(..covered);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stroke::DashAnchor;

    fn layout(dashes: &Dashes, total: f64, closed: bool) -> Vec<(f64, f64)> {
        let mut layout = Layout::default();
        assert!(layout.run(dashes, 0, total, closed));
        layout.dashes
    }

    #[track_caller]
    fn assert_intervals(found: &[(f64, f64)], expected: &[(f64, f64)]) {
        let close = found.len() == expected.len()
            && found
                .iter()
                .zip(expected)
                .all(|(a, b)| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9);
        assert!(close, "{found:?} != {expected:?}");
    }

    fn pattern(pattern: &[f64], offset: f64) -> Dashes {
        Dashes {
            pattern: pattern.to_vec(),
            offset,
            ..Dashes::default()
        }
    }

    fn anchored(anchors: &[(f64, DashCenter, Option<f64>)], fit: DashFit) -> Dashes {
        Dashes {
            pattern: vec![10.0, 5.0],
            fit,
            anchors: anchors
                .iter()
                .map(|&(at, center, size)| DashAnchor {
                    contour: 0,
                    at,
                    center,
                    size,
                })
                .collect(),
            ..Dashes::default()
        }
    }

    #[test]
    fn a_plain_pattern_runs_from_the_offset() {
        let found = layout(&pattern(&[10.0, 5.0], 3.0), 40.0, false);
        assert_intervals(&found, &[(0.0, 7.0), (12.0, 22.0), (27.0, 37.0)]);
        // An odd pattern repeats: 4 on, 2 off, 3 on, then 4 off, 2 on, 3 off.
        let found = layout(&pattern(&[4.0, 2.0, 3.0], 0.0), 18.0, false);
        assert_intervals(&found, &[(0.0, 4.0), (6.0, 9.0), (13.0, 15.0)]);
        // Zero-length dashes are dots, on both ends of an open subpath.
        let found = layout(&pattern(&[0.0, 5.0], 0.0), 10.0, false);
        assert_intervals(&found, &[(0.0, 0.0), (5.0, 5.0), (10.0, 10.0)]);
        // A pattern finer than a millionth of the subpath draws it solid.
        let mut layout = Layout::default();
        assert!(!layout.run(&pattern(&[1e-9, 1e-9], 0.0), 0, 1.0, false));
        assert!(!layout.run(&pattern(&[], 0.0), 0, 1.0, false));
    }

    #[test]
    fn a_dash_over_the_start_of_a_closed_subpath_is_one_dash() {
        // 10 on and 5 off over 45 from offset 3: the dash at 42 runs on into 0..7.
        let found = layout(&pattern(&[10.0, 5.0], 3.0), 45.0, true);
        assert_intervals(&found, &[(12.0, 22.0), (27.0, 37.0), (42.0, 52.0)]);
        // Without a dash at the end, the first one stays where it is.
        let found = layout(&pattern(&[10.0, 5.0], 3.0), 40.0, true);
        assert_intervals(&found, &[(0.0, 7.0), (12.0, 22.0), (27.0, 37.0)]);
        // No dot at the end of a closed subpath: it is the start.
        let found = layout(&pattern(&[0.0, 5.0], 0.0), 10.0, true);
        assert_intervals(&found, &[(0.0, 0.0), (5.0, 5.0)]);
    }

    #[test]
    fn anchors_center_their_dash_and_unscaled_patterns_fill_the_ends() {
        // Dashes centered at 20 and 80: the 50 between them holds exactly 3 dashes and 4 gaps.
        let dashes = anchored(
            &[(0.2, DashCenter::Dash, None), (0.8, DashCenter::Dash, None)],
            DashFit::ScaleBoth,
        );
        let found = layout(&dashes, 100.0, false);
        assert_intervals(
            &found,
            &[
                (0.0, 10.0),
                (15.0, 25.0),
                (30.0, 40.0),
                (45.0, 55.0),
                (60.0, 70.0),
                (75.0, 85.0),
                (90.0, 100.0),
            ],
        );
        // A gap of 20 centered at 50, the pattern unscaled on both sides.
        let dashes = anchored(&[(0.5, DashCenter::Gap, Some(20.0))], DashFit::ScaleBoth);
        let found = layout(&dashes, 100.0, false);
        assert_intervals(
            &found,
            &[
                (0.0, 10.0),
                (15.0, 25.0),
                (30.0, 40.0),
                (60.0, 70.0),
                (75.0, 85.0),
                (90.0, 100.0),
            ],
        );
    }

    #[test]
    fn each_fit_stretches_its_own_part() {
        // Dashes centered at 20 and 75: 45 between them for 3 dashes and 4 gaps (natural 50).
        let between = |fit| {
            let dashes = anchored(
                &[
                    (0.2, DashCenter::Dash, None),
                    (0.75, DashCenter::Dash, None),
                ],
                fit,
            );
            let found = layout(&dashes, 100.0, false);
            found[2..5].to_vec()
        };
        assert_intervals(
            &between(DashFit::ScaleBoth),
            &[(29.5, 38.5), (43.0, 52.0), (56.5, 65.5)],
        );
        let third = 25.0 / 3.0;
        assert_intervals(
            &between(DashFit::AdaptDashes),
            &[
                (30.0, 30.0 + third),
                (35.0 + third, 35.0 + 2.0 * third),
                (40.0 + 2.0 * third, 65.0),
            ],
        );
        assert_intervals(
            &between(DashFit::AdaptGaps),
            &[(28.75, 38.75), (42.5, 52.5), (56.25, 66.25)],
        );
        // A dash then a gap, which follow each other in the pattern: a whole pattern between.
        let dashes = anchored(
            &[(0.2, DashCenter::Dash, None), (0.5, DashCenter::Gap, None)],
            DashFit::ScaleBoth,
        );
        let found = layout(&dashes, 100.0, false);
        // 22.5 between 25 and 47.5 holds two patterns at 0.75: gap 3.75, dash 7.5, twice.
        assert_intervals(&found[2..4], &[(28.75, 36.25), (40.0, 47.5)]);
    }

    #[test]
    fn anchors_on_a_closed_subpath_go_round() {
        // One dash centered at 10 on a loop of 100. From its end round to its start, 90 hold
        // a gap and 6 whole patterns ((90 - 5) / 15 rounds to 6): 6 dashes and 7 gaps, 95
        // long, scaled by 90 / 95.
        let dashes = anchored(&[(0.1, DashCenter::Dash, None)], DashFit::ScaleBoth);
        let found = layout(&dashes, 100.0, true);
        let scale = 90.0 / 95.0;
        assert_eq!(found.len(), 7);
        assert!((found[0].0 - 5.0).abs() < 1e-9 && (found[0].1 - 15.0).abs() < 1e-9);
        let first_fill = 15.0 + 5.0 * scale;
        assert!((found[1].0 - first_fill).abs() < 1e-9, "{found:?}");
        // The last dash ends a gap before the anchored one, a lap later.
        let last_end = 105.0 - 5.0 * scale;
        assert!((found[6].1 - last_end).abs() < 1e-9, "{found:?}");
    }
}
