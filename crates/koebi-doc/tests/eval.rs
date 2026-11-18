//! Evaluation into per-page programs: canonical documents (stacks, groups, blends, masks), a
//! customized one (a shape masking an unrelated object through a transform), invalid links,
//! visibility, and what an edit re-evaluates.

use std::sync::Arc;

use koebi_core::LinearRgba;
use koebi_doc::eval::{Draw, Evaluator, Program, Step, StepCoverage};
use koebi_doc::kinds::{
    blend, coverage, fill, float, group_object, mask, merge, path_object, stroke,
    transform_geometry,
};
use koebi_doc::ops::{self, Shape};
use koebi_doc::{BlendMode, Document, Edit, NodeId, SocketRef, Value};
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
