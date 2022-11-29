extern crate lyon_tests;
extern crate lyon;
#[macro_use]
extern crate criterion;


use criterion::Criterion;
use lyon::geom::euclid::default::Rotation2D;
use lyon::geom::euclid::point2 as point;
use lyon::geom::{CubicBezierSegment, QuadraticBezierSegment};
use lyon_tests::*;


const N: usize = 1;

fn cubic_intersections(bench: &mut Criterion) {
    bench.bench_function("cubic intersection", |_| {
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

        std::hint::black_box(sum);
    });
}

fn bench_flatten<A: Flatten<f32>>(curves: &[CubicBezierSegment<f32>], tolerance: f32) {
    for _ in 0..N {
        for curve in curves {
            A::flatten(&curve, tolerance, &mut |seg, _| {
                std::hint::black_box(seg);
            });
        }
    }
}

fn bench_flatten_quad<A: Flatten<f32>>(curves: &[QuadraticBezierSegment<f32>], tolerance: f32) {
    for _ in 0..N {
        for curve in curves {
            A::flatten_quad(&curve, tolerance, &mut |seg, _| {
                std::hint::black_box(seg);
            });
        }
    }
}

use criterion::BenchmarkId;

static TOLERANCES: [f32; 10] = [0.01, 0.025, 0.05, 0.075, 0.1, 0.15, 0.2, 0.25, 0.5, 1.0];

fn cubic_flatten(c: &mut Criterion) {
    let curves = generate_bezier_curves();
    let mut g = c.benchmark_group("cubic");
    for tol in &TOLERANCES {
        g.bench_with_input(BenchmarkId::new("recursive", tol), tol, |b, tol| { b.iter(|| bench_flatten::<Recursive>(&curves, *tol)) });
        //g.bench_with_input(BenchmarkId::new("recursive_hfd", tol), tol, |b, tol| { b.iter(|| bench_flatten::<RecursiveHfd>(&curves, *tol)) });
        //g.bench_with_input(BenchmarkId::new("recursive_agg", tol), tol, |b, tol| { b.iter(|| bench_flatten::<RecursiveAgg>(&curves, *tol)) });
        g.bench_with_input(BenchmarkId::new("fwd-iff", tol), tol, |b, tol| { b.iter(|| bench_flatten::<ForwardDifference>(&curves, *tol)) });
        g.bench_with_input(BenchmarkId::new("hfd", tol), tol, |b, tol| { b.iter(|| bench_flatten::<Hfd>(&curves, *tol)) });
        g.bench_with_input(BenchmarkId::new("pa", tol), tol, |b, tol| { b.iter(|| bench_flatten::<Pa>(&curves, *tol)) });
        g.bench_with_input(BenchmarkId::new("levien", tol), tol, |b, tol| { b.iter(|| bench_flatten::<Levien>(&curves, *tol)) });
        g.bench_with_input(BenchmarkId::new("linear", tol), tol, |b, tol| { b.iter(|| bench_flatten::<Linear>(&curves, *tol)) });
        g.bench_with_input(BenchmarkId::new("linear2", tol), tol, |b, tol| { b.iter(|| bench_flatten::<Linear2>(&curves, *tol)) });
        g.bench_with_input(BenchmarkId::new("cagd", tol), tol, |b, tol| { b.iter(|| bench_flatten::<Cagd>(&curves, *tol)) });
    }
}

fn quad_flatten(c: &mut Criterion) {
    let curves = generate_quadratic_curves();
    let mut g = c.benchmark_group("quadratic");
    for tol in &TOLERANCES {
        g.bench_with_input(BenchmarkId::new("recursive", tol), tol, |b, tol| { b.iter(|| bench_flatten_quad::<Recursive>(&curves, *tol)) });
        g.bench_with_input(BenchmarkId::new("fwd-diff", tol), tol, |b, tol| { b.iter(|| bench_flatten_quad::<ForwardDifference>(&curves, *tol)) });
        g.bench_with_input(BenchmarkId::new("levien", tol), tol, |b, tol| { b.iter(|| bench_flatten_quad::<Levien>(&curves, *tol)) });
        g.bench_with_input(BenchmarkId::new("linear2", tol), tol, |b, tol| { b.iter(|| bench_flatten_quad::<Linear2>(&curves, *tol)) });
        g.bench_with_input(BenchmarkId::new("cagd", tol), tol, |b, tol| { b.iter(|| bench_flatten_quad::<Cagd>(&curves, *tol)) });    
    }
}


//criterion_group!(cubic, cubic_intersections);
criterion_group!(flatten, cubic_flatten, quad_flatten );

criterion_main!(flatten);

