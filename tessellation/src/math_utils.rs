//! Various math tools that are mostly usefull for the tessellators.

use math::*;
use bezier::utils::directed_angle;
use path_fill::Edge;

/// Fixed-point version of the line vs horizontal line intersection test.
pub fn line_horizontal_intersection_fixed(
    edge: &Edge,
    y: FixedPoint32,
) -> Option<FixedPoint32> {
    let v = edge.lower - edge.upper;

    if v.y.is_zero() {
        // the line is horizontal
        return None;
    }

    let tmp: FixedPoint64 = (y - edge.upper.y).to_fp64();
    return Some(edge.upper.x + tmp.mul_div(v.x.to_fp64(), v.y.to_fp64()).to_fp32());
}

#[inline]
fn x_aabb_test(a1: FixedPoint32, b1: FixedPoint32, a2: FixedPoint32, b2: FixedPoint32) -> bool {
    let (min1, max1) = a1.min_max(b1);
    let (min2, max2) = a2.min_max(b2);
    min1 <= max2 && max1 >= min2
}

// TODO[optim]: This function shows up pretty high in the profiles.
pub fn segment_intersection(
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

    let sign_v1_cross_v2 = if v1_cross_v2 > 0.0 { 1.0 } else { -1.0 };
    let abs_v1_cross_v2 = v1_cross_v2 * sign_v1_cross_v2;

    // t and u should be divided by v1_cross_v2, but we postpone that to not lose precision.
    // We have to respect the sign of v1_cross_v2 (and therefore t and u) so we apply it now and
    // will use the absolute value of v1_cross_v2 afterwards.
    let t = (a2 - a1).cross(v2) * sign_v1_cross_v2;
    let u = a2_a1_cross_v1 * sign_v1_cross_v2;
    if t > 0.0 && t <= abs_v1_cross_v2 && u > 0.0 && u <= abs_v1_cross_v2 {

        let res = a1 + (v1 * t) / abs_v1_cross_v2;
        debug_assert!(res.y <= b1.y && res.y <= b2.y);

        if res != a1 && res != a2 {
            return Some(tess_point(res.x, res.y));
        }
    }

    return None;
}

/// Compute a normal vector at a point P such that ```x ---e1----> P ---e2---> x```
///
/// The resulting vector is not normalized. The length is such that extruding the shape
/// would yield parallel segments exactly 1 unit away from their original. (useful
/// for generating strokes and vertex-aa).
/// The normal points towards the left side of e1.
pub fn compute_normal(e1: Vec2, e2: Vec2) -> Vec2 {
    let e1_norm = e1.normalize();
    let n = e1_norm - e2.normalize();
    if n.length() == 0.0 {
        return vec2(e1_norm.y, -e1_norm.x);
    }
    let mut n_norm = n.normalize();

    if e1_norm.cross(n_norm) > 0.0 {
        n_norm = -n_norm;
    }

    let angle = directed_angle(e1, e2) * 0.5;
    let sin = angle.sin();

    if sin == 0.0 {
        return e1_norm;
    }

    return n_norm / sin;
}

#[test]
fn test_compute_normal() {
    fn assert_almost_eq(a: Vec2, b: Vec2) {
        if (a - b).square_length() > 0.00001 {
            panic!("assert almost equal: {:?} != {:?}", a, b);
        }
    }

    for i in 1..10 {
        let f = i as f32;
        assert_almost_eq(compute_normal(vec2(f, 0.0), vec2(0.0, f * f)), vec2(1.0, -1.0));
    }
    for i in 1..10 {
        let f = i as f32;
        assert_almost_eq(compute_normal(vec2(f, 0.0), vec2(f * f, 0.0)), vec2(0.0, -1.0));
    }
}
