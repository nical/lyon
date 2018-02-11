use {Line, LineSegment, LineEquation};
use scalar::Scalar;
use generic_math::{Point, Vector, Rect, rect, Transform2D};
use arrayvec::ArrayVec;
use flatten_cubic::{flatten_cubic_bezier, find_cubic_bezier_inflection_points};
pub use flatten_cubic::Flattened;
use cubic_to_quadratic::*;
use monotonic::Monotonic;
use utils::{min_max, cubic_polynomial_roots};
use segment::{Segment, FlattenedForEach, approximate_length_from_flattening, BoundingRect};
use QuadraticBezierSegment;

use std::ops::Range;

/// A 2d curve segment defined by four points: the beginning of the segment, two control
/// points and the end of the segment.
///
/// The curve is defined by equation:²
/// ```∀ t ∈ [0..1],  P(t) = (1 - t)³ * from + 3 * (1 - t)² * t * ctrl1 + 3 * t² * (1 - t) * ctrl2 + t³ * to```
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct CubicBezierSegment<S> {
    pub from: Point<S>,
    pub ctrl1: Point<S>,
    pub ctrl2: Point<S>,
    pub to: Point<S>,
}

impl<S: Scalar> CubicBezierSegment<S> {
    /// Sample the curve at t (expecting t between 0 and 1).
    pub fn sample(&self, t: S) -> Point<S> {
        let t2 = t * t;
        let t3 = t2 * t;
        let one_t = S::ONE - t;
        let one_t2 = one_t * one_t;
        let one_t3 = one_t2 * one_t;
        return self.from * one_t3 +
            self.ctrl1.to_vector() * S::THREE * one_t2 * t +
            self.ctrl2.to_vector() * S::THREE * one_t * t2 +
            self.to.to_vector() * t3;
    }

    /// Sample the x coordinate of the curve at t (expecting t between 0 and 1).
    pub fn x(&self, t: S) -> S {
        let t2 = t * t;
        let t3 = t2 * t;
        let one_t = S::ONE - t;
        let one_t2 = one_t * one_t;
        let one_t3 = one_t2 * one_t;
        return self.from.x * one_t3 +
            self.ctrl1.x * S::THREE * one_t2 * t +
            self.ctrl2.x * S::THREE * one_t * t2 +
            self.to.x * t3;
    }

    /// Sample the y coordinate of the curve at t (expecting t between 0 and 1).
    pub fn y(&self, t: S) -> S {
        let t2 = t * t;
        let t3 = t2 * t;
        let one_t = S::ONE - t;
        let one_t2 = one_t * one_t;
        let one_t3 = one_t2 * one_t;
        return self.from.y * one_t3 +
            self.ctrl1.y * S::THREE * one_t2 * t +
            self.ctrl2.y * S::THREE * one_t * t2 +
            self.to.y * t3;
    }

    #[inline]
    fn derivative_coefficients(&self, t: S) -> (S, S, S, S) {
        let t2 = t*t;
        (
            - S::THREE * t2 + S::SIX * t - S::THREE,
            S::NINE * t2 - S::value(12.0) * t + S::THREE,
            - S::NINE * t2 + S::SIX * t,
            S::THREE * t2
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
        let t2 = (t_range.end - t_range.start) / (S::ONE - t_range.start);

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

    pub fn is_linear(&self, tolerance: S) -> bool {
        let epsilon = S::EPSILON;
        if (self.from - self.to).square_length() < epsilon {
            return false;
        }
        let line = self.baseline().to_line().equation();
        line.distance_to_point(&self.ctrl1) < tolerance
            && line.distance_to_point(&self.ctrl2) < tolerance
    }

    /// Computes a "fat line" of this segment.
    ///
    /// A fat line is two convervative lines between which the segment
    /// is fully contained.
    pub fn fat_line(&self) -> (LineEquation<S>, LineEquation<S>) {
        let baseline = self.baseline().to_line().equation();
        let (mut d1, mut d2) = min_max(
            baseline.signed_distance_to_point(&self.ctrl1),
            baseline.signed_distance_to_point(&self.ctrl2),
        );

        d1 = S::min(d1, S::ZERO);
        d2 = S::max(d2, S::ZERO);

        let frac_3_4 = S::THREE / S::FOUR;

        if (d1 * d2).is_sign_positive() {
            d1 = d1 * frac_3_4;
            d2 = d2 * frac_3_4;
        } else {
            d1 = d1 * frac_3_4 * frac_3_4;
            d2 = d2 * frac_3_4 * frac_3_4;
        }

        (baseline.offset(d1), baseline.offset(d2))
    }

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

    /// Invokes a callback between each monotonic part of the segment.
    pub fn for_each_monotonic_t<F>(&self, mut cb: F)
    where
        F: FnMut(S),
    {
        let mut x_extrema: ArrayVec<[S; 3]> = ArrayVec::new();
        self.for_each_local_x_extremum_t(&mut|t| { x_extrema.push(t) });

        let mut y_extrema: ArrayVec<[S; 3]> = ArrayVec::new();
        self.for_each_local_y_extremum_t(&mut|t| { y_extrema.push(t) });

        let mut it_x = x_extrema.iter().cloned();
        let mut it_y = y_extrema.iter().cloned();
        let mut tx = it_x.next();
        let mut ty = it_y.next();
        loop {
            let next = match (tx, ty) {
                (Some(a), Some(b)) => {
                    if a < b {
                        tx = it_x.next();
                        a
                    } else {
                        ty = it_y.next();
                        b
                    }
                }
                (Some(a), None) => {
                    tx = it_x.next();
                    a
                }
                (None, Some(b)) => {
                    ty = it_y.next();
                    b
                }
                (None, None) => {
                    return
                }
            };
            if next > S::ZERO && next < S::ONE {
                cb(next);
            }
        }
    }

    /// Invokes a callback for each monotonic part of the segment..
    pub fn for_each_monotonic_range<F>(&self, mut cb: F)
    where
        F: FnMut(Range<S>),
    {
        let mut t0 = S::ZERO;
        self.for_each_monotonic_t(|t| {
            cb(t0..t);
            t0 = t;
        });
        cb(t0..S::ONE);
    }

    /// Approximates the cubic bézier curve with sequence of quadratic ones,
    /// invoking a callback at each step.
    pub fn for_each_quadratic_bezier<F>(&self, tolerance: S, cb: &mut F)
    where
        F: FnMut(&QuadraticBezierSegment<S>)
    {
        cubic_to_quadratics(self, tolerance, cb);
    }

    /// Approximates the cubic bézier curve with sequence of monotonic quadratic
    /// ones, invoking a callback at each step.
    pub fn for_each_monotonic_quadratic<F>(&self, tolerance: S, cb: &mut F)
    where
        F: FnMut(&Monotonic<QuadraticBezierSegment<S>>)
    {
        cubic_to_monotonic_quadratics(self, tolerance, cb);
    }

    /// Iterates through the curve invoking a callback at each point.
    pub fn for_each_flattened<F: FnMut(Point<S>)>(&self, tolerance: S, call_back: &mut F) {
        flatten_cubic_bezier(*self, tolerance, call_back);
    }

    /// Compute the length of the segment using a flattened approximation.
    pub fn approximate_length(&self, tolerance: S) -> S {
        approximate_length_from_flattening(self, tolerance)
    }

    pub fn for_each_inflection_t<F>(&self, cb: &mut F)
    where F: FnMut(S) {
        find_cubic_bezier_inflection_points(self, cb);
    }

    /// Return local x extrema or None if this curve is monotonic.
    ///
    /// This returns the advancements along the curve, not the actual x position.
    pub fn for_each_local_x_extremum_t<F>(&self, cb: &mut F)
    where F: FnMut(S) {
        Self::for_each_local_extremum(self.from.x, self.ctrl1.x, self.ctrl2.x, self.to.x, cb)
    }

    /// Return local y extrema or None if this curve is monotonic.
    ///
    /// This returns the advancements along the curve, not the actual y position.
    pub fn for_each_local_y_extremum_t<F>(&self, cb: &mut F)
    where F: FnMut(S) {
        Self::for_each_local_extremum(self.from.y, self.ctrl1.y, self.ctrl2.y, self.to.y, cb)
    }


    fn for_each_local_extremum<F>(p0: S, p1: S, p2: S, p3: S, cb: &mut F)
    where F: FnMut(S) {
        // See www.faculty.idc.ac.il/arik/quality/appendixa.html for an explanation
        // The derivative of a cubic bezier curve is a curve representing a second degree polynomial function
        // f(x) = a * x² + b * x + c such as :

        let a = S::THREE * (p3 + S::THREE * (p1 - p2) - p0);
        let b = S::SIX * (p2 - S::TWO * p1 + p0);
        let c = S::THREE * (p1 - p0);

        fn in_range<S: Scalar>(t: S) -> bool { t > S::ZERO && t < S::ONE }

        // If the derivative is a linear function
        if a == S::ZERO {
            if b != S::ZERO {
                let t = -c / b;
                if in_range(t) {
                    cb(t);
                }
            }
            return;
        }

        let discriminant = b * b - S::FOUR * a * c;

        // There is no Real solution for the equation
        if discriminant < S::ZERO {
            return;
        }

        // There is one Real solution for the equation
        if discriminant == S::ZERO {
            let t = -b / (S::TWO * a);
            if in_range(t) {
                cb(t);
            }
            return;
        }

        // There are two Real solutions for the equation
        let discriminant_sqrt = discriminant.sqrt();

        let first_extremum = (-b - discriminant_sqrt) / (S::TWO * a);
        let second_extremum = (-b + discriminant_sqrt) / (S::TWO * a);

        if in_range(first_extremum) {
            cb(first_extremum);
        }

        if in_range(second_extremum) {
            cb(second_extremum);
        }
    }

    /// Find the advancement of the y-most position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual y position.
    pub fn y_maximum_t(&self) -> S {
        let mut max_t = S::ZERO;
        let mut max_y = self.from.y;
        if self.to.y > max_y {
            max_t = S::ONE;
            max_y = self.to.y;
        }
        self.for_each_local_y_extremum_t(&mut|t| {
            let y = self.y(t);
            if y > max_y {
                max_t = t;
                max_y = y;
            }
        });
        return max_t;
    }

    /// Find the advancement of the y-least position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual y position.
    pub fn y_minimum_t(&self) -> S {
        let mut min_t = S::ZERO;
        let mut min_y = self.from.y;
        if self.to.y < min_y {
            min_t = S::ONE;
            min_y = self.to.y;
        }
        self.for_each_local_y_extremum_t(&mut |t| {
            let y = self.y(t);
            if y < min_y {
                min_t = t;
                min_y = y;
            }
        });
        return min_t;
    }

    /// Find the advancement of the x-most position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual x position.
    pub fn x_maximum_t(&self) -> S {
        let mut max_t = S::ZERO;
        let mut max_x = self.from.x;
        if self.to.x > max_x {
            max_t = S::ONE;
            max_x = self.to.x;
        }
        self.for_each_local_x_extremum_t(&mut |t| {
            let x = self.x(t);
            if x > max_x {
                max_t = t;
                max_x = x;
            }
        });
        return max_t;
    }

    /// Find the x-least position in the curve.
    pub fn x_minimum_t(&self) -> S {
        let mut min_t = S::ZERO;
        let mut min_x = self.from.x;
        if self.to.x < min_x {
            min_t = S::ONE;
            min_x = self.to.x;
        }
        self.for_each_local_x_extremum_t(&mut |t| {
            let x = self.x(t);
            if x < min_x {
                min_t = t;
                min_x = x;
            }
        });
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

    /// Returns a conservative range of x this curve is contained in.
    #[inline]
    pub fn fast_bounding_range_x(&self) -> (S, S) {
        let min_x = self.from.x.min(self.ctrl1.x).min(self.ctrl2.x).min(self.to.x);
        let max_x = self.from.x.max(self.ctrl1.x).max(self.ctrl2.x).max(self.to.x);

        (min_x, max_x)
    }

    /// Returns a conservative range of y this curve is contained in.
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

    /// Returns the smallest range of x this curve is contained in.
    #[inline]
    pub fn bounding_range_x(&self) -> (S, S) {
        let min_x = self.x(self.x_minimum_t());
        let max_x = self.x(self.x_maximum_t());

        (min_x, max_x)
    }

    /// Returns the smallest range of y this curve is contained in.
    #[inline]
    pub fn bounding_range_y(&self) -> (S, S) {
        let min_y = self.y(self.y_minimum_t());
        let max_y = self.y(self.y_maximum_t());

        (min_y, max_y)
    }

    /// Cast this curve into a monotonic curve without checking that the monotonicity
    /// assumption is correct.
    pub fn assume_monotonic(&self) -> MonotonicCubicBezierSegment<S> {
        MonotonicCubicBezierSegment { segment: *self }
    }

    /// Returns whether this segment is monotonic on the x axis.
    pub fn is_x_monotonic(&self) -> bool {
        let mut found = false;
        self.for_each_local_x_extremum_t(&mut |_|{ found = true; });
        !found
    }

    /// Returns whether this segment is monotonic on the y axis.
    pub fn is_y_monotonic(&self) -> bool {
        let mut found = false;
        self.for_each_local_y_extremum_t(&mut |_|{ found = true; });
        !found
    }

    /// Returns whether this segment is fully monotonic.
    pub fn is_monotonic(&self) -> bool {
        self.is_x_monotonic() && self.is_y_monotonic()
    }

    /// Computes the intersections (if any) between this segment a line.
    ///
    /// The result is provided in the form of the `t` parameters of each
    /// point along curve. To get the intersection points, sample the curve
    /// at the corresponding values.
    pub fn line_intersections_t(&self, line: &Line<S>) -> ArrayVec<[S; 3]> {
        if line.vector.square_length() < S::EPSILON {
            return ArrayVec::new();
        }

        let from = self.from.to_vector();
        let ctrl1 = self.ctrl1.to_vector();
        let ctrl2 = self.ctrl2.to_vector();
        let to = self.to.to_vector();

        let p1 = to - from + (ctrl1 - ctrl2) * S::THREE;
        let p2 = from * S::THREE + (ctrl2 - ctrl1 * S::TWO) * S::THREE;
        let p3 = (ctrl1 - from) * S::THREE;
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
            if root > S::ZERO && root < S::ONE {
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

    #[inline]
    pub fn from(&self) -> Point<S> { self.from }

    #[inline]
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

impl<S: Scalar> Segment for CubicBezierSegment<S> { impl_segment!(S); }

impl<S: Scalar> BoundingRect for CubicBezierSegment<S> {
    type Scalar = S;
    fn bounding_rect(&self) -> Rect<S> { self.bounding_rect() }
    fn fast_bounding_rect(&self) -> Rect<S> { self.fast_bounding_rect() }
    fn bounding_range_x(&self) -> (S, S) { self.bounding_range_x() }
    fn bounding_range_y(&self) -> (S, S) { self.bounding_range_y() }
    fn fast_bounding_range_x(&self) -> (S, S) { self.fast_bounding_range_x() }
    fn fast_bounding_range_y(&self) -> (S, S) { self.fast_bounding_range_y() }
}

impl<S: Scalar> FlattenedForEach for CubicBezierSegment<S> {
    fn for_each_flattened<F: FnMut(Point<S>)>(&self, tolerance: S, call_back: &mut F) {
        self.for_each_flattened(tolerance, call_back);
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
fn y_maximum_t_for_simple_cubic_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.5, 1.0),
        ctrl2: Point::new(1.5, 1.0),
        to: Point::new(2.0, 2.0),
    };

    let expected_y_maximum = 1.0;

    let actual_y_maximum = a.y_maximum_t();

    assert!(expected_y_maximum == actual_y_maximum)
}

#[test]
fn y_minimum_t_for_simple_cubic_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.5, 1.0),
        ctrl2: Point::new(1.5, 1.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_y_minimum = 0.0;

    let actual_y_minimum = a.y_minimum_t();

    assert!(expected_y_minimum == actual_y_minimum)
}

#[test]
fn y_extrema_for_simple_cubic_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(1.0, 2.0),
        ctrl2: Point::new(2.0, 2.0),
        to: Point::new(3.0, 0.0),
    };

    let mut n: u32 = 0;
    a.for_each_local_y_extremum_t(&mut |t| {
        assert_eq!(t, 0.5);
        n += 1;
    });
    assert_eq!(n, 1);
}

#[test]
fn x_extrema_for_simple_cubic_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(1.0, 2.0),
        ctrl2: Point::new(1.0, 2.0),
        to: Point::new(0.0, 0.0),
    };

    let mut n: u32 = 0;
    a.for_each_local_x_extremum_t(&mut |t| {
        assert_eq!(t, 0.5);
        n += 1;
    });
    assert_eq!(n, 1);
}

#[test]
fn x_maximum_t_for_simple_cubic_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.5, 1.0),
        ctrl2: Point::new(1.5, 1.0),
        to: Point::new(2.0, 0.0),
    };
    let expected_x_maximum = 1.0;

    let actual_x_maximum = a.x_maximum_t();

    assert!(expected_x_maximum == actual_x_maximum)
}

#[test]
fn x_minimum_t_for_simple_cubic_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.5, 1.0),
        ctrl2: Point::new(1.5, 1.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_x_minimum = 0.0;

    let actual_x_minimum = a.x_minimum_t();

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
        assert!(f32::abs(x_diff) <= tolerance);
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

    let (l1, l2) = c1.fat_line();

    for i in 0..100 {
        let t = i as f32 / 99.0;
        assert!(l1.signed_distance_to_point(&c1.sample(t)) >= -0.000001);
        assert!(l2.signed_distance_to_point(&c1.sample(t)) <= 0.000001);
    }

    let c2 = CubicBezierSegment {
        from: point(1.0f32, 2.0),
        ctrl1: point(1.0, 3.0),
        ctrl2: point(11.0, 14.0),
        to: point(11.0, 12.0),
    };

    let (l1, l2) = c2.fat_line();

    for i in 0..100 {
        let t = i as f32 / 99.0;
        assert!(l1.signed_distance_to_point(&c2.sample(t)) >= -0.000001);
        assert!(l2.signed_distance_to_point(&c2.sample(t)) <= 0.000001);
    }
}

#[test]
fn is_linear() {
    let mut angle = 0.0;
    let center = Point::new(1000.0, -700.0);
    for _ in 0..100 {
        for i in 0..10 {
            for j in 0..10 {
                let (sin, cos) = f64::sin_cos(angle);
                let endpoint = Vector::new(cos * 100.0, sin * 100.0);
                let curve = CubicBezierSegment {
                    from: center - endpoint,
                    ctrl1: center + endpoint.lerp(-endpoint, i as f64 / 9.0),
                    ctrl2: center + endpoint.lerp(-endpoint, j as f64 / 9.0),
                    to: center + endpoint,
                };
                assert!(curve.is_linear(1e-10));
            }
        }
        angle += 0.001;
    }
}

#[test]
fn test_monotonic() {
    use math::point;
    let curve = CubicBezierSegment {
        from: point(1.0, 1.0),
        ctrl1: point(10.0, 2.0),
        ctrl2: point(1.0, 3.0),
        to: point(10.0, 4.0),
    };

    curve.for_each_monotonic_range(&mut|range| {
        let sub_curve = curve.split_range(range);
        assert!(sub_curve.is_monotonic());
    });
}
