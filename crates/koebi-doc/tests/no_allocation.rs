//! The application asks for the programs every frame: when nothing changed, the answer costs a
//! comparison and allocates nothing (CLAUDE.md, *Hot paths*; ADR 0026 counts it).

use koebi_doc::eval::Evaluator;
use koebi_doc::ops::{self, Shape};
use koebi_doc::{Document, Edit, Value};
use koebi_geom::Vector;

#[global_allocator]
static ALLOCATOR: koebi_testkit::CountingAllocator = koebi_testkit::CountingAllocator;

#[test]
fn programs_asked_for_again_allocate_nothing() {
    let mut doc = Document::new();
    let page = ops::add_page(&mut doc).expect("a page");
    for i in 0..20 {
        let size = Vector::new(1.0 + f64::from(i), 2.0);
        ops::add_shape(&mut doc, page.stack, Shape::Rectangle(size)).expect("a shape");
    }
    let mut evaluator = Evaluator::new();
    let revision = evaluator.programs(&doc)[0].revision();
    let (count, revisions) = koebi_testkit::allocations(|| {
        (0..100)
            .map(|_| evaluator.programs(&doc)[0].revision())
            .fold(0, |sum, seen| sum + u64::from(seen == revision))
    });
    assert_eq!(revisions, 100);
    assert_eq!(count, 0, "asking again allocated {count} times");
    // An edit is evaluated, once; asking again after it allocates nothing either.
    let object = doc
        .graph()
        .nodes()
        .find(|node| node.kind().id() == koebi_doc::kinds::path_object::KIND.id)
        .map(koebi_doc::Node::id)
        .expect("an object");
    doc.apply(Edit::SetInput {
        node: object,
        index: koebi_doc::kinds::path_object::LOCATION.index(),
        value: Value::Vector(Vector::new(5.0, 0.0)),
    })
    .expect("a location");
    let edited = evaluator.programs(&doc)[0].revision();
    assert_ne!(edited, revision);
    let (count, ()) = koebi_testkit::allocations(|| {
        for _ in 0..100 {
            assert_eq!(evaluator.programs(&doc)[0].revision(), edited);
        }
    });
    assert_eq!(count, 0, "asking again allocated {count} times");
}
