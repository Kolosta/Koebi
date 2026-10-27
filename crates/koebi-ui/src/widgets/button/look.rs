//! How a button looks: its family of tokens ([`Look`]: a button's, a toggle's, a
//! dropdown's), the metrics and colors they give before the call's overrides, the chevron of a
//! dropdown, and the corners squared where an open menu meets the button.

use koebi_core::Rect;
use koebi_render::Corners;

use super::{BACKGROUND, BORDER, Button, CONTENT};
use crate::ui::Ui;
use crate::widgets::dropdown::chevron;
use crate::widgets::parts::{Content, ContentMetrics, ControlState, Side, Tone, ToneColors};

/// Which tokens draw a button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Look {
    /// `component.button.*`.
    Button,
    /// `component.toggle.*`: a toggle of the default tone.
    Toggle,
    /// `component.dropdown.*`: the control of a [`Dropdown`](crate::Dropdown).
    Dropdown,
}

/// The metrics a look gives the button, before the call's overrides.
#[derive(Clone, Copy, Debug)]
pub(super) struct Defaults {
    pub(super) height: f32,
    pub(super) min_width: f32,
    pub(super) padding: f32,
    pub(super) padding_vertical: f32,
    pub(super) gap: f32,
    pub(super) corner_radius: f32,
    pub(super) border_width: f32,
    pub(super) icon_size: f32,
    pub(super) trailing_size: f32,
}

impl<'a> Button<'a> {
    /// The content as shown: a dropdown's chevron points where its menu opens, and back
    /// while it is open.
    pub(super) fn shown_content(&self) -> Content<'a> {
        let mut content = self.content;
        if self.look() == Look::Dropdown
            && content.trailing.is_some()
            && let Some((menu, placement)) = &self.menu
        {
            let open = menu
                .is_open()
                .then(|| menu.placed().map_or(placement.side, |placed| placed.side));
            content.trailing = Some(chevron(placement.side, open));
        }
        content
    }

    /// The tokens the button draws with: a dropdown's (whatever its tone, which only colors
    /// it), a toggle's of the default tone, or a button's.
    pub(crate) fn look(&self) -> Look {
        if self.look == Some(Look::Dropdown) {
            Look::Dropdown
        } else if self.toggle.is_some() && self.menu.is_none() && self.tone == Tone::DEFAULT {
            Look::Toggle
        } else {
            Look::Button
        }
    }

    /// The metrics of the look, before the call's overrides.
    pub(super) fn defaults(&self, ui: &Ui<'_>) -> Defaults {
        let styles = ui.styles();
        let button = &styles.button;
        let dropdown = &styles.dropdown;
        let base = Defaults {
            height: button.height,
            min_width: button.min_width,
            padding: button.padding,
            padding_vertical: 0.0,
            gap: button.gap,
            corner_radius: button.corner_radius,
            border_width: button.border_width,
            icon_size: button.icon_size,
            trailing_size: button.icon_size,
        };
        match self.look() {
            Look::Button => base,
            Look::Toggle => Defaults {
                corner_radius: button.toggle_corner_radius,
                border_width: button.toggle_border_width,
                ..base
            },
            Look::Dropdown => Defaults {
                height: dropdown.height,
                min_width: dropdown.min_width,
                padding: dropdown.padding,
                padding_vertical: dropdown.padding_vertical,
                gap: dropdown.gap,
                corner_radius: dropdown.corner_radius,
                border_width: dropdown.border_width,
                icon_size: dropdown.icon_size,
                trailing_size: dropdown.chevron_size,
            },
        }
    }

    /// The colors the button draws with: its look's or its tone's, with the overrides of the
    /// call.
    pub fn colors(&self, ui: &Ui<'_>) -> ToneColors {
        let styles = ui.styles();
        let mut colors = match self.look() {
            Look::Toggle => styles.button.toggle,
            Look::Dropdown if self.tone == Tone::DEFAULT => styles.dropdown.colors,
            Look::Dropdown | Look::Button => *styles.button.colors(self.tone),
        };
        for (part, overrides) in self.colors.iter().enumerate() {
            for (state, prop) in ControlState::ALL.into_iter().zip(overrides) {
                let Some(prop) = prop else {
                    continue;
                };
                let color = prop.resolve(&styles.tokens);
                // Parts in order: the icons' own overrides come after the content's.
                match part {
                    BACKGROUND => colors.background.set(state, color),
                    BORDER => colors.border.set(state, color),
                    CONTENT => {
                        colors.content.set(state, color);
                        colors.icon.set(state, color);
                    }
                    _ => colors.icon.set(state, color),
                }
            }
        }
        colors
    }

    /// The metrics of the content.
    pub(super) fn metrics<'s>(&self, ui: &Ui<'s>) -> ContentMetrics<'s> {
        let styles = ui.styles();
        let defaults = self.defaults(ui);
        ContentMetrics {
            gap: defaults.gap,
            icon_size: defaults.icon_size,
            trailing_size: defaults.trailing_size,
            stroke_width: styles.button.stroke_width,
            dot_size: styles.button.dot_size,
            font: &styles.text.label,
        }
    }

    /// The border width: the call's, or the style's.
    pub(super) fn border(&self, ui: &Ui<'_>) -> f32 {
        let fallback = self.defaults(ui).border_width;
        Self::dimension(ui, self.border_width.as_ref(), fallback)
    }

    /// The corner radii: the call's, or the look's.
    pub(super) fn radii(&self, ui: &Ui<'_>) -> Corners {
        match &self.radii {
            Some(radii) => radii.resolve(&ui.styles().tokens),
            None => Corners::all(self.defaults(ui).corner_radius),
        }
    }

    /// `radii` of the button in `rect` with the corners where its open menu meets it squared:
    /// where the menu opened on the last frame, or on the side it opens on while it has not
    /// been placed yet (the frame that opens it).
    pub(crate) fn junction(&self, rect: Rect, radii: Corners) -> Corners {
        let Some(menu) = self.menu_state() else {
            return radii;
        };
        if let Some(placed) = menu.placed() {
            return placed.control_corners(rect, radii);
        }
        match menu.opened_for().map(|(_, placement)| placement.side) {
            Some(Side::Below) => Corners {
                bottom_left: 0.0,
                bottom_right: 0.0,
                ..radii
            },
            Some(Side::Above) => Corners {
                top_left: 0.0,
                top_right: 0.0,
                ..radii
            },
            Some(Side::Right) => Corners {
                top_right: 0.0,
                bottom_right: 0.0,
                ..radii
            },
            Some(Side::Left) => Corners {
                top_left: 0.0,
                bottom_left: 0.0,
                ..radii
            },
            None => radii,
        }
    }
}
