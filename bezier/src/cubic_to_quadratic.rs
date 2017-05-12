use Point;
use CubicBezierSegment;
use QuadraticBezierSegment;


/// Approximate a cubic bezier segment with a sequence of quadratic bezier segments.
pub fn cubic_to_quadratic<F>(cubic: &CubicBezierSegment, _tolerance: f32, cb: &mut F)
where
    F: FnMut(QuadraticBezierSegment),
{
    mid_point_approximation(cubic, cb);
}

/// Approximate a cubic bezier segments with four quadratic bezier segments using
/// using the mid-point approximation approach.
///
/// TODO: This isn't a very good approximation.
pub fn mid_point_approximation<F>(cubic: &CubicBezierSegment, cb: &mut F)
where
    F: FnMut(QuadraticBezierSegment),
{
    let (c1, c2) = cubic.split(0.5);
    let (c11, c12) = c1.split(0.5);
    let (c21, c22) = c2.split(0.5);
    cb(single_curve_approximation(&c11));
    cb(single_curve_approximation(&c12));
    cb(single_curve_approximation(&c21));
    cb(single_curve_approximation(&c22));
}

/// This is terrible as a general approximation but works well if the cubic
/// curve does not have inflection points and is "flat" enough. Typically usable
/// after subdiving the curve a few times.
pub fn single_curve_approximation(cubic: &CubicBezierSegment) -> QuadraticBezierSegment {
    let cp = line_intersection(cubic.from, cubic.ctrl1, cubic.ctrl2, cubic.to).unwrap();
    QuadraticBezierSegment {
        from: cubic.from,
        ctrl: cp,
        to: cubic.to,
    }
}

// TODO copy pasted from core::math_utils. Figure out what the dependency should
// look like.
pub fn line_intersection(a1: Point, a2: Point, b1: Point, b2: Point) -> Option<Point> {
    let det = (a1.x - a2.x) * (b1.y - b2.y) - (a1.y - a2.y) * (b1.x - b2.x);
    if det.abs() <= 0.000001 {
        // The lines are very close to parallel
        return None;
    }
    let inv_det = 1.0 / det;
    let a = a1.x * a2.y - a1.y * a2.x;
    let b = b1.x * b2.y - b1.y * b2.x;
    return Some(
        Point::new(
            (a * (b1.x - b2.x) - b * (a1.x - a2.x)) * inv_det,
            (a * (b1.y - b2.y) - b * (a1.y - a2.y)) * inv_det,
        )
    );
}
