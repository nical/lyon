use crate::scalar::Scalar;
use crate::generic_math::{Point, Rect, Size, Transform2D};
use crate::LineSegment;

/// A 2D triangle defined by three points `a`, `b` and `c`.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Triangle<S> {
    pub a: Point<S>,
    pub b: Point<S>,
    pub c: Point<S>,
}

impl<S: Scalar> Triangle<S> {
    #[inline]
    fn get_barycentric_coords_for_point(&self, point: Point<S>) -> (S, S, S) {
        let v0 = self.b - self.a;
        let v1 = self.c - self.a;
        let v2 = point - self.a;
        let inv = S::ONE / v0.cross(v1);
        let a = v0.cross(v2) * inv;
        let b = v2.cross(v1) * inv;
        let c = S::ONE - a - b;
        (a, b, c)
    }

    pub fn contains_point(&self, point: Point<S>) -> bool {
        let coords = self.get_barycentric_coords_for_point(point);
        coords.0 > S::ZERO && coords.1 > S::ZERO && coords.2 > S::ZERO
    }

    /// Return the minimum bounding rectangle.
    #[inline]
    pub fn bounding_rect(&self) -> Rect<S> {
        let max_x = self.a.x.max(self.b.x).max(self.c.x);
        let min_x = self.a.x.min(self.b.x).min(self.c.x);
        let max_y = self.a.y.max(self.b.y).max(self.c.y);
        let min_y = self.a.y.min(self.b.y).min(self.c.y);

        let width = max_x - min_x;
        let height = max_y - min_y;
        Rect::new(Point::new(min_x, min_y), Size::new(width, height))
    }

    #[inline]
    pub fn ab(&self) -> LineSegment<S> {
        LineSegment { from: self.a, to: self.b }
    }

    #[inline]
    pub fn ba(&self) -> LineSegment<S> {
        LineSegment { from: self.b, to: self.a }
    }

    #[inline]
    pub fn bc(&self) -> LineSegment<S> {
        LineSegment { from: self.b, to: self.c }
    }

    #[inline]
    pub fn cb(&self) -> LineSegment<S> {
        LineSegment { from: self.c, to: self.b }
    }

    #[inline]
    pub fn ca(&self) -> LineSegment<S> {
        LineSegment { from: self.c, to: self.a }
    }

    #[inline]
    pub fn ac(&self) -> LineSegment<S> {
        LineSegment { from: self.a, to: self.c }
    }

    /// [Not implemented] Applies the transform to this triangle and returns the results.
    #[inline]
    pub fn transform(&self, transform: &Transform2D<S>) -> Self {
        Triangle {
            a: transform.transform_point(&self.a),
            b: transform.transform_point(&self.b),
            c: transform.transform_point(&self.c)
        }
    }

    /// Test for triangle-triangle intersection.
    pub fn intersects(&self, other: &Self) -> bool {
        // TODO: This should be optimized.
        // A bounding rect check should speed this up dramatically.
        // Inlining and reusing intermediate computation of the intersections
        // functions below and using SIMD would help too.
        return self.ab().intersects(&other.ab())
            || self.ab().intersects(&other.bc())
            || self.ab().intersects(&other.ac())
            || self.bc().intersects(&other.ab())
            || self.bc().intersects(&other.bc())
            || self.bc().intersects(&other.ac())
            || self.ac().intersects(&other.ab())
            || self.ac().intersects(&other.bc())
            || self.ac().intersects(&other.ac())
            || self.contains_point(other.a)
            || other.contains_point(self.a)
            || *self == *other;
    }

    /// Test for triangle-segment intersection.
    #[inline]
    pub fn intersects_line_segment(&self, segment: &LineSegment<S>) -> bool {
        return self.ab().intersects(segment)
            || self.bc().intersects(segment)
            || self.ac().intersects(segment)
            || self.contains_point(segment.from);
    }
}

#[cfg(test)]
use crate::math::point;

#[test]
fn test_triangle_contains() {

    assert!(
        Triangle {
            a: point(0.0, 0.0),
            b: point(1.0, 0.0),
            c: point(0.0, 1.0),
        }.contains_point(point(0.2, 0.2))
    );
    assert!(
        !Triangle {
            a: point(0.0, 0.0),
            b: point(1.0, 0.0),
            c: point(0.0, 1.0),
        }.contains_point(point(1.2, 0.2))
    );

    // Triangle vertex winding should not matter
    assert!(
        Triangle {
            a: point(1.0, 0.0),
            b: point(0.0, 0.0),
            c: point(0.0, 1.0),
        }.contains_point(point(0.2, 0.2))
    );

    // Point exactly on the edge counts as outside the triangle.
    assert!(
        !Triangle {
            a: point(0.0, 0.0),
            b: point(1.0, 0.0),
            c: point(0.0, 1.0),
        }.contains_point(point(0.0, 0.0))
    );
}

#[test]
fn test_segments() {
    let t = Triangle {
        a: point(1.0, 2.0),
        b: point(3.0, 4.0),
        c: point(5.0, 6.0),
    };

    assert!(t.ab() == t.ba().flip());
    assert!(t.ac() == t.ca().flip());
    assert!(t.bc() == t.cb().flip());
}

#[test]
fn test_triangle_intersections() {
    let t1 = Triangle {
        a: point(1.0, 1.0),
        b: point(6.0, 1.0),
        c: point(3.0, 6.0),
    };

    let t2 = Triangle {
        a: point(2.0, 2.0),
        b: point(0.0, 3.0),
        c: point(1.0, 6.0),
    };

    assert!(t1.intersects(&t2));
    assert!(t2.intersects(&t1));

    // t3 and t1 have an overlapping edge, they are "touching" but not intersecting.
    let t3 = Triangle {
        a: point(6.0, 5.0),
        b: point(6.0, 1.0),
        c: point(3.0, 6.0),
    };

    assert!(!t1.intersects(&t3));
    assert!(!t3.intersects(&t1));

    // t4 is entirely inside t1.
    let t4 = Triangle {
        a: point(2.0, 2.0),
        b: point(5.0, 2.0),
        c: point(3.0, 4.0),
    };

    assert!(t1.intersects(&t4));
    assert!(t4.intersects(&t1));

    // Triangles intersect themselves.
    assert!(t1.intersects(&t1));
    assert!(t2.intersects(&t2));
    assert!(t3.intersects(&t3));
    assert!(t4.intersects(&t4));
}

#[test]
fn test_segment_intersection() {
    let tri = Triangle {
        a: point(1.0, 1.0),
        b: point(6.0, 1.0),
        c: point(3.0, 6.0),
    };

    let l1 = LineSegment {
        from: point(2.0, 0.0),
        to: point(3.0, 4.0),
    };

    assert!(tri.intersects_line_segment(&l1));

    let l2 = LineSegment {
        from: point(1.0, 3.0),
        to: point(0.0, 4.0),
    };

    assert!(!tri.intersects_line_segment(&l2));

    // The segement is entirely inside the triangle.
    let inside = LineSegment {
        from: point(2.0, 2.0),
        to: point(5.0, 2.0),
    };

    assert!(tri.intersects_line_segment(&inside));

    // A triangle does not intersect its own segments.
    assert!(!tri.intersects_line_segment(&tri.ab()));
    assert!(!tri.intersects_line_segment(&tri.bc()));
    assert!(!tri.intersects_line_segment(&tri.ac()));
}

#[cfg(test)]
use euclid::rect;

#[test]
fn test_bounding_rect() {
    let t1 = Triangle {
        a: point(10., 20.),
        b: point(35., 40.),
        c: point(50., 10.),
    };
    let r1 = rect(10., 10., 40., 30.);

    let t2 = Triangle {
        a: point(5., 30.),
        b: point(25., 10.),
        c: point(35., 40.),
    };
    let r2 = rect(5., 10., 30., 30.);

    let t3 = Triangle {
        a: point(1., 1.),
        b: point(2., 5.),
        c: point(0., 4.),
    };
    let r3 = rect(0., 1., 2., 4.);

    let cases = vec![(t1, r1), (t2, r2), (t3, r3)];
    for &(tri, r) in &cases {
        assert_eq!(tri.bounding_rect(), r);
    }
}
