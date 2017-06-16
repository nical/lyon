use {CubicBezierSegment};
use {Point, Rect, rect};
use std::mem::swap;

/// A 2d curve segment defined by three points: the beginning of the segment, a control
/// point and the end of the segment.
///
/// The curve is defined by equation:
/// ```∀ t ∈ [0..1],  P(t) = (1 - t)² * from + 2 * (1 - t) * t * ctrl + 2 * t² * to```
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
        return self.from * one_t2 + self.ctrl.to_vector() * 2.0 * one_t * t + self.to.to_vector() * t2;
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

    /// Find the advancement of the y-least position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual y position.
    pub fn find_y_minimum(&self) -> f32 {
        if let Some(t) = self.find_y_inflection() {
            let p = self.sample(t);
            if p.y < self.from.y && p.y < self.to.y {
                return t;
            }
        }
        return if self.from.y < self.to.y { 0.0 } else { 1.0 };
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

    /// Find the advancement of the x-most position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual x position.
    pub fn find_x_maximum(&self) -> f32 {
        if let Some(t) = self.find_x_inflection() {
            let p = self.sample(t);
            if p.x > self.from.x && p.x > self.to.x {
                return t;
            }
        }
        return if self.from.x > self.to.x { 0.0 } else { 1.0 };
    }

    /// Find the advancement of the x-least position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual x position.
    pub fn find_x_minimum(&self) -> f32 {
        if let Some(t) = self.find_x_inflection() {
            let p = self.sample(t);
            if p.x < self.from.x && p.x < self.to.x {
                return t;
            }
        }
        return if self.from.x < self.to.x { 0.0 } else { 1.0 };
    }

    /// Return the x inflection point or None if this curve is x-monotone.
    pub fn find_x_inflection(&self) -> Option<f32> {
        let div = self.from.x - 2.0 * self.ctrl.x + self.to.x;
        if div == 0.0 {
            return None;
        }
        let t = (self.from.x - self.ctrl.x) / div;
        if t > 0.0 && t < 1.0 {
            return Some(t);
        }
        return None;
    }

    /// Split this curve into two sub-curves.
    pub fn split(&self, t: f32) -> (QuadraticBezierSegment, QuadraticBezierSegment) {
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
    pub fn before_split(&self, t: f32) -> QuadraticBezierSegment {
        return QuadraticBezierSegment {
            from: self.from,
            ctrl: self.from.lerp(self.ctrl, t),
            to: self.sample(t),
        };
    }

    /// Return the curve after the split point.
    pub fn after_split(&self, t: f32) -> QuadraticBezierSegment {
        return QuadraticBezierSegment {
            from: self.sample(t),
            ctrl: self.ctrl.lerp(self.to, t),
            to: self.to,
        };
    }

    /// Elevate this curve to a third order bézier.
    pub fn to_cubic(&self) -> CubicBezierSegment {
        CubicBezierSegment {
            from: self.from,
            ctrl1: (self.from + self.ctrl.to_vector() * 2.0) / 3.0,
            ctrl2: (self.to + self.ctrl.to_vector() * 2.0) / 3.0,
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
            len += (p - start).length();
            start = p;
        });
        return len;
    }

    pub fn bounding_rect(&self) -> Rect {
        let min_x = self.from.x.min(self.ctrl.x).min(self.to.x);
        let max_x = self.from.x.max(self.ctrl.x).max(self.to.x);
        let min_y = self.from.y.min(self.ctrl.y).min(self.to.y);
        let max_y = self.from.y.max(self.ctrl.y).max(self.to.y);

        return rect(min_x, min_y, max_x - min_x, max_y - min_y);
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

#[test]
fn bounding_rect_for_quadratic_bezier_segment() {
    let a = QuadraticBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(1.0, 1.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_bounding_rect = rect(0.0, 0.0, 2.0, 1.0);

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
fn find_y_inflection_for_simple_segment() {
    let a = QuadraticBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(1.0, 1.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_y_inflection = 0.5;

    match a.find_y_inflection() {
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
fn find_x_inflection_for_simple_segment() {
    let a = QuadraticBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(1.0, 1.0),
        to: Point::new(0.0, 2.0),
    };

    let expected_x_inflection = 0.5;

    match a.find_x_inflection() {
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
