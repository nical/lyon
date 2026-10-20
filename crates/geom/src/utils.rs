use crate::scalar::{Float, Scalar};
use crate::{vector, Point, Vector};
use arrayvec::ArrayVec;

#[inline]
pub fn min_max<S: Float>(a: S, b: S) -> (S, S) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

#[inline]
pub fn tangent<S: Float>(v: Vector<S>) -> Vector<S> {
    vector(-v.y, v.x)
}

#[inline]
pub fn normalized_tangent<S: Scalar>(v: Vector<S>) -> Vector<S> {
    tangent(v).normalize()
}

/// Angle between vectors v1 and v2 (oriented clockwise assuming y points downwards).
/// The result is a number between `0` and `2 * PI`.
///
/// ex: `directed_angle([0,1], [1,0]) = 3/2 Pi rad`
///
/// ```text
///     x       __
///   0-->     /  \
///  y|       |  x--> v2
///   v        \ |v1
///              v
/// ```
///
/// Or, assuming y points upwards:
/// `directed_angle([0,-1], [1,0]) = 1/2 Pi rad`
///
/// ```text
///   ^           v2
///  y|          x-->
///   0-->    v1 | /
///     x        v-
/// ```
///
#[inline]
pub fn directed_angle<S: Scalar>(v1: Vector<S>, v2: Vector<S>) -> S {
    let angle = S::fast_atan2(v2.y, v2.x) - S::fast_atan2(v1.y, v1.x);

    if angle < S::ZERO {
        angle + S::TWO * S::PI()
    } else {
        angle
    }
}

pub fn directed_angle2<S: Scalar>(center: Point<S>, a: Point<S>, b: Point<S>) -> S {
    directed_angle(a - center, b - center)
}

pub fn cubic_polynomial_roots<S: Scalar>(a: S, b: S, c: S, d: S) -> ArrayVec<S, 3> {
    let mut result = ArrayVec::new();

    let m = a.abs().max(b.abs()).max(c.abs()).max(d.abs());
    if !(m > S::ZERO) {
        // All coefficients are zero, or one of them is not a number.
        return result;
    }
    let (a, b, c, d) = (a / m, b / m, c / m, d / m);
    let epsilon = S::EPSILON;

    if S::abs(a) < epsilon {
        if S::abs(b) < epsilon {
            if S::abs(c) < epsilon {
                return result;
            }
            // linear equation
            result.push(-d / c);
            return result;
        }
        // quadratic equation
        let delta = c * c - S::FOUR * b * d;
        // Rounding can move the discriminant of a double root to either side of
        // zero.
        if S::abs(delta) < epsilon {
            result.push(-c / (S::TWO * b));
        } else if delta > S::ZERO {
            // q / b and d / q are the two roots, without subtracting values
            // that are close to each other.
            let sqrt_delta = S::sqrt(delta);
            let q = if c >= S::ZERO { -(c + sqrt_delta) } else { sqrt_delta - c } * S::HALF;
            let (r0, r1) = (q / b, d / q);
            if c >= S::ZERO {
                result.push(r0);
                result.push(r1);
            } else {
                result.push(r1);
                result.push(r0);
            }
        }
        return result;
    }

    let frac_1_3 = S::ONE / S::THREE;

    let bn = b / a;
    let cn = c / a;
    let dn = d / a;

    let delta0 = (S::THREE * cn - bn * bn) / S::NINE;
    let delta1 = (S::NINE * bn * cn - S::value(27.0) * dn - S::TWO * bn * bn * bn) / S::value(54.0);
    let delta_01 = delta0 * delta0 * delta0 + delta1 * delta1;

    if delta_01 >= S::ZERO {
        let delta_p_sqrt = delta1 + S::sqrt(delta_01);
        let delta_m_sqrt = delta1 - S::sqrt(delta_01);

        let s = delta_p_sqrt.signum() * S::abs(delta_p_sqrt).powf(frac_1_3);
        let t = delta_m_sqrt.signum() * S::abs(delta_m_sqrt).powf(frac_1_3);

        result.push(-bn * frac_1_3 + (s + t));

        // Don't add the repeated root when s + t == 0.
        if S::abs(s - t) < epsilon && S::abs(s + t) >= epsilon {
            result.push(-bn * frac_1_3 - (s + t) / S::TWO);
        }
    } else {
        let theta = S::acos(delta1 / S::sqrt(-delta0 * delta0 * delta0));
        let two_sqrt_delta0 = S::TWO * S::sqrt(-delta0);
        result.push(two_sqrt_delta0 * Float::cos(theta * frac_1_3) - bn * frac_1_3);
        result.push(
            two_sqrt_delta0 * Float::cos((theta + S::TWO * S::PI()) * frac_1_3) - bn * frac_1_3,
        );
        result.push(
            two_sqrt_delta0 * Float::cos((theta + S::FOUR * S::PI()) * frac_1_3) - bn * frac_1_3,
        );
    }

    result
}

#[test]
fn cubic_polynomial() {
    let cases: [([f32; 4], &[f32], f32); 11] = [
        ([2.0, -4.0, 2.0, 0.0], &[0.0, 1.0], 1e-6),
        ([-1.0, 1.0, -1.0, 1.0], &[1.0], 1e-6),
        ([-2.0, 2.0, -1.0, 10.0], &[2.0], 5e-5),
        // (t - 1)^3, with a triple root.
        ([1.0, -3.0, 3.0, -1.0], &[1.0], 5e-5),
        ([0.0, 1.0, -5.0, -14.0], &[-2.0, 7.0], 5e-5),
        // (t - 3)², with a double root.
        ([0.0, 1.0, -6.0, 9.0], &[3.0], 5e-5),
        ([0.0, 0.0, 2.0, 1.0], &[-0.5], 5e-5),
        ([0.0, 0.0, 0.0, 1.0], &[], 0.0),
        ([0.0, 0.0, 0.0, 0.0], &[], 0.0),
        // (t - 0.5) (t² + 1) * 1e-6.
        ([1e-6, -0.5e-6, 1e-6, -0.5e-6], &[0.5], 1e-5),
        ([0.0, 6.646663e-5, -9.966642e-5, 5.0959364e-4], &[], 0.0),
    ];
    for (coefficients, expected, epsilon) in cases {
        let [a, b, c, d] = coefficients;
        let mut roots = cubic_polynomial_roots(a, b, c, d);
        roots.sort_by(|a, b| a.total_cmp(b));
        assert_eq!(roots.len(), expected.len(), "{coefficients:?}: {roots:?}");
        for (root, expected) in roots.iter().zip(expected.iter()) {
            assert!((root - expected).abs() <= epsilon, "{:?}: {:?}", coefficients, roots);
        }

        // Scaling by powers of two is exact, so the roots are exactly the same.
        for exponent in [-40, -20, -10, 10, 20, 40] {
            let scale = 2.0f32.powi(exponent);
            let [a, b, c, d] = coefficients.map(|v| v * scale);
            let mut scaled_roots = cubic_polynomial_roots(a, b, c, d);
            scaled_roots.sort_by(|a, b| a.total_cmp(b));
            assert_eq!(scaled_roots, roots, "{:?} x 2^{}", coefficients, exponent);
        }
    }
}
