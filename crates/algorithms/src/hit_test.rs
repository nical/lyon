//! Determine whether a point is inside a path.

use crate::geom::{CubicBezierSegment, LineSegment, QuadraticBezierSegment};
use crate::math::Point;
use crate::path::{FillRule, PathEvent};

/// Returns whether the point is inside the path.
pub fn hit_test_path<Iter>(point: &Point, path: Iter, fill_rule: FillRule, tolerance: f32) -> bool
where
    Iter: IntoIterator<Item = PathEvent>,
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
    Iter: IntoIterator<Item = PathEvent>,
{
    // Loop over the edges and compute the winding number at that point by accumulating the
    // winding of all edges intersecting the horizontal line passing through our point which are
    // left of it.
    let mut winding = 0;

    for evt in path {
        match evt {
            PathEvent::Begin { .. } => {}
            PathEvent::Line { from, to } => {
                test_segment(
                    *point,
                    &LineSegment { from, to },
                    &mut winding,
                );
            }
            PathEvent::End { last, first, .. } => {
                test_segment(
                    *point,
                    &LineSegment {
                        from: last,
                        to: first,
                    },
                    &mut winding,
                );
            }
            PathEvent::Quadratic { from, ctrl, to } => {
                let segment = QuadraticBezierSegment { from, ctrl, to };
                let (min, max) = segment.fast_bounding_range_y();
                if min > point.y || max < point.y {
                    continue;
                }
                segment.for_each_flattened(tolerance, &mut |line| {
                    test_segment(*point, line, &mut winding);
                });
            }
            PathEvent::Cubic {
                from,
                ctrl1,
                ctrl2,
                to,
            } => {
                let segment = CubicBezierSegment {
                    from,
                    ctrl1,
                    ctrl2,
                    to,
                };
                let (min, max) = segment.fast_bounding_range_y();
                if min > point.y || max < point.y {
                    continue;
                }
                segment.for_each_flattened(tolerance, &mut |line| {
                    test_segment(*point, line, &mut winding);
                });
            }
        }
    }

    winding
}

fn test_segment(
    point: Point,
    segment: &LineSegment<f32>,
    winding: &mut i32,
) {
    let y0 = segment.from.y;
    let y1 = segment.to.y;
    let min_y = f32::min(y0, y1);
    let max_y = f32::max(y0, y1);

    if min_y > point.y
        || max_y <= point.y
        || f32::min(segment.from.x, segment.to.x) > point.x
        || y0 == y1
    {
        return;
    }

    let side = (segment.to.x - segment.from.x) * (point.y - segment.from.y)
        - (point.x - segment.from.x) * (segment.to.y - segment.from.y);

    let positive_winding = y1 - y0 >= 0.0;

    let is_left = if positive_winding {
        side > 0.0
    } else {
        side < 0.0
    };

    if is_left {
        return;
    }

    *winding += if positive_winding { 1 } else { -1 };
}

#[test]
fn test_hit_test() {
    use crate::math::point;
    use crate::path::Path;

    let mut builder = Path::builder();
    builder.begin(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(0.0, 1.0));
    builder.end(true);
    builder.begin(point(0.25, 0.25));
    builder.line_to(point(0.75, 0.25));
    builder.line_to(point(0.75, 0.75));
    builder.line_to(point(0.20, 0.75));
    builder.end(true);
    let path = builder.build();

    assert!(!hit_test_path(
        &point(-1.0, 0.5),
        path.iter(),
        FillRule::EvenOdd,
        0.1
    ));
    assert!(!hit_test_path(
        &point(2.0, 0.5),
        path.iter(),
        FillRule::EvenOdd,
        0.1
    ));
    std::println!(
        "winding {:?}",
        path_winding_number_at_position(&point(2.0, 0.0), path.iter(), 0.1)
    );
    assert!(!hit_test_path(
        &point(2.0, 0.0),
        path.iter(),
        FillRule::EvenOdd,
        0.1
    ));
    assert!(!hit_test_path(
        &point(0.5, -1.0),
        path.iter(),
        FillRule::EvenOdd,
        0.1
    ));
    assert!(!hit_test_path(
        &point(0.5, 2.0),
        path.iter(),
        FillRule::EvenOdd,
        0.1
    ));

    assert!(!hit_test_path(
        &point(0.5, 0.5),
        path.iter(),
        FillRule::EvenOdd,
        0.1
    ));
    assert!(hit_test_path(
        &point(0.5, 0.5),
        path.iter(),
        FillRule::NonZero,
        0.1
    ));
    assert!(hit_test_path(
        &point(0.2, 0.5),
        path.iter(),
        FillRule::EvenOdd,
        0.1
    ));
    assert!(hit_test_path(
        &point(0.8, 0.5),
        path.iter(),
        FillRule::EvenOdd,
        0.1
    ));
}

#[test]
fn hit_test_point_aligned() {
    use crate::math::point;
    use crate::path::polygon::Polygon;

    let poly = Polygon {
        points: &[
            point(-10.0, 10.0),
            point(10.0, 10.0),
            point(10.0, 5.0),
            point(10.0, -10.0),
            point(-10.0, -10.0),
        ],
        closed: true,
    };

    assert!(hit_test_path(
        &point(0.0, 5.0),
        poly.path_events(),
        FillRule::NonZero,
        0.1
    ));
    assert!(!hit_test_path(
        &point(15.0, 5.0),
        poly.path_events(),
        FillRule::NonZero,
        0.1
    ));
}

#[test]
fn hit_test_double_square() {
    use crate::math::point;
    use crate::path::Path;

    let mut builder = Path::builder();
    builder.begin(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(0.0, 1.0));
    builder.line_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(0.0, 1.0));
    builder.end(true);
    let path = builder.build();

    assert_eq!(
        path_winding_number_at_position(&point(0.5, 0.5), &path, 0.1),
        -2
    );
}

#[test]
fn hit_test_double_count() {
    use crate::math::point;
    use crate::path::Path;

    let mut builder = Path::builder();
    builder.begin(point(0.0, 0.0));
    builder.line_to(point(0.0, 1.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(1.0, 2.0));
    builder.line_to(point(1.0, 3.0));
    builder.line_to(point(3.0, 3.0));
    builder.line_to(point(3.0, 0.0));
    builder.end(true);
    let path = builder.build();

    assert_eq!(
        path_winding_number_at_position(&point(2.0, 1.0), &path, 0.1),
        1
    );
    assert_eq!(
        path_winding_number_at_position(&point(2.0, 2.0), &path, 0.1),
        1
    );
}

#[test]
fn issue_882() {
    use crate::math::point;
    use crate::path::Path;
    let mut pb = Path::builder();
    pb.begin(point(0.0, 50.0));
    pb.line_to(point(50.0, 50.0));
    pb.line_to(point(50.0, 0.0));
    pb.line_to(point(100.0, 0.0));
    pb.line_to(point(100.0, 100.0));
    pb.line_to(point(0.0, 100.0));
    pb.line_to(point(0.0, 50.0));
    pb.end(true);
    let p = pb.build();

    let x = point(55.0, 50.0);

    assert!(hit_test_path(&x, p.iter(), FillRule::EvenOdd, 1.0))
}

#[cfg(test)]
fn diamond(reversed: bool) -> crate::path::Path {
    use crate::math::point;
    use crate::path::Path;

    let mut points = [
        point(0.0, -1.0),
        point(1.0, 0.0),
        point(0.0, 1.0),
        point(-1.0, 0.0),
    ];
    if reversed {
        points.reverse();
    }

    let mut builder = Path::builder();
    builder.begin(points[0]);
    for p in &points[1..] {
        builder.line_to(*p);
    }
    builder.end(true);
    builder.build()
}

#[test]
fn hit_test_slanted_edges() {
    use crate::math::point;

    for (reversed, inside_winding) in [(false, -1), (true, 1)] {
        let path = diamond(reversed);
        let winding = |x: f32, y: f32| path_winding_number_at_position(&point(x, y), &path, 0.1);

        // Just inside and just outside each of the four slanted edges.
        assert_eq!(winding(0.4, 0.5), inside_winding);
        assert_eq!(winding(0.6, 0.5), 0);
        assert_eq!(winding(-0.4, 0.5), inside_winding);
        assert_eq!(winding(-0.6, 0.5), 0);
        assert_eq!(winding(0.4, -0.5), inside_winding);
        assert_eq!(winding(0.6, -0.5), 0);
        assert_eq!(winding(-0.4, -0.5), inside_winding);
        assert_eq!(winding(-0.6, -0.5), 0);

        // Left of the shape: all edges crossing the horizontal line are on the
        // right of the point and must be skipped individually.
        assert_eq!(winding(-0.95, 0.1), 0);
        assert_eq!(winding(-0.95, -0.1), 0);
        assert_eq!(winding(-5.0, 0.5), 0);
        assert_eq!(winding(-5.0, -0.5), 0);
    }
}

#[test]
fn hit_test_point_on_slanted_edge() {
    use crate::math::point;

    // An edge passing exactly through the point counts as being left of it,
    // so points on the left boundary are inside and points on the right
    // boundary are outside.
    for (reversed, inside_winding) in [(false, -1), (true, 1)] {
        let path = diamond(reversed);
        let winding = |x: f32, y: f32| path_winding_number_at_position(&point(x, y), &path, 0.1);

        assert_eq!(winding(-0.5, 0.5), inside_winding);
        assert_eq!(winding(-0.5, -0.5), inside_winding);
        assert_eq!(winding(0.5, 0.5), 0);
        assert_eq!(winding(0.5, -0.5), 0);
    }
}

#[test]
fn hit_test_quadratic() {
    use crate::math::point;
    use crate::path::Path;

    // At y = 2.5 the curve spans x in [~1.46, ~8.54].
    let mut builder = Path::builder();
    builder.begin(point(0.0, 0.0));
    builder.quadratic_bezier_to(point(5.0, 10.0), point(10.0, 0.0));
    builder.end(true);
    let path = builder.build();

    let hit = |x: f32, y: f32| hit_test_path(&point(x, y), path.iter(), FillRule::NonZero, 0.01);

    assert!(hit(5.0, 2.5));
    assert!(hit(2.0, 2.5));
    assert!(hit(8.0, 2.5));
    assert!(hit(5.0, 4.5));
    assert!(!hit(1.0, 2.5));
    assert!(!hit(9.0, 2.5));
    assert!(!hit(-5.0, 2.5));
    assert!(!hit(15.0, 2.5));
    assert!(!hit(5.0, -1.0));
    // Below the control point but above the curve.
    assert!(!hit(5.0, 6.0));
}

#[test]
fn hit_test_cubic_circle() {
    use crate::math::point;
    use crate::path::Path;

    // Circle of radius 10 centered on the origin, approximated with 4 cubics.
    let r = 10.0;
    let k = 0.5522848 * r;
    let mut builder = Path::builder();
    builder.begin(point(r, 0.0));
    builder.cubic_bezier_to(point(r, k), point(k, r), point(0.0, r));
    builder.cubic_bezier_to(point(-k, r), point(-r, k), point(-r, 0.0));
    builder.cubic_bezier_to(point(-r, -k), point(-k, -r), point(0.0, -r));
    builder.cubic_bezier_to(point(k, -r), point(r, -k), point(r, 0.0));
    builder.end(true);
    let path = builder.build();

    let hit = |x: f32, y: f32| hit_test_path(&point(x, y), path.iter(), FillRule::NonZero, 0.01);

    for (x, y) in [(0.0, 0.0), (6.0, 6.0), (-6.0, 6.0), (6.0, -6.0), (-6.0, -6.0), (9.5, 1.0), (-9.5, -1.0)] {
        assert!(hit(x, y), "({}, {}) should be inside", x, y);
    }
    for (x, y) in [(7.5, 7.5), (-7.5, 7.5), (7.5, -7.5), (-7.5, -7.5), (11.0, 1.0), (-11.0, 1.0), (0.0, 11.0), (0.0, -11.0)] {
        assert!(!hit(x, y), "({}, {}) should be outside", x, y);
    }
}
