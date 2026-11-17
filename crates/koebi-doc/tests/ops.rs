//! Operations and their history: each one undone and redone gives the same document, random
//! sequences of them unwind to the start, parenting and applying keep world placements.

use koebi_doc::kinds::{group_object as g, path_object as o};
use koebi_doc::ops::{self, Apply, ClearParent, Shape};
use koebi_doc::transforms::{self, close};
use koebi_doc::{BlendMode, ColorTag, Document, NodeId, Value};
use koebi_geom::{Path, Point, Rect, Vector};

/// A small deterministic generator.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next()).unwrap_or(0) % n.max(1)
    }

    fn unit(&mut self) -> f64 {
        #[allow(clippy::cast_precision_loss, reason = "31 bits")]
        let value = self.next() as f64 / f64::from(1_u32 << 31);
        value
    }
}

fn page(doc: &mut Document) -> NodeId {
    ops::add_page(doc).expect("a page").stack
}

fn place(
    doc: &mut Document,
    object: NodeId,
    location: (f64, f64),
    rotation: f64,
    scale: (f64, f64),
) {
    for (index, value) in [
        (
            o::LOCATION.index(),
            Value::Vector(Vector::new(location.0, location.1)),
        ),
        (o::ROTATION.index(), Value::Float(rotation)),
        (
            o::SCALE.index(),
            Value::Vector(Vector::new(scale.0, scale.1)),
        ),
    ] {
        ops::set_property(doc, object, index, value).expect("a placement");
    }
}

/// Undoing then redoing the last step gives the document it made.
#[track_caller]
fn round_trip(doc: &mut Document) {
    let after = doc.clone();
    assert!(doc.undo());
    assert!(doc.redo());
    assert_eq!(doc.content(), after.content());
}

#[test]
fn every_operation_undoes_and_redoes() {
    let mut doc = Document::new();
    let stack = page(&mut doc);
    round_trip(&mut doc);
    let square =
        ops::add_shape(&mut doc, stack, Shape::Rectangle(Vector::new(20.0, 10.0))).unwrap();
    round_trip(&mut doc);
    let circle = ops::add_shape(&mut doc, stack, Shape::Ellipse(Vector::new(5.0, 5.0))).unwrap();
    round_trip(&mut doc);
    let triangle = ops::add_shape(&mut doc, stack, Shape::Triangle(8.0)).unwrap();
    round_trip(&mut doc);
    let drawn = Path::rectangle(Rect::new(Point::new(0.0, 0.0), Point::new(3.0, 4.0)));
    ops::add_shape(&mut doc, stack, Shape::Path(drawn)).unwrap();
    round_trip(&mut doc);
    let stroke = ops::add_stroke(&mut doc, square).unwrap();
    round_trip(&mut doc);
    let second = ops::add_fill(&mut doc, square).unwrap();
    round_trip(&mut doc);
    ops::move_paint(&mut doc, square, stroke, 0).unwrap();
    round_trip(&mut doc);
    ops::remove_paint(&mut doc, square, second).unwrap();
    round_trip(&mut doc);
    ops::set_blend(&mut doc, circle, BlendMode::Multiply).unwrap();
    round_trip(&mut doc);
    ops::set_opacity(&mut doc, circle, 0.4).unwrap();
    round_trip(&mut doc);
    ops::move_in_stack(&mut doc, triangle, 0).unwrap();
    round_trip(&mut doc);
    let group = ops::group(&mut doc, &[square, circle]).unwrap();
    round_trip(&mut doc);
    ops::parent(&mut doc, triangle, group).unwrap();
    round_trip(&mut doc);
    ops::apply_transform(&mut doc, group, Apply::All).unwrap();
    round_trip(&mut doc);
    ops::clear_parent(&mut doc, triangle, ClearParent::KeepTransform).unwrap();
    round_trip(&mut doc);
    let copy = ops::duplicate(&mut doc, triangle).unwrap();
    round_trip(&mut doc);
    ops::duplicate_linked(&mut doc, copy).unwrap();
    round_trip(&mut doc);
    ops::ungroup(&mut doc, group).unwrap();
    round_trip(&mut doc);
    // Collections, every change undoable: Carto's were not.
    let shapes = ops::add_collection(&mut doc, "Shapes", None).unwrap();
    round_trip(&mut doc);
    let inner = ops::add_collection(&mut doc, "Round", Some(shapes)).unwrap();
    round_trip(&mut doc);
    ops::link_to_collection(&mut doc, inner, circle).unwrap();
    round_trip(&mut doc);
    ops::rename_collection(&mut doc, inner, "Circles").unwrap();
    round_trip(&mut doc);
    ops::set_collection_color(&mut doc, inner, ColorTag::Blue).unwrap();
    round_trip(&mut doc);
    ops::set_collection_selectable(&mut doc, inner, false).unwrap();
    round_trip(&mut doc);
    ops::set_collection_visible(&mut doc, shapes, false).unwrap();
    round_trip(&mut doc);
    // Hidden through its parent collection.
    assert!(doc.collections().hides(circle));
    assert!(!doc.collections().hides(square));
    ops::move_collection(&mut doc, inner, None).unwrap();
    round_trip(&mut doc);
    assert!(!doc.collections().hides(circle));
    ops::unlink_from_collection(&mut doc, inner, circle).unwrap();
    round_trip(&mut doc);
    ops::remove_collection(&mut doc, shapes).unwrap();
    round_trip(&mut doc);
    // A collection cannot go inside itself.
    let outer = ops::add_collection(&mut doc, "Outer", None).unwrap();
    let nested = ops::add_collection(&mut doc, "Nested", Some(outer)).unwrap();
    assert!(ops::move_collection(&mut doc, outer, Some(nested)).is_err());
}

#[test]
fn random_sequences_unwind_to_the_start_and_wind_back() {
    for seed in 1..=12 {
        let mut random = Random(seed);
        let mut doc = Document::new();
        let stack = page(&mut doc);
        let start = doc.clone();
        let base = doc.undo_labels().count();
        let mut objects: Vec<NodeId> = Vec::new();
        for _ in 0..40 {
            let before = doc.clone();
            let done = match random.below(10) {
                0 | 1 => {
                    let size = Vector::new(1.0 + random.unit() * 20.0, 1.0 + random.unit() * 20.0);
                    ops::add_shape(&mut doc, stack, Shape::Rectangle(size)).map(|o| objects.push(o))
                }
                2 if !objects.is_empty() => {
                    let o = objects[random.below(objects.len())];
                    let location = (random.unit() * 50.0, random.unit() * 50.0);
                    place(
                        &mut doc,
                        o,
                        location,
                        random.unit(),
                        (0.5 + random.unit(), 0.5 + random.unit()),
                    );
                    Ok(())
                }
                3 if objects.len() >= 2 => {
                    let (a, b) = (
                        objects[random.below(objects.len())],
                        objects[random.below(objects.len())],
                    );
                    ops::parent(&mut doc, a, b)
                }
                4 if !objects.is_empty() => {
                    let o = objects[random.below(objects.len())];
                    let mode = [
                        ClearParent::Clear,
                        ClearParent::KeepTransform,
                        ClearParent::Inverse,
                    ][random.below(3)];
                    ops::clear_parent(&mut doc, o, mode)
                }
                5 if !objects.is_empty() => {
                    let o = objects[random.below(objects.len())];
                    ops::duplicate(&mut doc, o).map(|copy| objects.push(copy))
                }
                6 if !objects.is_empty() => {
                    let o = objects[random.below(objects.len())];
                    ops::apply_transform(
                        &mut doc,
                        o,
                        [Apply::Location, Apply::Rotation, Apply::Scale, Apply::All]
                            [random.below(4)],
                    )
                }
                7 if !objects.is_empty() => {
                    let o = objects[random.below(objects.len())];
                    ops::add_stroke(&mut doc, o).map(|_| ())
                }
                8 if !objects.is_empty() => {
                    let o = objects[random.below(objects.len())];
                    ops::move_in_stack(&mut doc, o, random.below(objects.len()))
                }
                _ => {
                    let id = ops::add_collection(&mut doc, "C", None);
                    id.and_then(|c| match objects.first() {
                        Some(o) => ops::link_to_collection(&mut doc, c, *o),
                        None => Ok(()),
                    })
                }
            };
            // A refused operation leaves the document as it was.
            if done.is_err() {
                assert_eq!(doc.content(), before.content(), "seed {seed}");
            }
        }
        let end = doc.clone();
        let count = doc.undo_labels().count() - base;
        assert!(count > 10, "seed {seed}: {count} steps");
        for _ in 0..count {
            assert!(doc.undo());
        }
        assert_eq!(doc.content(), start.content(), "seed {seed}: undone");
        for _ in 0..count {
            assert!(doc.redo());
        }
        assert_eq!(doc.content(), end.content(), "seed {seed}: redone");
    }
}

#[test]
fn parenting_keeps_places_and_clearing_with_keep_transformation_restores_them() {
    let mut random = Random(7);
    for _ in 0..20 {
        let mut doc = Document::new();
        let stack = page(&mut doc);
        let shapes: Vec<NodeId> = (0..3)
            .map(|_| {
                ops::add_shape(&mut doc, stack, Shape::Ellipse(Vector::new(2.0, 1.0))).unwrap()
            })
            .collect();
        for &shape in &shapes {
            let scale = 0.3 + random.unit() * 2.0;
            place(
                &mut doc,
                shape,
                (random.unit() * 100.0 - 50.0, random.unit() * 100.0 - 50.0),
                random.unit() * 6.0,
                (scale, scale),
            );
        }
        let worlds: Vec<_> = shapes
            .iter()
            .map(|s| transforms::world(doc.graph(), *s))
            .collect();
        // A chain: the last under the middle under the first.
        ops::parent(&mut doc, shapes[1], shapes[0]).unwrap();
        ops::parent(&mut doc, shapes[2], shapes[1]).unwrap();
        for (shape, world) in shapes.iter().zip(&worlds) {
            assert!(close(transforms::world(doc.graph(), *shape), *world, 1e-9));
        }
        // Moving the root carries the chain, each keeping its own scale.
        let own_scale = transforms::local(doc.graph(), shapes[2]).unwrap().scale;
        place(&mut doc, shapes[0], (5.0, 5.0), 0.5, (2.0, 2.0));
        assert_eq!(
            transforms::local(doc.graph(), shapes[2]).unwrap().scale,
            own_scale
        );
        let moved: Vec<_> = shapes
            .iter()
            .map(|s| transforms::world(doc.graph(), *s))
            .collect();
        // Clearing with Keep Transformation restores those placements.
        ops::clear_parent(&mut doc, shapes[2], ClearParent::KeepTransform).unwrap();
        ops::clear_parent(&mut doc, shapes[1], ClearParent::KeepTransform).unwrap();
        for (shape, world) in shapes.iter().zip(&moved) {
            assert!(close(transforms::world(doc.graph(), *shape), *world, 1e-9));
        }
        assert!(ops::parent(&mut doc, shapes[0], shapes[0]).is_err());
    }
}

#[test]
fn applying_transforms_keeps_every_world_placement() {
    let mut doc = Document::new();
    let stack = page(&mut doc);
    let parent = ops::add_shape(&mut doc, stack, Shape::Rectangle(Vector::new(10.0, 4.0))).unwrap();
    let child = ops::add_shape(&mut doc, stack, Shape::Triangle(3.0)).unwrap();
    place(&mut doc, parent, (10.0, 20.0), 0.7, (2.0, 0.5));
    place(&mut doc, child, (30.0, -5.0), -0.3, (1.5, 1.5));
    ops::parent(&mut doc, child, parent).unwrap();
    let child_world = transforms::world(doc.graph(), child);
    for what in [Apply::Scale, Apply::Rotation, Apply::Location, Apply::All] {
        ops::apply_transform(&mut doc, parent, what).unwrap();
        assert!(
            close(transforms::world(doc.graph(), child), child_world, 1e-9),
            "{what:?}"
        );
        // The parent's own transform reset, its look carried by its geometry chain.
        let reset = transforms::local(doc.graph(), parent).unwrap();
        match what {
            Apply::Scale => assert_eq!(reset.scale, Vector::new(1.0, 1.0)),
            Apply::Rotation => assert!(reset.rotation.abs() < 1e-15),
            Apply::Location => assert_eq!(reset.location, Vector::ZERO),
            Apply::All => assert!(close(
                transforms::world(doc.graph(), parent),
                koebi_geom::Affine::IDENTITY,
                1e-12
            )),
        }
    }
    // A group's members keep their places when it is applied, and when it is ungrouped.
    let a = ops::add_shape(&mut doc, stack, Shape::Ellipse(Vector::new(1.0, 1.0))).unwrap();
    let b = ops::add_shape(&mut doc, stack, Shape::Ellipse(Vector::new(1.0, 1.0))).unwrap();
    place(&mut doc, a, (1.0, 2.0), 0.1, (1.0, 1.0));
    place(&mut doc, b, (-4.0, 3.0), 0.0, (2.0, 2.0));
    let worlds = [a, b].map(|o| transforms::world(doc.graph(), o));
    let group = ops::group(&mut doc, &[a, b]).unwrap();
    for (o, w) in [a, b].iter().zip(&worlds) {
        assert!(close(transforms::world(doc.graph(), *o), *w, 1e-9));
    }
    ops::set_property(&mut doc, group, g::ROTATION.index(), Value::Float(0.4)).unwrap();
    let turned = [a, b].map(|o| transforms::world(doc.graph(), o));
    ops::apply_transform(&mut doc, group, Apply::All).unwrap();
    for (o, w) in [a, b].iter().zip(&turned) {
        assert!(close(transforms::world(doc.graph(), *o), *w, 1e-9));
    }
    ops::ungroup(&mut doc, group).unwrap();
    for (o, w) in [a, b].iter().zip(&turned) {
        assert!(close(transforms::world(doc.graph(), *o), *w, 1e-9));
    }
}

#[test]
fn a_gesture_is_one_step_and_the_history_is_bounded() {
    let mut doc = Document::new();
    let stack = page(&mut doc);
    let shape = ops::add_shape(&mut doc, stack, Shape::Rectangle(Vector::new(1.0, 1.0))).unwrap();
    let before = doc.clone();
    doc.begin("Drag");
    for step in 0..50 {
        let at = Value::Vector(Vector::new(f64::from(step), 0.0));
        doc.apply(koebi_doc::Edit::SetInput {
            node: shape,
            index: o::LOCATION.index(),
            value: at,
        })
        .unwrap();
    }
    doc.commit();
    assert!(doc.undo());
    assert_eq!(doc.content(), before.content());
    // A cancelled gesture leaves nothing.
    doc.begin("Drag");
    let at = Value::Vector(Vector::new(9.0, 9.0));
    doc.apply(koebi_doc::Edit::SetInput {
        node: shape,
        index: o::LOCATION.index(),
        value: at,
    })
    .unwrap();
    doc.cancel();
    assert_eq!(doc.content(), before.content());
    // Bounded.
    doc.set_history_limit(3);
    for _ in 0..10 {
        ops::add_stroke(&mut doc, shape).unwrap();
    }
    assert_eq!(doc.undo_labels().count(), 3);
    assert!(doc.is_changed());
    doc.mark_saved();
    assert!(!doc.is_changed());
}
