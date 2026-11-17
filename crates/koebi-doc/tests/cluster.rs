//! The canonical cluster of a path object with two fills and a stroke (ADR 0028): built from
//! typed links, then checked as the views and evaluation will read it.

use koebi_core::LinearRgba;
use koebi_doc::kinds::{document, fill, merge, page, path_data, path_object, stroke};
use koebi_doc::{Graph, LinkStatus, MergeEntry, Settings, SocketRef};

#[test]
fn a_path_object_with_two_fills_and_a_stroke() {
    let mut graph = Graph::new();
    let output = graph.add_kind(document::KIND);
    let sheet = graph.add_kind(page::KIND);
    let stack = graph.add_kind(merge::KIND);
    let object = graph.add_kind(path_object::KIND);
    let shape = graph.add_kind(path_data::KIND);
    let (under, over, outline) = (
        graph.add_kind(fill::KIND),
        graph.add_kind(fill::KIND),
        graph.add_kind(stroke::KIND),
    );
    let blue = LinearRgba::new(0.0, 0.2, 1.0, 1.0);
    graph.set(over, fill::COLOR, blue).expect("a fill");
    graph.set(over, fill::OPACITY, 0.5).expect("a fill");
    graph.set(outline, stroke::WIDTH, 2.0).expect("a stroke");

    // Wired the canonical way, every link typed.
    let geometry = graph
        .connect(
            (shape, path_data::GEOMETRY),
            (object, path_object::GEOMETRY),
        )
        .unwrap();
    let paints: Vec<_> = [
        graph.connect((under, fill::PAINT), (object, path_object::PAINTS)),
        graph.connect((over, fill::PAINT), (object, path_object::PAINTS)),
        graph.connect((outline, stroke::PAINT), (object, path_object::PAINTS)),
    ]
    .into_iter()
    .map(|linked| linked.unwrap().id)
    .collect();
    let entered = graph
        .connect((object, path_object::LAYER), (stack, merge::LAYERS))
        .unwrap();
    graph
        .connect((stack, merge::LAYER), (sheet, page::CONTENT))
        .unwrap();
    graph
        .connect((sheet, page::PAGE), (output, document::PAGES))
        .unwrap();

    // Every link valid, carrying its value as is.
    let statuses = graph.statuses();
    assert_eq!(statuses.len(), 7);
    assert!(
        statuses
            .values()
            .all(|status| matches!(status, LinkStatus::Valid(_)))
    );

    // The paint stack in paint order: the two fills, then the stroke.
    let stack_of = SocketRef::new(object, path_object::PAINTS.index());
    assert_eq!(graph.links_into(stack_of), paints.as_slice());
    let sources: Vec<_> = paints
        .iter()
        .map(|id| graph.link_of(*id).unwrap().from.node)
        .collect();
    assert_eq!(sources, [under, over, outline]);
    assert_eq!(
        graph.links_into(SocketRef::new(object, path_object::GEOMETRY.index())),
        &[geometry.id]
    );

    // Values as set, and the rest as made.
    let over = graph.typed::<fill::Fill>(over).unwrap();
    assert_eq!(over.get(fill::COLOR), blue);
    assert!((over.get(fill::OPACITY) - 0.5).abs() < f64::EPSILON);
    let object_node = graph.typed::<path_object::PathObject>(object).unwrap();
    assert!(object_node.get(path_object::VISIBLE));
    assert_eq!(
        object_node.get(path_object::SCALE),
        koebi_geom::Vector::new(1.0, 1.0)
    );
    assert!(matches!(
        object_node.node().settings,
        Settings::Object { parent_inverse } if parent_inverse == koebi_geom::Affine::IDENTITY
    ));

    // The object enters the page's stack, whose entry composites it the default way.
    if let Some(node) = graph.node_mut(stack)
        && let Settings::Merge(entries) = &mut node.settings
    {
        entries.insert(entered.id, MergeEntry::default());
    }
    let Some(Settings::Merge(entries)) = graph.node(stack).map(|node| &node.settings) else {
        panic!("a merge has merge settings");
    };
    assert_eq!(entries[&entered.id], MergeEntry::default());
    assert!(matches!(
        graph.node(shape).unwrap().settings,
        Settings::Path(_)
    ));
}
