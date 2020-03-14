///! Utilities to flatten cubic bézier curve segments, implemented both with callback and
///! iterator based APIs.
///!
///! The algorithm implemented here is based on: "Fast, precise flattening of cubic Bézier path and offset curves"
///! http://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.106.5344&rep=rep1&type=pdf
///! It produces a better approximations than the usual recursive subdivision approach (or
///! in other words, it generates less points for a given tolerance threshold).

use crate::generic_math::Point;
use crate::scalar::Scalar;
use crate::CubicBezierSegment;

pub fn square_approximation_error<S: Scalar>(curve: &CubicBezierSegment<S>) -> S {
    // See http://caffeineowl.com/graphics/2d/vectorial/cubic2quad01.html
    S::THREE / S::value(1296.0)
        * ((curve.to - curve.ctrl2 * S::THREE) + (curve.ctrl1 * S::THREE - curve.from))
            .square_length()
}

pub fn flatten_cubic_bezier_with_t<S: Scalar, F>(curve: &CubicBezierSegment<S>, tolerance: S, callback: &mut F)
where
    F: FnMut(Point<S>, S),
{
    let quad_tolerance = tolerance * S::value(0.3);
    let square_quad_tolerance = quad_tolerance * quad_tolerance;
    let mut range = S::ZERO..S::ONE;
    loop {
        let sub_curve = curve.split_range(range.clone());
        let square_err = square_approximation_error(&sub_curve);
        if square_err < square_quad_tolerance {
            let flattening_tolerance = tolerance - square_err.sqrt() * S::HALF;
            let quadratic = crate::cubic_to_quadratic::single_curve_approximation(&sub_curve);

            quadratic.for_each_flattened_with_t(flattening_tolerance, &mut |point, t_sub| {
                let t = range.start + (range.end - range.start) * t_sub;
                callback(point, t);
            });

            if range.end == S::ONE {
                return;
            }
            range.start = range.end;
            range.end = S::ONE;
        } else {
            range.end = (range.start + range.end) * S::HALF;
        }
    }

}

pub struct Flattened<S: Scalar> {
    curve: CubicBezierSegment<S>,
    current_curve: crate::quadratic_bezier::FlattenedT<S>,
    range: std::ops::Range<S>,
    tolerance: S,
}

impl<S: Scalar> Flattened<S> {
    // TODO: pass by ref.
    pub fn new(curve: CubicBezierSegment<S>, tolerance: S) -> Self {
        let quad_tolerance = tolerance * S::value(0.3);
        let square_quad_tolerance = quad_tolerance * quad_tolerance;
        let mut first_range = S::ONE;
        let mut sub_curve = curve;
        loop {
            let square_err = square_approximation_error(&sub_curve);
            if square_err < square_quad_tolerance {
                return Flattened {
                    curve,
                    current_curve: crate::quadratic_bezier::FlattenedT::new(
                        &crate::cubic_to_quadratic::single_curve_approximation(&sub_curve),
                        tolerance - square_err.sqrt() * S::HALF,
                    ),
                    range: S::ZERO..first_range,
                    tolerance,
                };
            }

            first_range = first_range * S::HALF;
            sub_curve = curve.split_range(S::ZERO..first_range);
        }
    }
}

impl<S: Scalar> Iterator for Flattened<S> {
    type Item = Point<S>;

    fn next(&mut self) -> Option<Point<S>> {
        let quad_tolerance = self.tolerance * S::value(0.3);
        let square_quad_tolerance = quad_tolerance * quad_tolerance;

        if let Some(t_inner) = self.current_curve.next() {
            let t = self.range.start + t_inner * (self.range.end - self.range.start);
            return Some(self.curve.sample(t));
        }

        if self.range.end == S::ONE {
            return None;
        }

        self.range.start = self.range.end;
        self.range.end = S::ONE;

        loop {
            let sub_curve = self.curve.split_range(self.range.clone());
            let square_err = square_approximation_error(&sub_curve);
            if square_err < square_quad_tolerance {
                let flattening_tolerance = self.tolerance - square_err.sqrt() * S::HALF;
                let quadratic = crate::cubic_to_quadratic::single_curve_approximation(&sub_curve);
                self.current_curve = crate::quadratic_bezier::FlattenedT::new(
                    &quadratic,
                    flattening_tolerance,
                );

                if let Some(t_inner) = self.current_curve.next() {
                    return Some(quadratic.sample(t_inner));
                }
            } else {
                self.range.end = (self.range.start + self.range.end) * S::HALF;
            }
        }
    }
}

// Find the inflection points of a cubic bezier curve.
pub(crate) fn find_cubic_bezier_inflection_points<S, F>(bezier: &CubicBezierSegment<S>, cb: &mut F)
where
    S: Scalar,
    F: FnMut(S),
{
    // Find inflection points.
    // See www.faculty.idc.ac.il/arik/quality/appendixa.html for an explanation
    // of this approach.
    let pa = bezier.ctrl1 - bezier.from;
    let pb =
        bezier.ctrl2.to_vector() - (bezier.ctrl1.to_vector() * S::TWO) + bezier.from.to_vector();
    let pc = bezier.to.to_vector() - (bezier.ctrl2.to_vector() * S::THREE)
        + (bezier.ctrl1.to_vector() * S::THREE)
        - bezier.from.to_vector();

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

    fn in_range<S: Scalar>(t: S) -> bool {
        t >= S::ZERO && t < S::ONE
    }

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

#[cfg(test)]
fn print_arrays(a: &[Point<f32>], b: &[Point<f32>]) {
    println!("left:  {:?}", a);
    println!("right: {:?}", b);
}

#[cfg(test)]
fn assert_approx_eq(a: &[Point<f32>], b: &[Point<f32>]) {
    if a.len() != b.len() {
        print_arrays(a, b);
        panic!("Lengths differ ({} != {})", a.len(), b.len());
    }
    for i in 0..a.len() {
        let threshold = 0.029;
        let dx = f32::abs(a[i].x - b[i].x);
        let dy = f32::abs(a[i].y - b[i].y);
        if dx > threshold || dy > threshold {
            print_arrays(a, b);
            println!("diff = {:?} {:?}", dx, dy);
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
    c1.for_each_flattened(tolerance, &mut |p| {
        builder_points.push(p);
    });

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
    c1.for_each_flattened(tolerance, &mut |p| {
        builder_points.push(p);
    });

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
    c1.for_each_flattened(tolerance, &mut |p| {
        builder_points.push(p);
    });

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
    c1.for_each_flattened(tolerance, &mut |p| {
        builder_points.push(p);
    });

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
        segment.for_each_flattened(tolerance, &mut |p| {
            a.push(p);
        });

        let mut b = Vec::new();
        let mut ts = Vec::new();
        segment.for_each_flattened_with_t(tolerance, &mut |p, t| {
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
