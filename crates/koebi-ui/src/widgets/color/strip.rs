//! [`ColorStrip`]: a strip of one color component (a hue, a value, an opacity), vertical or
//! horizontal, marked by two triangles pointing inward that pass its sides a little.
//!
//! A press sets the component where it lands and starts a drag: the pointer hides while the
//! mark follows its motion (finer with Shift), and comes back at the mark on release. Escape
//! during the drag brings the value back.

use koebi_core::input::ModifierKey;
use koebi_core::{Color, Rect, Vec2};
use koebi_render::{Fill, FillPrimitive, HueModel, Pointing};

use crate::id::WidgetId;
use crate::ui::{Response, Ui};
use crate::widgets::menu::MENU_CLOSE;

/// What a strip shows along its length, from its start (top or left) to its end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StripFill<'a> {
    /// Every hue, at full saturation, or at a lightness and chroma in OKLCH.
    Hues {
        /// The model of the hues.
        model: HueModel,
        /// The lightness of OKLCH hues.
        lightness: f32,
        /// The chroma of OKLCH hues.
        chroma: f32,
    },
    /// Colors evenly spaced along it, blended between them.
    Stops(&'a [Color]),
}

/// What a strip keeps between frames: its drag.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ColorStripState {
    /// While dragged: where the mark is along the strip, and the value before the drag.
    drag: Option<(f32, f32)>,
}

impl ColorStripState {
    /// At rest.
    pub const fn new() -> Self {
        Self { drag: None }
    }

    /// `true` while it is dragged.
    pub const fn is_dragged(&self) -> bool {
        self.drag.is_some()
    }
}

/// What happened to a strip this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StripResponse {
    /// The pointer's interaction with it.
    pub response: Response,
    /// The value changed.
    pub changed: bool,
}

/// A strip of one component, configured then shown with [`ColorStrip::show`].
#[derive(Debug)]
pub struct ColorStrip<'a> {
    id: WidgetId,
    state: &'a mut ColorStripState,
    /// Along the strip, from 0 at its start to 1 at its end.
    value: &'a mut f32,
    fill: StripFill<'a>,
    vertical: bool,
    checker: bool,
}

impl<'a> ColorStrip<'a> {
    /// A strip of `fill` editing `value` (0 at its start, 1 at its end), vertical.
    pub fn new(
        id: WidgetId,
        state: &'a mut ColorStripState,
        value: &'a mut f32,
        fill: StripFill<'a>,
    ) -> Self {
        Self {
            id,
            state,
            value,
            fill,
            vertical: true,
            checker: false,
        }
    }

    /// Vertical, its start at the top, or horizontal, its start at the left.
    #[must_use]
    pub fn vertical(mut self, vertical: bool) -> Self {
        self.vertical = vertical;
        self
    }

    /// Over a checkerboard: a strip of opacity.
    #[must_use]
    pub fn checker(mut self, checker: bool) -> Self {
        self.checker = checker;
        self
    }

    /// Shows the strip in `rect`; its marks pass its sides by the picker's overhang.
    pub fn show(self, ui: &mut Ui<'_>, rect: Rect) -> StripResponse {
        let response = ui.interact(self.id, rect);
        let style = ui.styles().color_picker;
        let length = if self.vertical {
            rect.height()
        } else {
            rect.width()
        }
        .max(1.0);
        let mut changed = false;
        let along = |point: Vec2| {
            if self.vertical {
                (point.y - rect.min.y) / length
            } else {
                (point.x - rect.min.x) / length
            }
        };
        if response.pressed
            && let Some(point) = ui.input().pointer()
        {
            let at = along(point).clamp(0.0, 1.0);
            self.state.drag = Some((at, *self.value));
            changed |= set(self.value, at);
        } else if let Some((at, before)) = self.state.drag {
            if ui.triggered(&MENU_CLOSE) {
                // Escape: the value comes back, the drag ends.
                ui.consume(&MENU_CLOSE);
                changed |= set(self.value, before);
                self.state.drag = None;
            } else if response.held {
                let motion = ui.input().motion();
                let fine = ui.input().modifiers().sides(ModifierKey::Shift).any();
                let factor = if fine { style.fine_factor } else { 1.0 };
                let moved = if self.vertical { motion.y } else { motion.x } / length * factor;
                let at = (at + moved).clamp(0.0, 1.0);
                self.state.drag = Some((at, before));
                changed |= set(self.value, at);
            } else {
                self.state.drag = None;
            }
        }
        let value = self.value.clamp(0.0, 1.0);
        let mark = if self.vertical {
            Vec2::new(rect.center().x, rect.min.y + value * rect.height())
        } else {
            Vec2::new(rect.min.x + value * rect.width(), rect.center().y)
        };
        if self.state.drag.is_some() {
            // The pointer hides while it drags, and comes back at the mark.
            ui.lock_pointer(mark);
        }
        self.draw(ui, rect, mark);
        StripResponse { response, changed }
    }

    fn draw(&self, ui: &mut Ui<'_>, rect: Rect, mark: Vec2) {
        let style = ui.styles().color_picker;
        let field = ui.styles().color_field;
        let radius = style
            .corner_radius
            .min(rect.width().min(rect.height()) * 0.5);
        ui.clipped_rounded(rect, koebi_render::Corners::all(radius), |ui| {
            if self.checker {
                ui.scene().push_fill(&FillPrimitive::new(
                    rect,
                    Fill::Checker {
                        size: field.checker_size,
                        light: field.checker_light,
                        dark: field.checker_dark,
                    },
                ));
            }
            match self.fill {
                StripFill::Hues {
                    model,
                    lightness,
                    chroma,
                } => {
                    ui.scene().push_fill(&FillPrimitive::new(
                        rect,
                        Fill::Hues {
                            model,
                            vertical: self.vertical,
                            lightness,
                            chroma,
                        },
                    ));
                }
                StripFill::Stops(stops) => stops_fill(ui, rect, stops, self.vertical),
            }
        });
        // Two triangles pointing inward, across the strip at the mark.
        let depth = style.arrow_size;
        let breadth = depth * 1.6;
        let (first, second) = if self.vertical {
            let x0 = rect.min.x - style.arrow_overhang;
            let x1 = rect.max.x + style.arrow_overhang;
            (
                (
                    Rect::from_min_max(
                        Vec2::new(x0, mark.y - breadth * 0.5),
                        Vec2::new(x0 + depth, mark.y + breadth * 0.5),
                    ),
                    Pointing::Right,
                ),
                (
                    Rect::from_min_max(
                        Vec2::new(x1 - depth, mark.y - breadth * 0.5),
                        Vec2::new(x1, mark.y + breadth * 0.5),
                    ),
                    Pointing::Left,
                ),
            )
        } else {
            let y0 = rect.min.y - style.arrow_overhang;
            let y1 = rect.max.y + style.arrow_overhang;
            (
                (
                    Rect::from_min_max(
                        Vec2::new(mark.x - breadth * 0.5, y0),
                        Vec2::new(mark.x + breadth * 0.5, y0 + depth),
                    ),
                    Pointing::Down,
                ),
                (
                    Rect::from_min_max(
                        Vec2::new(mark.x - breadth * 0.5, y1 - depth),
                        Vec2::new(mark.x + breadth * 0.5, y1),
                    ),
                    Pointing::Up,
                ),
            )
        };
        for (at, pointing) in [first, second] {
            ui.scene().push_fill(
                &FillPrimitive::new(
                    at,
                    Fill::Arrow {
                        pointing,
                        color: style.marker,
                        outline: style.marker_outline,
                        outline_width: style.outline_width,
                    },
                )
                .with_edges(koebi_render::Edges::Exact),
            );
        }
    }
}

/// Paints `stops`, evenly spaced along `rect` (from its top or its left), blended between them.
fn stops_fill(ui: &mut Ui<'_>, rect: Rect, stops: &[Color], vertical: bool) {
    let Some(&first) = stops.first() else {
        return;
    };
    if stops.len() == 1 {
        ui.scene().push_fill(&FillPrimitive::new(
            rect,
            Fill::Corners {
                top_left: first,
                top_right: first,
                bottom_right: first,
                bottom_left: first,
            },
        ));
        return;
    }
    let segments = (stops.len() - 1) as f32;
    for (index, pair) in stops.windows(2).enumerate() {
        let (start, end) = (index as f32 / segments, (index + 1) as f32 / segments);
        let (part, fill) = if vertical {
            let at = |t: f32| rect.min.y + t * rect.height();
            (
                Rect::from_min_max(
                    Vec2::new(rect.min.x, at(start)),
                    Vec2::new(rect.max.x, at(end)),
                ),
                Fill::Corners {
                    top_left: pair[0],
                    top_right: pair[0],
                    bottom_right: pair[1],
                    bottom_left: pair[1],
                },
            )
        } else {
            let at = |t: f32| rect.min.x + t * rect.width();
            (
                Rect::from_min_max(
                    Vec2::new(at(start), rect.min.y),
                    Vec2::new(at(end), rect.max.y),
                ),
                Fill::Corners {
                    top_left: pair[0],
                    top_right: pair[1],
                    bottom_right: pair[1],
                    bottom_left: pair[0],
                },
            )
        };
        ui.scene().push_fill(&FillPrimitive::new(part, fill));
    }
}

/// Sets `value` to `to`; `true` when it changed.
fn set(value: &mut f32, to: f32) -> bool {
    let changed = (*value - to).abs() > f32::EPSILON;
    *value = to;
    changed
}
