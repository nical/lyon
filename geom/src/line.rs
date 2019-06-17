use crate::scalar::Scalar;
use crate::generic_math::{Point, point, Vector, vector, Rect, Size, Transform2D};
use crate::segment::{Segment, FlatteningStep, BoundingRect};
use crate::monotonic::MonotonicSegment;
use crate::utils::min_max;
use std::mem::swap;

use std::ops::Range;

/// A linear segment.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct LineSegment<S> {
    pub from: Point<S>,
    pub to: Point<S>,
}

impl<S: Scalar> LineSegment<S> {
    /// Sample the segment at t (expecting t between 0 and 1).
    #[inline]
    pub fn sample(&self, t: S) -> Point<S> {
        self.from.lerp(self.to, t)
    }

    /// Sample the x coordinate of the segment at t (expecting t between 0 and 1).
    #[inline]
    pub fn x(&self, t: S) -> S {
        self.from.x * (S::ONE - t) + self.to.x * t
    }

    /// Sample the y coordinate of the segment at t (expecting t between 0 and 1).
    #[inline]
    pub fn y(&self, t: S) -> S {
        self.from.y * (S::ONE - t) + self.to.y * t
    }

    #[inline]
    pub fn from(&self) -> Point<S> { self.from }

    #[inline]
    pub fn to(&self) -> Point<S> { self.to }

    pub fn solve_t_for_x(&self, x: S) -> S {
        let dx = self.to.x - self.from.x;
        if dx == S::ZERO {
            return S::ZERO;
        }

        (x - self.from.x) / dx
    }

    pub fn solve_t_for_y(&self, y: S) -> S {
        let dy = self.to.y - self.from.y;
        if dy == S::ZERO {
            return S::ZERO
        }

        (y - self.from.y) / dy
    }

    pub fn solve_y_for_x(&self, x: S) -> S {
        self.y(self.solve_t_for_x(x))
    }

    pub fn solve_x_for_y(&self, y: S) -> S {
        self.x(self.solve_t_for_y(y))
    }

    /// Returns an inverted version of this segment where the beginning and the end
    /// points are swapped.
    #[inline]
    pub fn flip(&self) -> Self {
        LineSegment { from: self.to, to: self.from }
    }

    /// Return the sub-segment inside a given range of t.
    ///
    /// This is equivalent splitting at the range's end points.
    pub fn split_range(&self, t_range: Range<S>) -> Self {
        LineSegment {
            from: self.from.lerp(self.to, t_range.start),
            to: self.from.lerp(self.to, t_range.end),
        }
    }

    /// Split this curve into two sub-segments.
    #[inline]
    pub fn split(&self, t: S) -> (Self, Self) {
        let split_point = self.sample(t);
        return (
            LineSegment { from: self.from, to: split_point },
            LineSegment { from: split_point, to: self.to },
        );
    }

    /// Return the segment before the split point.
    #[inline]
    pub fn before_split(&self, t: S) -> Self {
        LineSegment { from: self.from, to: self.sample(t) }
    }

    /// Return the segment after the split point.
    #[inline]
    pub fn after_split(&self, t: S) -> Self {
        LineSegment { from: self.sample(t), to: self.to }
    }

    pub fn split_at_x(&self, x: S) -> (Self, Self) {
        self.split(self.solve_t_for_x(x))
    }

    /// Return the minimum bounding rectangle
    #[inline]
    pub fn bounding_rect(&self) -> Rect<S> {
        let (min_x, max_x) = self.bounding_range_x();
        let (min_y, max_y) = self.bounding_range_y();

        let width  = max_x - min_x;
        let height = max_y - min_y;
        Rect::new(Point::new(min_x, min_y), Size::new(width, height))
    }

    #[inline]
    fn bounding_range_x(&self) -> (S, S) {
        min_max(self.from.x, self.to.x)
    }

    #[inline]
    fn bounding_range_y(&self) -> (S, S) {
        min_max(self.from.y, self.to.y)
    }

    /// Returns the vector between this segment's `from` and `to` points.
    #[inline]
    pub fn to_vector(&self) -> Vector<S> {
        self.to - self.from
    }

    /// Returns the line containing this segment.
    #[inline]
    pub fn to_line(&self) -> Line<S> {
        Line {
            point: self.from,
            vector: self.to - self.from,
        }
    }

    /// Computes the length of this segment.
    #[inline]
    pub fn length(&self) -> S {
        self.to_vector().length()
    }

    /// Changes the segment's length, moving destination point.
    pub fn set_length(&mut self, new_length: S) {
        let v = self.to_vector();
        let old_length = v.length();
        self.to = self.from + v * (new_length / old_length);
    }

    #[inline]
    pub fn translate(&mut self, by: Vector<S>) -> Self {
        LineSegment {
            from: self.from + by,
            to: self.to + by,
        }
    }

    /// Applies the transform to this segment and returns the results.
    #[inline]
    pub fn transform(&self, transform: &Transform2D<S>) -> Self {
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
    pub fn intersection_t(&self, other: &Self) -> Option<(S, S)> {
        let (min1, max1) = self.bounding_range_x();
        let (min2, max2) = other.bounding_range_x();
        if min1 > max2 || max1 < min2 {
            return None;
        }

        if self.to == other.to
            || self.from == other.from
            || self.from == other.to
            || self.to == other.from {
            return None;
        }

        let v1 = self.to_vector();
        let v2 = other.to_vector();

        let v1_cross_v2 = v1.cross(v2);

        if v1_cross_v2 == S::ZERO {
            // The segments are parallel
            return None;
        }

        let sign_v1_cross_v2 = S::signum(v1_cross_v2);
        let abs_v1_cross_v2 = S::abs(v1_cross_v2);

        let v3 = other.from - self.from;

        // t and u should be divided by v1_cross_v2, but we postpone that to not lose precision.
        // We have to respect the sign of v1_cross_v2 (and therefore t and u) so we apply it now and
        // will use the absolute value of v1_cross_v2 afterwards.
        let t = v3.cross(v2) * sign_v1_cross_v2;
        let u = v3.cross(v1) * sign_v1_cross_v2;

        if t < S::ZERO || t > abs_v1_cross_v2 || u < S::ZERO || u > abs_v1_cross_v2 {
            return None;
        }

        Some((
            t / abs_v1_cross_v2,
            u / abs_v1_cross_v2,
        ))
    }

    #[inline]
    pub fn intersection(&self, other: &Self) -> Option<Point<S>> {
        self.intersection_t(other).map(|(t, _)| self.sample(t))
    }

    pub fn line_intersection_t(&self, line: &Line<S>) -> Option<S> {
        let v1 = self.to_vector();
        let v2 = line.vector;

        let v1_cross_v2 = v1.cross(v2);

        if v1_cross_v2 == S::ZERO {
            // The segments are parallel
            return None;
        }

        let sign_v1_cross_v2 = S::signum(v1_cross_v2);
        let abs_v1_cross_v2 = S::abs(v1_cross_v2);

        let v3 = line.point - self.from;
        let t = v3.cross(v2) * sign_v1_cross_v2;

        if t < S::ZERO || t > abs_v1_cross_v2 {
            return None;
        }

        Some(t / abs_v1_cross_v2)
    }

    #[inline]
    pub fn line_intersection(&self, line: &Line<S>) -> Option<Point<S>> {
        self.line_intersection_t(line).map(|t| self.sample(t))
    }

    pub fn horizontal_line_intersection_t(&self, y: S) -> Option<S> {
        Self::axis_aligned_intersection_1d(self.from.y, self.to.y, y)
    }

    pub fn vertical_line_intersection_t(&self, x: S) -> Option<S> {
        Self::axis_aligned_intersection_1d(self.from.x, self.to.x, x)
    }

    #[inline]
    pub fn horizontal_line_intersection(&self, y: S) -> Option<Point<S>> {
        self.horizontal_line_intersection_t(y).map(|t| self.sample(t))
    }

    #[inline]
    pub fn vertical_line_intersection(&self, x: S) -> Option<Point<S>> {
        self.vertical_line_intersection_t(x).map(|t| self.sample(t))
    }

    fn axis_aligned_intersection_1d(mut a: S, mut b: S, v: S) -> Option<S> {
        let swap = a > b;
        if swap {
            std::mem::swap(&mut a, &mut b);
        }

        let d = b - a;
        if d == S::ZERO {
            return None;
        }

        let t = (v - a) / d;

        if t < S::ZERO || t > S::ONE {
            return None;
        }

        Some(if swap { S::ONE - t } else { t })
    }

    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        self.intersection_t(other).is_some()
    }

    #[inline]
    pub fn intersects_line(&self, line: &Line<S>) -> bool {
        self.line_intersection_t(line).is_some()
    }

    pub fn overlaps_line(&self, line: &Line<S>) -> bool {
        let v1 = self.to_vector();
        let v2 = line.vector;
        let v3 = line.point - self.from;

        v1.cross(v2) == S::ZERO && v1.cross(v3) == S::ZERO
    }

    pub fn overlaps_segment(&self, other: &LineSegment<S>) -> bool {
        if !self.overlaps_line(&other.to_line()) {
            return false;
        }

        let v1 = self.to - self.from;
        let v2 = other.from - self.from;
        let v3 = other.to - self.from;
        let mut a = S::ZERO;
        let mut b = v1.dot(v1);
        let mut c = v1.dot(v2);
        let mut d = v1.dot(v3);

        if a > b {
            swap(&mut a, &mut b);
        }
        if c > d {
            swap(&mut d, &mut c);
        }

        (c > a && c < b)
            || (d > a && d < b)
            || (a > c && a < d)
            || (b > c && b < d)
            || (a == c && b == d)
    }

    pub fn contains_segment(&self, other: &LineSegment<S>) -> bool {
        if !self.overlaps_line(&other.to_line()) {
            return false;
        }

        let v1 = self.to - self.from;
        let v2 = other.from - self.from;
        let v3 = other.to - self.from;
        let mut a = S::ZERO;
        let mut b = v1.dot(v1);
        let mut c = v1.dot(v2);
        let mut d = v1.dot(v3);

        if a > b {
            swap(&mut a, &mut b);
        }
        if c > d {
            swap(&mut d, &mut c);
        }

        c >= a && c <= b && d >= a && d <= b
    }
}

impl<S: Scalar> Segment for LineSegment<S> {
    type Scalar = S;
    fn from(&self) -> Point<S> { self.from }
    fn to(&self) -> Point<S> { self.to }
    fn sample(&self, t: S) -> Point<S> { self.sample(t) }
    fn x(&self, t: S) -> S { self.x(t) }
    fn y(&self, t: S) -> S { self.y(t) }
    fn derivative(&self, _t: S) -> Vector<S> { self.to_vector() }
    fn dx(&self, _t: S) -> S { self.to.x - self.from.x }
    fn dy(&self, _t: S) -> S { self.to.y - self.from.y }
    fn split_range(&self, t_range: Range<S>) -> Self { self.split_range(t_range) }
    fn split(&self, t: S) -> (Self, Self) { self.split(t) }
    fn before_split(&self, t: S) -> Self { self.before_split(t) }
    fn after_split(&self, t: S) -> Self { self.after_split(t) }
    fn flip(&self) -> Self { self.flip() }
    fn approximate_length(&self, _tolerance: S) -> S { self.length() }
}

impl<S: Scalar> BoundingRect for LineSegment<S> {
    type Scalar = S;
    fn bounding_rect(&self) -> Rect<S> { self.bounding_rect() }
    fn fast_bounding_rect(&self) -> Rect<S> { self.bounding_rect() }
    fn bounding_range_x(&self) -> (S, S) { self.bounding_range_x() }
    fn bounding_range_y(&self) -> (S, S) { self.bounding_range_y() }
    fn fast_bounding_range_x(&self) -> (S, S) { self.bounding_range_x() }
    fn fast_bounding_range_y(&self) -> (S, S) { self.bounding_range_y() }
}

impl<S: Scalar> MonotonicSegment for LineSegment<S> {
    type Scalar = S;
    fn solve_t_for_x(&self, x: S, _t_range: Range<S>, _tolerance: S) -> S {
        self.solve_t_for_x(x)
    }
}

impl<S: Scalar> FlatteningStep for LineSegment<S> {
    fn flattening_step(&self, _tolerance: S) -> S { S::ONE }
}

/// An infinite line defined by a point and a vector.
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Line<S> {
    pub point: Point<S>,
    pub vector: Vector<S>,
}

impl<S: Scalar> Line<S> {
    pub fn intersection(&self, other: &Self) -> Option<Point<S>> {
        let det = self.vector.cross(other.vector);
        if S::abs(det) <= S::EPSILON {
            // The lines are very close to parallel
            return None;
        }
        let inv_det = S::ONE / det;
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

    pub fn signed_distance_to_point(&self, p: &Point<S>) -> S {
        let v1 = self.point.to_vector();
        let v2 = v1 + self.vector;
        (self.vector.cross(p.to_vector()) + v1.cross(v2)) / self.vector.length()
    }

    pub fn distance_to_point(&self, p: &Point<S>) -> S {
        S::abs(self.signed_distance_to_point(p))
    }

    pub fn equation(&self) -> LineEquation<S> {
        let a = -self.vector.y;
        let b = self.vector.x;
        let c = -(a * self.point.x + b * self.point.y);

        LineEquation::new(a, b, c)
    }
}

/// A line defined by the equation
/// `a * x + b * y + c = 0; a * a + b * b = 1`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct LineEquation<S> {
    a: S,
    b: S,
    c: S,
}

impl<S: Scalar> LineEquation<S> {
    pub fn new(a: S, b: S, c: S) -> Self {
        debug_assert!(a != S::ZERO || b != S::ZERO);
        let div = S::ONE / S::sqrt(a * a + b * b);
        LineEquation { a: a * div, b: b * div, c: c * div }
    }

    #[inline]
    pub fn a(&self) -> S { self.a }

    #[inline]
    pub fn b(&self) -> S { self.b }

    #[inline]
    pub fn c(&self) -> S { self.c }

    pub fn project_point(&self, p: &Point<S>) -> Point<S> {
        point(
            self.b * (self.b * p.x - self.a * p.y) - self.a * self.c,
            self.a * (self.a * p.y - self.b * p.x) - self.b * self.c,
        )
    }

    #[inline]
    pub fn signed_distance_to_point(&self, p: &Point<S>) -> S {
        self.a * p.x + self.b * p.y + self.c
    }

    #[inline]
    pub fn distance_to_point(&self, p: &Point<S>) -> S {
        S::abs(self.signed_distance_to_point(p))
    }

    #[inline]
    pub fn invert(&self) -> Self {
        LineEquation { a: -self.a, b: -self.b, c: -self.c }
    }

    #[inline]
    pub fn parallel_line(&self, p: &Point<S>) -> Self {
        let c = -(self.a * p.x + self.b * p.y);
        LineEquation { a: self.a, b: self.b, c }
    }

    #[inline]
    pub fn offset(&self, d: S) -> Self {
        LineEquation { a: self.a, b: self.b, c: self.c - d }
    }

    #[inline]
    pub fn tangent(&self) -> Vector<S> {
        vector(self.b, -self.a)
    }

    #[inline]
    pub fn normal(&self) -> Vector<S> {
        vector(self.a, self.b)
    }

    #[inline]
    pub fn solve_y_for_x(&self, x: S) -> Option<S> {
        if self.b == S::ZERO {
            return None;
        }

        Some((self.a * x + self.c) / -self.b)
    }

    #[inline]
    pub fn solve_x_for_y(&self, y: S) -> Option<S> {
        if self.a == S::ZERO {
            return None;
        }

        Some((self.b * y + self.c) / -self.a)
    }

    #[inline]
    pub fn is_horizontal(&self) -> bool {
        self.a == S::ZERO
    }

    #[inline]
    pub fn is_vertical(&self) -> bool {
        self.b == S::ZERO
    }
}

#[cfg(test)]
fn fuzzy_eq_f32(a: f32, b: f32, epsilon: f32) -> bool {
    return f32::abs(a - b) <= epsilon;
}

#[cfg(test)]
fn fuzzy_eq_vector(a: Vector<f32>, b: Vector<f32>, epsilon: f32) -> bool {
    fuzzy_eq_f32(a.x, b.x, epsilon) && fuzzy_eq_f32(a.y, b.y, epsilon)
}

#[cfg(test)]
fn fuzzy_eq_point(a: Point<f32>, b: Point<f32>, epsilon: f32) -> bool {
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
                    l1.sample(l1.intersection_t(&l2).unwrap().0),
                    point(0.0, 0.0),
                    epsilon
                )
            );

            assert!(
                fuzzy_eq_point(
                    l2.sample(l1.intersection_t(&l2).unwrap().1),
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
#[cfg(test)]
use euclid::approxeq::ApproxEq;

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

#[test]
fn distance_to_point() {
    use crate::math::vector;

    let l1 = Line {
        point: point(2.0f32, 3.0),
        vector: vector(-1.5, 0.0),
    };

    let l2 = Line {
        point: point(3.0f32, 3.0),
        vector: vector(1.5, 1.5),
    };

    assert!(l1.signed_distance_to_point(&point(1.1, 4.0)).approx_eq(&-1.0));
    assert!(l1.signed_distance_to_point(&point(2.3, 2.0)).approx_eq(&1.0));

    assert!(l2.signed_distance_to_point(&point(1.0, 0.0)).approx_eq(&(-f32::sqrt(2.0)/2.0)));
    assert!(l2.signed_distance_to_point(&point(0.0, 1.0)).approx_eq(&(f32::sqrt(2.0)/2.0)));

    assert!(l1.equation().distance_to_point(&point(1.1, 4.0)).approx_eq(&1.0));
    assert!(l1.equation().distance_to_point(&point(2.3, 2.0)).approx_eq(&1.0));
    assert!(l2.equation().distance_to_point(&point(1.0, 0.0)).approx_eq(&(f32::sqrt(2.0)/2.0)));
    assert!(l2.equation().distance_to_point(&point(0.0, 1.0)).approx_eq(&(f32::sqrt(2.0)/2.0)));

    assert!(l1.equation().signed_distance_to_point(&point(1.1, 4.0)).approx_eq(&l1.signed_distance_to_point(&point(1.1, 4.0))));
    assert!(l1.equation().signed_distance_to_point(&point(2.3, 2.0)).approx_eq(&l1.signed_distance_to_point(&point(2.3, 2.0))));

    assert!(l2.equation().signed_distance_to_point(&point(1.0, 0.0)).approx_eq(&l2.signed_distance_to_point(&point(1.0, 0.0))));
    assert!(l2.equation().signed_distance_to_point(&point(0.0, 1.0)).approx_eq(&l2.signed_distance_to_point(&point(0.0, 1.0))));
}

#[test]
fn solve_y_for_x() {

    let line = Line {
        point: Point::new(1.0, 1.0),
        vector: Vector::new(2.0, 4.0),
    };
    let eqn = line.equation();

    if let Some(y) = eqn.solve_y_for_x(line.point.x) {
        println!("{:?} != {:?}", y, line.point.y);
        assert!(f64::abs(y - line.point.y) < 0.000001)
    }

    if let Some(x) = eqn.solve_x_for_y(line.point.y) {
        assert!(f64::abs(x - line.point.x) < 0.000001)
    }

    let mut angle = 0.1;
    for _ in 0..100 {
        let (sin, cos) = f64::sin_cos(angle);
        let line = Line {
            point: Point::new(-1000.0, 600.0),
            vector: Vector::new(cos * 100.0, sin * 100.0),
        };
        let eqn = line.equation();

        if let Some(y) = eqn.solve_y_for_x(line.point.x) {
            println!("{:?} != {:?}", y, line.point.y);
            assert!(f64::abs(y - line.point.y) < 0.000001)
        }

        if let Some(x) = eqn.solve_x_for_y(line.point.y) {
            assert!(f64::abs(x - line.point.x) < 0.000001)
        }

        angle += 0.001;
    }
}

#[test]
fn offset() {
    let l1 = LineEquation::new(2.0, 3.0, 1.0);
    let p = Point::new(10.0, 3.0);
    let d = l1.signed_distance_to_point(&p);
    let l2 = l1.offset(d);
    assert!(l2.distance_to_point(&p) < 0.0000001f64);
}

#[test]
fn set_length() {
    let mut a = LineSegment {
        from: point(10.0, 1.0),
        to: point(100.0, -15.0),
    };
    a.set_length(1.0);
    assert!(a.length().approx_eq(&1.0));
    a.set_length(1.5);
    assert!(a.length().approx_eq(&1.5));
    a.set_length(100.0);
    assert!(a.length().approx_eq(&100.0));
    a.set_length(-1.0);
    assert!(a.length().approx_eq(&1.0));
}

#[test]
fn overlap() {
    assert!(
        LineSegment {
            from: point(0.0, 0.0),
            to: point(-1.0, 0.0),
        }.overlaps_line(
            &Line {
                point: point(100.0, 0.0),
                vector: vector(10.0, 0.0),
            }
        )
    );

    assert!(
        LineSegment {
            from: point(0.0, 0.0),
            to: point(1.0, 0.0),
        }.overlaps_line(
            &Line {
                point: point(0.0, 0.0),
                vector: vector(1.0, 0.0),
            }
        )
    );

    assert!(
        LineSegment {
            from: point(0.0, 0.0),
            to: point(1.0, 0.0),
        }.overlaps_segment(
            &LineSegment {
                from: point(0.0, 0.0),
                to: point(1.0, 0.0),
            }
        )
    );

    assert!(
        !LineSegment {
            from: point(0.0, 0.0),
            to: point(1.0, 0.0),
        }.overlaps_line(
            &Line {
                point: point(0.0, 1.0),
                vector: vector(1.0, 1.0),
            }
        )
    );
}

#[test]
fn contains_segment() {
    assert!(
        LineSegment {
            from: point(-1.0, 1.0),
            to: point(4.0, 1.0),
        }.contains_segment(
            &LineSegment {
                from: point(2.0, 1.0),
                to: point(1.0, 1.0),
            }
        )
    );
}

#[test]
fn horizontal_line_intersection() {
    let segment = LineSegment {
        from: point(1.0, 2.0),
        to: point(2.0, 3.0),
    };

    assert_eq!(segment.horizontal_line_intersection_t(2.0), Some(0.0));
    assert_eq!(segment.horizontal_line_intersection_t(2.25), Some(0.25));
    assert_eq!(segment.horizontal_line_intersection_t(2.5), Some(0.5));
    assert_eq!(segment.horizontal_line_intersection_t(2.75), Some(0.75));
    assert_eq!(segment.horizontal_line_intersection_t(3.0), Some(1.0));

    assert_eq!(segment.horizontal_line_intersection_t(1.5), None);
    assert_eq!(segment.horizontal_line_intersection_t(3.5), None);

    let segment = LineSegment {
        from: point(2.0, 3.0),
        to: point(1.0, 2.0),
    };

    assert_eq!(segment.horizontal_line_intersection_t(2.0), Some(1.0));
    assert_eq!(segment.horizontal_line_intersection_t(2.25), Some(0.75));
    assert_eq!(segment.horizontal_line_intersection_t(2.5), Some(0.5));
    assert_eq!(segment.horizontal_line_intersection_t(2.75), Some(0.25));
    assert_eq!(segment.horizontal_line_intersection_t(3.0), Some(0.0));

    assert_eq!(segment.horizontal_line_intersection_t(1.5), None);
    assert_eq!(segment.horizontal_line_intersection_t(3.5), None);
}
