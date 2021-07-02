use crate::cubic_to_quadratic::single_curve_approximation;
use crate::quadratic_bezier::FlattenedT as FlattenedQuadraticSegment;
use crate::scalar::Scalar;
use crate::CubicBezierSegment;
///! Utilities to flatten cubic bézier curve segments, implemented both with callback and
///! iterator based APIs.
use crate::Point;

// Computes the number of quadratic bézier segments to approximate a cubic one.
// Derived by Raph Levien from section 10.6 of Sedeberg's CAGD notes
// https://scholarsarchive.byu.edu/cgi/viewcontent.cgi?article=1000&context=facpub#section.10.6
// and the error metric from the caffein owl blog post http://caffeineowl.com/graphics/2d/vectorial/cubic2quad01.html
fn num_quadratics<S: Scalar>(curve: &CubicBezierSegment<S>, tolerance: S) -> S {
    debug_assert!(tolerance > S::ZERO);

    let x = curve.from.x - S::THREE * curve.ctrl1.x + S::THREE * curve.ctrl2.x - curve.to.x;
    let y = curve.from.y - S::THREE * curve.ctrl1.y + S::THREE * curve.ctrl2.y - curve.to.y;

    let err = x * x + y * y;

    (err / (S::value(432.0) * tolerance * tolerance))
        .powf(S::ONE / S::SIX)
        .ceil()
        .max(S::ONE)
}

pub fn flatten_cubic_bezier_with_t<S: Scalar, F>(
    curve: &CubicBezierSegment<S>,
    tolerance: S,
    callback: &mut F,
) where
    F: FnMut(Point<S>, S),
{
    debug_assert!(tolerance >= S::EPSILON * S::EPSILON);
    let quadratics_tolerance = tolerance * S::value(0.2);
    let flattening_tolerance = tolerance * S::value(0.8);

    let num_quadratics = num_quadratics(&curve, quadratics_tolerance);
    let step = S::ONE / num_quadratics;
    let n = num_quadratics.to_u32().unwrap();
    let mut t0 = S::ZERO;
    for _ in 0..(n - 1) {
        let t1 = t0 + step;

        let quadratic = single_curve_approximation(&curve.split_range(t0..t1));
        quadratic.for_each_flattened_with_t(flattening_tolerance, &mut |point, t_sub| {
            let t = t0 + step * t_sub;
            callback(point, t);
        });

        t0 = t1;
    }

    // Do the last step manually to make sure we finish at t = 1.0 exactly.
    let quadratic = single_curve_approximation(&curve.split_range(t0..S::ONE));
    quadratic.for_each_flattened_with_t(flattening_tolerance, &mut |point, t_sub| {
        let t = t0 + step * t_sub;
        callback(point, t);
    });
}

pub struct Flattened<S: Scalar> {
    curve: CubicBezierSegment<S>,
    current_curve: FlattenedQuadraticSegment<S>,
    remaining_sub_curves: i32,
    tolerance: S,
    range_step: S,
    range_start: S,
}

impl<S: Scalar> Flattened<S> {
    pub(crate) fn new(curve: &CubicBezierSegment<S>, tolerance: S) -> Self {
        debug_assert!(tolerance >= S::EPSILON * S::EPSILON);

        let quadratics_tolerance = tolerance * S::value(0.2);
        let flattening_tolerance = tolerance * S::value(0.8);

        let num_quadratics = num_quadratics(&curve, quadratics_tolerance);

        let range_step = S::ONE / num_quadratics;

        let quadratic = single_curve_approximation(&curve.split_range(S::ZERO..range_step));
        let current_curve = FlattenedQuadraticSegment::new(&quadratic, flattening_tolerance);

        Flattened {
            curve: *curve,
            current_curve,
            remaining_sub_curves: num_quadratics.to_i32().unwrap() - 1,
            tolerance: flattening_tolerance,
            range_start: S::ZERO,
            range_step,
        }
    }
}

impl<S: Scalar> Iterator for Flattened<S> {
    type Item = Point<S>;

    fn next(&mut self) -> Option<Point<S>> {
        if let Some(t_inner) = self.current_curve.next() {
            let t = self.range_start + t_inner * self.range_step;
            return Some(self.curve.sample(t));
        }

        if self.remaining_sub_curves <= 0 {
            return None;
        }

        self.range_start += self.range_step;
        let t0 = self.range_start;
        let t1 = self.range_start + self.range_step;
        self.remaining_sub_curves -= 1;

        let quadratic = single_curve_approximation(&self.curve.split_range(t0..t1));
        self.current_curve = FlattenedQuadraticSegment::new(&quadratic, self.tolerance);

        let t_inner = self.current_curve.next().unwrap_or(S::ONE);
        let t = t0 + t_inner * self.range_step;

        Some(self.curve.sample(t))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            self.remaining_sub_curves as usize * self.current_curve.size_hint().0,
            None,
        )
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

#[test]
fn test_flatten_end() {
    let segment = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(100.0, 0.0),
        ctrl2: Point::new(100.0, 100.0),
        to: Point::new(100.0, 200.0),
    };

    let mut last = segment.from;
    segment.for_each_flattened(0.0001, &mut |p| {
        last = p;
    });

    assert_eq!(last, segment.to);
}

#[test]
fn test_flatten_point() {
    let segment = CubicBezierSegment {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.0, 0.0),
        ctrl2: Point::new(0.0, 0.0),
        to: Point::new(0.0, 0.0),
    };

    let mut last = segment.from;
    segment.for_each_flattened(0.0001, &mut |p| {
        last = p;
    });

    assert_eq!(last, segment.to);
}

#[test]
fn issue_652() {
    use crate::point;

    let curve = CubicBezierSegment {
        from: point(-1061.0, -3327.0),
        ctrl1: point(-1061.0, -3177.0),
        ctrl2: point(-1061.0, -3477.0),
        to: point(-1061.0, -3327.0),
    };

    for _ in curve.flattened(1.0) {}
    for _ in curve.flattened(0.1) {}
    for _ in curve.flattened(0.01) {}

    curve.for_each_flattened(1.0, &mut |_| {});
    curve.for_each_flattened(0.1, &mut |_| {});
    curve.for_each_flattened(0.01, &mut |_| {});
}
