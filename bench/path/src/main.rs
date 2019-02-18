extern crate lyon;
#[macro_use]
extern crate bencher;

use lyon::path::{Path, PathEvent, EndpointId, CtrlPointId};
use lyon::path::generic;
use lyon::math::{Point, point};

use bencher::Bencher;

#[cfg(feature = "profiling")]
const N: usize = 100;
#[cfg(not(feature = "profiling"))]
const N: usize = 1;

type GenericPathBuilder = generic::GenericPathBuilder<Point, Point>;

fn simple_path_build_empty(bench: &mut Bencher) {
    bench.iter(|| {
        let mut path = Path::builder();
        for _ in 0..N {
            for _ in 0..10 {
                path.move_to(point(0.0, 0.0));
                for _ in 0..1_000 {
                    path.line_to(point(1.0, 0.0));
                    path.cubic_bezier_to(point(2.0, 0.0), point(2.0, 1.0), point(2.0, 2.0));
                    path.quadratic_bezier_to(point(2.0, 0.0), point(2.0, 1.0));
                }
                path.close();
            }
        }

        let _ = path.build();
    });
}

fn simple_path_build_prealloc(bench: &mut Bencher) {
    bench.iter(|| {
        let n_points = 60010;
        let n_edges = N * 30_000 + N * 20;
        let mut path = lyon::path::Builder::with_capacity(n_points, n_edges);
        for _ in 0..N {
            for _ in 0..10 {
                path.move_to(point(0.0, 0.0));
                for _ in 0..1_000 {
                    path.line_to(point(1.0, 0.0));
                    path.cubic_bezier_to(point(2.0, 0.0), point(2.0, 1.0), point(2.0, 2.0));
                    path.quadratic_bezier_to(point(2.0, 0.0), point(2.0, 1.0));
                }
                path.close();
            }
        }

        let _ = path.build();
    });
}

fn generic_build_prealloc(bench: &mut Bencher) {
    use lyon::math::Point;
    bench.iter(|| {
        let n_endpoints = 30010;
        let n_ctrl_points = 30000;
        let n_edges = N * 30_000 + N * 20;

        let mut path: GenericPathBuilder = generic::GenericPathBuilder::with_capacity(
            n_endpoints,
            n_ctrl_points,
            n_edges,
        );

        for _ in 0..N {
            for _ in 0..10 {
                path.move_to(point(0.0, 0.0));
                for _ in 0..1_000 {
                    path.line_to(point(1.0, 0.0));
                    path.cubic_bezier_to(point(2.0, 0.0), point(2.0, 1.0), point(2.0, 2.0));
                    path.quadratic_bezier_to(point(2.0, 0.0), point(2.0, 1.0));
                }
                path.close();
            }
        }

        let _ = path.build();
    });
}

fn id_only_generic_build_empty(bench: &mut Bencher) {
    bench.iter(|| {
        let mut path = generic::PathCommandsBuilder::new();
        let mut ep = 0;
        let mut cp = 0;
        for _ in 0..N {
            for _ in 0..10 {
                path.move_to(EndpointId(ep));
                ep += 1;
                for _ in 0..1_000 {
                    path.line_to(EndpointId(ep));
                    path.cubic_bezier_to(CtrlPointId(cp), CtrlPointId(cp + 1), EndpointId(ep + 1));
                    path.quadratic_bezier_to(CtrlPointId(cp + 2), EndpointId(ep + 2));
                    cp += 3;
                    ep += 3;
                }
                path.close();
            }
        }

        let _ = path.build();
    });
}

fn generic_build_empty(bench: &mut Bencher) {
    use lyon::math::Point;

    bench.iter(|| {
        let mut path: GenericPathBuilder = generic::GenericPath::builder();
        for _ in 0..N {
            for _ in 0..10 {
                path.move_to(point(0.0, 0.0));
                for _ in 0..1_000 {
                    path.line_to(point(1.0, 0.0));
                    path.cubic_bezier_to(point(2.0, 0.0), point(2.0, 1.0), point(2.0, 2.0));
                    path.quadratic_bezier_to(point(2.0, 0.0), point(2.0, 1.0));
                }
                path.close();
            }
        }

        let _ = path.build();
    });
}

fn simple_path_iter(bench: &mut Bencher) {
    let mut path = Path::builder();
    for _ in 0..N {
        for _ in 0..10 {
            path.move_to(point(0.0, 0.0));
            for _ in 0..1_000 {
                path.line_to(point(1.0, 0.0));
                path.cubic_bezier_to(point(2.0, 0.0), point(2.0, 1.0), point(2.0, 2.0));
                path.quadratic_bezier_to(point(2.0, 0.0), point(2.0, 1.0));
            }
            path.close();
        }
    }

    let path = path.build();

    let mut p = point(0.0, 0.0);
    bench.iter(|| {
        for evt in path.iter() {
            p += match evt {
                PathEvent::Begin { at: p }
                | PathEvent::Line { to: p, .. }
                | PathEvent::Quadratic { to: p, .. }
                | PathEvent::Cubic { to: p, .. }
                | PathEvent::End { last: p, .. }
                => {
                    p.to_vector()
                }
            };
        }
    });
}

fn simple_path_id_iter(bench: &mut Bencher) {
    let mut path = Path::builder();
    for _ in 0..N {
        for _ in 0..10 {
            path.move_to(point(0.0, 0.0));
            for _ in 0..1_000 {
                path.line_to(point(1.0, 0.0));
                path.cubic_bezier_to(point(2.0, 0.0), point(2.0, 1.0), point(2.0, 2.0));
                path.quadratic_bezier_to(point(2.0, 0.0), point(2.0, 1.0));
            }
            path.close();
        }
    }

    let path = path.build();

    let mut i = 0;
    bench.iter(|| {
        for evt in path.id_iter() {
            i += match evt {
                PathEvent::Begin { at: p }
                | PathEvent::Line { to: p, .. }
                | PathEvent::Quadratic { to: p, .. }
                | PathEvent::Cubic { to: p, .. }
                | PathEvent::End { last: p, .. }
                => {
                    p.to_usize()
                }
            };
        }
    });
}

fn generic_id_iter(bench: &mut Bencher) {
    let mut path = generic::PathCommands::builder();
    let mut ep = 0;
    let mut cp = 0;
    for _ in 0..N {
        for _ in 0..10 {
            path.move_to(EndpointId(ep));
            ep += 1;
            for _ in 0..1_000 {
                path.line_to(EndpointId(ep));
                path.cubic_bezier_to(CtrlPointId(cp), CtrlPointId(cp + 1), EndpointId(ep + 1));
                path.quadratic_bezier_to(CtrlPointId(cp + 2), EndpointId(ep + 2));
                cp += 3;
                ep += 3;
            }
            path.close();
        }
    }

    let path = path.build();

    let mut i = 0;
    bench.iter(|| {
        for evt in path.id_events() {
            i += match evt {
                PathEvent::Begin { at: p }
                | PathEvent::Line { to: p, .. }
                | PathEvent::Quadratic { to: p, .. }
                | PathEvent::Cubic { to: p, .. }
                | PathEvent::End { last: p, .. }
                => {
                    p.to_usize()
                }
            };
        }
    });
}

fn generic_iter(bench: &mut Bencher) {
    use lyon::math::Point;

    let path = {
        let mut path: GenericPathBuilder = generic::GenericPath::builder();
        for _ in 0..N {
            for _ in 0..10 {
                path.move_to(point(0.0, 0.0));
                for _ in 0..1_000 {
                    path.line_to(point(1.0, 0.0));
                    path.cubic_bezier_to(point(2.0, 0.0), point(2.0, 1.0), point(2.0, 2.0));
                    path.quadratic_bezier_to(point(2.0, 0.0), point(2.0, 1.0));
                }
                path.close();
            }
        }

        path.build()
    };

    let mut p = point(0.0, 0.0);
    bench.iter(|| {
        for evt in path.events() {
            p += match evt {
                PathEvent::Begin { at: p }
                | PathEvent::Line { to: p, .. }
                | PathEvent::Quadratic { to: p, .. }
                | PathEvent::Cubic { to: p, .. }
                | PathEvent::End { last: p, .. }
                => {
                    p.to_vector()
                }
            };
        }
    });
}

fn generic_with_evt_id_iter(bench: &mut Bencher) {
    use lyon::math::Point;

    let path = {
        let mut path: GenericPathBuilder = generic::GenericPath::builder();
        for _ in 0..N {
            for _ in 0..10 {
                path.move_to(point(0.0, 0.0));
                for _ in 0..1_000 {
                    path.line_to(point(1.0, 0.0));
                    path.cubic_bezier_to(point(2.0, 0.0), point(2.0, 1.0), point(2.0, 2.0));
                    path.quadratic_bezier_to(point(2.0, 0.0), point(2.0, 1.0));
                }
                path.close();
            }
        }

        path.build()
    };

    let mut p = point(0.0, 0.0);
    let mut e = 0;
    bench.iter(|| {
        for (evt, evt_id) in path.events_and_event_ids() {
            let a: u32 = unsafe { std::mem::transmute(evt_id) };
            e += a;
            p += match evt {
                PathEvent::Begin { at: p }
                | PathEvent::Line { to: p, .. }
                | PathEvent::Quadratic { to: p, .. }
                | PathEvent::Cubic { to: p, .. }
                | PathEvent::End { last: p, .. }
                => {
                    p.to_vector()
                }
            };
        }
    });
}

benchmark_group!(builder,
    simple_path_build_empty,
    simple_path_build_prealloc,
    generic_build_empty,
    id_only_generic_build_empty,
    generic_build_prealloc,
);

benchmark_group!(iter,
    simple_path_iter,
    simple_path_id_iter,
    generic_id_iter,
    generic_iter,
    generic_with_evt_id_iter,
);

#[cfg(not(feature = "libtess2"))]
benchmark_main!(builder, iter);


