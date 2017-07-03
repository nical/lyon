use {Point, Vec2, Rect, rect, Transform2D};
use up_to_two::UpToTwo;
use flatten_cubic::{flatten_cubic_bezier, find_cubic_bezier_inflection_points};
pub use flatten_cubic::CubicFlatteningIter;
pub use cubic_to_quadratic::cubic_to_quadratic;

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
        return self.from * one_t3 +
            self.ctrl1.to_vector() * 3.0 * one_t2 * t +
            self.ctrl2.to_vector() * 3.0 * one_t * t2 +
            self.to.to_vector() * t3;
    }

    /// Sample the x coordinate of the curve at t (expecting t between 0 and 1).
    pub fn sample_x(&self, t: f32) -> f32 {
        let t2 = t * t;
        let t3 = t2 * t;
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        let one_t3 = one_t2 * one_t;
        return self.from.x * one_t3 +
            self.ctrl1.x * 3.0 * one_t2 * t +
            self.ctrl2.x * 3.0 * one_t * t2 +
            self.to.x * t3;
    }

    /// Sample the y coordinate of the curve at t (expecting t between 0 and 1).
    pub fn sample_y(&self, t: f32) -> f32 {
        let t2 = t * t;
        let t3 = t2 * t;
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        let one_t3 = one_t2 * one_t;
        return self.from.y * one_t3 +
            self.ctrl1.y * 3.0 * one_t2 * t +
            self.ctrl2.y * 3.0 * one_t * t2 +
            self.to.y * t3;
    }

    #[inline]
    fn derivative_coefficients(&self, t: f32) -> (f32, f32, f32, f32) {
        let t2 = t*t;
        (
            - 3.0 * t2 + 6.0 * t - 3.0,
            9.0 * t2 - 12.0 * t + 3.0,
            - 9.0 * t2 + 6.0 * t,
            3.0 * t2
        )
    }

    /// Sample the curve's derivative at t (expecting t between 0 and 1).
    pub fn sample_derivative(&self, t: f32) -> Vec2 {
        let (c0, c1, c2, c3) = self.derivative_coefficients(t);
        self.from.to_vector() * c0 +
            self.ctrl1.to_vector() * c1 +
            self.ctrl2.to_vector() * c2 +
            self.to.to_vector() * c3
    }

    /// Sample the x coordinate of the curve's derivative at t (expecting t between 0 and 1).
    pub fn sample_x_derivative(&self, t: f32) -> f32 {
        let (c0, c1, c2, c3) = self.derivative_coefficients(t);
        self.from.x * c0 + self.ctrl1.x * c1 + self.ctrl2.x * c2 + self.to.x * c3
    }

    /// Sample the y coordinate of the curve's derivative at t (expecting t between 0 and 1).
    pub fn sample_y_derivative(&self, t: f32) -> f32 {
        let (c0, c1, c2, c3) = self.derivative_coefficients(t);
        self.from.y * c0 + self.ctrl1.y * c1 + self.ctrl2.y * c2 + self.to.y * c3
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

    /// [Not implemented] Applies the transform to this curve and returns the results.
    #[inline]
    #[must_use]
    pub fn transform(&self, transform: &Transform2D) -> Self {
        CubicBezierSegment {
            from: transform.transform_point(&self.from),
            ctrl1: transform.transform_point(&self.ctrl1),
            ctrl2: transform.transform_point(&self.ctrl2),
            to: transform.transform_point(&self.to)
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

    pub fn find_inflection_points(&self) -> UpToTwo<f32> {
        find_cubic_bezier_inflection_points(self)
    }

    /// Find the advancement of the y-most position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual y position.
    pub fn find_y_maximum(&self) -> f32 {
        let mut max_t = 0.0;
        let mut max_y = self.from.y;
        if self.to.y > max_y {
            max_t = 1.0;
            max_y = self.to.y;
        }
        for t in self.find_inflection_points() {
            let point = self.sample(t);
            if point.y > max_y {
                max_t = t;
                max_y = point.y;
            }
        }
        return max_t;
    }

    /// Find the advancement of the y-least position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual y position.
    pub fn find_y_minimum(&self) -> f32 {
        let mut min_t = 0.0;
        let mut min_y = self.from.y;
        if self.to.y < min_y {
            min_t = 1.0;
            min_y = self.to.y;
        }
        for t in self.find_inflection_points() {
            let point = self.sample(t);
            if point.y < min_y {
                min_t = t;
                min_y = point.y;
            }
        }
        return min_t;
    }

    /// Find the advancement of the x-most position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual x position.
    pub fn find_x_maximum(&self) -> f32 {
        let mut max_t = 0.0;
        let mut max_x = self.from.x;
        if self.to.x > max_x {
            max_t = 1.0;
            max_x = self.to.x;
        }
        for t in self.find_inflection_points() {
            let point = self.sample(t);
            if point.x > max_x {
                max_t = t;
                max_x = point.x;
            }
        }
        return max_t;
    }

    /// Find the advancement of the x-least position in the curve.
    ///
    /// This returns the advancement along the curve, not the actual x position.
    pub fn find_x_minimum(&self) -> f32 {
        let mut min_t = 0.0;
        let mut min_x = self.from.x;
        if self.to.x < min_x {
            min_t = 1.0;
            min_x = self.to.x;
        }
        for t in self.find_inflection_points() {
            let point = self.sample(t);
            if point.x < min_x {
                min_t = t;
                min_x = point.x;
            }
        }
        return min_t;
    }

    pub fn bounding_rect(&self) -> Rect {
        let min_x = self.from.x.min(self.ctrl1.x).min(self.ctrl2.x).min(self.to.x);
        let max_x = self.from.x.max(self.ctrl1.x).max(self.ctrl2.x).max(self.to.x);
        let min_y = self.from.y.min(self.ctrl1.y).min(self.ctrl2.y).min(self.to.y);
        let max_y = self.from.y.max(self.ctrl1.y).max(self.ctrl2.y).max(self.to.y);

        return rect(min_x, min_y, max_x - min_x, max_y - min_y);
    }
}

#[test]
fn bounding_rect_for_cubic_bezier_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.5, 1.0),
        ctrl2: Point::new(1.5, -1.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_bounding_rect = rect(0.0, -1.0, 2.0, 2.0);

    let actual_bounding_rect = a.bounding_rect();

    assert!(expected_bounding_rect == actual_bounding_rect)
}

#[test]
fn find_y_maximum_for_simple_cubic_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.5, 1.0),
        ctrl2: Point::new(1.5, 1.0),
        to: Point::new(2.0, 2.0),
    };

    let expected_y_maximum = 1.0;

    let actual_y_maximum = a.find_y_maximum();

    assert!(expected_y_maximum == actual_y_maximum, "got {}", actual_y_maximum)
}

#[test]
fn find_y_minimum_for_simple_cubic_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.5, 1.0),
        ctrl2: Point::new(1.5, 1.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_y_minimum = 0.0;

    let actual_y_minimum = a.find_y_minimum();

    assert!(expected_y_minimum == actual_y_minimum, "got {} ", actual_y_minimum)
}

#[test]
fn find_x_maximum_for_simple_cubic_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.5, 1.0),
        ctrl2: Point::new(1.5, 1.0),
        to: Point::new(2.0, 0.0),
    };
    let expected_x_maximum = 1.0;

    let actual_x_maximum = a.find_x_maximum();

    assert!(expected_x_maximum == actual_x_maximum, "got {}", actual_x_maximum)
}

#[test]
fn find_x_minimum_for_simple_cubic_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.5, 1.0),
        ctrl2: Point::new(1.5, 1.0),
        to: Point::new(2.0, 0.0),
    };

    let expected_x_minimum = 0.0;

    let actual_x_minimum = a.find_x_minimum();

    assert!(expected_x_minimum == actual_x_minimum, "got {} ", actual_x_minimum)
}

#[test]
fn derivatives() {
    let c1 = CubicBezierSegment {
        from: Point::new(1.0, 1.0,),
        ctrl1: Point::new(1.0, 2.0,),
        ctrl2: Point::new(2.0, 1.0,),
        to: Point::new(2.0, 2.0,),
    };

    println!(" -- {:?}", c1.sample_derivative(0.0));
    println!(" -- {:?}", c1.sample_derivative(1.0));
    println!(" -- {:?}", c1.sample_derivative(0.5));

    assert_eq!(c1.sample_x_derivative(0.0), 0.0);
    assert_eq!(c1.sample_x_derivative(1.0), 0.0);
    assert_eq!(c1.sample_y_derivative(0.5), 0.0);
}
