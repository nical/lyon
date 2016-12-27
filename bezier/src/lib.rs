//! Bezier curve related maths and tools.
//!
//! This crate implement simple 2d quadratic and cubic bezier math, and an efficient
//! flattening algorithm.
//!
//! The flatteing algorithm implemented here is the most interesting part. It is based on:
//! http://cis.usouthal.edu/~hain/general/Publications/Bezier/Bezier%20Offset%20Curves.pdf
//! It produces a better approximations than the usual recursive subdivision approach (or
//! in other words, it generates less points for a given tolerance threshold).

extern crate euclid;

mod flatten_cubic;
mod cubic_to_quadratic;

use std::mem::swap;
use flatten_cubic::flatten_cubic_bezier;
pub use flatten_cubic::CubicFlatteningIter;
pub use cubic_to_quadratic::cubic_to_quadratic;

pub type Point = euclid::Point2D<f32>;
pub type Vec2 = euclid::Point2D<f32>;

/// A 2d curve segment defined by three points: the beginning of the segment, a control
/// point and the end of the segment.
///
/// The curve is defined by equation:
/// ```∀ t ∈ [0..1],  P(t) = (1 - t)² * from + 2 * (1 - t) * t * ctrl + 2 * t * to```
#[derive(Copy, Clone, Debug)]
pub struct QuadraticBezierSegment {
    pub from: Vec2,
    pub ctrl: Vec2,
    pub to: Vec2,
}

impl QuadraticBezierSegment {

    /// Sample the curve at t (expecting t between 0 and 1).
    pub fn sample(&self, t: f32) -> Point {
        let t2 = t*t;
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        return self.from * one_t2
             + self.ctrl * 2.0 * one_t * t
             + self.to * t2;
    }

    /// Sample the x coordinate of the curve at t (expecting t between 0 and 1).
    pub fn sample_x(&self, t: f32) -> f32 {
        let t2 = t*t;
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        return self.from.x * one_t2
             + self.ctrl.x * 2.0*one_t*t
             + self.to.x * t2;
    }

    /// Sample the y coordinate of the curve at t (expecting t between 0 and 1).
    pub fn sample_y(&self, t: f32) -> f32 {
        let t2 = t*t;
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        return self.from.y * one_t2
             + self.ctrl.y * 2.0*one_t*t
             + self.to.y * t2;
    }

    /// Swap the beginning and the end of the segment.
    pub fn flip(&mut self) { swap(&mut self.from, &mut self.to); }

    /// Find the advancement of the y-most position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual y position.
    pub fn find_y_maximum(&self) -> f32 {
        if let Some(t) = self.find_y_inflection() {
            let p = self.sample(t);
            if p.y > self.from.y && p.y > self.to.y {
              return t;
            }
        }
        return if self.from.y > self.to.y { 0.0 } else { 1.0 };
    }

    /// Return the y inflection point or None if this curve is y-monotone.
    pub fn find_y_inflection(&self) -> Option<f32> {
        let div = self.from.y - 2.0 * self.ctrl.y + self.to.y;
        if div == 0.0 {
           return None;
        }
        let t = (self.from.y - self.ctrl.y) / div;
        if t > 0.0 && t < 1.0 {
            return Some(t);
        }
        return None;
    }

    /// Split this curve into two sub-curves.
    pub fn split(&self, t: f32) -> (QuadraticBezierSegment, QuadraticBezierSegment) {
        let t_one = t - 1.0;
        let split_point = self.sample(t);
        return (
            QuadraticBezierSegment {
                from: self.from,
                ctrl: self.ctrl * t - self.from * t_one,
                to: split_point,
            },
            QuadraticBezierSegment {
                from: split_point,
                ctrl: self.to * t - self.ctrl * t_one,
                to: self.to,
            }
        );
    }

    /// Return the curve before the split point.
    pub fn before_split(&self, t: f32) -> QuadraticBezierSegment {
        let t_one = t - 1.0;
        return QuadraticBezierSegment {
            from: self.from,
            ctrl: self.ctrl * t - self.from * t_one,
            to: self.sample(t),
        };
    }

    /// Return the curve after the split point.
    pub fn after_split(&self, t: f32) -> QuadraticBezierSegment {
        let t_one = t - 1.0;
        return QuadraticBezierSegment {
            from: self.sample(t),
            ctrl: self.to * t - self.ctrl * t_one,
            to: self.to
        };
    }

    /// Elevate this curve to a third order bezier.
    pub fn to_cubic(&self) -> CubicBezierSegment {
        CubicBezierSegment {
            from: self.from,
            ctrl1: (self.from + self.ctrl * 2.0) / 3.0,
            ctrl2: (self.to + self.ctrl * 2.0) / 3.0,
            to: self.to,
        }
    }

    /// Find the interval of the begining of the curve that can be approximated with a
    /// line segment.
    pub fn flattening_step(&self, tolerance: f32) -> f32 {
        let v1 = self.ctrl - self.from;
        let v2 = self.to - self.from;

        let v1_cross_v2 = v2.x * v1.y - v2.y * v1.x;
        let h = v1.x.hypot(v1.y);

        if (v1_cross_v2 * h).abs() <= 0.000001 {
            return 1.0;
        }

        let s2inv = h / v1_cross_v2;

        let t = 2.0 * (tolerance * s2inv.abs() / 3.0).sqrt();

        if t > 1.0 {
            return 1.0;
        }

        return t;
    }

    /// Iterates through the curve invoking a callback at each point.
    pub fn flattened_for_each<F: FnMut(Point)>(&self, tolerance: f32, call_back: &mut F) {
        let mut iter = *self;
        loop {
            let t = iter.flattening_step(tolerance);
            if t == 1.0 {
                call_back(iter.to);
                break
            }
            iter = iter.after_split(t);
            call_back(iter.from);
        }
    }

    /// Returns the flattened representation of the curve as an iterator, starting *after* the
    /// current point.
    pub fn flattening_iter(&self, tolerance: f32) -> QuadraticFlatteningIter {
        QuadraticFlatteningIter::new(*self, tolerance)
    }
}

/// An iterator over a quadratic bezier segment that yields line segments approximating the
/// curve for a given approximation threshold.
///
/// The iterator starts at the first point *after* the origin of the curve and ends at the
/// destination.
pub struct QuadraticFlatteningIter {
    curve: QuadraticBezierSegment,
    tolerance: f32,
    done: bool,
}

impl QuadraticFlatteningIter {
    pub fn new(curve: QuadraticBezierSegment, tolerance: f32) -> Self {
        assert!(tolerance > 0.0);
        QuadraticFlatteningIter {
            curve: curve,
            tolerance: tolerance,
            done: false,
        }
    }
}

impl Iterator for QuadraticFlatteningIter {
    type Item = Point;
    fn next(&mut self) -> Option<Point> {
        if self.done {
            return None;
        }
        let t = self.curve.flattening_step(self.tolerance);
        if t == 1.0 {
            self.done = true;
            return Some(self.curve.to);
        }
        self.curve = self.curve.after_split(t);
        return Some(self.curve.from);
    }
}

/// A 2d curve segment defined by four points: the beginning of the segment, two control
/// points and the end of the segment.
///
/// The curve is defined by equation:²
/// ```∀ t ∈ [0..1],  P(t) = (1 - t)³ * from + 3 * (1 - t)² * t * ctrl1 + 3 * t² * (1 - t) * ctrl2 + t³ * to```
#[derive(Copy, Clone, Debug)]
pub struct CubicBezierSegment {
    pub from: Vec2,
    pub ctrl1: Vec2,
    pub ctrl2: Vec2,
    pub to: Vec2,
}

impl CubicBezierSegment {
    /// Sample the curve at t (expecting t between 0 and 1).
    pub fn sample(&self, t: f32) -> Vec2 {
        let t2 = t * t;
        let t3 = t2 * t;
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        let one_t3 = one_t2 * one_t;
        return self.from * one_t3
             + self.ctrl1 * 3.0 * one_t2 * t
             + self.ctrl2 * 3.0 * one_t * t2
             + self.to * t3;
    }

    /// Split this curve into two sub-curves.
    pub fn split(&self, t: f32) -> (CubicBezierSegment, CubicBezierSegment) {
        let ctrl1a = self.from + (self.ctrl1 - self.from) * t;
        let ctrl2a = self.ctrl1 + (self.ctrl2 - self.ctrl1) * t;
        let ctrl1aa = ctrl1a + (ctrl2a - ctrl1a) * t;
        let ctrl3a = self.ctrl2 + (self.to - self.ctrl2) * t;
        let ctrl2aa = ctrl2a + (ctrl3a - ctrl2a) * t;
        let ctrl1aaa = ctrl1aa + (ctrl2aa - ctrl1aa) * t;
        let to = self.to;

        return (
            CubicBezierSegment {
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
            },
        );
    }

    /// Return the curve before the split point.
    pub fn before_split(&self, t: f32) -> CubicBezierSegment {
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
        }
    }

    /// Return the curve after the split point.
    pub fn after_split(&self, t: f32) -> CubicBezierSegment {
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
        }
    }

    /// Returns the flattened representation of the curve as an iterator, starting *after* the
    /// current point.
    pub fn flattening_iter(&self, tolerance: f32) -> CubicFlatteningIter {
        CubicFlatteningIter::new(*self, tolerance)
    }

    /// Iterates through the curve invoking a callback at each point.
    pub fn flattened_for_each<F: FnMut(Point)>(&self, tolerance: f32, call_back: &mut F) {
        flatten_cubic_bezier(*self, tolerance, call_back);
    }
}
