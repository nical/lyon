//! Bounding rectangle computation for paths.

use crate::path::{PathEvent, FlattenedEvent};
use crate::math::{Point, point, Rect};
use crate::geom::{QuadraticBezierSegment, CubicBezierSegment};
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

impl FastBoundingRect for PathEvent<Point, Point> {
    fn min_max(&self, min: &mut Point, max: &mut Point) {
        match self {
            PathEvent::Begin { at } => {
                *min = Point::min(*min, *at);
                *max = Point::max(*max, *at);
            }
            PathEvent::Line { to, .. } => {
                *min = Point::min(*min, *to);
                *max = Point::max(*max, *to);
            }
            PathEvent::Quadratic { ctrl, to, .. } => {
                *min = Point::min(*min, Point::min(*ctrl, *to));
                *max = Point::max(*max, Point::max(*ctrl, *to));
            }
            PathEvent::Cubic { ctrl1, ctrl2, to, .. } => {
                *min = Point::min(*min, Point::min(*ctrl1, Point::min(*ctrl2, *to)));
                *max = Point::max(*max, Point::max(*ctrl1, Point::max(*ctrl2, *to)));
            }
            PathEvent::End { .. } => {}
        }
    }
}

impl FastBoundingRect for FlattenedEvent<Point> {
    fn min_max(&self, min: &mut Point, max: &mut Point) {
        match self {
            FlattenedEvent::Begin { at } => {
                *min = Point::min(*min, *at);
                *max = Point::max(*max, *at);
            }
            FlattenedEvent::Line { to, .. } => {
                *min = Point::min(*min, *to);
                *max = Point::max(*max, *to);
            }
            FlattenedEvent::End { .. } => {}
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

impl TightBoundingRect for PathEvent<Point, Point> {
    fn min_max(&self, min: &mut Point, max: &mut Point) {
        match self {
            PathEvent::Begin { at } => {
                *min = Point::min(*min, *at);
                *max = Point::max(*max, *at);
            }
            PathEvent::Line { to, .. } => {
                *min = Point::min(*min, *to);
                *max = Point::max(*max, *to);
            }
            PathEvent::Quadratic { from, ctrl, to } => {
                let r = QuadraticBezierSegment { from: *from, ctrl: *ctrl, to: *to }.bounding_rect();
                *min = Point::min(*min, r.min());
                *max = Point::max(*max, r.max());
            }
            PathEvent::Cubic { from, ctrl1, ctrl2, to } => {
                let r = CubicBezierSegment { from: *from, ctrl1: *ctrl1, ctrl2: *ctrl2, to: *to }.bounding_rect();
                *min = Point::min(*min, r.min());
                *max = Point::max(*max, r.max());
            }
            PathEvent:: End { .. } => {}
        }
    }
}

#[test]
fn simple_bounding_rect() {
    use crate::path::Path;
    use crate::math::rect;

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
