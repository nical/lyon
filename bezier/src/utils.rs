use {Point, Vec2, vec2};
use std::f32::consts::PI;

#[inline]
pub fn tangent(v: Vec2) -> Vec2 {
    vec2(-v.y, v.x)
}

#[inline]
pub fn normalized_tangent(v: Vec2) -> Vec2 {
    tangent(v).normalize()
}

/// Angle between vectors v1 and v2 (oriented clockwise assyming y points downwards).
/// The result is a number between 0 and 2*PI.
///
/// ex: directed_angle([0,1], [1,0]) = 3/2 Pi rad
///     x       __
///   0-->     /  \
///  y|       |  x--> v2
///   v        \ |v1
///              v
///
/// Or, assuming y points upwards:
/// directed_angle([0,-1], [1,0]) = 1/2 Pi rad
///
///   ^           v2
///  y|          x-->
///   0-->    v1 | /
///     x        v-
///
#[inline]
pub fn directed_angle(a: Vec2, b: Vec2) -> f32 {
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
