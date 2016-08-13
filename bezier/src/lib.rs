//! Bezier curve related maths and tools.

extern crate euclid;

mod flatten_cubic;

use std::mem::swap;
use flatten_cubic::{ flatten_cubic_bezier, CubicFlattenIter };

pub type Point = euclid::Point2D<f32>;
pub type Vec2 = euclid::Point2D<f32>;

#[derive(Copy, Clone, Debug)]
pub struct QuadraticBezierSegment {
    pub from: Vec2,
    pub cp: Vec2,
    pub to: Vec2,
}

impl QuadraticBezierSegment {

    pub fn sample(&self, t: f32) -> Point {
        let t2 = t*t;
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        return self.from * one_t2
             + self.cp * 2.0 * one_t * t
             + self.to * t2;
    }

    pub fn sample_x(&self, t: f32) -> f32 {
        let t2 = t*t;
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        return self.from.x * one_t2
             + self.cp.x * 2.0*one_t*t
             + self.to.x * t2;
    }

    pub fn sample_y(&self, t: f32) -> f32 {
        let t2 = t*t;
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        return self.from.y * one_t2
             + self.cp.y * 2.0*one_t*t
             + self.to.y * t2;
    }

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
        let div = self.from.y - 2.0 * self.cp.y + self.to.y;
        if div == 0.0 {
           return None;
        }
        let t = (self.from.y - self.cp.y) / div;
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
                cp: self.cp * t - self.from * t_one,
                to: split_point,
            },
            QuadraticBezierSegment {
                from: split_point,
                cp: self.to * t - self.cp * t_one,
                to: self.to,
            }
        );
    }

    /// Return the curve before the split point.
    pub fn before_split(&self, t: f32) -> QuadraticBezierSegment {
        let t_one = t - 1.0;
        return QuadraticBezierSegment {
            from: self.from,
            cp: self.cp * t - self.from * t_one,
            to: self.sample(t),
        };
    }

    /// Return the curve after the split point.
    pub fn after_split(&self, t: f32) -> QuadraticBezierSegment {
        let t_one = t - 1.0;
        return QuadraticBezierSegment {
            from: self.sample(t),
            cp: self.to * t - self.cp * t_one,
            to: self.to
        };
    }

    /// Elevate this curve to a third order bezier.
    pub fn to_cubic(&self) -> CubicBezierSegment {
        CubicBezierSegment {
            from: self.from,
            cp1: (self.from + self.cp * 2.0) / 3.0,
            cp2: (self.to + self.cp * 2.0) / 3.0,
            to: self.to,
        }
    }

    /// Find the interval of the begining of the curve that can be approximated with a
    /// line segment.
    pub fn flattening_step(&self, tolerance: f32) -> f32 {
        let v1 = self.cp - self.from;
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
    pub fn flatten_iter(&self, tolerance: f32) -> QuadraticFlattenIter {
        QuadraticFlattenIter::new(*self, tolerance)
    }
}

pub struct QuadraticFlattenIter {
    curve: QuadraticBezierSegment,
    tolerance: f32,
    done: bool,
}

impl QuadraticFlattenIter {
    pub fn new(curve: QuadraticBezierSegment, tolerance: f32) -> Self {
        assert!(tolerance > 0.0);
        QuadraticFlattenIter {
            curve: curve,
            tolerance: tolerance,
            done: false,
        }
    }
}

impl Iterator for QuadraticFlattenIter {
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

#[derive(Copy, Clone, Debug)]
pub struct CubicBezierSegment {
    pub from: Vec2,
    pub cp1: Vec2,
    pub cp2: Vec2,
    pub to: Vec2,
}

impl CubicBezierSegment {
    pub fn sample(&self, t: f32) -> Vec2 {
        let t2 = t * t;
        let t3 = t2 * t;
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        let one_t3 = one_t2 * one_t;
        return self.from * one_t3
             + self.cp1 * 3.0 * one_t2 * t
             + self.cp2 * 3.0 * one_t * t2
             + self.to * t3;
    }

    /// Split this curve into two sub-curves.
    pub fn split(&self, t: f32) -> (CubicBezierSegment, CubicBezierSegment) {
        let cp1a = self.from + (self.cp1 - self.from) * t;
        let cp2a = self.cp1 + (self.cp2 - self.cp1) * t;
        let cp1aa = cp1a + (cp2a - cp1a) * t;
        let cp3a = self.cp2 + (self.to - self.cp2) * t;
        let cp2aa = cp2a + (cp3a - cp2a) * t;
        let cp1aaa = cp1aa + (cp2aa - cp1aa) * t;
        let to = self.to;

        return (
            CubicBezierSegment {
                from: self.from,
                cp1: cp1a,
                cp2: cp1aa,
                to: cp1aaa,
            },
            CubicBezierSegment {
                from: cp1aaa,
                cp1: cp2aa,
                cp2: cp3a,
                to: to,
            },
        );
    }

    /// Return the curve before the split point.
    pub fn before_split(&self, t: f32) -> CubicBezierSegment {
        let cp1a = self.from + (self.cp1 - self.from) * t;
        let cp2a = self.cp1 + (self.cp2 - self.cp1) * t;
        let cp1aa = cp1a + (cp2a - cp1a) * t;
        let cp3a = self.cp2 + (self.to - self.cp2) * t;
        let cp2aa = cp2a + (cp3a - cp2a) * t;
        let cp1aaa = cp1aa + (cp2aa - cp1aa) * t;
        return CubicBezierSegment {
            from: self.from,
            cp1: cp1a,
            cp2: cp1aa,
            to: cp1aaa,
        }
    }

    /// Return the curve after the split point.
    pub fn after_split(&self, t: f32) -> CubicBezierSegment {
        let cp1a = self.from + (self.cp1 - self.from) * t;
        let cp2a = self.cp1 + (self.cp2 - self.cp1) * t;
        let cp1aa = cp1a + (cp2a - cp1a) * t;
        let cp3a = self.cp2 + (self.to - self.cp2) * t;
        let cp2aa = cp2a + (cp3a - cp2a) * t;
        return CubicBezierSegment {
            from: cp1aa + (cp2aa - cp1aa) * t,
            cp1: cp2a + (cp3a - cp2a) * t,
            cp2: cp3a,
            to: self.to,
        }
    }

    /// Returns the flattened representation of the curve as an iterator, starting *after* the
    /// current point.
    pub fn flatten_iter(&self, tolerance: f32) -> CubicFlattenIter {
        CubicFlattenIter::new(*self, tolerance)
    }

    /// Iterates through the curve invoking a callback at each point.
    pub fn flattened_for_each<F: FnMut(Point)>(&self, tolerance: f32, call_back: &mut F) {
        flatten_cubic_bezier(*self, tolerance, call_back);
    }
}
