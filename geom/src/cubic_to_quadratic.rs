use scalar::Scalar;
use CubicBezierSegment;
use QuadraticBezierSegment;
use Line;
use arrayvec::ArrayVec;

/// Approximate a cubic bezier segment with a sequence of quadratic bezier segments.
pub fn cubic_to_quadratic<S: Scalar, F>(cubic: &CubicBezierSegment<S>, _tolerance: S, cb: &mut F)
where
    F: FnMut(QuadraticBezierSegment<S>),
{
    inflection_based_approximation(cubic, cb);
}

/// Approximate a cubic bezier segments with four quadratic bezier segments using
/// using the mid-point approximation approach.
///
/// TODO: This isn't a very good approximation.
pub fn mid_point_approximation<S: Scalar, F>(cubic: &CubicBezierSegment<S>, cb: &mut F)
where
    F: FnMut(QuadraticBezierSegment<S>),
{
    let (c1, c2) = cubic.split(S::HALF);
    let (c11, c12) = c1.split(S::HALF);
    let (c21, c22) = c2.split(S::HALF);
    cb(single_curve_approximation(&c11));
    cb(single_curve_approximation(&c12));
    cb(single_curve_approximation(&c21));
    cb(single_curve_approximation(&c22));
}

/// This is terrible as a general approximation but works well if the cubic
/// curve does not have inflection points and is "flat" enough. Typically usable
/// after subdiving the curve a few times.
pub fn single_curve_approximation<S: Scalar>(cubic: &CubicBezierSegment<S>) -> QuadraticBezierSegment<S> {
    let l1 = Line { point: cubic.from, vector: cubic.ctrl1 - cubic.from };
    let l2 = Line { point: cubic.to, vector: cubic.ctrl2 - cubic.to };
    let cp = match l1.intersection(&l2) {
        Some(p) => p,
        None => cubic.from.lerp(cubic.to, S::HALF),
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
pub fn inflection_based_approximation<S: Scalar, F>(curve: &CubicBezierSegment<S>, cb: &mut F)
where
    F: FnMut(QuadraticBezierSegment<S>),
{
    fn step<S: Scalar, F>(
        curve: &CubicBezierSegment<S>,
        t0: S, t1: S,
        cb: &mut F
    )
    where
        F: FnMut(QuadraticBezierSegment<S>),
    {
        let dt = t1 - t0;
        if dt > S::value(0.01) {
            let sub_curve = curve.split_range(t0..t1);
            if dt < S::value(0.25) {
                cb(single_curve_approximation(&sub_curve));
            } else {
                mid_point_approximation(&sub_curve, cb);
            }
        }
    }

    let inflections = curve.find_inflection_points();

    let mut t = S::ZERO;
    for inflection in inflections {
        // don't split if we are very close to the end.
        let next = if inflection < S::value(0.99) { inflection } else { S::ONE };

        step(curve, t, next, cb);
        t = next;
    }

    if t < S::ONE {
        step(curve, t, S::ONE, cb)
    }
}


// TODO(breaking change) - take the curve by reference in the callback.
pub fn monotonic_approximation<S: Scalar, F>(curve: &CubicBezierSegment<S>, cb: &mut F)
where
    F: FnMut(QuadraticBezierSegment<S>),
{
    curve.for_each_monotonic_range(|range| {
        cb(curve.split_range(range).to_quadratic());
    });
}

pub struct MonotonicQuadraticBezierSegments<S> {
    curve: CubicBezierSegment<S>,
    splits: ArrayVec<[S; 4]>,
    t0: S,
    idx: u8,
}

impl<S: Scalar> MonotonicQuadraticBezierSegments<S> {
    pub fn new(curve: &CubicBezierSegment<S>) -> Self {
        let mut splits = ArrayVec::new();
        curve.for_each_monotonic_t(|t| {
            splits.push(t);
        });
        MonotonicQuadraticBezierSegments {
            curve: *curve,
            splits,
            t0: S::ZERO,
            idx: 0,
        }
    }
}

impl<S: Scalar> Iterator for MonotonicQuadraticBezierSegments<S> {
    type Item = QuadraticBezierSegment<S>;
    fn next(&mut self) -> Option<QuadraticBezierSegment<S>> {
        let i = self.idx as usize;
        if i < self.splits.len() {
            let a = self.t0;
            let b = self.splits[i];
            self.t0 = b;
            self.idx += 1;
            return Some(self.curve.split_range(a..b).to_quadratic());
        }
        None
    }
}
