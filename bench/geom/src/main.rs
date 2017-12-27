extern crate lyon;
#[macro_use]
extern crate bencher;

use bencher::Bencher;

use lyon::math::*;
use lyon::geom::{QuadraticBezierSegment, first_monotonic_segment_intersecion};

const N: usize = 10;

fn monotonic_intersection(bench: &mut Bencher) {
    // TODO: bench a variety of curves
    let c1 = QuadraticBezierSegment {
        from: point(10.0, 0.0),
        ctrl: point(10.0, 90.0),
        to: point(100.0, 90.0),
    }.assume_monotonic();
    let c2 = QuadraticBezierSegment {
        from: point(0.0, 10.0),
        ctrl: point(90.0, 10.0),
        to: point(90.0, 100.0),
    }.assume_monotonic();

    bench.iter(|| {
        for _ in 0..N {
            first_monotonic_segment_intersecion(
                &c1, 0.0..1.0,
                &c2, 0.0..1.0,
                0.001,
            );
            first_monotonic_segment_intersecion(
                &c1, 0.0..0.5,
                &c2, 0.0..0.5,
                0.001,
            );
            first_monotonic_segment_intersecion(
                &c1, 0.5..1.0,
                &c2, 0.5..1.0,
                0.001,
            );
            first_monotonic_segment_intersecion(
                &c1, 0.3..0.7,
                &c2, 0.3..0.7,
                0.001,
            );
        }
    });
}

benchmark_group!(intersections,
  monotonic_intersection
);

benchmark_main!(
    intersections
);
