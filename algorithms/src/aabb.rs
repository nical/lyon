use path::{PathEvent, QuadraticEvent, FlattenedEvent};
use std::f32;
use math::{Point, point, vector, Rect};

/// Computes a conservative axis-aligned rectangle that contains the path.
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
                let rmax = f32::max(f32::abs(radii.x), f32::abs(radii.y));
                let rmin = f32::min(f32::abs(radii.x), f32::abs(radii.y));
                *max = Point::max(*max, *center - vector(rmax, rmax));
                *min = Point::min(*min, *center - vector(rmin, rmin));
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
