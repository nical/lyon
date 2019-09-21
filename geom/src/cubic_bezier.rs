pub use crate::flatten_cubic::Flattened;
use crate::{Line, LineSegment, LineEquation, QuadraticBezierSegment};
use crate::scalar::Scalar;
use crate::generic_math::{Point, Vector, Rect, rect, Transform2D};
use crate::flatten_cubic::{flatten_cubic_bezier, flatten_cubic_bezier_with_t, find_cubic_bezier_inflection_points};
use crate::cubic_to_quadratic::*;
use crate::cubic_bezier_intersections::cubic_bezier_intersections_t;
use crate::monotonic::Monotonic;
use crate::utils::{min_max, cubic_polynomial_roots};
use crate::segment::{Segment, BoundingRect};
use arrayvec::ArrayVec;

use std::ops::Range;
use std::cmp::Ordering::{Less, Equal, Greater};

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

    /// Return the parameter values corresponding to a given x coordinate.
    /// See also solve_t_for_x for monotonic curves.
    pub fn solve_t_for_x(&self, x: S) -> ArrayVec<[S; 3]> {
        if self.is_a_point(S::ZERO)
            || (self.non_point_is_linear(S::ZERO) && self.from.x == self.to.x)
        {
            return ArrayVec::new();
        }

        self.parameters_for_xy_value(x, self.from.x, self.ctrl1.x, self.ctrl2.x, self.to.x)
    }

    /// Return the parameter values corresponding to a given y coordinate.
    /// See also solve_t_for_y for monotonic curves.
    pub fn solve_t_for_y(&self, y: S) -> ArrayVec<[S; 3]> {
        if self.is_a_point(S::ZERO)
            || (self.non_point_is_linear(S::ZERO) && self.from.y == self.to.y)
        {
            return ArrayVec::new();
        }

        self.parameters_for_xy_value(y, self.from.y, self.ctrl1.y, self.ctrl2.y, self.to.y)
    }

    fn parameters_for_xy_value(
        &self,
        value: S,
        from: S,
        ctrl1: S,
        ctrl2: S,
        to: S,
    ) -> ArrayVec<[S; 3]> {
        let mut result = ArrayVec::new();

        let a = -from + S::THREE * ctrl1 - S::THREE * ctrl2 + to;
        let b = S::THREE * from - S::SIX * ctrl1 + S::THREE * ctrl2;
        let c = -S::THREE * from + S::THREE * ctrl1;
        let d = from - value;

        let roots = cubic_polynomial_roots(a, b, c, d);
        for root in roots {
            if root > S::ZERO && root < S::ONE {
                result.push(root);
            }
        }

        result
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
    /// This is equivalent to splitting at the range's end points.
    pub fn split_range(&self, t_range: Range<S>) -> Self {
        let (t0, t1) = (t_range.start, t_range.end);
        let from = self.sample(t0);
        let to = self.sample(t1);

        let d = QuadraticBezierSegment {
            from: (self.ctrl1 - self.from).to_point(),
            ctrl: (self.ctrl2 - self.ctrl1).to_point(),
            to: (self.to - self.ctrl2).to_point(),
        };

        let dt = t1 - t0;
        let ctrl1 = from + d.sample(t0).to_vector() * dt;
        let ctrl2 = to - d.sample(t1).to_vector() * dt;

        CubicBezierSegment { from, ctrl1, ctrl2, to }
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

        self.non_point_is_linear(tolerance)
    }

    #[inline]
    fn non_point_is_linear(&self, tolerance: S) -> bool {
        let line = self.baseline().to_line().equation();
        line.distance_to_point(&self.ctrl1) <= tolerance
            && line.distance_to_point(&self.ctrl2) <= tolerance
    }

    pub(crate) fn is_a_point(&self, tolerance: S) -> bool {
        let tolerance_squared = tolerance * tolerance;
        // Use <= so that tolerance can be zero.
        (self.from - self.to).square_length() <= tolerance_squared
            && (self.from - self.ctrl1).square_length() <= tolerance_squared
            && (self.to - self.ctrl2).square_length() <= tolerance_squared
    }

    /// Computes the signed distances (min <= 0 and max >= 0) from the baseline of this
    /// curve to its two "fat line" boundary lines.
    ///
    /// A fat line is two conservative lines between which the segment
    /// is fully contained.
    pub(crate) fn fat_line_min_max(&self) -> (S, S) {
        let baseline = self.baseline().to_line().equation();
        let (d1, d2) = min_max(
            baseline.signed_distance_to_point(&self.ctrl1),
            baseline.signed_distance_to_point(&self.ctrl2),
        );

        let factor = if (d1 * d2) > S::ZERO {
            S::THREE / S::FOUR
        } else {
            S::FOUR / S::NINE
        };

        let d_min = factor * S::min(d1, S::ZERO);
        let d_max = factor * S::max(d2, S::ZERO);

        (d_min, d_max)
    }

    /// Computes a "fat line" of this segment.
    ///
    /// A fat line is two conservative lines between which the segment
    /// is fully contained.
    pub fn fat_line(&self) -> (LineEquation<S>, LineEquation<S>) {
        let baseline = self.baseline().to_line().equation();
        let (d1, d2) = self.fat_line_min_max();

        (baseline.offset(d1), baseline.offset(d2))
    }

    /// Applies the transform to this curve and returns the results.
    #[inline]
    pub fn transform(&self, transform: &Transform2D<S>) -> Self {
        CubicBezierSegment {
            from: transform.transform_point(self.from),
            ctrl1: transform.transform_point(self.ctrl1),
            ctrl2: transform.transform_point(self.ctrl2),
            to: transform.transform_point(self.to)
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
    /// Iterates through the curve invoking a callback at each point.
    pub fn for_each_flattened_with_t<F: FnMut(Point<S>, S)>(&self, tolerance: S, call_back: &mut F) {
        flatten_cubic_bezier_with_t(*self, tolerance, call_back);
    }

    /// Compute the length of the segment using a flattened approximation.
    pub fn approximate_length(&self, tolerance: S) -> S {
        let mut from = self.from;
        let mut len = S::ZERO;
        self.for_each_flattened(tolerance, &mut|to| {
            len = len + (to - from).length();
            from = to;
        });

        len
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

    /// Computes the intersections (if any) between this segment and another one.
    ///
    /// The result is provided in the form of the `t` parameters of each point along the curves. To
    /// get the intersection points, sample the curves at the corresponding values.
    ///
    /// Returns endpoint intersections where an endpoint intersects the interior of the other curve,
    /// but not endpoint/endpoint intersections.
    ///
    /// Returns no intersections if either curve is a point.
    pub fn cubic_intersections_t(&self, curve: &CubicBezierSegment<S>) -> ArrayVec<[(S, S); 9]> {
        cubic_bezier_intersections_t(self, curve)
    }

    /// Computes the intersection points (if any) between this segment and another one.
    pub fn cubic_intersections(&self, curve: &CubicBezierSegment<S>) -> ArrayVec<[Point<S>; 9]> {
        let intersections = self.cubic_intersections_t(curve);

        let mut result_with_repeats = ArrayVec::<[_; 9]>::new();
        for (t, _) in intersections {
            result_with_repeats.push(self.sample(t));
        }

        // We can have up to nine "repeated" values here (for example: two lines, each of which
        // overlaps itself 3 times, intersecting in their 3-fold overlaps). We make an effort to
        // dedupe the results, but that's hindered by not having predictable control over how far
        // the repeated intersections can be from each other (and then by the fact that true
        // intersections can be arbitrarily close), so the results will never be perfect.

        let pair_cmp = |s: &Point<S>, t: &Point<S>| {
            if s.x < t.x || (s.x == t.x && s.y < t.y) {
                Less
            } else if s.x == t.x && s.y == t.y {
                Equal
            } else {
                Greater
            }
        };
        result_with_repeats.sort_unstable_by(pair_cmp);
        if result_with_repeats.len() <= 1 {
            return result_with_repeats;
        }

        #[inline]
        fn dist_sq<S: Scalar>(p1: &Point<S>, p2: &Point<S>) -> S {
            (p1.x - p2.x) * (p1.x - p2.x) + (p1.y - p2.y) * (p1.y - p2.y)
        }

        let epsilon_squared = S::EPSILON * S::EPSILON;
        let mut result = ArrayVec::new();
        let mut reference_intersection = &result_with_repeats[0];
        result.push(*reference_intersection);
        for i in 1..result_with_repeats.len() {
            let intersection = &result_with_repeats[i];
            if dist_sq(reference_intersection, intersection) < epsilon_squared {
                continue;
            } else {
                result.push(*intersection);
                reference_intersection = intersection;
            }
        }

        result
    }

    /// Computes the intersections (if any) between this segment a quadratic bézier segment.
    ///
    /// The result is provided in the form of the `t` parameters of each point along the curves. To
    /// get the intersection points, sample the curves at the corresponding values.
    ///
    /// Returns endpoint intersections where an endpoint intersects the interior of the other curve,
    /// but not endpoint/endpoint intersections.
    ///
    /// Returns no intersections if either curve is a point.
    pub fn quadratic_intersections_t(&self, curve: &QuadraticBezierSegment<S>) -> ArrayVec<[(S, S); 9]> {
        self.cubic_intersections_t(&curve.to_cubic())
    }

    /// Computes the intersection points (if any) between this segment and a quadratic bézier segment.
    pub fn quadratic_intersections(&self, curve: &QuadraticBezierSegment<S>) -> ArrayVec<[Point<S>; 9]> {
        self.cubic_intersections(&curve.to_cubic())
    }

    /// Computes the intersections (if any) between this segment and a line.
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

    /// Computes the intersection points (if any) between this segment and a line.
    pub fn line_intersections(&self, line: &Line<S>) -> ArrayVec<[Point<S>; 3]> {
        let intersections = self.line_intersections_t(&line);

        let mut result = ArrayVec::new();
        for t in intersections {
            result.push(self.sample(t));
        }

        return result;
    }

    /// Computes the intersections (if any) between this segment and a line segment.
    ///
    /// The result is provided in the form of the `t` parameters of each
    /// point along curve and segment. To get the intersection points, sample
    /// the segments at the corresponding values.
    pub fn line_segment_intersections_t(&self, segment: &LineSegment<S>) -> ArrayVec<[(S, S); 3]> {
        if !self.fast_bounding_rect().intersects(&segment.bounding_rect()) {
            return ArrayVec::new();
        }

        let intersections = self.line_intersections_t(&segment.to_line());

        let mut result = ArrayVec::new();
        if intersections.len() == 0 {
            return result;
        }

        let seg_is_mostly_vertical =
            S::abs(segment.from.y - segment.to.y) >= S::abs(segment.from.x - segment.to.x);
        let (seg_long_axis_min, seg_long_axis_max) = if seg_is_mostly_vertical {
            segment.bounding_range_y()
        } else {
            segment.bounding_range_x()
        };

        for t in intersections {
            let intersection_xy = if seg_is_mostly_vertical { self.y(t) } else { self.x(t) };
            if intersection_xy >= seg_long_axis_min && intersection_xy <= seg_long_axis_max {
                let t2 = (self.sample(t) - segment.from).length() / segment.length();
                result.push((t, t2));
            }
        }

        result
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
    use crate::math::point;

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

    let c3 = CubicBezierSegment {
        from: point(0.0f32, 1.0),
        ctrl1: point(0.5, 0.0),
        ctrl2: point(0.5, 0.0),
        to: point(1.0, 1.0),
    };

    let (l1, l2) = c3.fat_line();

    for i in 0..100 {
        let t = i as f32 / 99.0;
        assert!(l1.signed_distance_to_point(&c3.sample(t)) >= -0.000001);
        assert!(l2.signed_distance_to_point(&c3.sample(t)) <= 0.000001);
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
    use crate::math::point;
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

#[test]
fn test_line_segment_intersections() {
    use crate::math::point;
    fn assert_approx_eq(a: ArrayVec<[(f32, f32); 3]>, b: &[(f32, f32)], epsilon: f32) {
        for i in 0..a.len() {
            if f32::abs(a[i].0 - b[i].0) > epsilon || f32::abs(a[i].1 - b[i].1) > epsilon {
                println!("{:?} != {:?}", a, b);
            }
            assert!((a[i].0 - b[i].0).abs() <= epsilon && (a[i].1 - b[i].1).abs() <= epsilon);
        }
        assert_eq!(a.len(), b.len());
    }

    let epsilon = 0.0001;

    // Make sure we find intersections with horizontal and vertical lines.

    let curve1 = CubicBezierSegment {
        from: point(-1.0, -1.0),
        ctrl1: point(0.0, 4.0),
        ctrl2: point(10.0, -4.0),
        to: point(11.0, 1.0),
    };
    let seg1 = LineSegment { from: point(0.0, 0.0), to: point(10.0, 0.0) };
    assert_approx_eq(curve1.line_segment_intersections_t(&seg1), &[(0.5, 0.5)], epsilon);

    let curve2 = CubicBezierSegment {
        from: point(-1.0, 0.0),
        ctrl1: point(0.0, 5.0),
        ctrl2: point(0.0, 5.0),
        to: point(1.0, 0.0),
    };
    let seg2 = LineSegment { from: point(0.0, 0.0), to: point(0.0, 5.0) };
    assert_approx_eq(curve2.line_segment_intersections_t(&seg2), &[(0.5, 0.75)], epsilon);
}

#[test]
fn test_parameters_for_value() {
    use crate::math::point;
    fn assert_approx_eq(a: ArrayVec<[f32; 3]>, b: &[f32], epsilon: f32) {
        for i in 0..a.len() {
            if f32::abs(a[i] - b[i]) > epsilon {
                println!("{:?} != {:?}", a, b);
            }
            assert!((a[i] - b[i]).abs() <= epsilon);
        }
        assert_eq!(a.len(), b.len());
    }

    {
        let curve = CubicBezierSegment {
            from: point(0.0, 0.0),
            ctrl1: point(0.0, 8.0),
            ctrl2: point(10.0, 8.0),
            to: point(10.0, 0.0)
        };

        let epsilon = 1e-4;
        assert_approx_eq(curve.solve_t_for_x(5.0), &[0.5], epsilon);
        assert_approx_eq(curve.solve_t_for_y(6.0), &[0.5], epsilon);
    }
    {
        let curve = CubicBezierSegment {
            from: point(0.0, 10.0),
            ctrl1: point(0.0, 10.0),
            ctrl2: point(10.0, 10.0),
            to: point(10.0, 10.0)
        };

        assert_approx_eq(curve.solve_t_for_y(10.0), &[], 0.0);
    }
}

#[test]
fn test_cubic_intersection_deduping() {
    use crate::math::point;

    let epsilon = 0.0001;

    // Two "line segments" with 3-fold overlaps, intersecting in their overlaps for a total of nine
    // parameter intersections.
    let line1 = CubicBezierSegment {
        from: point(-1_000_000.0, 0.0),
        ctrl1: point(2_000_000.0, 3_000_000.0),
        ctrl2: point(-2_000_000.0, -1_000_000.0),
        to: point(1_000_000.0, 2_000_000.0),
    };
    let line2 = CubicBezierSegment {
        from: point(-1_000_000.0, 2_000_000.0),
        ctrl1: point(2_000_000.0, -1_000_000.0),
        ctrl2: point(-2_000_000.0, 3_000_000.0),
        to: point(1_000_000.0, 0.0),
    };
    let intersections = line1.cubic_intersections(&line2);
    // (If you increase the coordinates above to 10s of millions, you get two returned intersection
    // points; i.e. the test fails.)
    assert_eq!(intersections.len(), 1);
    assert!(f64::abs(intersections[0].x) < epsilon);
    assert!(f64::abs(intersections[0].y - 1_000_000.0) < epsilon);

    // Two self-intersecting curves that intersect in their self-intersections, for a total of four
    // parameter intersections.
    let curve1 = CubicBezierSegment {
        from: point(-10.0, -13.636363636363636),
        ctrl1: point(15.0, 11.363636363636363),
        ctrl2: point(-15.0, 11.363636363636363),
        to: point(10.0, -13.636363636363636),
    };
    let curve2 = CubicBezierSegment {
        from: point(13.636363636363636, -10.0),
        ctrl1: point(-11.363636363636363, 15.0),
        ctrl2: point(-11.363636363636363, -15.0),
        to: point(13.636363636363636, 10.0),
    };
    let intersections = curve1.cubic_intersections(&curve2);
    assert_eq!(intersections.len(), 1);
    assert!(f64::abs(intersections[0].x) < epsilon);
    assert!(f64::abs(intersections[0].y) < epsilon);
}
