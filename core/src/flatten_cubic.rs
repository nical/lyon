use math::*;
use bezier::{ CubicBezierSegment };
use path_builder::PrimitiveBuilder;

use std::mem::swap;

// Find the inflection points of a cubic bezier curve.
fn find_cubic_bezier_inflection_points(
    bezier: &CubicBezierSegment,
) -> (Option<f32>, Option<f32>) {
    // Find inflection points.
    // See www.faculty.idc.ac.il/arik/quality/appendixa.html for an explanation
    // of this approach.
    let pa = bezier.cp1 - bezier.from;
    let pb = bezier.cp2 - (bezier.cp1 * 2.0) + bezier.from;
    let pc = bezier.to - (bezier.cp2 * 3.0) + (bezier.cp1 * 3.0) - bezier.from;

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

    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return (None, None);
    }

    if discriminant == 0.0 {
        return (Some(-b / (2.0 * a)), None);
    }

    let discriminant_sqrt = discriminant.sqrt();
    let q = if b < 0.0 { b - discriminant_sqrt } else { b + discriminant_sqrt } * -0.5;

    let mut t1 = q / a;
    let mut t2 = c / q;
    if t1 > t2 {
        swap(&mut t1, &mut t2);
    }

    return (Some(t1), Some(t2));
}

fn cubic_root(val: f32) -> f32 {
    if val < 0.0 {
        return -cubic_root(-val);
    }

    return val.powf(1.0 / 3.0);
}

fn find_cubic_bezier_inflection_approximation_range(
    bezier_segment: &CubicBezierSegment,
    t: f32, tolerance: f32,
    min: &mut f32, max: &mut f32
) {
    let bezier = bezier_segment.after_split(t);

    let cp21 = bezier.cp1 - bezier.from;
    let cp41 = bezier.to - bezier.from;

    if cp21.x == 0.0 && cp21.y == 0.0 {
      // In this case s3 becomes lim[n->0] (cp41.x * n) / n - (cp41.y * n) / n = cp41.x - cp41.y.

      // Use the absolute value so that Min and Max will correspond with the
      // minimum and maximum of the range.
      *min = t - cubic_root((tolerance / (cp41.x - cp41.y)).abs());
      *max = t + cubic_root((tolerance / (cp41.x - cp41.y)).abs());
      return;
    }

    let s3 = (cp41.x * cp21.y - cp41.y * cp21.x) / cp21.x.hypot(cp21.y);

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

pub fn flatten_cubic_bezier<Builder: PrimitiveBuilder>(
    bezier: CubicBezierSegment,
    tolerance: f32,
    path: &mut Builder
) {
    let (t1, t2) = find_cubic_bezier_inflection_points(&bezier);
    let count = if t1.is_none() { 0 } else if t2.is_none() { 1 } else { 2 };
    let t1 = if let Some(t) = t1 { t } else { -1.0 };
    let t2 = if let Some(t) = t2 { t } else { -1.0 };

    // Check that at least one of the inflection points is inside [0..1]
    if count == 0 || ((t1 <= 0.0 || t1 >= 1.0) && (count == 1 || (t2 <= 0.0 || t2 >= 1.0))) {
        return flatten_cubic_no_inflection(bezier, tolerance, path);
    }

    let mut t1min = t1;
    let mut t1max = t1;
    let mut t2min = t2;
    let mut t2max = t2;

    // For both inflection points, calulate the range where they can be linearly
    // approximated if they are positioned within [0,1]
    if count > 0 && t1 >= 0.0 && t1 < 1.0 {
        find_cubic_bezier_inflection_approximation_range(&bezier, t1, tolerance, &mut t1min, &mut t1max);
    }
    if count > 1 && t2 >= 0.0 && t2 < 1.0 {
        find_cubic_bezier_inflection_approximation_range(&bezier, t2, tolerance, &mut t2min, &mut t2max);
    }

    // Process ranges. [t1min, t1max] and [t2min, t2max] are approximated by line
    // segments.
    if count == 1 && t1min <= 0.0 && t1max >= 1.0 {
        // The whole range can be approximated by a line segment.
        path.line_to(bezier.to);
        return;
    }

    if t1min > 0.0 {
        // Flatten the Bezier up until the first inflection point's approximation
        // point.
        flatten_cubic_no_inflection(bezier.before_split(t1min), tolerance, path);
    }
    if t1max >= 0.0 && t1max < 1.0 && (count == 1 || t2min > t1max) {
        // The second inflection point's approximation range begins after the end
        // of the first, approximate the first inflection point by a line and
        // subsequently flatten up until the end or the next inflection point.
        let next_bezier = bezier.after_split(t1max);

        path.line_to(next_bezier.from);

        if count == 1 || (count > 1 && t2min >= 1.0) {
            // No more inflection points to deal with, flatten the rest of the curve.
            flatten_cubic_no_inflection(next_bezier, tolerance, path);
            return;
        }
    } else if count > 1 && t2min > 1.0 {
        // We've already concluded t2min <= t1max, so if this is true the
        // approximation range for the first inflection point runs past the
        // end of the curve, draw a line to the end and we're done.
        path.line_to(bezier.to);
        return;
    }

    if count > 1 && t2min < 1.0 && t2max > 0.0 {
        if t2min > 0.0 && t2min < t1max {
            // In this case the t2 approximation range starts inside the t1
            // approximation range.
            path.line_to(bezier.sample(t1max));
        } else if t2min > 0.0 && t1max > 0.0 {
            let next_bezier = bezier.after_split(t1max);

            // Find a control points describing the portion of the curve between t1max and t2min.
            let t2mina = (t2min - t1max) / (1.0 - t1max);
            flatten_cubic_no_inflection(next_bezier.before_split(t2mina), tolerance, path);
        } else if t2min > 0.0 {
            // We have nothing interesting before t2min, find that bit and flatten it.
            flatten_cubic_no_inflection(bezier.before_split(t2min), tolerance, path);
        }

        if t2max < 1.0 {
            // Flatten the portion of the curve after t2max
            let next_bezier = bezier.after_split(t2max);

            // Draw a line to the start, this is the approximation between t2min and t2max.
            path.line_to(next_bezier.from);
            flatten_cubic_no_inflection(next_bezier, tolerance, path);
        } else {
            // Our approximation range extends beyond the end of the curve.
            path.line_to(bezier.to);
        }
    }
}

fn flatten_cubic_no_inflection<Builder: PrimitiveBuilder>(
    mut bezier: CubicBezierSegment,
    tolerance: f32,
    path: &mut Builder
) {
    let end = bezier.to;

    // The algorithm implemented here is based on:
    // http://cis.usouthal.edu/~hain/general/Publications/Bezier/Bezier%20Offset%20Curves.pdf
    //
    // The basic premise is that for a small t the third order term in the
    // equation of a cubic bezier curve is insignificantly small. This can
    // then be approximated by a quadratic equation for which the maximum
    // difference from a linear approximation can be much more easily determined.
    let mut t = 0.0;
    while t < 1.0 {
        let v1 = bezier.cp1 - bezier.from;
        let v2 = bezier.cp2 - bezier.from;

        // To remove divisions and check for divide-by-zero, this is optimized from:
        // Float s2 = (v2.x * v1.y - v2.y * v1.x) / hypot(v1.x, v1.y);
        // t = 2 * Float(sqrt(tolerance / (3. * abs(s2))));
        let v1_cross_v2 = v2.x * v1.y - v2.y * v1.x;
        let h = v1.x.hypot(v1.y);
        if v1_cross_v2 * h == 0.0 {
            break;
        }
        let s2inv = h / v1_cross_v2;

        t = 2.0 * (tolerance * s2inv.abs() / 3.0).sqrt();

        if t >= 0.9999 {
            break;
        }

        bezier = bezier.after_split(t);

        path.line_to(bezier.from);
    }

    path.line_to(end);
}

#[derive(Copy, Clone, Debug)]
pub enum CubicFlattenIter {
    NoInflection(CubicBezierSegment),
    BeforeInflection(CubicBezierSegment, CubicBezierSegment, (f32, f32), Option<(f32, f32)>),
    AtInflection(CubicBezierSegment, (f32, f32), Option<(f32, f32)>),
    LineTo(Point),
    Done,
}

impl CubicFlattenIter {
    pub fn new(bezier: CubicBezierSegment, tolerance: f32) -> Self {
        let (t1, t2) = find_cubic_bezier_inflection_points(&bezier);
        let count = if t1.is_none() { 0 } else if t2.is_none() { 1 } else { 2 };
        let t1 = if let Some(t) = t1 { t } else { -1.0 };
        let t2 = if let Some(t) = t2 { t } else { -1.0 };

        // Check that if none of the inflection points are inside [0..1]
        if count == 0 || ((t1 <= 0.0 || t1 >= 1.0) && (count == 1 || (t2 <= 0.0 || t2 >= 1.0))) {
            return CubicFlattenIter::NoInflection(bezier);
        }

        let mut t1min = t1;
        let mut t1max = t1;
        // For both inflection points, calulate the range where they can be linearly
        // approximated if they are positioned within [0,1]
        if count > 0 {
            assert!(t1 >= 0.0 && t1 < 1.0);
            find_cubic_bezier_inflection_approximation_range(&bezier, t1, tolerance, &mut t1min, &mut t1max);
        }

        if count == 1 && t1min <= 0.0 && t1max >= 1.0 {
            // The whole range can be approximated by a line segment.
            return CubicFlattenIter::LineTo(bezier.to);
        }

        let first_range = (t1min, t1max);

        let second_range = if count == 2 {
            let mut t2min = t2;
            let mut t2max = t2;
            assert!(t2 >= 0.0 && t2 < 1.0);
            find_cubic_bezier_inflection_approximation_range(&bezier, t2, tolerance, &mut t2min, &mut t2max);
            Some((t2min, t2max))
        } else { None };

        // Starting with the only inflection approximation range.
        if count == 1 && t1min <= 0.0 {
            return CubicFlattenIter::AtInflection(bezier, first_range, second_range);
        }

        return CubicFlattenIter::BeforeInflection(
            bezier,
            bezier.before_split(t1min),
            first_range, second_range
        );
    }
}

impl Iterator for CubicFlattenIter {
    type Item = Point;
    fn next(&mut self) -> Option<Point> {
        match self {
            &mut CubicFlattenIter::LineTo(to) => {
                *self = CubicFlattenIter::Done;
                return Some(to);
            }
            &mut CubicFlattenIter::NoInflection(bezier) => {
                unimplemented!();
            }
            &mut CubicFlattenIter::AtInflection(bezier_total, first_range, second_range) => {
                unimplemented!();
            }
            &mut CubicFlattenIter::BeforeInflection(bezier_before, bezier_total, first_range, second_range) => {
                unimplemented!();
            }
            &mut CubicFlattenIter::Done => { return None; }
        }
    }
}

#[test]
fn test_cubic_inflection_extremity() {
    use path_builder::flattened_path_builder;

    // This curve has inflection points t1=-0.125 and t2=1.0 which used to fall
    // between the branches of flatten_cubic_bezier and not produce any vertex
    // because of t2 being exactly at the extremity of the curve.
    let mut builder = flattened_path_builder(0.05);
    builder.move_to(vec2(141.0, 135.0));
    builder.cubic_bezier_to(vec2(141.0, 130.0), vec2(140.0, 130.0),vec2(131.0, 130.0));
    builder.close();

    let path = builder.build();
    // check that
    assert!(path.num_vertices() > 2);
}