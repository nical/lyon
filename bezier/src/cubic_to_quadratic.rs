use CubicBezierSegment;
use QuadraticBezierSegment;
use Line;

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
    let l1 = Line { point: cubic.from, vector: cubic.ctrl1 - cubic.from };
    let l2 = Line { point: cubic.to, vector: cubic.ctrl2 - cubic.to };
    let cp = match l1.intersection(&l2) {
        Some(p) => p,
        None => cubic.from.lerp(cubic.to, 0.5),
    };
    QuadraticBezierSegment {
        from: cubic.from,
        ctrl: cp,
        to: cubic.to,
    }
}
