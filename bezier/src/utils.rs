use {Point, Vector, vec2};
use std::f32::consts::PI;
use arrayvec::ArrayVec;

#[inline]
pub fn tangent(v: Vector) -> Vector {
    vec2(-v.y, v.x)
}

#[inline]
pub fn normalized_tangent(v: Vector) -> Vector {
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
pub fn directed_angle(a: Vector, b: Vector) -> f32 {
    let angle = fast_atan2(b.y, b.x) - fast_atan2(a.y, a.x);
    return if angle < 0.0 { angle + 2.0 * PI } else { angle };
}

pub fn directed_angle2(center: Point, a: Point, b: Point) -> f32 {
    directed_angle(a - center, b - center)
}

/// A slightly faster approximation of atan2.
///
/// Note that it does not deal with the case where both x and y are 0.
#[inline]
pub fn fast_atan2(y: f32, x: f32) -> f32 {
    let x_abs = x.abs();
    let y_abs = y.abs();
    let a = x_abs.min(y_abs) / x_abs.max(y_abs);
    let s = a * a;
    let mut r = ((-0.0464964749 * s + 0.15931422) * s - 0.327622764) * s * a + a;
    if y_abs > x_abs {
        r = 1.57079637 - r;
    }
    if x < 0.0 {
        r = 3.14159274 - r
    }
    if y < 0.0 {
        r = -r
    }
    return r;
}

pub fn cubic_polynomial_roots(a: f32, b: f32, c: f32, d: f32) -> ArrayVec<[f32; 3]> {
    let mut result = ArrayVec::new();

    if a.abs() < 1e-6 {
        // quadratic equation
        let delta = b * b - 4.0 * a * c;
        if delta > 0.0 {
            let sqrt_delta = delta.sqrt();
            result.push((-b - sqrt_delta) / (2.0 * a));
            result.push((-b + sqrt_delta) / (2.0 * a));
        } else if delta.abs() < 1e-6 {
            result.push(-b / (2.0 * a));
        }
        return result;
    }

    let frac_1_3 = 1.0 / 3.0;

    let bn = b / a;
    let cn = c / a;
    let dn = d / a;

    let delta0 = (3.0 * cn - bn * bn) / 9.0;
    let delta1 = (9.0 * bn * cn - 27.0 * dn - 2.0 * bn * bn * bn) / 54.0;
    let delta_01 = delta0 * delta0 * delta0 + delta1 * delta1;

    if delta_01 >= 0.0 {
        let delta_p_sqrt = delta1 + delta_01.sqrt();
        let delta_m_sqrt = delta1 - delta_01.sqrt();

        let s = delta_p_sqrt.signum() * delta_p_sqrt.abs().powf(frac_1_3);
        let t = delta_m_sqrt.signum() * delta_m_sqrt.abs().powf(frac_1_3);

        result.push(-bn * frac_1_3 + (s + t));

        if (s - t).abs() < 1e-5 {
            result.push(-bn * frac_1_3 - (s + t) / 2.0);
        }
    } else {
        let theta = (delta1 / (-delta0 * delta0 * delta0).sqrt()).acos();
        let two_sqrt_delta0 = 2.0 * (-delta0).sqrt();
        result.push(two_sqrt_delta0 * (theta * frac_1_3).cos() - bn * frac_1_3);
        result.push(two_sqrt_delta0 * ((theta + 2.0 * PI) * frac_1_3).cos() - bn * frac_1_3);
        result.push(two_sqrt_delta0 * ((theta + 4.0 * PI) * frac_1_3).cos() - bn * frac_1_3);
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
