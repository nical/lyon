
pub trait XMonotoneParametricCurve {
    fn x(&self, t: f32) -> f32;
    fn dx(&self, t: f32) -> f32;
}

pub fn solve_t_for_x(curve: &XMonotoneParametricCurve, x: f32, tolerance: f32) -> f32 {
    let from = curve.x(0.0);
    let to = curve.x(1.0);
    if x <= from {
        return 0.0;
    }
    if x >= to {
        return 1.0;
    }

    // Newton's method.
    let mut t = x - from / (to - from);
    for _ in 0..8 {
        let x2 = curve.x(t);

        if (x2 - x).abs() <= tolerance {
            return t
        }

        let dx = curve.dx(t);

        if dx <= 1e-5 {
            break
        }

        t -= (x2 - x) / dx;
    }

    // Fall back to binary search.
    let mut min = 0.0;
    let mut max = 1.0;
    let mut t = 0.5;

    while min < max {
        let x2 = curve.x(t);

        if (x2 - x).abs() < tolerance {
            return t;
        }

        if x > x2 {
            min = t;
        } else {
            max = t;
        }

        t = (max - min) * 0.5 + min;
    }

    return t;
}
