//! [`Touches`]: the fingers on a touch screen, and what two or more of them did during a frame:
//! how far their middle moved and how much they spread apart. One finger alone is the pointer,
//! through the mouse events the platform makes of it.

use koebi_core::Vec2;

/// Fingers on a touch screen, and their gesture during a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Touches {
    fingers: [Option<(u64, Vec2)>; Self::CAPACITY],
    /// How far the middle of two or more fingers moved during the frame.
    moved: Vec2,
    /// How much two or more fingers spread apart during the frame: above 1 apart.
    spread: f32,
    /// The system reported this gesture's pinch itself: the spread is its, not ours.
    pinched: bool,
}

impl Default for Touches {
    fn default() -> Self {
        Self {
            fingers: [None; Self::CAPACITY],
            moved: Vec2::ZERO,
            spread: 1.0,
            pinched: false,
        }
    }
}

/// The middle of `fingers` and their mean distance to it.
fn middle_and_spread(fingers: impl Iterator<Item = Vec2> + Clone) -> (Vec2, f32) {
    let (sum, count) = fingers
        .clone()
        .fold((Vec2::ZERO, 0.0_f32), |(sum, count), at| {
            (sum + at, count + 1.0)
        });
    let middle = sum * (1.0 / count.max(1.0));
    let spread = fingers.map(|at| (at - middle).length()).sum::<f32>() / count.max(1.0);
    (middle, spread)
}

impl Touches {
    /// More fingers at once are ignored: ten hands hold.
    const CAPACITY: usize = 10;

    fn down(&self) -> impl Iterator<Item = Vec2> + Clone + '_ {
        self.fingers.iter().flatten().map(|(_, at)| *at)
    }

    /// How many fingers are down.
    pub(crate) fn count(&self) -> usize {
        self.fingers.iter().flatten().count()
    }

    /// The middle of the fingers down, if any.
    pub(crate) fn middle(&self) -> Option<Vec2> {
        (self.count() > 0).then(|| middle_and_spread(self.down()).0)
    }

    /// How far the middle of two or more fingers moved this frame.
    pub(crate) fn moved(&self) -> Vec2 {
        self.moved
    }

    /// How much two or more fingers spread apart this frame: 1 when the system reports the
    /// pinch itself.
    pub(crate) fn spread(&self) -> f32 {
        if self.pinched { 1.0 } else { self.spread }
    }

    /// The finger `id` touched at `at`.
    pub(crate) fn touched(&mut self, id: u64, at: Vec2) {
        let at_slot = self
            .fingers
            .iter()
            .position(|slot| slot.is_some_and(|(known, _)| known == id))
            .or_else(|| self.fingers.iter().position(Option::is_none));
        if let Some(index) = at_slot {
            self.fingers[index] = Some((id, at));
        }
    }

    /// The finger `id` moved to `at`. With another finger down, the gesture's middle and
    /// spread follow.
    pub(crate) fn moved_to(&mut self, id: u64, at: Vec2) {
        let several = self.count() >= 2;
        let (middle, spread) = middle_and_spread(self.down());
        let Some(slot) = self
            .fingers
            .iter_mut()
            .flatten()
            .find(|(known, _)| *known == id)
        else {
            return;
        };
        slot.1 = at;
        if several {
            let (now, apart) = middle_and_spread(self.down());
            self.moved += now - middle;
            // Fingers almost together tell no spread; the system's pinch tells its own.
            if !self.pinched && spread > 1.0 && apart > 1.0 {
                self.spread *= apart / spread;
            }
        }
    }

    /// The finger `id` left the screen. Under two fingers, the gesture ends.
    pub(crate) fn lifted(&mut self, id: u64) {
        for slot in &mut self.fingers {
            if slot.is_some_and(|(known, _)| known == id) {
                *slot = None;
            }
        }
        if self.count() < 2 {
            self.pinched = false;
        }
    }

    /// The system reported a pinch: while two or more fingers stay down, the spread is its.
    pub(crate) fn pinched(&mut self) {
        if self.count() >= 2 {
            self.pinched = true;
        }
    }

    /// The same fingers, before any motion of the next frame.
    pub(crate) fn settled(&self) -> Self {
        Self {
            moved: Vec2::ZERO,
            spread: 1.0,
            ..*self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_fingers_pan_by_their_middle_and_zoom_by_their_spread() {
        let mut touches = Touches::default();
        touches.touched(1, Vec2::new(100.0, 100.0));
        // One finger alone is the pointer: no gesture.
        touches.moved_to(1, Vec2::new(110.0, 100.0));
        assert_eq!(touches.moved(), Vec2::ZERO);
        touches.touched(2, Vec2::new(210.0, 100.0));
        assert_eq!(touches.count(), 2);
        assert_eq!(touches.middle(), Some(Vec2::new(160.0, 100.0)));
        // Both move right by 20; then the second moves 50 further: apart by half again.
        touches.moved_to(1, Vec2::new(130.0, 100.0));
        touches.moved_to(2, Vec2::new(230.0, 100.0));
        assert_eq!(touches.moved(), Vec2::new(20.0, 0.0));
        assert!((touches.spread() - 1.0).abs() < 1e-6);
        touches.moved_to(2, Vec2::new(280.0, 100.0));
        assert!((touches.spread() - 1.5).abs() < 1e-5);
        // The next frame starts afresh; a system pinch takes the spread over.
        let mut next = touches.settled();
        assert_eq!(next.moved(), Vec2::ZERO);
        next.pinched();
        next.moved_to(2, Vec2::new(430.0, 100.0));
        assert!((next.spread() - 1.0).abs() < 1e-6);
        // A finger lifted ends the gesture, the system's pinch with it.
        next.lifted(2);
        assert_eq!(next.count(), 1);
        next.touched(3, Vec2::new(300.0, 100.0));
        next.moved_to(3, Vec2::new(400.0, 100.0));
        assert!(next.spread() > 1.0);
    }

    #[test]
    fn a_finger_touching_again_takes_its_own_place() {
        let mut touches = Touches::default();
        touches.touched(4, Vec2::ZERO);
        touches.touched(4, Vec2::new(5.0, 5.0));
        assert_eq!(touches.count(), 1);
        for id in 0..20 {
            touches.touched(100 + id, Vec2::ZERO);
        }
        assert_eq!(touches.count(), Touches::CAPACITY);
        touches.lifted(999);
        assert_eq!(touches.count(), Touches::CAPACITY);
    }
}
