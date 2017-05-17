//! # Bézier curve related maths and tools.
//!
//! This crate implements simple 2d quadratic and cubic bézier math and an efficient
//! flattening algorithm on top of [eulcid](https://crates.io/crates/euclid).
//!
//! # Flattening
//!
//! Flattening is the action of approximating a curve with a succession of line segments.
//!
//! <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 30" height="30mm" width="120mm">
//!   <path d="M26.7 24.94l.82-11.15M44.46 5.1L33.8 7.34" fill="none" stroke="#55d400" stroke-width=".5"/>
//!   <path d="M26.7 24.94c.97-11.13 7.17-17.6 17.76-19.84M75.27 24.94l1.13-5.5 2.67-5.48 4-4.42L88 6.7l5.02-1.6" fill="none" stroke="#000"/>
//!   <path d="M77.57 19.37a1.1 1.1 0 0 1-1.08 1.08 1.1 1.1 0 0 1-1.1-1.08 1.1 1.1 0 0 1 1.08-1.1 1.1 1.1 0 0 1 1.1 1.1" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M77.57 19.37a1.1 1.1 0 0 1-1.08 1.08 1.1 1.1 0 0 1-1.1-1.08 1.1 1.1 0 0 1 1.08-1.1 1.1 1.1 0 0 1 1.1 1.1" color="#000" fill="#fff"/>
//!   <path d="M80.22 13.93a1.1 1.1 0 0 1-1.1 1.1 1.1 1.1 0 0 1-1.08-1.1 1.1 1.1 0 0 1 1.1-1.08 1.1 1.1 0 0 1 1.08 1.08" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M80.22 13.93a1.1 1.1 0 0 1-1.1 1.1 1.1 1.1 0 0 1-1.08-1.1 1.1 1.1 0 0 1 1.1-1.08 1.1 1.1 0 0 1 1.08 1.08" color="#000" fill="#fff"/>
//!   <path d="M84.08 9.55a1.1 1.1 0 0 1-1.08 1.1 1.1 1.1 0 0 1-1.1-1.1 1.1 1.1 0 0 1 1.1-1.1 1.1 1.1 0 0 1 1.08 1.1" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M84.08 9.55a1.1 1.1 0 0 1-1.08 1.1 1.1 1.1 0 0 1-1.1-1.1 1.1 1.1 0 0 1 1.1-1.1 1.1 1.1 0 0 1 1.08 1.1" color="#000" fill="#fff"/>
//!   <path d="M89.1 6.66a1.1 1.1 0 0 1-1.08 1.1 1.1 1.1 0 0 1-1.08-1.1 1.1 1.1 0 0 1 1.08-1.08 1.1 1.1 0 0 1 1.1 1.08" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M89.1 6.66a1.1 1.1 0 0 1-1.08 1.1 1.1 1.1 0 0 1-1.08-1.1 1.1 1.1 0 0 1 1.08-1.08 1.1 1.1 0 0 1 1.1 1.08" color="#000" fill="#fff"/>
//!   <path d="M94.4 5a1.1 1.1 0 0 1-1.1 1.1A1.1 1.1 0 0 1 92.23 5a1.1 1.1 0 0 1 1.08-1.08A1.1 1.1 0 0 1 94.4 5" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M94.4 5a1.1 1.1 0 0 1-1.1 1.1A1.1 1.1 0 0 1 92.23 5a1.1 1.1 0 0 1 1.08-1.08A1.1 1.1 0 0 1 94.4 5" color="#000" fill="#fff"/>
//!   <path d="M76.44 25.13a1.1 1.1 0 0 1-1.1 1.1 1.1 1.1 0 0 1-1.08-1.1 1.1 1.1 0 0 1 1.1-1.1 1.1 1.1 0 0 1 1.08 1.1" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M76.44 25.13a1.1 1.1 0 0 1-1.1 1.1 1.1 1.1 0 0 1-1.08-1.1 1.1 1.1 0 0 1 1.1-1.1 1.1 1.1 0 0 1 1.08 1.1" color="#000" fill="#fff"/>
//!   <path d="M27.78 24.9a1.1 1.1 0 0 1-1.08 1.08 1.1 1.1 0 0 1-1.1-1.08 1.1 1.1 0 0 1 1.1-1.1 1.1 1.1 0 0 1 1.08 1.1" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M27.78 24.9a1.1 1.1 0 0 1-1.08 1.08 1.1 1.1 0 0 1-1.1-1.08 1.1 1.1 0 0 1 1.1-1.1 1.1 1.1 0 0 1 1.08 1.1" color="#000" fill="#fff"/>
//!   <path d="M45.4 5.14a1.1 1.1 0 0 1-1.08 1.1 1.1 1.1 0 0 1-1.1-1.1 1.1 1.1 0 0 1 1.1-1.08 1.1 1.1 0 0 1 1.1 1.08" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M45.4 5.14a1.1 1.1 0 0 1-1.08 1.1 1.1 1.1 0 0 1-1.1-1.1 1.1 1.1 0 0 1 1.1-1.08 1.1 1.1 0 0 1 1.1 1.08" color="#000" fill="#fff"/>
//!   <path d="M28.67 13.8a1.1 1.1 0 0 1-1.1 1.08 1.1 1.1 0 0 1-1.08-1.08 1.1 1.1 0 0 1 1.08-1.1 1.1 1.1 0 0 1 1.1 1.1" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M28.67 13.8a1.1 1.1 0 0 1-1.1 1.08 1.1 1.1 0 0 1-1.08-1.08 1.1 1.1 0 0 1 1.08-1.1 1.1 1.1 0 0 1 1.1 1.1" color="#000" fill="#fff"/>
//!   <path d="M35 7.32a1.1 1.1 0 0 1-1.1 1.1 1.1 1.1 0 0 1-1.08-1.1 1.1 1.1 0 0 1 1.1-1.1A1.1 1.1 0 0 1 35 7.33" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M35 7.32a1.1 1.1 0 0 1-1.1 1.1 1.1 1.1 0 0 1-1.08-1.1 1.1 1.1 0 0 1 1.1-1.1A1.1 1.1 0 0 1 35 7.33" color="#000" fill="#fff"/>
//!   <text style="line-height:6.61458302px" x="35.74" y="284.49" font-size="5.29" font-family="Sans" letter-spacing="0" word-spacing="0" fill="#b3b3b3" stroke-width=".26" transform="translate(19.595 -267)">
//!     <tspan x="35.74" y="284.49" font-size="10.58">→</tspan>
//!   </text>
//! </svg>
//!
//! The flattening algorithm implemented in this crate is based on the paper
//! [Fast, Precise Flattening of Cubic Bézier Segment Offset Curves](http://cis.usouthal.edu/~hain/general/Publications/Bezier/Bezier%20Offset%20Curves.pdf).
//! It tends to produce a better approximations than the usual recursive subdivision approach (or
//! in other words, it generates less segments for a given tolerance threshold).
//!
//! The tolerance threshold taken as input by the flattening algorithms corresponds
//! to the maximum distance between the curve and its linear approximation.
//! The smaller the tolerance is, the more precise the approximation and the more segments
//! are generated. This value is typically chosen in function of the zoom level.
//!
//! <svg viewBox="0 0 47.5 13.2" height="100" width="350" xmlns="http://www.w3.org/2000/svg">
//!   <path d="M-2.44 9.53c16.27-8.5 39.68-7.93 52.13 1.9" fill="none" stroke="#dde9af" stroke-width="4.6"/>
//!   <path d="M-1.97 9.3C14.28 1.03 37.36 1.7 49.7 11.4" fill="none" stroke="#00d400" stroke-width=".57" stroke-linecap="round" stroke-dasharray="4.6, 2.291434"/>
//!   <path d="M-1.94 10.46L6.2 6.08l28.32-1.4 15.17 6.74" fill="none" stroke="#000" stroke-width=".6"/>
//!   <path d="M6.83 6.57a.9.9 0 0 1-1.25.15.9.9 0 0 1-.15-1.25.9.9 0 0 1 1.25-.15.9.9 0 0 1 .15 1.25" color="#000" stroke="#000" stroke-width=".57" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M35.35 5.3a.9.9 0 0 1-1.25.15.9.9 0 0 1-.15-1.25.9.9 0 0 1 1.25-.15.9.9 0 0 1 .15 1.24" color="#000" stroke="#000" stroke-width=".6" stroke-opacity=".5"/>
//!   <g fill="none" stroke="#ff7f2a" stroke-width=".26">
//!     <path d="M20.4 3.8l.1 1.83M19.9 4.28l.48-.56.57.52M21.02 5.18l-.5.56-.6-.53" stroke-width=".2978872"/>
//!   </g>
//! </svg>
//!
//! The figure above shows a close up on a curve (the dotted line) and its linear
//! approximation (the black segments). The tolerance threshold is represented by
//! the light green area and the orange arrow.
//!

extern crate euclid;

mod flatten_cubic;
mod cubic_to_quadratic;

use std::mem::swap;
use flatten_cubic::flatten_cubic_bezier;
pub use flatten_cubic::CubicFlatteningIter;
pub use cubic_to_quadratic::cubic_to_quadratic;

/// Alias for ```euclid::Point2D<f32>```.
pub type Point = euclid::Point2D<f32>;

/// Alias for ```euclid::Point2D<f32>```.
pub type Vec2 = euclid::Point2D<f32>;

/// A 2d curve segment defined by three points: the beginning of the segment, a control
/// point and the end of the segment.
///
/// The curve is defined by equation:
/// ```∀ t ∈ [0..1],  P(t) = (1 - t)² * from + 2 * (1 - t) * t * ctrl + 2 * t * to```
#[derive(Copy, Clone, Debug)]
pub struct QuadraticBezierSegment {
    pub from: Point,
    pub ctrl: Point,
    pub to: Point,
}

impl QuadraticBezierSegment {
    /// Sample the curve at t (expecting t between 0 and 1).
    pub fn sample(&self, t: f32) -> Point {
        let t2 = t * t;
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        return self.from * one_t2 + self.ctrl * 2.0 * one_t * t + self.to * t2;
    }

    /// Sample the x coordinate of the curve at t (expecting t between 0 and 1).
    pub fn sample_x(&self, t: f32) -> f32 {
        let t2 = t * t;
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        return self.from.x * one_t2 + self.ctrl.x * 2.0 * one_t * t + self.to.x * t2;
    }

    /// Sample the y coordinate of the curve at t (expecting t between 0 and 1).
    pub fn sample_y(&self, t: f32) -> f32 {
        let t2 = t * t;
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        return self.from.y * one_t2 + self.ctrl.y * 2.0 * one_t * t + self.to.y * t2;
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
        return (QuadraticBezierSegment {
            from: self.from,
            ctrl: self.ctrl * t - self.from * t_one,
            to: split_point,
        },
        QuadraticBezierSegment {
            from: split_point,
            ctrl: self.to * t - self.ctrl * t_one,
            to: self.to,
        });
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
            to: self.to,
        };
    }

    /// Elevate this curve to a third order bézier.
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
                break;
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

    /// Compute the length of the segment using a flattened approximation.
    pub fn compute_length(&self, tolerance: f32) -> f32 {
        let mut start = self.from;
        let mut len = 0.0;
        self.flattened_for_each(tolerance, &mut|p| {
            len += vec2_length(p - start);
            start = p;
        });
        return len;
    }
}

/// An iterator over a quadratic bézier segment that yields line segments approximating the
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
    pub from: Point,
    pub ctrl1: Point,
    pub ctrl2: Point,
    pub to: Point,
}

impl CubicBezierSegment {
    /// Sample the curve at t (expecting t between 0 and 1).
    pub fn sample(&self, t: f32) -> Point {
        let t2 = t * t;
        let t3 = t2 * t;
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        let one_t3 = one_t2 * one_t;
        return self.from * one_t3 + self.ctrl1 * 3.0 * one_t2 * t +
                   self.ctrl2 * 3.0 * one_t * t2 + self.to * t3;
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
        };
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
        };
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

    /// Compute the length of the segment using a flattened approximation.
    pub fn compute_length(&self, tolerance: f32) -> f32 {
        let mut start = self.from;
        let mut len = 0.0;
        self.flattened_for_each(tolerance, &mut|p| {
            len += vec2_length(p - start);
            start = p;
        });
        return len;
    }
}

fn vec2_length(v: Vec2) -> f32 {
    (v.x * v.x + v.y * v.y).sqrt()
}

#[test]
fn length_straight_line() {
    // Sanity check: aligned points so both these curves are straight lines
    // that go form (0.0, 0.0) to (2.0, 0.0).

    let len = QuadraticBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(1.0, 0.0),
        to: Point::new(2.0, 0.0),
    }.compute_length(0.01);
    assert_eq!(len, 2.0);

    let len = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(1.0, 0.0),
        ctrl2: Point::new(1.0, 0.0),
        to: Point::new(2.0, 0.0),
    }.compute_length(0.01);
    assert_eq!(len, 2.0);
}
