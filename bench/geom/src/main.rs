extern crate lyon;
#[macro_use]
extern crate bencher;

use bencher::Bencher;
use lyon::geom::euclid::default::Rotation2D;
use lyon::geom::euclid::point2 as point;
use lyon::geom::CubicBezierSegment;

const N: usize = 1000;

fn cubic_intersections(bench: &mut Bencher) {
    bench.iter(|| {
        let mut sum = 0.0;
        let mut r: f64 = 0.0;
        for _ in 0..N {
            r += 0.01;
            let curve1 = CubicBezierSegment {
                from: point(-100.0, -100.0),
                ctrl1: point(100.0, -100.0),
                ctrl2: point(-100.0, 100.0),
                to: point(-100.0, 100.0),
            }
            .transformed(&Rotation2D::radians(r));

            let curve2 = CubicBezierSegment {
                from: point(-100.0, -100.0),
                ctrl1: point(100.0, -100.0),
                ctrl2: point(-100.0, 100.0),
                to: point(-100.0, 100.0),
            }
            .transformed(&Rotation2D::radians(1.6));

            let intersections = curve1.cubic_intersections_t(&curve2);

            for a in &intersections {
                sum += a.0;
            }
        }

        bencher::black_box(sum);
    });
}

benchmark_group!(cubic, cubic_intersections);

benchmark_main!(cubic);
