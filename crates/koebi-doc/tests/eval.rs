//! Evaluation into per-page programs: canonical documents (stacks, groups, blends, masks), a
//! customized one (a shape masking an unrelated object through a transform), invalid links,
//! visibility, and what an edit re-evaluates.

use std::sync::Arc;

use koebi_core::LinearRgba;
use koebi_doc::eval::{Draw, Evaluator, Program, Step, StepCoverage, StepId, ViewStroke};
use koebi_doc::kinds::{
    blend, coverage, fill, float, group_object, mask, merge, path_object, stroke,
    transform_geometry,
};
use koebi_doc::ops::{self, Shape};
use koebi_doc::{BlendMode, Document, Edit, NodeId, SocketRef, SocketValue, StrokeUnits, Value};
use koebi_geom::{Affine, Vector};

fn set(doc: &mut Document, node: NodeId, index: usize, value: Value) {
    doc.apply(Edit::SetInput { node, index, value })
        .expect("a value of its type");
}

fn at(node: NodeId, index: usize) -> SocketRef {
    SocketRef::new(node, index)
}

fn draws(program: &Program) -> Vec<&Draw> {
    program
        .steps()
        .iter()
        .filter_map(|step| match step {
            Step::Draw(draw) => Some(draw),
            _ => None,
        })
        .collect()
}

/// The bounds of one contour: its segments'.
fn contour_bounds(contour: &koebi_geom::Contour) -> koebi_geom::Rect {
    contour
        .segments()
        .iter()
        .fold(koebi_geom::Rect::EMPTY, |bounds, segment| {
            bounds.union(segment.bounds())
        })
}

/// The program of the first page of `doc`, from a new evaluator.
fn first_program(doc: &Document) -> Arc<Program> {
    Arc::clone(&Evaluator::new().programs(doc)[0])
}

fn root(program: &Program) -> &Step {
    &program.steps()[program.root().expect("the page draws something").0]
}

fn page_with(doc: &mut Document, shapes: &[Shape]) -> (NodeId, Vec<NodeId>) {
    let page = ops::add_page(doc).expect("a page");
    let objects = shapes
        .iter()
        .map(|shape| ops::add_shape(doc, page.stack, shape.clone()).expect("a shape"))
        .collect();
    (page.stack, objects)
}

#[test]
fn a_stack_of_objects_becomes_a_merge_of_their_draws() {
    let mut doc = Document::new();
    let (_, objects) = page_with(
        &mut doc,
        &[
            Shape::Rectangle(Vector::new(10.0, 10.0)),
            Shape::Ellipse(Vector::new(5.0, 5.0)),
        ],
    );
    ops::add_stroke(&mut doc, objects[1]).unwrap();
    let location = Value::Vector(Vector::new(100.0, 50.0));
    set(
        &mut doc,
        objects[0],
        path_object::LOCATION.index(),
        location,
    );
    let programs = Evaluator::new().programs(&doc).to_vec();
    assert_eq!(programs.len(), 1);
    let program = &programs[0];
    assert_eq!(program.size(), Vector::new(297.0, 210.0));
    // The square below, the disc (a fill and a stroke) above.
    let Step::Merge(layers) = root(program) else {
        panic!("a merge at the root: {program:?}");
    };
    assert_eq!(layers.len(), 2);
    assert_eq!(draws(program).len(), 3);
    // The square is placed in the world, centered on its location.
    let Step::Draw(square) = &program.steps()[layers[0].step.0] else {
        panic!("the square draws its one fill");
    };
    let bounds = square.outline.bounds();
    assert!((bounds.center().x - 100.0).abs() < 1e-9 && (bounds.center().y - 50.0).abs() < 1e-9);
    assert!((bounds.width() - 10.0).abs() < 1e-9);
}

#[test]
fn groups_place_their_members_and_composite_them_together() {
    let mut doc = Document::new();
    let square = Shape::Rectangle(Vector::new(2.0, 2.0));
    let (_, objects) = page_with(&mut doc, &[square.clone(), square]);
    set(
        &mut doc,
        objects[0],
        path_object::LOCATION.index(),
        Value::Vector(Vector::new(-5.0, 0.0)),
    );
    set(
        &mut doc,
        objects[1],
        path_object::LOCATION.index(),
        Value::Vector(Vector::new(5.0, 0.0)),
    );
    let group = ops::group(&mut doc, &objects).unwrap();
    set(
        &mut doc,
        group,
        group_object::OPACITY.index(),
        Value::Float(0.5),
    );
    // A half turn about the group's origin, their middle, swaps the members.
    set(
        &mut doc,
        group,
        group_object::ROTATION.index(),
        Value::Float(core::f64::consts::PI),
    );
    let program = &first_program(&doc);
    let centers: Vec<f64> = draws(program)
        .iter()
        .map(|d| d.outline.bounds().center().x)
        .collect();
    assert_eq!(centers.len(), 2);
    assert!(
        (centers[0] - 5.0).abs() < 1e-9 && (centers[1] + 5.0).abs() < 1e-9,
        "{centers:?}"
    );
    // The group's opacity applies to its members together.
    assert!(
        matches!(root(program), Step::Merge(inputs) if inputs.len() == 1 && (inputs[0].opacity - 0.5).abs() < 1e-12)
    );
}

#[test]
fn a_blend_and_a_mask_through_a_transform_on_an_unrelated_object() {
    let mut doc = Document::new();
    let (stack, objects) = page_with(
        &mut doc,
        &[
            Shape::Rectangle(Vector::new(20.0, 20.0)),
            Shape::Ellipse(Vector::new(3.0, 3.0)),
            Shape::Rectangle(Vector::new(8.0, 8.0)),
        ],
    );
    let (backdrop, shape, masked) = (objects[0], objects[1], objects[2]);
    // The shape's world geometry, moved 7 to the right, restricts the masked object...
    let moved = doc.add_node(transform_geometry::KIND).unwrap();
    let cover = doc.add_node(coverage::KIND).unwrap();
    let restrict = doc.add_node(mask::KIND).unwrap();
    doc.link(
        at(shape, path_object::WORLD_GEOMETRY.index()),
        at(moved, transform_geometry::GEOMETRY.index()),
    )
    .unwrap();
    set(
        &mut doc,
        moved,
        transform_geometry::TRANSFORM.index(),
        Value::Transform(Affine::translate(Vector::new(7.0, 0.0))),
    );
    doc.link(
        at(moved, transform_geometry::RESULT.index()),
        at(cover, coverage::GEOMETRY.index()),
    )
    .unwrap();
    doc.link(
        at(cover, coverage::COVERAGE.index()),
        at(restrict, mask::COVERAGE.index()),
    )
    .unwrap();
    doc.link(
        at(masked, path_object::LAYER.index()),
        at(restrict, mask::CONTENT.index()),
    )
    .unwrap();
    // ...which is blended by Multiply onto the backdrop, in the page's stack.
    let mix = doc.add_node(blend::KIND).unwrap();
    doc.link(
        at(backdrop, path_object::LAYER.index()),
        at(mix, blend::BACKDROP.index()),
    )
    .unwrap();
    doc.link(
        at(restrict, mask::LAYER.index()),
        at(mix, blend::SOURCE.index()),
    )
    .unwrap();
    set(
        &mut doc,
        mix,
        blend::MODE.index(),
        Value::Enum(koebi_doc::EnumKind::BlendMode, BlendMode::Multiply as u32),
    );
    doc.link(
        at(mix, blend::LAYER.index()),
        at(stack, merge::LAYERS.index()),
    )
    .unwrap();
    let program = &first_program(&doc);
    let blend_step = program.steps().iter().find_map(|step| match step {
        Step::Blend { mode, source, .. } => Some((*mode, *source)),
        _ => None,
    });
    let (mode, source) = blend_step.expect("a blend step");
    assert_eq!(mode, BlendMode::Multiply);
    let Step::Mask {
        coverage: StepCoverage::Outline { outline, .. },
        ..
    } = &program.steps()[source.0]
    else {
        panic!("the blend's source is the mask: {program:?}");
    };
    let center = outline.bounds().center();
    assert!(
        (center.x - 7.0).abs() < 1e-9 && center.y.abs() < 1e-9,
        "{center:?}"
    );
    // The backdrop and the masked object are drawn once each, though each feeds two places.
    let backdrop_draws = draws(program)
        .iter()
        .filter(|d| (d.outline.bounds().width() - 20.0).abs() < 1e-9)
        .count();
    assert_eq!(backdrop_draws, 1);
}

#[test]
fn invalid_links_are_ignored_and_hidden_objects_draw_nothing() {
    let mut doc = Document::new();
    let (_, objects) = page_with(&mut doc, &[Shape::Rectangle(Vector::new(4.0, 4.0))]);
    let paint = doc
        .graph()
        .links_into(at(objects[0], path_object::PAINTS.index()))
        .first()
        .and_then(|id| doc.graph().link_of(*id))
        .map(|link| link.from.node)
        .unwrap();
    // A stroke's paint into the fill's color: invalid, so the fill keeps its own black.
    let other = doc.add_node(stroke::KIND).unwrap();
    doc.link(
        at(other, stroke::PAINT.index()),
        at(paint, fill::COLOR.index()),
    )
    .unwrap();
    let program = &first_program(&doc);
    assert_eq!(draws(program)[0].color, LinearRgba::new(0.0, 0.0, 0.0, 1.0));
    // Hidden by itself, then by a collection above its own.
    set(
        &mut doc,
        objects[0],
        path_object::VISIBLE.index(),
        Value::Bool(false),
    );
    assert_eq!(first_program(&doc).root(), None);
    set(
        &mut doc,
        objects[0],
        path_object::VISIBLE.index(),
        Value::Bool(true),
    );
    let outer = ops::add_collection(&mut doc, "Outer", None).unwrap();
    let inner = ops::add_collection(&mut doc, "Inner", Some(outer)).unwrap();
    ops::link_to_collection(&mut doc, inner, objects[0]).unwrap();
    let mut evaluator = Evaluator::new();
    assert!(evaluator.programs(&doc)[0].root().is_some());
    ops::set_collection_visible(&mut doc, outer, false).unwrap();
    assert_eq!(evaluator.programs(&doc)[0].root(), None);
}

#[test]
fn an_edit_re_evaluates_only_what_depends_on_it() {
    let mut doc = Document::new();
    let (_, objects) = page_with(
        &mut doc,
        &[
            Shape::Rectangle(Vector::new(4.0, 4.0)),
            Shape::Ellipse(Vector::new(2.0, 2.0)),
        ],
    );
    let stroked = ops::add_stroke(&mut doc, objects[1]).unwrap();
    let paint_of = |doc: &Document, object: NodeId| {
        let id = doc
            .graph()
            .links_into(at(object, path_object::PAINTS.index()))[0];
        doc.graph().link_of(id).unwrap().from.node
    };
    let first_fill = paint_of(&doc, objects[0]);
    let mut evaluator = Evaluator::new();
    evaluator.programs(&doc);
    // Nothing changed: nothing evaluated, nothing outlined.
    evaluator.reset_stats();
    evaluator.programs(&doc);
    assert_eq!(evaluator.stats.evaluated.values().sum::<u64>(), 0);
    // One fill's color: that paint again, no geometry, no outline, nothing of the other object.
    evaluator.reset_stats();
    let red = Value::Color(LinearRgba::new(1.0, 0.0, 0.0, 1.0));
    set(&mut doc, first_fill, fill::COLOR.index(), red);
    let program = evaluator.programs(&doc).to_vec();
    assert_eq!(evaluator.stats.of(fill::KIND.id), 1);
    assert_eq!(evaluator.stats.of(koebi_doc::kinds::rectangle::KIND.id), 0);
    assert_eq!(evaluator.stats.of(koebi_doc::kinds::ellipse::KIND.id), 0);
    assert_eq!(evaluator.stats.of(stroke::KIND.id), 0);
    assert_eq!(evaluator.stats.outlines, 0);
    assert_eq!(
        draws(&program[0])[0].color,
        LinearRgba::new(1.0, 0.0, 0.0, 1.0)
    );
    // A stroke's color: its outline is kept; its width: outlined again.
    evaluator.reset_stats();
    set(
        &mut doc,
        stroked,
        stroke::COLOR.index(),
        Value::Color(LinearRgba::new(0.0, 0.0, 1.0, 1.0)),
    );
    evaluator.programs(&doc);
    assert_eq!(evaluator.stats.outlines, 0);
    evaluator.reset_stats();
    set(&mut doc, stroked, stroke::WIDTH.index(), Value::Float(3.0));
    evaluator.programs(&doc);
    assert_eq!(evaluator.stats.outlines, 1);
}

#[test]
fn driven_values_and_parents_evaluate_through_links() {
    let mut doc = Document::new();
    let (_, objects) = page_with(
        &mut doc,
        &[
            Shape::Rectangle(Vector::new(2.0, 2.0)),
            Shape::Rectangle(Vector::new(2.0, 2.0)),
        ],
    );
    let (parent, child) = (objects[0], objects[1]);
    set(
        &mut doc,
        child,
        path_object::LOCATION.index(),
        Value::Vector(Vector::new(10.0, 0.0)),
    );
    ops::parent(&mut doc, child, parent).unwrap();
    // A number drives the parent's rotation: a quarter turn carries the child to (0, 10).
    let angle = doc.add_node(float::KIND).unwrap();
    set(
        &mut doc,
        angle,
        float::VALUE.index(),
        Value::Float(core::f64::consts::FRAC_PI_2),
    );
    doc.link(at(angle, 0), at(parent, path_object::ROTATION.index()))
        .unwrap();
    let program = &first_program(&doc);
    let centers: Vec<_> = draws(program)
        .iter()
        .map(|d| d.outline.bounds().center())
        .collect();
    assert!(
        centers
            .iter()
            .any(|c| c.x.abs() < 1e-9 && (c.y - 10.0).abs() < 1e-9),
        "{centers:?}"
    );
}

#[test]
fn a_page_whose_content_did_not_change_keeps_its_program() {
    let mut doc = Document::new();
    let (_, first) = page_with(&mut doc, &[Shape::Rectangle(Vector::new(4.0, 4.0))]);
    let (_, second) = page_with(&mut doc, &[Shape::Ellipse(Vector::new(2.0, 2.0))]);
    let mut evaluator = Evaluator::new();
    let before = evaluator.programs(&doc).to_vec();
    assert_eq!(before.len(), 2);
    assert_ne!(before[0].revision(), before[1].revision());
    // Nothing changed: the very same programs.
    let again = evaluator.programs(&doc).to_vec();
    assert!(Arc::ptr_eq(&before[0], &again[0]) && Arc::ptr_eq(&before[1], &again[1]));
    // An edit on the second page: the first keeps its program, the second gets a new revision.
    let moved = Value::Vector(Vector::new(30.0, 0.0));
    set(&mut doc, second[0], path_object::LOCATION.index(), moved);
    let after = evaluator.programs(&doc).to_vec();
    assert!(Arc::ptr_eq(&before[0], &after[0]));
    assert_ne!(after[1].revision(), before[1].revision());
    assert_ne!(*after[1], *before[1]);
    // Equality compares the content: the same page evaluated anew equals it, revision aside.
    let fresh = Evaluator::new().programs(&doc).to_vec();
    assert_eq!(*fresh[0], *after[0]);
    assert_ne!(fresh[0].revision(), after[0].revision());
    // Moving the first page's object back where it was gives a new program all the same.
    set(
        &mut doc,
        first[0],
        path_object::LOCATION.index(),
        Value::Vector(Vector::ZERO),
    );
    let last = evaluator.programs(&doc).to_vec();
    assert!(Arc::ptr_eq(&after[1], &last[1]));
}

#[test]
fn a_recolored_object_keeps_the_outline_it_was_placed_with() {
    let mut doc = Document::new();
    let (_, objects) = page_with(
        &mut doc,
        &[
            Shape::Rectangle(Vector::new(4.0, 4.0)),
            Shape::Ellipse(Vector::new(2.0, 2.0)),
        ],
    );
    for (object, x) in objects.iter().zip([10.0, -20.0]) {
        let location = Value::Vector(Vector::new(x, 5.0));
        set(&mut doc, *object, path_object::LOCATION.index(), location);
    }
    let fill_of = |doc: &Document, object: NodeId| {
        let id = doc
            .graph()
            .links_into(at(object, path_object::PAINTS.index()))[0];
        doc.graph().link_of(id).unwrap().from.node
    };
    let mut evaluator = Evaluator::new();
    let before = Arc::clone(&evaluator.programs(&doc)[0]);
    let red = Value::Color(LinearRgba::new(1.0, 0.0, 0.0, 1.0));
    let paint = fill_of(&doc, objects[0]);
    set(&mut doc, paint, fill::COLOR.index(), red);
    let after = Arc::clone(&evaluator.programs(&doc)[0]);
    let (old, new) = (draws(&before), draws(&after));
    assert_eq!(new[0].color, LinearRgba::new(1.0, 0.0, 0.0, 1.0));
    // Placed in the world by the same transform: the very same outlines, the other object's
    // too.
    for (old, new) in old.iter().zip(&new) {
        assert!(Arc::ptr_eq(&old.outline, &new.outline));
    }
    // Moved: a new outline, in its new place.
    let moved = Value::Vector(Vector::new(11.0, 5.0));
    set(&mut doc, objects[0], path_object::LOCATION.index(), moved);
    let moved = Arc::clone(&evaluator.programs(&doc)[0]);
    let placed = &draws(&moved)[0].outline;
    assert!(!Arc::ptr_eq(&new[0].outline, placed));
    assert!((placed.bounds().min.x - new[0].outline.bounds().min.x - 1.0).abs() < 1e-12);
    assert!(Arc::ptr_eq(&new[1].outline, &draws(&moved)[1].outline));
}

#[test]
fn a_stack_reads_again_only_the_layers_that_changed() {
    let mut doc = Document::new();
    let shapes: Vec<Shape> = (0..50)
        .map(|i| Shape::Rectangle(Vector::new(1.0 + f64::from(i), 2.0)))
        .collect();
    let (stack, objects) = page_with(&mut doc, &shapes);
    let mut evaluator = Evaluator::new();
    let before = Arc::clone(&evaluator.programs(&doc)[0]);
    evaluator.reset_stats();
    let moved = Value::Vector(Vector::new(7.0, 0.0));
    set(&mut doc, objects[20], path_object::LOCATION.index(), moved);
    let after = Arc::clone(&evaluator.programs(&doc)[0]);
    // The object's layer, and the stack and page that hold it: nothing of the others.
    assert_eq!(evaluator.stats.of(path_object::KIND.id), 1);
    assert_eq!(evaluator.stats.of(merge::KIND.id), 1);
    let (old, new) = (draws(&before), draws(&after));
    assert_eq!(old.len(), 50);
    for (at, (old, new)) in old.iter().zip(&new).enumerate() {
        assert_eq!(Arc::ptr_eq(&old.outline, &new.outline), at != 20, "{at}");
    }
    // The program is the one a new evaluator makes.
    assert_eq!(*after, *first_program(&doc));
    // An object removed and one added: the stack's links change, all are found again.
    doc.apply(Edit::RemoveNode(objects[3])).unwrap();
    ops::add_shape(&mut doc, stack, Shape::Ellipse(Vector::new(3.0, 3.0))).unwrap();
    let changed = Arc::clone(&evaluator.programs(&doc)[0]);
    assert_eq!(draws(&changed).len(), 50);
    assert_eq!(*changed, *first_program(&doc));
}

#[test]
fn a_stroke_in_pixels_is_left_to_the_canvas_with_its_world_geometry() {
    let mut doc = Document::new();
    let (_, objects) = page_with(&mut doc, &[Shape::Rectangle(Vector::new(10.0, 4.0))]);
    let object = objects[0];
    set(
        &mut doc,
        object,
        path_object::LOCATION.index(),
        Value::Vector(Vector::new(50.0, 20.0)),
    );
    set(
        &mut doc,
        object,
        path_object::SCALE.index(),
        Value::Vector(Vector::new(3.0, 3.0)),
    );
    let stroked = ops::add_stroke(&mut doc, object).unwrap();
    set(&mut doc, stroked, stroke::WIDTH.index(), Value::Float(1.5));
    set(
        &mut doc,
        stroked,
        stroke::UNITS.index(),
        StrokeUnits::Viewport.into_value(),
    );
    let mut evaluator = Evaluator::new();
    evaluator.reset_stats();
    let program = Arc::clone(&evaluator.programs(&doc)[0]);
    let strokes: Vec<&ViewStroke> = program
        .steps()
        .iter()
        .filter_map(|step| match step {
            Step::ViewStroke(stroke) => Some(stroke),
            _ => None,
        })
        .collect();
    assert_eq!(strokes.len(), 1);
    // Nothing outlined here; the geometry placed in the world, the pixels left as they are.
    assert_eq!(evaluator.stats.outlines, 0);
    let bounds = strokes[0].geometry.bounds();
    assert!((bounds.width() - 30.0).abs() < 1e-9 && (bounds.center().x - 50.0).abs() < 1e-9);
    assert!((strokes[0].style.width - 1.5).abs() < 1e-12);
    // The fill still draws its outline.
    assert_eq!(draws(&program).len(), 1);
    // Recolored: the same placed geometry.
    set(
        &mut doc,
        stroked,
        stroke::COLOR.index(),
        Value::Color(LinearRgba::new(0.0, 0.0, 1.0, 1.0)),
    );
    let recolored = Arc::clone(&evaluator.programs(&doc)[0]);
    let again = recolored
        .steps()
        .iter()
        .find_map(|step| match step {
            Step::ViewStroke(stroke) => Some(stroke),
            _ => None,
        })
        .unwrap();
    assert!(Arc::ptr_eq(&again.geometry, &strokes[0].geometry));
    assert_eq!(again.color, LinearRgba::new(0.0, 0.0, 1.0, 1.0));
    // Back in document units: outlined here again.
    set(
        &mut doc,
        stroked,
        stroke::UNITS.index(),
        StrokeUnits::Document.into_value(),
    );
    let document = Arc::clone(&evaluator.programs(&doc)[0]);
    assert_eq!(draws(&document).len(), 2);
    assert!(
        !document
            .steps()
            .iter()
            .any(|step| matches!(step, Step::ViewStroke(_)))
    );
}

#[test]
fn draws_name_the_object_they_paint_an_instance_its_own() {
    let mut doc = Document::new();
    let (_, objects) = page_with(&mut doc, &[Shape::Rectangle(Vector::new(4.0, 4.0))]);
    ops::add_stroke(&mut doc, objects[0]).unwrap();
    let instance = ops::duplicate_linked(&mut doc, objects[0]).unwrap();
    let program = first_program(&doc);
    let sources: Vec<Option<NodeId>> = draws(&program).iter().map(|draw| draw.object).collect();
    // The rectangle's fill and stroke, then the instance's, which show the same outlines.
    assert_eq!(
        sources,
        [
            Some(objects[0]),
            Some(objects[0]),
            Some(instance),
            Some(instance)
        ]
    );
    let all = draws(&program);
    assert!(Arc::ptr_eq(&all[0].outline, &all[2].outline));
}

#[test]
fn a_pattern_fill_repeats_its_motif_on_a_lattice_cut_at_its_clip() {
    use koebi_doc::eval::Lattice;
    use koebi_doc::kinds::pattern_fill;
    use koebi_doc::{PatternAnchor, PatternClip};

    let mut doc = Document::new();
    let (_, objects) = page_with(
        &mut doc,
        &[
            Shape::Rectangle(Vector::new(60.0, 40.0)),
            Shape::Ellipse(Vector::new(3.0, 3.0)),
        ],
    );
    let (host, motif) = (objects[0], objects[1]);
    set(
        &mut doc,
        host,
        path_object::LOCATION.index(),
        Value::Vector(Vector::new(100.0, 50.0)),
    );
    set(
        &mut doc,
        motif,
        path_object::LOCATION.index(),
        Value::Vector(Vector::new(500.0, 500.0)),
    );
    let pattern = ops::add_pattern_fill(&mut doc, host, motif).unwrap();
    set(
        &mut doc,
        pattern,
        pattern_fill::SPACING.index(),
        Value::Vector(Vector::new(10.0, 8.0)),
    );
    let pattern_of = |doc: &Document| {
        let program = first_program(doc);
        let steps = program.steps();
        let found = steps.iter().find_map(|step| match step {
            Step::Pattern {
                object,
                motif,
                lattice,
                bounds,
            } => Some((*object, *motif, *lattice, *bounds)),
            _ => None,
        });
        let masked = steps.iter().any(|step| {
            matches!(step, Step::Mask { content, .. }
                if matches!(steps[content.0], Step::Pattern { .. }))
        });
        (found.expect("a pattern step"), masked, Arc::clone(&program))
    };
    // At its bounds by default: no mask; the lattice follows the object from its origin.
    let ((object, motif_step, lattice, bounds), masked, program) = pattern_of(&doc);
    assert_eq!(object, Some(host));
    assert!(!masked);
    assert_eq!(
        lattice,
        Lattice {
            origin: koebi_geom::Point::new(100.0, 50.0),
            a: Vector::new(10.0, 0.0),
            b: Vector::new(0.0, 8.0),
            motif: Affine::IDENTITY,
        }
    );
    assert!((bounds.min.x - 70.0).abs() < 1e-9 && (bounds.max.y - 70.0).abs() < 1e-9);
    // The motif is drawn in its own space: centered on its origin, not where it lies.
    let first_draw = |id: StepId| match &program.steps()[id.0] {
        Step::Draw(draw) => Some(draw.outline.bounds().center()),
        Step::Merge(inputs) => match &program.steps()[inputs[0].step.0] {
            Step::Draw(draw) => Some(draw.outline.bounds().center()),
            _ => None,
        },
        _ => None,
    };
    let center = first_draw(motif_step).expect("the motif's fill");
    assert!(center.x.abs() < 1e-9 && center.y.abs() < 1e-9, "{center:?}");
    // Cut at the outline: the pattern is a mask's content.
    set(
        &mut doc,
        pattern,
        pattern_fill::CLIP.index(),
        PatternClip::Contour.into_value(),
    );
    let ((_, _, _, bounds), masked, _) = pattern_of(&doc);
    assert!(masked && (bounds.width() - 60.0).abs() < 1e-9);
    // Cut at the outer edge of a stroke 4 wide: the outline grown by 2; at its inner edge,
    // shrunk by 2.
    let stroked = ops::add_stroke(&mut doc, host).unwrap();
    set(&mut doc, stroked, stroke::WIDTH.index(), Value::Float(4.0));
    set(
        &mut doc,
        pattern,
        pattern_fill::CLIP.index(),
        PatternClip::StrokeOuter.into_value(),
    );
    let ((_, _, _, bounds), masked, _) = pattern_of(&doc);
    assert!(masked && (bounds.width() - 64.0).abs() < 1e-6, "{bounds:?}");
    set(
        &mut doc,
        pattern,
        pattern_fill::CLIP.index(),
        PatternClip::StrokeInner.into_value(),
    );
    let ((_, _, _, bounds), _, _) = pattern_of(&doc);
    assert!((bounds.width() - 56.0).abs() < 1e-6, "{bounds:?}");
    // Turned with the object, the lattice and its motifs turn; pinned to the document, the
    // lattice stays.
    set(
        &mut doc,
        host,
        path_object::ROTATION.index(),
        Value::Float(0.5),
    );
    let ((_, _, lattice, _), _, _) = pattern_of(&doc);
    assert!((lattice.a.y - 10.0 * 0.5_f64.sin()).abs() < 1e-9);
    let turned = lattice.motif.apply_vector(Vector::new(1.0, 0.0));
    assert!((turned.y - 0.5_f64.sin()).abs() < 1e-9);
    set(
        &mut doc,
        pattern,
        pattern_fill::ANCHOR.index(),
        PatternAnchor::Document.into_value(),
    );
    let ((_, _, lattice, _), _, _) = pattern_of(&doc);
    assert_eq!(lattice.origin, koebi_geom::Point::new(0.0, 0.0));
    assert_eq!(lattice.a, Vector::new(10.0, 0.0));
}

#[test]
fn an_instanced_fill_generates_its_stamps_and_lines() {
    use koebi_doc::instanced::{InstancedElement, InstancedSettings, LineSet, StampMode};
    use koebi_doc::kinds::instanced_fill;
    use koebi_doc::{InstancedLayout, PatternAnchor, Settings};

    let mut doc = Document::new();
    let (_, objects) = page_with(&mut doc, &[Shape::Rectangle(Vector::new(60.0, 40.0))]);
    let host = objects[0];
    set(
        &mut doc,
        host,
        path_object::LOCATION.index(),
        Value::Vector(Vector::new(100.0, 50.0)),
    );
    let fill = ops::add_instanced_fill(&mut doc, host).unwrap();
    set(
        &mut doc,
        fill,
        instanced_fill::SPACING.index(),
        Value::Vector(Vector::new(10.0, 10.0)),
    );
    let settings = |doc: &mut Document, settings: InstancedSettings| {
        doc.apply(Edit::SetSettings {
            node: fill,
            settings: Box::new(Settings::Instanced(Arc::new(settings))),
        })
        .unwrap();
    };
    // The draws the fill generated, after the host's own fill: those naming the host, past the
    // first.
    let stamps = |doc: &Document| -> Vec<Draw> {
        let program = first_program(doc);
        draws(&program)
            .into_iter()
            .filter(|draw| draw.object == Some(host))
            .skip(1)
            .cloned()
            .collect()
    };
    // By default: circles of radius 2 fused into one outline, a stamp at every lattice point in
    // the shape, in the fill's color.
    let fused = stamps(&doc);
    assert_eq!(fused.len(), 1);
    let circles = fused[0].outline.contours().len();
    assert!((15..=35).contains(&circles), "{circles}");
    let stamp = contour_bounds(&fused[0].outline.contours()[0]);
    assert!((stamp.width() - 4.0).abs() < 1e-9);
    // Stamps centered on the lattice from the shape's origin, placed with the shape.
    let center = stamp.center();
    let on_lattice = |value: f64, origin: f64| {
        ((value - origin) / 10.0 - ((value - origin) / 10.0).round()).abs() < 1e-9
    };
    assert!(
        on_lattice(center.x, 100.0) && on_lattice(center.y, 50.0),
        "{center:?}"
    );
    // Blended: one draw a stamp; subtracted: one erasing draw.
    let element = |mode: StampMode| InstancedElement {
        mode,
        ..InstancedElement::default()
    };
    settings(
        &mut doc,
        InstancedSettings {
            elements: vec![element(StampMode::Blend)],
            lines: Vec::new(),
        },
    );
    assert_eq!(stamps(&doc).len(), circles);
    settings(
        &mut doc,
        InstancedSettings {
            elements: vec![element(StampMode::Fusion), element(StampMode::Subtract)],
            lines: Vec::new(),
        },
    );
    let program = first_program(&doc);
    let erases = program.steps().iter().any(|step| {
        matches!(step, Step::Merge(inputs) if inputs.iter().any(|input| input.blend == BlendMode::Erase))
    });
    assert!(erases);
    // A family of lines, fused: one outline across the shape.
    settings(
        &mut doc,
        InstancedSettings {
            elements: Vec::new(),
            lines: vec![LineSet {
                spacing: 8.0,
                width: 2.0,
                ..LineSet::default()
            }],
        },
    );
    let lined = stamps(&doc);
    assert_eq!(lined.len(), 1);
    assert!(lined[0].outline.contours().len() >= 5);
    // Scattered from a seed: the same seed, the same stamps; another, others.
    settings(&mut doc, InstancedSettings::default());
    set(
        &mut doc,
        fill,
        instanced_fill::LAYOUT.index(),
        InstancedLayout::Scatter.into_value(),
    );
    let first = stamps(&doc);
    assert_eq!(first, stamps(&doc));
    assert_eq!(
        first[0],
        Evaluator::new().programs(&doc)[0]
            .steps()
            .iter()
            .find_map(|step| match step {
                Step::Draw(draw)
                    if draw.object == Some(host) && draw.outline.contours().len() > 1 =>
                    Some(draw.clone()),
                _ => None,
            })
            .unwrap()
    );
    set(&mut doc, fill, instanced_fill::SEED.index(), Value::Int(9));
    assert_ne!(first, stamps(&doc));
    // Pinned to the document, the layout stays when the shape moves.
    set(
        &mut doc,
        fill,
        instanced_fill::LAYOUT.index(),
        InstancedLayout::Grid.into_value(),
    );
    set(
        &mut doc,
        fill,
        instanced_fill::ANCHOR.index(),
        PatternAnchor::Document.into_value(),
    );
    let before = contour_bounds(&stamps(&doc)[0].outline.contours()[0]).center();
    set(
        &mut doc,
        host,
        path_object::LOCATION.index(),
        Value::Vector(Vector::new(103.0, 50.0)),
    );
    let after = contour_bounds(&stamps(&doc)[0].outline.contours()[0]).center();
    assert!(
        on_lattice(before.x, 0.0) && on_lattice(after.x, 0.0),
        "{before:?} {after:?}"
    );
}
