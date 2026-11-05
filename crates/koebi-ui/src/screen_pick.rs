//! Picking a color on the screen: an eyedropper's session, for the whole application.
//!
//! A component starts a session ([`Ui::start_screen_pick`]); while it lasts, every window of the
//! application picks: the application calls [`Ui::screen_pick_layer`] over each window once per
//! frame, last, and tells the other windows a session is going on elsewhere
//! ([`UiState::set_pick_elsewhere`]), routing what they pick back to the window that started it
//! ([`UiState::take_foreign_pick`], [`UiState::deliver_pick`]).
//!
//! The layer is an operation, as an open menu takes the input: nothing below it is hovered, the
//! press that picks reaches nothing below and neither does its release, title bars stop moving
//! the window, and the operation's keys apply ([`crate::OPERATION`]): its cancel (Escape,
//! Delete, the secondary button) gives up, its confirm (Enter) picks the pixel under the
//! pointer. A round magnifier shows the pixels read back around the pointer, the one picked
//! always at its center; it keeps clear of the pointer and inside the window, sliding along its
//! edges rather than jumping to the other side.

use core::time::Duration;

use koebi_core::input::CursorShape;
use koebi_core::{Color, Rect, Vec2};
use koebi_render::{Corners, RectPrimitive};

use crate::id::WidgetId;
use crate::input::PointerButton;
use crate::operation::OperationEnd;
use crate::routing::Layer;
use crate::ui::{EYEDROPPER_PIXELS, ScreenPixels, Ui, UiState};

/// The identity of the layer that catches the pointer while picking.
const LAYER: WidgetId = WidgetId::new("screen-pick");

/// What a session ended with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickOutcome {
    /// The pixel picked, gamma-encoded RGBA as the screen showed it.
    Picked([u8; 4]),
    /// Given up, or nothing to pick from.
    Cancelled,
}

/// A window's part of the session.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PickState {
    /// The component that started a session in this window, while it lasts.
    owner: Option<WidgetId>,
    /// A session started in another window: this one picks too.
    elsewhere: bool,
    /// After the press that ended picking: the pointer stays held until the buttons are up, so
    /// that the release reaches nothing either.
    releasing: bool,
    /// What this window picked for a session started elsewhere, until the application routes
    /// it.
    foreign: Option<PickOutcome>,
    /// What the owner gets, and how many layers passed since without it reading it.
    delivered: Option<PickOutcome>,
    unread: u8,
}

impl UiState {
    /// `true` while a component of this window holds a session: the other windows pick too.
    pub fn picks_here(&self) -> bool {
        self.pick.owner.is_some()
    }

    /// Whether a session started in another window is going on. Returns `true` when that
    /// changes, and the window should be drawn again.
    pub fn set_pick_elsewhere(&mut self, elsewhere: bool) -> bool {
        let changed = self.pick.elsewhere != elsewhere;
        self.pick.elsewhere = elsewhere;
        if changed {
            self.forget_pixels();
            if !elsewhere {
                self.pick.foreign = None;
            }
        }
        changed
    }

    /// What this window picked for a session started in another one, once.
    pub fn take_foreign_pick(&mut self) -> Option<PickOutcome> {
        self.pick.foreign.take()
    }

    /// Gives the session's owner here what another window picked.
    pub fn deliver_pick(&mut self, outcome: PickOutcome) {
        if self.pick.owner.is_some() {
            self.pick.delivered = Some(outcome);
            self.pick.unread = 0;
        }
    }
}

impl Ui<'_> {
    /// Starts a session owned by `id`: every window of the application picks until
    /// [`Self::take_screen_pick`] hands `id` the outcome. One at a time: starting another ends
    /// the one before.
    pub fn start_screen_pick(&mut self, id: WidgetId) {
        let pick = &mut self.state.pick;
        pick.owner = Some(id);
        pick.delivered = None;
        pick.unread = 0;
        self.state.forget_pixels();
    }

    /// Ends the session `id` owns, picking nothing.
    pub fn cancel_screen_pick(&mut self, id: WidgetId) {
        if self.state.pick.owner == Some(id) {
            self.state.pick.owner = None;
            self.state.pick.delivered = None;
        }
    }

    /// `true` while `id` holds a session.
    pub fn is_screen_picking(&self, id: WidgetId) -> bool {
        self.state.pick.owner == Some(id)
    }

    /// The outcome of the session `id` holds, once, ending it.
    pub fn take_screen_pick(&mut self, id: WidgetId) -> Option<PickOutcome> {
        if self.state.pick.owner != Some(id) {
            return None;
        }
        let outcome = self.state.pick.delivered.take()?;
        self.state.pick.owner = None;
        Some(outcome)
    }

    /// The layer of a session over the whole of `area` (the window), drawn last in the frame:
    /// it catches the pointer, shows the magnifier and picks. Does nothing without a session.
    pub fn screen_pick_layer(&mut self, area: Rect) {
        if self.state.pick.releasing {
            // Nothing reacts to the release of the press that ended picking.
            self.hold_pointer();
            let down = [
                PointerButton::Primary,
                PointerButton::Secondary,
                PointerButton::Middle,
            ]
            .into_iter()
            .any(|button| self.input().is_button_down(button));
            self.state.pick.releasing = down;
        }
        // An outcome its owner no longer reads (the picker went): the session ends.
        if self.state.pick.delivered.is_some() {
            self.state.pick.unread = self.state.pick.unread.saturating_add(1);
            if self.state.pick.unread > 2 {
                self.state.pick = PickState {
                    releasing: self.state.pick.releasing,
                    ..PickState::default()
                };
            }
            return;
        }
        let here = self.state.pick.owner.is_some();
        if !(here || self.state.pick.elsewhere) || self.state.pick.foreign.is_some() {
            return;
        }
        if !self.can_read_screen() {
            self.end_pick(PickOutcome::Cancelled);
            return;
        }
        // Over everything: hovers, presses and title bars stop below it.
        let catcher = self.on_layer(Layer::OVERLAY, |ui| {
            ui.interact_with_cursor(LAYER, area, CursorShape::Crosshair)
        });
        match self.operation() {
            Some(OperationEnd::Cancelled) => {
                self.end_pick(PickOutcome::Cancelled);
                return;
            }
            Some(OperationEnd::Confirmed) => {
                let outcome = self
                    .center_pixel()
                    .map_or(PickOutcome::Cancelled, PickOutcome::Picked);
                self.end_pick(outcome);
                return;
            }
            None => {}
        }
        self.request_redraw_after(Duration::from_millis(16));
        let Some(pointer) = self.input().pointer() else {
            return;
        };
        self.want_screen_pixels(pointer);
        // Copied out of the interface's state, which the scene's borrow would hold otherwise.
        let magnified = self.screen_pixels().map(Magnified::of);
        if let Some(magnified) = &magnified {
            self.on_layer(Layer::OVERLAY, |ui| magnifier(ui, pointer, area, magnified));
        }
        if catcher.pressed {
            // A press always ends picking; the pixel at the magnifier's center is the one picked.
            let outcome = magnified
                .and_then(|magnified| magnified.center())
                .map_or(PickOutcome::Cancelled, PickOutcome::Picked);
            self.end_pick(outcome);
        }
    }

    /// The pixel read back at the center of the square: the one under the pointer when it was
    /// read.
    fn center_pixel(&self) -> Option<[u8; 4]> {
        let pixels = self.screen_pixels()?;
        pixels.at(pixels.center.0, pixels.center.1)
    }

    /// Ends this window's picking with `outcome`: for the owner here, or for the application to
    /// route to the window that started the session.
    fn end_pick(&mut self, outcome: PickOutcome) {
        let input = self.input();
        let down = [
            PointerButton::Primary,
            PointerButton::Secondary,
            PointerButton::Middle,
        ]
        .into_iter()
        .any(|button| input.is_button_down(button) || input.button_pressed_at(button).is_some());
        let pick = &mut self.state.pick;
        pick.releasing = down;
        if pick.owner.is_some() {
            pick.delivered = Some(outcome);
            pick.unread = 0;
        } else {
            pick.foreign = Some(outcome);
        }
        self.state.forget_pixels();
        if down {
            self.hold_pointer();
        }
    }
}

/// The most pixels a magnifier shows: the square an eyedropper reads.
const MAGNIFIED: usize = (EYEDROPPER_PIXELS * EYEDROPPER_PIXELS) as usize;

/// The pixels around the one picked, copied for one frame without allocating, laid out with
/// the picked one at the center: [`EYEDROPPER_PIXELS`] across, those the square read back did
/// not reach (at the window's edges) left empty.
#[derive(Clone, Copy, Debug)]
struct Magnified {
    pixels: [Option<[u8; 4]>; MAGNIFIED],
}

impl Magnified {
    fn of(screen: &ScreenPixels) -> Self {
        let side = EYEDROPPER_PIXELS;
        let half = side / 2;
        let mut pixels = [None; MAGNIFIED];
        for row in 0..side {
            for column in 0..side {
                let x = (screen.center.0 + column).checked_sub(half);
                let y = (screen.center.1 + row).checked_sub(half);
                if let (Some(x), Some(y)) = (x, y) {
                    pixels[(row * side + column) as usize] = screen.at(x, y);
                }
            }
        }
        Self { pixels }
    }

    fn get(&self, column: u32, row: u32) -> Option<[u8; 4]> {
        let side = EYEDROPPER_PIXELS;
        if column >= side || row >= side {
            return None;
        }
        self.pixels[(row * side + column) as usize]
    }

    /// The picked pixel, at the center.
    fn center(&self) -> Option<[u8; 4]> {
        let half = EYEDROPPER_PIXELS / 2;
        self.get(half, half)
    }
}

/// Where the magnifier's center lies for the pointer at `pointer`: below and right of it
/// (`clearance` from the pointer to the magnifier's center), inside `bounds` whose edges it
/// slides along, moving to the other side of the pointer smoothly as an edge comes near, and
/// never over the pointer.
pub(crate) fn magnifier_center(pointer: Vec2, bounds: Rect, radius: f32, clearance: f32) -> Vec2 {
    // In shares of the clearance from the pointer: where the center may lie for the magnifier
    // to stay inside the window.
    let reach = clearance.max(1.0);
    let low = Vec2::new(
        (bounds.min.x + radius - pointer.x) / reach,
        (bounds.min.y + radius - pointer.y) / reach,
    );
    let high = Vec2::new(
        (bounds.max.x - radius - pointer.x) / reach,
        (bounds.max.y - radius - pointer.y) / reach,
    );
    // Below and right by preference, as far as the window lets: past the far edge it moves to
    // the other side of the pointer, as the pointer moves.
    let prefer = |low: f32, high: f32| {
        if high >= low {
            1.0_f32.min(high).max(low)
        } else {
            f32::midpoint(low, high)
        }
    };
    let share = Vec2::new(prefer(low.x, high.x), prefer(low.y, high.y));
    if share.length() >= 1.0 {
        return pointer + share * reach;
    }
    // Near a corner it would come too close: the point of the circle around the pointer that
    // lies inside the window, nearest to the preference.
    let inside = |at: Vec2| {
        at.x >= low.x - 1e-4
            && at.x <= high.x + 1e-4
            && at.y >= low.y - 1e-4
            && at.y <= high.y + 1e-4
    };
    let mut best: Option<Vec2> = None;
    let mut consider = |at: Vec2| {
        if inside(at) && best.is_none_or(|kept| (kept - share).length() > (at - share).length()) {
            best = Some(at);
        }
    };
    if share.length() > 1e-3 {
        consider(share / share.length());
    }
    for x in [low.x, high.x] {
        if x.abs() <= 1.0 {
            let y = (1.0 - x * x).sqrt();
            consider(Vec2::new(x, y));
            consider(Vec2::new(x, -y));
        }
    }
    for y in [low.y, high.y] {
        if y.abs() <= 1.0 {
            let x = (1.0 - y * y).sqrt();
            consider(Vec2::new(x, y));
            consider(Vec2::new(-x, y));
        }
    }
    // A window too small to hold it clear of the pointer: the preference.
    pointer + best.unwrap_or(share) * reach
}

/// The round magnifier near `pointer` inside `bounds`: the pixels zoomed, the picked one
/// outlined at its center.
fn magnifier(ui: &mut Ui<'_>, pointer: Vec2, bounds: Rect, pixels: &Magnified) {
    let style = ui.styles().color_picker;
    let cell = (style.marker_size * 0.75).max(4.0).round();
    let side = EYEDROPPER_PIXELS as f32 * cell;
    let radius = side * 0.5;
    let clearance = radius + style.marker_size * 1.5;
    let center = magnifier_center(pointer, bounds, radius, clearance);
    let frame = Rect::from_center_size(center, Vec2::splat(side));
    let outline = style.outline_width;
    let rim = (outline * 2.0).max(1.0);
    // A rim, white outlined in black, so that the magnifier shows on any color.
    ui.scene().push_rect(
        &RectPrimitive::new(frame.inset(-rim - outline), style.marker_outline)
            .with_corner_radius(radius + rim + outline),
    );
    ui.scene().push_rect(
        &RectPrimitive::new(frame.inset(-rim), style.marker).with_corner_radius(radius + rim),
    );
    ui.clipped_rounded(frame, Corners::all(radius), |ui| {
        ui.scene()
            .push_rect(&RectPrimitive::new(frame, style.marker_outline));
        for row in 0..EYEDROPPER_PIXELS {
            for column in 0..EYEDROPPER_PIXELS {
                let Some([r, g, b, _]) = pixels.get(column, row) else {
                    continue;
                };
                let at = Rect::from_origin_size(
                    frame.min + Vec2::new(column as f32 * cell, row as f32 * cell),
                    Vec2::splat(cell),
                );
                ui.scene()
                    .push_rect(&RectPrimitive::new(at, Color::from_rgba8(r, g, b, 255)));
            }
        }
    });
    // The pixel picked, always at the center: outlined in black, then white.
    let picked = Rect::from_center_size(center, Vec2::splat(cell));
    ui.scene().push_rect(
        &RectPrimitive::new(picked.inset(-outline), Color::TRANSPARENT)
            .with_border(outline, style.marker_outline),
    );
    ui.scene().push_rect(
        &RectPrimitive::new(picked, Color::TRANSPARENT).with_border(outline, style.marker),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds() -> Rect {
        Rect::from_origin_size(Vec2::ZERO, Vec2::new(800.0, 600.0))
    }

    #[test]
    fn the_magnifier_lies_below_and_right_inside_the_window_and_never_over_the_pointer() {
        let (radius, clearance) = (50.0, 70.0);
        let inside = |center: Vec2| {
            center.x - radius >= -1e-3
                && center.y - radius >= -1e-3
                && center.x + radius <= 800.0 + 1e-3
                && center.y + radius <= 600.0 + 1e-3
        };
        // In the middle: below and right.
        let center = magnifier_center(Vec2::new(400.0, 300.0), bounds(), radius, clearance);
        assert_eq!(center, Vec2::new(470.0, 370.0));
        // Everywhere, sampled: inside the window, clear of the pointer.
        let mut y = 0.0;
        while y <= 600.0 {
            let mut x = 0.0;
            while x <= 800.0 {
                let pointer = Vec2::new(x, y);
                let center = magnifier_center(pointer, bounds(), radius, clearance);
                assert!(
                    inside(center) || x < radius || y < radius,
                    "{pointer:?} {center:?}"
                );
                assert!(
                    (center - pointer).length() >= clearance - 1e-3,
                    "{pointer:?}"
                );
                x += 10.0;
            }
            y += 10.0;
        }
    }

    #[test]
    fn the_magnifier_slides_smoothly_to_the_other_side_near_an_edge() {
        let (radius, clearance) = (50.0, 70.0);
        // Moving right toward the edge, one pixel at a time: it never jumps.
        let mut previous = magnifier_center(Vec2::new(500.0, 300.0), bounds(), radius, clearance);
        let mut x = 501.0;
        while x <= 800.0 {
            let center = magnifier_center(Vec2::new(x, 300.0), bounds(), radius, clearance);
            assert!(
                (center - previous).length() < 4.0,
                "jump at {x}: {previous:?} {center:?}"
            );
            previous = center;
            x += 1.0;
        }
        // At the edge, on the left of the pointer.
        assert!(previous.x < 800.0 - radius + 1e-3 && previous.x < 800.0);
    }

    #[test]
    fn the_picked_pixel_lies_at_the_center_even_at_the_window_s_edge() {
        // Read back at the top left corner: the square starts at the window's edge, the pixel
        // asked for is the top left one.
        let mut pixels = vec![[0, 0, 0, 255]; 121];
        pixels[0] = [255, 0, 0, 255];
        let screen = ScreenPixels {
            origin: (0, 0),
            center: (0, 0),
            width: 11,
            height: 11,
            pixels,
        };
        let magnified = Magnified::of(&screen);
        assert_eq!(magnified.center(), Some([255, 0, 0, 255]));
        // What lies past the edge stays empty.
        assert_eq!(magnified.get(0, 0), None);
    }
}
