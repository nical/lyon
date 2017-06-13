use {Point, Rect, LineSegment};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Triangle {
    pub a: Point,
    pub b: Point,
    pub c: Point,
}

impl Triangle {
    pub fn contains_point(&self, point: Point) -> bool {
        // see http://blackpawn.com/texts/pointinpoly/
        let v0 = self.c - self.a;
        let v1 = self.b - self.a;
        let v2 = point - self.a;

        let dot00 = v0.dot(v0);
        let dot01 = v0.dot(v1);
        let dot02 = v0.dot(v2);
        let dot11 = v1.dot(v1);
        let dot12 = v1.dot(v2);
        let inv = 1.0 / (dot00 * dot11 - dot01 * dot01);
        let u = (dot11 * dot02 - dot01 * dot12) * inv;
        let v = (dot11 * dot12 - dot01 * dot02) * inv;

        return u >= 0.0 && v >= 0.0 && u + v < 1.0;
    }

    /// [Not implemented]
    #[inline]
    pub fn bounding_rect(&self) -> Rect {
        unimplemented!()
    }

    #[inline]
    pub fn ab(&self) -> LineSegment {
        LineSegment { from: self.a, to: self.b }
    }

    #[inline]
    pub fn ba(&self) -> LineSegment {
        LineSegment { from: self.b, to: self.a }
    }

    #[inline]
    pub fn bc(&self) -> LineSegment {
        LineSegment { from: self.b, to: self.c }
    }

    #[inline]
    pub fn cb(&self) -> LineSegment {
        LineSegment { from: self.c, to: self.b }
    }

    #[inline]
    pub fn ca(&self) -> LineSegment {
        LineSegment { from: self.c, to: self.a }
    }

    #[inline]
    pub fn ac(&self) -> LineSegment {
        LineSegment { from: self.a, to: self.c }
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
}

#[cfg(test)]
use euclid::point2 as point;

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
    // Point exactly on the edge counts as in the triangle.
    assert!(
        Triangle {
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
