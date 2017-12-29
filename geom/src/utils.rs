use scalar::{Float, FloatExt, FloatConst, Trig, ApproxEq};
use generic_math::{Point, Vector, vector, Angle};
use arrayvec::ArrayVec;

#[inline]
pub fn min_max<S: Float>(a: S, b: S) -> (S, S) {
    if a < b { (a, b) } else { (b, a) }
}

#[inline]
pub fn tangent<S: Float>(v: Vector<S>) -> Vector<S> {
    vector(-v.y, v.x)
}

#[inline]
pub fn normalized_tangent<S: Float + ApproxEq<S>>(v: Vector<S>) -> Vector<S> {
    tangent(v).normalize()
}

/// Angle between vectors v1 and v2 (oriented clockwise assyming y points downwards).
/// The result is a number between 0 and 2*PI.
///
/// ex: `directed_angle([0,1], [1,0]) = 3/2 Pi rad`
///     x       __
///   0-->     /  \
///  y|       |  x--> v2
///   v        \ |v1
///              v
///
/// Or, assuming y points upwards:
/// `directed_angle([0,-1], [1,0]) = 1/2 Pi rad`
///
///   ^           v2
///  y|          x-->
///   0-->    v1 | /
///     x        v-
///
#[inline]
pub fn directed_angle<S: Float + FloatConst + Trig>(a: Vector<S>, b: Vector<S>) -> S {
    let angle = S::fast_atan2(b.y, b.x) - S::fast_atan2(a.y, a.x);
    return if angle < S::zero() { angle + S::c(2.0) * S::PI() } else { angle };
}

pub fn directed_angle2<S: Float + FloatConst + Trig>(center: Point<S>, a: Point<S>, b: Point<S>) -> S {
    directed_angle(a - center, b - center)
}

#[inline]
pub fn vector_angle<S: Float + FloatConst + Trig>(v: Vector<S>) -> Angle<S> { Angle::radians(S::fast_atan2(v.y, v.x)) }

pub fn cubic_polynomial_roots<S: Float + FloatConst>(a: S, b: S, c: S, d: S) -> ArrayVec<[S; 3]> {
    let mut result = ArrayVec::new();

    if a.abs() < S::c(1e-6) {
        // quadratic equation
        let delta = b * b - S::c(4.0) * a * c;
        if delta > S::zero() {
            let sqrt_delta = delta.sqrt();
            result.push((-b - sqrt_delta) / (S::c(2.0) * a));
            result.push((-b + sqrt_delta) / (S::c(2.0) * a));
        } else if delta.abs() < S::c(1e-6) {
            result.push(-b / (S::c(2.0) * a));
        }
        return result;
    }

    let frac_1_3 = S::c(1.0 / 3.0);

    let bn = b / a;
    let cn = c / a;
    let dn = d / a;

    let delta0 = (S::c(3.0) * cn - bn * bn) / S::c(9.0);
    let delta1 = (S::c(9.0) * bn * cn - S::c(27.0) * dn - S::c(2.0) * bn * bn * bn) / S::c(54.0);
    let delta_01 = delta0 * delta0 * delta0 + delta1 * delta1;

    if delta_01 >= S::zero() {
        let delta_p_sqrt = delta1 + delta_01.sqrt();
        let delta_m_sqrt = delta1 - delta_01.sqrt();

        let s = delta_p_sqrt.signum() * delta_p_sqrt.abs().powf(frac_1_3);
        let t = delta_m_sqrt.signum() * delta_m_sqrt.abs().powf(frac_1_3);

        result.push(-bn * frac_1_3 + (s + t));

        if (s - t).abs() < S::c(1e-5) {
            result.push(-bn * frac_1_3 - (s + t) / S::c(2.0));
        }
    } else {
        let theta = (delta1 / (-delta0 * delta0 * delta0).sqrt()).acos();
        let two_sqrt_delta0 = S::c(2.0) * (-delta0).sqrt();
        result.push(two_sqrt_delta0 * (theta * frac_1_3).cos() - bn * frac_1_3);
        result.push(two_sqrt_delta0 * ((theta + S::c(2.0) * S::PI()) * frac_1_3).cos() - bn * frac_1_3);
        result.push(two_sqrt_delta0 * ((theta + S::c(4.0) * S::PI()) * frac_1_3).cos() - bn * frac_1_3);
    }

    //result.sort();

    return result;
}

#[test]
fn cubic_polynomial() {
    fn assert_approx_eq(a: ArrayVec<[f32; 3]>, b: &[f32], epsilon: f32) {
        for i in 0..a.len() {
            if (a[i] - b[i]).abs() > epsilon {
                println!("{:?} != {:?}", a, b);
            }
            assert!((a[i] - b[i]).abs() <= epsilon);
        }
        assert_eq!(a.len(), b.len());
    }

    assert_approx_eq(cubic_polynomial_roots(2.0, -4.0, 2.0, 0.0), &[0.0, 1.0], 0.0000001);
    assert_approx_eq(cubic_polynomial_roots(-1.0, 1.0, -1.0, 1.0), &[1.0], 0.000001);
    assert_approx_eq(cubic_polynomial_roots(-2.0, 2.0, -1.0, 10.0), &[2.0], 0.00005);
}
