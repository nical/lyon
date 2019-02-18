//! Determine whether a point is inside a path.

use crate::path::{PathEvent, FillRule};
use crate::math::Point;
use crate::geom::{LineSegment, QuadraticBezierSegment, CubicBezierSegment};
use std::f32;

/// Returns whether the point is inside the path.
pub fn hit_test_path<Iter>(point: &Point, path: Iter, fill_rule: FillRule, tolerance: f32) -> bool
where
    Iter: Iterator<Item=PathEvent<Point, Point>>,
{
    let winding = path_winding_number_at_position(point, path, tolerance);

    match fill_rule {
        FillRule::EvenOdd => winding % 2 != 0,
        FillRule::NonZero => winding != 0,
    }
}

/// Compute the winding number of a given position with respect to the path.
pub fn path_winding_number_at_position<Iter>(point: &Point, path: Iter, tolerance: f32) -> i32
where
    Iter: Iterator<Item=PathEvent<Point, Point>>,
{
    // Loop over the edges and compute the winding number at that point by accumulating the
    // winding of all edges intersecting the horizontal line passing through our point which are
    // left of it.
    let mut winding = 0;

    for evt in path {
        match evt {
            PathEvent::Begin { .. } => {}
            PathEvent::Line { from, to } => {
                test_segment(*point, &LineSegment { from, to }, &mut winding);
            }
            PathEvent::End { last, first, .. } => {
                test_segment(*point, &LineSegment { from: last, to: first }, &mut winding);
            }
            PathEvent::Quadratic { from, ctrl, to } => {
                let segment = QuadraticBezierSegment { from, ctrl, to };
                let (min, max) = segment.fast_bounding_range_y();
                if min > point.y || max < point.y {
                    continue;
                }
                let mut prev = segment.from;
                segment.for_each_flattened(tolerance, &mut|p| {
                    test_segment(*point, &LineSegment { from: prev, to: p }, &mut winding);
                    prev = p;
                });
            }
            PathEvent::Cubic { from, ctrl1, ctrl2, to } => {
                let segment = CubicBezierSegment { from, ctrl1, ctrl2, to };
                let (min, max) = segment.fast_bounding_range_y();
                if min > point.y || max < point.y {
                    continue;
                }
                let mut prev = segment.from;
                segment.for_each_flattened(tolerance, &mut|p| {
                    test_segment(*point, &LineSegment { from: prev, to: p }, &mut winding);
                    prev = p;
                });
            }
        }
    }

    winding
}

fn test_segment(point: Point, segment: &LineSegment<f32>, winding: &mut i32) {
    if let Some(pos) = segment.horizontal_line_intersection(point.y) {
        if pos.x < point.x {
            if segment.to.y > segment.from.y {
                *winding += 1;
            } else if segment.to.y < segment.from.y {
                *winding -= 1;
            }
        }
    }
}

#[test]
fn test_hit_test() {
    use crate::path::Path;
    use crate::math::point;

    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(0.0, 1.0));
    builder.close();
    builder.move_to(point(0.25, 0.25));
    builder.line_to(point(0.75, 0.25));
    builder.line_to(point(0.75, 0.75));
    builder.line_to(point(0.20, 0.75));
    builder.close();
    let path = builder.build();

    assert!(!hit_test_path(&point(-1.0, 0.5), path.iter(), FillRule::EvenOdd, 0.1));
    assert!(!hit_test_path(&point(2.0, 0.5), path.iter(), FillRule::EvenOdd, 0.1));
    assert!(!hit_test_path(&point(2.0, 0.0), path.iter(), FillRule::EvenOdd, 0.1));
    assert!(!hit_test_path(&point(0.5, -1.0), path.iter(), FillRule::EvenOdd, 0.1));
    assert!(!hit_test_path(&point(0.5, 2.0), path.iter(), FillRule::EvenOdd, 0.1));

    assert!(!hit_test_path(&point(0.5, 0.5), path.iter(), FillRule::EvenOdd, 0.1));
    assert!(hit_test_path(&point(0.5, 0.5), path.iter(), FillRule::NonZero, 0.1));
    assert!(hit_test_path(&point(0.2, 0.5), path.iter(), FillRule::EvenOdd, 0.1));
    assert!(hit_test_path(&point(0.8, 0.5), path.iter(), FillRule::EvenOdd, 0.1));
}
