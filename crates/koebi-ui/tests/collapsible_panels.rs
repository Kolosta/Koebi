//! Collapsible panels on screen: three nested levels, each header in its parent's content
//! color and each content in its own, a closed panel, a header with an icon, matching their
//! golden image.

mod common;

use common::render;
use koebi_core::{Extent, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_render::Scene;
use koebi_testkit::{Tolerance, assert_golden, gpu_or_skip};
use koebi_text::Fonts;
use koebi_tokens::KOEBI;
use koebi_ui::{
    Checkbox, CollapsiblePanel, CollapsiblePanelState, Icon, Input, PanelLevel, Styles, Ui,
    UiState, WidgetId,
};

/// Anti-aliased edges may differ by a few units between drivers.
const CROSS_DRIVER: Tolerance = Tolerance::new(3, 8);

/// A checkbox row inset by the panel padding at `y` in `rect`; returns the row's height.
fn row(ui: &mut Ui<'_>, rect: Rect, y: f32, id: &'static str, label: &str) -> f32 {
    let padding = ui.styles().panel.padding;
    let mut checked = true;
    let checkbox = Checkbox::new(WidgetId::new(id), &mut checked).label(label);
    let size = checkbox.size(ui);
    checkbox.show(
        ui,
        Rect::from_origin_size(Vec2::new(rect.min.x + padding, y), size),
    );
    size.y
}

fn panel_sheet(design: &DesignSystem) -> Scene {
    let styles = Styles::resolve(design);
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let input = Input::new();
    let mut panels = [
        CollapsiblePanelState::new(true),
        CollapsiblePanelState::new(true),
        CollapsiblePanelState::new(true),
        CollapsiblePanelState::new(false),
    ];
    let area = Rect::from_origin_size(Vec2::new(8.0, 8.0), Vec2::new(264.0, 304.0));
    // A panel measures its content in the frame it first draws it, and asks for the next
    // frame: nested levels settle one frame each.
    for _ in 0..4 {
        scene.clear();
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        let [transform, delta, advanced, visibility] = &mut panels;
        let first = CollapsiblePanel::new(WidgetId::new("transform"), transform, "Transform")
            .icon(Icon::Plus)
            .show(&mut ui, area, |ui, rect| {
                let mut y = rect.min.y;
                y += row(ui, rect, y, "snap", "Snap to grid") + 4.0;
                let below = Rect::from_min_max(Vec2::new(rect.min.x, y), rect.max);
                let second = CollapsiblePanel::new(WidgetId::new("delta"), delta, "Delta")
                    .level(PanelLevel::Second)
                    .show(ui, below, |ui, rect| {
                        let mut y = rect.min.y;
                        y += row(ui, rect, y, "relative", "Relative") + 4.0;
                        let below = Rect::from_min_max(Vec2::new(rect.min.x, y), rect.max);
                        let third =
                            CollapsiblePanel::new(WidgetId::new("advanced"), advanced, "Advanced")
                                .level(PanelLevel::Third)
                                .show(ui, below, |ui, rect| {
                                    row(ui, rect, rect.min.y, "lock", "Lock")
                                });
                        y + third.height - rect.min.y
                    });
                y + second.height - rect.min.y
            });
        let gap = ui.styles().collapsible_panel.gap;
        let next = Rect::from_min_max(
            Vec2::new(area.min.x, area.min.y + first.height + gap),
            area.max,
        );
        CollapsiblePanel::new(WidgetId::new("visibility"), visibility, "Visibility").show(
            &mut ui,
            next,
            |_, _| 40.0,
        );
    }
    scene
}

#[test]
fn collapsible_panels_match_golden() {
    let Some(gpu) = gpu_or_skip("collapsible_panels_match_golden") else {
        return;
    };
    let design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
    let image = render(&gpu, Extent::new(280, 220), &panel_sheet(&design));
    assert_golden!(image, "collapsible_panels_dark", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}
