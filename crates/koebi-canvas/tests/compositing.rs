//! Compositing on the GPU (ADR 0029): every blend mode against the W3C formulas, computed here
//! apart from the canvas, within 1/255; Carto's three defects as regressions (a blend leaving
//! what lies above it alone, an Erase in a group leaving the rest of the page, nested masks
//! applied once); a pan over a blended program allocating nothing once its targets are pooled.
//! Every image also matches the CPU reference.

mod common;

use std::sync::Arc;

use common::{Session, assert_matches, color};
use koebi_canvas::{View, reference};
use koebi_core::{Extent, LinearRgba};
use koebi_doc::eval::{Draw, MergeInput, Program, Step, StepCoverage, StepId};
use koebi_doc::{BlendMode, NodeId};
use koebi_geom::{Contours, FillRule, Path, Point, Rect, Vector};
use koebi_testkit::gpu_or_skip;

#[global_allocator]
static ALLOCATOR: koebi_testkit::CountingAllocator = koebi_testkit::CountingAllocator;

const MODES: [BlendMode; 17] = [
    BlendMode::Normal,
    BlendMode::Multiply,
    BlendMode::Screen,
    BlendMode::Overlay,
    BlendMode::Darken,
    BlendMode::Lighten,
    BlendMode::ColorDodge,
    BlendMode::ColorBurn,
    BlendMode::HardLight,
    BlendMode::SoftLight,
    BlendMode::Difference,
    BlendMode::Exclusion,
    BlendMode::Hue,
    BlendMode::Saturation,
    BlendMode::Color,
    BlendMode::Luminosity,
    BlendMode::Erase,
];

fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Arc<Contours> {
    Arc::new(Path::rectangle(Rect::new(Point::new(x0, y0), Point::new(x1, y1))).contours())
}

/// Builds programs step by step.
#[derive(Default)]
struct Steps(Vec<Step>);

impl Steps {
    fn push(&mut self, step: Step) -> StepId {
        self.0.push(step);
        StepId(self.0.len() - 1)
    }

    fn draw(&mut self, outline: &Arc<Contours>, paint: LinearRgba) -> StepId {
        self.push(Step::Draw(Draw {
            object: None,
            outline: Arc::clone(outline),
            rule: FillRule::NonZero,
            color: paint,
            opacity: 1.0,
        }))
    }

    fn merge(&mut self, inputs: &[(StepId, BlendMode, f64)]) -> StepId {
        self.push(Step::Merge(
            inputs
                .iter()
                .map(|&(step, blend, opacity)| MergeInput {
                    step,
                    blend,
                    opacity,
                })
                .collect(),
        ))
    }

    fn program(self, root: StepId, background: LinearRgba) -> Program {
        Program::new(
            NodeId(0),
            Vector::new(64.0, 48.0),
            background,
            self.0,
            Some(root),
        )
    }
}

const CLEAR: LinearRgba = LinearRgba::new(0.0, 0.0, 0.0, 0.0);
const WHITE: LinearRgba = LinearRgba::new(1.0, 1.0, 1.0, 1.0);

fn view() -> View {
    View {
        center: Point::new(32.0, 24.0),
        zoom: 1.0,
        extent: Extent::new(64, 48),
    }
}

/// The pixel of the view at document point (`x`, `y`).
fn at(pixels: &[[f32; 4]], x: f64, y: f64) -> [f32; 4] {
    let view = view();
    let [px, py] = view.to_target(Point::new(x, y));
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a pixel of the view"
    )]
    let index = (py as usize) * view.extent.width as usize + px as usize;
    pixels[index]
}

#[track_caller]
fn assert_near(got: [f32; 4], want: [f64; 4], what: &str) {
    for channel in 0..4 {
        let delta = (f64::from(got[channel]) - want[channel]).abs();
        assert!(
            delta <= 1.0 / 255.0,
            "{what}: {got:?} against {want:?} (channel {channel} off by {:.2}/255)",
            delta * 255.0
        );
    }
}

/// The W3C *Compositing and Blending Level 1* formulas, in f64, written apart from the canvas.
mod w3c {
    use koebi_doc::BlendMode;

    fn lum(c: [f64; 3]) -> f64 {
        0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
    }

    fn clip(c: [f64; 3]) -> [f64; 3] {
        let l = lum(c);
        let n = c.iter().copied().fold(f64::INFINITY, f64::min);
        let x = c.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let mut c = c;
        if n < 0.0 {
            c = c.map(|v| l + (v - l) * l / (l - n));
        }
        if x > 1.0 {
            c = c.map(|v| l + (v - l) * (1.0 - l) / (x - l));
        }
        c
    }

    fn set_lum(c: [f64; 3], l: f64) -> [f64; 3] {
        let d = l - lum(c);
        clip(c.map(|v| v + d))
    }

    fn sat(c: [f64; 3]) -> f64 {
        c.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - c.iter().copied().fold(f64::INFINITY, f64::min)
    }

    fn set_sat(c: [f64; 3], s: f64) -> [f64; 3] {
        let x = c.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let n = c.iter().copied().fold(f64::INFINITY, f64::min);
        if x > n {
            c.map(|v| (v - n) * s / (x - n))
        } else {
            [0.0; 3]
        }
    }

    fn channel(mode: BlendMode, b: f64, s: f64) -> f64 {
        let hard = |b: f64, s: f64| {
            if s <= 0.5 {
                b * 2.0 * s
            } else {
                let t = 2.0 * s - 1.0;
                b + t - b * t
            }
        };
        match mode {
            BlendMode::Multiply => b * s,
            BlendMode::Screen => b + s - b * s,
            BlendMode::Overlay => hard(s, b),
            BlendMode::Darken => b.min(s),
            BlendMode::Lighten => b.max(s),
            BlendMode::ColorDodge if b <= 0.0 => 0.0,
            BlendMode::ColorDodge if s >= 1.0 => 1.0,
            BlendMode::ColorDodge => (b / (1.0 - s)).min(1.0),
            BlendMode::ColorBurn if b >= 1.0 => 1.0,
            BlendMode::ColorBurn if s <= 0.0 => 0.0,
            BlendMode::ColorBurn => 1.0 - ((1.0 - b) / s).min(1.0),
            BlendMode::HardLight => hard(b, s),
            BlendMode::SoftLight if s <= 0.5 => b - (1.0 - 2.0 * s) * b * (1.0 - b),
            BlendMode::SoftLight => {
                let d = if b <= 0.25 {
                    ((16.0 * b - 12.0) * b + 4.0) * b
                } else {
                    b.sqrt()
                };
                b + (2.0 * s - 1.0) * (d - b)
            }
            BlendMode::Difference => (b - s).abs(),
            BlendMode::Exclusion => b + s - 2.0 * b * s,
            _ => s,
        }
    }

    /// `source` composited onto `backdrop` in `mode`, both straight colors with alpha.
    pub(super) fn composite(mode: BlendMode, backdrop: [f64; 4], source: [f64; 4]) -> [f64; 4] {
        let (cb, ab) = ([backdrop[0], backdrop[1], backdrop[2]], backdrop[3]);
        let (cs, a_s) = ([source[0], source[1], source[2]], source[3]);
        if mode == BlendMode::Erase {
            let a = ab * (1.0 - a_s);
            return [cb[0] * a, cb[1] * a, cb[2] * a, a];
        }
        let mixed = match mode {
            BlendMode::Hue => set_lum(set_sat(cs, sat(cb)), lum(cb)),
            BlendMode::Saturation => set_lum(set_sat(cb, sat(cs)), lum(cb)),
            BlendMode::Color => set_lum(cs, lum(cb)),
            BlendMode::Luminosity => set_lum(cb, lum(cs)),
            _ => [0, 1, 2].map(|i| channel(mode, cb[i], cs[i])),
        };
        let mut out = [0.0; 4];
        for i in 0..3 {
            // Premultiplied result: source-over with the blend function where both cover.
            out[i] = a_s * (1.0 - ab) * cs[i] + ab * (1.0 - a_s) * cb[i] + a_s * ab * mixed[i];
        }
        out[3] = a_s + ab * (1.0 - a_s);
        out
    }
}

/// The straight color and alpha of `paint`.
fn straight(paint: LinearRgba) -> [f64; 4] {
    [paint.r, paint.g, paint.b, paint.a].map(f64::from)
}

#[test]
fn every_blend_mode_follows_the_w3c_formulas() {
    let Some(gpu) = gpu_or_skip("every_blend_mode_follows_the_w3c_formulas") else {
        return;
    };
    let mut session = Session::new(&gpu, Extent::new(64, 48));
    // Two pairs of colors, so that the piecewise modes take both their branches.
    let pairs = [
        (color(0.3, 0.6, 0.9, 0.8), color(0.8, 0.4, 0.1, 0.7)),
        (color(0.7, 0.2, 0.15, 0.6), color(0.2, 0.9, 0.55, 0.9)),
    ];
    let (low, high) = (square(8.0, 8.0, 40.0, 40.0), square(24.0, 16.0, 56.0, 44.0));
    for (backdrop, source) in pairs {
        for mode in MODES {
            let mut steps = Steps::default();
            let below = steps.draw(&low, backdrop);
            let above = steps.draw(&high, source);
            let root = steps.merge(&[(below, BlendMode::Normal, 1.0), (above, mode, 0.75)]);
            let program = steps.program(root, CLEAR);
            let view = view();
            let pixels = session.render(&program, &view).expect("drawn");
            assert_matches(&pixels, &reference::render(&program, &view), &view);
            let mut faded = straight(source);
            faded[3] *= 0.75;
            let want = w3c::composite(mode, straight(backdrop), faded);
            assert_near(
                at(&pixels, 32.0, 28.0),
                want,
                &format!("{mode:?} where both cover"),
            );
            // Where the source alone covers: it over nothing, Erase erasing nothing.
            let alone = if mode == BlendMode::Erase {
                [0.0; 4]
            } else {
                let a = faded[3];
                [faded[0] * a, faded[1] * a, faded[2] * a, a]
            };
            assert_near(at(&pixels, 50.0, 42.0), alone, &format!("{mode:?} alone"));
        }
    }
    gpu.assert_no_validation_errors();
}

/// The premultiplied result of `source` over `backdrop`, straight colors with alpha.
fn over(backdrop: [f64; 4], source: [f64; 4]) -> [f64; 4] {
    w3c::composite(BlendMode::Normal, backdrop, source)
}

/// A premultiplied color made straight again.
fn unpremultiplied(c: [f64; 4]) -> [f64; 4] {
    if c[3] > 0.0 {
        [c[0] / c[3], c[1] / c[3], c[2] / c[3], c[3]]
    } else {
        [0.0; 4]
    }
}

#[test]
fn a_blend_leaves_what_lies_above_it_alone() {
    let Some(gpu) = gpu_or_skip("a_blend_leaves_what_lies_above_it_alone") else {
        return;
    };
    let mut session = Session::new(&gpu, Extent::new(64, 48));
    let (red, blue, green) = (
        color(0.9, 0.2, 0.1, 1.0),
        color(0.1, 0.3, 0.9, 1.0),
        color(0.2, 0.8, 0.3, 0.5),
    );
    let mut steps = Steps::default();
    let a = steps.draw(&square(4.0, 4.0, 40.0, 40.0), red);
    let b = steps.draw(&square(20.0, 10.0, 60.0, 30.0), blue);
    let c = steps.draw(&square(30.0, 4.0, 50.0, 44.0), green);
    let root = steps.merge(&[
        (a, BlendMode::Normal, 1.0),
        (b, BlendMode::Multiply, 1.0),
        (c, BlendMode::Normal, 1.0),
    ]);
    let program = steps.program(root, CLEAR);
    let view = view();
    let pixels = session.render(&program, &view).expect("drawn");
    assert_matches(&pixels, &reference::render(&program, &view), &view);
    // Above the multiplied pair, the green lies over their product: not multiplied itself.
    let product = w3c::composite(BlendMode::Multiply, straight(red), straight(blue));
    let want = over(unpremultiplied(product), straight(green));
    assert_near(at(&pixels, 35.0, 20.0), want, "green over the product");
    // Over nothing but the red, the green is the green over the red.
    assert_near(
        at(&pixels, 35.0, 36.0),
        over(straight(red), straight(green)),
        "green over red",
    );
    gpu.assert_no_validation_errors();
}

#[test]
fn an_erase_in_a_group_leaves_the_rest_of_the_page() {
    let Some(gpu) = gpu_or_skip("an_erase_in_a_group_leaves_the_rest_of_the_page") else {
        return;
    };
    let mut session = Session::new(&gpu, Extent::new(64, 48));
    let (blue, red) = (color(0.1, 0.3, 0.9, 1.0), color(0.9, 0.2, 0.1, 1.0));
    let mut steps = Steps::default();
    let x = steps.draw(&square(4.0, 4.0, 30.0, 44.0), blue);
    let y = steps.draw(&square(16.0, 8.0, 56.0, 40.0), red);
    let z = steps.draw(&square(20.0, 12.0, 50.0, 36.0), WHITE);
    let group = steps.merge(&[(y, BlendMode::Normal, 1.0), (z, BlendMode::Erase, 1.0)]);
    let root = steps.merge(&[(x, BlendMode::Normal, 1.0), (group, BlendMode::Normal, 1.0)]);
    let program = steps.program(root, WHITE);
    let view = view();
    let pixels = session.render(&program, &view).expect("drawn");
    assert_matches(&pixels, &reference::render(&program, &view), &view);
    // The erased part of the red shows the blue below the group, and the page where no blue
    // is; the red is left around the hole.
    assert_near(
        at(&pixels, 25.0, 24.0),
        [0.1, 0.3, 0.9, 1.0],
        "blue through the hole",
    );
    assert_near(
        at(&pixels, 40.0, 24.0),
        [1.0, 1.0, 1.0, 1.0],
        "the page through the hole",
    );
    assert_near(
        at(&pixels, 53.0, 38.0),
        [0.9, 0.2, 0.1, 1.0],
        "red around the hole",
    );
    // An Erase at the page's root reveals the page's background, not the view's transparency.
    let mut steps = Steps::default();
    let x = steps.draw(&square(4.0, 4.0, 30.0, 44.0), blue);
    let z = steps.draw(&square(20.0, 12.0, 50.0, 36.0), WHITE);
    let root = steps.merge(&[(x, BlendMode::Normal, 1.0), (z, BlendMode::Erase, 1.0)]);
    let program = steps.program(root, color(0.95, 0.9, 0.8, 1.0));
    let pixels = session.render(&program, &view).expect("drawn");
    assert_matches(&pixels, &reference::render(&program, &view), &view);
    assert_near(at(&pixels, 25.0, 24.0), [0.95, 0.9, 0.8, 1.0], "the page");
    gpu.assert_no_validation_errors();
}

#[test]
fn nested_masks_apply_once_each() {
    let Some(gpu) = gpu_or_skip("nested_masks_apply_once_each") else {
        return;
    };
    let mut session = Session::new(&gpu, Extent::new(64, 48));
    let paint = color(0.2, 0.5, 0.9, 0.6);
    let mut steps = Steps::default();
    let content = steps.draw(&square(8.0, 8.0, 56.0, 40.0), paint);
    // Masked by the left half, then by the top half: the top-left quarter shows.
    let left = steps.push(Step::Mask {
        content,
        coverage: StepCoverage::Outline {
            outline: square(0.0, 0.0, 32.0, 48.0),
            rule: FillRule::NonZero,
        },
        invert: false,
    });
    let top = steps.push(Step::Mask {
        content: left,
        coverage: StepCoverage::Outline {
            outline: square(0.0, 0.0, 64.0, 24.0),
            rule: FillRule::NonZero,
        },
        invert: false,
    });
    // A sibling above, unmasked; and the content again under an inverted mask.
    let sibling = steps.draw(&square(40.0, 26.0, 60.0, 46.0), color(0.9, 0.6, 0.1, 1.0));
    let inverted = steps.push(Step::Mask {
        content,
        coverage: StepCoverage::Outline {
            outline: square(0.0, 0.0, 48.0, 48.0),
            rule: FillRule::NonZero,
        },
        invert: true,
    });
    let root = steps.merge(&[
        (top, BlendMode::Normal, 1.0),
        (sibling, BlendMode::Normal, 1.0),
        (inverted, BlendMode::Normal, 1.0),
    ]);
    let program = steps.program(root, CLEAR);
    let view = view();
    let pixels = session.render(&program, &view).expect("drawn");
    assert_matches(&pixels, &reference::render(&program, &view), &view);
    let once = {
        let a = f64::from(paint.a);
        [
            f64::from(paint.r) * a,
            f64::from(paint.g) * a,
            f64::from(paint.b) * a,
            a,
        ]
    };
    // The quarter at the content's own alpha: each mask applied once.
    assert_near(at(&pixels, 20.0, 16.0), once, "the top-left quarter");
    assert_near(at(&pixels, 40.0, 16.0), [0.0; 4], "masked by the left half");
    assert_near(at(&pixels, 20.0, 32.0), [0.0; 4], "masked by the top half");
    // The sibling, whole; the inverted mask shows the content right of 48.
    assert_near(
        at(&pixels, 44.0, 44.0),
        [0.9, 0.6, 0.1, 1.0],
        "the sibling, unmasked",
    );
    assert_near(at(&pixels, 52.0, 16.0), once, "the inverted mask's content");
    gpu.assert_no_validation_errors();
}

#[test]
fn a_blended_program_allocates_nothing_once_its_targets_are_pooled() {
    let Some(gpu) = gpu_or_skip("a_blended_program_allocates_nothing_once_its_targets_are_pooled")
    else {
        return;
    };
    let mut session = Session::new(&gpu, Extent::new(64, 48));
    // Groups in groups, every way of compositing.
    let mut steps = Steps::default();
    let mut layers = Vec::new();
    for (i, mode) in MODES.iter().enumerate() {
        #[allow(clippy::cast_precision_loss, reason = "a small index")]
        let x = 2.0 + 3.0 * i as f64;
        #[allow(clippy::cast_precision_loss, reason = "a small index")]
        let shade = i as f32 / 17.0;
        let draw = steps.draw(
            &square(x, 4.0, x + 12.0, 40.0),
            color(shade, 0.5, 1.0 - shade, 0.8),
        );
        layers.push((draw, *mode, 0.9));
    }
    let inner = steps.merge(&layers);
    let masked = steps.push(Step::Mask {
        content: inner,
        coverage: StepCoverage::Outline {
            outline: square(0.0, 0.0, 50.0, 30.0),
            rule: FillRule::NonZero,
        },
        invert: false,
    });
    let base = steps.draw(&square(0.0, 0.0, 64.0, 48.0), color(0.5, 0.5, 0.5, 1.0));
    let root = steps.merge(&[
        (base, BlendMode::Normal, 1.0),
        (masked, BlendMode::Overlay, 0.8),
    ]);
    let program = steps.program(root, WHITE);
    let pans: Vec<View> = (0..6)
        .map(|i| View {
            center: Point::new(32.0 + f64::from(i) * 0.5, 24.0),
            ..view()
        })
        .collect();
    for pan in &pans {
        let pixels = session.render(&program, pan).expect("drawn");
        assert_matches(&pixels, &reference::render(&program, pan), pan);
    }
    let mut allocated = Vec::new();
    for pan in &pans {
        let (canvas, target) = (&mut session.canvas, &mut session.target);
        let (count, drawn) = gpu
            .device
            .submit_and_wait(|encoder| {
                koebi_testkit::allocations(|| canvas.render(encoder, target, &program, pan))
            })
            .expect("submitted");
        assert!(drawn.expect("rendered"));
        allocated.push(count);
    }
    assert!(
        allocated.iter().all(|count| *count == 0),
        "renders allocated {allocated:?} times"
    );
    gpu.assert_no_validation_errors();
}

#[test]
fn groups_drawn_but_never_composited_are_taken_again_safely() {
    let Some(gpu) = gpu_or_skip("groups_drawn_but_never_composited_are_taken_again_safely") else {
        return;
    };
    let mut session = Session::new(&gpu, Extent::new(64, 48));
    // An Erase first in its merge erases nothing, and a mask whose coverage lies out of sight
    // shows nothing: both groups draw, then go back to the pool unread. Groups taken after them,
    // in this frame and the next, must find their writes done.
    let (blue, red) = (color(0.1, 0.3, 0.9, 1.0), color(0.9, 0.2, 0.1, 0.8));
    let mut steps = Steps::default();
    let eraser = steps.draw(&square(10.0, 10.0, 30.0, 30.0), WHITE);
    let shape = steps.draw(&square(20.0, 20.0, 50.0, 40.0), blue);
    let erasing = steps.merge(&[
        (eraser, BlendMode::Erase, 1.0),
        (shape, BlendMode::Normal, 1.0),
    ]);
    let content = steps.draw(&square(5.0, 5.0, 60.0, 45.0), red);
    let hidden = steps.push(Step::Mask {
        content,
        coverage: StepCoverage::Outline {
            outline: square(1000.0, 1000.0, 1010.0, 1010.0),
            rule: FillRule::NonZero,
        },
        invert: false,
    });
    let below = steps.draw(&square(30.0, 4.0, 60.0, 30.0), red);
    let above = steps.draw(&square(40.0, 10.0, 62.0, 44.0), blue);
    let blended = steps.merge(&[
        (below, BlendMode::Normal, 1.0),
        (above, BlendMode::Screen, 0.7),
    ]);
    let root = steps.merge(&[
        (erasing, BlendMode::Normal, 1.0),
        (hidden, BlendMode::Normal, 1.0),
        (blended, BlendMode::Normal, 1.0),
    ]);
    let program = steps.program(root, CLEAR);
    for x in [32.0, 33.0, 32.0] {
        let view = View {
            center: Point::new(x, 24.0),
            ..view()
        };
        let pixels = session.render(&program, &view).expect("drawn");
        assert_matches(&pixels, &reference::render(&program, &view), &view);
    }
    gpu.assert_no_validation_errors();
}
