//! Find the first collision between a ray and a path.

use crate::path::PathEvent;
use crate::math::{Point, point, Vector, vector};
use crate::geom::{LineSegment, QuadraticBezierSegment, CubicBezierSegment, Line};
use std::f32;

pub struct Ray {
    pub origin: Point,
    pub direction: Vector,
}

// Position and normal at the point of contact between a ray and a shape.
pub struct Hit {
    pub position: Point,
    pub normal: Vector,
}

// TODO: early out in the bézier/arc cases using bounding rect or circle
// to speed things up.

/// Find the closest collision between a ray and the path.
pub fn raycast_path<Iter>(ray: &Ray, path: Iter, tolerance: f32) -> Option<Hit>
where
    Iter: Iterator<Item=PathEvent<Point, Point>>,
{
    let ray_len = ray.direction.square_length();
    if ray_len == 0.0 || ray_len.is_nan() {
        return None;
    }

    let mut state = RayCastInner {
        ray: Line {
            point: ray.origin,
            vector: ray.direction,
        },
        min_dot: f32::MAX,
        result: point(0.0, 0.0),
        normal: vector(0.0, 0.0),
    };

    for evt in path {
        match evt {
            PathEvent::Begin { .. } => {}
            PathEvent::Line { from, to } => {
                test_segment(&mut state, &LineSegment { from, to });
            }
            PathEvent::End { last, first, .. } => {
                test_segment(&mut state, &LineSegment { from: last, to: first });
            }
            PathEvent::Quadratic { from, ctrl, to } => {
                let mut prev = from;
                QuadraticBezierSegment { from, ctrl, to }.for_each_flattened(
                    tolerance,
                    &mut|p| {
                        test_segment(&mut state, &LineSegment { from: prev, to: p });
                        prev = p;
                    }
                );
            }
            PathEvent::Cubic { from, ctrl1, ctrl2, to } => {
                let mut prev = from;
                CubicBezierSegment { from, ctrl1, ctrl2, to }.for_each_flattened(
                    tolerance,
                    &mut|p| {
                        test_segment(&mut state, &LineSegment { from: prev, to: p });
                        prev = p;
                    }
                );
            }
        }
    }

    if state.min_dot == f32::MAX {
        return None;
    }

    if state.normal.dot(ray.direction) > 0.0 {
        state.normal = -state.normal;
    }

    Some(Hit {
        position: state.result,
        normal: state.normal.normalize(),
    })
}

struct RayCastInner {
    ray: Line<f32>,
    min_dot: f32,
    result: Point,
    normal: Vector,
}

fn test_segment(state: &mut RayCastInner, segment: &LineSegment<f32>) {
    if let Some(pos) = segment.line_intersection(&state.ray) {
        let dot = (pos - state.ray.point).dot(state.ray.vector);
        if dot >= 0.0 && dot < state.min_dot {
            state.min_dot = dot;
            state.result = pos;
            let v = segment.to_vector();
            state.normal = vector(-v.y, v.x);
        }
    }
}

#[test]
fn test_raycast() {
    use crate::geom::euclid::approxeq::ApproxEq;
    use crate::path::Path;

    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(0.0, 1.0));
    builder.close();
    let path = builder.build();

    assert!(
        raycast_path(
            &Ray { origin: point(-1.0, 2.0), direction: vector(1.0, 0.0) },
            path.iter(),
            0.1
        ).is_none()
    );

    let hit = raycast_path(
        &Ray { origin: point(-1.0, 0.5), direction: vector(1.0, 0.0) },
        path.iter(),
        0.1
    ).unwrap();
    assert!(hit.position.approx_eq(&point(0.0, 0.5)));
    assert!(hit.normal.approx_eq(&vector(-1.0, 0.0)));

    let hit = raycast_path(
        &Ray { origin: point(-1.0, 0.0), direction: vector(1.0, 0.0) },
        path.iter(),
        0.1
    ).unwrap();
    assert!(hit.position.approx_eq(&point(0.0, 0.0)));

    let hit = raycast_path(
        &Ray { origin: point(0.5, 0.5), direction: vector(1.0, 0.0) },
        path.iter(),
        0.1
    ).unwrap();
    assert!(hit.position.approx_eq(&point(1.0, 0.5)));
    assert!(hit.normal.approx_eq(&vector(-1.0, 0.0)));

    let hit = raycast_path(
        &Ray { origin: point(0.0, -1.0), direction: vector(1.0, 1.0) },
        path.iter(),
        0.1
    ).unwrap();
    assert!(hit.position.approx_eq(&point(1.0, 0.0)));
}
