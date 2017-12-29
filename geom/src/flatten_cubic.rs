///! Utilities to flatten cubic bezier curve segments, implmeneted both with callback and
///! iterator based APIs.
///!
///! The algorithm implemented here is based on:
///! http://cis.usouthal.edu/~hain/general/Publications/Bezier/Bezier%20Offset%20Curves.pdf
///! It produces a better approximations than the usual recursive subdivision approach (or
///! in other words, it generates less points for a given tolerance threshold).

use CubicBezierSegment;
use scalar::{Float, FloatExt, ApproxEq};
use generic_math::Point;
use arrayvec::ArrayVec;
use std::mem::swap;

/// An iterator over a cubic bezier segment that yields line segments approximating the
/// curve for a given approximation threshold.
///
/// The iterator starts at the first point *after* the origin of the curve and ends at the
/// destination.
pub struct Flattened<S: Float> {
    remaining_curve: CubicBezierSegment<S>,
    // current portion of the curve, does not have inflections.
    current_curve: Option<CubicBezierSegment<S>>,
    next_inflection: Option<S>,
    following_inflection: Option<S>,
    tolerance: S,
    check_inflection: bool,
}

impl<S: Float> Flattened<S> {
    /// Creates an iterator that yields points along a cubic bezier segment, useful to build a
    /// flattened approximation of the curve given a certain tolerance.
    pub fn new(bezier: CubicBezierSegment<S>, tolerance: S) -> Self {
        let inflections = find_cubic_bezier_inflection_points(&bezier);

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
                let t2 = (t2 - t1) / (S::one() - t1);
                iter.following_inflection = Some(t2)
            }

            return iter;
        }

        iter.current_curve = Some(bezier);

        return iter;
    }
}

impl<S: Float + ApproxEq<S>> Iterator for Flattened<S> {
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
            if t >= S::one() {
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

pub fn flatten_cubic_bezier<S: Float + ApproxEq<S>, F: FnMut(Point<S>)>(
    mut bezier: CubicBezierSegment<S>,
    tolerance: S,
    call_back: &mut F,
) {
    let inflections = find_cubic_bezier_inflection_points(&bezier);

    if let Some(&t1) = inflections.get(0) {
        bezier = flatten_including_inflection(&bezier, t1, tolerance, call_back);
        if let Some(&t2) = inflections.get(1) {
            // Adjust the second inflection since we removed the part before the
            // first inflection from the bezier curve.
            let t2 = (t2 - t1) / (S::one() - t1);
            bezier = flatten_including_inflection(&bezier, t2, tolerance, call_back);
        }
    }

    flatten_cubic_no_inflection(bezier, tolerance, call_back);
}

// Flatten the curve up to the the inflection point and its approximation range included.
fn flatten_including_inflection<S: Float + ApproxEq<S>, F: FnMut(Point<S>)>(
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
fn flatten_cubic_no_inflection<S: Float, F: FnMut(Point<S>)>(
    mut bezier: CubicBezierSegment<S>,
    tolerance: S,
    call_back: &mut F,
) {
    let end = bezier.to;

    let mut t = S::zero();
    while t < S::one() {
        t = no_inflection_flattening_step(&bezier, tolerance);

        if t == S::one() {
            break;
        }
        bezier = bezier.after_split(t);
        call_back(bezier.from);
    }

    call_back(end);
}

fn no_inflection_flattening_step<S: Float>(bezier: &CubicBezierSegment<S>, tolerance: S) -> S {
    let v1 = bezier.ctrl1 - bezier.from;
    let v2 = bezier.ctrl2 - bezier.from;

    // This function assumes that the bézier segment is not starting at an inflection point,
    // otherwise the following cross product may result in very small numbers which will hit
    // floating point precision issues.

    // To remove divisions and check for divide-by-zero, this is optimized from:
    // s2 = (v2.x * v1.y - v2.y * v1.x) / hypot(v1.x, v1.y);
    // t = 2 * sqrt(tolerance / (3. * abs(s2)));
    let v2_cross_v1 = v2.cross(v1);
    if v2_cross_v1 == S::zero() {
        return S::one();
    }
    let s2inv = v1.x.hypot(v1.y) / v2_cross_v1;

    let t = S::c(2.0) * (tolerance * s2inv.abs() / S::c(3.0)).sqrt();

    // TODO: We start having floating point precision issues if this constant
    // is closer to 1.0 with a small enough tolerance threshold.
    if t >= S::c(0.995) || t == S::zero() {
        return S::one();
    }

    return t;
}

// Find the inflection points of a cubic bezier curve.
pub fn find_cubic_bezier_inflection_points<S: Float>(bezier: &CubicBezierSegment<S>) -> ArrayVec<[S; 2]> {
    // Find inflection points.
    // See www.faculty.idc.ac.il/arik/quality/appendixa.html for an explanation
    // of this approach.
    let pa = bezier.ctrl1 - bezier.from;
    let pb = bezier.ctrl2.to_vector() - (bezier.ctrl1.to_vector() * S::c(2.0)) + bezier.from.to_vector();
    let pc = bezier.to.to_vector() - (bezier.ctrl2.to_vector() * S::c(3.0)) + (bezier.ctrl1.to_vector() * S::c(3.0)) - bezier.from.to_vector();

    let a = pb.cross(pc);
    let b = pa.cross(pc);
    let c = pa.cross(pb);

    let mut ret = ArrayVec::new();

    if a.abs() < S::c(1e-5) {
        // Not a quadratic equation.
        if b.abs() < S::c(1e-5) {
            // Instead of a linear acceleration change we have a constant
            // acceleration change. This means the equation has no solution
            // and there are no inflection points, unless the constant is 0.
            // In that case the curve is a straight line, essentially that means
            // the easiest way to deal with is is by saying there's an inflection
            // point at t == 0. The inflection point approximation range found will
            // automatically extend into infinity.
            if c.abs() < S::c(1e-5) {
                ret.push(S::zero());
            }
        } else {
            let t = -c / b;
            if in_range(t) {
                ret.push(t);
            }
        }

        return ret;
    }

    fn in_range<S: Float>(t: S) -> bool { t >= S::zero() && t < S::one() }

    let discriminant = b * b - S::c(4.0) * a * c;

    if discriminant < S::zero() {
        return ret;
    }

    if discriminant.abs() < S::c(1e-5) {
        let t = -b / (S::c(2.0) * a);

        if in_range(t) {
            ret.push(t);
        }

        return ret;
    }

    let discriminant_sqrt = discriminant.sqrt();
    let q = if b < S::zero() { b - discriminant_sqrt } else { b + discriminant_sqrt } * S::c(-0.5);

    let mut first_inflection = q / a;
    let mut second_inflection = c / q;
    if first_inflection > second_inflection {
        swap(&mut first_inflection, &mut second_inflection);
    }

    if in_range(first_inflection) {
        ret.push(first_inflection);
    }

    if in_range(second_inflection) {
        ret.push(second_inflection);
    }

    ret
}

// Find the range around the start of the curve where the curve can locally be approximated
// with a line segment, given a tolerance threshold.
fn inflection_approximation_range<S: Float + ApproxEq<S>>(
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
    if p1.x.abs() < S::c(1e-5) && p1.y.abs() < S::c(1e-5) {
        // Assume p1 = 0.
        if p2.x.abs() < S::c(1e-5) && p2.y.abs() < S::c(1e-5) {
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
    let tf = (tolerance / s3).abs().powf(S::c(1.0 / 3.0));

    return if tf < S::one() { Some(tf) } else { None };
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
        if (a[i].x - b[i].x).abs() > 0.0000001 || (a[i].y - b[i].y).abs() > 0.0000001 {
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
    c1.flattened_for_each(tolerance, &mut |p| { builder_points.push(p); });

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
    c1.flattened_for_each(tolerance, &mut |p| { builder_points.push(p); });

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
    c1.flattened_for_each(tolerance, &mut |p| { builder_points.push(p); });

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
    c1.flattened_for_each(tolerance, &mut |p| { builder_points.push(p); });

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
    segment.flattened_for_each(0.1, &mut |p| {
        points.push(p);
    });

    assert!(points.len() > 2);
}
