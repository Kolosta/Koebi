//! Queries on contours run on hot paths (picking under the pointer, snapping every frame of a
//! drag): they never allocate (CLAUDE.md, *Hot paths*; ADR 0026 counts it).

use koebi_geom::{FillRule, Path, Point, Vector};

#[global_allocator]
static ALLOCATOR: koebi_testkit::CountingAllocator = koebi_testkit::CountingAllocator;

#[test]
fn hit_tests_measures_and_evaluation_allocate_nothing() {
    let mut path = Path::exact_ellipse(Point::new(0.0, 0.0), Vector::new(40.0, 25.0));
    path.subpaths
        .extend(Path::ellipse(Point::new(100.0, 0.0), Vector::new(30.0, 30.0)).subpaths);
    let contours = path.contours();
    let (count, inside) = koebi_testkit::allocations(|| {
        let mut inside = 0;
        for i in 0..200 {
            let point = Point::new(f64::from(i) - 50.0, 3.0);
            inside += usize::from(contours.contains(point, FillRule::NonZero));
            inside += usize::from(contours.contains(point, FillRule::EvenOdd));
            let _ = contours.nearest(point);
        }
        let _ = contours.bounds();
        for contour in contours.contours() {
            let _ = contour.point_at_length(10.0);
            for segment in contour.segments() {
                let _ = segment.eval(0.3);
                let _ = segment.derivative(0.7);
                let _ = segment.bounds();
            }
        }
        inside
    });
    assert_eq!(count, 0);
    assert!(inside > 0);
}
