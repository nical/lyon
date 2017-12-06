use segment::{Segment, BoundingRect};
use math::{Point, Vector, Rect};
use std::ops::Range;

pub trait XMonotoneSegment {
    fn solve_t_for_x(&self, x: f32, t_range: Range<f32>, tolerance: f32) -> f32;
}

pub trait YMonotoneSegment {
    fn solve_t_for_y(&self, y: f32, t_range: Range<f32>, tolerance: f32) -> f32;
}

#[derive(Copy, Clone, Debug)]
pub struct XMonotone<S> {
    pub(crate) segment: S,
}

impl<S: Segment> XMonotone<S> {
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

impl<S: Segment> XMonotoneSegment for XMonotone<S> {
    fn solve_t_for_x(&self, x: f32, t_range: Range<f32>, tolerance: f32) -> f32 {
        self.solve_t(x, t_range, tolerance)
    }
}

impl<S: Segment> Segment for XMonotone<S> { impl_segment!(); }

impl<S: BoundingRect> BoundingRect for XMonotone<S> {
    fn bounding_rect(&self) -> Rect {
        self.segment.bounding_rect()
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

#[derive(Copy, Clone, Debug)]
pub struct YMonotone<S> {
    pub(crate) segment: S,
}

impl<S: Segment> YMonotone<S> {
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

    pub fn solve_t_for_y(&self, y: f32, t_range: Range<f32>, tolerance: f32) -> f32 {
        self.solve_t(y, t_range, tolerance)
    }

    pub fn solve_x_for_y(&self, y: f32, t_range: Range<f32>, tolerance: f32) -> f32 {
        self.y(self.solve_t(y, t_range, tolerance))
    }
}

impl<S: Segment> YMonotoneSegment for YMonotone<S> {
    fn solve_t_for_y(&self, y: f32, t_range: Range<f32>, tolerance: f32) -> f32 {
        self.solve_t(y, t_range, tolerance)
    }
}

impl<S: Segment> Segment for YMonotone<S> { impl_segment!(); }

trait MonotoneFunction {
    fn f(&self, t: f32) -> f32;
    fn df(&self, t: f32) -> f32;

    fn solve_t(&self, x: f32, t_range: Range<f32>, tolerance: f32) -> f32 {
        debug_assert!(t_range.start <= t_range.end);
        let from = self.f(t_range.start);
        let to = self.f(t_range.end);
        if x <= from {
            return 0.0;
        }
        if x >= to {
            return 1.0;
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

impl<S: Segment> MonotoneFunction for XMonotone<S> {
    fn f(&self, t: f32) -> f32 { self.x(t) }
    fn df(&self, t: f32) -> f32 { self.dx(t) }
}

impl<S: Segment> MonotoneFunction for YMonotone<S> {
    fn f(&self, t: f32) -> f32 { self.y(t) }
    fn df(&self, t: f32) -> f32 { self.dy(t) }
}

// TODO: This returns at most one intersection but there could be two.
pub fn monotone_segment_intersecion<A, B>(
    a: &A,
    b: &B,
    tolerance: f32,
) -> Option<(f32, f32)>
where
    A: Segment + XMonotoneSegment + BoundingRect,
    B: Segment + XMonotoneSegment + BoundingRect,
{
    let (a_min, a_max) = a.fast_bounding_range_x();
    let (b_min, b_max) = b.fast_bounding_range_x();

    if a_min > b_max || a_max < b_min {
        return None;
    }

    let mut min_x = f32::max(a_min, b_min);
    let mut max_x = f32::min(a_max, b_max);

    let mut t_min_a = a.solve_t_for_x(min_x, 0.0..1.0, tolerance);
    let mut t_max_a = a.solve_t_for_x(max_x, t_min_a..1.0, tolerance);
    let mut t_min_b = b.solve_t_for_x(min_x, 0.0..1.0, tolerance);
    let mut t_max_b = b.solve_t_for_x(max_x, t_min_b..1.0, tolerance);

    const MAX_ITERATIONS: u32 = 32;
    for _ in 0..MAX_ITERATIONS {
        let mid_x = (min_x + max_x) * 0.5;
        let t_mid_a = a.solve_t_for_x(mid_x, t_min_a..t_max_a, tolerance);
        let t_mid_b = b.solve_t_for_x(mid_x, t_min_b..t_max_b, tolerance);
        let y_mid_a = a.y(t_mid_a);
        let y_mid_b = b.y(t_mid_b);

        if f32::abs(y_mid_a - y_mid_b) < tolerance * 0.5 {
            return Some((t_mid_a, t_mid_b));
        }

        let y_min_a = a.y(t_mid_a);
        let y_max_a = a.y(t_max_a);
        let y_min_b = b.y(t_min_b);
        let y_max_b = b.y(t_max_b);

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
