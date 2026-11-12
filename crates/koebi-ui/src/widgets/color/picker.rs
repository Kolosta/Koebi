//! [`ColorPicker`]: the content of the panel a color field opens. Its parts, in an order the
//! caller may change: a title, the area of two components with the strips of the third and of
//! the opacity and the swatches of the current, previous and original colors, the switch of
//! the model the fields edit, those fields, and a palette.
//!
//! Every change shows at once: the color is written on every frame something moves it (a drag
//! in the area or a strip, a field, a swatch), and the caller sees it in the same frame.

use koebi_core::input::ModifierKey;
use koebi_core::{Cmyk, Color, Hsl, Hsv, Oklch, Rect, Vec2};
use koebi_i18n::tr;
use koebi_render::{Corners, Fill, FillPrimitive, RectPrimitive};

use super::field::{Transparency, paint_color};
use super::shape::{
    Components, OKLCH_CHROMA, PickerShape, RING_INNER, area_point, from_area, hue_at, on_ring,
    ring_point,
};
use super::strip::{ColorStrip, ColorStripState, StripFill};
use crate::id::WidgetId;
use crate::operation::OperationEnd;
use crate::screen_pick::PickOutcome;
use crate::text::Align;
use crate::ui::Ui;
use crate::widgets::button::{Button, ButtonGroup, GroupCell};
use crate::widgets::field_group::FieldGroup;
use crate::widgets::number_field::{BarMapping, NumberField};
use crate::widgets::text_field::TextField;

/// The model the picker's fields edit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorMode {
    /// Red, green and blue, 0 to 255.
    #[default]
    Rgb,
    /// Hue in degrees, saturation and value in percent.
    Hsv,
    /// Hue in degrees, saturation and lightness in percent.
    Hsl,
    /// OKLCH: lightness in percent, chroma, hue in degrees.
    Oklch,
    /// Cyan, magenta, yellow and black in percent, without an ink profile.
    Cmyk,
    /// `#RRGGBB`, or `#RRGGBBAA` with an opacity.
    Hex,
}

impl ColorMode {
    /// Every model, in the order of the switch.
    pub const ALL: [Self; 6] = [
        Self::Rgb,
        Self::Hsv,
        Self::Hsl,
        Self::Oklch,
        Self::Cmyk,
        Self::Hex,
    ];

    /// Its name on the switch.
    pub fn label(self) -> &'static str {
        match self {
            Self::Rgb => "RGB",
            Self::Hsv => "HSV",
            Self::Hsl => "HSL",
            Self::Oklch => "OKLCH",
            Self::Cmyk => "CMYK",
            Self::Hex => tr("Hex"),
        }
    }
}

/// A part of the picker, shown in the order given ([`ColorPicker::sections`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PickerSection {
    /// The title, when there is one.
    Title,
    /// The area, its strips and the swatches.
    Area,
    /// The switch of the model the fields edit.
    Modes,
    /// The fields of the model.
    Values,
    /// The palette, when there is one.
    Palette,
}

/// The parts of a picker, in their default order.
pub const PICKER_SECTIONS: [PickerSection; 5] = [
    PickerSection::Title,
    PickerSection::Area,
    PickerSection::Modes,
    PickerSection::Values,
    PickerSection::Palette,
];

/// A drag in the area: where the mark is (in the area's unit square), on the ring or not,
/// and what the picker held before it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct AreaDrag {
    at: Vec2,
    ring: bool,
    before: (Components, f32),
}

/// What a picker keeps between frames: the components it edits, the color when it opened, the
/// model of its fields, its drags.
#[derive(Debug)]
pub struct ColorPickerState {
    components: Components,
    alpha: f32,
    /// The color the components were last taken from or given: another one came from outside.
    synced: Option<Color>,
    previous: Option<Color>,
    mode: ColorMode,
    drag: Option<AreaDrag>,
    third: ColorStripState,
    opacity: ColorStripState,
    hex: String,
}

impl Default for ColorPickerState {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorPickerState {
    /// Closed, its fields in RGB.
    pub fn new() -> Self {
        let hsv = Hsv::default();
        Self {
            components: Components {
                hsv,
                hsl: Hsl::default(),
                oklch: Oklch::new(0.7, 0.0, 0.0),
            },
            alpha: 1.0,
            synced: None,
            previous: None,
            mode: ColorMode::Rgb,
            drag: None,
            third: ColorStripState::new(),
            opacity: ColorStripState::new(),
            hex: String::new(),
        }
    }

    /// The model its fields edit.
    pub fn mode(&self) -> ColorMode {
        self.mode
    }

    /// Edits in `mode`.
    pub fn set_mode(&mut self, mode: ColorMode) {
        self.mode = mode;
    }

    /// The color when the picker opened.
    pub fn previous(&self) -> Option<Color> {
        self.previous
    }

    /// Remembers `color` as the one before the picker opened.
    pub(super) fn opened_with(&mut self, color: Color) {
        self.previous = Some(color);
    }

    /// Takes `color` unless the picker gave it last: a change from outside. Hues stay where
    /// they were for grays, whose hue is not theirs.
    fn sync(&mut self, color: Color) {
        if self.synced == Some(color) {
            return;
        }
        self.take(color, None);
        self.hex = hex_of(color);
    }

    /// Takes `color` into every set of components but `kept` (the one just edited).
    fn take(&mut self, color: Color, kept: Option<PickerShape>) {
        let Components { hsv, hsl, oklch } = self.components;
        let edited_hsv = kept
            .is_some_and(|shape| !matches!(shape, PickerShape::WheelHsl | PickerShape::WheelOklch));
        if !edited_hsv {
            let next = Hsv::from_color(color);
            self.components.hsv = if next.s <= 1e-4 || next.v <= 1e-4 {
                Hsv { h: hsv.h, ..next }
            } else {
                next
            };
        }
        if kept != Some(PickerShape::WheelHsl) {
            let next = Hsl::from_color(color);
            self.components.hsl = if next.s <= 1e-4 {
                Hsl { h: hsl.h, ..next }
            } else {
                next
            };
        }
        if kept != Some(PickerShape::WheelOklch) {
            let next = color.to_oklch();
            self.components.oklch = if next.c <= 1e-4 {
                Oklch::new(next.l, next.c, oklch.h)
            } else {
                next
            };
        }
        self.alpha = color.a;
        self.synced = Some(color);
    }

    /// The color of the components `shape` edits, with the opacity.
    fn color_of(&self, shape: PickerShape) -> Color {
        let Components { hsv, hsl, oklch } = self.components;
        match shape {
            PickerShape::WheelHsl => hsl.to_color(self.alpha),
            PickerShape::WheelOklch => oklch.to_oklab().to_color().with_alpha(self.alpha),
            _ => hsv.to_color(self.alpha),
        }
    }
}

/// A color picker editing `color`, configured then shown with [`ColorPicker::show`].
#[derive(Debug)]
pub struct ColorPicker<'a> {
    id: WidgetId,
    state: &'a mut ColorPickerState,
    color: &'a mut Color,
    shape: PickerShape,
    title: Option<&'a str>,
    original: Option<Color>,
    palette: &'a [Color],
    sections: &'a [PickerSection],
    /// Whether it has an eyedropper.
    eyedropper: bool,
    /// Whether the color has an opacity to edit.
    opacity: bool,
}

impl<'a> ColorPicker<'a> {
    /// A picker of `color`, its state kept in `state`.
    pub fn new(id: WidgetId, state: &'a mut ColorPickerState, color: &'a mut Color) -> Self {
        Self {
            id,
            state,
            color,
            shape: PickerShape::SquareSv,
            title: None,
            original: None,
            palette: &[],
            sections: &PICKER_SECTIONS,
            eyedropper: false,
            opacity: true,
        }
    }

    /// Whether the color has an opacity to edit (on by default). Without one (the source of
    /// a palette, always opaque), neither the opacity strip nor the opacity field shows, the
    /// hexadecimal form has six digits, and the color stays opaque: a picker shows only what
    /// changes its color.
    #[must_use]
    pub fn opacity(mut self, opacity: bool) -> Self {
        self.opacity = opacity;
        self
    }

    /// Shows the color as `shape` does.
    #[must_use]
    pub fn shape(mut self, shape: PickerShape) -> Self {
        self.shape = shape;
        self
    }

    /// A title at the top: what the color is for.
    #[must_use]
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// The color before any change of the user's (a token's own value): a third swatch, which
    /// brings it back.
    #[must_use]
    pub fn original(mut self, original: Color) -> Self {
        self.original = Some(original);
        self
    }

    /// Colors offered below, each a swatch that picks it.
    #[must_use]
    pub fn palette(mut self, palette: &'a [Color]) -> Self {
        self.palette = palette;
        self
    }

    /// An eyedropper: a button below the swatches starting a session that picks a color in
    /// any window of the application ([`Ui::start_screen_pick`]).
    #[must_use]
    pub fn eyedropper(mut self, eyedropper: bool) -> Self {
        self.eyedropper = eyedropper;
        self
    }

    /// The parts shown, in this order ([`PICKER_SECTIONS`] by default).
    #[must_use]
    pub fn sections(mut self, sections: &'a [PickerSection]) -> Self {
        self.sections = sections;
        self
    }

    /// The size the picker takes.
    pub fn size(&self, ui: &Ui<'_>) -> Vec2 {
        self.layout().size(ui)
    }

    /// What its size and the place of its parts depend on.
    fn layout(&self) -> PickerLayout<'a> {
        PickerLayout {
            shape: self.shape,
            titled: self.title.is_some(),
            swatches: self.palette.len(),
            fields: field_count(self.state.mode, self.opacity),
            sections: self.sections,
            opacity: self.opacity,
        }
    }

    /// Shows the picker in `rect` (as large as [`ColorPicker::size`]); returns whether the
    /// color changed.
    pub fn show(mut self, ui: &mut Ui<'_>, rect: Rect) -> bool {
        self.state.sync(*self.color);
        let gap = ui.styles().color_picker.gap;
        let mut y = rect.min.y;
        let mut changed = None;
        let layout = self.layout();
        for section in layout.shown() {
            let height = layout.section_height(ui, section, rect.width());
            let at =
                Rect::from_min_max(Vec2::new(rect.min.x, y), Vec2::new(rect.max.x, y + height));
            let next = match section {
                PickerSection::Title => {
                    let font = ui.styles().text.label.clone();
                    let color = ui.styles().text.primary;
                    ui.label(self.title.unwrap_or(""), at, &font, color, Align::Start);
                    None
                }
                PickerSection::Area => self.area(ui, at),
                PickerSection::Modes => {
                    self.modes(ui, at);
                    None
                }
                PickerSection::Values => self.values(ui, at),
                PickerSection::Palette => self.palette_row(ui, at),
            };
            changed = next.or(changed);
            y += height + gap;
        }
        changed = self.eyedrop(ui).or(changed);
        if !self.opacity {
            // Whatever the source of the change (a hexadecimal form with an opacity, the
            // eyedropper on a translucent pixel), the color stays opaque.
            changed = changed.map(|change| match change {
                Change::Color(color) => Change::Color(color.with_alpha(1.0)),
                Change::Components => {
                    self.state.alpha = 1.0;
                    Change::Components
                }
            });
        }
        match changed {
            Some(Change::Components) => {
                let color = self.state.color_of(self.shape);
                self.state.take(color, Some(self.shape));
                self.state.hex = hex_of(color);
                *self.color = color;
                true
            }
            Some(Change::Color(color)) => {
                self.state.take(color, None);
                self.state.hex = hex_of(color);
                *self.color = color;
                true
            }
            None => false,
        }
    }
}

/// What the size of a picker and the place of its parts depend on.
#[derive(Clone, Copy, Debug)]
pub(super) struct PickerLayout<'a> {
    pub(super) shape: PickerShape,
    pub(super) titled: bool,
    pub(super) swatches: usize,
    /// How many fields the model edited has.
    pub(super) fields: usize,
    pub(super) sections: &'a [PickerSection],
    /// Whether the picker has its opacity strip.
    pub(super) opacity: bool,
}

impl PickerLayout<'_> {
    /// The size the picker takes.
    pub(super) fn size(&self, ui: &Ui<'_>) -> Vec2 {
        let style = ui.styles().color_picker;
        let strips = f32::from(u8::from(self.shape.has_strip()) + u8::from(self.opacity));
        let width = style.area_size
            + strips * (style.strip_width + style.gap)
            + style.gap
            + style.swatch_width;
        let height = self
            .shown()
            .map(|section| self.section_height(ui, section, width) + style.gap)
            .sum::<f32>()
            - style.gap;
        Vec2::new(width, height.max(0.0))
    }

    /// The sections that show: without a title or a palette, theirs do not.
    fn shown(&self) -> impl Iterator<Item = PickerSection> + '_ {
        self.sections
            .iter()
            .copied()
            .filter(|section| match section {
                PickerSection::Title => self.titled,
                PickerSection::Palette => self.swatches > 0,
                _ => true,
            })
    }

    fn section_height(&self, ui: &Ui<'_>, section: PickerSection, width: f32) -> f32 {
        let style = ui.styles().color_picker;
        let row = ui.styles().button.height;
        match section {
            PickerSection::Area => style.area_size,
            PickerSection::Title | PickerSection::Modes => row,
            PickerSection::Values => row * self.fields as f32,
            PickerSection::Palette => {
                let (columns, side) = palette_grid(width, row, style.gap, self.swatches);
                let lines = self.swatches.div_ceil(columns.max(1)) as f32;
                lines * side + (lines - 1.0).max(0.0) * style.gap
            }
        }
    }
}

/// What changed this frame: the components (a drag, a strip), or the whole color (a field, a
/// swatch).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Change {
    Components,
    Color(Color),
}

impl ColorPicker<'_> {
    /// The area, its strips and the swatches in `rect`.
    fn area(&mut self, ui: &mut Ui<'_>, rect: Rect) -> Option<Change> {
        let style = ui.styles().color_picker;
        let side = style.area_size.min(rect.height());
        let area = Rect::from_origin_size(rect.min, Vec2::splat(side));
        let mut changed = self.area_drag(ui, area);
        self.paint_area(ui, area);
        let mut x = area.max.x + style.gap;
        let strip = |x: f32| {
            Rect::from_origin_size(Vec2::new(x, area.min.y), Vec2::new(style.strip_width, side))
        };
        if self.shape.has_strip() {
            let at = strip(x);
            changed = self.third_strip(ui, at).or(changed);
            x = at.max.x + style.gap;
        }
        if self.opacity {
            let at = strip(x);
            let opaque = self.state.color_of(self.shape).with_alpha(1.0);
            let stops = [opaque, opaque.with_alpha(0.0)];
            let mut transparency = 1.0 - self.state.alpha;
            let moved = ColorStrip::new(
                self.id.with(3),
                &mut self.state.opacity,
                &mut transparency,
                StripFill::Stops(&stops),
            )
            .checker(true)
            .show(ui, at);
            if moved.changed {
                self.state.alpha = (1.0 - transparency).clamp(0.0, 1.0);
                changed = Some(Change::Components);
            }
            x = at.max.x + style.gap;
        }
        // The swatches: current, previous, original.
        let column =
            Rect::from_min_max(Vec2::new(x, area.min.y), Vec2::new(rect.max.x, area.max.y));
        // Drawn every frame, a drag or not: a widget never hides in a lazy branch.
        let picked = self.swatches(ui, column);
        changed.or(picked)
    }

    /// The drag in the area: a press sets the components where it lands; the pointer hides
    /// while the mark follows it (finer with Shift), even past the area's rounded corners up
    /// to its real ones; the operation's cancel brings the color back, its confirm keeps it.
    fn area_drag(&mut self, ui: &mut Ui<'_>, area: Rect) -> Option<Change> {
        let response = ui.interact(self.id.with(1), area);
        let size = area.width().max(1.0);
        let unit = |point: Vec2| (point - area.min) / size;
        let state = &mut *self.state;
        let mut changed = None;
        if response.pressed
            && let Some(point) = ui.input().pointer()
        {
            let at = unit(point);
            let ring = self.shape.has_ring() && on_ring(at);
            state.drag = Some(AreaDrag {
                at,
                ring,
                before: (state.components, state.alpha),
            });
        } else if let Some(drag) = state.drag {
            match ui.operation() {
                Some(OperationEnd::Cancelled) => {
                    (state.components, state.alpha) = drag.before;
                    state.drag = None;
                    return Some(Change::Components);
                }
                Some(OperationEnd::Confirmed) => {
                    state.drag = None;
                    return None;
                }
                None => {}
            }
            if response.held {
                let fine = ui.input().modifiers().sides(ModifierKey::Shift).any();
                let factor = if fine {
                    ui.styles().color_picker.fine_factor
                } else {
                    1.0
                };
                let at = drag.at + ui.input().motion() / size * factor;
                state.drag = Some(AreaDrag { at, ..drag });
            } else {
                state.drag = None;
            }
        }
        if let Some(drag) = state.drag {
            let before = state.components;
            if drag.ring {
                state.components.hsv.h = hue_at(drag.at);
            } else {
                state.components = from_area(self.shape, &state.components, drag.at);
            }
            if state.components != before {
                changed = Some(Change::Components);
            }
            // The pointer comes back at the mark.
            let mark = if drag.ring {
                ring_point(state.components.hsv.h)
            } else {
                area_point(self.shape, &state.components)
            };
            ui.lock_pointer(area.min + mark * size);
        }
        changed
    }

    /// Paints the area of the shape in `area` and its marks.
    fn paint_area(&self, ui: &mut Ui<'_>, area: Rect) {
        let style = ui.styles().color_picker;
        let Components { hsv, hsl, oklch } = self.state.components;
        let square = |ui: &mut Ui<'_>, rect: Rect, radius: f32| {
            let hue = Hsv {
                h: hsv.h,
                s: 1.0,
                v: 1.0,
            }
            .to_color(1.0);
            ui.scene().push_fill(
                &FillPrimitive::new(
                    rect,
                    Fill::Corners {
                        top_left: Color::WHITE,
                        top_right: hue,
                        bottom_right: Color::BLACK,
                        bottom_left: Color::BLACK,
                    },
                )
                .with_corner_radius(radius),
            );
        };
        match self.shape {
            PickerShape::SquareSv => square(ui, area, style.corner_radius),
            PickerShape::SquareHs => ui.scene().push_fill(
                &FillPrimitive::new(area, Fill::HueSaturation { value: hsv.v })
                    .with_corner_radius(style.corner_radius),
            ),
            PickerShape::WheelHsv | PickerShape::WheelHsl | PickerShape::WheelOklch => {
                let (level, chroma) = match self.shape {
                    PickerShape::WheelHsv => (hsv.v, 0.0),
                    PickerShape::WheelHsl => (hsl.l, 0.0),
                    _ => (oklch.l, OKLCH_CHROMA),
                };
                ui.scene().push_fill(&FillPrimitive::new(
                    area,
                    Fill::Wheel {
                        model: self.shape.hue_model(),
                        level,
                        chroma,
                    },
                ));
            }
            PickerShape::RingTriangle | PickerShape::RingSquare => {
                ui.scene().push_fill(&FillPrimitive::new(
                    area,
                    Fill::Ring {
                        model: self.shape.hue_model(),
                        inner: RING_INNER,
                        lightness: 0.0,
                        chroma: 0.0,
                    },
                ));
                if self.shape == PickerShape::RingTriangle {
                    let inner = Rect::from_center_size(area.center(), area.size() * RING_INNER);
                    ui.scene()
                        .push_fill(&FillPrimitive::new(inner, Fill::Triangle { hue: hsv.h }));
                } else {
                    let side = area.width() * RING_INNER * core::f32::consts::FRAC_1_SQRT_2;
                    square(
                        ui,
                        Rect::from_center_size(area.center(), Vec2::splat(side)),
                        0.0,
                    );
                }
                marker(
                    ui,
                    area.min + ring_point(hsv.h) * area.width(),
                    Color::TRANSPARENT,
                );
            }
        }
        let point = area_point(self.shape, &self.state.components);
        let shown = self.state.color_of(self.shape).with_alpha(1.0);
        marker(ui, area.min + point * area.width(), shown);
    }

    /// The strip of the third component in `rect`: the hue of the squares, the value or the
    /// lightness of the others.
    fn third_strip(&mut self, ui: &mut Ui<'_>, rect: Rect) -> Option<Change> {
        let Components { hsv, hsl, oklch } = self.state.components;
        let id = self.id.with(2);
        let state = &mut self.state.third;
        let changed = match self.shape {
            PickerShape::SquareSv => {
                let mut hue = hsv.h;
                let fill = StripFill::Hues {
                    model: koebi_render::HueModel::Hsv,
                    lightness: 0.0,
                    chroma: 0.0,
                };
                let moved = ColorStrip::new(id, state, &mut hue, fill).show(ui, rect);
                self.state.components.hsv.h = hue.min(0.9999);
                moved.changed
            }
            PickerShape::WheelHsl => {
                let stops = [
                    Color::WHITE,
                    Hsl { l: 0.5, ..hsl }.to_color(1.0),
                    Color::BLACK,
                ];
                let mut t = 1.0 - hsl.l;
                let moved =
                    ColorStrip::new(id, state, &mut t, StripFill::Stops(&stops)).show(ui, rect);
                self.state.components.hsl.l = 1.0 - t;
                moved.changed
            }
            PickerShape::WheelOklch => {
                let stops: [Color; 9] = core::array::from_fn(|index| {
                    let l = 1.0 - index as f32 / 8.0;
                    Oklch::new(l, oklch.c, oklch.h).to_oklab().to_color()
                });
                let mut t = 1.0 - oklch.l;
                let moved =
                    ColorStrip::new(id, state, &mut t, StripFill::Stops(&stops)).show(ui, rect);
                self.state.components.oklch.l = 1.0 - t;
                moved.changed
            }
            _ => {
                let stops = [Hsv { v: 1.0, ..hsv }.to_color(1.0), Color::BLACK];
                let mut t = 1.0 - hsv.v;
                let moved =
                    ColorStrip::new(id, state, &mut t, StripFill::Stops(&stops)).show(ui, rect);
                self.state.components.hsv.v = 1.0 - t;
                moved.changed
            }
        };
        changed.then_some(Change::Components)
    }

    /// The current, previous and original colors, stacked in `column`; a click on the last
    /// two brings them back.
    fn swatches(&mut self, ui: &mut Ui<'_>, column: Rect) -> Option<Change> {
        let style = ui.styles().color_picker;
        // The eyedropper below them, a control high, when the application can read its frames.
        let column = if self.eyedropper && ui.can_read_screen() {
            let row = ui.styles().button.height;
            let button =
                Rect::from_min_max(Vec2::new(column.min.x, column.max.y - row), column.max);
            let key = self.id.with(19);
            let was = ui.is_screen_picking(key);
            let mut on = was;
            Button::new(key)
                .icon(koebi_icons::interface::EYEDROPPER)
                .toggle(&mut on)
                .show(ui, button);
            if on && !was {
                ui.start_screen_pick(key);
            } else if was && !on {
                ui.cancel_screen_pick(key);
            }
            ui.tooltip(self.id.with(19), tr("Pick a color on the screen"), button);
            Rect::from_min_max(
                column.min,
                Vec2::new(column.max.x, button.min.y - style.gap),
            )
        } else {
            column
        };
        let colors = [Some(*self.color), self.state.previous, self.original];
        let count = colors.iter().flatten().count().max(1) as f32;
        let height = ((column.height() - style.gap * (count - 1.0)) / count).max(0.0);
        let mut y = column.min.y;
        let mut chosen = None;
        for (index, color) in colors.into_iter().enumerate() {
            let Some(color) = color else {
                continue;
            };
            let at = Rect::from_origin_size(
                Vec2::new(column.min.x, y),
                Vec2::new(column.width(), height),
            );
            let id = self.id.with(10 + index as u64);
            let response = ui.interact(id, at);
            paint_color(
                ui,
                at,
                color,
                Corners::all(style.corner_radius),
                (Transparency::Opaque, Transparency::Checker),
                response.hovered && index > 0,
            );
            let tip = [
                tr("The current color"),
                tr("The color before the picker opened"),
                tr("The color without any change"),
            ][index];
            ui.tooltip(id, tip, at);
            if index > 0 && response.clicked {
                chosen = Some(Change::Color(color));
            }
            y += height + style.gap;
        }
        chosen
    }

    /// The switch of the model the fields edit.
    fn modes(&mut self, ui: &mut Ui<'_>, rect: Rect) {
        let current = self.state.mode;
        let mut cells: [GroupCell<'_>; 6] = core::array::from_fn(|index| {
            let mode = ColorMode::ALL[index];
            GroupCell::new(
                Button::new(self.id.with(20 + index as u64))
                    .label(mode.label())
                    .selected(mode == current),
            )
        });
        if let Some(index) = ButtonGroup::horizontal().show(ui, rect, &mut cells).clicked {
            self.state.mode = ColorMode::ALL[index];
        }
    }

    /// The fields of the model in `rect`.
    fn values(&mut self, ui: &mut Ui<'_>, rect: Rect) -> Option<Change> {
        let color = *self.color;
        if self.state.mode == ColorMode::Hex {
            let response = TextField::new(self.id.with(30), &mut self.state.hex).show(ui, rect);
            return (response.changed)
                .then(|| parse_hex(&self.state.hex))
                .flatten()
                .map(Change::Color);
        }
        let Fields {
            labels,
            values,
            limits,
        } = fields_of(self.state.mode, color, &self.state.components);
        // The opacity is the last field of every model: without one, it does not show.
        let count =
            labels.iter().filter(|label| !label.is_empty()).count() - usize::from(!self.opacity);
        let style = &ui.styles().number_field;
        // Stacked, each as wide as the picker: a label and a bar each.
        let group = FieldGroup::vertical(rect, count, style.corner_radius, style.border_width);
        let mut edited = values;
        let mut changed = false;
        for index in 0..count {
            let (cell, corners) = group.cell(index);
            let (min, max, decimals) = limits[index];
            changed |= NumberField::new(self.id.with(40 + index as u64), &mut edited[index])
                .label(labels[index])
                .range(min, max)
                .decimals(usize::from(decimals))
                .bar(BarMapping::Linear)
                .corner_radii(corners)
                .show(ui, cell)
                .changed;
        }
        changed.then(|| {
            Change::Color(color_of_fields(
                self.state.mode,
                edited,
                &self.state.components,
            ))
        })
    }

    /// The palette in `rect`: a swatch per color, picking it.
    fn palette_row(&mut self, ui: &mut Ui<'_>, rect: Rect) -> Option<Change> {
        let style = ui.styles().color_picker;
        let row = ui.styles().button.height;
        let (columns, side) = palette_grid(rect.width(), row, style.gap, self.palette.len());
        let mut chosen = None;
        for (index, &color) in self.palette.iter().enumerate() {
            let (column, line) = (index % columns, index / columns);
            let at = Rect::from_origin_size(
                Vec2::new(
                    rect.min.x + (side + style.gap) * column as f32,
                    rect.min.y + (side + style.gap) * line as f32,
                ),
                Vec2::splat(side),
            );
            if ui
                .swatch(
                    self.id.with(100 + index as u64),
                    at,
                    color,
                    color == *self.color,
                )
                .clicked
            {
                chosen = Some(Change::Color(color));
            }
        }
        chosen
    }
}

impl ColorPicker<'_> {
    /// What the session its eyedropper started ended with, once: the pixel picked becomes the
    /// color, keeping the opacity.
    fn eyedrop(&mut self, ui: &mut Ui<'_>) -> Option<Change> {
        if !self.eyedropper {
            return None;
        }
        match ui.take_screen_pick(self.id.with(19))? {
            PickOutcome::Picked([r, g, b, _]) => Some(Change::Color(
                Color::from_rgba8(r, g, b, 255).with_alpha(self.state.alpha),
            )),
            PickOutcome::Cancelled => None,
        }
    }
}

/// The ring marking the color on an area at `center`, filled with `color`: a thin white ring
/// (`color-picker.marker.ring.width`), thinly outlined in black on both sides so that it shows
/// on any color.
fn marker(ui: &mut Ui<'_>, center: Vec2, color: Color) {
    let style = ui.styles().color_picker;
    let side = style.marker_size;
    let outline = style.outline_width;
    let ring_width = style.ring_width.min(side * 0.5);
    let outer = Rect::from_center_size(center, Vec2::splat(side + 2.0 * outline));
    let ring = Rect::from_center_size(center, Vec2::splat(side));
    let exact = koebi_render::Edges::Exact;
    ui.scene().push_rect(
        &RectPrimitive::new(outer, Color::TRANSPARENT)
            .with_corner_radius(outer.width() * 0.5)
            .with_border(outline, style.marker_outline)
            .with_edges(exact),
    );
    ui.scene().push_rect(
        &RectPrimitive::new(ring, color)
            .with_corner_radius(side * 0.5)
            .with_border(ring_width, style.marker)
            .with_edges(exact),
    );
    let inner = Rect::from_center_size(center, Vec2::splat((side - 2.0 * ring_width).max(0.0)));
    ui.scene().push_rect(
        &RectPrimitive::new(inner, Color::TRANSPARENT)
            .with_corner_radius(inner.width() * 0.5)
            .with_border(outline, style.marker_outline)
            .with_edges(exact),
    );
}

/// How many swatches of a palette go in a row of `width`, and their side.
fn palette_grid(width: f32, row: f32, gap: f32, count: usize) -> (usize, f32) {
    let side = (row * 0.8).max(1.0);
    let fitting = ((width + gap) / (side + gap)).floor();
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a few swatches, a positive whole number"
    )]
    let columns = (fitting.max(1.0) as usize).min(count.max(1));
    (columns, side)
}

/// How many fields `mode` has: its components and, with `opacity`, the opacity; one text for
/// hex.
pub(super) const fn field_count(mode: ColorMode, opacity: bool) -> usize {
    let opacity = if opacity { 1 } else { 0 };
    match mode {
        ColorMode::Hex => 1,
        ColorMode::Cmyk => 4 + opacity,
        _ => 3 + opacity,
    }
}

/// The fields of a model: five at most, those without a label unused.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Fields {
    labels: [&'static str; 5],
    values: [f64; 5],
    /// Each field's lowest and highest value, and its decimals.
    limits: [(f64, f64, u8); 5],
}

/// The fields of `mode` for `color`.
fn fields_of(mode: ColorMode, color: Color, components: &Components) -> Fields {
    let percent = |value: f32| f64::from(value) * 100.0;
    let degrees = |turns: f32| f64::from(turns) * 360.0;
    let alpha = percent(color.a);
    let part = (0.0, 100.0, 1);
    let none = (0.0, 1.0, 0);
    let (labels, values, limits) = match mode {
        ColorMode::Rgb | ColorMode::Hex => (
            ["R", "G", "B", "A", ""],
            [
                f64::from(color.r) * 255.0,
                f64::from(color.g) * 255.0,
                f64::from(color.b) * 255.0,
                alpha,
                0.0,
            ],
            [
                (0.0, 255.0, 0),
                (0.0, 255.0, 0),
                (0.0, 255.0, 0),
                part,
                none,
            ],
        ),
        ColorMode::Hsv => {
            let hsv = components.hsv;
            (
                ["H", "S", "V", "A", ""],
                [degrees(hsv.h), percent(hsv.s), percent(hsv.v), alpha, 0.0],
                [(0.0, 360.0, 1), part, part, part, none],
            )
        }
        ColorMode::Hsl => {
            let hsl = components.hsl;
            (
                ["H", "S", "L", "A", ""],
                [degrees(hsl.h), percent(hsl.s), percent(hsl.l), alpha, 0.0],
                [(0.0, 360.0, 1), part, part, part, none],
            )
        }
        ColorMode::Oklch => {
            let oklch = components.oklch;
            (
                ["L", "C", "H", "A", ""],
                [
                    percent(oklch.l),
                    f64::from(oklch.c),
                    f64::from(oklch.h),
                    alpha,
                    0.0,
                ],
                [part, (0.0, 0.37, 3), (0.0, 360.0, 1), part, none],
            )
        }
        ColorMode::Cmyk => {
            let cmyk = Cmyk::from_color(color);
            (
                ["C", "M", "Y", "K", "A"],
                [
                    percent(cmyk.c),
                    percent(cmyk.m),
                    percent(cmyk.y),
                    percent(cmyk.k),
                    alpha,
                ],
                [part; 5],
            )
        }
    };
    Fields {
        labels,
        values,
        limits,
    }
}

/// The color the fields of `mode` hold.
fn color_of_fields(mode: ColorMode, values: [f64; 5], components: &Components) -> Color {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "components within their ranges, which f32 holds"
    )]
    let value = |index: usize| values[index] as f32;
    let share = |index: usize| (value(index) / 100.0).clamp(0.0, 1.0);
    let turns = |index: usize| (value(index) / 360.0).rem_euclid(1.0);
    match mode {
        ColorMode::Rgb | ColorMode::Hex => Color::new(
            (value(0) / 255.0).clamp(0.0, 1.0),
            (value(1) / 255.0).clamp(0.0, 1.0),
            (value(2) / 255.0).clamp(0.0, 1.0),
            share(3),
        ),
        ColorMode::Hsv => Hsv {
            h: turns(0),
            s: share(1),
            v: share(2),
        }
        .to_color(share(3)),
        ColorMode::Hsl => Hsl {
            h: turns(0),
            s: share(1),
            l: share(2),
        }
        .to_color(share(3)),
        ColorMode::Oklch => {
            let _ = components;
            Oklch::new(share(0), value(1).max(0.0), value(2).rem_euclid(360.0))
                .to_oklab()
                .to_color()
                .with_alpha(share(3))
        }
        ColorMode::Cmyk => Cmyk {
            c: share(0),
            m: share(1),
            y: share(2),
            k: share(3),
        }
        .to_color(share(4)),
    }
}

/// `#RRGGBB` for an opaque color, `#RRGGBBAA` for another.
fn hex_of(color: Color) -> String {
    let [r, g, b, a] = color.to_rgba8();
    if a == 255 {
        format!("#{r:02X}{g:02X}{b:02X}")
    } else {
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
    }
}

/// The color `text` writes in hex, with or without its `#`, opaque without an opacity.
fn parse_hex(text: &str) -> Option<Color> {
    let digits = text.trim().trim_start_matches('#');
    let value = u32::from_str_radix(digits, 16).ok()?;
    match digits.len() {
        6 => Some(Color::from_hex(value)),
        8 => Some(Color::from_hex_rgba(value)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;
    use koebi_text::Fonts;

    use super::*;
    use crate::{Input, Styles, UiState};

    /// A picker with every part, frame after frame.
    struct Harness {
        state: UiState,
        picker: ColorPickerState,
        color: Color,
        input: Input,
        styles: Styles,
    }

    /// What a frame drew.
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Drawn {
        rects: usize,
        fills: usize,
        icons: usize,
    }

    impl Harness {
        fn new() -> Self {
            let mut state = UiState::new();
            state.set_fonts(Fonts::with_embedded());
            Self {
                state,
                picker: ColorPickerState::new(),
                color: Color::rgb(0.2, 0.5, 0.8),
                input: Input::new(),
                styles: crate::test_styles(),
            }
        }

        fn frame(&mut self) -> Drawn {
            let mut scene = Scene::new();
            {
                let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
                let picker =
                    ColorPicker::new(WidgetId::new("picker"), &mut self.picker, &mut self.color)
                        .original(Color::BLACK)
                        .eyedropper(true);
                let size = picker.size(&ui);
                picker.show(&mut ui, Rect::from_origin_size(Vec2::ZERO, size));
            }
            self.input.end_frame();
            Drawn {
                rects: scene.rect_count(),
                fills: scene.fill_count(),
                icons: scene.icon_count(),
            }
        }

        /// What the frames drew while a drag went from `from` to `to`: the press, then the
        /// motion, the color changing in both; the drag then ends.
        fn dragging(&mut self, from: Vec2, to: Vec2) -> [Drawn; 2] {
            self.input.pointer_moved(from);
            self.frame();
            let color = self.color;
            self.input.primary_button(true, from);
            let pressed = self.frame();
            assert_ne!(self.color, color, "the press picks a color");
            let color = self.color;
            // A locked pointer reports its motion; one that is not, its position.
            self.input.pointer_moved(to);
            self.input.pointer_moved_by(to - from);
            let moved = self.frame();
            assert_ne!(self.color, color, "the drag picks another");
            self.input.primary_button(false, to);
            self.frame();
            [pressed, moved]
        }
    }

    #[test]
    fn every_part_stays_drawn_while_the_area_or_a_strip_is_dragged() {
        let mut harness = Harness::new();
        let idle = harness.frame();
        assert!(idle.icons > 0, "the eyedropper: {idle:?}");
        let style = harness.styles.color_picker;
        let middle = style.area_size * 0.5;
        let area = Vec2::splat(middle);
        let strip = |index: f32| {
            let x = style.area_size + style.gap + index * (style.strip_width + style.gap);
            Vec2::new(x + style.strip_width * 0.5, middle)
        };
        for (part, from) in [("area", area), ("hue", strip(0.0)), ("opacity", strip(1.0))] {
            for drawn in harness.dragging(from, from + Vec2::new(-6.0, 9.0)) {
                // A swatch of the previous color may join the others: never fewer.
                assert_eq!(
                    drawn.icons, idle.icons,
                    "{part}: {drawn:?} against {idle:?}"
                );
                assert!(
                    drawn.rects >= idle.rects,
                    "{part}: {drawn:?} against {idle:?}"
                );
                assert!(
                    drawn.fills >= idle.fills,
                    "{part}: {drawn:?} against {idle:?}"
                );
            }
        }
    }

    #[test]
    fn hex_texts_go_there_and_back() {
        let color = Color::from_rgba8(18, 52, 86, 255);
        assert_eq!(hex_of(color), "#123456");
        assert_eq!(parse_hex("#123456"), Some(color));
        assert_eq!(
            parse_hex("12345678").map(Color::to_rgba8),
            Some([0x12, 0x34, 0x56, 0x78])
        );
        assert_eq!(parse_hex("#12"), None);
        assert_eq!(parse_hex("#GGGGGG"), None);
        assert_eq!(hex_of(color.with_alpha(0.5)).len(), 9);
    }

    #[test]
    fn every_mode_writes_back_the_color_its_fields_show() {
        let color = Color::new(0.2, 0.55, 0.8, 0.6);
        let mut state = ColorPickerState::new();
        state.sync(color);
        for mode in ColorMode::ALL {
            let values = fields_of(mode, color, &state.components).values;
            let back = color_of_fields(mode, values, &state.components);
            let delta = (back.r - color.r)
                .abs()
                .max((back.g - color.g).abs())
                .max((back.b - color.b).abs());
            assert!(delta < 3e-3, "{mode:?}: {back:?}");
            assert!((back.a - color.a).abs() < 1e-3, "{mode:?}");
        }
    }

    #[test]
    fn grays_keep_their_hue_and_outside_changes_are_taken() {
        let mut state = ColorPickerState::new();
        state.sync(
            Hsv {
                h: 0.4,
                s: 0.8,
                v: 0.9,
            }
            .to_color(1.0),
        );
        assert!((state.components.hsv.h - 0.4).abs() < 1e-3);
        // The saturation to zero: a gray, its hue kept.
        state.sync(Color::rgb(0.5, 0.5, 0.5));
        assert!((state.components.hsv.h - 0.4).abs() < 1e-3);
        assert_eq!(state.components.hsv.s, 0.0);
        // What the picker gave itself is not taken again.
        let synced = state.synced;
        state.sync(Color::rgb(0.5, 0.5, 0.5));
        assert_eq!(state.synced, synced);
    }

    #[test]
    fn palettes_fill_rows_of_swatches() {
        assert_eq!(palette_grid(100.0, 20.0, 4.0, 10), (5, 16.0));
        assert_eq!(palette_grid(100.0, 20.0, 4.0, 2), (2, 16.0));
        assert_eq!(palette_grid(5.0, 20.0, 4.0, 3).0, 1);
    }
}
