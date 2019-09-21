///! Utilities to flatten cubic bezier curve segments, implmeneted both with callback and
///! iterator based APIs.
///!
///! The algorithm implemented here is based on:
///! http://cis.usouthal.edu/~hain/general/Publications/Bezier/Bezier%20Offset%20Curves.pdf
///! It produces a better approximations than the usual recursive subdivision approach (or
///! in other words, it generates less points for a given tolerance threshold).

use crate::CubicBezierSegment;
use crate::scalar::Scalar;
use crate::generic_math::Point;
use arrayvec::ArrayVec;

/// An iterator over a cubic bezier segment that yields line segments approximating the
/// curve for a given approximation threshold.
///
/// The iterator starts at the first point *after* the origin of the curve and ends at the
/// destination.
pub struct Flattened<S: Scalar> {
    remaining_curve: CubicBezierSegment<S>,
    // current portion of the curve, does not have inflections.
    current_curve: Option<CubicBezierSegment<S>>,
    next_inflection: Option<S>,
    following_inflection: Option<S>,
    tolerance: S,
    check_inflection: bool,
}

impl<S: Scalar> Flattened<S> {
    /// Creates an iterator that yields points along a cubic bezier segment, useful to build a
    /// flattened approximation of the curve given a certain tolerance.
    pub fn new(bezier: CubicBezierSegment<S>, tolerance: S) -> Self {
        let mut inflections: ArrayVec<[S; 2]> = ArrayVec::new();
        find_cubic_bezier_inflection_points(&bezier, &mut|t| { inflections.push(t); });

        let mut iter = Flattened {
            remaining_curve: bezier,
            current_curve: None,
            next_inflection: inflections.get(0).cloned(),
            following_inflection: inflections.get(1).cloned(),
            tolerance: tolerance,
            check_inflection: false,
        };

        if let Some(&t1) = inflections.get(0) {
            let (before, after) = bezier.split(t1);
            iter.current_curve = Some(before);
            iter.remaining_curve = after;
            if let Some(&t2) = inflections.get(1) {
                // Adjust the second inflection since we removed the part before the
                // first inflection from the bezier curve.
                let t2 = (t2 - t1) / (S::ONE - t1);
                iter.following_inflection = Some(t2)
            }

            return iter;
        }

        iter.current_curve = Some(bezier);

        return iter;
    }
}

impl<S: Scalar> Iterator for Flattened<S> {
    type Item = Point<S>;
    fn next(&mut self) -> Option<Point<S>> {

        if self.current_curve.is_none() && self.next_inflection.is_some() {
            if let Some(t2) = self.following_inflection {
                // No need to re-map t2 in the curve because we already did iter_points
                // in the iterator's new function.
                let (before, after) = self.remaining_curve.split(t2);
                self.current_curve = Some(before);
                self.remaining_curve = after;
            } else {
                // The last chunk doesn't have inflection points, use it.
                self.current_curve = Some(self.remaining_curve);
            }

            // Pop the inflection stack.
            self.next_inflection = self.following_inflection;
            self.following_inflection = None;
            self.check_inflection = true;
        }

        if let Some(sub_curve) = self.current_curve {
            if sub_curve.is_a_point(self.tolerance) {
                self.current_curve = None;
                return self.next();
            }

            if self.check_inflection {
                self.check_inflection = false;
                if let Some(tf) = inflection_approximation_range(&sub_curve, self.tolerance) {
                    let next = sub_curve.after_split(tf);
                    self.current_curve = Some(next);
                    return Some(next.from);
                }
            }

            // We are iterating over a sub-curve that does not have inflections.
            let t = no_inflection_flattening_step(&sub_curve, self.tolerance);
            if t >= S::ONE {
                let to = sub_curve.to;
                self.current_curve = None;
                return Some(to);
            }

            let next_curve = sub_curve.after_split(t);
            self.current_curve = Some(next_curve);
            return Some(next_curve.from);
        }

        return None;
    }
}

pub fn flatten_cubic_bezier<S: Scalar, F: FnMut(Point<S>)>(
    mut bezier: CubicBezierSegment<S>,
    tolerance: S,
    call_back: &mut F,
) {
    let mut inflections: ArrayVec<[S; 2]> = ArrayVec::new();
    find_cubic_bezier_inflection_points(&bezier, &mut|t| { inflections.push(t); });

    if let Some(&t1) = inflections.get(0) {
        bezier = flatten_including_inflection(&bezier, t1, tolerance, call_back);
        if let Some(&t2) = inflections.get(1) {
            // Adjust the second inflection since we removed the part before the
            // first inflection from the bezier curve.
            let t2 = (t2 - t1) / (S::ONE - t1);
            bezier = flatten_including_inflection(&bezier, t2, tolerance, call_back);
        }
    }

    flatten_cubic_no_inflection(bezier, tolerance, call_back);
}

pub fn flatten_cubic_bezier_with_t<S: Scalar, F: FnMut(Point<S>, S)>(
    mut bezier: CubicBezierSegment<S>,
    tolerance: S,
    call_back: &mut F,
) {
    let mut inflections: ArrayVec<[S; 2]> = ArrayVec::new();
    find_cubic_bezier_inflection_points(&bezier, &mut|t| { inflections.push(t); });

    let mut t = S::ZERO;
    for t_inflection in inflections {
        let (before, mut after) = bezier.split(t_inflection);

        // Flatten up to the inflection point.
        flatten_cubic_no_inflection_with_t(before, t, t_inflection, tolerance, call_back);

        t = t_inflection;

        // Approximate the inflection with a segment if need be.
        if let Some(tf) = inflection_approximation_range(&after, tolerance) {
            after = after.after_split(tf);
            t += tf * (S::ONE - t);
            call_back(after.from, t);
        }

        bezier = after;
    }

    // Do the rest of the curve.
    flatten_cubic_no_inflection_with_t(bezier, t, S::ONE, tolerance, call_back);
}

// Flatten the curve up to the the inflection point and its approximation range included.
fn flatten_including_inflection<S: Scalar, F: FnMut(Point<S>)>(
    bezier: &CubicBezierSegment<S>,
    up_to_t: S,
    tolerance: S,
    call_back: &mut F,
) -> CubicBezierSegment<S> {
    let (before, mut after) = bezier.split(up_to_t);
    flatten_cubic_no_inflection(before, tolerance, call_back);

    if let Some(tf) = inflection_approximation_range(&after, tolerance) {
        after = after.after_split(tf);
        call_back(after.from);
    }

    after
}

// The algorithm implemented here is based on:
// http://cis.usouthal.edu/~hain/general/Publications/Bezier/Bezier%20Offset%20Curves.pdf
//
// The basic premise is that for a small t the third order term in the
// equation of a cubic bezier curve is insignificantly small. This can
// then be approximated by a quadratic equation for which the maximum
// difference from a linear approximation can be much more easily determined.
fn flatten_cubic_no_inflection<S: Scalar, F: FnMut(Point<S>)>(
    mut curve: CubicBezierSegment<S>,
    tolerance: S,
    call_back: &mut F,
) {
    let end = curve.to;

    loop {
        let step = no_inflection_flattening_step(&curve, tolerance);

        if step >= S::ONE {
            if !curve.is_a_point(S::ZERO) {
                call_back(end);
            }

            break;
        }
        curve = curve.after_split(step);
        call_back(curve.from);
    }
}

fn flatten_cubic_no_inflection_with_t<S: Scalar, F: FnMut(Point<S>, S)>(
    mut curve: CubicBezierSegment<S>,
    mut t0: S,
    t1: S,
    tolerance: S,
    call_back: &mut F,
) {
    let end = curve.to;
    loop {
        let step = no_inflection_flattening_step(&curve, tolerance);

        if step >= S::ONE {
            if t0 < t1 {
                call_back(end, t1);
            }
            break;
        }

        curve = curve.after_split(step);
        t0 += step * (t1 - t0);
        call_back(curve.from, t0);
    }
}

fn no_inflection_flattening_step<S: Scalar>(bezier: &CubicBezierSegment<S>, tolerance: S) -> S {
    let v1 = bezier.ctrl1 - bezier.from;
    let v2 = bezier.ctrl2 - bezier.from;

    // This function assumes that the bézier segment is not starting at an inflection point,
    // otherwise the following cross product may result in very small numbers which will hit
    // floating point precision issues.

    // To remove divisions and check for divide-by-zero, this is optimized from:
    // s2 = (v2.x * v1.y - v2.y * v1.x) / hypot(v1.x, v1.y);
    // t = 2 * sqrt(tolerance / (3. * abs(s2)));
    let v2_cross_v1 = v2.cross(v1);
    if v2_cross_v1 == S::ZERO {
        return S::ONE;
    }
    let s2inv = v1.x.hypot(v1.y) / v2_cross_v1;

    let t = S::TWO * S::sqrt(tolerance * S::abs(s2inv) / S::THREE);

    // TODO: We start having floating point precision issues if this constant
    // is closer to 1.0 with a small enough tolerance threshold.
    if t >= S::value(0.995) || t == S::ZERO {
        return S::ONE;
    }

    return t;
}

// Find the inflection points of a cubic bezier curve.
pub(crate) fn find_cubic_bezier_inflection_points<S, F>(
    bezier: &CubicBezierSegment<S>,
    cb: &mut F,
)
where
    S: Scalar,
    F: FnMut(S)
{
    // Find inflection points.
    // See www.faculty.idc.ac.il/arik/quality/appendixa.html for an explanation
    // of this approach.
    let pa = bezier.ctrl1 - bezier.from;
    let pb = bezier.ctrl2.to_vector() - (bezier.ctrl1.to_vector() * S::TWO) + bezier.from.to_vector();
    let pc = bezier.to.to_vector() - (bezier.ctrl2.to_vector() * S::THREE) + (bezier.ctrl1.to_vector() * S::THREE) - bezier.from.to_vector();

    let a = pb.cross(pc);
    let b = pa.cross(pc);
    let c = pa.cross(pb);

    if S::abs(a) < S::EPSILON {
        // Not a quadratic equation.
        if S::abs(b) < S::EPSILON {
            // Instead of a linear acceleration change we have a constant
            // acceleration change. This means the equation has no solution
            // and there are no inflection points, unless the constant is 0.
            // In that case the curve is a straight line, essentially that means
            // the easiest way to deal with is is by saying there's an inflection
            // point at t == 0. The inflection point approximation range found will
            // automatically extend into infinity.
            if S::abs(c) < S::EPSILON {
                cb(S::ZERO);
            }
        } else {
            let t = -c / b;
            if in_range(t) {
                cb(t);
            }
        }

        return;
    }

    fn in_range<S: Scalar>(t: S) -> bool { t >= S::ZERO && t < S::ONE }

    let discriminant = b * b - S::FOUR * a * c;

    if discriminant < S::ZERO {
        return;
    }

    if discriminant < S::EPSILON {
        let t = -b / (S::TWO * a);

        if in_range(t) {
            cb(t);
        }

        return;
    }

    // This code is derived from https://www2.units.it/ipl/students_area/imm2/files/Numerical_Recipes.pdf page 184.
    // Computing the roots this way avoids precision issues when a, c or both are small.
    let discriminant_sqrt = S::sqrt(discriminant);
    let sign_b = if b >= S::ZERO { S::ONE } else { -S::ONE };
    let q = -S::HALF * (b + sign_b * discriminant_sqrt);
    let mut first_inflection = q / a;
    let mut second_inflection = c / q;

    if first_inflection > second_inflection {
        std::mem::swap(&mut first_inflection, &mut second_inflection);
    }

    if in_range(first_inflection) {
        cb(first_inflection);
    }

    if in_range(second_inflection) {
        cb(second_inflection);
    }
}

// Find the range around the start of the curve where the curve can locally be approximated
// with a line segment, given a tolerance threshold.
fn inflection_approximation_range<S: Scalar>(
    bezier: &CubicBezierSegment<S>,
    tolerance: S,
) -> Option<S> {
    // Transform the curve such that it starts at the origin.
    let p1 = bezier.ctrl1 - bezier.from;
    let p2 = bezier.ctrl2 - bezier.from;
    let p3 = bezier.to - bezier.from;

    // Thus, curve(t) = t^3 * (3*p1 - 3*p2 + p3) + t^2 * (-6*p1 + 3*p2) + t * (3*p1).
    // Since curve(0) is an inflection point, cross(p1, p2) = 0, i.e. p1 and p2 are parallel.

    // Let s(t) = s3 * t^3 be the (signed) perpendicular distance of curve(t) from a line that will be determined below.
    let s3;
    if S::abs(p1.x) < S::EPSILON && S::abs(p1.y) < S::EPSILON {
        // Assume p1 = 0.
        if S::abs(p2.x) < S::EPSILON && S::abs(p2.y) < S::EPSILON {
            // Assume p2 = 0.
            // The curve itself is a line or a point.
            return None;
        } else {
            // In this case p2 is away from zero.
            // Choose the line in direction p2.
            s3 = p2.cross(p3) / p2.length();
        }
    } else {
        // In this case p1 is away from zero.
        // Choose the line in direction p1 and use that p1 and p2 are parallel.
        s3 = p1.cross(p3) / p1.length();
    }

    // Calculate the maximal t value such that the (absolute) distance is within the tolerance.
    let tf = S::abs(tolerance / s3).powf(S::ONE / S::THREE);

    return if tf < S::ONE { Some(tf) } else { None };
}

#[cfg(test)]
fn print_arrays(a: &[Point<f32>], b: &[Point<f32>]) {
    println!("left:  {:?}", a);
    println!("right: {:?}", b);
}

#[cfg(test)]
fn assert_approx_eq(a: &[Point<f32>], b: &[Point<f32>]) {
    if a.len() != b.len() {
        print_arrays(a, b);
        panic!("Lenths differ ({} != {})", a.len(), b.len());
    }
    for i in 0..a.len() {
        if f32::abs(a[i].x - b[i].x) > 0.0000001 || f32::abs(a[i].y - b[i].y) > 0.0000001 {
            print_arrays(a, b);
            panic!("The arrays are not equal");
        }
    }
}

#[test]
fn test_iterator_builder_1() {
    let tolerance = 0.01;
    let c1 = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(1.0, 0.0),
        ctrl2: Point::new(1.0, 1.0),
        to: Point::new(0.0, 1.0),
    };
    let iter_points: Vec<Point<f32>> = c1.flattened(tolerance).collect();
    let mut builder_points = Vec::new();
    c1.for_each_flattened(tolerance, &mut |p| { builder_points.push(p); });

    assert!(iter_points.len() > 2);
    assert_approx_eq(&iter_points[..], &builder_points[..]);
}

#[test]
fn test_iterator_builder_2() {
    let tolerance = 0.01;
    let c1 = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(1.0, 0.0),
        ctrl2: Point::new(0.0, 1.0),
        to: Point::new(1.0, 1.0),
    };
    let iter_points: Vec<Point<f32>> = c1.flattened(tolerance).collect();
    let mut builder_points = Vec::new();
    c1.for_each_flattened(tolerance, &mut |p| { builder_points.push(p); });

    assert!(iter_points.len() > 2);
    assert_approx_eq(&iter_points[..], &builder_points[..]);
}

#[test]
fn test_iterator_builder_3() {
    let tolerance = 0.01;
    let c1 = CubicBezierSegment {
        from: Point::new(141.0, 135.0),
        ctrl1: Point::new(141.0, 130.0),
        ctrl2: Point::new(140.0, 130.0),
        to: Point::new(131.0, 130.0),
    };
    let iter_points: Vec<Point<f32>> = c1.flattened(tolerance).collect();
    let mut builder_points = Vec::new();
    c1.for_each_flattened(tolerance, &mut |p| { builder_points.push(p); });

    assert!(iter_points.len() > 2);
    assert_approx_eq(&iter_points[..], &builder_points[..]);
}

#[test]
fn test_issue_19() {
    let tolerance = 0.15;
    let c1 = CubicBezierSegment {
        from: Point::new(11.71726, 9.07143),
        ctrl1: Point::new(1.889879, 13.22917),
        ctrl2: Point::new(18.142855, 19.27679),
        to: Point::new(18.142855, 19.27679),
    };
    let iter_points: Vec<Point<f32>> = c1.flattened(tolerance).collect();
    let mut builder_points = Vec::new();
    c1.for_each_flattened(tolerance, &mut |p| { builder_points.push(p); });

    assert_approx_eq(&iter_points[..], &builder_points[..]);

    assert!(iter_points.len() > 1);
}

#[test]
fn test_issue_194() {
    let segment = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.0, 0.0),
        ctrl2: Point::new(50.0, 70.0),
        to: Point::new(100.0, 100.0),
    };

    let mut points = Vec::new();
    segment.for_each_flattened(0.1, &mut |p| {
        points.push(p);
    });

    assert!(points.len() > 2);
}

#[test]
fn flatten_with_t() {
    let segment = CubicBezierSegment {
        from: Point::new(0.0f32, 0.0),
        ctrl1: Point::new(0.0, 0.0),
        ctrl2: Point::new(50.0, 70.0),
        to: Point::new(100.0, 100.0),
    };

    for tolerance in &[0.1, 0.01, 0.001, 0.0001] {
        let tolerance = *tolerance;

        let mut a = Vec::new();
        segment.for_each_flattened(tolerance, &mut|p| { a.push(p); });

        let mut b = Vec::new();
        let mut ts = Vec::new();
        segment.for_each_flattened_with_t(tolerance, &mut|p, t| {
            b.push(p);
            ts.push(t);
        });

        assert_eq!(a, b);

        for i in 0..b.len() {
            let sampled = segment.sample(ts[i]);
            let point = b[i];
            let dist = (sampled - point).length();
            assert!(dist <= tolerance);
        }
    }

}
