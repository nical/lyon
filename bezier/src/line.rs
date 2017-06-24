use {Point, Vec2, vec2, Rect, Size, Transform2D};
use euclid::point2 as point;

// TODO: Perhaps it would be better to have LineSegment<T> where T can be f32, f64
// or some fixed precision number (See comment in the intersection function).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LineSegment {
    pub from: Point,
    pub to: Point,
}

impl LineSegment {
    /// Sample the segment at t (expecting t between 0 and 1).
    #[inline]
    pub fn sample(&self, t: f32) -> Point {
        self.from.lerp(self.to, t)
    }

    /// Sample the x coordinate of the segment at t (expecting t between 0 and 1).
    #[inline]
    pub fn sample_x(&self, t: f32) -> f32 {
        self.from.x * (1.0 - t) + self.to.x * t
    }

    /// Sample the y coordinate of the segment at t (expecting t between 0 and 1).
    #[inline]
    pub fn sample_y(&self, t: f32) -> f32 {
        self.from.y * (1.0 - t) + self.to.y * t
    }

    /// Returns an inverted version of this segment where the beginning and the end
    /// points are swapped.
    #[inline]
    #[must_use]
    pub fn flip(&self) -> Self {
        LineSegment { from: self.to, to: self.from }
    }

    /// Split this curve into two sub-segments.
    #[inline]
    pub fn split(&self, t: f32) -> (Self, Self) {
        let split_point = self.sample(t);
        return (
            LineSegment { from: self.from, to: split_point },
            LineSegment { from: split_point, to: self.to },
        );
    }

    /// Return the segment before the split point.
    #[inline]
    pub fn before_split(&self, t: f32) -> Self {
        LineSegment { from: self.from, to: self.sample(t) }
    }

    /// Return the segment after the split point.
    #[inline]
    pub fn after_split(&self, t: f32) -> Self {
        LineSegment { from: self.sample(t), to: self.to }
    }

    /// Return the minimum bounding rectangle
    #[inline]
    pub fn bounding_rect(&self) -> Rect {
        let min_x = self.from.x.min(self.to.x);
        let min_y = self.from.y.min(self.to.y);

        let width  = (self.from.x.max(self.to.x) - min_x).abs();
        let height = (self.from.y.max(self.to.y) - min_y).abs();
        Rect::new(Point::new(min_x, min_y), Size::new(width, height))
    }

    /// Returns the vector between this segment's `from` and `to` points.
    #[inline]
    pub fn to_vector(&self) -> Vec2 {
        self.to - self.from
    }

    /// Returns the line containing this segment.
    #[inline]
    pub fn to_line(&self) -> Line {
        Line {
            point: self.from,
            vector: self.to - self.from,
        }
    }

    /// Computes the length of this segment.
    #[inline]
    pub fn length(&self) -> f32 {
        self.to_vector().length()
    }

    #[inline]
    #[must_use]
    pub fn translate(&mut self, by: Vec2) -> Self {
        LineSegment {
            from: self.from + by,
            to: self.to + by,
        }
    }

    /// Applies the transform to this segment and returns the results.
    #[inline]
    #[must_use]
    pub fn transform(&self, transform: &Transform2D) -> Self {
        LineSegment {
            from: transform.transform_point(&self.from),
            to: transform.transform_point(&self.to),
        }
    }

    /// Computes the intersection (if any) between this segment and another one.
    pub fn intersection(&self, other: &Self) -> Option<Point> {
        // TODO: The precision with of the function with f32 is pretty bad so
        // this uses f64. It'd be better if we made the scalar type a typed
        // parameter and chose depending on that.

        use euclid::Vector2D;
        fn vec2_f64(v: Vec2) -> Vector2D<f64> { vec2( v.x as f64, v.y as f64) }

        let v1 = vec2_f64(self.to_vector());
        let v2 = vec2_f64(-other.to_vector());

        let v1_cross_v2 = v1.cross(v2);

        if v1_cross_v2 == 0.0 {
            // The segments are parallel
            return None;
        }

        let v3 = vec2_f64(other.to - self.from);

        let sign_v1_cross_v2 = v1_cross_v2.signum();
        let abs_v1_cross_v2 = v1_cross_v2 * sign_v1_cross_v2;

        // t and u should be divided by v1_cross_v2, but we postpone that to not lose precision.
        // We have to respect the sign of v1_cross_v2 (and therefore t and u) so we apply it now and
        // will use the absolute value of v1_cross_v2 afterwards.
        let t = v3.cross(v2) * sign_v1_cross_v2;
        let u = v3.cross(v1) * sign_v1_cross_v2;

        if t > 0.0 && t < abs_v1_cross_v2 && u > 0.0 && u < abs_v1_cross_v2 {
            return Some(self.from + (v1 * t / abs_v1_cross_v2).to_f32());
        }

        return None;
    }

    pub fn intersects(&self, other: &Self) -> bool {
        // we don't need as much precision if we don't compute the position of
        // the intersection, so this version uses f32 arithmetic.
        let v1 = self.to_vector();
        let v2 = -other.to_vector();

        let v1_cross_v2 = v1.cross(v2);

        if v1_cross_v2 == 0.0 {
            return false;
        }

        let v3 = other.to - self.from;

        let sign_v1_cross_v2 = v1_cross_v2.signum();
        let abs_v1_cross_v2 = v1_cross_v2 * sign_v1_cross_v2;

        let t = v3.cross(v2) * sign_v1_cross_v2;
        let u = v3.cross(v1) * sign_v1_cross_v2;

        if t > 0.0 && t < abs_v1_cross_v2 && u > 0.0 && u < abs_v1_cross_v2 {
            return true;
        }

        return false;
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Line {
    pub point: Point,
    pub vector: Vec2,
}

impl Line {
    pub fn intersection(&self, other: &Self) -> Option<Point> {
        let epsilon = 0.000001;
        let det = self.vector.cross(other.vector);
        if det.abs() <= epsilon {
            // The lines are very close to parallel
            return None;
        }
        let inv_det = 1.0 / det;
        let self_p2 = self.point + self.vector;
        let other_p2 = other.point + other.vector;
        let a = self.point.to_vector().cross(self_p2.to_vector());
        let b = other.point.to_vector().cross(other_p2.to_vector());
        return Some(
            point(
                (b * self.vector.x - a * other.vector.x) * inv_det,
                (b * self.vector.y - a * other.vector.y) * inv_det,
            )
        );
    }
}

#[cfg(test)]
fn fuzzy_eq_f32(a: f32, b: f32, epsilon: f32) -> bool {
    return (a - b).abs() <= epsilon;
}

#[cfg(test)]
fn fuzzy_eq_vec2(a: Vec2, b: Vec2, epsilon: f32) -> bool {
    fuzzy_eq_f32(a.x, b.x, epsilon) && fuzzy_eq_f32(a.y, b.y, epsilon)
}

#[cfg(test)]
fn fuzzy_eq_point(a: Point, b: Point, epsilon: f32) -> bool {
    fuzzy_eq_vec2(a.to_vector(), b.to_vector(), epsilon)
}

#[test]
fn intersection_rotated() {
    use std::f32::consts::PI;
    let epsilon = 0.0001;
    let count: u32 = 100;

    for i in 0..count {
        for j in 0..count {
            if i % (count / 2) == j % (count / 2) {
                // avoid the colinear case.
                continue;
            }

            let angle1 = i as f32 / (count as f32) * 2.0 * PI;
            let angle2 = j as f32 / (count as f32) * 2.0 * PI;

            let l1 = LineSegment {
                from: point(10.0 * angle1.cos(), 10.0 * angle1.sin()),
                to: point(-10.0 * angle1.cos(), -10.0 * angle1.sin()),
            };

            let l2 = LineSegment {
                from: point(10.0 * angle2.cos(), 10.0 * angle2.sin()),
                to: point(-10.0 * angle2.cos(), -10.0 * angle2.sin()),
            };

            assert!(l1.intersects(&l2));

            assert!(
                fuzzy_eq_point(
                    l1.intersection(&l2).unwrap(),
                    point(0.0, 0.0),
                    epsilon
                )
            );
        }
    }
}

#[test]
fn intersection_touching() {
    let l1 = LineSegment {
        from: point(0.0, 0.0),
        to: point(10.0, 10.0),
    };

    let l2 = LineSegment {
        from: point(10.0, 10.0),
        to: point(10.0, 0.0),
    };

    assert!(!l1.intersects(&l2));
    assert!(l1.intersection(&l2).is_none());
}

#[test]
fn intersection_overlap() {
    // It's hard to define the intersection points of two segments that overlap,
    // (would be a region rather than a point) and more importanly, in practice
    // the algorithms in lyon don't need to consider this special case as an intersection,
    // so we choose to treat overlapping segments as not intersecting.

    let l1 = LineSegment {
        from: point(0.0, 0.0),
        to: point(10.0, 0.0),
    };

    let l2 = LineSegment {
        from: point(5.0, 00.0),
        to: point(15.0, 0.0),
    };

    assert!(!l1.intersects(&l2));
    assert!(l1.intersection(&l2).is_none());
}

#[cfg(test)]
use euclid::rect;

#[test]
fn bounding_rect() {
    let l1 = LineSegment {
        from: point(1., 5.),
        to: point(5., 7.),
    };
    let r1 = rect(1., 5., 4., 2.);

    let l2 = LineSegment {
        from: point(5., 5.),
        to: point(1., 1.),
    };
    let r2 = rect(1., 1., 4., 4.);

    let l3 = LineSegment {
        from: point(3., 3.),
        to: point(1., 5.),
    };
    let r3 = rect(1., 3., 2., 2.);

    let cases = vec![(l1, r1), (l2, r2), (l3, r3)];
    for &(ls, r) in &cases {
        assert_eq!(ls.bounding_rect(), r);
    }
}
