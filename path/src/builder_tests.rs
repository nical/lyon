#[cfg(test)]
mod tests {
    use crate::path::Path;
    use crate::events::PathEvent;
    use crate::builder::{Build, PathBuilder};
    use crate::geom::math::*;

    #[test]
    fn test_flatten() {
        fn test1(builder: &mut impl PathBuilder) {
            builder.move_to(point(9.8589325,53.186916));
            builder.cubic_bezier_to(point(10.3262615,56.03796), point(8.514468,58.483364), point(7.0338364,60.40962));
            builder.cubic_bezier_to(point(5.5532045,62.335873), point(6.1438327,61.547035), point(3.9364057,60.891937));
        }

        let tolerance = 0.01;
        let mut builder = Path::builder().flattened(tolerance);
        let mut builder2 = Path::builder();

        test1(&mut builder);
        test1(&mut builder2);

        let flat_path = builder.build();
        let normal_path = builder2.build();

        let mut max_deviation: f32 = 0.0;
        for event in flat_path.iter() {
            match event {
                PathEvent::Line { from, to } => {
                    // Test deviation from the middle of the line.
                    // This point (and every other point on the line) should be
                    // at most #tolerance units from the original curve
                    let dist = distance_path_point(&normal_path, (from+to.to_vector())*0.5);
                    max_deviation = max_deviation.max(dist);
                }
                _ => {}
            }
        }

        assert!(max_deviation <= tolerance, "Deviation from orignal curve is larger than the allowed tolerance. {} > {}", max_deviation, tolerance);
    }

    // Approximate distance from a path to a point
    fn distance_path_point(path: &Path, point: Point) -> f32 {
        let mut smallest_dist = std::f32::INFINITY;
        for event in path.iter() {
            match event {
                PathEvent::Cubic { from, ctrl1, ctrl2, to } => {
                    let dist = sqr_distance_bezier_point(from, ctrl1, ctrl2, to, point);
                    smallest_dist = smallest_dist.min(dist.0);
                }
                PathEvent::Quadratic { .. } => { panic!() },
                PathEvent::Line { .. } => { panic!() },
                PathEvent::Begin { .. } => {},
                PathEvent::End { .. } => {},
            }
        }
        smallest_dist.sqrt()
    }

    /// Evaluates a cubic bezier at a time T
    pub fn evalute_cubic_bezier(
        p0: Point,
        p1: Point,
        p2: Point,
        p3: Point,
        t: f32,
    ) -> Point {
        let p0 = p0.to_untyped().to_vector();
        let p1 = p1.to_untyped().to_vector();
        let p2 = p2.to_untyped().to_vector();
        let p3 = p3.to_untyped().to_vector();
        let t1 = 1.0 - t;
        let t2 = t1 * t1;
        let t3 = t1 * t1 * t1;
        (p0 * t3 + p1 * (3.0 * t2 * t) + p2 * (3.0 * t1 * t * t) + p3 * (t * t * t))
            .to_point()
            .cast_unit()
    }

    /// Approximate smallest squared distance from a bezier curve to a point
    pub fn sqr_distance_bezier_point(
        p0: Point,
        p1: Point,
        p2: Point,
        p3: Point,
        p: Point,
    ) -> (f32, Point) {
        let mut closest = point(0.0, 0.0);
        let mut closest_dist = std::f32::INFINITY;

        // Sample a bunch of points along the curve and check the distance
        for i in 0..10000 {
            let t = i as f32 / 10000.0;
            let bezier_point = evalute_cubic_bezier(p0, p1, p2, p3, t);
            let dist = (bezier_point - p).square_length();
            if dist < closest_dist {
                closest_dist = dist;
                closest = bezier_point;
            }
        }
        (closest_dist, closest)
    }
}