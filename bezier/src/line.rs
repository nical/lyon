use {Point, Vec2, vec2, Rect};

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

    /// [Not implemented] Sample the x coordinate of the segment at t (expecting t between 0 and 1).
    #[inline]
    pub fn sample_x(&self, _t: f32) -> f32 {
        unimplemented!()
    }

    /// [Not implemented] Sample the y coordinate of the segment at t (expecting t between 0 and 1).
    #[inline]
    pub fn sample_y(&self, _t: f32) -> f32 {
        unimplemented!()
    }

    /// Returns an inverted version of this segment where the beginning and the end
    /// points are swapped.
    #[inline]
    #[must_use]
    pub fn flip(&self) -> Self {
        LineSegment { from: self.to, to: self.from }
    }

    /// [Not implemented] Split this curve into two sub-segments.
    #[inline]
    pub fn split(&self, _t: f32) -> (Self, Self) {
        unimplemented!()
    }

    /// [Not implemented] Return the segment before the split point.
    #[inline]
    pub fn before_split(&self, _t: f32) -> Self {
        unimplemented!()
    }

    /// [Not implemented] Return the segment after the split point.
    #[inline]
    pub fn after_split(&self, _t: f32) -> Self {
        unimplemented!()
    }

    /// [Not implemented]
    #[inline]
    pub fn bounding_rect(&self) -> Rect {
        unimplemented!()
    }

    /// Returns the vector between this segment's `from` and `to` points.
    #[inline]
    pub fn to_vector(&self) -> Vec2 {
        self.to - self.from
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

#[cfg(test)]
use euclid::point2 as point;

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
