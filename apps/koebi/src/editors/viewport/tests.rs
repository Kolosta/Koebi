//! The viewport's camera, its rulers' numbers, and every gesture through the application's
//! keymap, as a window routes them.

use std::time::Instant;

use koebi_core::input::Modifiers;
use koebi_input::{InputEvent, InputSettings, Keymap};
use koebi_text::Fonts;
use koebi_ui::{Input, Styles, UiState};

use super::*;
use crate::actions;

fn area() -> Rect {
    Rect::from_origin_size(Vec2::new(100.0, 50.0), Vec2::new(400.0, 300.0))
}

/// A4 landscape at the origin.
fn page() -> koebi_geom::Rect {
    koebi_geom::Rect::new(Point::ORIGIN, Point::new(297.0, 210.0))
}

fn camera(center: Point, zoom: f64) -> View {
    View {
        camera: Camera { center, zoom },
        ..View::default()
    }
}

#[track_caller]
fn assert_near(got: f64, want: f64, tolerance: f64) {
    assert!(
        (got - want).abs() <= tolerance,
        "{got} is not within {tolerance} of {want}"
    );
}

#[test]
fn screen_and_document_round_trip_at_every_zoom() {
    let zooms = [1e-6, 1e-4, 1e-2, 1.0, 37.5, 1e3, 1e6, 1e9];
    let centers = [
        Point::ORIGIN,
        Point::new(1.0e5, -2.5e4),
        Point::new(0.3, 0.7),
    ];
    for zoom in zooms {
        for center in centers {
            let view = camera(center, zoom);
            for screen in [
                Vec2::new(117.0, 61.0),
                area().center(),
                Vec2::new(499.5, 349.0),
            ] {
                let document = view.to_document(area(), screen);
                let [x, y] = view.to_screen(area(), document);
                // As exact as f64 holds the document point: a few of its last bits, seen at
                // this zoom; within a billionth of a pixel near the origin.
                let magnitude = document.x.abs().max(document.y.abs());
                let pixels = 1e-9 + 4.0 * f64::EPSILON * magnitude * zoom;
                assert_near(x, f64::from(screen.x), pixels);
                assert_near(y, f64::from(screen.y), pixels);
                // And back to the same document point, to its last bits.
                let back = view.to_document(area(), Vec2::new(screen.x, screen.y));
                assert!((back - document).length() <= 4.0 * f64::EPSILON * magnitude);
            }
        }
    }
}

#[test]
fn zooming_keeps_the_point_under_the_pointer() {
    let mut view = View::default();
    let anchor = Vec2::new(150.0, 80.0);
    let before = view.to_document(area(), anchor);
    view.zoom_at(area(), anchor, 2.0);
    assert_near(view.camera.zoom, 2.0 * ACTUAL, 1e-12);
    assert!((view.to_document(area(), anchor) - before).length() < 1e-9);
    // Deep in, the point stays as exactly.
    for _ in 0..200 {
        view.zoom_at(area(), anchor, 1.15);
    }
    assert!((view.to_document(area(), anchor) - before).length() < 1e-9);
    // The zoom stays within its limits, and factors that are not one do nothing.
    view.zoom_at(area(), anchor, 1e40);
    assert_near(view.camera.zoom, ZOOM_RANGE.1, 1.0);
    view.zoom_at(area(), anchor, f64::NAN);
    view.zoom_at(area(), anchor, -1.0);
    assert_near(view.camera.zoom, ZOOM_RANGE.1, 1.0);
}

#[test]
fn fitting_frames_the_page_in_the_area() {
    let fitted = View::fitted(area(), page()).expect("a camera");
    let view = View {
        camera: fitted,
        ..View::default()
    };
    assert_eq!(fitted.center, page().center());
    let (min, max) = (
        view.to_screen(area(), page().min),
        view.to_screen(area(), page().max),
    );
    assert_near(max[0] - min[0], 360.0, 1e-9);
    assert!(max[1] - min[1] <= f64::from(area().height()));
    let empty = koebi_geom::Rect::new(Point::ORIGIN, Point::ORIGIN);
    assert_eq!(View::fitted(area(), empty), None);
}

#[test]
fn a_framing_move_eases_to_its_camera() {
    let mut view = View::default();
    let to = Camera {
        center: Point::new(100.0, 50.0),
        zoom: 4.0,
    };
    let easing = CubicBezier {
        x1: 0.4,
        y1: 0.0,
        x2: 0.2,
        y2: 1.0,
    };
    let second = Duration::from_secs(1);
    view.move_to(to, Duration::ZERO);
    assert!(view.advance(Duration::from_millis(500), second, &easing));
    assert!(view.camera.zoom > ACTUAL.min(4.0) && view.camera.zoom < ACTUAL.max(4.0));
    assert!(!view.advance(second, second, &easing));
    assert_eq!(view.camera, to);
    assert_eq!(view.motion, None);
}

#[test]
fn the_canvas_image_fills_whole_physical_pixels() {
    let view = camera(Point::new(10.0, 20.0), 2.0);
    let area = Rect::from_origin_size(Vec2::new(10.3, 20.6), Vec2::new(100.2, 50.1));
    let (canvas, snapped) = view.canvas(area, 1.5).expect("a view");
    // From 15.45 to 165.75 physical pixels across: 15 to 166.
    assert_eq!(canvas.extent, Extent::new(151, 75));
    for edge in [snapped.min.x, snapped.min.y, snapped.max.x, snapped.max.y] {
        assert!((edge * 1.5 - (edge * 1.5).round()).abs() < 1e-4, "{edge}");
    }
    assert_near(canvas.zoom, 3.0, 1e-12);
    // The document point at the image's middle is the one the area shows there.
    let middle = view.to_document(area, snapped.center());
    assert!((canvas.center - middle).length() < 1e-5);
    // A document point lands on the image where the area shows it.
    let point = Point::new(12.0, 21.0);
    let [x, y] = canvas.to_target(point);
    let [sx, sy] = view.to_screen(area, point);
    assert_near(x, (sx - f64::from(snapped.min.x)) * 1.5, 1e-6);
    assert_near(y, (sy - f64::from(snapped.min.y)) * 1.5, 1e-6);
    assert!(view.canvas(Rect::ZERO, 1.0).is_none());
}

#[test]
fn rulers_take_their_side() {
    let mut view = View::default();
    let rect = area();
    assert_eq!(
        view.canvas_area(rect, 20.0, 30.0).min,
        rect.min + Vec2::new(30.0, 20.0)
    );
    view.rulers = [false, false, true, true];
    assert_eq!(
        view.canvas_area(rect, 20.0, 30.0).max,
        rect.max - Vec2::new(30.0, 20.0)
    );
}

#[cfg(target_vendor = "apple")]
const PRIMARY: Modifiers = Modifiers::LEFT_GUI;
#[cfg(not(target_vendor = "apple"))]
const PRIMARY: Modifiers = Modifiers::LEFT_CTRL;

/// A viewport in a window of its own, its input routed through the application's keymap as
/// [`AppWindow::handle`](crate::window::AppWindow::handle) routes it, one frame at a time.
struct Harness {
    design: DesignSystem,
    styles: Styles,
    state: UiState,
    input: Input,
    keymap: Keymap,
    chain: Vec<Context>,
    view: View,
    views: CanvasViews,
    rect: Rect,
    modifiers: Modifiers,
    /// Where the pointer is.
    pointer: Vec2,
}

impl Harness {
    fn new() -> Self {
        Self::sized(Vec2::new(800.0, 600.0))
    }

    /// A viewport filling a window of `size`.
    fn sized(size: Vec2) -> Self {
        let design = DesignSystem::new(&koebi_tokens::KOEBI).expect("Koebi's schema is valid");
        let mut state = UiState::new();
        state.set_fonts(Fonts::with_embedded());
        let mut harness = Self {
            styles: Styles::resolve(&design),
            design,
            state,
            input: Input::new(),
            keymap: Keymap::new(&actions::GROUPS).expect("the default shortcuts do not conflict"),
            chain: Vec::new(),
            view: View::default(),
            views: CanvasViews::default(),
            rect: Rect::from_origin_size(Vec2::ZERO, size),
            modifiers: Modifiers::NONE,
            pointer: Vec2::ZERO,
        };
        // The first frame frames the page and lays out the zones the keymap reads.
        harness.frame();
        harness
    }

    /// The canvas area of the last frame.
    fn area(&self) -> Rect {
        self.view.area
    }

    fn style(&self) -> Style {
        Style::resolve(&self.design)
    }

    fn frame(&mut self) -> koebi_render::Scene {
        let mut scene = koebi_render::Scene::new();
        self.frame_into(&mut scene);
        scene
    }

    /// Draws a frame into `scene`, cleared first.
    fn frame_into(&mut self, scene: &mut koebi_render::Scene) {
        scene.clear();
        self.views.begin_frame();
        self.state.set_window(self.rect);
        {
            let mut ui = Ui::new(scene, &self.styles, &self.input, &mut self.state);
            let shown = Shown {
                editor: EditorId::from_saved(1),
                page: Some(page()),
                views: &mut self.views,
            };
            show(
                &mut ui,
                WidgetId::new("viewport"),
                self.rect,
                &self.design,
                &mut self.view,
                shown,
            );
        }
        self.input.end_frame();
    }

    /// Feeds `event` to the input and to the keymap, through the contexts at `position`.
    fn send(&mut self, event: InputEvent, position: Vec2) {
        self.state.chain_at(Some(position), &mut self.chain);
        let triggers = self.keymap.handle(
            event,
            &self.chain,
            &InputSettings::default(),
            Instant::now(),
        );
        for triggered in triggers.iter() {
            self.input
                .trigger_by(triggered.action, actions::amount(triggered.gesture.kind));
        }
    }

    fn hold(&mut self, modifiers: Modifiers) {
        self.modifiers = modifiers;
        self.input.set_modifiers(modifiers);
    }

    fn move_to(&mut self, position: Vec2) {
        self.pointer = position;
        self.input.pointer_moved(position);
        let modifiers = self.modifiers;
        self.send(
            InputEvent::PointerMove {
                position,
                modifiers,
            },
            position,
        );
    }

    fn wheel(&mut self, delta: Vec2) {
        let position = self.pointer;
        self.input.scrolled(delta);
        let modifiers = self.modifiers;
        self.send(
            InputEvent::Wheel {
                delta,
                position,
                modifiers,
            },
            position,
        );
    }

    fn pinch(&mut self, scale: f32) {
        let position = self.pointer;
        self.send(
            InputEvent::Pinch {
                scale,
                position,
                modifiers: Modifiers::NONE,
            },
            position,
        );
    }

    fn button(&mut self, button: MouseButton, down: bool) {
        let position = self.pointer;
        self.input.mouse_button(button, down);
        if let Some(pointer) = actions::pointer_button(button) {
            self.input.button(pointer, down, position);
        }
        let modifiers = self.modifiers;
        self.send(
            InputEvent::Button {
                button,
                down,
                clicks: 1,
                position,
                modifiers,
            },
            position,
        );
    }

    fn press(&mut self, key: Key) {
        let position = self.pointer;
        let modifiers = self.modifiers;
        for down in [true, false] {
            self.input.key(key, down);
            self.send(
                InputEvent::Key {
                    key,
                    down,
                    repeat: false,
                    modifiers,
                },
                position,
            );
        }
    }

    /// The document point under `screen`.
    fn under(&self, screen: Vec2) -> Point {
        self.view.to_document(self.area(), screen)
    }

    /// Draws frames until the camera's move ends.
    fn settle(&mut self) {
        for _ in 0..200 {
            if self.view.motion.is_none() {
                return;
            }
            let next = self.input.time() + Duration::from_millis(16);
            self.input.set_time(next);
            self.frame();
        }
        panic!("the camera never settles");
    }
}

#[test]
fn the_first_frame_fits_the_page() {
    let harness = Harness::new();
    let fitted = View::fitted(harness.area(), page()).expect("a camera");
    assert_eq!(harness.view.camera, fitted);
}

#[test]
fn the_wheel_zooms_around_the_pointer_by_the_step_of_its_token() {
    let mut harness = Harness::new();
    let at = Vec2::new(300.0, 200.0);
    harness.move_to(at);
    harness.frame();
    let (before, zoom) = (harness.under(at), harness.view.camera.zoom);
    harness.wheel(Vec2::new(0.0, 1.0));
    harness.frame();
    assert_near(
        harness.view.camera.zoom,
        zoom * harness.style().zoom_step,
        1e-9,
    );
    assert!((harness.under(at) - before).length() < 1e-9);
    // Two notches back out, and a touchpad's many small turns add up in one frame.
    harness.wheel(Vec2::new(0.0, -1.0));
    harness.wheel(Vec2::new(0.0, -0.5));
    harness.wheel(Vec2::new(0.0, -0.5));
    harness.frame();
    assert_near(
        harness.view.camera.zoom,
        zoom / harness.style().zoom_step,
        1e-9,
    );
    assert!((harness.under(at) - before).length() < 1e-9);
}

#[test]
fn a_pinch_zooms_around_the_pointer_by_its_scale() {
    let mut harness = Harness::new();
    let at = Vec2::new(250.0, 300.0);
    harness.move_to(at);
    harness.frame();
    let (before, zoom) = (harness.under(at), harness.view.camera.zoom);
    harness.pinch(1.5);
    harness.pinch(2.0);
    harness.frame();
    assert_near(harness.view.camera.zoom, zoom * 3.0, 1e-6 * zoom);
    harness.pinch(0.5);
    harness.frame();
    assert_near(harness.view.camera.zoom, zoom * 1.5, 1e-6 * zoom);
    assert!((harness.under(at) - before).length() < 1e-6);
}

#[test]
fn ctrl_shift_and_a_sideways_wheel_pan() {
    let mut harness = Harness::new();
    let at = Vec2::new(300.0, 200.0);
    harness.move_to(at);
    harness.frame();
    let step = harness.style().pan_step / harness.view.camera.zoom;
    let start = harness.view.camera.center;
    // Ctrl and the wheel turned up: the view goes left.
    harness.hold(Modifiers::LEFT_CTRL);
    harness.wheel(Vec2::new(0.0, 1.0));
    harness.frame();
    assert!((harness.view.camera.center - (start + Vector::new(-step, 0.0))).length() < 1e-9);
    // Shift and the wheel turned down twice: the view goes down.
    harness.hold(Modifiers::LEFT_SHIFT);
    harness.wheel(Vec2::new(0.0, -2.0));
    harness.frame();
    let moved = start + Vector::new(-step, 2.0 * step);
    assert!((harness.view.camera.center - moved).length() < 1e-9);
    // A sideways wheel, alone: the view follows it.
    harness.hold(Modifiers::NONE);
    harness.wheel(Vec2::new(0.5, 0.0));
    harness.frame();
    let moved = moved + Vector::new(0.5 * step, 0.0);
    assert!((harness.view.camera.center - moved).length() < 1e-9);
    // The zoom never changed.
    let zoom = View::fitted(harness.area(), page()).expect("a camera").zoom;
    assert_near(harness.view.camera.zoom, zoom, 1e-12);
}

#[test]
fn the_middle_button_drags_the_canvas_until_it_is_released() {
    let mut harness = Harness::new();
    let at = Vec2::new(300.0, 200.0);
    harness.move_to(at);
    harness.frame();
    let held = harness.under(at);
    harness.button(MouseButton::Middle, true);
    harness.frame();
    for to in [Vec2::new(340.0, 230.0), Vec2::new(420.0, 150.0)] {
        harness.move_to(to);
        harness.frame();
        // The document point under the press stays under the pointer.
        assert!((harness.under(to) - held).length() < 1e-9);
    }
    harness.button(MouseButton::Middle, false);
    harness.frame();
    let camera = harness.view.camera;
    harness.move_to(Vec2::new(200.0, 100.0));
    harness.frame();
    assert_eq!(harness.view.camera, camera);
    assert_eq!(harness.view.grip, None);
}

#[test]
fn the_keypad_zooms_on_the_middle_and_pans() {
    let mut harness = Harness::new();
    harness.move_to(Vec2::new(250.0, 120.0));
    harness.frame();
    let middle = harness.area().center();
    let (before, zoom) = (harness.under(middle), harness.view.camera.zoom);
    harness.press(Key::KeypadPlus);
    harness.frame();
    assert_near(
        harness.view.camera.zoom,
        zoom * harness.style().zoom_step,
        1e-9,
    );
    assert!((harness.under(middle) - before).length() < 1e-9);
    harness.press(Key::KeypadMinus);
    harness.frame();
    assert_near(harness.view.camera.zoom, zoom, 1e-9);
    let start = harness.view.camera.center;
    harness.hold(Modifiers::LEFT_CTRL);
    harness.press(Key::Keypad8);
    harness.frame();
    let step = harness.style().pan_step / zoom;
    assert!((harness.view.camera.center - (start + Vector::new(0.0, -step))).length() < 1e-9);
    harness.hold(Modifiers::NONE);
    harness.press(Key::Keypad1);
    harness.frame();
    assert_near(harness.view.camera.zoom, ACTUAL, 1e-12);
}

#[test]
fn framing_moves_the_camera_in_an_eased_motion() {
    let mut harness = Harness::new();
    let fitted = harness.view.camera;
    harness.move_to(Vec2::new(300.0, 200.0));
    harness.wheel(Vec2::new(0.0, 4.0));
    harness.frame();
    assert_ne!(harness.view.camera, fitted);
    // Shift C: Fit Document, eased from where the camera is.
    harness.hold(Modifiers::LEFT_SHIFT);
    harness.press(Key::C);
    harness.frame();
    assert!(harness.view.motion.is_some());
    harness.settle();
    assert_eq!(harness.view.camera, fitted);
    // Ctrl (Command) and the keypad's 0: Reset View, the page centered at its real size.
    harness.hold(PRIMARY);
    harness.press(Key::Keypad0);
    harness.frame();
    harness.settle();
    assert_eq!(harness.view.camera.center, page().center());
    assert_near(harness.view.camera.zoom, ACTUAL, 1e-12);
    // The keypad's period: Frame Selected, the document without a selection.
    harness.hold(Modifiers::NONE);
    harness.press(Key::KeypadPeriod);
    harness.frame();
    harness.settle();
    assert_eq!(harness.view.camera, fitted);
}

#[test]
fn overlays_toggle_from_their_actions() {
    let mut harness = Harness::new();
    harness.move_to(Vec2::new(300.0, 200.0));
    harness.frame();
    harness.hold(PRIMARY);
    harness.press(Key::R);
    harness.frame();
    assert_eq!(harness.view.rulers, [false; 4]);
    harness.press(Key::R);
    harness.frame();
    assert_eq!(harness.view.rulers, [true, true, false, false]);
    harness.input.trigger(&TOGGLE_GRID);
    harness.frame();
    assert!(!harness.view.grid);
}

#[test]
fn a_viewport_asks_for_its_view_of_the_document() {
    let mut harness = Harness::new();
    assert!(harness.views.any_wanted());
    let area = harness.area();
    let (wanted, _) = harness.view.canvas(area, 1.0).expect("a view");
    let editor = EditorId::from_saved(1);
    harness.views.begin_frame();
    assert_eq!(harness.views.show(editor, wanted), None);
    harness.frame();
    assert!(harness.views.any_wanted());
}

/// The viewport's frames rendered offscreen as a window renders them: the canvas draws the
/// views the frame asked for into their targets, then the frame shows them.
struct Offscreen<'a> {
    gpu: &'a koebi_testkit::HeadlessGpu,
    canvas: koebi_canvas::Canvas,
    renderer: koebi_render::Renderer2D,
    targets: crate::canvas::CanvasTargets,
    program: std::sync::Arc<koebi_doc::eval::Program>,
}

impl<'a> Offscreen<'a> {
    const FORMAT: koebi_gpu::TextureFormat = koebi_gpu::TextureFormat::Rgba8Unorm;

    fn new(gpu: &'a koebi_testkit::HeadlessGpu) -> Self {
        let mut renderer =
            koebi_render::Renderer2D::new(&gpu.device, Self::FORMAT).expect("a renderer");
        renderer.set_fonts(Fonts::with_embedded());
        renderer
            .set_icons(koebi_icons::packed())
            .expect("a valid icon set");
        Self {
            gpu,
            canvas: koebi_canvas::Canvas::new(&gpu.device).expect("a canvas"),
            renderer,
            targets: crate::canvas::CanvasTargets::new(),
            program: crate::document::DocumentState::new()
                .program()
                .expect("the default document's page"),
        }
    }

    /// Draws a frame of `harness` and renders it; returns the image, whether a view got its
    /// first image, and how many allocations drawing the views made.
    fn render(&mut self, harness: &mut Harness) -> (koebi_testkit::Image, bool, u64) {
        let mut scene = harness.frame();
        scene.set_clear_color(harness.style().canvas);
        let Self {
            gpu,
            canvas,
            renderer,
            targets,
            program,
        } = self;
        let (mut appeared, mut allocations) = (false, 0);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the window's size in pixels"
        )]
        let extent = Extent::new(harness.rect.width() as u32, harness.rect.height() as u32);
        let image = gpu
            .render_offscreen(extent, Self::FORMAT, |encoder, target| {
                let (count, drawn) = koebi_testkit::allocations(|| {
                    targets.render(
                        &gpu.device,
                        encoder,
                        canvas,
                        renderer,
                        &mut harness.views,
                        Some(program),
                    )
                });
                appeared = drawn.expect("the views drawn");
                allocations = count;
                renderer
                    .render(encoder, target, &scene, 0, 1.0)
                    .expect("the frame rendered");
            })
            .expect("offscreen rendering");
        (image, appeared, allocations)
    }

    /// The image of `harness` once its views show.
    fn settled(&mut self, harness: &mut Harness) -> koebi_testkit::Image {
        self.render(harness);
        let (image, again, _) = self.render(harness);
        assert!(
            !again,
            "a view shows from the frame after its image appears"
        );
        image
    }
}

#[track_caller]
fn assert_golden(image: &koebi_testkit::Image, name: &str) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    koebi_testkit::assert_golden_at(
        &root.join("tests").join("golden"),
        &std::env::temp_dir().join("koebi-golden-failures"),
        name,
        image,
        koebi_testkit::Tolerance::new(3, 8),
    );
}

#[test]
fn the_default_document_matches_golden_at_several_zooms() {
    let Some(gpu) =
        koebi_testkit::gpu_or_skip("the_default_document_matches_golden_at_several_zooms")
    else {
        return;
    };
    let mut offscreen = Offscreen::new(&gpu);
    let mut harness = Harness::sized(Vec2::new(480.0, 320.0));
    // The first frame asks for the view; its image shows from the next.
    let (_, appeared, _) = offscreen.render(&mut harness);
    assert!(appeared);
    let (fitted, appeared, _) = offscreen.render(&mut harness);
    assert!(!appeared);
    assert_golden(&fitted, "viewport_fit");
    // A view that did not change draws nothing again, and allocates nothing.
    let (_, _, allocations) = offscreen.render(&mut harness);
    assert_eq!(allocations, 0);
    // The page at its real size, centered: wider than the view.
    harness.move_to(Vec2::new(240.0, 160.0));
    harness.hold(PRIMARY);
    harness.press(Key::Keypad0);
    harness.frame();
    harness.settle();
    harness.hold(Modifiers::NONE);
    assert_golden(&offscreen.settled(&mut harness), "viewport_actual");
    // Close on the page's top left corner, its edge on whole pixels.
    harness.press(Key::Home);
    harness.frame();
    harness.settle();
    let corner = harness.view.to_screen_f32(harness.area(), Point::ORIGIN);
    harness.move_to(corner + Vec2::new(0.5, 0.5));
    harness.wheel(Vec2::new(0.0, 16.0));
    assert_golden(&offscreen.settled(&mut harness), "viewport_corner");
    // Far out: the page small among the dots.
    harness.wheel(Vec2::new(0.0, -30.0));
    assert_golden(&offscreen.settled(&mut harness), "viewport_far");
    // A window resized: the same image shows the view at its new size.
    let editor = EditorId::from_saved(1);
    let shown = |harness: &mut Harness| {
        let (view, _) = harness.view.canvas(harness.area(), 1.0).expect("a view");
        harness.views.show(editor, view)
    };
    let before = shown(&mut harness);
    assert!(before.is_some());
    harness.rect = Rect::from_origin_size(Vec2::ZERO, Vec2::new(360.0, 240.0));
    let (_, appeared, _) = offscreen.render(&mut harness);
    assert!(!appeared);
    assert_eq!(shown(&mut harness), before);
    // A view no viewport asks for gives back its target and its image.
    assert_eq!(offscreen.renderer.image_count(), 1);
    harness.views.begin_frame();
    let Offscreen {
        canvas,
        renderer,
        targets,
        program,
        ..
    } = &mut offscreen;
    gpu.render_offscreen(Extent::new(8, 8), Offscreen::FORMAT, |encoder, _| {
        targets
            .render(
                &gpu.device,
                encoder,
                canvas,
                renderer,
                &mut harness.views,
                Some(program),
            )
            .expect("nothing drawn");
    })
    .expect("offscreen rendering");
    assert_eq!(renderer.image_count(), 0);
    gpu.assert_no_validation_errors();
}

#[test]
fn steady_frames_of_a_viewport_allocate_nothing() {
    let mut harness = Harness::new();
    let mut scene = koebi_render::Scene::new();
    let mut failures = Vec::new();
    let mut check = |harness: &mut Harness, what: &str| {
        for _ in 0..5 {
            harness.frame_into(&mut scene);
        }
        let (count, ()) = koebi_testkit::allocations(|| harness.frame_into(&mut scene));
        if count > 0 {
            failures.push(format!("{what}: {count}"));
        }
    };
    check(&mut harness, "still");
    harness.move_to(Vec2::new(300.0, 200.0));
    check(&mut harness, "the pointer over the canvas");
    harness.move_to(Vec2::new(300.0, 6.0));
    check(&mut harness, "the pointer over a ruler");
    harness.move_to(Vec2::new(300.0, 200.0));
    harness.wheel(Vec2::new(0.0, 12.0));
    check(&mut harness, "zoomed in, the rulers in decimals");
    harness.button(MouseButton::Middle, true);
    check(&mut harness, "holding the canvas");
    harness.move_to(Vec2::new(320.0, 210.0));
    check(&mut harness, "dragged");
    harness.button(MouseButton::Middle, false);
    harness.hold(Modifiers::LEFT_SHIFT);
    harness.press(Key::C);
    check(&mut harness, "framing");
    assert!(
        failures.is_empty(),
        "allocations in steady frames: {failures:#?}"
    );
    // The canvas, its grid and its rulers were drawn.
    assert!(scene.rect_count() > 10 && scene.glyph_count() > 10);
}

#[test]
fn a_click_on_the_rulers_corner_cycles_their_units() {
    let mut harness = Harness::new();
    let corner = Rect::from_min_max(harness.rect.min, harness.area().min);
    harness.move_to(corner.center());
    harness.frame();
    for want in [
        Units::Metric,
        Units::Imperial,
        Units::Typographic,
        Units::Pixel,
        Units::Document,
    ] {
        harness.button(MouseButton::Left, true);
        harness.frame();
        harness.button(MouseButton::Left, false);
        harness.frame();
        assert_eq!(harness.view.units, want);
    }
}

#[test]
fn two_fingers_drag_and_spread_the_canvas() {
    let mut harness = Harness::new();
    let (a, b) = (Vec2::new(300.0, 300.0), Vec2::new(400.0, 300.0));
    harness.input.touched(1, a);
    harness.input.touched(2, b);
    harness.frame();
    let held = harness.under(Vec2::new(350.0, 300.0));
    let zoom = harness.view.camera.zoom;
    // Both fingers move: the point under their middle follows it.
    let by = Vec2::new(30.0, -20.0);
    harness.input.finger_moved(1, a + by);
    harness.input.finger_moved(2, b + by);
    harness.frame();
    let middle = Vec2::new(380.0, 280.0);
    assert!((harness.under(middle) - held).length() < 1e-9);
    assert_near(harness.view.camera.zoom, zoom, 1e-12);
    // They spread to twice their distance around the same middle: twice the zoom there.
    harness.input.finger_moved(1, Vec2::new(280.0, 280.0));
    harness.input.finger_moved(2, Vec2::new(480.0, 280.0));
    harness.frame();
    assert_near(harness.view.camera.zoom, 2.0 * zoom, 1e-6 * zoom);
    assert!((harness.under(middle) - held).length() < 1e-6);
    // A system pinch zooms through the keymap instead: the spread no longer does.
    harness.input.system_pinched();
    harness.input.finger_moved(2, Vec2::new(580.0, 280.0));
    harness.frame();
    assert!(harness.view.camera.zoom < 2.0 * zoom * (1.0 + 1e-6));
    harness.input.lifted(1);
    harness.input.lifted(2);
    harness.frame();
    assert_eq!(harness.input.touch_count(), 0);
}
