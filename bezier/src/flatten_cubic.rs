///! Utilities to flatten cubic bezier curve segments, implmeneted both with callback and
///! iterator based APIs.
///!
///! The algorithm implemented here is based on:
///! http://cis.usouthal.edu/~hain/general/Publications/Bezier/Bezier%20Offset%20Curves.pdf
///! It produces a better approximations than the usual recursive subdivision approach (or
///! in other words, it generates less points for a given tolerance threshold).

use super::{ Point, CubicBezierSegment };

use std::f32;
use std::mem::swap;

/// An iterator that expresses the linearization of a cubic bezier segment for given a tolerance
/// threshold.
/// The iterator starts at the first point *after* the origin of the curve and ends at the
/// destination.
pub struct CubicFlatteningIter {
    curve: CubicBezierSegment,
    sub_curve: Option<CubicBezierSegment>,
    next_inflection_start: f32,
    next_inflection_end: f32,
    following_inflection_start: f32,
    following_inflection_end: f32,
    num_inflecions: i32,
    tolerance: f32,
}

fn clamp(a: f32, min: f32, max: f32) -> f32 {
    if a < min { return min; }
    if a > max { return max; }
    return a;
}

impl CubicFlatteningIter {
    /// Creates an iterator that yields points along a cubic bezier segment, useful to build a
    /// flattened approximation of the curve given a certain tolerance.
    pub fn new(bezier: CubicBezierSegment, tolerance: f32) -> Self {
        let (first_inflection, second_inflection) = find_cubic_bezier_inflection_points(&bezier);
        let num_inflections = if first_inflection.is_none() { 0 } else if second_inflection.is_none() { 1 } else { 2 };
        let first_inflection = first_inflection.unwrap_or(-1.0);
        let second_inflection = second_inflection.unwrap_or(-1.0);

        let mut iter = CubicFlatteningIter {
            curve: bezier,
            sub_curve: None,
            next_inflection_start: f32::NAN,
            next_inflection_end: f32::NAN,
            following_inflection_start: f32::NAN,
            following_inflection_end: f32::NAN,
            num_inflecions: num_inflections,
            tolerance: tolerance
        };

        if iter.num_inflecions == 0 {
            iter.sub_curve = Some(bezier);
            return iter;
        }

        // Calulate the range where the inflection can be linearly approximated
        find_cubic_bezier_inflection_approximation_range(
            &bezier, first_inflection, tolerance,
            &mut iter.next_inflection_start,
            &mut iter.next_inflection_end
        );

        iter.next_inflection_start = clamp(iter.next_inflection_start, 0.0, 1.0);
        iter.next_inflection_end = clamp(iter.next_inflection_end, 0.0, 1.0);

        if iter.num_inflecions == 2 {
            find_cubic_bezier_inflection_approximation_range(
                &bezier, second_inflection, tolerance,
                &mut iter.following_inflection_start,
                &mut iter.following_inflection_end
            );
            iter.following_inflection_start = clamp(iter.following_inflection_start, 0.0, 1.0);
            iter.following_inflection_end = clamp(iter.following_inflection_end, 0.0, 1.0);
        }

        if iter.next_inflection_start > 0.0 {
            iter.sub_curve = Some(bezier.before_split(iter.next_inflection_start));
        }

        return iter;
    }
}

impl Iterator for CubicFlatteningIter {
    type Item = Point;
    fn next(&mut self) -> Option<Point> {
        if let Some(sub_curve) = self.sub_curve {
            // We are iterating over a sub-curve that does not have inflections.
            let t = no_inflection_flattening_step(&sub_curve, self.tolerance);
            if t >= 1.0 {
                let to = sub_curve.to;
                self.sub_curve = None;
                return Some(to);
            } else {
                let next_curve = sub_curve.after_split(t);
                self.sub_curve = Some(next_curve);
                return Some(next_curve.from);
            }
        }

        if self.num_inflecions > 0 {
            // We are at the beginning of an inflection range which is approximated with a line
            // segment.
            let current_range_end = self.next_inflection_end;

            // Pop the inflection range.
            self.num_inflecions -= 1;
            self.next_inflection_start = self.following_inflection_start;
            self.next_inflection_end = self.following_inflection_end;

            // If the range doesn't extend all the way to the end of the curve, prepare the next
            // sub-curve portion that we are going to iterator over.
            if current_range_end < 1.0 {
                let mut next_curve = self.curve.after_split(current_range_end);
                if self.num_inflecions > 0 && current_range_end < self.next_inflection_start {
                    next_curve = next_curve.before_split(self.next_inflection_start);
                }
                self.sub_curve = Some(next_curve);
            }

            return Some(self.curve.sample(current_range_end));
        }

        return None;
    }
}

pub fn flatten_cubic_bezier<F: FnMut(Point)>(
    bezier: CubicBezierSegment,
    tolerance: f32,
    call_back: &mut F
) {
    let (first_inflection, second_inflection) = find_cubic_bezier_inflection_points(&bezier);
    let num_inflections = if first_inflection.is_none() { 0 } else if second_inflection.is_none() { 1 } else { 2 };
    let first_inflection = if let Some(t) = first_inflection { t } else { -1.0 };
    let second_inflection = if let Some(t) = second_inflection { t } else { -1.0 };

    if num_inflections == 0 {
        flatten_cubic_no_inflection(bezier, tolerance, call_back);
        return;
    }

    let mut first_inflection_start = first_inflection;
    let mut first_inflection_end = first_inflection;
    let mut second_inflection_start = second_inflection;
    let mut second_inflection_end = second_inflection;

    // For both inflection points, calulate the range where they can be linearly approximated if
    // they are positioned within [0, 1]
    if num_inflections > 0 {
        find_cubic_bezier_inflection_approximation_range(
            &bezier, first_inflection, tolerance,
            &mut first_inflection_start,
            &mut first_inflection_end
        );
    }

    if num_inflections == 2 {
        find_cubic_bezier_inflection_approximation_range(
            &bezier, second_inflection, tolerance,
            &mut second_inflection_start,
            &mut second_inflection_end
        );
    }

    // Process ranges. [first_inflection_start, first_inflection_end] and
    // [second_inflection_start, second_inflection_end] are approximated by line segments.
    if num_inflections == 1 && first_inflection_start <= 0.0 && first_inflection_end >= 1.0 {
        // The whole range can be approximated by a line segment.
        call_back(bezier.to);
        return;
    }

    if first_inflection_start > 0.0 {
        // Flatten the Bezier up until the first inflection point's approximation
        // point.
        flatten_cubic_no_inflection(
            bezier.before_split(first_inflection_start),
            tolerance,
            call_back
        );
    }

    if first_inflection_end >= 0.0 && first_inflection_end < 1.0
    && (num_inflections == 1 || second_inflection_start > first_inflection_end) {
        // The second inflection point's approximation range begins after the end
        // of the first, approximate the first inflection point by a line and
        // subsequently flatten up until the end or the next inflection point.
        let next_bezier = bezier.after_split(first_inflection_end);

        call_back(next_bezier.from);

        if num_inflections == 1 || (num_inflections > 1 && second_inflection_start >= 1.0) {
            // No more inflection points to deal with, flatten the rest of the curve.
            flatten_cubic_no_inflection(next_bezier, tolerance, call_back);
            return;
        }
    } else if num_inflections > 1 && second_inflection_start > 1.0 {
        // We've already concluded second_inflection_start <= first_inflection_end, so if this is
        // true the approximation range for the first inflection point runs past the end of the
        // curve, draw a line to the end and we're done.
        call_back(bezier.to);
        return;
    }

    if num_inflections > 1 && second_inflection_start < 1.0 && second_inflection_end > 0.0 {
        if second_inflection_start > 0.0 && second_inflection_start < first_inflection_end {
            // In this case the second_inflection approximation range starts inside the
            // first inflection's approximation range.
            call_back(bezier.sample(first_inflection_end));
        } else if second_inflection_start > 0.0 && first_inflection_end > 0.0 {
            let next_bezier = bezier.after_split(first_inflection_end);

            // Find a control points describing the portion of the curve between
            // first_inflection_end and second_inflection_start.
            let second_inflection_starta = (second_inflection_start - first_inflection_end) / (1.0 - first_inflection_end);
            flatten_cubic_no_inflection(
                next_bezier.before_split(second_inflection_starta),
                tolerance,
                call_back
            );
        } else if second_inflection_start > 0.0 {
            // We have nothing interesting before second_inflection_start, find that bit and
            // flatten it.
            flatten_cubic_no_inflection(
                bezier.before_split(second_inflection_start),
                tolerance,
                call_back
            );
        }

        if second_inflection_end < 1.0 {
            // Flatten the portion of the curve after second_inflection_end
            let next_bezier = bezier.after_split(second_inflection_end);

            // Draw a line to the start, this is the approximation between second_inflection_start and second_inflection_end.
            call_back(next_bezier.from);
            flatten_cubic_no_inflection(next_bezier, tolerance, call_back);
        } else {
            // Our approximation range extends beyond the end of the curve.
            call_back(bezier.to);
        }
    }
}

// The algorithm implemented here is based on:
// http://cis.usouthal.edu/~hain/general/Publications/Bezier/Bezier%20Offset%20Curves.pdf
//
// The basic premise is that for a small t the third order term in the
// equation of a cubic bezier curve is insignificantly small. This can
// then be approximated by a quadratic equation for which the maximum
// difference from a linear approximation can be much more easily determined.
fn flatten_cubic_no_inflection<F: FnMut(Point)>(
    mut bezier: CubicBezierSegment,
    tolerance: f32,
    call_back: &mut F
) {
    let end = bezier.to;

    let mut t = 0.0;
    while t < 1.0 {
        t = no_inflection_flattening_step(&bezier, tolerance);

        if t == 1.0 {
            break
        }
        bezier = bezier.after_split(t);
        call_back(bezier.from);
    }

    call_back(end);
}

fn no_inflection_flattening_step(
    bezier: &CubicBezierSegment,
    tolerance: f32,
) -> f32 {
    let v1 = bezier.ctrl1 - bezier.from;
    let v2 = bezier.ctrl2 - bezier.from;

    // To remove divisions and check for divide-by-zero, this is optimized from:
    // Float s2 = (v2.x * v1.y - v2.y * v1.x) / hypot(v1.x, v1.y);
    // t = 2 * Float(sqrt(tolerance / (3. * abs(s2))));
    let v1_cross_v2 = v2.x * v1.y - v2.y * v1.x;
    let h = v1.x.hypot(v1.y);
    if v1_cross_v2 * h == 0.0 {
        return 1.0;
    }
    let s2inv = h / v1_cross_v2;

    let t = 2.0 * (tolerance * s2inv.abs() / 3.0).sqrt();

    if t >= 0.9999 {
        return 1.0;
    }

    return t;
}

// Find the inflection points of a cubic bezier curve.
fn find_cubic_bezier_inflection_points(
    bezier: &CubicBezierSegment,
) -> (Option<f32>, Option<f32>) {
    // Find inflection points.
    // See www.faculty.idc.ac.il/arik/quality/appendixa.html for an explanation
    // of this approach.
    let pa = bezier.ctrl1 - bezier.from;
    let pb = bezier.ctrl2 - (bezier.ctrl1 * 2.0) + bezier.from;
    let pc = bezier.to - (bezier.ctrl2 * 3.0) + (bezier.ctrl1 * 3.0) - bezier.from;

    let a = pb.x * pc.y - pb.y * pc.x;
    let b = pa.x * pc.y - pa.y * pc.x;
    let c = pa.x * pb.y - pa.y * pb.x;

    if a == 0.0 {
        // Not a quadratic equation.
        if b == 0.0 {
            // Instead of a linear acceleration change we have a constant
            // acceleration change. This means the equation has no solution
            // and there are no inflection points, unless the constant is 0.
            // In that case the curve is a straight line, essentially that means
            // the easiest way to deal with is is by saying there's an inflection
            // point at t == 0. The inflection point approximation range found will
            // automatically extend into infinity.
            if c == 0.0 {
               return (Some(0.0), None);
            }
            return (None, None);
        }
        return (Some(-c / b), None);
    }

    fn in_range(t: f32) -> Option<f32> { if t >= 0.0 && t < 1.0 { Some(t) } else { None } }

    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return (None, None);
    }

    if discriminant == 0.0 {
        return (in_range(-b / (2.0 * a)), None);
    }

    let discriminant_sqrt = discriminant.sqrt();
    let q = if b < 0.0 { b - discriminant_sqrt } else { b + discriminant_sqrt } * -0.5;

    let mut first_inflection = q / a;
    let mut second_inflection = c / q;
    if first_inflection > second_inflection {
        swap(&mut first_inflection, &mut second_inflection);
    }

    let first_inflection = in_range(first_inflection);
    let second_inflection = in_range(second_inflection);

    if first_inflection.is_none() {
        return (second_inflection, None);
    }

    return (first_inflection, second_inflection);
}

fn cubic_root(val: f32) -> f32 {
    if val < 0.0 {
        return -cubic_root(-val);
    }

    return val.powf(1.0 / 3.0);
}

// Find the range around a point where the curve can be approximated with a line segment, given
// a tolerance threshold.
fn find_cubic_bezier_inflection_approximation_range(
    bezier_segment: &CubicBezierSegment,
    t: f32, tolerance: f32,
    min: &mut f32, max: &mut f32
) {
    let bezier = bezier_segment.after_split(t);

    let ctrl21 = bezier.ctrl1 - bezier.from;
    let ctrl41 = bezier.to - bezier.from;

    if ctrl21.x == 0.0 && ctrl21.y == 0.0 {
        // In this case s3 becomes lim[n->0] (ctrl41.x * n) / n - (ctrl41.y * n) / n = ctrl41.x - ctrl41.y.

        // Use the absolute value so that Min and Max will correspond with the
        // minimum and maximum of the range.
        *min = t - cubic_root((tolerance / (ctrl41.x - ctrl41.y)).abs());
        *max = t + cubic_root((tolerance / (ctrl41.x - ctrl41.y)).abs());
        return;
    }

    let s3 = (ctrl41.x * ctrl21.y - ctrl41.y * ctrl21.x) / ctrl21.x.hypot(ctrl21.y);

    if s3 == 0.0 {
        // This means within the precision we have it can be approximated
        // infinitely by a linear segment. Deal with this by specifying the
        // approximation range as extending beyond the entire curve.
        *min = -1.0;
        *max = 2.0;
        return;
    }

    let tf = cubic_root((tolerance / s3).abs());

    *min = t - tf * (1.0 - t);
    *max = t + tf * (1.0 - t);
}

#[cfg(test)]
fn print_arrays(a: &[Point], b: &[Point]) {
    println!("left:  {:?}", a);
    println!("right: {:?}", b);
}

#[cfg(test)]
fn assert_approx_eq(a: &[Point], b: &[Point]) {
    if a.len() != b.len() {
        print_arrays(a, b);
        panic!("Lenths differ ({} != {})", a.len(), b.len());
    }
    for i in 0..a.len() {
        if (a[i].x - b[i].x).abs() > 0.0000001 ||
           (a[i].y - b[i].y).abs() > 0.0000001 {
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
    let iter_points: Vec<Point> = c1.flattening_iter(tolerance).collect();
    let mut builder_points = Vec::new();
    c1.flattened_for_each(tolerance, &mut|p|{ builder_points.push(p); });

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
    let iter_points: Vec<Point> = c1.flattening_iter(tolerance).collect();
    let mut builder_points = Vec::new();
    c1.flattened_for_each(tolerance, &mut|p|{ builder_points.push(p); });

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
    let iter_points: Vec<Point> = c1.flattening_iter(tolerance).collect();
    let mut builder_points = Vec::new();
    c1.flattened_for_each(tolerance, &mut|p|{ builder_points.push(p); });

    assert!(iter_points.len() > 2);
    assert_approx_eq(&iter_points[..], &builder_points[..]);
}