use crate::scalar::{Scalar, One};
use crate::generic_math::{Point, Vector, Rect};
use crate::{LineSegment, QuadraticBezierSegment, CubicBezierSegment};

use std::ops::Range;

/// Common APIs to segment types.
pub trait Segment: Copy + Sized {
    type Scalar: Scalar;

    /// Start of the curve.
    fn from(&self) -> Point<Self::Scalar>;

    /// End of the curve.
    fn to(&self) -> Point<Self::Scalar>;

    /// Sample the curve at t (expecting t between 0 and 1).
    fn sample(&self, t: Self::Scalar) -> Point<Self::Scalar>;

    /// Sample x at t (expecting t between 0 and 1).
    fn x(&self, t: Self::Scalar) -> Self::Scalar { self.sample(t).x }

    /// Sample y at t (expecting t between 0 and 1).
    fn y(&self, t: Self::Scalar) -> Self::Scalar { self.sample(t).y }

    /// Sample the derivative at t (expecting t between 0 and 1).
    fn derivative(&self, t: Self::Scalar) -> Vector<Self::Scalar>;

    /// Sample x derivative at t (expecting t between 0 and 1).
    fn dx(&self, t: Self::Scalar) -> Self::Scalar { self.derivative(t).x }

    /// Sample y derivative at t (expecting t between 0 and 1).
    fn dy(&self, t: Self::Scalar) -> Self::Scalar { self.derivative(t).y }

    /// Split this curve into two sub-curves.
    fn split(&self, t: Self::Scalar) -> (Self, Self);

    /// Return the curve before the split point.
    fn before_split(&self, t: Self::Scalar) -> Self;

    /// Return the curve after the split point.
    fn after_split(&self, t: Self::Scalar) -> Self;

    /// Return the curve inside a given range of t.
    ///
    /// This is equivalent splitting at the range's end points.
    fn split_range(&self, t_range: Range<Self::Scalar>) -> Self;

    /// Swap the direction of the segment.
    fn flip(&self) -> Self;

    /// Compute the length of the segment using a flattened approximation.
    fn approximate_length(&self, tolerance: Self::Scalar) -> Self::Scalar;
}

pub trait BoundingRect {
    type Scalar: Scalar;

    /// Returns a rectangle that contains the curve.
    fn bounding_rect(&self) -> Rect<Self::Scalar>;

    /// Returns a rectangle that contains the curve.
    ///
    /// This does not necessarily return the smallest possible bounding rectangle.
    fn fast_bounding_rect(&self) -> Rect<Self::Scalar> { self.bounding_rect() }

    /// Returns a range of x values that contains the curve.
    fn bounding_range_x(&self) -> (Self::Scalar, Self::Scalar);

    /// Returns a range of y values that contains the curve.
    fn bounding_range_y(&self) -> (Self::Scalar, Self::Scalar);

    /// Returns a range of x values that contains the curve.
    fn fast_bounding_range_x(&self) -> (Self::Scalar, Self::Scalar);

    /// Returns a range of y values that contains the curve.
    fn fast_bounding_range_y(&self) -> (Self::Scalar, Self::Scalar);
}

/// Types that implement local flattening approximation at the start of the curve.
pub trait FlatteningStep: Segment {
    /// Find the interval of the begining of the curve that can be approximated with a
    /// line segment.
    fn flattening_step(&self, tolerance: Self::Scalar) -> Self::Scalar;
}

pub(crate) fn for_each_flattened<T, F>(curve: &T, tolerance: T::Scalar, call_back: &mut F)
where
    T: FlatteningStep,
    F: FnMut(Point<T::Scalar>)
{
    let mut iter = curve.clone();
    loop {
        let t = iter.flattening_step(tolerance);
        if t >= T::Scalar::one() {
            call_back(iter.to());
            break;
        }
        iter = iter.after_split(t);
        call_back(iter.from());
    }
}

pub(crate) fn for_each_flattened_with_t<T, F>(curve: &T, tolerance: T::Scalar, call_back: &mut F)
where
    T: FlatteningStep,
    F: FnMut(Point<T::Scalar>, T::Scalar)
{
    let end = curve.to();
    let mut curve = curve.clone();
    let mut t0 = T::Scalar::ZERO;
    loop {
        let step = curve.flattening_step(tolerance);

        if step >= T::Scalar::ONE {
            break;
        }

        curve = curve.after_split(step);
        t0 += step * (T::Scalar::ONE - t0);
        call_back(curve.from(), t0);
    }

    call_back(end, T::Scalar::ONE);
}

/// An iterator over a generic curve segment that yields line segments approximating the
/// curve for a given approximation threshold.
///
/// The iterator starts at the first point *after* the origin of the curve and ends at the
/// destination.
pub struct Flattened<S, T> {
    curve: T,
    tolerance: S,
    done: bool,
}

impl<S: Scalar, T: FlatteningStep> Flattened<S, T> {
    pub fn new(curve: T, tolerance: S) -> Self {
        assert!(tolerance > S::ZERO);
        Flattened {
            curve: curve,
            tolerance: tolerance,
            done: false,
        }
    }
}
impl<S: Scalar, T: FlatteningStep<Scalar=S>> Iterator for Flattened<S, T>
{
    type Item = Point<S>;
    fn next(&mut self) -> Option<Point<S>> {
        if self.done {
            return None;
        }
        let t = self.curve.flattening_step(self.tolerance);
        if t >= S::ONE {
            self.done = true;
            return Some(self.curve.to());
        }
        self.curve = self.curve.after_split(t);
        return Some(self.curve.from());
    }
}

macro_rules! impl_segment {
    ($S:ty) => (
        type Scalar = $S;
        fn from(&self) -> Point<$S> { self.from() }
        fn to(&self) -> Point<$S> { self.to() }
        fn sample(&self, t: $S) -> Point<$S> { self.sample(t) }
        fn x(&self, t: $S) -> $S { self.x(t) }
        fn y(&self, t: $S) -> $S { self.y(t) }
        fn derivative(&self, t: $S) -> Vector<$S> { self.derivative(t) }
        fn dx(&self, t: $S) -> $S { self.dx(t) }
        fn dy(&self, t: $S) -> $S { self.dy(t) }
        fn split(&self, t: $S) -> (Self, Self) { self.split(t) }
        fn before_split(&self, t: $S) -> Self { self.before_split(t) }
        fn after_split(&self, t: $S) -> Self { self.after_split(t) }
        fn split_range(&self, t_range: Range<$S>) -> Self { self.split_range(t_range) }
        fn flip(&self) -> Self { self.flip() }
        fn approximate_length(&self, tolerance: $S) -> $S {
            self.approximate_length(tolerance)
        }
    )
}

/// Either a cubic, quadratic or linear bézier segment.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BezierSegment<S> {
    Linear(LineSegment<S>),
    Quadratic(QuadraticBezierSegment<S>),
    Cubic(CubicBezierSegment<S>),
}

impl<S: Scalar> BezierSegment<S> {
    #[inline]
    pub fn sample(&self, t: S) -> Point<S> {
        match self {
            BezierSegment::Linear(segment) => segment.sample(t),
            BezierSegment::Quadratic(segment) => segment.sample(t),
            BezierSegment::Cubic(segment) => segment.sample(t),
        }
    }

    #[inline]
    pub fn from(&self) -> Point<S> {
        match self {
            BezierSegment::Linear(segment) => segment.from,
            BezierSegment::Quadratic(segment) => segment.from,
            BezierSegment::Cubic(segment) => segment.from,
        }
    }

    #[inline]
    pub fn to(&self) -> Point<S> {
        match self {
            BezierSegment::Linear(segment) => segment.to,
            BezierSegment::Quadratic(segment) => segment.to,
            BezierSegment::Cubic(segment) => segment.to,
        }
    }

    #[inline]
    pub fn is_linear(&self, tolerance: S) -> bool {
        match self {
            BezierSegment::Linear(..) => true,
            BezierSegment::Quadratic(segment) => segment.is_linear(tolerance),
            BezierSegment::Cubic(segment) => segment.is_linear(tolerance),
        }
    }

    #[inline]
    pub fn baseline(&self) -> LineSegment<S> {
        match self {
            BezierSegment::Linear(segment) => *segment,
            BezierSegment::Quadratic(segment) => segment.baseline(),
            BezierSegment::Cubic(segment) => segment.baseline(),
        }
    }

    /// Split this segment into two sub-segments.
    pub fn split(&self, t: S) -> (BezierSegment<S>, BezierSegment<S>) {
        match self {
            BezierSegment::Linear(segment) => {
                let (a, b) = segment.split(t);
                (BezierSegment::Linear(a), BezierSegment::Linear(b))
            }
            BezierSegment::Quadratic(segment) => {
                let (a, b) = segment.split(t);
                (BezierSegment::Quadratic(a), BezierSegment::Quadratic(b))
            }
            BezierSegment::Cubic(segment) => {
                let (a, b) = segment.split(t);
                (BezierSegment::Cubic(a), BezierSegment::Cubic(b))
            }
        }
    }
}

impl<S> From<LineSegment<S>> for BezierSegment<S> {
    fn from(s: LineSegment<S>) -> Self {
        BezierSegment::Linear(s)
    }
}

impl<S> From<QuadraticBezierSegment<S>> for BezierSegment<S> {
    fn from(s: QuadraticBezierSegment<S>) -> Self {
        BezierSegment::Quadratic(s)
    }
}

impl<S> From<CubicBezierSegment<S>> for BezierSegment<S> {
    fn from(s: CubicBezierSegment<S>) -> Self {
        BezierSegment::Cubic(s)
    }
}

