//! [`ColorField`]: a color one interface unit high, outlined in a neutral gray of its own, which
//! opens a color picker below it (a dropdown, its corners meeting the field's) or as a panel of
//! its own.
//!
//! Each half of the field shows the color's transparency its own way ([`Transparency`]): by
//! default, the left half opaque and the right half over a checkerboard.

use koebi_core::{Color, Rect, Vec2};
use koebi_i18n::tr;
use koebi_render::{Corners, Fill, FillPrimitive, RectPrimitive};

use super::picker::{
    ColorPicker, ColorPickerState, PICKER_SECTIONS, PickerLayout, PickerSection, field_count,
};
use super::shape::PickerShape;
use crate::id::WidgetId;
use crate::ui::{Response, Ui};
use crate::widgets::menu::MenuState;

/// How a color field shows the transparency of its color, in one half.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Transparency {
    /// The color without its transparency.
    #[default]
    Opaque,
    /// The color over what lies behind the field.
    Over,
    /// The color over white.
    White,
    /// The color over black.
    Black,
    /// The color over a checkerboard: how transparent it is shows best.
    Checker,
}

impl Transparency {
    /// Every way, in the order a list offers them.
    pub const ALL: [Self; 5] = [
        Self::Opaque,
        Self::Over,
        Self::White,
        Self::Black,
        Self::Checker,
    ];

    /// Its name, for a list.
    pub fn label(self) -> &'static str {
        match self {
            Self::Opaque => tr("Opaque"),
            Self::Over => tr("Over the background"),
            Self::White => tr("Over white"),
            Self::Black => tr("Over black"),
            Self::Checker => tr("Over a checkerboard"),
        }
    }
}

/// Paints `color` in `rect`, its left half shown as `halves.0` and its right half as
/// `halves.1`, outlined in the color fields' gray (lighter when `hovered`).
pub(super) fn paint_color(
    ui: &mut Ui<'_>,
    rect: Rect,
    color: Color,
    radii: Corners,
    halves: (Transparency, Transparency),
    hovered: bool,
) {
    let style = ui.styles().color_field;
    ui.clipped_rounded(rect, radii, |ui| {
        let middle = rect.center().x;
        let left = Rect::from_min_max(rect.min, Vec2::new(middle, rect.max.y));
        let right = Rect::from_min_max(Vec2::new(middle, rect.min.y), rect.max);
        let parts = if halves.0 == halves.1 {
            [(rect, halves.0), (Rect::ZERO, halves.1)]
        } else {
            [(left, halves.0), (right, halves.1)]
        };
        for (part, transparency) in parts {
            if part.is_empty() {
                continue;
            }
            match transparency {
                Transparency::Opaque => {
                    ui.scene()
                        .push_rect(&RectPrimitive::new(part, color.with_alpha(1.0)));
                    continue;
                }
                Transparency::Over => {}
                Transparency::White => ui
                    .scene()
                    .push_rect(&RectPrimitive::new(part, Color::WHITE)),
                Transparency::Black => ui
                    .scene()
                    .push_rect(&RectPrimitive::new(part, Color::BLACK)),
                Transparency::Checker => ui.scene().push_fill(&FillPrimitive::new(
                    part,
                    Fill::Checker {
                        size: style.checker_size,
                        light: style.checker_light,
                        dark: style.checker_dark,
                    },
                )),
            }
            ui.scene().push_rect(&RectPrimitive::new(part, color));
        }
    });
    let outline = if hovered {
        style.outline_hovered
    } else {
        style.outline
    };
    ui.scene().push_rect(
        &RectPrimitive::new(rect, Color::TRANSPARENT)
            .with_corner_radii(radii)
            .with_border(style.outline_width, outline),
    );
}

/// What a color field keeps between frames: whether its picker is open, and the picker's own
/// state.
#[derive(Debug, Default)]
pub struct ColorFieldState {
    menu: MenuState,
    picker: ColorPickerState,
}

impl ColorFieldState {
    /// Closed.
    pub fn new() -> Self {
        Self::default()
    }

    /// `true` while its picker is open.
    pub fn is_open(&self) -> bool {
        self.menu.is_open()
    }

    /// The state of its picker.
    pub fn picker(&mut self) -> &mut ColorPickerState {
        &mut self.picker
    }
}

/// What happened to a color field this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ColorFieldResponse {
    /// The pointer's interaction with the field.
    pub response: Response,
    /// The color changed, in its picker.
    pub changed: bool,
}

/// A color field editing `color` in a picker, configured then shown with [`ColorField::show`].
#[derive(Debug)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "a builder's independent switches, each set by its own method"
)]
pub struct ColorField<'a> {
    id: WidgetId,
    color: &'a mut Color,
    state: &'a mut ColorFieldState,
    halves: (Transparency, Transparency),
    dropdown: bool,
    shape: PickerShape,
    title: Option<&'a str>,
    original: Option<Color>,
    palette: &'a [Color],
    sections: Option<&'a [PickerSection]>,
    eyedropper: bool,
    opacity: bool,
    enabled: bool,
}

impl<'a> ColorField<'a> {
    /// A field of `color`, its picker kept in `state`: a dropdown below it.
    pub fn new(id: WidgetId, color: &'a mut Color, state: &'a mut ColorFieldState) -> Self {
        Self {
            id,
            color,
            state,
            halves: (Transparency::Opaque, Transparency::Checker),
            dropdown: true,
            shape: PickerShape::SquareSv,
            title: None,
            original: None,
            palette: &[],
            sections: None,
            eyedropper: true,
            opacity: true,
            enabled: true,
        }
    }

    /// Whether the color has an opacity to edit (on by default): without one, the picker
    /// shows neither its opacity strip nor its opacity field ([`ColorPicker::opacity`]).
    #[must_use]
    pub fn opacity(mut self, opacity: bool) -> Self {
        self.opacity = opacity;
        self
    }

    /// How its left and right halves show the color's transparency.
    #[must_use]
    pub fn halves(mut self, left: Transparency, right: Transparency) -> Self {
        self.halves = (left, right);
        self
    }

    /// Opens its picker below it, their corners meeting (a dropdown), or as a panel of its
    /// own at the field.
    #[must_use]
    pub fn dropdown(mut self, dropdown: bool) -> Self {
        self.dropdown = dropdown;
        self
    }

    /// The shape of its picker.
    #[must_use]
    pub fn shape(mut self, shape: PickerShape) -> Self {
        self.shape = shape;
        self
    }

    /// The title of its picker.
    #[must_use]
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// The color before any change ([`ColorPicker::original`]).
    #[must_use]
    pub fn original(mut self, original: Color) -> Self {
        self.original = Some(original);
        self
    }

    /// Colors its picker offers ([`ColorPicker::palette`]).
    #[must_use]
    pub fn palette(mut self, palette: &'a [Color]) -> Self {
        self.palette = palette;
        self
    }

    /// The parts of its picker, in order ([`ColorPicker::sections`]).
    #[must_use]
    pub fn sections(mut self, sections: &'a [PickerSection]) -> Self {
        self.sections = Some(sections);
        self
    }

    /// Whether its picker has an eyedropper, picking anywhere in the bounds the field is shown
    /// in (on by default).
    #[must_use]
    pub fn eyedropper(mut self, eyedropper: bool) -> Self {
        self.eyedropper = eyedropper;
        self
    }

    /// A disabled field shows its color and opens nothing.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows the field in `rect` and its picker while open, kept inside `bounds`.
    pub fn show(self, ui: &mut Ui<'_>, rect: Rect, bounds: Rect) -> ColorFieldResponse {
        let response = ui.interact(self.id, rect);
        let style = ui.styles().color_field;
        let menu = &mut self.state.menu;
        if !self.enabled {
            menu.close();
        } else if response.clicked {
            if menu.is_open() {
                menu.close();
            } else {
                self.state.picker.opened_with(*self.color);
                if self.dropdown {
                    menu.open_below(rect);
                } else {
                    menu.open_at(Vec2::new(rect.min.x, rect.max.y));
                }
            }
        }
        if self.dropdown && self.state.menu.is_open() {
            self.state.menu.follow(rect);
        }
        // Where the dropdown meets the field, their corners are square.
        let radii = Corners::all(style.corner_radius);
        let radii = match self.state.menu.placed() {
            Some(placed) if self.dropdown => placed.control_corners(rect, radii),
            _ => radii,
        };
        paint_color(
            ui,
            rect,
            *self.color,
            radii,
            self.halves,
            self.enabled && (response.hovered || self.state.menu.is_open()),
        );
        let Self {
            id,
            color,
            state,
            shape,
            title,
            original,
            palette,
            sections,
            eyedropper,
            opacity,
            ..
        } = self;
        let sections = sections.unwrap_or(&PICKER_SECTIONS);
        let layout = PickerLayout {
            shape,
            titled: title.is_some(),
            swatches: palette.len(),
            fields: field_count(state.picker.mode(), opacity),
            sections,
            opacity,
        };
        let size = layout.size(ui);
        // The panel takes the menu, the picker the rest.
        let ColorFieldState { menu, picker } = state;
        let changed = ui
            .popup(id.with(2), menu, size, bounds, |ui, content| {
                let mut shown = ColorPicker::new(id.with(1), picker, color)
                    .shape(shape)
                    .palette(palette)
                    .sections(sections)
                    .opacity(opacity);
                if let Some(title) = title {
                    shown = shown.title(title);
                }
                if let Some(original) = original {
                    shown = shown.original(original);
                }
                if eyedropper {
                    shown = shown.eyedropper(true);
                }
                shown.show(ui, content)
            })
            .unwrap_or(false);
        ColorFieldResponse { response, changed }
    }
}
