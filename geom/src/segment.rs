use math::{Point, Vector, Rect};

/// Common APIs to segment types.
pub trait Segment: Copy + Sized {
    /// Start of the curve.
    fn from(&self) -> Point;

    /// End of the curve.
    fn to(&self) -> Point;

    /// Sample the curve at t (expecting t between 0 and 1).
    fn sample(&self, t: f32) -> Point;

    /// Sample x at t (expecting t between 0 and 1).
    fn x(&self, t: f32) -> f32 { self.sample(t).x }

    /// Sample y at t (expecting t between 0 and 1).
    fn y(&self, t: f32) -> f32 { self.sample(t).y }

    /// Sample the derivative at t (expecting t between 0 and 1).
    fn derivative(&self, t: f32) -> Vector;

    /// Sample x derivative at t (expecting t between 0 and 1).
    fn dx(&self, t: f32) -> f32 { self.derivative(t).x }

    /// Sample y derivative at t (expecting t between 0 and 1).
    fn dy(&self, t: f32) -> f32 { self.derivative(t).y }

    /// Split this curve into two sub-curves.
    fn split(&self, t: f32) -> (Self, Self);

    /// Return the curve before the split point.
    fn before_split(&self, t: f32) -> Self;

    /// Return the curve after the split point.
    fn after_split(&self, t: f32) -> Self;

    /// Swap the direction of the segment.
    fn flip(&self) -> Self;

    /// Compute the length of the segment using a flattened approximation.
    fn approximate_length(&self, tolerance: f32) -> f32;
}

pub trait BoundingRect {
    /// Returns a rectangle that contains the curve.
    fn bounding_rect(&self) -> Rect;

    /// Returns a rectangle that contains the curve.
    ///
    /// This does not necessarily return the smallest possible bounding rectangle.
    fn fast_bounding_rect(&self) -> Rect { self.bounding_rect() }

    /// Returns a range of x values that contains the curve.
    fn bounding_range_x(&self) -> (f32, f32);

    /// Returns a range of y values that contains the curve.
    fn bounding_range_y(&self) -> (f32, f32);

    /// Returns a range of x values that contains the curve.
    fn fast_bounding_range_x(&self) -> (f32, f32);

    /// Returns a range of y values that contains the curve.
    fn fast_bounding_range_y(&self) -> (f32, f32);
}

/// Types that implement call-back based iteration
pub trait FlattenedForEach: Segment {
    /// Iterates through the curve invoking a callback at each point.
    fn flattened_for_each<F: FnMut(Point)>(&self, tolerance: f32, call_back: &mut F);
}

/// Types that implement local flattening approximation at the start of the curve.
pub trait FlatteningStep: FlattenedForEach {
    /// Find the interval of the begining of the curve that can be approximated with a
    /// line segment.
    fn flattening_step(&self, tolerance: f32) -> f32;

    /// Returns the flattened representation of the curve as an iterator, starting *after* the
    /// current point.
    fn flattened(self, tolerance: f32) -> Flattened<Self> {
        Flattened::new(self, tolerance)
    }
}

impl<T> FlattenedForEach for T
where T: FlatteningStep
{
    fn flattened_for_each<F: FnMut(Point)>(&self, tolerance: f32, call_back: &mut F) {
        let mut iter = *self;
        loop {
            let t = iter.flattening_step(tolerance);
            if t == 1.0 {
                call_back(iter.to());
                break;
            }
            iter = iter.after_split(t);
            call_back(iter.from());
        }
    }
}

/// An iterator over a generic curve segment that yields line segments approximating the
/// curve for a given approximation threshold.
///
/// The iterator starts at the first point *after* the origin of the curve and ends at the
/// destination.
pub struct Flattened<T> {
    curve: T,
    tolerance: f32,
    done: bool,
}

impl<T: FlatteningStep> Flattened<T> {
    pub fn new(curve: T, tolerance: f32) -> Self {
        assert!(tolerance > 0.0);
        Flattened {
            curve: curve,
            tolerance: tolerance,
            done: false,
        }
    }
}

impl<T: FlatteningStep> Iterator for Flattened<T> {
    type Item = Point;
    fn next(&mut self) -> Option<Point> {
        if self.done {
            return None;
        }
        let t = self.curve.flattening_step(self.tolerance);
        if t == 1.0 {
            self.done = true;
            return Some(self.curve.to());
        }
        self.curve = self.curve.after_split(t);
        return Some(self.curve.from());
    }
}

pub(crate) fn approximate_length_from_flattening<T>(curve: &T, tolerance: f32) -> f32
where T: FlattenedForEach {
    let mut start = curve.from();
    let mut len = 0.0;
    curve.flattened_for_each(tolerance, &mut|p| {
        len += (p - start).length();
        start = p;
    });
    return len;
}

macro_rules! impl_segment {
    () => (
        fn from(&self) -> Point { self.from() }
        fn to(&self) -> Point { self.to() }
        fn sample(&self, t: f32) -> Point { self.sample(t) }
        fn x(&self, t: f32) -> f32 { self.x(t) }
        fn y(&self, t: f32) -> f32 { self.y(t) }
        fn derivative(&self, t: f32) -> Vector { self.derivative(t) }
        fn dx(&self, t: f32) -> f32 { self.dx(t) }
        fn dy(&self, t: f32) -> f32 { self.dy(t) }
        fn split(&self, t: f32) -> (Self, Self) { self.split(t) }
        fn before_split(&self, t: f32) -> Self { self.before_split(t) }
        fn after_split(&self, t: f32) -> Self { self.after_split(t) }
        fn flip(&self) -> Self { self.flip() }
        fn approximate_length(&self, tolerance: f32) -> f32 {
            self.approximate_length(tolerance)
        }
    )
}