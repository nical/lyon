use {Line, LineSegment};
use scalar::{Float, FloatExt, FloatConst, Trig, ApproxEq};
use generic_math::{Point, Vector, Rect, rect, Transform2D};
use arrayvec::ArrayVec;
use flatten_cubic::{flatten_cubic_bezier, find_cubic_bezier_inflection_points};
pub use flatten_cubic::Flattened;
pub use cubic_to_quadratic::cubic_to_quadratic;
use monotonic::Monotonic;
use utils::{min_max, cubic_polynomial_roots};
use segment::{Segment, FlattenedForEach, approximate_length_from_flattening, BoundingRect};

use std::ops::Range;

/// A 2d curve segment defined by four points: the beginning of the segment, two control
/// points and the end of the segment.
///
/// The curve is defined by equation:²
/// ```∀ t ∈ [0..1],  P(t) = (1 - t)³ * from + 3 * (1 - t)² * t * ctrl1 + 3 * t² * (1 - t) * ctrl2 + t³ * to```
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CubicBezierSegment<S: Float> {
    pub from: Point<S>,
    pub ctrl1: Point<S>,
    pub ctrl2: Point<S>,
    pub to: Point<S>,
}

impl<S: Float> CubicBezierSegment<S> {
    /// Sample the curve at t (expecting t between 0 and 1).
    pub fn sample(&self, t: S) -> Point<S> {
        let t2 = t * t;
        let t3 = t2 * t;
        let one_t = S::one() - t;
        let one_t2 = one_t * one_t;
        let one_t3 = one_t2 * one_t;
        return self.from * one_t3 +
            self.ctrl1.to_vector() * S::c(3.0) * one_t2 * t +
            self.ctrl2.to_vector() * S::c(3.0) * one_t * t2 +
            self.to.to_vector() * t3;
    }

    /// Sample the x coordinate of the curve at t (expecting t between 0 and 1).
    pub fn x(&self, t: S) -> S {
        let t2 = t * t;
        let t3 = t2 * t;
        let one_t = S::one() - t;
        let one_t2 = one_t * one_t;
        let one_t3 = one_t2 * one_t;
        return self.from.x * one_t3 +
            self.ctrl1.x * S::c(3.0) * one_t2 * t +
            self.ctrl2.x * S::c(3.0) * one_t * t2 +
            self.to.x * t3;
    }

    /// Sample the y coordinate of the curve at t (expecting t between 0 and 1).
    pub fn y(&self, t: S) -> S {
        let t2 = t * t;
        let t3 = t2 * t;
        let one_t = S::one() - t;
        let one_t2 = one_t * one_t;
        let one_t3 = one_t2 * one_t;
        return self.from.y * one_t3 +
            self.ctrl1.y * S::c(3.0) * one_t2 * t +
            self.ctrl2.y * S::c(3.0) * one_t * t2 +
            self.to.y * t3;
    }

    #[inline]
    fn derivative_coefficients(&self, t: S) -> (S, S, S, S) {
        let t2 = t*t;
        (
            - S::c(3.0) * t2 + S::c(6.0) * t - S::c(3.0),
            S::c(9.0) * t2 - S::c(12.0) * t + S::c(3.0),
            - S::c(9.0) * t2 + S::c(6.0) * t,
            S::c(3.0) * t2
        )
    }

    /// Sample the curve's derivative at t (expecting t between 0 and 1).
    pub fn derivative(&self, t: S) -> Vector<S> {
        let (c0, c1, c2, c3) = self.derivative_coefficients(t);
        self.from.to_vector() * c0 +
            self.ctrl1.to_vector() * c1 +
            self.ctrl2.to_vector() * c2 +
            self.to.to_vector() * c3
    }

    /// Sample the x coordinate of the curve's derivative at t (expecting t between 0 and 1).
    pub fn dx(&self, t: S) -> S {
        let (c0, c1, c2, c3) = self.derivative_coefficients(t);
        self.from.x * c0 + self.ctrl1.x * c1 + self.ctrl2.x * c2 + self.to.x * c3
    }

    /// Sample the y coordinate of the curve's derivative at t (expecting t between 0 and 1).
    pub fn dy(&self, t: S) -> S {
        let (c0, c1, c2, c3) = self.derivative_coefficients(t);
        self.from.y * c0 + self.ctrl1.y * c1 + self.ctrl2.y * c2 + self.to.y * c3
    }

    /// Return the sub-curve inside a given range of t.
    ///
    /// This is equivalent splitting at the range's end points.
    pub fn split_range(&self, t_range: Range<S>) -> Self {
        let t1 = t_range.start;
        let t2 = (t_range.end - t_range.start) / (S::one() - t_range.start);

        self.after_split(t1).before_split(t2)
    }

    /// Split this curve into two sub-curves.
    pub fn split(&self, t: S) -> (CubicBezierSegment<S>, CubicBezierSegment<S>) {
        let ctrl1a = self.from + (self.ctrl1 - self.from) * t;
        let ctrl2a = self.ctrl1 + (self.ctrl2 - self.ctrl1) * t;
        let ctrl1aa = ctrl1a + (ctrl2a - ctrl1a) * t;
        let ctrl3a = self.ctrl2 + (self.to - self.ctrl2) * t;
        let ctrl2aa = ctrl2a + (ctrl3a - ctrl2a) * t;
        let ctrl1aaa = ctrl1aa + (ctrl2aa - ctrl1aa) * t;
        let to = self.to;

        return (CubicBezierSegment {
            from: self.from,
            ctrl1: ctrl1a,
            ctrl2: ctrl1aa,
            to: ctrl1aaa,
        },
        CubicBezierSegment {
            from: ctrl1aaa,
            ctrl1: ctrl2aa,
            ctrl2: ctrl3a,
            to: to,
        });
    }

    /// Return the curve before the split point.
    pub fn before_split(&self, t: S) -> CubicBezierSegment<S> {
        let ctrl1a = self.from + (self.ctrl1 - self.from) * t;
        let ctrl2a = self.ctrl1 + (self.ctrl2 - self.ctrl1) * t;
        let ctrl1aa = ctrl1a + (ctrl2a - ctrl1a) * t;
        let ctrl3a = self.ctrl2 + (self.to - self.ctrl2) * t;
        let ctrl2aa = ctrl2a + (ctrl3a - ctrl2a) * t;
        let ctrl1aaa = ctrl1aa + (ctrl2aa - ctrl1aa) * t;
        return CubicBezierSegment {
            from: self.from,
            ctrl1: ctrl1a,
            ctrl2: ctrl1aa,
            to: ctrl1aaa,
        };
    }

    /// Return the curve after the split point.
    pub fn after_split(&self, t: S) -> CubicBezierSegment<S> {
        let ctrl1a = self.from + (self.ctrl1 - self.from) * t;
        let ctrl2a = self.ctrl1 + (self.ctrl2 - self.ctrl1) * t;
        let ctrl1aa = ctrl1a + (ctrl2a - ctrl1a) * t;
        let ctrl3a = self.ctrl2 + (self.to - self.ctrl2) * t;
        let ctrl2aa = ctrl2a + (ctrl3a - ctrl2a) * t;
        return CubicBezierSegment {
            from: ctrl1aa + (ctrl2aa - ctrl1aa) * t,
            ctrl1: ctrl2a + (ctrl3a - ctrl2a) * t,
            ctrl2: ctrl3a,
            to: self.to,
        };
    }

    #[inline]
    pub fn baseline(&self) -> LineSegment<S> {
        LineSegment { from: self.from, to: self.to }
    }

    /// Computes the "fat line" of this segment.
    ///
    /// A fat line is a bounding box of the segment oriented along the
    /// baseline segment with the maximum signed distances on each side
    /// of the baseline to the control points.
    pub fn fat_line(&self) -> (LineSegment<S>, S, S) where S : ApproxEq<S> {
        let baseline = self.baseline();
        let (mut d1, mut d2) = min_max(
            baseline.to_line().signed_distance_to_point(&self.ctrl1),
            baseline.to_line().signed_distance_to_point(&self.ctrl2),
        );

        d1 = Float::min(d1, S::zero());
        d2 = Float::max(d2, S::zero());

        (baseline, d1, d2)
    }
}

impl<S: Float + Trig> CubicBezierSegment<S> {
    /// Applies the transform to this curve and returns the results.
    #[inline]
    pub fn transform(&self, transform: &Transform2D<S>) -> Self {
        CubicBezierSegment {
            from: transform.transform_point(&self.from),
            ctrl1: transform.transform_point(&self.ctrl1),
            ctrl2: transform.transform_point(&self.ctrl2),
            to: transform.transform_point(&self.to)
        }
    }
}

impl<S: Float> CubicBezierSegment<S> {
    /// Swap the beginning and the end of the segment.
    pub fn flip(&self) -> Self {
        CubicBezierSegment {
            from: self.to,
            ctrl1: self.ctrl2,
            ctrl2: self.ctrl1,
            to: self.from,
        }
    }

    /// Returns the flattened representation of the curve as an iterator, starting *after* the
    /// current point.
    pub fn flattened(&self, tolerance: S) -> Flattened<S> {
        Flattened::new(*self, tolerance)
    }
}

impl<S: Float + FloatConst + ApproxEq<S>> CubicBezierSegment<S> {
    /// Iterates through the curve invoking a callback at each point.
    pub fn flattened_for_each<F: FnMut(Point<S>)>(&self, tolerance: S, call_back: &mut F) {
        flatten_cubic_bezier(*self, tolerance, call_back);
    }

    /// Compute the length of the segment using a flattened approximation.
    pub fn approximate_length(&self, tolerance: S) -> S {
        approximate_length_from_flattening(self, tolerance)
    }
}

impl<S: Float> CubicBezierSegment<S> {
    pub fn find_inflection_points(&self) -> ArrayVec<[S; 2]> {
        find_cubic_bezier_inflection_points(self)
    }

    /// Return local x extrema or None if this curve is monotonic.
    ///
    /// This returns the advancements along the curve, not the actual x position.
    pub fn find_local_x_extrema(&self) -> ArrayVec<[S; 2]> {
        let mut ret = ArrayVec::new();
        // See www.faculty.idc.ac.il/arik/quality/appendixa.html for an explanation
        // The derivative of a cubic bezier curve is a curve representing a second degree polynomial function
        // f(x) = a * x² + b * x + c such as :
        let a = S::c(3.0) * (self.to.x - S::c(3.0) * self.ctrl2.x + S::c(3.0) * self.ctrl1.x - self.from.x);
        let b = S::c(6.0) * (self.ctrl2.x - S::c(2.0) * self.ctrl1.x + self.from.x);
        let c = S::c(3.0) * (self.ctrl1.x - self.from.x);

        // If the derivative is a linear function
        if a == S::zero() {
            if b == S::zero() {
                // If the derivative is a constant function
                if c == S::zero() {
                    ret.push(S::zero());
                }
            } else {
                ret.push(-c / b);
            }
            return ret;
        }

        fn in_range<S: Float>(t: S) -> bool { t > S::zero() && t < S::one() }

        let discriminant = b * b - S::c(4.0) * a * c;

        // There is no Real solution for the equation
        if discriminant < S::zero() {
            return ret;
        }

        // There is one Real solution for the equation
        if discriminant == S::zero() {
            let t = -b / (S::c(2.0) * a);
            if in_range(t) {
                ret.push(t);
            }
            return ret;
        }

        // There are two Real solutions for the equation
        let discriminant_sqrt = discriminant.sqrt();

        let first_extremum = (-b - discriminant_sqrt) / (S::c(2.0) * a);
        let second_extremum = (-b + discriminant_sqrt) / (S::c(2.0) * a);

        if in_range(first_extremum) {
            ret.push(first_extremum);
        }

        if in_range(second_extremum) {
            ret.push(second_extremum);
        }
        ret
    }

    /// Return local y extrema or None if this curve is monotonic.
    ///
    /// This returns the advancements along the curve, not the actual y position.
    pub fn find_local_y_extrema(&self) -> ArrayVec<[S; 2]> {
       let switched_segment = CubicBezierSegment {
               from: self.from.yx(),
               ctrl1: self.ctrl1.yx(),
               ctrl2: self.ctrl2.yx(),
               to: self.to.yx(),
       };

        switched_segment.find_local_x_extrema()
    }

    /// Find the advancement of the y-most position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual y position.
    pub fn find_y_maximum(&self) -> S {
        let mut max_t = S::zero();
        let mut max_y = self.from.y;
        if self.to.y > max_y {
            max_t = S::one();
            max_y = self.to.y;
        }
        for t in self.find_local_y_extrema() {
            let point = self.sample(t);
            if point.y > max_y {
                max_t = t;
                max_y = point.y;
            }
        }
        return max_t;
    }

    /// Find the advancement of the y-least position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual y position.
    pub fn find_y_minimum(&self) -> S {
        let mut min_t = S::zero();
        let mut min_y = self.from.y;
        if self.to.y < min_y {
            min_t = S::one();
            min_y = self.to.y;
        }
        for t in self.find_local_y_extrema() {
            let point = self.sample(t);
            if point.y < min_y {
                min_t = t;
                min_y = point.y;
            }
        }
        return min_t;
    }

    /// Find the advancement of the x-most position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual x position.
    pub fn find_x_maximum(&self) -> S {
        let mut max_t = S::zero();
        let mut max_x = self.from.x;
        if self.to.x > max_x {
            max_t = S::one();
            max_x = self.to.x;
        }
        for t in self.find_local_x_extrema() {
            let point = self.sample(t);
            if point.x > max_x {
                max_t = t;
                max_x = point.x;
            }
        }
        return max_t;
    }

    /// Find the x-least position in the curve.
    pub fn find_x_minimum(&self) -> S {
        let mut min_t = S::zero();
        let mut min_x = self.from.x;
        if self.to.x < min_x {
            min_t = S::one();
            min_x = self.to.x;
        }
        for t in self.find_local_x_extrema() {
            let point = self.sample(t);
            if point.x < min_x {
                min_t = t;
                min_x = point.x;
            }
        }
        return min_t;
    }

    /// Returns a conservative rectangle the curve is contained in.
    ///
    /// This method is faster than `bounding_rect` but more conservative.
    pub fn fast_bounding_rect(&self) -> Rect<S> {
        let (min_x, max_x) = self.fast_bounding_range_x();
        let (min_y, max_y) = self.fast_bounding_range_y();

        return rect(min_x, min_y, max_x - min_x, max_y - min_y);
    }

    #[inline]
    pub fn fast_bounding_range_x(&self) -> (S, S) {
        let min_x = self.from.x.min(self.ctrl1.x).min(self.ctrl2.x).min(self.to.x);
        let max_x = self.from.x.max(self.ctrl1.x).max(self.ctrl2.x).max(self.to.x);

        (min_x, max_x)
    }

    #[inline]
    pub fn fast_bounding_range_y(&self) -> (S, S) {
        let min_y = self.from.y.min(self.ctrl1.y).min(self.ctrl2.y).min(self.to.y);
        let max_y = self.from.y.max(self.ctrl1.y).max(self.ctrl2.y).max(self.to.y);

        (min_y, max_y)
    }

    /// Returns the smallest rectangle the curve is contained in
    pub fn bounding_rect(&self) -> Rect<S> {
        let (min_x, max_x) = self.bounding_range_x();
        let (min_y, max_y) = self.bounding_range_y();

        return rect(min_x, min_y, max_x - min_x, max_y - min_y);
    }

    #[inline]
    pub fn bounding_range_x(&self) -> (S, S) {
        let min_x = self.x(self.find_x_minimum());
        let max_x = self.x(self.find_x_maximum());

        (min_x, max_x)
    }

    #[inline]
    pub fn bounding_range_y(&self) -> (S, S) {
        let min_y = self.y(self.find_y_minimum());
        let max_y = self.y(self.find_y_maximum());

        (min_y, max_y)
    }

    /// Cast this curve into a monotonic curve without checking that the monotonicity
    /// assumption is correct.
    pub fn assume_monotonic(&self) -> MonotonicCubicBezierSegment<S> {
        MonotonicCubicBezierSegment { segment: *self }
    }
}

impl<S: Float + FloatConst + ApproxEq<S>> CubicBezierSegment<S> {
    /// Computes the intersections (if any) between this segment a line.
    ///
    /// The result is provided in the form of the `t` parameters of each
    /// point along curve. To get the intersection points, sample the curve
    /// at the corresponding values.
    pub fn line_intersections_t(&self, line: &Line<S>) -> ArrayVec<[S; 3]> {
        if line.vector.square_length() < S::c(1e-6) {
            return ArrayVec::new();
        }

        let from = self.from.to_vector();
        let ctrl1 = self.ctrl1.to_vector();
        let ctrl2 = self.ctrl2.to_vector();
        let to = self.to.to_vector();

        let p1 = to - from + (ctrl1 - ctrl2) * S::c(3.0);
        let p2 = from * S::c(3.0) + (ctrl2 - ctrl1 * S::c(2.0)) * S::c(3.0);
        let p3 = (ctrl1 - from) * S::c(3.0);
        let p4 = from;

        let c = line.point.y * line.vector.x - line.point.x * line.vector.y;

        let roots = cubic_polynomial_roots(
            line.vector.y * p1.x - line.vector.x * p1.y,
            line.vector.y * p2.x - line.vector.x * p2.y,
            line.vector.y * p3.x - line.vector.x * p3.y,
            line.vector.y * p4.x - line.vector.x * p4.y + c,
        );

        let mut result = ArrayVec::new();

        for root in roots {
            if root > S::zero() && root < S::one() {
                result.push(root);
            }
        }

        return result;
    }

    pub fn line_intersections(&self, line: &Line<S>) -> ArrayVec<[Point<S>; 3]> {
        let intersections = self.line_intersections_t(&line);

        let mut result = ArrayVec::new();
        for t in intersections {
            result.push(self.sample(t));
        }

        return result;
    }

    /// Computes the intersections (if any) between this segment a line segment.
    ///
    /// The result is provided in the form of the `t` parameters of each
    /// point along curve and segment. To get the intersection points, sample
    /// the segments at the corresponding values.
    pub fn line_segment_intersections_t(&self, segment: &LineSegment<S>) -> ArrayVec<[(S, S); 3]> {
        if !self.fast_bounding_rect().intersects(&segment.bounding_rect()) {
            return ArrayVec::new();
        }

        let intersections = self.line_intersections_t(&segment.to_line());
        let aabb = segment.bounding_rect();

        let mut result = ArrayVec::new();
        for t in intersections {
            if aabb.contains(&self.sample(t)) {
                let t2 = (self.sample(t) - segment.from).length() / segment.length();
                result.push((t,t2));
            }
        }
        return result;
    }
}

impl<S: Float + FloatConst + ApproxEq<S>> CubicBezierSegment<S> {
    pub fn from(&self) -> Point<S> { self.from }

    pub fn to(&self) -> Point<S> { self.to }

    pub fn line_segment_intersections(&self, segment: &LineSegment<S>) -> ArrayVec<[Point<S>; 3]> {
        let intersections = self.line_segment_intersections_t(&segment);

        let mut result = ArrayVec::new();
        for (t, _) in intersections {
            result.push(self.sample(t));
        }

        return result;
    }
}

impl<S: Float + FloatConst + ApproxEq<S>> Segment for CubicBezierSegment<S> { impl_segment!(S); }

impl<S: Float> BoundingRect for CubicBezierSegment<S> {
    type Scalar = S;
    fn bounding_rect(&self) -> Rect<S> { self.bounding_rect() }
    fn fast_bounding_rect(&self) -> Rect<S> { self.fast_bounding_rect() }
    fn bounding_range_x(&self) -> (S, S) { self.bounding_range_x() }
    fn bounding_range_y(&self) -> (S, S) { self.bounding_range_y() }
    fn fast_bounding_range_x(&self) -> (S, S) { self.fast_bounding_range_x() }
    fn fast_bounding_range_y(&self) -> (S, S) { self.fast_bounding_range_y() }
}

impl<S: Float + FloatConst + ApproxEq<S>> FlattenedForEach for CubicBezierSegment<S> {
    fn flattened_for_each<F: FnMut(Point<S>)>(&self, tolerance: S, call_back: &mut F) {
        self.flattened_for_each(tolerance, call_back);
    }
}

/// A monotonically increasing in x and y quadratic bézier curve segment
pub type MonotonicCubicBezierSegment<S> = Monotonic<CubicBezierSegment<S>>;

#[test]
fn fast_bounding_rect_for_cubic_bezier_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.5, 1.0),
        ctrl2: Point::new(1.5, -1.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_bounding_rect = rect(0.0, -1.0, 2.0, 2.0);

    let actual_bounding_rect = a.fast_bounding_rect();

    assert!(expected_bounding_rect == actual_bounding_rect)
}

#[test]
fn minimum_bounding_rect_for_cubic_bezier_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.5, 2.0),
        ctrl2: Point::new(1.5, -2.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_bigger_bounding_rect: Rect<f32> = rect(0.0, -0.6, 2.0, 1.2);
    let expected_smaller_bounding_rect: Rect<f32> = rect(0.1, -0.5, 1.9, 1.0);

    let actual_minimum_bounding_rect: Rect<f32> = a.bounding_rect();

    assert!(expected_bigger_bounding_rect.contains_rect(&actual_minimum_bounding_rect));
    assert!(actual_minimum_bounding_rect.contains_rect(&expected_smaller_bounding_rect));
}

#[test]
fn find_y_maximum_for_simple_cubic_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.5, 1.0),
        ctrl2: Point::new(1.5, 1.0),
        to: Point::new(2.0, 2.0),
    };

    let expected_y_maximum = 1.0;

    let actual_y_maximum = a.find_y_maximum();

    assert!(expected_y_maximum == actual_y_maximum)
}

#[test]
fn find_y_minimum_for_simple_cubic_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.5, 1.0),
        ctrl2: Point::new(1.5, 1.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_y_minimum = 0.0;

    let actual_y_minimum = a.find_y_minimum();

    assert!(expected_y_minimum == actual_y_minimum)
}

#[test]
fn find_y_extrema_for_simple_cubic_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(1.0, 2.0),
        ctrl2: Point::new(2.0, 2.0),
        to: Point::new(3.0, 0.0),
    };

    let mut expected_y_extremums = Vec::new();
    expected_y_extremums.push(0.5);

    let actual_y_extremums = a.find_local_y_extrema();

    for extremum in expected_y_extremums {
        assert!(actual_y_extremums.contains(&extremum))
    }
}

#[test]
fn find_x_extrema_for_simple_cubic_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(1.0, 2.0),
        ctrl2: Point::new(1.0, 2.0),
        to: Point::new(0.0, 0.0),
    };

    let mut expected_x_extremums = Vec::new();
    expected_x_extremums.push(0.5);

    let actual_x_extremums = a.find_local_x_extrema();

    for extremum in expected_x_extremums {
        assert!(actual_x_extremums.contains(&extremum))
    }
}

#[test]
fn find_x_maximum_for_simple_cubic_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.5, 1.0),
        ctrl2: Point::new(1.5, 1.0),
        to: Point::new(2.0, 0.0),
    };
    let expected_x_maximum = 1.0;

    let actual_x_maximum = a.find_x_maximum();

    assert!(expected_x_maximum == actual_x_maximum)
}

#[test]
fn find_x_minimum_for_simple_cubic_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.5, 1.0),
        ctrl2: Point::new(1.5, 1.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_x_minimum = 0.0;

    let actual_x_minimum = a.find_x_minimum();

    assert!(expected_x_minimum == actual_x_minimum)
}

#[test]
fn derivatives() {
    let c1 = CubicBezierSegment {
        from: Point::new(1.0, 1.0,),
        ctrl1: Point::new(1.0, 2.0,),
        ctrl2: Point::new(2.0, 1.0,),
        to: Point::new(2.0, 2.0,),
    };

    assert_eq!(c1.dx(0.0), 0.0);
    assert_eq!(c1.dx(1.0), 0.0);
    assert_eq!(c1.dy(0.5), 0.0);
}

#[test]
fn monotonic_solve_t_for_x() {
    let c1 = CubicBezierSegment {
        from: Point::new(1.0, 1.0),
        ctrl1: Point::new(1.0, 2.0),
        ctrl2: Point::new(2.0, 1.0),
        to: Point::new(2.0, 2.0),
    };

    let tolerance = 0.0001;

    for i in 0..10u32 {
        let t = i as f32 / 10.0;
        let p = c1.sample(t);
        let t2 = c1.assume_monotonic().solve_t_for_x(p.x, 0.0..1.0, tolerance);
        // t should be pretty close to t2 but the only guarantee we have and can test
        // against is that x(t) - x(t2) is within the specified tolerance threshold.
        let x_diff = c1.x(t) - c1.x(t2);
        assert!(x_diff.abs() <= tolerance);
    }
}

#[test]
fn fat_line() {
    use math::point;

    let c1 = CubicBezierSegment {
        from: point(1.0f32, 2.0),
        ctrl1: point(1.0, 3.0),
        ctrl2: point(11.0, 11.0),
        to: point(11.0, 12.0),
    };

    let (baseline, d1, d2) = c1.fat_line();
    assert_eq!(baseline, LineSegment { from: c1.from, to: c1. to });
    assert!(d1 <= 0.0);
    assert!(d2 >= 0.0);
    let sqrt_2_2: f32 = Float::sqrt(2.0)/2.0;
    assert!(d1.approx_eq(&-sqrt_2_2));
    assert!(d2.approx_eq(&sqrt_2_2));

    let c2 = CubicBezierSegment {
        from: point(1.0f32, 2.0),
        ctrl1: point(1.0, 3.0),
        ctrl2: point(11.0, 14.0),
        to: point(11.0, 12.0),
    };

    let (baseline, d1, d2) = c2.fat_line();
    assert_eq!(baseline, LineSegment { from: c1.from, to: c1. to });
    assert!(d1.approx_eq(&(-2.0 * sqrt_2_2)));
    assert!(d2.approx_eq(&0.0));

    let c3 = CubicBezierSegment {
        from: point(1.0f32, 2.0),
        ctrl1: point(1.0, 0.0),
        ctrl2: point(11.0, 11.0),
        to: point(11.0, 12.0),
    };

    let (baseline, d1, d2) = c3.fat_line();
    assert_eq!(baseline, LineSegment { from: c1.from, to: c1. to });
    assert!(d1.approx_eq(&0.0));
    assert!(d2.approx_eq(&(2.0 * sqrt_2_2)));
}
