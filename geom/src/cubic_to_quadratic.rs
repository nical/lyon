use crate::scalar::Scalar;
use crate::{QuadraticBezierSegment, CubicBezierSegment};
use crate::monotonic::Monotonic;
use crate::math::point;

/// Approximates a cubic bézier segment with a sequence of quadratic béziers.
pub fn cubic_to_quadratics<S: Scalar, F>(
    curve: &CubicBezierSegment<S>,
    tolerance: S,
    cb: &mut F
)
where
    F: FnMut(&QuadraticBezierSegment<S>)
{
    debug_assert!(tolerance >= S::EPSILON);

    let mut sub_curve = curve.clone();
    let mut range = S::ZERO..S::ONE;
    loop {
        if single_curve_approximation_test(&sub_curve, tolerance) {
            cb(&single_curve_approximation(&sub_curve));
            if range.end >= S::ONE {
                return;
            }
            range.start = range.end;
            range.end = S::ONE;
        } else {
            range.end = (range.start + range.end) * S::HALF;
        }
        sub_curve = curve.split_range(range.clone());
    }
}

/// This is terrible as a general approximation but works if the cubic
/// curve does not have inflection points and is "flat" enough. Typically
/// usables after subdiving the curve a few times.
pub fn single_curve_approximation<S: Scalar>(cubic: &CubicBezierSegment<S>) -> QuadraticBezierSegment<S> {
    let c1 = (cubic.ctrl1 * S::THREE - cubic.from) * S::HALF;
    let c2 = (cubic.ctrl2 * S::THREE - cubic.to) * S::HALF;
    QuadraticBezierSegment {
        from: cubic.from,
        ctrl: ((c1 + c2) * S::HALF).to_point(),
        to: cubic.to,
    }
}

/// Evaluates an upper bound on the maximum distance between the curve
/// and its quadratic approximation obtained using the single curve approximation.
pub fn single_curve_approximation_error<S: Scalar>(curve: &CubicBezierSegment<S>) -> S {
    // See http://caffeineowl.com/graphics/2d/vectorial/cubic2quad01.html
    S::sqrt(S::THREE) / S::value(36.0) * ((curve.to - curve.ctrl2 * S::THREE) + (curve.ctrl1 * S::THREE - curve.from)).length()
}

// Similar to single_curve_approximation_error avoiding the square root.
fn single_curve_approximation_test<S: Scalar>(curve: &CubicBezierSegment<S>, tolerance: S) -> bool {
    S::THREE / S::value(1296.0) * ((curve.to - curve.ctrl2 * S::THREE) + (curve.ctrl1 * S::THREE - curve.from)).square_length() <= tolerance * tolerance
}

pub fn cubic_to_monotonic_quadratics<S: Scalar, F>(
    curve: &CubicBezierSegment<S>,
    tolerance: S,
    cb: &mut F
)
where
    F: FnMut(&Monotonic<QuadraticBezierSegment<S>>),
{
    curve.for_each_monotonic_range(|range| {
        cubic_to_quadratics(
            &curve.split_range(range),
            tolerance,
            &mut|c| {
                cb(&make_monotonic(c))
            }
        );
    });
}

// Unfortunately the single curve approximation can turn a monotonic cubic
// curve into an almost-but-exactly monotonic quadratic segment, so we may
// need to nudge the control point slightly to not break downstream code
// that rely on the monotonicity.
fn make_monotonic<S: Scalar>(curve: &QuadraticBezierSegment<S>) -> Monotonic<QuadraticBezierSegment<S>>{
    Monotonic {
        segment: QuadraticBezierSegment {
            from: curve.from,
            ctrl: point(
                S::min(S::max(curve.from.x, curve.ctrl.x), curve.to.x),
                S::min(S::max(curve.from.y, curve.ctrl.y), curve.to.y),
            ),
            to: curve.to,
        }
    }
}

/*
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
            return Some(single_curve_approximation(&self.curve.split_range(a..b)));
        }
        None
    }
}
*/

#[test]
fn test_cubic_to_quadratics() {
    use euclid::approxeq::ApproxEq;

    let quadratic = QuadraticBezierSegment {
        from: point(1.0, 2.0),
        ctrl: point(10.0, 5.0),
        to: point(0.0, 1.0),
    };

    let mut count = 0;
    cubic_to_quadratics(&quadratic.to_cubic(), 0.0001, &mut|c| {
        assert!(count == 0);
        assert!(c.from.approx_eq(&quadratic.from));
        assert!(c.ctrl.approx_eq(&quadratic.ctrl));
        assert!(c.to.approx_eq(&quadratic.to));
        count += 1;
    });

    let cubic = CubicBezierSegment {
        from: point(1.0, 1.0),
        ctrl1: point(10.0, 2.0),
        ctrl2: point(1.0, 3.0),
        to: point(10.0, 4.0),
    };

    let mut prev = cubic.from;
    let mut count = 0;
    cubic_to_quadratics(&cubic, 0.01, &mut|c| {
        assert!(c.from.approx_eq(&prev));
        prev = c.to;
        count += 1;
    });
    assert!(prev.approx_eq(&cubic.to));
    assert!(count < 10);
    assert!(count > 4);
}

#[test]
fn test_cubic_to_monotonic_quadratics() {
    use euclid::approxeq::ApproxEq;

    let cubic = CubicBezierSegment {
        from: point(1.0, 1.0),
        ctrl1: point(10.0, 2.0),
        ctrl2: point(1.0, 3.0),
        to: point(10.0, 4.0),
    };

    let mut prev = cubic.from;
    let mut count = 0;
    cubic_to_monotonic_quadratics(&cubic, 0.01, &mut|c| {
        assert!(c.segment().from.approx_eq(&prev));
        prev = c.segment().to;
        assert!(c.segment().is_monotonic());
        count += 1;
    });
    assert!(prev.approx_eq(&cubic.to));
    assert!(count < 10);
    assert!(count > 4);
}
