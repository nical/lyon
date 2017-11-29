use segment::Segment;
use math::{Point, Vector};

pub trait XMonotoneSegment : Segment + Sized {
    fn solve_t_for_x(&self, x: f32, tolerance: f32) -> f32;

    fn solve_y_for_x(&self, x: f32, tolerance: f32) -> f32 {
        self.y(self.solve_t_for_x(x, tolerance))
    }
}

pub trait YMonotoneSegment : Segment + Sized {
    fn solve_t_for_y(&self, y: f32, tolerance: f32) -> f32;

    fn solve_x_for_y(&self, y: f32, tolerance: f32) -> f32 {
        self.x(self.solve_t_for_y(y, tolerance))
    }
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

    pub fn solve_t_for_x(&self, x: f32, tolerance: f32) -> f32 {
        self.solve_t(x, tolerance)
    }

    pub fn solve_y_for_x(&self, x: f32, tolerance: f32) -> f32 {
        self.y(self.solve_t(x, tolerance))
    }
}

impl<S: Segment> XMonotoneSegment for XMonotone<S> {
    fn solve_t_for_x(&self, x: f32, tolerance: f32) -> f32 {
        self.solve_t(x, tolerance)
    }
}

impl<S: Segment> Segment for XMonotone<S> { impl_segment!(); }

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

    pub fn solve_t_for_y(&self, y: f32, tolerance: f32) -> f32 {
        self.solve_t(y, tolerance)
    }

    pub fn solve_x_for_y(&self, y: f32, tolerance: f32) -> f32 {
        self.y(self.solve_t(y, tolerance))
    }
}



impl<S: Segment> YMonotoneSegment for YMonotone<S> {
    fn solve_t_for_y(&self, y: f32, tolerance: f32) -> f32 {
        self.solve_t(y, tolerance)
    }
}

impl<S: Segment> Segment for YMonotone<S> { impl_segment!(); }

trait MonotoneFunction {
    fn f(&self, t: f32) -> f32;
    fn df(&self, t: f32) -> f32;

    fn solve_t(&self, x: f32, tolerance: f32) -> f32 {
        let from = self.f(0.0);
        let to = self.f(1.0);
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
        let mut min = 0.0;
        let mut max = 1.0;
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
