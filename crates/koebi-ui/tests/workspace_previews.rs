//! The previews of the workspace on screen: a join whose leftover goes to a neighbour, with
//! everything around the future area darkened, matched against its golden image.

mod common;

use common::render;
use koebi_core::{Color, Extent, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_render::{RectPrimitive, Scene};
use koebi_testkit::{Tolerance, assert_golden, gpu_or_skip};
use koebi_text::Fonts;
use koebi_tokens::KOEBI;
use koebi_ui::{EditorHost, Input, Styles, Ui, UiState, WorkspaceView};
use koebi_workspace::{EditorId, GRID, GridRect, Layout, Workspace};

/// Anti-aliased edges may differ by a few units between drivers.
const CROSS_DRIVER: Tolerance = Tolerance::new(3, 8);

/// Editors filled with a color of their own, so that the darkening shows.
struct Host;

impl EditorHost for Host {
    fn title(&self, _editor: EditorId) -> &'static str {
        "Editor"
    }

    fn content(&mut self, ui: &mut Ui<'_>, _editor: EditorId, rect: Rect) {
        ui.scene()
            .push_rect(&RectPrimitive::new(rect, Color::rgb(0.25, 0.45, 0.65)));
    }

    fn copied(&mut self, _source: EditorId, _editor: EditorId) {}

    fn removed(&mut self, _editors: &[EditorId]) {}
}

fn join_scene(design: &DesignSystem) -> Scene {
    let styles = Styles::resolve(design);
    let half = GRID / 2;
    let areas = [
        (0, GridRect::new(0, 0, half, GRID)),
        (1, GridRect::new(half, 0, GRID, half)),
        (2, GridRect::new(half, half, GRID, GRID)),
    ];
    let layout = Layout::from_areas(&areas).expect("the areas tile the grid");
    let stacks: Vec<(u32, Vec<u32>, usize)> =
        areas.iter().map(|(id, _)| (*id, vec![*id], 0)).collect();
    let mut workspace = Workspace::from_saved(layout, &stacks).expect("one editor per area");
    let mut view = WorkspaceView::new();
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let mut input = Input::new();
    let window = Rect::from_origin_size(Vec2::ZERO, Vec2::new(320.0, 240.0));
    let mut frame = |input: &mut Input, state: &mut UiState| {
        let mut scene = Scene::new();
        scene.set_clear_color(styles.background);
        {
            let mut ui = Ui::new(&mut scene, &styles, input, state);
            ui.workspace(&mut view, &mut workspace, window, &mut Host);
        }
        input.end_frame();
        scene
    };
    // The bottom-left corner of the top right area, dragged into the left area.
    let corner = Vec2::new(166.0, 114.0);
    let target = Vec2::new(80.0, 60.0);
    input.pointer_moved(corner);
    frame(&mut input, &mut state);
    input.primary_button(true, corner);
    frame(&mut input, &mut state);
    input.pointer_moved((corner + target) * 0.5);
    frame(&mut input, &mut state);
    input.pointer_moved(target);
    frame(&mut input, &mut state);
    frame(&mut input, &mut state)
}

#[test]
fn a_partial_join_matches_golden() {
    let Some(gpu) = gpu_or_skip("a_partial_join_matches_golden") else {
        return;
    };
    let design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
    let scene = join_scene(&design);
    let image = render(&gpu, Extent::new(320, 240), &scene);
    assert_golden!(image, "workspace_join_dark", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}
