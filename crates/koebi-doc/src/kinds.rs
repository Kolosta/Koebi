//! The first node kinds (ADR 0028): document, page, objects, geometry, paint, composition,
//! values and inspection. Each module holds a kind's marker type, its typed socket handles and
//! its definition, checked at compile time.
//!
//! Lengths are in document units (millimetres); angles in radians; shares from 0 to 1.

#![allow(
    clippy::wildcard_imports,
    reason = "each kind's module declares its sockets from the same vocabulary, its parent's"
)]

use crate::kind::{Family, In, Initial, Kind, NodeKind, Out};
use crate::value::{
    Anything, BlendMode, Coverage, EnumKind, Geometry, Layer, Paint, PatternAnchor, PatternClip,
    Quantity, StrokeAlign, StrokeCap, StrokeJoin, StrokeUnits,
};
use koebi_core::LinearRgba;
use koebi_geom::{Affine, FillRule, Vector};

/// Every kind of this module, for [`crate::Registry::standard`].
pub const ALL: &[&NodeKind] = &[
    document::KIND,
    page::KIND,
    path_object::KIND,
    group_object::KIND,
    instance_object::KIND,
    path_data::KIND,
    rectangle::KIND,
    ellipse::KIND,
    polygon::KIND,
    transform_geometry::KIND,
    fill::KIND,
    pattern_fill::KIND,
    stroke::KIND,
    merge::KIND,
    blend::KIND,
    mask::KIND,
    coverage::KIND,
    layer_coverage::KIND,
    color::KIND,
    float::KIND,
    vector::KIND,
    transform::KIND,
    preview::KIND,
];

// Enumerations start at their first variant: Normal, non-zero, centered, butt, miter.
const WHITE: Initial = Initial::Color([1.0, 1.0, 1.0, 1.0]);
const BLACK: Initial = Initial::Color([0.0, 0.0, 0.0, 1.0]);

/// Declares a kind's marker type and its [`Kind`] implementation, and checks its definition at
/// compile time.
macro_rules! kind {
    ($marker:ident) => {
        #[doc = concat!("The marker type of the kind [`KIND`].")]
        #[derive(Clone, Copy, Debug)]
        pub struct $marker;

        impl Kind for $marker {
            const KIND: &'static NodeKind = KIND;
        }

        const _: () = KIND.check();
    };
}

/// The object sockets every object kind shares, at the same indexes after its own.
macro_rules! placement {
    ($marker:ident, $first:expr) => {
        /// Location, in document units.
        pub const LOCATION: In<$marker, Vector> = In::new($first);
        /// Rotation, in radians.
        pub const ROTATION: In<$marker, f64> = In::new($first + 1);
        /// Scale, its own: a parent's never applies to it.
        pub const SCALE: In<$marker, Vector> = In::new($first + 2);
        /// The space of its parent: the parent's world transform, linked by parenting.
        pub const PARENT: In<$marker, Affine> = In::new($first + 3);
        /// Opacity, from 0 to 1.
        pub const OPACITY: In<$marker, f64> = In::new($first + 4);
        /// Whether it shows.
        pub const VISIBLE: In<$marker, bool> = In::new($first + 5);
    };
}

macro_rules! placement_decls {
    () => {
        [
            LOCATION
                .decl("Location")
                .quantity(Quantity::Length)
                .initial(Initial::Vector(0.0, 0.0)),
            ROTATION
                .decl("Rotation")
                .quantity(Quantity::Angle)
                .initial(Initial::Float(0.0)),
            SCALE.decl("Scale").initial(Initial::Vector(1.0, 1.0)),
            PARENT.decl("Parent").initial(Initial::Identity),
            OPACITY
                .decl("Opacity")
                .quantity(Quantity::Percent)
                .initial(Initial::Float(1.0)),
            VISIBLE.decl("Visible").initial(Initial::Bool(true)),
        ]
    };
}

/// The document's output: its pages, in order.
pub mod document {
    use super::*;
    kind!(Document);
    /// The pages, in order.
    pub const PAGES: In<Document, Layer> = In::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.document.output",
        name: "Document",
        family: Family::Document,
        inputs: &[PAGES.decl("Pages").multi()],
        outputs: &[],
    };
}

/// A page: a size, a background, and its content.
pub mod page {
    use super::*;
    kind!(Page);
    /// Its size: A4 landscape when made.
    pub const SIZE: In<Page, Vector> = In::new(0);
    /// Its background color.
    pub const BACKGROUND: In<Page, LinearRgba> = In::new(1);
    /// Its content: its stack, a Merge.
    pub const CONTENT: In<Page, Layer> = In::new(2);
    /// The page, for the document.
    pub const PAGE: Out<Page, Layer> = Out::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.page",
        name: "Page",
        family: Family::Page,
        inputs: &[
            SIZE.decl("Size")
                .quantity(Quantity::Length)
                .initial(Initial::Vector(297.0, 210.0))
                .pass_through(),
            BACKGROUND.decl("Background").initial(WHITE).pass_through(),
            CONTENT.decl("Content"),
        ],
        outputs: &[PAGE.decl("Page")],
    };
}

/// A path object: geometry, a paint stack, and its placement.
pub mod path_object {
    use super::*;
    kind!(PathObject);
    /// Its geometry: path data, then its modifiers.
    pub const GEOMETRY: In<PathObject, Geometry> = In::new(0);
    /// Its fills and strokes, in paint order.
    pub const PAINTS: In<PathObject, Paint> = In::new(1);
    placement!(PathObject, 2);
    /// Its layer, placed in the world.
    pub const LAYER: Out<PathObject, Layer> = Out::new(0);
    /// Its layer in its own space, for instances.
    pub const LOCAL_LAYER: Out<PathObject, Layer> = Out::new(1);
    /// Its geometry, placed in the world.
    pub const WORLD_GEOMETRY: Out<PathObject, Geometry> = Out::new(2);
    /// Its geometry in its own space.
    pub const LOCAL_GEOMETRY: Out<PathObject, Geometry> = Out::new(3);
    /// Its world transform: its children's space.
    pub const WORLD: Out<PathObject, Affine> = Out::new(4);
    const PLACEMENT: [crate::kind::InputDecl; 6] = placement_decls!();
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.object.path",
        name: "Path object",
        family: Family::Object,
        inputs: &[
            GEOMETRY.decl("Geometry"),
            PAINTS.decl("Paints").multi(),
            PLACEMENT[0],
            PLACEMENT[1],
            PLACEMENT[2],
            PLACEMENT[3],
            PLACEMENT[4],
            PLACEMENT[5],
        ],
        outputs: &[
            LAYER.decl("Layer"),
            LOCAL_LAYER.decl("Local layer"),
            WORLD_GEOMETRY.decl("World geometry"),
            LOCAL_GEOMETRY.decl("Local geometry"),
            WORLD.decl("World transform"),
        ],
    };
}

/// A group object: no geometry, no paint; the objects it holds, composited together.
pub mod group_object {
    use super::*;
    kind!(GroupObject);
    /// Its children's stack, a Merge.
    pub const CONTENT: In<GroupObject, Layer> = In::new(0);
    placement!(GroupObject, 1);
    /// Its layer, placed in the world.
    pub const LAYER: Out<GroupObject, Layer> = Out::new(0);
    /// Its world transform: the space of its children without a parent of their own.
    pub const WORLD: Out<GroupObject, Affine> = Out::new(1);
    const PLACEMENT: [crate::kind::InputDecl; 6] = placement_decls!();
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.object.group",
        name: "Group object",
        family: Family::Object,
        inputs: &[
            CONTENT.decl("Content"),
            PLACEMENT[0],
            PLACEMENT[1],
            PLACEMENT[2],
            PLACEMENT[3],
            PLACEMENT[4],
            PLACEMENT[5],
        ],
        outputs: &[LAYER.decl("Layer"), WORLD.decl("World transform")],
    };
}

/// An instance object: another object's local layer, placed anew (a linked duplicate).
pub mod instance_object {
    use super::*;
    kind!(InstanceObject);
    /// The source: another object's local layer.
    pub const SOURCE: In<InstanceObject, Layer> = In::new(0);
    placement!(InstanceObject, 1);
    /// Its layer, placed in the world.
    pub const LAYER: Out<InstanceObject, Layer> = Out::new(0);
    /// Its world transform.
    pub const WORLD: Out<InstanceObject, Affine> = Out::new(1);
    const PLACEMENT: [crate::kind::InputDecl; 6] = placement_decls!();
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.object.instance",
        name: "Instance object",
        family: Family::Object,
        inputs: &[
            SOURCE.decl("Source"),
            PLACEMENT[0],
            PLACEMENT[1],
            PLACEMENT[2],
            PLACEMENT[3],
            PLACEMENT[4],
            PLACEMENT[5],
        ],
        outputs: &[LAYER.decl("Layer"), WORLD.decl("World transform")],
    };
}

/// Path data: the subpaths the user draws, held in the node's settings.
pub mod path_data {
    use super::*;
    kind!(PathData);
    /// The path.
    pub const GEOMETRY: Out<PathData, Geometry> = Out::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.geometry.path",
        name: "Path data",
        family: Family::Geometry,
        inputs: &[],
        outputs: &[GEOMETRY.decl("Geometry")],
    };
}

/// A rectangle centered on the origin of its space.
pub mod rectangle {
    use super::*;
    kind!(Rectangle);
    /// Its size.
    pub const SIZE: In<Rectangle, Vector> = In::new(0);
    /// The radius of its corners.
    pub const CORNER_RADIUS: In<Rectangle, f64> = In::new(1);
    /// The rectangle.
    pub const GEOMETRY: Out<Rectangle, Geometry> = Out::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.geometry.rectangle",
        name: "Rectangle",
        family: Family::Geometry,
        inputs: &[
            SIZE.decl("Size")
                .quantity(Quantity::Length)
                .initial(Initial::Vector(100.0, 100.0))
                .pass_through(),
            CORNER_RADIUS
                .decl("Corner radius")
                .quantity(Quantity::Length)
                .initial(Initial::Float(0.0))
                .pass_through(),
        ],
        outputs: &[GEOMETRY.decl("Geometry")],
    };
}

/// An ellipse centered on the origin of its space.
pub mod ellipse {
    use super::*;
    kind!(Ellipse);
    /// Its radii.
    pub const RADII: In<Ellipse, Vector> = In::new(0);
    /// Whether it is the exact (NURBS) ellipse rather than the editable one.
    pub const EXACT: In<Ellipse, bool> = In::new(1);
    /// The ellipse.
    pub const GEOMETRY: Out<Ellipse, Geometry> = Out::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.geometry.ellipse",
        name: "Ellipse",
        family: Family::Geometry,
        inputs: &[
            RADII
                .decl("Radii")
                .quantity(Quantity::Length)
                .initial(Initial::Vector(50.0, 50.0))
                .pass_through(),
            EXACT.decl("Exact").initial(Initial::Bool(false)),
        ],
        outputs: &[GEOMETRY.decl("Geometry")],
    };
}

/// A regular polygon centered on the origin of its space.
pub mod polygon {
    use super::*;
    kind!(Polygon);
    /// The distance from its center to its corners.
    pub const RADIUS: In<Polygon, f64> = In::new(0);
    /// Its number of sides, 3 or more.
    pub const SIDES: In<Polygon, i64> = In::new(1);
    /// The angle of its first corner.
    pub const ROTATION: In<Polygon, f64> = In::new(2);
    /// The polygon.
    pub const GEOMETRY: Out<Polygon, Geometry> = Out::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.geometry.polygon",
        name: "Polygon",
        family: Family::Geometry,
        inputs: &[
            RADIUS
                .decl("Radius")
                .quantity(Quantity::Length)
                .initial(Initial::Float(50.0))
                .pass_through(),
            SIDES.decl("Sides").initial(Initial::Int(6)).pass_through(),
            ROTATION
                .decl("Rotation")
                .quantity(Quantity::Angle)
                .initial(Initial::Float(0.0)),
        ],
        outputs: &[GEOMETRY.decl("Geometry")],
    };
}

/// Geometry moved by a transform.
pub mod transform_geometry {
    use super::*;
    kind!(TransformGeometry);
    /// The geometry.
    pub const GEOMETRY: In<TransformGeometry, Geometry> = In::new(0);
    /// The transform.
    pub const TRANSFORM: In<TransformGeometry, Affine> = In::new(1);
    /// The moved geometry.
    pub const RESULT: Out<TransformGeometry, Geometry> = Out::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.geometry.transform",
        name: "Transform geometry",
        family: Family::Geometry,
        inputs: &[
            GEOMETRY.decl("Geometry"),
            TRANSFORM.decl("Transform").initial(Initial::Identity),
        ],
        outputs: &[RESULT.decl("Geometry")],
    };
}

/// A solid fill.
pub mod fill {
    use super::*;
    kind!(Fill);
    /// Its color.
    pub const COLOR: In<Fill, LinearRgba> = In::new(0);
    /// Its opacity.
    pub const OPACITY: In<Fill, f64> = In::new(1);
    /// Its fill rule.
    pub const RULE: In<Fill, FillRule> = In::new(2);
    /// How it blends onto the paints below it.
    pub const BLEND: In<Fill, BlendMode> = In::new(3);
    /// The paint.
    pub const PAINT: Out<Fill, Paint> = Out::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.paint.fill",
        name: "Fill",
        family: Family::Paint,
        inputs: &[
            COLOR.decl("Color").initial(BLACK).pass_through(),
            OPACITY
                .decl("Opacity")
                .quantity(Quantity::Percent)
                .initial(Initial::Float(1.0))
                .pass_through(),
            RULE.decl("Fill rule")
                .initial(Initial::Enum(EnumKind::FillRule, 0)),
            BLEND
                .decl("Blend mode")
                .initial(Initial::Enum(EnumKind::BlendMode, 0)),
        ],
        outputs: &[PAINT.decl("Paint")],
    };
}

/// A fill of a motif repeated on a lattice: the layer linked to Motif (an object's own layer)
/// drawn at every point of the lattice over the shape, cut at its bounds, its outline or its
/// widest stroke's edges.
pub mod pattern_fill {
    use super::*;
    kind!(PatternFill);
    /// The motif: a layer in its own space, an object's own layer by link.
    pub const MOTIF: In<PatternFill, Layer> = In::new(0);
    /// The lattice's pitch along its two axes.
    pub const SPACING: In<PatternFill, Vector> = In::new(1);
    /// Where the lattice's origin lies.
    pub const PHASE: In<PatternFill, Vector> = In::new(2);
    /// The lattice's rotation.
    pub const ROTATION: In<PatternFill, f64> = In::new(3);
    /// Each motif's own rotation, on top of the lattice's.
    pub const MOTIF_ROTATION: In<PatternFill, f64> = In::new(4);
    /// Each motif's scale.
    pub const SCALE: In<PatternFill, f64> = In::new(5);
    /// Where the pattern is cut.
    pub const CLIP: In<PatternFill, PatternClip> = In::new(6);
    /// What the lattice is pinned to.
    pub const ANCHOR: In<PatternFill, PatternAnchor> = In::new(7);
    /// Its opacity.
    pub const OPACITY: In<PatternFill, f64> = In::new(8);
    /// How it blends onto the paints below it.
    pub const BLEND: In<PatternFill, BlendMode> = In::new(9);
    /// The paint.
    pub const PAINT: Out<PatternFill, Paint> = Out::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.paint.pattern_fill",
        name: "Pattern Fill",
        family: Family::Paint,
        inputs: &[
            MOTIF.decl("Motif"),
            SPACING
                .decl("Spacing")
                .quantity(Quantity::Length)
                .initial(Initial::Vector(40.0, 40.0)),
            PHASE
                .decl("Phase")
                .quantity(Quantity::Length)
                .initial(Initial::Vector(0.0, 0.0)),
            ROTATION
                .decl("Rotation")
                .quantity(Quantity::Angle)
                .initial(Initial::Float(0.0)),
            MOTIF_ROTATION
                .decl("Motif rotation")
                .quantity(Quantity::Angle)
                .initial(Initial::Float(0.0)),
            SCALE.decl("Scale").initial(Initial::Float(1.0)),
            CLIP.decl("Clip")
                .initial(Initial::Enum(EnumKind::PatternClip, 0)),
            ANCHOR
                .decl("Anchor")
                .initial(Initial::Enum(EnumKind::PatternAnchor, 0)),
            OPACITY
                .decl("Opacity")
                .quantity(Quantity::Percent)
                .initial(Initial::Float(1.0)),
            BLEND
                .decl("Blend mode")
                .initial(Initial::Enum(EnumKind::BlendMode, 0)),
        ],
        outputs: &[PAINT.decl("Paint")],
    };
}

/// A stroke; its dashes, marks and repeats are the node's settings.
pub mod stroke {
    use super::*;
    kind!(Stroke);
    /// Its color.
    pub const COLOR: In<Stroke, LinearRgba> = In::new(0);
    /// Its opacity.
    pub const OPACITY: In<Stroke, f64> = In::new(1);
    /// Its width.
    pub const WIDTH: In<Stroke, f64> = In::new(2);
    /// The side of the path it lies on.
    pub const ALIGN: In<Stroke, StrokeAlign> = In::new(3);
    /// How far from the path an aligned stroke's middle lies, as a share of its width.
    pub const OFFSET: In<Stroke, f64> = In::new(4);
    /// Its ends.
    pub const CAP: In<Stroke, StrokeCap> = In::new(5);
    /// The tilt of a butt cap.
    pub const CAP_TILT: In<Stroke, f64> = In::new(6);
    /// The length of a taper (0: twice the width).
    pub const TAPER: In<Stroke, f64> = In::new(7);
    /// Its corners.
    pub const JOIN: In<Stroke, StrokeJoin> = In::new(8);
    /// The longest miter, as a multiple of the width.
    pub const MITER_LIMIT: In<Stroke, f64> = In::new(9);
    /// How it blends onto the paints below it.
    pub const BLEND: In<Stroke, BlendMode> = In::new(10);
    /// What its lengths count: document units, or the pixels of the view showing it.
    pub const UNITS: In<Stroke, StrokeUnits> = In::new(11);
    /// The paint.
    pub const PAINT: Out<Stroke, Paint> = Out::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.paint.stroke",
        name: "Stroke",
        family: Family::Paint,
        inputs: &[
            COLOR.decl("Color").initial(BLACK).pass_through(),
            OPACITY
                .decl("Opacity")
                .quantity(Quantity::Percent)
                .initial(Initial::Float(1.0))
                .pass_through(),
            WIDTH
                .decl("Width")
                .quantity(Quantity::Length)
                .initial(Initial::Float(1.0))
                .pass_through(),
            ALIGN
                .decl("Align")
                .initial(Initial::Enum(EnumKind::StrokeAlign, 0)),
            OFFSET
                .decl("Offset")
                .quantity(Quantity::Percent)
                .initial(Initial::Float(0.5)),
            CAP.decl("Cap")
                .initial(Initial::Enum(EnumKind::StrokeCap, 0)),
            CAP_TILT
                .decl("Cap tilt")
                .quantity(Quantity::Angle)
                .initial(Initial::Float(0.0)),
            TAPER
                .decl("Taper length")
                .quantity(Quantity::Length)
                .initial(Initial::Float(0.0)),
            JOIN.decl("Join")
                .initial(Initial::Enum(EnumKind::StrokeJoin, 0)),
            MITER_LIMIT.decl("Miter limit").initial(Initial::Float(4.0)),
            BLEND
                .decl("Blend mode")
                .initial(Initial::Enum(EnumKind::BlendMode, 0)),
            UNITS
                .decl("Units")
                .initial(Initial::Enum(EnumKind::StrokeUnits, 0)),
        ],
        outputs: &[PAINT.decl("Paint")],
    };
}

/// A stack: layers composited in order, bottom first, each entry with its blend mode, opacity
/// and visibility (the node's settings, by link).
pub mod merge {
    use super::*;
    kind!(Merge);
    /// The layers, bottom first.
    pub const LAYERS: In<Merge, Layer> = In::new(0);
    /// The composited layer.
    pub const LAYER: Out<Merge, Layer> = Out::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.composition.merge",
        name: "Merge",
        family: Family::Composition,
        inputs: &[LAYERS.decl("Layers").multi()],
        outputs: &[LAYER.decl("Layer")],
    };
}

/// A source composited onto a backdrop by a blend mode.
pub mod blend {
    use super::*;
    kind!(Blend);
    /// What lies below.
    pub const BACKDROP: In<Blend, Layer> = In::new(0);
    /// What is blended onto it.
    pub const SOURCE: In<Blend, Layer> = In::new(1);
    /// How.
    pub const MODE: In<Blend, BlendMode> = In::new(2);
    /// The source's opacity.
    pub const OPACITY: In<Blend, f64> = In::new(3);
    /// Whether the source is composited on its own first.
    pub const ISOLATE: In<Blend, bool> = In::new(4);
    /// The result.
    pub const LAYER: Out<Blend, Layer> = Out::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.composition.blend",
        name: "Blend",
        family: Family::Composition,
        inputs: &[
            BACKDROP.decl("Backdrop"),
            SOURCE.decl("Source"),
            MODE.decl("Mode")
                .initial(Initial::Enum(EnumKind::BlendMode, 0)),
            OPACITY
                .decl("Opacity")
                .quantity(Quantity::Percent)
                .initial(Initial::Float(1.0)),
            ISOLATE.decl("Isolate").initial(Initial::Bool(false)),
        ],
        outputs: &[LAYER.decl("Layer")],
    };
}

/// Content restricted to a coverage.
pub mod mask {
    use super::*;
    kind!(Mask);
    /// The content.
    pub const CONTENT: In<Mask, Layer> = In::new(0);
    /// Where it shows.
    pub const COVERAGE: In<Mask, Coverage> = In::new(1);
    /// Whether it shows outside the coverage instead.
    pub const INVERT: In<Mask, bool> = In::new(2);
    /// The masked content.
    pub const LAYER: Out<Mask, Layer> = Out::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.composition.mask",
        name: "Mask",
        family: Family::Composition,
        inputs: &[
            CONTENT.decl("Content"),
            COVERAGE.decl("Coverage"),
            INVERT.decl("Invert").initial(Initial::Bool(false)),
        ],
        outputs: &[LAYER.decl("Layer")],
    };
}

/// The coverage of a geometry: where it is filled.
pub mod coverage {
    use super::*;
    kind!(GeometryCoverage);
    /// The geometry.
    pub const GEOMETRY: In<GeometryCoverage, Geometry> = In::new(0);
    /// Its fill rule.
    pub const RULE: In<GeometryCoverage, FillRule> = In::new(1);
    /// The coverage.
    pub const COVERAGE: Out<GeometryCoverage, Coverage> = Out::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.composition.coverage",
        name: "Coverage",
        family: Family::Composition,
        inputs: &[
            GEOMETRY.decl("Geometry"),
            RULE.decl("Fill rule")
                .initial(Initial::Enum(EnumKind::FillRule, 0)),
        ],
        outputs: &[COVERAGE.decl("Coverage")],
    };
}

/// The coverage of a layer: its opacity.
pub mod layer_coverage {
    use super::*;
    kind!(LayerCoverage);
    /// The layer.
    pub const LAYER: In<LayerCoverage, Layer> = In::new(0);
    /// The coverage.
    pub const COVERAGE: Out<LayerCoverage, Coverage> = Out::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.composition.layer_coverage",
        name: "Layer coverage",
        family: Family::Composition,
        inputs: &[LAYER.decl("Layer")],
        outputs: &[COVERAGE.decl("Coverage")],
    };
}

/// A color: its input, passed through.
pub mod color {
    use super::*;
    kind!(Color);
    /// The color, and its output.
    pub const COLOR: In<Color, LinearRgba> = In::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.value.color",
        name: "Color",
        family: Family::Value,
        inputs: &[COLOR.decl("Color").initial(WHITE).pass_through()],
        outputs: &[],
    };
}

/// A number: its input, passed through.
pub mod float {
    use super::*;
    kind!(Float);
    /// The number, and its output.
    pub const VALUE: In<Float, f64> = In::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.value.float",
        name: "Float",
        family: Family::Value,
        inputs: &[VALUE
            .decl("Value")
            .initial(Initial::Float(0.0))
            .pass_through()],
        outputs: &[],
    };
}

/// A vector: its input, passed through.
pub mod vector {
    use super::*;
    kind!(VectorValue);
    /// The vector, and its output.
    pub const VALUE: In<VectorValue, Vector> = In::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.value.vector",
        name: "Vector",
        family: Family::Value,
        inputs: &[VALUE
            .decl("Vector")
            .initial(Initial::Vector(0.0, 0.0))
            .pass_through()],
        outputs: &[],
    };
}

/// A transform from a location, a rotation and a scale.
pub mod transform {
    use super::*;
    kind!(TransformValue);
    /// Location.
    pub const LOCATION: In<TransformValue, Vector> = In::new(0);
    /// Rotation.
    pub const ROTATION: In<TransformValue, f64> = In::new(1);
    /// Scale.
    pub const SCALE: In<TransformValue, Vector> = In::new(2);
    /// The transform.
    pub const TRANSFORM: Out<TransformValue, Affine> = Out::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.value.transform",
        name: "Transform",
        family: Family::Value,
        inputs: &[
            LOCATION
                .decl("Location")
                .quantity(Quantity::Length)
                .initial(Initial::Vector(0.0, 0.0)),
            ROTATION
                .decl("Rotation")
                .quantity(Quantity::Angle)
                .initial(Initial::Float(0.0)),
            SCALE.decl("Scale").initial(Initial::Vector(1.0, 1.0)),
        ],
        outputs: &[TRANSFORM.decl("Transform")],
    };
}

/// Shows any socket's value in the Node Graph.
pub mod preview {
    use super::*;
    kind!(Preview);
    /// What it shows.
    pub const VALUE: In<Preview, Anything> = In::new(0);
    /// Its definition.
    pub const KIND: &NodeKind = &NodeKind {
        id: "koebi.inspection.preview",
        name: "Preview",
        family: Family::Inspection,
        inputs: &[VALUE.decl("Value")],
        outputs: &[],
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn ids_are_unique_and_namespaced() {
        let ids: BTreeSet<&str> = ALL.iter().map(|kind| kind.id).collect();
        assert_eq!(ids.len(), ALL.len());
        assert!(ids.iter().all(|id| id.starts_with("koebi.")));
    }

    #[test]
    fn pass_through_outputs_follow_the_declared_ones() {
        let kind = fill::KIND;
        assert_eq!(kind.output_count(), 3);
        assert_eq!(kind.output(1), Some(("Color", crate::SocketType::Color)));
        assert_eq!(kind.pass_through_of(fill::OPACITY.index()), Some(2));
        assert_eq!(kind.pass_through_of(fill::RULE.index()), None);
        // A value node's output is its input passed through.
        assert_eq!(color::KIND.output_count(), 1);
        assert_eq!(color::KIND.output_type(0), Some(crate::SocketType::Color));
    }
}
