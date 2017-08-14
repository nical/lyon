use {Point, Vec2, Rect, rect, Line, LineSegment, Transform2D};
use up_to_two::UpToTwo;
use arrayvec::ArrayVec;
use flatten_cubic::{flatten_cubic_bezier, find_cubic_bezier_inflection_points};
pub use flatten_cubic::Flattened;
pub use cubic_to_quadratic::cubic_to_quadratic;
use monotone::{XMonotoneParametricCurve, solve_t_for_x};
use utils::cubic_polynomial_roots;

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

    /// Applies the transform to this curve and returns the results.
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
    pub fn flattened(&self, tolerance: f32) -> Flattened {
        Flattened::new(*self, tolerance)
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

    /// Return local x extrema or None if this curve is monotone.
    ///
    /// This returns the advancements along the curve, not the actual x position.
    pub fn find_local_x_extrema(&self) -> UpToTwo<f32> {
        let mut ret = UpToTwo::new();
        // See www.faculty.idc.ac.il/arik/quality/appendixa.html for an explanation
        // The derivative of a cubic bezier curve is a curve representing a second degree polynomial function
        // f(x) = a * x² + b * x + c such as :
        let a = 3.0 * (self.to.x - 3.0 * self.ctrl2.x + 3.0 * self.ctrl1.x - self.from.x);
        let b = 6.0 * (self.ctrl2.x - 2.0 * self.ctrl1.x + self.from.x);
        let c = 3.0 * (self.ctrl1.x - self.from.x);

        // If the derivative is a linear function
        if a == 0.0 {
            if b == 0.0 {
                // If the derivative is a constant function
                if c == 0.0 {
                    ret.push(0.0);
                }
            } else {
                ret.push(-c / b);
            }
            return ret;
        }

        fn in_range(t: f32) -> bool { t > 0.0 && t < 1.0 }

        let discriminant = b * b - 4.0 * a * c;

        // There is no Real solution for the equation
        if discriminant < 0.0 {
            return ret;
        }

        // There is one Real solution for the equation
        if discriminant == 0.0 {
            let t = -b / (2.0 * a);
            if in_range(t) {
                ret.push(t);
            }
            return ret;
        }

        // There are two Real solutions for the equation
        let discriminant_sqrt = discriminant.sqrt();

        let first_extremum = (-b - discriminant_sqrt) / (2.0 * a);
        let second_extremum = (-b + discriminant_sqrt) / (2.0 * a);

        if in_range(first_extremum) {
            ret.push(first_extremum);
        }

        if in_range(second_extremum) {
            ret.push(second_extremum);
        }
        ret
    }

    /// Return local y extrema or None if this curve is monotone.
    ///
    /// This returns the advancements along the curve, not the actual y position.
    pub fn find_local_y_extrema(&self) -> UpToTwo<f32> {
       let switched_segment = CubicBezierSegment {
               from: yx(self.from),
               ctrl1: yx(self.ctrl1),
               ctrl2: yx(self.ctrl2),
               to: yx(self.to),
       };

        switched_segment.find_local_x_extrema()
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
        for t in self.find_local_y_extrema() {
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
        for t in self.find_local_y_extrema() {
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
        for t in self.find_local_x_extrema() {
            let point = self.sample(t);
            if point.x > max_x {
                max_t = t;
                max_x = point.x;
            }
        }
        return max_t;
    }

    /// Find the x-least position in the curve.
    pub fn find_x_minimum(&self) -> f32 {
        let mut min_t = 0.0;
        let mut min_x = self.from.x;
        if self.to.x < min_x {
            min_t = 1.0;
            min_x = self.to.x;
        }
        for t in self.find_local_x_extrema() {
            let point = self.sample(t);
            if point.x < min_x {
                min_t = t;
                min_x = point.x;
            }
        }
        return min_t;
    }

    /// Returns a rectangle the curve is contained in
    pub fn bounding_rect(&self) -> Rect {
        let min_x = self.from.x.min(self.ctrl1.x).min(self.ctrl2.x).min(self.to.x);
        let max_x = self.from.x.max(self.ctrl1.x).max(self.ctrl2.x).max(self.to.x);
        let min_y = self.from.y.min(self.ctrl1.y).min(self.ctrl2.y).min(self.to.y);
        let max_y = self.from.y.max(self.ctrl1.y).max(self.ctrl2.y).max(self.to.y);

        return rect(min_x, min_y, max_x - min_x, max_y - min_y);
    }

    /// Returns the smallest rectangle the curve is contained in
    pub fn minimum_bounding_rect(&self) -> Rect {
        let min_x = self.sample_x(self.find_x_minimum());
        let max_x = self.sample_x(self.find_x_maximum());
        let min_y = self.sample_y(self.find_y_minimum());
        let max_y = self.sample_y(self.find_y_maximum());

        return rect(min_x, min_y, max_x - min_x, max_y - min_y);
    }

    /// Cast this curve into a x-montone curve without checking that the monotonicity
    /// assumption is correct.
    pub fn assume_x_montone(&self) -> XMonotoneCubicBezierSegment {
        XMonotoneCubicBezierSegment { curve: *self }
    }

    /// Cast this curve into a y-montone curve without checking that the monotonicity
    /// assumption is correct.
    pub fn assume_y_montone(&self) -> YMonotoneCubicBezierSegment {
        YMonotoneCubicBezierSegment { curve: *self }
    }

    pub fn line_intersections(&self, line: &Line) -> ArrayVec<[Point; 3]> {
        if line.vector.square_length() < 1e-6 {
            return ArrayVec::new();
        }

        let from = self.from.to_vector();
        let ctrl1 = self.ctrl1.to_vector();
        let ctrl2 = self.ctrl2.to_vector();
        let to = self.to.to_vector();

        let p1 = to - from + (ctrl1 - ctrl2) * 3.0;
        let p2 = from * 3.0 + (ctrl2 - ctrl1 * 2.0) * 3.0;
        let p3 = (ctrl1 - from) * 3.0;
        let p4 = from;

        let c = line.point.y * line.vector.x - line.point.x * line.vector.y;

        let roots = cubic_polynomial_roots(
            line.vector.y * p1.x - line.vector.x * p1.y,
            line.vector.y * p2.x - line.vector.x * p2.y,
            line.vector.y * p3.x - line.vector.x * p3.y,
            line.vector.y * p4.x - line.vector.x * p4.y + c,
        );

        let mut result = ArrayVec::new();

        for root in roots {
            let t = root;
            if t <= 0.0 || t >= 1.0 {
                continue;
            }

            let position = (p1 * t * t * t + p2 * t * t + p3 * t + p4).to_point();

            result.push(position);
        }

        return result;
    }

    pub fn line_segment_intersections(&self, segment: &LineSegment) -> ArrayVec<[Point; 3]> {
        let intersections = self.line_intersections(&segment.to_line());
        let aabb = segment.bounding_rect();
        let mut result = ArrayVec::new();
        for point in intersections {
            if aabb.contains(&point) {
                result.push(point);
            }
        }
        return result;
    }
}

/// A monotonically increasing in x cubic bézier curve segment
#[derive(Copy, Clone, Debug)]
pub struct XMonotoneCubicBezierSegment {
    curve: CubicBezierSegment
}

impl XMonotoneCubicBezierSegment {
    #[inline]
    pub fn curve(&self) -> &CubicBezierSegment {
        &self.curve
    }

    /// Approximates y for a given value of x and a tolerance threshold.
    #[inline]
    pub fn solve_y_for_x(&self, x: f32, tolerance: f32) -> f32 {
        self.curve.sample_y(self.solve_t_for_x(x, tolerance))
    }

    /// Approximates t for a given value of x and a tolerance threshold.
    #[inline]
    pub fn solve_t_for_x(&self, x: f32, tolerance: f32) -> f32 {
        solve_t_for_x(self, x, tolerance)
    }
}

impl XMonotoneParametricCurve for XMonotoneCubicBezierSegment {
    fn x(&self, t: f32) -> f32 { self.curve.sample_x(t) }
    fn dx(&self, t: f32) -> f32 { self.curve.sample_x_derivative(t) }
}

/// A monotonically increasing in y cubic bézier curve segment
#[derive(Copy, Clone, Debug)]
pub struct YMonotoneCubicBezierSegment {
    curve: CubicBezierSegment
}

impl YMonotoneCubicBezierSegment {
    #[inline]
    pub fn curve(&self) -> &CubicBezierSegment {
        &self.curve
    }

    /// Approximates x for a given value of y and a tolerance threshold.
    #[inline]
    pub fn solve_x_for_y(&self, y: f32, tolerance: f32) -> f32 {
        self.curve.sample_y(self.solve_t_for_y(y, tolerance))
    }

    /// Approximates t for a given value of y and a tolerance threshold.
    #[inline]
    pub fn solve_t_for_y(&self, y: f32, tolerance: f32) -> f32 {
        let transposed = XMonotoneCubicBezierSegment {
            curve: CubicBezierSegment {
                from: yx(self.curve.from),
                ctrl1: yx(self.curve.ctrl1),
                ctrl2: yx(self.curve.ctrl2),
                to: yx(self.curve.to),
            }
        };

        transposed.solve_t_for_x(y, tolerance)
    }
}

// TODO: add this to euclid.
fn yx(point: Point) -> Point { Point::new(point.y, point.x) }

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
fn minimum_bounding_rect_for_cubic_bezier_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.5, 2.0),
        ctrl2: Point::new(1.5, -2.0),
        to: Point::new(2.0, 0.0),
    };
    
    let expected_bigger_bounding_rect: Rect = rect(0.0, -0.6, 2.0, 1.2);
    let expected_smaller_bounding_rect: Rect = rect(0.1, -0.5, 1.9, 1.0);

    let actual_minimum_bounding_rect: Rect = a.minimum_bounding_rect();

    assert!(expected_bigger_bounding_rect.contains_rect(&actual_minimum_bounding_rect));
    assert!(actual_minimum_bounding_rect.contains_rect(&expected_smaller_bounding_rect));
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

    assert!(expected_y_maximum == actual_y_maximum)
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

    assert!(expected_y_minimum == actual_y_minimum)
}

#[test]
fn find_y_extrema_for_simple_cubic_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(1.0, 2.0),
        ctrl2: Point::new(2.0, 2.0),
        to: Point::new(3.0, 0.0),
    };

    let mut expected_y_extremums = UpToTwo::new();
    expected_y_extremums.push(0.5);

    let actual_y_extremums = a.find_local_y_extrema();

    for extremum in expected_y_extremums {
        assert!(actual_y_extremums.contains(&extremum))
    }
}

#[test]
fn find_x_extrema_for_simple_cubic_segment() {
    let a = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(1.0, 2.0),
        ctrl2: Point::new(1.0, 2.0),
        to: Point::new(0.0, 0.0),
    };

    let mut expected_x_extremums = UpToTwo::new();
    expected_x_extremums.push(0.5);

    let actual_x_extremums = a.find_local_x_extrema();

    for extremum in expected_x_extremums {
        assert!(actual_x_extremums.contains(&extremum))
    }
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

    assert!(expected_x_maximum == actual_x_maximum)
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

    assert!(expected_x_minimum == actual_x_minimum)
}

#[test]
fn derivatives() {
    let c1 = CubicBezierSegment {
        from: Point::new(1.0, 1.0,),
        ctrl1: Point::new(1.0, 2.0,),
        ctrl2: Point::new(2.0, 1.0,),
        to: Point::new(2.0, 2.0,),
    };

    assert_eq!(c1.sample_x_derivative(0.0), 0.0);
    assert_eq!(c1.sample_x_derivative(1.0), 0.0);
    assert_eq!(c1.sample_y_derivative(0.5), 0.0);
}

#[test]
fn monotone_solve_t_for_x() {
    let c1 = CubicBezierSegment {
        from: Point::new(1.0, 1.0),
        ctrl1: Point::new(1.0, 2.0),
        ctrl2: Point::new(2.0, 1.0),
        to: Point::new(2.0, 2.0),
    };

    let tolerance = 0.0001;

    for i in 0..10u32 {
        let t = i as f32 / 10.0;
        let p = c1.sample(t);
        let t2 = c1.assume_x_montone().solve_t_for_x(p.x, tolerance);
        // t should be pretty close to t2 but the only guarantee we have and can test
        // against is that x(t) - x(t2) is within the specified tolerance threshold.
        let x_diff = c1.sample_x(t) - c1.sample_x(t2);
        assert!(x_diff.abs() <= tolerance);
    }
}
