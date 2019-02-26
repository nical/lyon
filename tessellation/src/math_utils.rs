//! Various math tools that are mostly usefull for the tessellators.

use crate::fixed;
use crate::geom::math::*;
use crate::path_fill::Edge;
use crate::geom::euclid;
use std::f64;

pub type FixedPoint32 = fixed::Fp32<fixed::_16>;
pub type FixedPoint64 = fixed::Fp64<fixed::_16>;
pub type TessVector = euclid::Vector2D<FixedPoint32>;
pub type TessPoint = euclid::Point2D<FixedPoint32>;
pub type TessPoint64 = euclid::Point2D<FixedPoint64>;
#[inline]
pub fn fixed(val: f32) -> FixedPoint32 { FixedPoint32::from_f32(val) }

#[inline]
fn x_aabb_test(a1: FixedPoint32, b1: FixedPoint32, a2: FixedPoint32, b2: FixedPoint32) -> bool {
    let (min1, max1) = a1.min_max(b1);
    let (min2, max2) = a2.min_max(b2);
    min1 <= max2 && max1 >= min2
}

// TODO[optim]: This function shows up pretty high in the profiles.
pub(crate) fn segment_intersection(
    e1: &Edge, // The new edge.
    e2: &Edge, // An already inserted edge.
) -> Option<TessPoint> {

    // This early-out test gives a noticeable performance improvement.
    if !x_aabb_test(e1.upper.x, e1.lower.x, e2.upper.x, e2.lower.x) {
        return None;
    }

    if e1.upper == e2.lower || e1.upper == e2.upper || e1.lower == e2.upper || e1.lower == e2.lower {
        return None;
    }

    fn tess_point(x: f64, y: f64) -> TessPoint {
        TessPoint::new(FixedPoint32::from_f64(x), FixedPoint32::from_f64(y))
    }

    let a1 = F64Point::new(e1.upper.x.to_f64(), e1.upper.y.to_f64());
    let b1 = F64Point::new(e1.lower.x.to_f64(), e1.lower.y.to_f64());
    let a2 = F64Point::new(e2.upper.x.to_f64(), e2.upper.y.to_f64());
    let b2 = F64Point::new(e2.lower.x.to_f64(), e2.lower.y.to_f64());

    let v1 = b1 - a1;
    let v2 = b2 - a2;

    debug_assert!(v2.x != 0.0 || v2.y != 0.0, "zero-length edge");

    let v1_cross_v2 = v1.cross(v2);
    let a2_a1_cross_v1 = (a2 - a1).cross(v1);

    if v1_cross_v2 == 0.0 {
        return None;
    }

    let sign_v1_cross_v2 = v1_cross_v2.signum();
    let abs_v1_cross_v2 = f64::abs(v1_cross_v2);

    // t and u should be divided by v1_cross_v2, but we postpone that to not lose precision.
    // We have to respect the sign of v1_cross_v2 (and therefore t and u) so we apply it now and
    // will use the absolute value of v1_cross_v2 afterwards.
    let t = (a2 - a1).cross(v2) * sign_v1_cross_v2;
    let u = a2_a1_cross_v1 * sign_v1_cross_v2;
    if t >= 0.0 && t <= abs_v1_cross_v2 && u > 0.0 && u <= abs_v1_cross_v2 {

        // Snap intersections to the edge if it is very close.
        // This helps with preventing small floating points errors from
        // accumulating when many edges intersect at the same position.
        let threshold = 0.000_001;
        if 1.0 - t / abs_v1_cross_v2 < threshold {
            return Some(e1.lower);
        }
        if 1.0 - u / abs_v1_cross_v2 < threshold {
            return Some(e2.lower);
        }

        let res = a1 + (v1 * t) / abs_v1_cross_v2;

        let res = tess_point(res.x, res.y);
        // It would be great if the assertion below held, but it happens
        // to fail due to precision issues.
        // debug_assert!(res.y <= e1.lower.y && res.y <= e2.lower.y);
        if res != e1.upper && res != e2.upper
            && res.y <= e1.lower.y && res.y <= e2.lower.y {
            return Some(res);
        }
    }

    None
}

/// Compute a normal vector at a point P such that ```x ---e1----> P ---e2---> x```
///
/// The resulting vector is not normalized. The length is such that extruding the shape
/// would yield parallel segments exactly 1 unit away from their original. (useful
/// for generating strokes and vertex-aa).
/// The normal points towards the left side of e1.
///
/// v1 and v2 are expected to be normalized.
pub fn compute_normal(v1: Vector, v2: Vector) -> Vector {
    //debug_assert!((v1.length() - 1.0).abs() < 0.001, "v1 should be normalized ({})", v1.length());
    //debug_assert!((v2.length() - 1.0).abs() < 0.001, "v2 should be normalized ({})", v2.length());

    let epsilon = 1e-4;

    let n1 = vector(-v1.y, v1.x);

    let v12 = v1 + v2;

    if v12.square_length() < epsilon {
        return n1;
    }

    let tangent = v12.normalize();
    let n = vector(-tangent.y, tangent.x);

    let inv_len = n.dot(n1);

    if inv_len.abs() < epsilon {
        return n1;
    }

    n / inv_len
}

#[test]
fn test_compute_normal() {
    fn assert_almost_eq(a: Vector, b: Vector) {
        if (a - b).square_length() > 0.00001 {
            panic!("assert almost equal: {:?} != {:?}", a, b);
        }
    }

    assert_almost_eq(compute_normal(vector(1.0, 0.0), vector(0.0, 1.0)), vector(-1.0, 1.0));
    assert_almost_eq(compute_normal(vector(1.0, 0.0), vector(1.0, 0.0)), vector(0.0, 1.0));
}
