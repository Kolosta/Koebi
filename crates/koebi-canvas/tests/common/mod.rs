//! What the canvas's GPU tests share: programs from outlines, shapes, a canvas and a target
//! kept from render to render and read back, and the comparison with the CPU reference.

#![allow(
    dead_code,
    reason = "each test binary uses its own part of these helpers"
)]

use std::sync::Arc;

use koebi_canvas::{Canvas, View, ViewTarget};
use koebi_core::{Extent, LinearRgba};
use koebi_doc::eval::{Draw, MergeInput, Program, Step, StepId};
use koebi_doc::{BlendMode, NodeId};
use koebi_geom::{Contour, Contours, FillRule, Path, Point, Segment, Vector};
use koebi_gpu::{Buffer, BufferDesc, BufferUsage, MemoryLocation, TextureState};
use koebi_testkit::HeadlessGpu;

/// A program drawing `layers`, bottom first, on a transparent page of `size`.
pub(crate) fn program(size: Vector, layers: &[(Contours, FillRule, LinearRgba)]) -> Program {
    let layers: Vec<_> = layers
        .iter()
        .map(|(outline, rule, color)| (Arc::new(outline.clone()), *rule, *color))
        .collect();
    program_of(size, LinearRgba::new(0.0, 0.0, 0.0, 0.0), &layers)
}

/// A program drawing `layers`, bottom first, their outlines shared, on a page of `size` over
/// `background`.
pub(crate) fn program_of(
    size: Vector,
    background: LinearRgba,
    layers: &[(Arc<Contours>, FillRule, LinearRgba)],
) -> Program {
    let mut steps: Vec<Step> = layers
        .iter()
        .map(|(outline, rule, color)| {
            Step::Draw(Draw {
                outline: Arc::clone(outline),
                rule: *rule,
                color: *color,
                opacity: 1.0,
            })
        })
        .collect();
    let inputs = (0..steps.len())
        .map(|index| MergeInput {
            step: StepId(index),
            blend: BlendMode::Normal,
            opacity: 1.0,
        })
        .collect();
    steps.push(Step::Merge(inputs));
    let root = Some(StepId(steps.len() - 1));
    Program::new(NodeId(0), size, background, steps, root)
}

/// The 32-bit float of the half `bits`.
fn from_half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let mantissa = f32::from(bits & 0x3ff);
    sign * match exponent {
        0 => mantissa * (-24.0_f32).exp2(),
        31 => f32::INFINITY,
        _ => {
            (1.0 + mantissa / 1024.0) * f32::from(i16::try_from(exponent).unwrap_or(0) - 15).exp2()
        }
    }
}

/// A canvas and a view's target kept from render to render, as the interface keeps them.
pub(crate) struct Session<'a> {
    pub(crate) gpu: &'a HeadlessGpu,
    pub(crate) canvas: Canvas,
    pub(crate) target: ViewTarget,
    readback: Buffer,
}

impl<'a> Session<'a> {
    /// A canvas and a target of `extent`.
    pub(crate) fn new(gpu: &'a HeadlessGpu, extent: Extent) -> Self {
        Self {
            gpu,
            canvas: Canvas::new(&gpu.device).expect("a canvas"),
            target: ViewTarget::new(&gpu.device, extent).expect("a target"),
            readback: gpu
                .device
                .create_buffer(&BufferDesc {
                    label: "canvas readback",
                    size: extent.area() * 8,
                    usage: BufferUsage::TRANSFER_DST,
                    location: MemoryLocation::GpuToCpu,
                })
                .expect("a readback buffer"),
        }
    }

    /// `program` seen by `view`, drawn and read back: linear light, premultiplied. `None` when
    /// the target showed it already and nothing was drawn.
    pub(crate) fn render(&mut self, program: &Program, view: &View) -> Option<Vec<[f32; 4]>> {
        let (canvas, target, readback) = (&mut self.canvas, &mut self.target, &self.readback);
        let drawn = self
            .gpu
            .device
            .submit_and_wait(|encoder| {
                let drawn = canvas.render(encoder, target, program, view)?;
                if drawn {
                    read_back(encoder, target, readback)?;
                }
                Ok::<_, koebi_gpu::Error>(drawn)
            })
            .expect("submitted")
            .expect("recorded");
        drawn.then(|| self.pixels())
    }

    /// What the last render read back.
    pub(crate) fn pixels(&self) -> Vec<[f32; 4]> {
        pixels_of(&self.readback, self.target.extent())
    }
}

/// The pixels of a target of `extent` read back into `readback`: linear light, premultiplied.
pub(crate) fn pixels_of(readback: &Buffer, extent: Extent) -> Vec<[f32; 4]> {
    let mut halves = vec![0_u16; usize::try_from(extent.area() * 4).expect("a small view")];
    readback.read(0, &mut halves).expect("read back");
    halves
        .as_chunks::<4>()
        .0
        .iter()
        .map(|texel| texel.map(from_half))
        .collect()
}

/// Records the copy of `target` into `readback`, the target left as the canvas left it.
pub(crate) fn read_back(
    encoder: &mut koebi_gpu::CommandEncoder<'_>,
    target: &ViewTarget,
    readback: &Buffer,
) -> koebi_gpu::Result<()> {
    encoder.texture_barrier(
        target.texture(),
        TextureState::ShaderRead,
        TextureState::TransferSrc,
    );
    encoder.copy_texture_to_buffer(target.texture(), readback)?;
    encoder.texture_barrier(
        target.texture(),
        TextureState::TransferSrc,
        TextureState::ShaderRead,
    );
    Ok(())
}

/// `program` seen by `view`, drawn by a new canvas and read back.
pub(crate) fn render(gpu: &HeadlessGpu, program: &Program, view: &View) -> Vec<[f32; 4]> {
    Session::new(gpu, view.extent)
        .render(program, view)
        .expect("a new target is drawn")
}

/// Asserts that the GPU's pixels are the reference's within 1/255, half precision aside.
#[track_caller]
pub(crate) fn assert_matches(gpu: &[[f32; 4]], cpu: &[[f32; 4]], view: &View) {
    let width = view.extent.width as usize;
    let mut worst = (0.0_f32, 0, 0);
    for (index, (a, b)) in gpu.iter().zip(cpu).enumerate() {
        for channel in 0..4 {
            let delta = (a[channel] - b[channel]).abs();
            if delta > worst.0 {
                worst = (delta, index % width, index / width);
            }
        }
    }
    // A half holds 11 bits: 1/2048 of 1 at most beside the reference's 1/255.
    assert!(
        worst.0 <= 1.0 / 255.0 + 1.0 / 2048.0,
        "pixel ({}, {}) off by {} (×255: {:.2})",
        worst.1,
        worst.2,
        worst.0,
        worst.0 * 255.0
    );
}

pub(crate) fn circle(center: Point, radius: f64) -> Contours {
    Path::ellipse(center, Vector::new(radius, radius)).contours()
}

/// A closed polyline through `points`.
pub(crate) fn polyline(points: &[Point]) -> Contours {
    let segments = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(&a, &b)| Segment::line(a, b))
        .collect();
    let mut contours = Contours::default();
    contours.push(Contour::new(segments, true));
    contours
}

/// A five-pointed star drawn in one stroke: its middle winds twice.
pub(crate) fn star(center: Point, radius: f64) -> Contours {
    let points: Vec<Point> = (0..5)
        .map(|k| {
            let angle =
                f64::from(k) * 4.0 * core::f64::consts::PI / 5.0 - core::f64::consts::FRAC_PI_2;
            Point::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            )
        })
        .collect();
    polyline(&points)
}

pub(crate) fn color(r: f32, g: f32, b: f32, a: f32) -> LinearRgba {
    LinearRgba::new(r, g, b, a)
}
