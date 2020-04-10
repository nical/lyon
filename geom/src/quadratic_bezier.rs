use crate::{rect, Point, Rect, Vector};
use crate::monotonic::Monotonic;
use crate::scalar::Scalar;
use crate::segment::{BoundingRect, Segment};
use crate::traits::Transformation;
use crate::{CubicBezierSegment, Line, LineEquation, LineSegment, Triangle};
use arrayvec::ArrayVec;

use std::mem;
use std::ops::Range;

/// A 2d curve segment defined by three points: the beginning of the segment, a control
/// point and the end of the segment.
///
/// The curve is defined by equation:
/// ```∀ t ∈ [0..1],  P(t) = (1 - t)² * from + 2 * (1 - t) * t * ctrl + 2 * t² * to```
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct QuadraticBezierSegment<S> {
    pub from: Point<S>,
    pub ctrl: Point<S>,
    pub to: Point<S>,
}

impl<S: Scalar> QuadraticBezierSegment<S> {
    /// Sample the curve at t (expecting t between 0 and 1).
    pub fn sample(&self, t: S) -> Point<S> {
        let t2 = t * t;
        let one_t = S::ONE - t;
        let one_t2 = one_t * one_t;

        self.from * one_t2
            + self.ctrl.to_vector() * S::TWO * one_t * t
            + self.to.to_vector() * t2
    }

    /// Sample the x coordinate of the curve at t (expecting t between 0 and 1).
    pub fn x(&self, t: S) -> S {
        let t2 = t * t;
        let one_t = S::ONE - t;
        let one_t2 = one_t * one_t;

        self.from.x * one_t2 + self.ctrl.x * S::TWO * one_t * t + self.to.x * t2
    }

    /// Sample the y coordinate of the curve at t (expecting t between 0 and 1).
    pub fn y(&self, t: S) -> S {
        let t2 = t * t;
        let one_t = S::ONE - t;
        let one_t2 = one_t * one_t;

        self.from.y * one_t2 + self.ctrl.y * S::TWO * one_t * t + self.to.y * t2
    }

    #[inline]
    fn derivative_coefficients(&self, t: S) -> (S, S, S) {
        (S::TWO * t - S::TWO, -S::FOUR * t + S::TWO, S::TWO * t)
    }

    /// Sample the curve's derivative at t (expecting t between 0 and 1).
    pub fn derivative(&self, t: S) -> Vector<S> {
        let (c0, c1, c2) = self.derivative_coefficients(t);
        self.from.to_vector() * c0 + self.ctrl.to_vector() * c1 + self.to.to_vector() * c2
    }

    /// Sample the x coordinate of the curve's derivative at t (expecting t between 0 and 1).
    pub fn dx(&self, t: S) -> S {
        let (c0, c1, c2) = self.derivative_coefficients(t);
        self.from.x * c0 + self.ctrl.x * c1 + self.to.x * c2
    }

    /// Sample the y coordinate of the curve's derivative at t (expecting t between 0 and 1).
    pub fn dy(&self, t: S) -> S {
        let (c0, c1, c2) = self.derivative_coefficients(t);
        self.from.y * c0 + self.ctrl.y * c1 + self.to.y * c2
    }

    /// Swap the beginning and the end of the segment.
    pub fn flip(&self) -> Self {
        QuadraticBezierSegment {
            from: self.to,
            ctrl: self.ctrl,
            to: self.from,
        }
    }

    /// Find the advancement of the y-most position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual y position.
    pub fn y_maximum_t(&self) -> S {
        if let Some(t) = self.local_y_extremum_t() {
            let y = self.y(t);
            if y > self.from.y && y > self.to.y {
                return t;
            }
        }

        if self.from.y > self.to.y { S::ZERO } else { S::ONE }
    }

    /// Find the advancement of the y-least position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual y position.
    pub fn y_minimum_t(&self) -> S {
        if let Some(t) = self.local_y_extremum_t() {
            let y = self.y(t);
            if y < self.from.y && y < self.to.y {
                return t;
            }
        }

        if self.from.y < self.to.y { S::ZERO } else { S::ONE }
    }

    /// Return the y inflection point or None if this curve is y-monotonic.
    pub fn local_y_extremum_t(&self) -> Option<S> {
        let div = self.from.y - S::TWO * self.ctrl.y + self.to.y;
        if div == S::ZERO {
            return None;
        }
        let t = (self.from.y - self.ctrl.y) / div;
        if t > S::ZERO && t < S::ONE {
            return Some(t);
        }

        None
    }

    /// Find the advancement of the x-most position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual x position.
    pub fn x_maximum_t(&self) -> S {
        if let Some(t) = self.local_x_extremum_t() {
            let x = self.x(t);
            if x > self.from.x && x > self.to.x {
                return t;
            }
        }

        if self.from.x > self.to.x { S::ZERO } else { S::ONE }
    }

    /// Find the advancement of the x-least position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual x position.
    pub fn x_minimum_t(&self) -> S {
        if let Some(t) = self.local_x_extremum_t() {
            let x = self.x(t);
            if x < self.from.x && x < self.to.x {
                return t;
            }
        }

        if self.from.x < self.to.x { S::ZERO } else { S::ONE }
    }

    /// Return the x inflection point or None if this curve is x-monotonic.
    pub fn local_x_extremum_t(&self) -> Option<S> {
        let div = self.from.x - S::TWO * self.ctrl.x + self.to.x;
        if div == S::ZERO {
            return None;
        }
        let t = (self.from.x - self.ctrl.x) / div;
        if t > S::ZERO && t < S::ONE {
            return Some(t);
        }

        None
    }

    /// Return the sub-curve inside a given range of t.
    ///
    /// This is equivalent splitting at the range's end points.
    pub fn split_range(&self, t_range: Range<S>) -> Self {
        let t0 = t_range.start;
        let t1 = t_range.end;

        let from = self.sample(t0);
        let to = self.sample(t1);
        let ctrl = from + (self.ctrl - self.from).lerp(self.to - self.ctrl, t0) * (t1 - t0);

        QuadraticBezierSegment { from, ctrl, to }
    }

    /// Split this curve into two sub-curves.
    pub fn split(&self, t: S) -> (QuadraticBezierSegment<S>, QuadraticBezierSegment<S>) {
        let split_point = self.sample(t);

        (
            QuadraticBezierSegment {
                from: self.from,
                ctrl: self.from.lerp(self.ctrl, t),
                to: split_point,
            },
            QuadraticBezierSegment {
                from: split_point,
                ctrl: self.ctrl.lerp(self.to, t),
                to: self.to,
            },
        )
    }

    /// Return the curve before the split point.
    pub fn before_split(&self, t: S) -> QuadraticBezierSegment<S> {
        QuadraticBezierSegment {
            from: self.from,
            ctrl: self.from.lerp(self.ctrl, t),
            to: self.sample(t),
        }
    }

    /// Return the curve after the split point.
    pub fn after_split(&self, t: S) -> QuadraticBezierSegment<S> {
        QuadraticBezierSegment {
            from: self.sample(t),
            ctrl: self.ctrl.lerp(self.to, t),
            to: self.to,
        }
    }

    /// Elevate this curve to a third order bézier.
    pub fn to_cubic(&self) -> CubicBezierSegment<S> {
        CubicBezierSegment {
            from: self.from,
            ctrl1: (self.from + self.ctrl.to_vector() * S::TWO) / S::THREE,
            ctrl2: (self.to + self.ctrl.to_vector() * S::TWO) / S::THREE,
            to: self.to,
        }
    }

    #[inline]
    pub fn baseline(&self) -> LineSegment<S> {
        LineSegment {
            from: self.from,
            to: self.to,
        }
    }

    pub fn is_linear(&self, tolerance: S) -> bool {
        let epsilon = S::EPSILON;
        if (self.from - self.to).square_length() < epsilon {
            return false;
        }
        let line = self.baseline().to_line().equation();

        line.distance_to_point(&self.ctrl) < tolerance
    }

    /// Computes a "fat line" of this segment.
    ///
    /// A fat line is two conservative lines between which the segment
    /// is fully contained.
    pub fn fat_line(&self) -> (LineEquation<S>, LineEquation<S>) {
        let l1 = self.baseline().to_line().equation();
        let d = S::HALF * l1.signed_distance_to_point(&self.ctrl);
        let l2 = l1.offset(d);
        if d >= S::ZERO {
            (l1, l2)
        } else {
            (l2, l1)
        }
    }

    /// Applies the transform to this curve and returns the results.
    #[inline]
    pub fn transformed<T: Transformation<S>>(&self, transform: &T) -> Self {
        QuadraticBezierSegment {
            from: transform.transform_point(self.from),
            ctrl: transform.transform_point(self.ctrl),
            to: transform.transform_point(self.to),
        }
    }

    /// Find the interval of the beginning of the curve that can be approximated with a
    /// line segment.
    pub fn flattening_step(&self, tolerance: S) -> S {
        let v1 = self.ctrl - self.from;
        let v2 = self.to - self.from;

        let v1_cross_v2 = v2.x * v1.y - v2.y * v1.x;
        let h = v1.x.hypot(v1.y);

        if S::abs(v1_cross_v2 * h) <= S::EPSILON {
            return S::ONE;
        }

        let s2inv = h / v1_cross_v2;

        let t = S::TWO * S::sqrt(tolerance * S::abs(s2inv) / S::THREE);

        if t > S::ONE {
            return S::ONE;
        }

        t
    }

    /// Compute a flattened approximation of the curve, invoking a callback at
    /// each step.
    ///
    /// The callback takes the point on the curve at each step.
    ///
    /// This implements the algorithm described by Raph Levien at
    /// https://raphlinus.github.io/graphics/curves/2019/12/23/flatten-quadbez.html
    pub fn for_each_flattened<F>(&self, tolerance: S, callback: &mut F)
    where
        F: FnMut(Point<S>),
    {
        self.for_each_flattened_t(tolerance, &mut|t| {
            callback(self.sample(t))
        });
    }

    /// Compute a flattened approximation of the curve, invoking a callback at
    /// each step.
    ///
    /// The callback takes the curve parameter at each step.
    ///
    /// This implements the algorithm described by Raph Levien at
    /// https://raphlinus.github.io/graphics/curves/2019/12/23/flatten-quadbez.html
    pub fn for_each_flattened_t<F>(&self, tolerance: S, callback: &mut F)
    where
        F: FnMut(S),
    {
        let params = FlatteningParameters::from_curve(self, tolerance);
        if params.is_point {
            return;
        }

        let mut i = S::ONE;
        for _ in 1..params.count.to_u32().unwrap() {
            let t = params.t_at_iteration(i);
            i += S::ONE;

            callback(t);
        }

        callback(S::ONE);
    }

    /// Compute a flattened approximation of the curve, invoking a callback at
    /// each step.
    ///
    /// The callback takes the point and corresponding curve parameter at each step.
    ///
    /// This implements the algorithm described by Raph Levien at
    /// https://raphlinus.github.io/graphics/curves/2019/12/23/flatten-quadbez.html
    pub fn for_each_flattened_with_t<F>(&self, tolerance: S, callback: &mut F)
    where
        F: FnMut(Point<S>, S),
    {
        self.for_each_flattened_t(tolerance, &mut|t| {
            callback(self.sample(t), t)
        });
    }

    /// Returns the flattened representation of the curve as an iterator, starting *after* the
    /// current point.
    pub fn flattened(&self, tolerance: S) -> Flattened<S> {
        Flattened::new(self, tolerance)
    }
    /// Returns the flattened representation of the curve as an iterator, starting *after* the
    /// current point.
    pub fn flattened_t(&self, tolerance: S) -> FlattenedT<S> {
        FlattenedT::new(self, tolerance)
    }

    /// Invokes a callback between each monotonic part of the segment.
    pub fn for_each_monotonic_t<F>(&self, mut cb: F)
    where
        F: FnMut(S),
    {
        let mut t0 = self.local_x_extremum_t();
        let mut t1 = self.local_x_extremum_t();

        let swap = match (t0, t1) {
            (None, Some(_)) => true,
            (Some(tx), Some(ty)) => tx > ty,
            _ => false,
        };

        if swap {
            mem::swap(&mut t0, &mut t1);
        }

        if let Some(t) = t0 {
            if t < S::ONE {
                cb(t);
            }
        }

        if let Some(t) = t1 {
            if t < S::ONE {
                cb(t);
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

    pub fn for_each_monotonic<F>(&self, cb: &mut F)
    where
        F: FnMut(&Monotonic<QuadraticBezierSegment<S>>),
    {
        self.for_each_monotonic_range(|range| cb(&self.split_range(range).assume_monotonic()));
    }

    /// Compute the length of the segment using a flattened approximation.
    pub fn approximate_length(&self, tolerance: S) -> S {
        let mut from = self.from;
        let mut len = S::ZERO;
        self.for_each_flattened(tolerance, &mut |to| {
            len += (to - from).length();
            from = to;
        });

        len
    }

    /// Returns a triangle containing this curve segment.
    pub fn bounding_triangle(&self) -> Triangle<S> {
        Triangle {
            a: self.from,
            b: self.ctrl,
            c: self.to,
        }
    }

    /// Returns a conservative rectangle that contains the curve.
    pub fn fast_bounding_rect(&self) -> Rect<S> {
        let (min_x, max_x) = self.fast_bounding_range_x();
        let (min_y, max_y) = self.fast_bounding_range_y();

        rect(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// Returns a conservative range of x this curve is contained in.
    pub fn fast_bounding_range_x(&self) -> (S, S) {
        let min_x = self.from.x.min(self.ctrl.x).min(self.to.x);
        let max_x = self.from.x.max(self.ctrl.x).max(self.to.x);

        (min_x, max_x)
    }

    /// Returns a conservative range of y this curve is contained in.
    pub fn fast_bounding_range_y(&self) -> (S, S) {
        let min_y = self.from.y.min(self.ctrl.y).min(self.to.y);
        let max_y = self.from.y.max(self.ctrl.y).max(self.to.y);

        (min_y, max_y)
    }

    /// Returns the smallest rectangle the curve is contained in
    pub fn bounding_rect(&self) -> Rect<S> {
        let (min_x, max_x) = self.bounding_range_x();
        let (min_y, max_y) = self.bounding_range_y();

        rect(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// Returns the smallest range of x this curve is contained in.
    pub fn bounding_range_x(&self) -> (S, S) {
        let min_x = self.x(self.x_minimum_t());
        let max_x = self.x(self.x_maximum_t());

        (min_x, max_x)
    }

    /// Returns the smallest range of y this curve is contained in.
    pub fn bounding_range_y(&self) -> (S, S) {
        let min_y = self.y(self.y_minimum_t());
        let max_y = self.y(self.y_maximum_t());

        (min_y, max_y)
    }

    /// Cast this curve into a monotonic curve without checking that the monotonicity
    /// assumption is correct.
    pub fn assume_monotonic(&self) -> MonotonicQuadraticBezierSegment<S> {
        MonotonicQuadraticBezierSegment { segment: *self }
    }

    /// Returns whether this segment is monotonic on the x axis.
    pub fn is_x_monotonic(&self) -> bool {
        self.local_x_extremum_t().is_none()
    }

    /// Returns whether this segment is monotonic on the y axis.
    pub fn is_y_monotonic(&self) -> bool {
        self.local_y_extremum_t().is_none()
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
    pub fn line_intersections_t(&self, line: &Line<S>) -> ArrayVec<[S; 2]> {
        // TODO: a specific quadratic bézier vs line intersection function
        // would allow for better performance.
        let intersections = self.to_cubic().line_intersections_t(line);

        let mut result = ArrayVec::new();
        for t in intersections {
            result.push(t);
        }

        result
    }

    /// Computes the intersection points (if any) between this segment a line.
    pub fn line_intersections(&self, line: &Line<S>) -> ArrayVec<[Point<S>; 2]> {
        let intersections = self.to_cubic().line_intersections_t(line);

        let mut result = ArrayVec::new();
        for t in intersections {
            result.push(self.sample(t));
        }

        result
    }

    /// Computes the intersections (if any) between this segment a line segment.
    ///
    /// The result is provided in the form of the `t` parameters of each
    /// point along curve and segment. To get the intersection points, sample
    /// the segments at the corresponding values.
    pub fn line_segment_intersections_t(&self, segment: &LineSegment<S>) -> ArrayVec<[(S, S); 2]> {
        // TODO: a specific quadratic bézier vs line intersection function
        // would allow for better performance.
        let intersections = self.to_cubic().line_segment_intersections_t(&segment);
        assert!(intersections.len() <= 2);

        let mut result = ArrayVec::new();
        for t in intersections {
            result.push(t);
        }

        result
    }

    #[inline]
    pub fn from(&self) -> Point<S> {
        self.from
    }

    #[inline]
    pub fn to(&self) -> Point<S> {
        self.to
    }

    /// Computes the intersection points (if any) between this segment a line segment.
    pub fn line_segment_intersections(&self, segment: &LineSegment<S>) -> ArrayVec<[Point<S>; 2]> {
        let intersections = self.to_cubic().line_segment_intersections_t(&segment);
        assert!(intersections.len() <= 2);

        let mut result = ArrayVec::new();
        for (t, _) in intersections {
            result.push(self.sample(t));
        }

        result
    }
}

pub struct FlatteningParameters<S> {
    count: S,
    integral_from: S,
    integral_step: S,
    inv_integral_from: S,
    div_inv_integral_diff: S,
    is_point: bool,
}

impl<S: Scalar> FlatteningParameters<S> {
    // See https://raphlinus.github.io/graphics/curves/2019/12/23/flatten-quadbez.html
    // TODO: this does not handle having the control point aligned with the endpoints unless
    // it is between the endpoints.

    pub fn from_curve(curve: &QuadraticBezierSegment<S>, tolerance: S) -> Self {
        // Map the quadratic bézier segment to y = x^2 parabola.
        let ddx = S::TWO * curve.ctrl.x - curve.from.x - curve.to.x;
        let ddy = S::TWO * curve.ctrl.y - curve.from.y - curve.to.y;
        let cross = (curve.to.x - curve.from.x) * ddy - (curve.to.y - curve.from.y) * ddx;
        let parabola_from = ((curve.ctrl.x - curve.from.x) * ddx + (curve.ctrl.y - curve.from.y) * ddy) / cross;
        let parabola_to = ((curve.to.x - curve.ctrl.x) * ddx + (curve.to.y - curve.ctrl.y) * ddy) / cross;
        // Note, scale can be NaN, for example with straight lines. When it happens the NaN will
        // propagate to other parameters. We catch it all by setting the iteration count to zero
        // and leave the rest as garbage.
        let scale = cross.abs() / (ddx.hypot(ddy) * (parabola_to - parabola_from).abs());

        let integral_from = approx_parabola_integral(parabola_from);
        let integral_to = approx_parabola_integral(parabola_to);
        let integral_diff = integral_to - integral_from;

        let inv_integral_from = approx_parabola_inv_integral(integral_from);
        let inv_integral_to = approx_parabola_inv_integral(integral_to);
        let div_inv_integral_diff = S::ONE / (inv_integral_to - inv_integral_from);

        // We could store this as an integer but the generic code makes that awkward and we'll
        // use it as a scalar again while iterating, so it's kept as a scalar.
        let mut count = (S::HALF * integral_diff.abs() * (scale / tolerance).sqrt()).ceil();
        let mut is_point = false;
        // If count is NaN the curve can be approximated by a single straight line or a point.
        if !count.is_finite() {
            count = S::ZERO;
            is_point = (curve.to - curve.from).square_length() < tolerance * tolerance;
        }

        let integral_step = integral_diff / count;

        FlatteningParameters {
            integral_from,
            integral_step,
            inv_integral_from,
            div_inv_integral_diff,
            count,
            is_point,
        }
    }

    fn t_at_iteration(&self, iteration: S) -> S {
        let u = approx_parabola_inv_integral(self.integral_from + self.integral_step * iteration);
        let t = (u - self.inv_integral_from) * self.div_inv_integral_diff;

        t
    }
}

/// Compute an approximation to integral (1 + 4x^2) ^ -0.25 dx used in the flattening code.
fn approx_parabola_integral<S: Scalar>(x: S) -> S {
    let d = S::value(0.67);
    let quarter = S::HALF * S::HALF;
    x / (S::ONE - d + (d.powi(4) + quarter * x * x).sqrt().sqrt())
}

/// Approximate the inverse of the function above.
fn approx_parabola_inv_integral<S: Scalar>(x: S) -> S {
    let b = S::value(0.39);
    let quarter = S::HALF * S::HALF;
    x * (S::ONE - b + (b * b + quarter * x * x).sqrt())
}


/// A flattening iterator for quadratic bézier segments.
///
/// Yields points at each iteration.
pub struct Flattened<S> {
    curve: QuadraticBezierSegment<S>,
    params: FlatteningParameters<S>,
    i: S,
    done: bool,
}

impl<S: Scalar> Flattened<S> {
    #[inline]
    pub(crate) fn new(curve: &QuadraticBezierSegment<S>, tolerance: S) -> Self {
        let params = FlatteningParameters::from_curve(curve, tolerance);
        let done = params.is_point;

        Flattened {
            curve: *curve,
            params,
            i: S::ONE,
            done,
        }
    }
}

impl<S: Scalar> Iterator for Flattened<S> {
    type Item = Point<S>;

    #[inline]
    fn next(&mut self) -> Option<Point<S>> {
        if self.done {
            return None;
        }

        if self.i >= self.params.count - S::EPSILON {
            self.done = true;
            return Some(self.curve.to);
        }

        let t = self.params.t_at_iteration(self.i);
        self.i += S::ONE;

        Some(self.curve.sample(t))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = (self.params.count + S::ONE - self.i).to_usize().unwrap();
        (count, Some(count))
    }
}

/// A flattening iterator for quadratic bézier segments.
///
/// Yields the curve parameter at each iteration.
pub struct FlattenedT<S> {
    params: FlatteningParameters<S>,
    i: S,
    done: bool,
}

impl<S: Scalar> FlattenedT<S> {
    #[inline]
    pub(crate) fn new(curve: &QuadraticBezierSegment<S>, tolerance: S) -> Self {
        let params = FlatteningParameters::from_curve(curve, tolerance);
        FlattenedT {
            i: S::ONE,
            done: params.is_point,
            params,
        }
    }
}

impl<S: Scalar> Iterator for FlattenedT<S> {
    type Item = S;

    #[inline]
    fn next(&mut self) -> Option<S> {
        if self.done {
            return None;
        }

        if self.i > self.params.count - S::EPSILON {
            self.done = true;
            return Some(S::ONE);
        }

        let t = self.params.t_at_iteration(self.i);
        self.i += S::ONE;

        Some(t)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = (self.params.count + S::ONE - self.i).to_usize().unwrap();
        (count, Some(count))
    }
}

impl<S: Scalar> Segment for QuadraticBezierSegment<S> {
    impl_segment!(S);
}

impl<S: Scalar> BoundingRect for QuadraticBezierSegment<S> {
    type Scalar = S;
    fn bounding_rect(&self) -> Rect<S> {
        self.bounding_rect()
    }
    fn fast_bounding_rect(&self) -> Rect<S> {
        self.fast_bounding_rect()
    }
    fn bounding_range_x(&self) -> (S, S) {
        self.bounding_range_x()
    }
    fn bounding_range_y(&self) -> (S, S) {
        self.bounding_range_y()
    }
    fn fast_bounding_range_x(&self) -> (S, S) {
        self.fast_bounding_range_x()
    }
    fn fast_bounding_range_y(&self) -> (S, S) {
        self.fast_bounding_range_y()
    }
}

/// A monotonically increasing in x and y quadratic bézier curve segment
pub type MonotonicQuadraticBezierSegment<S> = Monotonic<QuadraticBezierSegment<S>>;

#[test]
fn bounding_rect_for_monotonic_quadratic_bezier_segment() {
    let a = QuadraticBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(0.0, 0.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_bounding_rect = rect(0.0, 0.0, 2.0, 0.0);

    let actual_bounding_rect = a.bounding_rect();

    assert!(expected_bounding_rect == actual_bounding_rect)
}

#[test]
fn fast_bounding_rect_for_quadratic_bezier_segment() {
    let a = QuadraticBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(1.0, 1.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_bounding_rect = rect(0.0, 0.0, 2.0, 1.0);

    let actual_bounding_rect = a.fast_bounding_rect();

    assert!(expected_bounding_rect == actual_bounding_rect)
}

#[test]
fn minimum_bounding_rect_for_quadratic_bezier_segment() {
    let a = QuadraticBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(1.0, 1.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_bounding_rect = rect(0.0, 0.0, 2.0, 0.5);

    let actual_bounding_rect = a.bounding_rect();

    assert!(expected_bounding_rect == actual_bounding_rect)
}

#[test]
fn y_maximum_t_for_simple_segment() {
    let a = QuadraticBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(1.0, 1.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_y_maximum = 0.5;

    let actual_y_maximum = a.y_maximum_t();

    assert!(expected_y_maximum == actual_y_maximum)
}

#[test]
fn local_y_extremum_for_simple_segment() {
    let a = QuadraticBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(1.0, 1.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_y_inflection = 0.5;

    match a.local_y_extremum_t() {
        Some(actual_y_inflection) => assert!(expected_y_inflection == actual_y_inflection),
        None => panic!(),
    }
}

#[test]
fn y_minimum_t_for_simple_segment() {
    let a = QuadraticBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(1.0, -1.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_y_minimum = 0.5;

    let actual_y_minimum = a.y_minimum_t();

    assert!(expected_y_minimum == actual_y_minimum)
}

#[test]
fn x_maximum_t_for_simple_segment() {
    let a = QuadraticBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(1.0, 1.0),
        to: Point::new(0.0, 2.0),
    };

    let expected_x_maximum = 0.5;

    let actual_x_maximum = a.x_maximum_t();

    assert!(expected_x_maximum == actual_x_maximum)
}

#[test]
fn local_x_extremum_for_simple_segment() {
    let a = QuadraticBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(1.0, 1.0),
        to: Point::new(0.0, 2.0),
    };

    let expected_x_inflection = 0.5;

    match a.local_x_extremum_t() {
        Some(actual_x_inflection) => assert!(expected_x_inflection == actual_x_inflection),
        None => panic!(),
    }
}

#[test]
fn x_minimum_t_for_simple_segment() {
    let a = QuadraticBezierSegment {
        from: Point::new(2.0, 0.0),
        ctrl: Point::new(1.0, 1.0),
        to: Point::new(2.0, 2.0),
    };

    let expected_x_minimum = 0.5;

    let actual_x_minimum = a.x_minimum_t();

    assert!(expected_x_minimum == actual_x_minimum)
}

#[test]
fn length_straight_line() {
    // Sanity check: aligned points so both these curves are straight lines
    // that go form (0.0, 0.0) to (2.0, 0.0).

    let len = QuadraticBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(1.0, 0.0),
        to: Point::new(2.0, 0.0),
    }
    .approximate_length(0.01);
    assert_eq!(len, 2.0);

    let len = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(1.0, 0.0),
        ctrl2: Point::new(1.0, 0.0),
        to: Point::new(2.0, 0.0),
    }
    .approximate_length(0.01);
    assert_eq!(len, 2.0);
}

#[test]
fn derivatives() {
    let c1 = QuadraticBezierSegment {
        from: Point::new(1.0, 1.0),
        ctrl: Point::new(2.0, 1.0),
        to: Point::new(2.0, 2.0),
    };

    assert_eq!(c1.dy(0.0), 0.0);
    assert_eq!(c1.dx(1.0), 0.0);
    assert_eq!(c1.dy(0.5), c1.dx(0.5));
}

#[test]
fn monotonic_solve_t_for_x() {
    let curve = QuadraticBezierSegment {
        from: Point::new(1.0, 1.0),
        ctrl: Point::new(5.0, 5.0),
        to: Point::new(10.0, 2.0),
    };

    let tolerance = 0.0001;

    for i in 0..10u32 {
        let t = i as f32 / 10.0;
        let p = curve.sample(t);
        let t2 = curve.assume_monotonic().solve_t_for_x(p.x);
        // t should be pretty close to t2 but the only guarantee we have and can test
        // against is that x(t) - x(t2) is within the specified tolerance threshold.
        let x_diff = curve.x(t) - curve.x(t2);
        assert!(f32::abs(x_diff) <= tolerance);
    }
}

#[test]
fn fat_line() {
    use crate::point;

    let c1 = QuadraticBezierSegment {
        from: point(1.0f32, 2.0),
        ctrl: point(1.0, 3.0),
        to: point(11.0, 12.0),
    };

    let (l1, l2) = c1.fat_line();

    for i in 0..100 {
        let t = i as f32 / 99.0;
        assert!(l1.signed_distance_to_point(&c1.sample(t)) >= -0.000001);
        assert!(l2.signed_distance_to_point(&c1.sample(t)) <= 0.000001);
    }
}

#[test]
fn is_linear() {
    let mut angle = 0.0;
    let center = Point::new(1000.0, -700.0);
    for _ in 0..100 {
        for i in 0..10 {
            let (sin, cos) = f64::sin_cos(angle);
            let endpoint = Vector::new(cos * 100.0, sin * 100.0);
            let curve = QuadraticBezierSegment {
                from: center - endpoint,
                ctrl: center + endpoint.lerp(-endpoint, i as f64 / 9.0),
                to: center + endpoint,
            };

            assert!(curve.is_linear(1e-10));
        }
        angle += 0.001;
    }
}

#[test]
fn test_flattening() {
    use crate::point;

    let c1 = QuadraticBezierSegment {
        from: point(0.0, 0.0),
        ctrl: point(5.0, 0.0),
        to: point(5.0, 5.0),
    };

    let c2 = QuadraticBezierSegment {
        from: point(0.0, 0.0),
        ctrl: point(50.0, 0.0),
        to: point(50.0, 50.0),
    };

    let c3 = QuadraticBezierSegment {
        from: point(0.0, 0.0),
        ctrl: point(100.0, 100.0),
        to: point(5.0, 0.0),
    };

    fn check_tolerance(curve: &QuadraticBezierSegment<f64>, tolerance: f64) {
        let mut c = curve.clone();
        loop {
            let t = c.flattening_step(tolerance);
            if t >= 1.0 {
                break;
            }
            let (before, after) = c.split(t);
            let mid_point = before.sample(0.5);
            let distance = before
                .baseline()
                .to_line()
                .equation()
                .distance_to_point(&mid_point);
            assert!(distance <= tolerance);
            c = after;
        }
    }

    check_tolerance(&c1, 1.0);
    check_tolerance(&c1, 0.1);
    check_tolerance(&c1, 0.01);
    check_tolerance(&c1, 0.001);
    check_tolerance(&c1, 0.0001);

    check_tolerance(&c2, 1.0);
    check_tolerance(&c2, 0.1);
    check_tolerance(&c2, 0.01);
    check_tolerance(&c2, 0.001);
    check_tolerance(&c2, 0.0001);

    check_tolerance(&c3, 1.0);
    check_tolerance(&c3, 0.1);
    check_tolerance(&c3, 0.01);
    check_tolerance(&c3, 0.001);
    check_tolerance(&c3, 0.0001);
}

#[test]
fn test_flattening_empty_curve() {
    use crate::point;

    let curve = QuadraticBezierSegment {
        from: point(0.0, 0.0),
        ctrl: point(0.0, 0.0),
        to: point(0.0, 0.0),
    };

    let mut iter = FlattenedT::new(&curve, 0.1);

    assert!(iter.next().is_none());

    let mut count: u32 = 0;
    curve.for_each_flattened(0.1, &mut |_| { count += 1 });
    assert_eq!(count, 0);
}

#[test]
fn test_flattening_straight_line() {
    use crate::point;

    let curve = QuadraticBezierSegment {
        from: point(0.0, 0.0),
        ctrl: point(10.0, 0.0),
        to: point(20.0, 0.0),
    };

    let mut iter = FlattenedT::new(&curve, 0.1);

    assert_eq!(iter.next(), Some(1.0));
    assert!(iter.next().is_none());

    let mut count: u32 = 0;
    curve.for_each_flattened(0.1, &mut |_| { count += 1 });
    assert_eq!(count, 1);
}
