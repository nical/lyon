use segment::{Segment, BoundingRect};
use math::{Point, Vector, Rect};
use std::ops::Range;
use arrayvec::ArrayVec;

pub trait MonotonicSegment {
    fn solve_t_for_x(&self, x: f32, t_range: Range<f32>, tolerance: f32) -> f32;
}

#[derive(Copy, Clone, Debug)]
pub struct Monotonic<S> {
    pub(crate) segment: S,
}

impl<S: Segment> Monotonic<S> {
    #[inline]
    pub fn segment(&self) -> &S { &self.segment }
    #[inline]
    pub fn from(&self) -> Point { self.segment.from() }
    #[inline]
    pub fn to(&self) -> Point { self.segment.to() }
    #[inline]
    pub fn sample(&self, t: f32) -> Point { self.segment.sample(t) }
    #[inline]
    pub fn x(&self, t: f32) -> f32 { self.segment.x(t) }
    #[inline]
    pub fn y(&self, t: f32) -> f32 { self.segment.y(t) }
    #[inline]
    pub fn derivative(&self, t: f32) -> Vector { self.segment.derivative(t) }
    #[inline]
    pub fn dx(&self, t: f32) -> f32 { self.segment.dx(t) }
    #[inline]
    pub fn dy(&self, t: f32) -> f32 { self.segment.dy(t) }
    #[inline]
    pub fn split_range(&self, t_range: Range<f32>) -> Self {
        Self { segment: self.segment.split_range(t_range) }
    }
    #[inline]
    pub fn split(&self, t: f32) -> (Self, Self) {
        let (a, b) = self.segment.split(t);
        (Self { segment: a }, Self { segment: b })
    }
    #[inline]
    pub fn before_split(&self, t: f32) -> Self {
        Self { segment: self.segment.before_split(t) }
    }
    #[inline]
    pub fn after_split(&self, t: f32) -> Self {
        Self { segment: self.segment.after_split(t) }
    }
    #[inline]
    pub fn flip(&self) -> Self {
        Self { segment: self.segment.flip() }
    }
    #[inline]
    pub fn approximate_length(&self, tolerance: f32) -> f32 {
        self.segment.approximate_length(tolerance)
    }

    pub fn solve_t_for_x(&self, x: f32, t_range: Range<f32>, tolerance: f32) -> f32 {
        self.solve_t(x, t_range, tolerance)
    }

    pub fn solve_y_for_x(&self, x: f32, t_range: Range<f32>, tolerance: f32) -> f32 {
        self.y(self.solve_t(x, t_range, tolerance))
    }
}

impl<S: Segment> MonotonicSegment for Monotonic<S> {
    fn solve_t_for_x(&self, x: f32, t_range: Range<f32>, tolerance: f32) -> f32 {
        self.solve_t(x, t_range, tolerance)
    }
}

impl<S: Segment> Segment for Monotonic<S> { impl_segment!(); }

impl<S: BoundingRect> BoundingRect for Monotonic<S> {
    fn bounding_rect(&self) -> Rect {
        // For monotonic segments the fast bounding rect approximation
        // is exact.
        self.segment.fast_bounding_rect()
    }
    fn fast_bounding_rect(&self) -> Rect {
        self.segment.fast_bounding_rect()
    }
    fn bounding_range_x(&self) -> (f32, f32) {
        self.segment.bounding_range_x()
    }
    fn bounding_range_y(&self) -> (f32, f32) {
        self.segment.bounding_range_y()
    }
    fn fast_bounding_range_x(&self) -> (f32, f32) {
        self.segment.fast_bounding_range_x()
    }
    fn fast_bounding_range_y(&self) -> (f32, f32) {
        self.segment.fast_bounding_range_y()
    }
}

trait MonotonicFunction {
    fn f(&self, t: f32) -> f32;
    fn df(&self, t: f32) -> f32;

    fn solve_t(&self, x: f32, t_range: Range<f32>, tolerance: f32) -> f32 {
        debug_assert!(t_range.start <= t_range.end);
        let from = self.f(t_range.start);
        let to = self.f(t_range.end);
        if x <= from {
            return t_range.start;
        }
        if x >= to {
            return t_range.end;
        }

        // Newton's method.
        let mut t = x - from / (to - from);
        for _ in 0..8 {
            let x2 = self.f(t);

            if (x2 - x).abs() <= tolerance {
                return t
            }

            let dx = self.df(t);

            if dx <= 1e-5 {
                break
            }

            t -= (x2 - x) / dx;
        }

        // Fall back to binary search.
        let mut min = t_range.start;
        let mut max = t_range.end;
        let mut t = 0.5;

        while min < max {
            let x2 = self.f(t);

            if (x2 - x).abs() < tolerance {
                return t;
            }

            if x > x2 {
                min = t;
            } else {
                max = t;
            }

            t = (max - min) * 0.5 + min;
        }

        return t;
    }
}

impl<S: Segment> MonotonicFunction for Monotonic<S> {
    fn f(&self, t: f32) -> f32 { self.x(t) }
    fn df(&self, t: f32) -> f32 { self.dx(t) }
}

/// Return the first intersection point (if any) of two monotonic curve
/// segments.
///
/// Both segments must be monotonically increasing in x.
pub fn monotonic_segment_intersecion<A, B>(
    a: &A, a_t_range: Range<f32>,
    b: &B, b_t_range: Range<f32>,
    tolerance: f32,
) -> Option<(f32, f32)>
where
    A: Segment + MonotonicSegment + BoundingRect,
    B: Segment + MonotonicSegment + BoundingRect,
{
    debug_assert!(a.from().x <= a.to().x);
    debug_assert!(b.from().x <= b.to().x);

    // We need to have a stricter tolerance in solve_t_for_x otherwise
    // the error accumulation becomes pretty bad.
    let tx_tolerance = tolerance * 0.1;

    let (a_min, a_max) = a.split_range(a_t_range).fast_bounding_range_x();
    let (b_min, b_max) = b.split_range(b_t_range).fast_bounding_range_x();

    if a_min > b_max || a_max < b_min {
        return None;
    }

    let mut min_x = f32::max(a_min, b_min);
    let mut max_x = f32::min(a_max, b_max);

    let mut t_min_a = a.solve_t_for_x(min_x, 0.0..1.0, tx_tolerance);
    let mut t_max_a = a.solve_t_for_x(max_x, t_min_a..1.0, tx_tolerance);
    let mut t_min_b = b.solve_t_for_x(min_x, 0.0..1.0, tx_tolerance);
    let mut t_max_b = b.solve_t_for_x(max_x, t_min_b..1.0, tx_tolerance);

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
        if f32::abs(y_max_a - y_max_b) < tolerance {
            return Some((t_max_a, t_max_b));
        }

        let mid_x = (min_x + max_x) * 0.5;
        let t_mid_a = a.solve_t_for_x(mid_x, t_min_a..t_max_a, tx_tolerance);
        let t_mid_b = b.solve_t_for_x(mid_x, t_min_b..t_max_b, tx_tolerance);

        let y_mid_a = a.y(t_mid_a);
        let y_min_a = a.y(t_min_a);

        let y_mid_b = b.y(t_mid_b);
        let y_min_b = b.y(t_min_b);

        let min_sign = f32::signum(y_min_a - y_min_b);
        let mid_sign = f32::signum(y_mid_a - y_mid_b);
        let max_sign = f32::signum(y_max_a - y_max_b);

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
pub fn monotonic_segment_intersecions<A, B>(
    a: &A, a_t_range: Range<f32>,
    b: &B, b_t_range: Range<f32>,
    tolerance: f32,
) -> ArrayVec<[(f32, f32); 2]>
where
    A: Segment + MonotonicSegment + BoundingRect,
    B: Segment + MonotonicSegment + BoundingRect,
{
    let (t1, t2) = match monotonic_segment_intersecion(
        a, a_t_range.clone(),
        b, b_t_range.clone(),
        tolerance
    ) {
        Some(intersection) => { intersection }
        None => { return ArrayVec::new(); }
    };

    let mut result = ArrayVec::new();
    result.push((t1, t2));

    match monotonic_segment_intersecion(
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
    use QuadraticBezierSegment;
    use math::point;

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
