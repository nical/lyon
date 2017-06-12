use {Point, Rect};

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

    /// [Not implemented]
    #[inline]
    pub fn intersects(&self, _other: &Self) -> bool {
        unimplemented!()
    }
}

#[test]
fn test_triangle_contains() {
    use euclid::point2 as point;

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

