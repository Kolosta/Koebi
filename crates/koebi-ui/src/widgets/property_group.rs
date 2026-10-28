//! Property groups: labeled inputs in two columns, as in Blender's properties.
//!
//! A [`PropertyGroup`] lays rows out from the top of an area: each row's label is
//! right-aligned in the first column (`property-group.label.width-ratio` of the width, 35 % by
//! default) and its input fills the second. Rows of a sub-group follow each other closely
//! (`property-group.row.gap`); sub-groups are apart (`property-group.gap.size`). A vector's
//! label goes with its first component only: "Location X", then "Y" and "Z"
//! ([`PropertyGroup::vector`]). The group draws labels; the caller shows the inputs in the
//! rectangles it gets.

use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_design::kind;
use koebi_tokens::component;

use crate::style::Prop;
use crate::text::Align;
use crate::ui::Ui;

/// The layout of property groups, from the `component.property-group.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PropertyGroupStyle {
    /// Share of the width the labels take.
    pub label_ratio: f32,
    /// Height of a row.
    pub row_height: f32,
    /// Gap between two rows of a sub-group.
    pub row_gap: f32,
    /// Gap between two sub-groups.
    pub group_gap: f32,
    /// Gap between a label and its input.
    pub column_gap: f32,
    /// Label color.
    pub label: Color,
    /// Label color of disabled rows.
    pub label_disabled: Color,
}

impl PropertyGroupStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        Self {
            label_ratio: tokens.get(c::PROPERTY_GROUP_LABEL_WIDTH_RATIO),
            row_height: tokens.get(c::PROPERTY_GROUP_ROW_HEIGHT_DEFAULT),
            row_gap: tokens.get(c::PROPERTY_GROUP_ROW_GAP_DEFAULT),
            group_gap: tokens.get(c::PROPERTY_GROUP_GAP_SIZE_DEFAULT),
            column_gap: tokens.get(c::PROPERTY_GROUP_COLUMN_GAP_DEFAULT),
            label: tokens.get(c::PROPERTY_GROUP_LABEL_COLOR_DEFAULT),
            label_disabled: tokens.get(c::PROPERTY_GROUP_LABEL_COLOR_DISABLED),
        }
    }
}

/// Lays labeled inputs out in two columns, one row at a time, from the top of an area. Keep it
/// for one frame: make one per area and frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PropertyGroup {
    area: Rect,
    /// Where the next row starts.
    y: f32,
    /// A row was placed: the next one comes after a gap.
    placed: bool,
    /// The next row starts a sub-group: after the larger gap.
    new_group: bool,
    enabled: bool,
    label_ratio: f32,
    row_height: f32,
    row_gap: f32,
    group_gap: f32,
    column_gap: f32,
}

impl PropertyGroup {
    /// A group laying rows out from the top of `area`, measured by the tokens.
    pub fn new(ui: &Ui<'_>, area: Rect) -> Self {
        let style = &ui.styles().property_group;
        Self {
            area,
            y: area.min.y,
            placed: false,
            new_group: false,
            enabled: true,
            label_ratio: style.label_ratio,
            row_height: style.row_height,
            row_gap: style.row_gap,
            group_gap: style.group_gap,
            column_gap: style.column_gap,
        }
    }

    /// Resolves `prop` against the tokens of `ui`.
    fn resolve<K: kind::Kind>(ui: &Ui<'_>, prop: impl Into<Prop<K>>) -> K::Value {
        prop.into().resolve(&ui.styles().tokens)
    }

    /// Gives the labels `ratio` of the width (0 to 1) in place of the token's share.
    #[must_use]
    pub fn label_ratio(mut self, ui: &Ui<'_>, ratio: impl Into<Prop<kind::Number>>) -> Self {
        self.label_ratio = Self::resolve(ui, ratio).clamp(0.0, 1.0);
        self
    }

    /// Makes rows `height` high.
    #[must_use]
    pub fn row_height(mut self, ui: &Ui<'_>, height: impl Into<Prop<kind::Dimension>>) -> Self {
        self.row_height = Self::resolve(ui, height).max(0.0);
        self
    }

    /// The gaps between rows of a sub-group, and between sub-groups.
    #[must_use]
    pub fn gaps(
        mut self,
        ui: &Ui<'_>,
        row: impl Into<Prop<kind::Dimension>>,
        group: impl Into<Prop<kind::Dimension>>,
    ) -> Self {
        self.row_gap = Self::resolve(ui, row).max(0.0);
        self.group_gap = Self::resolve(ui, group).max(0.0);
        self
    }

    /// Shows the labels of the rows that follow as available or not, like the inputs beside
    /// them.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Starts a sub-group: the next row comes after the larger gap.
    pub fn group(&mut self) {
        self.new_group = true;
    }

    /// The height used so far, from the top of the area to the bottom of the last row.
    pub fn height(&self) -> f32 {
        self.y - self.area.min.y
    }

    /// The height of `rows` rows in `groups` sub-groups, with the tokens' metrics: what to
    /// reserve for a group.
    pub fn measure(ui: &Ui<'_>, rows: usize, groups: usize) -> f32 {
        let style = &ui.styles().property_group;
        let count = |count: usize| f32::from(u16::try_from(count).unwrap_or(u16::MAX));
        let groups = groups.clamp(usize::from(rows > 0), rows.max(1));
        count(rows) * style.row_height
            + count(rows - groups.min(rows)) * style.row_gap
            + count(groups.saturating_sub(1)) * style.group_gap
    }

    /// The band of the next row, `height` high, across both columns.
    fn band(&mut self, height: f32) -> Rect {
        if self.placed {
            self.y += if self.new_group {
                self.group_gap
            } else {
                self.row_gap
            };
        }
        self.placed = true;
        self.new_group = false;
        let band = Rect::from_min_max(
            Vec2::new(self.area.min.x, self.y),
            Vec2::new(self.area.max.x, self.y + height),
        );
        self.y += height;
        band
    }

    /// The x where the input column starts, and where the label column ends.
    fn columns(&self) -> (f32, f32) {
        let width = self.area.width().max(0.0);
        let labels = (width * self.label_ratio).round();
        let label_end = self.area.min.x + (labels - self.column_gap).max(0.0);
        let input_start = (self.area.min.x + labels).min(self.area.max.x);
        (label_end, input_start)
    }

    /// The rectangle of the input of `band`.
    fn input(&self, band: Rect) -> Rect {
        let (_, start) = self.columns();
        Rect::from_min_max(Vec2::new(start, band.min.y), band.max)
    }

    /// Draws the label made of `parts` (written one after the other) right-aligned at the end
    /// of the label column of `band`.
    fn label(&self, ui: &mut Ui<'_>, band: Rect, parts: &[&str]) {
        let (end, _) = self.columns();
        let style = &ui.styles().property_group;
        let font = &ui.styles().text.body;
        let color = if self.enabled {
            style.label
        } else {
            style.label_disabled
        };
        let column = Rect::from_min_max(band.min, Vec2::new(end.max(band.min.x), band.max.y));
        let mut right = column.max.x;
        // From the last part to the first, each right-aligned before the next.
        for part in parts.iter().rev() {
            let room =
                Rect::from_min_max(column.min, Vec2::new(right.max(column.min.x), column.max.y));
            let width = ui.label(part, room, font, color, Align::End);
            right -= width;
        }
    }

    /// A row labeled `label`: draws the label, returns the rectangle of its input.
    pub fn row(&mut self, ui: &mut Ui<'_>, label: &str) -> Rect {
        self.row_with_height(ui, label, self.row_height)
    }

    /// A row labeled `label` whose input is `height` high (a list, a preview).
    pub fn row_with_height(&mut self, ui: &mut Ui<'_>, label: &str, height: f32) -> Rect {
        let band = self.band(height.max(0.0));
        let first_line = Rect::from_min_max(
            band.min,
            Vec2::new(band.max.x, band.min.y + self.row_height.min(band.height())),
        );
        self.label(ui, first_line, &[label]);
        self.input(band)
    }

    /// A row without label: the input column only (a checkbox, whose label follows its box).
    pub fn unlabeled(&mut self) -> Rect {
        let band = self.band(self.row_height);
        self.input(band)
    }

    /// A row across both columns (a button, a message).
    pub fn wide(&mut self) -> Rect {
        self.band(self.row_height)
    }

    /// The rows of a vector, a sub-group of its own: `label` goes with the first component
    /// only ("Location X", then "Y" and "Z"). Returns the inputs' rectangles.
    pub fn vector<const N: usize>(
        &mut self,
        ui: &mut Ui<'_>,
        label: &str,
        components: [&str; N],
    ) -> [Rect; N] {
        if self.placed {
            self.group();
        }
        let mut rects = [Rect::ZERO; N];
        for (index, (rect, component)) in rects.iter_mut().zip(components).enumerate() {
            let band = self.band(self.row_height);
            if index == 0 {
                self.label(ui, band, &[label, " ", component]);
            } else {
                self.label(ui, band, &[component]);
            }
            *rect = self.input(band);
        }
        self.group();
        rects
    }
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;
    use koebi_text::Fonts;

    use super::*;
    use crate::{Input, UiState};

    fn area() -> Rect {
        Rect::from_origin_size(Vec2::new(10.0, 20.0), Vec2::new(300.0, 400.0))
    }

    /// Runs `lay_out` in a frame, returning what it returns and the scene.
    fn frame<R>(lay_out: impl FnOnce(&mut Ui<'_>) -> R) -> (R, Scene) {
        let styles = crate::test_styles();
        let (mut scene, mut state, input) = (Scene::new(), UiState::new(), Input::new());
        state.set_fonts(Fonts::with_embedded());
        let result = {
            let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
            lay_out(&mut ui)
        };
        (result, scene)
    }

    #[test]
    fn inputs_fill_the_column_after_the_labels_share() {
        let style = crate::test_styles().property_group;
        let ((first, second), scene) = frame(|ui| {
            let mut group = PropertyGroup::new(ui, area());
            (group.row(ui, "Name"), group.row(ui, "Size"))
        });
        let labels = (300.0 * style.label_ratio).round();
        assert_eq!(first.min, Vec2::new(10.0 + labels, 20.0));
        assert_eq!(first.max, Vec2::new(310.0, 20.0 + style.row_height));
        // Rows of a sub-group follow each other closely.
        assert_eq!(second.min.y, first.max.y + style.row_gap);
        // "Name" and "Size".
        assert_eq!(scene.glyph_count(), 8);
    }

    #[test]
    fn sub_groups_are_apart_and_vectors_label_their_first_component() {
        let style = crate::test_styles().property_group;
        let ((name, location, after, height), scene) = frame(|ui| {
            let mut group = PropertyGroup::new(ui, area());
            let name = group.row(ui, "Name");
            let location = group.vector(ui, "Location", ["X", "Y", "Z"]);
            let after = group.row(ui, "Mode");
            (name, location, after, group.height())
        });
        assert_eq!(location[0].min.y, name.max.y + style.group_gap);
        assert_eq!(location[1].min.y, location[0].max.y + style.row_gap);
        assert_eq!(after.min.y, location[2].max.y + style.group_gap);
        // "Name", "Location X" (three parts), "Y", "Z", "Mode".
        assert_eq!(scene.glyph_count(), 4 + 8 + 1 + 1 + 1 + 1 + 4);
        assert_eq!(height, after.max.y - area().min.y);
        frame(|ui| {
            assert_eq!(PropertyGroup::measure(ui, 5, 3), height);
            assert_eq!(PropertyGroup::measure(ui, 0, 0), 0.0);
        });
    }

    #[test]
    fn labels_end_before_the_gap_and_follow_the_enabled_state() {
        let style = crate::test_styles().property_group;
        let (rects, _) = frame(|ui| {
            let mut group = PropertyGroup::new(ui, area());
            let wide = group.wide();
            let bare = group.unlabeled();
            group.set_enabled(false);
            let tall = group.row_with_height(ui, "List", 100.0);
            (wide, bare, tall)
        });
        let (wide, bare, tall) = rects;
        assert_eq!((wide.min.x, wide.max.x), (10.0, 310.0));
        assert_eq!(bare.min.x, 10.0 + (300.0 * style.label_ratio).round());
        assert_eq!(tall.height(), 100.0);
        let ratio = frame(|ui| {
            let mut group = PropertyGroup::new(ui, area()).label_ratio(ui, 0.5);
            group.row(ui, "Half")
        })
        .0;
        assert_eq!(ratio.min.x, 160.0);
        // Rows and gaps sized at the call.
        let (rows, _) = frame(|ui| {
            let mut group = PropertyGroup::new(ui, area())
                .row_height(ui, 30.0)
                .gaps(ui, 5.0, 20.0);
            let first = group.row(ui, "A");
            let second = group.row(ui, "B");
            group.group();
            (first, second, group.row(ui, "C"))
        });
        assert_eq!(rows.0.height(), 30.0);
        assert_eq!(rows.1.min.y, rows.0.max.y + 5.0);
        assert_eq!(rows.2.min.y, rows.1.max.y + 20.0);
    }
}
