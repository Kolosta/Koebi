//! The curves of the gallery: the curve editor, temporary until curves get a fuller one, with
//! presets to start from and, beside it, where a square moving by the curve lies at equal
//! times, one row per time: close together where it goes slowly, apart where it goes fast, past
//! the end where it overshoots.

use koebi_core::{Rect, Vec2};
use koebi_design::value::{CubicBezier, Curve, CurveKey};
use koebi_i18n::n_;
use koebi_i18n::tr;
use koebi_render::RectPrimitive;
use koebi_ui::{
    Align, Button, ButtonGroup, CurveEditor, CurveEditorState, GroupCell, Overflow, PanelLevel, Ui,
    WidgetId,
};

use super::{Metrics, cut_left, cut_top};

/// The width of the editor.
const EDITOR_WIDTH: f32 = 360.0;

/// The widest the preview gets.
const PREVIEW_WIDTH: f32 = 480.0;

/// How many times the preview shows the moving square at.
const STEPS: usize = 11;

/// Curves to start from: their name and their keys.
const PRESETS: [(&str, &[CurveKey]); 4] = [
    (
        n_!("Linear"),
        &[
            CurveKey::new(0.0, 0.0, CubicBezier::LINEAR),
            CurveKey::new(1.0, 1.0, CubicBezier::LINEAR),
        ],
    ),
    (
        n_!("Ease in and out"),
        &[
            CurveKey::new(0.0, 0.0, CubicBezier::new(0.42, 0.0, 0.58, 1.0)),
            CurveKey::new(1.0, 1.0, CubicBezier::LINEAR),
        ],
    ),
    (
        n_!("Overshoot"),
        &[
            CurveKey::new(0.0, 0.0, CubicBezier::new(0.3, 0.0, 0.6, 1.0)),
            CurveKey::new(0.6, 1.15, CubicBezier::new(0.4, 0.0, 0.6, 1.0)),
            CurveKey::new(1.0, 1.0, CubicBezier::LINEAR),
        ],
    ),
    (
        n_!("Bounce"),
        &[
            CurveKey::new(0.0, 0.0, CubicBezier::new(0.5, 0.0, 1.0, 1.0)),
            CurveKey::new(0.55, 1.0, CubicBezier::new(0.0, 0.0, 0.5, 1.0)),
            CurveKey::new(0.75, 0.8, CubicBezier::new(0.5, 0.0, 1.0, 1.0)),
            CurveKey::new(0.9, 1.0, CubicBezier::new(0.0, 0.0, 0.5, 1.0)),
            CurveKey::new(0.95, 0.95, CubicBezier::new(0.5, 0.0, 1.0, 1.0)),
            CurveKey::new(1.0, 1.0, CubicBezier::LINEAR),
        ],
    ),
];

/// What the panel keeps: the curve edited and the editor's state.
#[derive(Debug)]
pub(super) struct Curves {
    curve: Curve,
    editor: CurveEditorState,
}

impl Curves {
    pub(super) fn new() -> Self {
        Self {
            curve: Curve::new(PRESETS[2].1),
            editor: CurveEditorState::new(),
        }
    }

    /// The height of the panel: a row of presets, then the editor.
    pub(super) fn height(ui: &Ui<'_>, metrics: Metrics) -> f32 {
        let panel = &ui.styles().panel;
        metrics.row
            + metrics.row_gap()
            + CurveEditor::height(ui)
            + 2.0 * (panel.padding + panel.border_width)
    }

    /// Draws the panel in `rect`.
    pub(super) fn show(&mut self, ui: &mut Ui<'_>, rect: Rect, metrics: Metrics) {
        const ID: WidgetId = WidgetId::new("curves");
        let mut content = ui.panel(rect, PanelLevel::First);
        let (row, gap) = (metrics.row, metrics.row_gap());
        let font = ui.styles().text.label.clone();
        let secondary = ui.styles().text.secondary;

        // The title, then the presets.
        let mut line = cut_top(&mut content, row, gap);
        let title = cut_left(&mut line, EDITOR_WIDTH, gap);
        ui.label(
            tr("Curve editor (temporary)"),
            title,
            &font,
            secondary,
            Align::Start,
        );
        let presets = Rect::from_min_max(
            line.min,
            Vec2::new((line.min.x + 4.0 * 7.0 * row).min(line.max.x), line.max.y),
        );
        let mut cells: [GroupCell<'_>; 4] = core::array::from_fn(|index| {
            let (name, keys) = PRESETS[index];
            GroupCell::new(
                Button::new(ID.with(1 + index as u64))
                    .label(name)
                    .selected(Curve::new(keys) == self.curve),
            )
        });
        if let Some(index) = ButtonGroup::horizontal()
            .show(ui, presets, &mut cells)
            .clicked
            && let Some((_, keys)) = PRESETS.get(index)
        {
            self.curve = Curve::new(keys);
            self.editor.select(None);
        }

        // The editor, and where a square moving by the curve lies at equal times.
        let editor = cut_left(&mut content, EDITOR_WIDTH, metrics.gap);
        CurveEditor::new(ID.with(10), &mut self.curve, &mut self.editor).show(ui, editor);
        let caption = cut_top(&mut content, row, gap);
        ui.label_with(
            tr("Where a square moving by the curve lies at equal times"),
            caption,
            &font,
            secondary,
            Align::Start,
            Overflow::Shrink {
                min_size: font.size * 0.8,
            },
        );
        let preview_area = Rect::from_min_max(
            content.min,
            Vec2::new(
                (content.min.x + PREVIEW_WIDTH).min(content.max.x),
                content.max.y,
            ),
        );
        preview(ui, preview_area, &self.curve);
    }
}

/// Where a square moving by `curve` from the start line to the end line of `rect` lies at
/// [`STEPS`] equal times, one row per time from the top: an overshoot passes the end line.
fn preview(ui: &mut Ui<'_>, rect: Rect, curve: &Curve) {
    let style = ui.styles().curve_editor;
    let pitch = rect.height() / STEPS as f32;
    let side = (pitch * 0.7).round().max(2.0);
    // Room past both lines for overshoots of a fifth of the way.
    let margin = (rect.width() * 0.15).max(side);
    let (start, end) = (
        rect.min.x + margin,
        (rect.max.x - margin).max(rect.min.x + margin),
    );
    let line = ui.whole_pixels(1.0);
    for x in [start, end] {
        let stroke = Rect::from_min_max(
            Vec2::new(x - line * 0.5, rect.min.y),
            Vec2::new(x + line * 0.5, rect.max.y),
        );
        ui.scene()
            .push_rect(&RectPrimitive::new(stroke, style.grid));
    }
    for step in 0..STEPS {
        let t = step as f32 / (STEPS - 1) as f32;
        let x = start + curve.ease(t) * (end - start);
        let y = rect.min.y + pitch * (step as f32 + 0.5);
        let at = Rect::from_center_size(Vec2::new(x, y), Vec2::splat(side));
        ui.scene().push_rect(
            &RectPrimitive::new(at, style.curve).with_corner_radius(style.corner_radius),
        );
    }
}
