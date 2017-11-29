use math::{Point, point, Vector, Rect, Size, Transform2D};
use segment::{Segment, FlatteningStep, BoundingRect};
use utils::min_max;

// TODO: Perhaps it would be better to have LineSegment<T> where T can be f32, f64
// or some fixed precision number (See comment in the intersection function).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LineSegment {
    pub from: Point,
    pub to: Point,
}

impl LineSegment {
    /// Sample the segment at t (expecting t between 0 and 1).
    #[inline]
    pub fn sample(&self, t: f32) -> Point {
        self.from.lerp(self.to, t)
    }

    /// Sample the x coordinate of the segment at t (expecting t between 0 and 1).
    #[inline]
    pub fn x(&self, t: f32) -> f32 {
        self.from.x * (1.0 - t) + self.to.x * t
    }

    /// Sample the y coordinate of the segment at t (expecting t between 0 and 1).
    #[inline]
    pub fn y(&self, t: f32) -> f32 {
        self.from.y * (1.0 - t) + self.to.y * t
    }

    /// Returns an inverted version of this segment where the beginning and the end
    /// points are swapped.
    #[inline]
    pub fn flip(&self) -> Self {
        LineSegment { from: self.to, to: self.from }
    }

    /// Split this curve into two sub-segments.
    #[inline]
    pub fn split(&self, t: f32) -> (Self, Self) {
        let split_point = self.sample(t);
        return (
            LineSegment { from: self.from, to: split_point },
            LineSegment { from: split_point, to: self.to },
        );
    }

    /// Return the segment before the split point.
    #[inline]
    pub fn before_split(&self, t: f32) -> Self {
        LineSegment { from: self.from, to: self.sample(t) }
    }

    /// Return the segment after the split point.
    #[inline]
    pub fn after_split(&self, t: f32) -> Self {
        LineSegment { from: self.sample(t), to: self.to }
    }

    /// Return the minimum bounding rectangle
    #[inline]
    pub fn bounding_rect(&self) -> Rect {
        let (min_x, max_x) = self.bounding_range_x();
        let (min_y, max_y) = self.bounding_range_y();

        let width  = max_x - min_x;
        let height = max_y - min_y;
        Rect::new(Point::new(min_x, min_y), Size::new(width, height))
    }

    #[inline]
    fn bounding_range_x(&self) -> (f32, f32) {
        min_max(self.from.x, self.to.x)
    }

    #[inline]
    fn bounding_range_y(&self) -> (f32, f32) {
        min_max(self.from.y, self.to.y)
    }

    /// Returns the vector between this segment's `from` and `to` points.
    #[inline]
    pub fn to_vector(&self) -> Vector {
        self.to - self.from
    }

    /// Returns the line containing this segment.
    #[inline]
    pub fn to_line(&self) -> Line {
        Line {
            point: self.from,
            vector: self.to - self.from,
        }
    }

    /// Computes the length of this segment.
    #[inline]
    pub fn length(&self) -> f32 {
        self.to_vector().length()
    }

    #[inline]
    pub fn translate(&mut self, by: Vector) -> Self {
        LineSegment {
            from: self.from + by,
            to: self.to + by,
        }
    }

    /// Applies the transform to this segment and returns the results.
    #[inline]
    pub fn transform(&self, transform: &Transform2D) -> Self {
        LineSegment {
            from: transform.transform_point(&self.from),
            to: transform.transform_point(&self.to),
        }
    }

    /// Computes the intersection (if any) between this segment and another one.
    ///
    /// The result is provided in the form of the `t` parameter of each
    /// segment. To get the intersection point, sample one of the segments
    /// at the corresponding value.
    pub fn intersection(&self, other: &Self) -> Option<(f32, f32)> {
        let (min1, max1) = self.bounding_range_x();
        let (min2, max2) = other.bounding_range_x();
        if min1 > max2 || max1 < min2 {
            return None;
        }

        let v1 = self.to_vector().to_f64();
        let v2 = other.to_vector().to_f64();

        let v1_cross_v2 = v1.cross(v2);

        if v1_cross_v2 == 0.0 {
            // The segments are parallel
            return None;
        }

        let sign_v1_cross_v2 = v1_cross_v2.signum();
        let abs_v1_cross_v2 = f64::abs(v1_cross_v2);

        let v3 = (other.from - self.from).to_f64();

        // t and u should be divided by v1_cross_v2, but we postpone that to not lose precision.
        // We have to respect the sign of v1_cross_v2 (and therefore t and u) so we apply it now and
        // will use the absolute value of v1_cross_v2 afterwards.
        let t = v3.cross(v2) * sign_v1_cross_v2;
        let u = v3.cross(v1) * sign_v1_cross_v2;

        if t <= 0.0 || t >= abs_v1_cross_v2 || u <= 0.0 || u >= abs_v1_cross_v2 {
            return None;
        }

        Some((
            (t / abs_v1_cross_v2) as f32,
            (u / abs_v1_cross_v2) as f32,
        ))
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.intersection(other).is_some()
    }
}

impl Segment for LineSegment {
    fn from(&self) -> Point { self.from }
    fn to(&self) -> Point { self.to }
    fn sample(&self, t: f32) -> Point { self.sample(t) }
    fn x(&self, t: f32) -> f32 { self.x(t) }
    fn y(&self, t: f32) -> f32 { self.y(t) }
    fn derivative(&self, _t: f32) -> Vector { self.to_vector() }
    fn dx(&self, _t: f32) -> f32 { self.to.x - self.from.x }
    fn dy(&self, _t: f32) -> f32 { self.to.y - self.from.y }
    fn split(&self, t: f32) -> (Self, Self) { self.split(t) }
    fn before_split(&self, t: f32) -> Self { self.before_split(t) }
    fn after_split(&self, t: f32) -> Self { self.after_split(t) }
    fn flip(&self) -> Self { self.flip() }
    fn approximate_length(&self, _tolerance: f32) -> f32 { self.length() }
}

impl BoundingRect for LineSegment {
    fn bounding_rect(&self) -> Rect { self.bounding_rect() }
    fn fast_bounding_rect(&self) -> Rect { self.bounding_rect() }
    fn bounding_range_x(&self) -> (f32, f32) { self.bounding_range_x() }
    fn bounding_range_y(&self) -> (f32, f32) { self.bounding_range_y() }
    fn fast_bounding_range_x(&self) -> (f32, f32) { self.bounding_range_x() }
    fn fast_bounding_range_y(&self) -> (f32, f32) { self.bounding_range_y() }
}

impl FlatteningStep for LineSegment {
    fn flattening_step(&self, _tolerance: f32) -> f32 { 1.0 }
}

// TODO: we could implement this more efficiently with specialization
// impl FlattenedForEach for LineSegment {
//     fn flattened_for_each<F: FnMut(Point)>(&self, _tolerance: f32, call_back: &mut F) {
//         call_back(self.to);
//     }
// }

#[derive(Copy, Clone, Debug)]
pub struct Line {
    pub point: Point,
    pub vector: Vector,
}

impl Line {
    pub fn intersection(&self, other: &Self) -> Option<Point> {
        let epsilon = 0.000001;
        let det = self.vector.cross(other.vector);
        if det.abs() <= epsilon {
            // The lines are very close to parallel
            return None;
        }
        let inv_det = 1.0 / det;
        let self_p2 = self.point + self.vector;
        let other_p2 = other.point + other.vector;
        let a = self.point.to_vector().cross(self_p2.to_vector());
        let b = other.point.to_vector().cross(other_p2.to_vector());
        return Some(
            point(
                (b * self.vector.x - a * other.vector.x) * inv_det,
                (b * self.vector.y - a * other.vector.y) * inv_det,
            )
        );
    }
}

#[cfg(test)]
fn fuzzy_eq_f32(a: f32, b: f32, epsilon: f32) -> bool {
    return (a - b).abs() <= epsilon;
}

#[cfg(test)]
fn fuzzy_eq_vector(a: Vector, b: Vector, epsilon: f32) -> bool {
    fuzzy_eq_f32(a.x, b.x, epsilon) && fuzzy_eq_f32(a.y, b.y, epsilon)
}

#[cfg(test)]
fn fuzzy_eq_point(a: Point, b: Point, epsilon: f32) -> bool {
    fuzzy_eq_vector(a.to_vector(), b.to_vector(), epsilon)
}

#[test]
fn intersection_rotated() {
    use std::f32::consts::PI;
    let epsilon = 0.0001;
    let count: u32 = 100;

    for i in 0..count {
        for j in 0..count {
            if i % (count / 2) == j % (count / 2) {
                // avoid the colinear case.
                continue;
            }

            let angle1 = i as f32 / (count as f32) * 2.0 * PI;
            let angle2 = j as f32 / (count as f32) * 2.0 * PI;

            let l1 = LineSegment {
                from: point(10.0 * angle1.cos(), 10.0 * angle1.sin()),
                to: point(-10.0 * angle1.cos(), -10.0 * angle1.sin()),
            };

            let l2 = LineSegment {
                from: point(10.0 * angle2.cos(), 10.0 * angle2.sin()),
                to: point(-10.0 * angle2.cos(), -10.0 * angle2.sin()),
            };

            assert!(l1.intersects(&l2));

            assert!(
                fuzzy_eq_point(
                    l1.sample(l1.intersection(&l2).unwrap().0),
                    point(0.0, 0.0),
                    epsilon
                )
            );

            assert!(
                fuzzy_eq_point(
                    l2.sample(l1.intersection(&l2).unwrap().1),
                    point(0.0, 0.0),
                    epsilon
                )
            );
        }
    }
}

#[test]
fn intersection_touching() {
    let l1 = LineSegment {
        from: point(0.0, 0.0),
        to: point(10.0, 10.0),
    };

    let l2 = LineSegment {
        from: point(10.0, 10.0),
        to: point(10.0, 0.0),
    };

    assert!(!l1.intersects(&l2));
    assert!(l1.intersection(&l2).is_none());
}

#[test]
fn intersection_overlap() {
    // It's hard to define the intersection points of two segments that overlap,
    // (would be a region rather than a point) and more importanly, in practice
    // the algorithms in lyon don't need to consider this special case as an intersection,
    // so we choose to treat overlapping segments as not intersecting.

    let l1 = LineSegment {
        from: point(0.0, 0.0),
        to: point(10.0, 0.0),
    };

    let l2 = LineSegment {
        from: point(5.0, 00.0),
        to: point(15.0, 0.0),
    };

    assert!(!l1.intersects(&l2));
    assert!(l1.intersection(&l2).is_none());
}

#[cfg(test)]
use euclid::rect;

#[test]
fn bounding_rect() {
    let l1 = LineSegment {
        from: point(1., 5.),
        to: point(5., 7.),
    };
    let r1 = rect(1., 5., 4., 2.);

    let l2 = LineSegment {
        from: point(5., 5.),
        to: point(1., 1.),
    };
    let r2 = rect(1., 1., 4., 4.);

    let l3 = LineSegment {
        from: point(3., 3.),
        to: point(1., 5.),
    };
    let r3 = rect(1., 3., 2., 2.);

    let cases = vec![(l1, r1), (l2, r2), (l3, r3)];
    for &(ls, r) in &cases {
        assert_eq!(ls.bounding_rect(), r);
    }
}
