//! Hit testing by layers: what is on top under the pointer receives it, and nothing else.
//!
//! Every frame, widgets and zones register their rectangle in drawing order, on a [`Layer`].
//! The next frame resolves the pointer against them: the region on the highest layer wins,
//! then the one registered last. Resolving against the previous frame is what lets a
//! component drawn later (on top) take precedence over one drawn earlier, which an
//! immediate-mode frame cannot know while it draws; frames follow input, so the previous frame
//! is the layout the user is looking at.
//!
//! Zones are regions too: they carry a shortcut [`Context`] and nest, so that the chain of
//! contexts under the pointer reads innermost first, and they block what lies under them on
//! lower layers (a menu blocks the area below it).

use koebi_core::input::CursorShape;
use koebi_core::{Rect, Vec2};
use koebi_input::Context;

use crate::id::WidgetId;

/// A drawing layer: everything on a higher layer is above everything on a lower one, whatever
/// the drawing order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Layer(pub u8);

impl Layer {
    /// The workspace: areas, editors and their components.
    pub const BASE: Self = Self(0);
    /// Floating panels above the workspace: menus, popovers.
    pub const FLOATING: Self = Self(10);
    /// Transient overlays above everything: tooltips, drag previews.
    pub const OVERLAY: Self = Self(20);
}

/// A rectangle registered during a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Region {
    /// Where it is, clipped to the enclosing clip rectangle.
    pub(crate) rect: Rect,
    pub(crate) layer: Layer,
    /// The widget it belongs to; zones and blockers have none.
    pub(crate) widget: Option<WidgetId>,
    /// The shortcut context of a zone.
    pub(crate) context: Option<Context>,
    /// The cursor shown while the pointer is over it, or while its widget holds the pointer.
    pub(crate) cursor: Option<CursorShape>,
    /// The enclosing zone, by index.
    pub(crate) parent: Option<u32>,
}

/// The regions of one frame, in registration order.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Regions {
    list: Vec<Region>,
}

impl Regions {
    pub(crate) fn clear(&mut self) {
        self.list.clear();
    }

    /// Registers `region`; returns its index, the parent of the regions nested in it.
    pub(crate) fn push(&mut self, region: Region) -> u32 {
        self.list.push(region);
        u32::try_from(self.list.len() - 1).unwrap_or(u32::MAX)
    }

    /// The region on top at `point`: the highest layer, then the latest registered.
    pub(crate) fn topmost(&self, point: Vec2) -> Option<(u32, &Region)> {
        let mut best: Option<(u32, &Region)> = None;
        for (index, region) in self.list.iter().enumerate() {
            let above = best.is_none_or(|(_, top)| region.layer >= top.layer);
            if above && region.rect.contains(point) {
                best = Some((u32::try_from(index).unwrap_or(u32::MAX), region));
            }
        }
        best
    }

    /// The region of `widget`.
    pub(crate) fn of_widget(&self, widget: WidgetId) -> Option<&Region> {
        self.list
            .iter()
            .rev()
            .find(|region| region.widget == Some(widget))
    }

    /// The region at `index` and every enclosing zone, innermost first.
    pub(crate) fn ancestry(&self, index: u32) -> impl Iterator<Item = &Region> {
        let mut next = Some(index);
        core::iter::from_fn(move || {
            let region = self.list.get(usize::try_from(next?).ok()?)?;
            next = region.parent;
            Some(region)
        })
    }

    /// `true` when the region on top at `point` is `widget`'s, or lies in its zone.
    pub(crate) fn is_within(&self, point: Vec2, widget: WidgetId) -> bool {
        self.topmost(point).is_some_and(|(index, _)| {
            self.ancestry(index)
                .any(|region| region.widget == Some(widget))
        })
    }

    /// The contexts under `point`, innermost first, replacing the content of `chain`.
    pub(crate) fn contexts_at(&self, point: Vec2, chain: &mut Vec<Context>) {
        chain.clear();
        if let Some((index, _)) = self.topmost(point) {
            chain.extend(self.ancestry(index).filter_map(|region| region.context));
        }
    }

    /// The cursor asked for by the region on top at `point`, or by its zones.
    pub(crate) fn cursor_at(&self, point: Vec2) -> Option<CursorShape> {
        let (index, _) = self.topmost(point)?;
        self.ancestry(index).find_map(|region| region.cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect::from_origin_size(Vec2::new(x, y), Vec2::new(width, height))
    }

    fn region(rect: Rect, layer: Layer, widget: Option<&str>) -> Region {
        Region {
            rect,
            layer,
            widget: widget.map(WidgetId::new),
            context: None,
            cursor: None,
            parent: None,
        }
    }

    #[test]
    fn the_latest_region_of_the_highest_layer_is_on_top() {
        let mut regions = Regions::default();
        regions.push(region(
            rect(0.0, 0.0, 100.0, 100.0),
            Layer::BASE,
            Some("under"),
        ));
        regions.push(region(
            rect(20.0, 20.0, 50.0, 50.0),
            Layer::BASE,
            Some("over"),
        ));
        let menu = rect(40.0, 40.0, 50.0, 50.0);
        regions.push(region(menu, Layer::FLOATING, Some("menu")));
        // Drawn before the menu but on the base layer: below it.
        regions.push(region(
            rect(60.0, 60.0, 10.0, 10.0),
            Layer::BASE,
            Some("late"),
        ));
        let widget = |x, y| regions.topmost(Vec2::new(x, y)).and_then(|(_, r)| r.widget);
        assert_eq!(widget(10.0, 10.0), Some(WidgetId::new("under")));
        assert_eq!(widget(30.0, 30.0), Some(WidgetId::new("over")));
        assert_eq!(widget(65.0, 65.0), Some(WidgetId::new("menu")));
        assert_eq!(widget(200.0, 200.0), None);
    }

    #[test]
    fn context_chains_and_cursors_read_innermost_first() {
        let mut regions = Regions::default();
        let app = regions.push(Region {
            context: Some(Context::new("app")),
            ..region(rect(0.0, 0.0, 100.0, 100.0), Layer::BASE, None)
        });
        let area = regions.push(Region {
            context: Some(Context::new("area")),
            cursor: Some(CursorShape::Crosshair),
            parent: Some(app),
            ..region(rect(0.0, 0.0, 50.0, 100.0), Layer::BASE, None)
        });
        regions.push(Region {
            parent: Some(area),
            cursor: Some(CursorShape::Pointer),
            ..region(rect(10.0, 10.0, 10.0, 10.0), Layer::BASE, Some("button"))
        });
        let mut chain = Vec::new();
        regions.contexts_at(Vec2::new(15.0, 15.0), &mut chain);
        assert_eq!(chain, [Context::new("area"), Context::new("app")]);
        regions.contexts_at(Vec2::new(75.0, 50.0), &mut chain);
        assert_eq!(chain, [Context::new("app")]);
        assert_eq!(
            regions.cursor_at(Vec2::new(15.0, 15.0)),
            Some(CursorShape::Pointer)
        );
        assert_eq!(
            regions.cursor_at(Vec2::new(30.0, 30.0)),
            Some(CursorShape::Crosshair)
        );
        assert_eq!(regions.cursor_at(Vec2::new(75.0, 50.0)), None);
    }
}
