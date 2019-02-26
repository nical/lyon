use crate::segment::{Segment, BoundingRect};
use crate::scalar::{Scalar, NumCast};
use crate::generic_math::{Point, Vector, Rect};
use crate::{QuadraticBezierSegment, CubicBezierSegment};
use std::ops::Range;
use arrayvec::ArrayVec;

use std::f64;

pub(crate) trait MonotonicSegment {
    type Scalar: Scalar;
    fn solve_t_for_x(&self, x: Self::Scalar, t_range: Range<Self::Scalar>, tolerance: Self::Scalar) -> Self::Scalar;
}

/// A x and y monotonic curve segment, for example `Monotonic<QuadraticBezierSegment>`.
#[derive(Copy, Clone, Debug)]
pub struct Monotonic<T> {
    pub(crate) segment: T,
}

impl<T: Segment> Monotonic<T> {
    #[inline]
    pub fn segment(&self) -> &T { &self.segment }
    #[inline]
    pub fn from(&self) -> Point<T::Scalar> { self.segment.from() }
    #[inline]
    pub fn to(&self) -> Point<T::Scalar> { self.segment.to() }
    #[inline]
    pub fn sample(&self, t: T::Scalar) -> Point<T::Scalar> { self.segment.sample(t) }
    #[inline]
    pub fn x(&self, t: T::Scalar) -> T::Scalar { self.segment.x(t) }
    #[inline]
    pub fn y(&self, t: T::Scalar) -> T::Scalar { self.segment.y(t) }
    #[inline]
    pub fn derivative(&self, t: T::Scalar) -> Vector<T::Scalar> { self.segment.derivative(t) }
    #[inline]
    pub fn dx(&self, t: T::Scalar) -> T::Scalar { self.segment.dx(t) }
    #[inline]
    pub fn dy(&self, t: T::Scalar) -> T::Scalar { self.segment.dy(t) }
    #[inline]
    pub fn split_range(&self, t_range: Range<T::Scalar>) -> Self {
        Self { segment: self.segment.split_range(t_range) }
    }
    #[inline]
    pub fn split(&self, t: T::Scalar) -> (Self, Self) {
        let (a, b) = self.segment.split(t);
        (Self { segment: a }, Self { segment: b })
    }
    #[inline]
    pub fn before_split(&self, t: T::Scalar) -> Self {
        Self { segment: self.segment.before_split(t) }
    }
    #[inline]
    pub fn after_split(&self, t: T::Scalar) -> Self {
        Self { segment: self.segment.after_split(t) }
    }
    #[inline]
    pub fn flip(&self) -> Self {
        Self { segment: self.segment.flip() }
    }
    #[inline]
    pub fn approximate_length(&self, tolerance: T::Scalar) -> T::Scalar {
        self.segment.approximate_length(tolerance)
    }
}

impl<T: Segment> Segment for Monotonic<T> { impl_segment!(T::Scalar); }

impl<T: BoundingRect> BoundingRect for Monotonic<T> {
    type Scalar = T::Scalar;
    fn bounding_rect(&self) -> Rect<T::Scalar> {
        // For monotonic segments the fast bounding rect approximation
        // is exact.
        self.segment.fast_bounding_rect()
    }
    fn fast_bounding_rect(&self) -> Rect<T::Scalar> {
        self.segment.fast_bounding_rect()
    }
    fn bounding_range_x(&self) -> (T::Scalar, T::Scalar) {
        self.segment.bounding_range_x()
    }
    fn bounding_range_y(&self) -> (T::Scalar, T::Scalar) {
        self.segment.bounding_range_y()
    }
    fn fast_bounding_range_x(&self) -> (T::Scalar, T::Scalar) {
        self.segment.fast_bounding_range_x()
    }
    fn fast_bounding_range_y(&self) -> (T::Scalar, T::Scalar) {
        self.segment.fast_bounding_range_y()
    }
}

impl<S: Scalar> Monotonic<QuadraticBezierSegment<S>> {
    pub fn solve_t_for_x(&self, x: S) -> S {
        Self::solve_t(
            NumCast::from(self.segment.from.x).unwrap(),
            NumCast::from(self.segment.ctrl.x).unwrap(),
            NumCast::from(self.segment.to.x).unwrap(),
            NumCast::from(x).unwrap(),
        )
    }

    pub fn solve_t_for_y(&self, y: S) -> S {
        Self::solve_t(
            NumCast::from(self.segment.from.y).unwrap(),
            NumCast::from(self.segment.ctrl.y).unwrap(),
            NumCast::from(self.segment.to.y).unwrap(),
            NumCast::from(y).unwrap(),
        )
    }

    fn solve_t(from: f64, ctrl: f64, to: f64, x: f64) -> S {
        let a = from - 2.0 * ctrl + to;
        let b = -2.0 * from + 2.0 * ctrl;
        let c = from - x;

        let t = 2.0 * c / (-b - f64::sqrt(b * b - 4.0 * a * c));

        NumCast::from(t.max(0.0).min(1.0)).unwrap()
    }

    #[inline]
    pub fn split_at_x(&self, x: S) -> (Self, Self) {
        self.split(self.solve_t_for_x(x))
    }

    pub fn intersections_t(
        &self, self_t_range: Range<S>,
        other: &Self, other_t_range: Range<S>,
        tolerance: S,
    ) -> ArrayVec<[(S, S);2]> {
        monotonic_segment_intersecions(
            self, self_t_range,
            other, other_t_range,
            tolerance
        )
    }

    pub fn intersections(
        &self, self_t_range: Range<S>,
        other: &Self, other_t_range: Range<S>,
        tolerance: S,
    ) -> ArrayVec<[Point<S>;2]> {
        let intersections = monotonic_segment_intersecions(
            self, self_t_range,
            other, other_t_range,
            tolerance
        );
        let mut result = ArrayVec::new();
        for (t, _) in intersections {
            result.push(self.sample(t));
        }

        result
    }

    pub fn first_intersection_t(
        &self, self_t_range: Range<S>,
        other: &Self, other_t_range: Range<S>,
        tolerance: S,
    ) -> Option<(S, S)> {
        first_monotonic_segment_intersecion(
            self, self_t_range,
            other, other_t_range,
            tolerance
        )
    }

    pub fn first_intersection(
        &self, self_t_range: Range<S>,
        other: &Self, other_t_range: Range<S>,
        tolerance: S,
    ) -> Option<Point<S>> {
        first_monotonic_segment_intersecion(
            self, self_t_range,
            other, other_t_range,
            tolerance
        ).map(|(t, _)|{ self.sample(t) })
    }
}

impl<S: Scalar> MonotonicSegment for Monotonic<QuadraticBezierSegment<S>> {
    type Scalar = S;
    fn solve_t_for_x(&self, x: S, _t_range: Range<S>, _tolerance: S) -> S {
        self.solve_t_for_x(x)
    }
}

impl<S: Scalar> Monotonic<CubicBezierSegment<S>> {
    pub fn solve_t_for_x(&self, x: S, t_range: Range<S>, tolerance: S) -> S {
        debug_assert!(t_range.start <= t_range.end);
        let from = self.x(t_range.start);
        let to = self.x(t_range.end);
        if x <= from {
            return t_range.start;
        }
        if x >= to {
            return t_range.end;
        }

        // Newton's method.
        let mut t = x - from / (to - from);
        for _ in 0..8 {
            let x2 = self.x(t);

            if S::abs(x2 - x) <= tolerance {
                return t
            }

            let dx = self.dx(t);

            if dx <= S::EPSILON {
                break
            }

            t = t - (x2 - x) / dx;
        }

        // Fall back to binary search.
        let mut min = t_range.start;
        let mut max = t_range.end;
        let mut t = S::HALF;

        while min < max {
            let x2 = self.x(t);

            if S::abs(x2 - x) < tolerance {
                return t;
            }

            if x > x2 {
                min = t;
            } else {
                max = t;
            }

            t = (max - min) * S::HALF + min;
        }

        return t;
    }

    #[inline]
    pub fn split_at_x(&self, x: S) -> (Self, Self) {
        // TODO tolerance param.
        self.split(self.solve_t_for_x(x, S::ZERO..S::ONE, S::value(0.001)))
    }
}

impl<S: Scalar> MonotonicSegment for Monotonic<CubicBezierSegment<S>> {
    type Scalar = S;
    fn solve_t_for_x(&self, x: S, t_range: Range<S>, tolerance: S) -> S {
        self.solve_t_for_x(x, t_range, tolerance)
    }
}

/// Return the first intersection point (if any) of two monotonic curve
/// segments.
///
/// Both segments must be monotonically increasing in x.
pub(crate) fn first_monotonic_segment_intersecion<S: Scalar, A, B>(
    a: &A, a_t_range: Range<S>,
    b: &B, b_t_range: Range<S>,
    tolerance: S,
) -> Option<(S, S)>
where
    A: Segment<Scalar=S> + MonotonicSegment<Scalar=S> + BoundingRect<Scalar=S>,
    B: Segment<Scalar=S> + MonotonicSegment<Scalar=S> + BoundingRect<Scalar=S>,
{
    debug_assert!(a.from().x <= a.to().x);
    debug_assert!(b.from().x <= b.to().x);

    // We need to have a stricter tolerance in solve_t_for_x otherwise
    // the error accumulation becomes pretty bad.
    let tx_tolerance = tolerance / S::TEN;

    let (a_min, a_max) = a.split_range(a_t_range).fast_bounding_range_x();
    let (b_min, b_max) = b.split_range(b_t_range).fast_bounding_range_x();

    if a_min > b_max || a_max < b_min {
        return None;
    }

    let mut min_x = S::max(a_min, b_min);
    let mut max_x = S::min(a_max, b_max);

    let mut t_min_a = a.solve_t_for_x(min_x, S::ZERO..S::ONE, tx_tolerance);
    let mut t_max_a = a.solve_t_for_x(max_x, t_min_a..S::ONE, tx_tolerance);
    let mut t_min_b = b.solve_t_for_x(min_x, S::ZERO..S::ONE, tx_tolerance);
    let mut t_max_b = b.solve_t_for_x(max_x, t_min_b..S::ONE, tx_tolerance);

    const MAX_ITERATIONS: u32 = 32;
    for _ in 0..MAX_ITERATIONS {

        let y_max_a = a.y(t_max_a);
        let y_max_b = b.y(t_max_b);
        // It would seem more sensible to use the mid point instead of
        // the max point, but using the mid point means we don't know whether
        // the approximation will be slightly before or slightly after the
        // point.
        // Using the max point ensures that the we return an approximation
        // that is always slightly after the real intersection, which
        // means that if we search for intersections after the one we
        // found, we are not going to converge towards it again.
        if S::abs(y_max_a - y_max_b) < tolerance {
            return Some((t_max_a, t_max_b));
        }

        let mid_x = (min_x + max_x) * S::HALF;
        let t_mid_a = a.solve_t_for_x(mid_x, t_min_a..t_max_a, tx_tolerance);
        let t_mid_b = b.solve_t_for_x(mid_x, t_min_b..t_max_b, tx_tolerance);

        let y_mid_a = a.y(t_mid_a);
        let y_min_a = a.y(t_min_a);

        let y_mid_b = b.y(t_mid_b);
        let y_min_b = b.y(t_min_b);

        let min_sign = S::signum(y_min_a - y_min_b);
        let mid_sign = S::signum(y_mid_a - y_mid_b);
        let max_sign = S::signum(y_max_a - y_max_b);

        if min_sign != mid_sign {
            max_x = mid_x;
            t_max_a = t_mid_a;
            t_max_b = t_mid_b;
        } else if max_sign != mid_sign {
            min_x = mid_x;
            t_min_a = t_mid_a;
            t_min_b = t_mid_b;
        } else {
            // TODO: This is not always correct: if the min, max and mid
            // points are all on the same side, we consider that there is
            // no intersection, but there could be a pair of intersections
            // between the min/max and the mid point.
            break;
        }
    }

    None
}

/// Return the intersection points (if any) of two monotonic curve
/// segments.
///
/// Both segments must be monotonically increasing in x.
pub(crate) fn monotonic_segment_intersecions<S: Scalar, A, B>(
    a: &A, a_t_range: Range<S>,
    b: &B, b_t_range: Range<S>,
    tolerance: S,
) -> ArrayVec<[(S, S); 2]>
where
    A: Segment<Scalar=S> + MonotonicSegment<Scalar=S> + BoundingRect<Scalar=S>,
    B: Segment<Scalar=S> + MonotonicSegment<Scalar=S> + BoundingRect<Scalar=S>,
{
    let (t1, t2) = match first_monotonic_segment_intersecion(
        a, a_t_range.clone(),
        b, b_t_range.clone(),
        tolerance
    ) {
        Some(intersection) => { intersection }
        None => { return ArrayVec::new(); }
    };

    let mut result = ArrayVec::new();
    result.push((t1, t2));

    match first_monotonic_segment_intersecion(
        a, t1..a_t_range.end,
        b, t2..b_t_range.end,
        tolerance
    ) {
        Some(intersection) => { result.push(intersection); }
        None => {}
    }

    result
}

#[test]
fn two_intersections() {
    use crate::QuadraticBezierSegment;
    use crate::math::point;

    let c1 = QuadraticBezierSegment {
        from: point(10.0, 0.0),
        ctrl: point(10.0, 90.0),
        to: point(100.0, 90.0),
    }.assume_monotonic();
    let c2 = QuadraticBezierSegment {
        from: point(0.0, 10.0),
        ctrl: point(90.0, 10.0),
        to: point(90.0, 100.0),
    }.assume_monotonic();

    let intersections = monotonic_segment_intersecions(
        &c1, 0.0..1.0,
        &c2, 0.0..1.0,
        0.001,
    );

    assert_eq!(intersections.len(), 2);
    assert!(intersections[0].0 < 0.1, "{:?} < 0.1", intersections[0].0);
    assert!(intersections[1].1 > 0.9, "{:?} > 0.9", intersections[0].1);
}
