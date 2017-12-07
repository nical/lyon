use CubicBezierSegment;
use QuadraticBezierSegment;
use Line;

/// Approximate a cubic bezier segment with a sequence of quadratic bezier segments.
pub fn cubic_to_quadratic<F>(cubic: &CubicBezierSegment, _tolerance: f32, cb: &mut F)
where
    F: FnMut(QuadraticBezierSegment),
{
    inflection_based_approximation(cubic, cb);
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

/// Approximate the curve by first splitting it at the inflection points
/// and then using single or mid point approximations depending on the
/// size of the parts.
pub fn inflection_based_approximation<F>(curve: &CubicBezierSegment, cb: &mut F)
where
    F: FnMut(QuadraticBezierSegment),
{
    fn step<F>(
        curve: &CubicBezierSegment,
        t0: f32, t1: f32,
        cb: &mut F
    )
    where
        F: FnMut(QuadraticBezierSegment),
    {
        let dt = t1 - t0;
        if dt > 0.01 {
            let sub_curve = curve.split_range(t0..t1);
            if dt < 0.25 {
                cb(single_curve_approximation(&sub_curve));
            } else {
                mid_point_approximation(&sub_curve, cb);
            }
        }
    }

    let inflections = curve.find_inflection_points();

    let mut t: f32 = 0.0;
    for inflection in inflections {
        // don't split if we are very close to the end.
        let next = if inflection < 0.99 { inflection } else { 1.0 };

        step(curve, t, next, cb);
        t = next;
    }

    if t < 1.0 {
        step(curve, t, 1.0, cb)
    }
}
