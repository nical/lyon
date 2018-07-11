//! Bounding rectangle computation for paths.

use path::{PathEvent, QuadraticEvent, FlattenedEvent};
use math::{Point, point, vector, Rect};
use geom::{QuadraticBezierSegment, CubicBezierSegment, Arc};
use std::f32;

/// Computes a conservative axis-aligned rectangle that contains the path.
///
/// This bounding rectangle approximation is faster but less precise than
/// [`building_rect`](fn.bounding_rect.html).
pub fn fast_bounding_rect<Iter, Evt>(path: Iter) -> Rect
where
    Iter: Iterator<Item=Evt>,
    Evt: FastBoundingRect
{
    let mut min = point(f32::MAX, f32::MAX);
    let mut max = point(f32::MIN, f32::MIN);
    for e in path {
        e.min_max(&mut min, &mut max);
    }

    // Return an empty rectangle by default if there was no event in the path.
    if min == point(f32::MAX, f32::MAX) {
        return Rect::zero();
    }

    Rect {
        origin: min,
        size: (max - min).to_size(),
    }
}

#[doc(Hidden)]
pub trait FastBoundingRect {
    fn min_max(&self, min: &mut Point, max: &mut Point);
}

impl FastBoundingRect for PathEvent {
    fn min_max(&self, min: &mut Point, max: &mut Point) {
        match self {
            PathEvent::MoveTo(to) |
            PathEvent::LineTo(to) => {
                *min = Point::min(*min, *to);
                *max = Point::max(*max, *to);
            }
            PathEvent::QuadraticTo(ctrl, to) => {
                *min = Point::min(*min, Point::min(*ctrl, *to));
                *max = Point::max(*max, Point::max(*ctrl, *to));
            }
            PathEvent::CubicTo(ctrl1, ctrl2, to) => {
                *min = Point::min(*min, Point::min(*ctrl1, Point::min(*ctrl2, *to)));
                *max = Point::max(*max, Point::max(*ctrl1, Point::max(*ctrl2, *to)));
            }
            PathEvent::Arc(center, radii, _, _) => {
                let r = f32::max(f32::abs(radii.x), f32::abs(radii.y));
                *max = Point::max(*max, *center + vector(r, r));
                *min = Point::min(*min, *center - vector(r, r));
            }
            PathEvent::Close => {}
        }
    }
}

impl FastBoundingRect for QuadraticEvent {
    fn min_max(&self, min: &mut Point, max: &mut Point) {
        match self {
            QuadraticEvent::MoveTo(to) |
            QuadraticEvent::LineTo(to) => {
                *min = Point::min(*min, *to);
                *max = Point::max(*max, *to);
            }
            QuadraticEvent::QuadraticTo(ctrl, to) => {
                *min = Point::min(*min, Point::min(*ctrl, *to));
                *max = Point::max(*max, Point::max(*ctrl, *to));
            }
            QuadraticEvent::Close => {}
        }
    }
}

impl FastBoundingRect for FlattenedEvent {
    fn min_max(&self, min: &mut Point, max: &mut Point) {
        match self {
            FlattenedEvent::MoveTo(to) |
            FlattenedEvent::LineTo(to) => {
                *min = Point::min(*min, *to);
                *max = Point::max(*max, *to);
            }
            FlattenedEvent::Close => {}
        }
    }
}

/// Computes the smallest axis-aligned rectangle that contains the path.
pub fn bounding_rect<Iter, Evt>(path: Iter) -> Rect
where
    Iter: Iterator<Item=Evt>,
    Evt: TightBoundingRect,
{
    let mut current = point(0.0, 0.0);
    let mut first = point(0.0, 0.0);
    let mut min = point(f32::MAX, f32::MAX);
    let mut max = point(f32::MIN, f32::MIN);

    for evt in path {
        evt.min_max(&mut current, &mut first, &mut min, &mut max);
    }

    // Return an empty rectangle by default if there was no event in the path.
    if min == point(f32::MAX, f32::MAX) {
        return Rect::zero();
    }

    Rect {
        origin: min,
        size: (max - min).to_size(),
    }
}

#[doc(Hidden)]
pub trait TightBoundingRect {
    fn min_max(&self, current: &mut Point, first: &mut Point, min: &mut Point, max: &mut Point);
}

impl TightBoundingRect for PathEvent {
    fn min_max(&self, current: &mut Point, first: &mut Point, min: &mut Point, max: &mut Point) {
        match self {
            PathEvent::MoveTo(to) => {
                *min = Point::min(*min, *to);
                *max = Point::max(*max, *to);
                *current = *to;
                *first = *to;
            }
            PathEvent::LineTo(to) => {
                *min = Point::min(*min, *to);
                *max = Point::max(*max, *to);
                *current = *to;
            }
            PathEvent::QuadraticTo(ctrl, to) => {
                let r = QuadraticBezierSegment {
                    from: *current,
                    ctrl: *ctrl,
                    to: *to,
                }.bounding_rect();
                *min = Point::min(*min, r.origin);
                *max = Point::max(*max, r.bottom_right());
                *current = *to;
            }
            PathEvent::CubicTo(ctrl1, ctrl2, to) => {
                let r = CubicBezierSegment {
                    from: *current,
                    ctrl1: *ctrl1,
                    ctrl2: *ctrl2,
                    to: *to,
                }.bounding_rect();
                *min = Point::min(*min, r.origin);
                *max = Point::max(*max, r.bottom_right());
                *current = *to;
            }
            PathEvent::Arc(center, radii, sweep_angle, x_rotation) => {
                let start_angle = (*center - *current).angle_from_x_axis();
                let arc = Arc {
                    center: *center,
                    radii: *radii,
                    start_angle,
                    sweep_angle: *sweep_angle,
                    x_rotation: *x_rotation,
                };
                let r = arc.bounding_rect();
                *min = Point::min(*min, r.origin);
                *max = Point::max(*max, r.bottom_right());
                *current = arc.to();
            }
            PathEvent::Close => {
                *current = *first;
            }
        }
    }
}

impl TightBoundingRect for QuadraticEvent {
    fn min_max(&self, current: &mut Point, first: &mut Point, min: &mut Point, max: &mut Point) {
        match self {
            QuadraticEvent::MoveTo(to) => {
                *min = Point::min(*min, *to);
                *max = Point::max(*max, *to);
                *current = *to;
                *first = *to;
            }
            QuadraticEvent::LineTo(to) => {
                *min = Point::min(*min, *to);
                *max = Point::max(*max, *to);
                *current = *to;
            }
            QuadraticEvent::QuadraticTo(ctrl, to) => {
                let r = QuadraticBezierSegment {
                    from: *current,
                    ctrl: *ctrl,
                    to: *to,
                }.bounding_rect();
                *min = Point::min(*min, r.origin);
                *max = Point::max(*max, r.bottom_right());
                *current = *to;
            }
            QuadraticEvent::Close => {
                *current = *first;
            }
        }
    }
}

#[test]
fn simple_bounding_rect() {
    use path::default::Path;
    use path::builder::*;
    use math::{rect, Angle};

    let mut builder = Path::builder();
    builder.move_to(point(-10.0, -3.0));
    builder.line_to(point(0.0, -12.0));
    builder.quadratic_bezier_to(point(3.0, 4.0), point(5.0, 3.0));
    builder.close();
    let path = builder.build();

    assert_eq!(fast_bounding_rect(path.iter()), rect(-10.0, -12.0, 15.0, 16.0));

   let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.cubic_bezier_to(point(-1.0, 2.0), point(3.0, -4.0), point(1.0, -1.0));
    let path = builder.build();

    assert_eq!(fast_bounding_rect(path.iter()), rect(-1.0, -4.0, 4.0, 6.0));

   let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.arc(
        point(1.0, 0.0),
        vector(3.0, 4.0),
        Angle::degrees(45.0),
        Angle::degrees(90.0),
    );
    let path = builder.build();

    assert_eq!(fast_bounding_rect(path.iter()), rect(-3.0, -4.0, 8.0, 8.0));
}
