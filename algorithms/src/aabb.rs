//! Bounding rectangle computation for paths.

use path::{PathEvent, QuadraticEvent, FlattenedEvent};
use math::{Point, point, Rect};
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
            PathEvent::MoveTo(to) => {
                *min = Point::min(*min, *to);
                *max = Point::max(*max, *to);
            }
            PathEvent::Line(ref segment) => {
                *min = Point::min(*min, segment.to);
                *max = Point::max(*max, segment.to);
            }
            PathEvent::Quadratic(ref segment) => {
                *min = Point::min(*min, Point::min(segment.ctrl, segment.to));
                *max = Point::max(*max, Point::max(segment.ctrl, segment.to));
            }
            PathEvent::Cubic(ref segment) => {
                *min = Point::min(*min, Point::min(segment.ctrl1, Point::min(segment.ctrl2, segment.to)));
                *max = Point::max(*max, Point::max(segment.ctrl1, Point::max(segment.ctrl2, segment.to)));
            }
            PathEvent::Close(..) => {}
        }
    }
}

impl FastBoundingRect for QuadraticEvent {
    fn min_max(&self, min: &mut Point, max: &mut Point) {
        match self {
            QuadraticEvent::MoveTo(to) => {
                *min = Point::min(*min, *to);
                *max = Point::max(*max, *to);
            }
            QuadraticEvent::Line(ref segment) => {
                *min = Point::min(*min, segment.to);
                *max = Point::max(*max, segment.to);
            }
            QuadraticEvent::Quadratic(ref segment) => {
                *min = Point::min(*min, Point::min(segment.ctrl, segment.to));
                *max = Point::max(*max, Point::max(segment.ctrl, segment.to));
            }
            QuadraticEvent::Close(..) => {}
        }
    }
}

impl FastBoundingRect for FlattenedEvent {
    fn min_max(&self, min: &mut Point, max: &mut Point) {
        match self {
            FlattenedEvent::MoveTo(to) => {
                *min = Point::min(*min, *to);
                *max = Point::max(*max, *to);
            }
            FlattenedEvent::Line(segment) => {
                *min = Point::min(*min, segment.to);
                *max = Point::max(*max, segment.to);
            }
            FlattenedEvent::Close(..) => {}
        }
    }
}

/// Computes the smallest axis-aligned rectangle that contains the path.
pub fn bounding_rect<Iter, Evt>(path: Iter) -> Rect
where
    Iter: Iterator<Item=Evt>,
    Evt: TightBoundingRect,
{
    let mut min = point(f32::MAX, f32::MAX);
    let mut max = point(f32::MIN, f32::MIN);

    for evt in path {
        evt.min_max(&mut min, &mut max);
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
    fn min_max(&self, min: &mut Point, max: &mut Point);
}

impl TightBoundingRect for PathEvent {
    fn min_max(&self, min: &mut Point, max: &mut Point) {
        match self {
            PathEvent::MoveTo(to) => {
                *min = Point::min(*min, *to);
                *max = Point::max(*max, *to);
            }
            PathEvent::Line(ref segment) => {
                *min = Point::min(*min, segment.to);
                *max = Point::max(*max, segment.to);
            }
            PathEvent::Quadratic(ref segment) => {
                let r = segment.bounding_rect();
                *min = Point::min(*min, r.origin);
                *max = Point::max(*max, r.bottom_right());
            }
            PathEvent::Cubic(ref segment) => {
                let r = segment.bounding_rect();
                *min = Point::min(*min, r.origin);
                *max = Point::max(*max, r.bottom_right());
            }
            PathEvent::Close(..) => {}
        }
    }
}

impl TightBoundingRect for QuadraticEvent {
    fn min_max(&self, min: &mut Point, max: &mut Point) {
        match self {
            QuadraticEvent::MoveTo(to) => {
                *min = Point::min(*min, *to);
                *max = Point::max(*max, *to);
            }
            QuadraticEvent::Line(segment) => {
                *min = Point::min(*min, segment.to);
                *max = Point::max(*max, segment.to);
            }
            QuadraticEvent::Quadratic(ref segment) => {
                let r = segment.bounding_rect();
                *min = Point::min(*min, r.origin);
                *max = Point::max(*max, r.bottom_right());
            }
            QuadraticEvent::Close(..) => {}
        }
    }
}

#[test]
fn simple_bounding_rect() {
    use path::Path;
    use math::rect;

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
}
