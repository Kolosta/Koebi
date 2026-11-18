//! What the canvas keeps from render to render (ADR 0029): a steady view records nothing and
//! allocates nothing; an edit prepares the outlines it changed and no other; pans and zooms
//! prepare what they reveal; forgotten records moved together draw the same; two views drawn
//! in one frame each get their own. Every image drawn from kept cells matches the CPU
//! reference, which prepares everything anew.

mod common;

use std::sync::Arc;

use common::{Session, assert_matches, circle, color, program_of, read_back};
use koebi_canvas::{Canvas, View, ViewTarget, reference};
use koebi_core::{Extent, LinearRgba};
use koebi_doc::eval::Program;
use koebi_geom::{Contours, FillRule, Path, Point, Rect, Vector};
use koebi_gpu::{BufferDesc, BufferUsage, MemoryLocation};
use koebi_testkit::gpu_or_skip;

#[global_allocator]
static ALLOCATOR: koebi_testkit::CountingAllocator = koebi_testkit::CountingAllocator;

type Layer = (Arc<Contours>, FillRule, LinearRgba);

const CLEAR: LinearRgba = LinearRgba::new(0.0, 0.0, 0.0, 0.0);

fn rectangle(x0: f64, y0: f64, x1: f64, y1: f64) -> Arc<Contours> {
    Arc::new(Path::rectangle(Rect::new(Point::new(x0, y0), Point::new(x1, y1))).contours())
}

/// `count` discs in rows of ten, 12 units apart from (6, 6), in turning colors.
fn discs(count: u32) -> Vec<Layer> {
    (0..count)
        .map(|i| {
            let center = Point::new(
                6.0 + 12.0 * f64::from(i % 10),
                6.0 + 12.0 * f64::from(i / 10),
            );
            #[allow(clippy::cast_precision_loss, reason = "a small index")]
            let hue = (i % 7) as f32 / 7.0;
            (
                Arc::new(circle(center, 4.5)),
                FillRule::NonZero,
                color(hue, 0.5, 1.0 - hue, 0.8),
            )
        })
        .collect()
}

fn view(center: Point, zoom: f64) -> View {
    View {
        center,
        zoom,
        extent: Extent::new(96, 64),
    }
}

/// Draws `program` through `view` in `session`, and checks it against the reference.
#[track_caller]
fn drawn_as_the_reference(session: &mut Session<'_>, program: &Program, view: &View) {
    let pixels = session
        .render(program, view)
        .expect("a new program or view is drawn");
    assert_matches(&pixels, &reference::render(program, view), view);
}

#[test]
fn a_steady_view_records_nothing_and_allocates_nothing() {
    let Some(gpu) = gpu_or_skip("a_steady_view_records_nothing_and_allocates_nothing") else {
        return;
    };
    let mut session = Session::new(&gpu, Extent::new(96, 64));
    let program = program_of(Vector::new(120.0, 80.0), CLEAR, &discs(40));
    let still = view(Point::new(48.0, 32.0), 1.0);
    drawn_as_the_reference(&mut session, &program, &still);
    session.canvas.reset_stats();
    // The same program through the same view: nothing recorded, nothing allocated.
    let (canvas, target) = (&mut session.canvas, &mut session.target);
    let (count, drawn) = gpu
        .device
        .submit_and_wait(|encoder| {
            koebi_testkit::allocations(|| {
                (0..100)
                    .filter(|_| {
                        canvas
                            .render(encoder, target, &program, &still)
                            .expect("rendered")
                    })
                    .count()
            })
        })
        .expect("submitted");
    assert_eq!(drawn, 0);
    assert_eq!(count, 0, "steady renders allocated {count} times");
    assert_eq!(session.canvas.stats().rendered, 0);
    // Pans draw again. The first prepare the discs they reveal; once warm, they prepare
    // nothing and allocate nothing.
    let pans: Vec<View> = (0..12)
        .map(|i| view(Point::new(40.0 + f64::from(i), 30.0), 1.0))
        .collect();
    let mut allocated = Vec::new();
    for pass in 0..3 {
        if pass == 2 {
            session.canvas.reset_stats();
        }
        for pan in &pans {
            let (canvas, target) = (&mut session.canvas, &mut session.target);
            let (count, drawn) = gpu
                .device
                .submit_and_wait(|encoder| {
                    koebi_testkit::allocations(|| canvas.render(encoder, target, &program, pan))
                })
                .expect("submitted");
            assert!(drawn.expect("rendered"));
            if pass == 2 {
                allocated.push(count);
            }
        }
    }
    assert!(
        allocated.iter().all(|count| *count == 0),
        "pans allocated {allocated:?} times"
    );
    assert_eq!(session.canvas.stats().prepared, 0);
    assert_eq!(session.canvas.stats().rendered, 12);
    gpu.assert_no_validation_errors();
}

#[test]
fn an_edit_prepares_only_the_outlines_it_changed() {
    let Some(gpu) = gpu_or_skip("an_edit_prepares_only_the_outlines_it_changed") else {
        return;
    };
    let mut session = Session::new(&gpu, Extent::new(96, 64));
    let still = view(Point::new(60.0, 40.0), 0.8);
    let mut layers = discs(50);
    let first = program_of(Vector::new(120.0, 80.0), CLEAR, &layers);
    drawn_as_the_reference(&mut session, &first, &still);
    assert_eq!(session.canvas.stats().prepared, 50);
    // One disc moved: a new outline, the 49 others the same.
    session.canvas.reset_stats();
    layers[17].0 = Arc::new(circle(Point::new(75.0, 22.0), 6.0));
    let moved = program_of(Vector::new(120.0, 80.0), CLEAR, &layers);
    drawn_as_the_reference(&mut session, &moved, &still);
    let stats = session.canvas.stats();
    assert_eq!(stats.prepared, 1);
    assert!(stats.uploaded < 200, "{} texels uploaded", stats.uploaded);
    // Recolored: the same outlines, nothing prepared or uploaded.
    session.canvas.reset_stats();
    layers[3].2 = color(1.0, 0.0, 0.0, 1.0);
    let recolored = program_of(Vector::new(120.0, 80.0), CLEAR, &layers);
    drawn_as_the_reference(&mut session, &recolored, &still);
    assert_eq!(session.canvas.stats().prepared, 0);
    assert_eq!(session.canvas.stats().uploaded, 0);
    // A page under them all: its background is prepared once, and kept.
    let paged = program_of(Vector::new(120.0, 80.0), color(1.0, 1.0, 0.9, 1.0), &layers);
    drawn_as_the_reference(&mut session, &paged, &still);
    assert_eq!(session.canvas.stats().prepared, 1);
    let paged_again = program_of(Vector::new(120.0, 80.0), color(0.9, 1.0, 1.0, 1.0), &layers);
    drawn_as_the_reference(&mut session, &paged_again, &still);
    assert_eq!(session.canvas.stats().prepared, 1);
    gpu.assert_no_validation_errors();
}

#[test]
fn pans_and_zooms_prepare_what_they_reveal() {
    let Some(gpu) = gpu_or_skip("pans_and_zooms_prepare_what_they_reveal") else {
        return;
    };
    let mut session = Session::new(&gpu, Extent::new(96, 64));
    // A band 10 000 units wide under 20 discs.
    let mut layers = vec![(
        rectangle(-5000.0, 20.0, 5000.0, 30.0),
        FillRule::NonZero,
        color(0.2, 0.6, 0.3, 1.0),
    )];
    layers.extend(discs(20));
    let program = program_of(Vector::new(120.0, 80.0), CLEAR, &layers);
    let prepared = |session: &mut Session<'_>| {
        let prepared = session.canvas.stats().prepared;
        session.canvas.reset_stats();
        prepared
    };
    // At 0.8, the view spans 120 units: every disc in sight.
    let near = Point::new(60.0, 20.0);
    drawn_as_the_reference(&mut session, &program, &view(near, 0.8));
    assert_eq!(prepared(&mut session), 21);
    // Within the area prepared around the view: nothing again.
    drawn_as_the_reference(&mut session, &program, &view(Point::new(90.0, 25.0), 0.8));
    assert_eq!(prepared(&mut session), 0);
    // Far along the band: the band again, the discs out of sight left alone; back, the band
    // again.
    drawn_as_the_reference(&mut session, &program, &view(Point::new(3000.0, 25.0), 0.8));
    assert_eq!(prepared(&mut session), 1);
    drawn_as_the_reference(&mut session, &program, &view(near, 0.8));
    assert_eq!(prepared(&mut session), 1);
    // A zoom within the octave keeps the cells; across it, what is in sight is prepared at the
    // new octave; back, the old octave's cells are there still.
    drawn_as_the_reference(&mut session, &program, &view(near, 0.95));
    assert_eq!(prepared(&mut session), 0);
    drawn_as_the_reference(&mut session, &program, &view(near, 1.3));
    let in_sight = prepared(&mut session);
    assert!((1..21).contains(&in_sight), "{in_sight}");
    drawn_as_the_reference(&mut session, &program, &view(near, 0.6));
    assert_eq!(prepared(&mut session), 0);
    drawn_as_the_reference(&mut session, &program, &view(near, 1.9));
    assert_eq!(prepared(&mut session), 0);
    gpu.assert_no_validation_errors();
}

#[test]
fn forgotten_records_moved_together_draw_the_same() {
    let Some(gpu) = gpu_or_skip("forgotten_records_moved_together_draw_the_same") else {
        return;
    };
    let mut session = Session::new(&gpu, Extent::new(96, 64));
    // 6000 small squares, then a program without them drawn until they are forgotten.
    let squares: Vec<Layer> = (0..6000_u32)
        .map(|i| {
            let (x, y) = (f64::from(i % 100) * 1.2, f64::from(i / 100) * 1.2);
            (
                rectangle(x, y, x + 0.8, y + 0.8),
                FillRule::NonZero,
                color(0.3, 0.3, 0.8, 1.0),
            )
        })
        .collect();
    let crowded = program_of(Vector::new(120.0, 80.0), CLEAR, &squares);
    let whole = view(Point::new(60.0, 36.0), 0.75);
    drawn_as_the_reference(&mut session, &crowded, &whole);
    let sparse = program_of(
        Vector::new(120.0, 80.0),
        CLEAR,
        &[(
            Arc::new(circle(Point::new(60.0, 36.0), 20.0)),
            FillRule::NonZero,
            color(0.9, 0.4, 0.1, 1.0),
        )],
    );
    let views = [whole, view(Point::new(61.0, 36.0), 0.75)];
    let mut renders = 0;
    while session.canvas.stats().moves == 0 {
        session
            .render(&sparse, &views[renders % 2])
            .expect("another view is drawn");
        renders += 1;
        assert!(renders <= 200, "records were never moved");
    }
    drawn_as_the_reference(&mut session, &sparse, &views[renders % 2]);
    // The squares were forgotten: prepared anew, and drawn as the reference.
    session.canvas.reset_stats();
    drawn_as_the_reference(&mut session, &crowded, &whole);
    assert_eq!(session.canvas.stats().prepared, 6000);
    gpu.assert_no_validation_errors();
}

#[test]
fn two_views_drawn_in_one_frame_each_show_their_own() {
    let Some(gpu) = gpu_or_skip("two_views_drawn_in_one_frame_each_show_their_own") else {
        return;
    };
    let mut canvas = Canvas::new(&gpu.device).expect("a canvas");
    let first = program_of(Vector::new(120.0, 80.0), CLEAR, &discs(30));
    let mut other = discs(30);
    other.truncate(10);
    other.push((
        rectangle(10.0, 10.0, 70.0, 50.0),
        FillRule::EvenOdd,
        color(0.1, 0.2, 0.9, 0.5),
    ));
    let second = program_of(Vector::new(120.0, 80.0), color(1.0, 1.0, 1.0, 1.0), &other);
    let views = [
        view(Point::new(50.0, 30.0), 1.0),
        View {
            center: Point::new(40.0, 30.0),
            zoom: 1.7,
            extent: Extent::new(70, 50),
        },
    ];
    let mut targets: Vec<ViewTarget> = views
        .iter()
        .map(|view| ViewTarget::new(&gpu.device, view.extent).expect("a target"))
        .collect();
    let readbacks: Vec<_> = views
        .iter()
        .map(|view| {
            gpu.device
                .create_buffer(&BufferDesc {
                    label: "canvas readback",
                    size: view.extent.area() * 8,
                    usage: BufferUsage::TRANSFER_DST,
                    location: MemoryLocation::GpuToCpu,
                })
                .expect("a readback buffer")
        })
        .collect();
    let programs = [&first, &second];
    gpu.device
        .submit_and_wait(|encoder| {
            for ((target, readback), (program, view)) in targets
                .iter_mut()
                .zip(&readbacks)
                .zip(programs.iter().zip(&views))
            {
                assert!(canvas.render(encoder, target, program, view)?);
                read_back(encoder, target, readback)?;
            }
            Ok::<_, koebi_gpu::Error>(())
        })
        .expect("submitted")
        .expect("recorded");
    for ((readback, program), view) in readbacks.iter().zip(programs).zip(&views) {
        let pixels = common::pixels_of(readback, view.extent);
        assert_matches(&pixels, &reference::render(program, view), view);
    }
    gpu.assert_no_validation_errors();
}
