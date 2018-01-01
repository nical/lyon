use {CubicBezierSegment, Triangle, Line, LineSegment, LineEquation};
use scalar::{Float, FloatExt, FloatConst, Trig, ApproxEq};
use generic_math::{Point, Vector, Rect, rect, Transform2D};
use monotonic::Monotonic;
use arrayvec::ArrayVec;
use segment::{Segment, FlatteningStep, FlattenedForEach, BoundingRect};
use segment;

use std::ops::Range;

/// A flattening iterator for quadratic bézier segments.
pub type Flattened<S> = segment::Flattened<S, QuadraticBezierSegment<S>>;

/// A 2d curve segment defined by three points: the beginning of the segment, a control
/// point and the end of the segment.
///
/// The curve is defined by equation:
/// ```∀ t ∈ [0..1],  P(t) = (1 - t)² * from + 2 * (1 - t) * t * ctrl + 2 * t² * to```
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct QuadraticBezierSegment<S: Float> {
    pub from: Point<S>,
    pub ctrl: Point<S>,
    pub to: Point<S>,
}

impl<S: Float> QuadraticBezierSegment<S> {
    /// Sample the curve at t (expecting t between 0 and 1).
    pub fn sample(&self, t: S) -> Point<S> {
        let t2 = t * t;
        let one_t = S::one() - t;
        let one_t2 = one_t * one_t;
        return self.from * one_t2 + self.ctrl.to_vector() * S::c(2.0) * one_t * t + self.to.to_vector() * t2;
    }

    /// Sample the x coordinate of the curve at t (expecting t between 0 and 1).
    pub fn x(&self, t: S) -> S {
        let t2 = t * t;
        let one_t = S::one() - t;
        let one_t2 = one_t * one_t;
        return self.from.x * one_t2 + self.ctrl.x * S::c(2.0) * one_t * t + self.to.x * t2;
    }

    /// Sample the y coordinate of the curve at t (expecting t between 0 and 1).
    pub fn y(&self, t: S) -> S {
        let t2 = t * t;
        let one_t = S::one() - t;
        let one_t2 = one_t * one_t;
        return self.from.y * one_t2 + self.ctrl.y * S::c(2.0) * one_t * t + self.to.y * t2;
    }

    #[inline]
    fn derivative_coefficients(&self, t: S) -> (S, S, S) {
        (S::c(2.0) * t - S::c(2.0), S::c(-4.0) * t + S::c(2.0), S::c(2.0) * t)
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
    pub fn find_y_maximum(&self) -> S {
        if let Some(t) = self.find_local_y_extremum() {
            let p = self.sample(t);
            if p.y > self.from.y && p.y > self.to.y {
                return t;
            }
        }
        return if self.from.y > self.to.y { S::zero() } else { S::one() };
    }

    /// Find the advancement of the y-least position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual y position.
    pub fn find_y_minimum(&self) -> S {
        if let Some(t) = self.find_local_y_extremum() {
            let p = self.sample(t);
            if p.y < self.from.y && p.y < self.to.y {
                return t;
            }
        }
        return if self.from.y < self.to.y { S::zero() } else { S::one() };
    }

    /// Return the y inflection point or None if this curve is y-monotonic.
    pub fn find_local_y_extremum(&self) -> Option<S> {
        let div = self.from.y - S::c(2.0) * self.ctrl.y + self.to.y;
        if div == S::zero() {
            return None;
        }
        let t = (self.from.y - self.ctrl.y) / div;
        if t > S::zero() && t < S::one() {
            return Some(t);
        }
        return None;
    }

    /// Find the advancement of the x-most position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual x position.
    pub fn find_x_maximum(&self) -> S {
        if let Some(t) = self.find_local_x_extremum() {
            let p = self.sample(t);
            if p.x > self.from.x && p.x > self.to.x {
                return t;
            }
        }
        return if self.from.x > self.to.x { S::zero() } else { S::one() };
    }

    /// Find the advancement of the x-least position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual x position.
    pub fn find_x_minimum(&self) -> S {
        if let Some(t) = self.find_local_x_extremum() {
            let p = self.sample(t);
            if p.x < self.from.x && p.x < self.to.x {
                return t;
            }
        }
        return if self.from.x < self.to.x { S::zero() } else { S::one() };
    }

    /// Return the x inflection point or None if this curve is x-monotonic.
    pub fn find_local_x_extremum(&self) -> Option<S> {
        let div = self.from.x - S::c(2.0) * self.ctrl.x + self.to.x;
        if div == S::zero() {
            return None;
        }
        let t = (self.from.x - self.ctrl.x) / div;
        if t > S::zero() && t < S::one() {
            return Some(t);
        }
        return None;
    }

    /// Return the sub-curve inside a given range of t.
    ///
    /// This is equivalent splitting at the range's end points.
    pub fn split_range(&self, t_range: Range<S>) -> Self {
        let t1 = t_range.start;
        let t2 = t_range.end;

        debug_assert!(t1 >= S::zero());
        debug_assert!(t2 <= S::one());
        debug_assert!(t1 <= t2);
        debug_assert!(t1 != S::one());

        let from = self.sample(t1);
        let to = self.sample(t2);
        let a = self.from.lerp(self.ctrl, t1);
        let b = self.ctrl.lerp(self.to, t1);
        let ctrl = a.lerp(b, (t2 - t1) / (S::one() - t1));

        QuadraticBezierSegment { from, ctrl, to }
    }

    /// Split this curve into two sub-curves.
    pub fn split(&self, t: S) -> (QuadraticBezierSegment<S>, QuadraticBezierSegment<S>) {
        let split_point = self.sample(t);
        return (QuadraticBezierSegment {
            from: self.from,
            ctrl: self.from.lerp(self.ctrl, t),
            to: split_point,
        },
        QuadraticBezierSegment {
            from: split_point,
            ctrl: self.ctrl.lerp(self.to, t),
            to: self.to,
        });
    }

    /// Return the curve before the split point.
    pub fn before_split(&self, t: S) -> QuadraticBezierSegment<S> {
        return QuadraticBezierSegment {
            from: self.from,
            ctrl: self.from.lerp(self.ctrl, t),
            to: self.sample(t),
        };
    }

    /// Return the curve after the split point.
    pub fn after_split(&self, t: S) -> QuadraticBezierSegment<S> {
        return QuadraticBezierSegment {
            from: self.sample(t),
            ctrl: self.ctrl.lerp(self.to, t),
            to: self.to,
        };
    }

    /// Elevate this curve to a third order bézier.
    pub fn to_cubic(&self) -> CubicBezierSegment<S> {
        CubicBezierSegment {
            from: self.from,
            ctrl1: (self.from + self.ctrl.to_vector() * S::c(2.0)) / S::c(3.0),
            ctrl2: (self.to + self.ctrl.to_vector() * S::c(2.0)) / S::c(3.0),
            to: self.to,
        }
    }

    #[inline]
    pub fn baseline(&self) -> LineSegment<S> {
        LineSegment { from: self.from, to: self.to }
    }

    /// Computes a "fat line" of this segment.
    ///
    /// A fat line is two convervative lines between which the segment
    /// is fully contained.
    pub fn fat_line(&self) -> (LineEquation<S>, LineEquation<S>)
    where S : ApproxEq<S> {
        let l1 = self.baseline().to_line().equation();
        let d = l1.signed_distance_to_point(&self.ctrl);
        let two = S::one() + S::one();
        let l2 = l1.offset(d / two);

        if d >= S::zero() { (l1, l2) } else { (l2, l1) }
    }
}

impl<S: Float + Trig> QuadraticBezierSegment<S> {
    /// Applies the transform to this curve and returns the results.
    #[inline]
    pub fn transform(&self, transform: &Transform2D<S>) -> Self {
        QuadraticBezierSegment {
            from: transform.transform_point(&self.from),
            ctrl: transform.transform_point(&self.ctrl),
            to: transform.transform_point(&self.to)
        }
    }
}

impl<S: Float> QuadraticBezierSegment<S> {
    /// Find the interval of the begining of the curve that can be approximated with a
    /// line segment.
    pub fn flattening_step(&self, tolerance: S) -> S {
        let v1 = self.ctrl - self.from;
        let v2 = self.to - self.from;

        let v1_cross_v2 = v2.x * v1.y - v2.y * v1.x;
        let h = v1.x.hypot(v1.y);

        if (v1_cross_v2 * h).abs() <= S::c(0.000001) {
            return S::one();
        }

        let s2inv = h / v1_cross_v2;

        let t = S::c(2.0) * (tolerance * s2inv.abs() / S::c(3.0)).sqrt();

        if t > S::one() {
            return S::one();
        }

        return t;
    }
}

impl<S: Float + FloatConst + ApproxEq<S>> QuadraticBezierSegment<S> {
    /// Iterates through the curve invoking a callback at each point.
    pub fn flattened_for_each<F: FnMut(Point<S>)>(&self, tolerance: S, call_back: &mut F) {
        <Self as FlattenedForEach>::flattened_for_each(self, tolerance, call_back);
    }

    /// Returns the flattened representation of the curve as an iterator, starting *after* the
    /// current point.
    pub fn flattened(&self, tolerance: S) -> Flattened<S> {
        Flattened::new(*self, tolerance)
    }
}

impl<S: Float + FloatConst + ApproxEq<S>> QuadraticBezierSegment<S> {
    /// Compute the length of the segment using a flattened approximation.
    pub fn approximate_length(&self, tolerance: S) -> S {
        segment::approximate_length_from_flattening(self, tolerance)
    }
}

impl<S: Float> QuadraticBezierSegment<S> {
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

    pub fn fast_bounding_range_x(&self) -> (S, S) {
        let min_x = self.from.x.min(self.ctrl.x).min(self.to.x);
        let max_x = self.from.x.max(self.ctrl.x).max(self.to.x);
        (min_x, max_x)
    }

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

    pub fn bounding_range_x(&self) -> (S, S) {
        let min_x = self.sample(self.find_x_minimum()).x;
        let max_x = self.sample(self.find_x_maximum()).x;
        (min_x, max_x)
    }

    pub fn bounding_range_y(&self) -> (S, S) {
        let min_y = self.sample(self.find_y_minimum()).y;
        let max_y = self.sample(self.find_y_maximum()).y;
        (min_y, max_y)
    }

    /// Cast this curve into a monotonic curve without checking that the monotonicity
    /// assumption is correct.
    pub fn assume_monotonic(&self) -> MonotonicQuadraticBezierSegment<S> {
        MonotonicQuadraticBezierSegment { segment: *self }
    }
}

impl<S: Float + FloatConst + ApproxEq<S>> QuadraticBezierSegment<S> {
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

        return result;
    }

    /// Computes the intersection points (if any) between this segment a line.
    pub fn line_intersections(&self, line: &Line<S>) -> ArrayVec<[Point<S>;2]> {
        let intersections = self.to_cubic().line_intersections_t(line);

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
    pub fn line_segment_intersections_t(&self, segment: &LineSegment<S>) -> ArrayVec<[(S, S); 2]> {
        // TODO: a specific quadratic bézier vs line intersection function
        // would allow for better performance.
        let intersections = self.to_cubic().line_segment_intersections_t(&segment);
        assert!(intersections.len() <= 2);

        let mut result = ArrayVec::new();
        for t in intersections {
            result.push(t);
        }

        return result;
    }
}

impl<S: Float + FloatConst + ApproxEq<S>> QuadraticBezierSegment<S> {
    pub fn from(&self) -> Point<S> { self.from }

    pub fn to(&self) -> Point<S> { self.to }

    /// Computes the intersection points (if any) between this segment a line segment.
    pub fn line_segment_intersections(&self, segment: &LineSegment<S>) -> ArrayVec<[Point<S>; 2]> {
        let intersections = self.to_cubic().line_segment_intersections_t(&segment);
        assert!(intersections.len() <= 2);

        let mut result = ArrayVec::new();
        for (t, _) in intersections {
            result.push(self.sample(t));
        }

        return result;
    }
}

impl<S: Float + FloatConst + ApproxEq<S>> Segment for QuadraticBezierSegment<S> { impl_segment!(S); }

impl<S: Float> BoundingRect for QuadraticBezierSegment<S> {
    type Scalar = S;
    fn bounding_rect(&self) -> Rect<S> { self.bounding_rect() }
    fn fast_bounding_rect(&self) -> Rect<S> { self.fast_bounding_rect() }
    fn bounding_range_x(&self) -> (S, S) { self.bounding_range_x() }
    fn bounding_range_y(&self) -> (S, S) { self.bounding_range_y() }
    fn fast_bounding_range_x(&self) -> (S, S) { self.fast_bounding_range_x() }
    fn fast_bounding_range_y(&self) -> (S, S) { self.fast_bounding_range_y() }
}

impl<S: Float + FloatConst + ApproxEq<S>> FlatteningStep for QuadraticBezierSegment<S> {
    fn flattening_step(&self, tolerance: S) -> S {
        self.flattening_step(tolerance)
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
fn find_y_maximum_for_simple_segment() {
    let a = QuadraticBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(1.0, 1.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_y_maximum = 0.5;

    let actual_y_maximum = a.find_y_maximum();

    assert!(expected_y_maximum == actual_y_maximum)
}

#[test]
fn find_local_y_extremum_for_simple_segment() {
    let a = QuadraticBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(1.0, 1.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_y_inflection = 0.5;

    match a.find_local_y_extremum() {
        Some(actual_y_inflection) => assert!(expected_y_inflection == actual_y_inflection),
        None => panic!(),
    }
}

#[test]
fn find_y_minimum_for_simple_segment() {
    let a = QuadraticBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(1.0, -1.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_y_minimum = 0.5;

    let actual_y_minimum = a.find_y_minimum();

    assert!(expected_y_minimum == actual_y_minimum)
}

#[test]
fn find_x_maximum_for_simple_segment() {
    let a = QuadraticBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(1.0, 1.0),
        to: Point::new(0.0, 2.0),
    };

    let expected_x_maximum = 0.5;

    let actual_x_maximum = a.find_x_maximum();

    assert!(expected_x_maximum == actual_x_maximum)
}

#[test]
fn find_local_x_extremum_for_simple_segment() {
    let a = QuadraticBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(1.0, 1.0),
        to: Point::new(0.0, 2.0),
    };

    let expected_x_inflection = 0.5;

    match a.find_local_x_extremum() {
        Some(actual_x_inflection) => assert!(expected_x_inflection == actual_x_inflection),
        None => panic!(),
    }
}

#[test]
fn find_x_minimum_for_simple_segment() {
    let a = QuadraticBezierSegment {
        from: Point::new(2.0, 0.0),
        ctrl: Point::new(1.0, 1.0),
        to: Point::new(2.0, 2.0),
    };

    let expected_x_minimum = 0.5;

    let actual_x_minimum = a.find_x_minimum();

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
    }.approximate_length(0.01);
    assert_eq!(len, 2.0);

    let len = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(1.0, 0.0),
        ctrl2: Point::new(1.0, 0.0),
        to: Point::new(2.0, 0.0),
    }.approximate_length(0.01);
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
        assert!(x_diff.abs() <= tolerance);
    }
}
