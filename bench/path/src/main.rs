extern crate lyon;
#[macro_use]
extern crate bencher;

use lyon::path::{Path, PathEvent};
use lyon::path::new_path;
use lyon::path::id_path;
use lyon::path::new_path::Event;

use lyon::math::point;

use bencher::Bencher;

#[cfg(feature = "profiling")]
const N: usize = 100;
#[cfg(not(feature = "profiling"))]
const N: usize = 1;

fn old_path_build(bench: &mut Bencher) {
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

fn new_path_build(bench: &mut Bencher) {
    bench.iter(|| {
        let mut path = lyon::path::new_path::Path::builder();
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


fn old_path_build_prealloc(bench: &mut Bencher) {
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

fn new_path_build_prealloc(bench: &mut Bencher) {
    bench.iter(|| {
        let n_endpoints = 30010;
        let n_ctrl_points = 30000;
        let n_edges = N * 30_000 + N * 20;
        let mut path = lyon::path::new_path::Builder::with_capacity(n_endpoints, n_ctrl_points, n_edges);
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

        let _path = path.build();
    });
}

fn newer_path_build_prealloc(bench: &mut Bencher) {
    use lyon::math::Point;
    bench.iter(|| {
        let n_endpoints = 30010;
        let n_ctrl_points = 30000;
        let n_edges = N * 30_000 + N * 20;

        let mut path = id_path::PathBuilder::<Point, Point>::with_capacity(n_endpoints, n_ctrl_points, n_edges);

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

fn newer_id_path_build(bench: &mut Bencher) {
    use lyon::path::new_path::{EndpointId, CtrlPointId};

    bench.iter(|| {
        let mut path = id_path::PathCommandsBuilder::new();
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

fn newer_path_build(bench: &mut Bencher) {
    use lyon::math::Point;
    bench.iter(|| {
        let mut path = id_path::PathBuilder::<Point, Point>::new();
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



fn old_path_iter(bench: &mut Bencher) {
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
                PathEvent::MoveTo(to) => {
                    to
                }
                PathEvent::Line(segment) => {
                    segment.to
                }
                PathEvent::Quadratic(segment) => {
                    segment.to
                }
                PathEvent::Cubic(segment) => {
                    segment.to
                }
                PathEvent::Close(segment) => {
                    segment.to
                }
            }.to_vector();
        }
    });
}

fn new_path_iter(bench: &mut Bencher) {
    let mut path = lyon::path::new_path::Path::builder();
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
            match evt {
                Event::Begin { at, .. } => {
                    p += at.to_vector();
                }
                Event::Line { to, ..} => {
                    p += to.to_vector();
                }
                Event::Quadratic { to, .. } => {
                    p += to.to_vector();
                }
                Event::Cubic { to, .. } => {
                    p += to.to_vector();
                }
                Event::End { first, .. } => {
                    p += first.to_vector();
                }
            }
        }
    });
}

fn new_path_id_iter(bench: &mut Bencher) {
    let mut path = lyon::path::new_path::Path::builder();
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
                Event::Begin { at, .. } => {
                    at.to_usize()
                }
                Event::Line { to, .. } => {
                    to.to_usize()
                }
                Event::Quadratic { to, .. } => {
                    to.to_usize()
                }
                Event::Cubic { to, .. } => {
                    to.to_usize()
                }
                Event::End { first, .. } => {
                    first.to_usize()
                }
            };
        }
    });
}

fn newer_path_id_iter(bench: &mut Bencher) {
    use lyon::path::new_path::{EndpointId, CtrlPointId};
    let mut path = id_path::PathCommands::builder();
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
        for evt in path.iter_all() {
            i += match evt {
                id_path::Event::Begin { at, .. } => {
                    at.to_usize()
                }
                id_path::Event::Line { to, .. } => {
                    to.to_usize()
                }
                id_path::Event::Quadratic { to, .. } => {
                    to.to_usize()
                }
                id_path::Event::Cubic { to, .. } => {
                    to.to_usize()
                }
                id_path::Event::End { last, .. } => {
                    last.to_usize()
                }
            };
        }
    });
}

fn newer_path_iter(bench: &mut Bencher) {
    use lyon::math::Point;
    let mut path = id_path::PathBuilder::<Point, Point>::new();
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
            match evt {
                id_path::Event::Begin { at, .. } => {
                    p += at.to_vector();
                }
                id_path::Event::Line { to, ..} => {
                    p += to.to_vector();
                }
                id_path::Event::Quadratic { to, .. } => {
                    p += to.to_vector();
                }
                id_path::Event::Cubic { to, .. } => {
                    p += to.to_vector();
                }
                id_path::Event::End { last, .. } => {
                    p += last.to_vector();
                }
            }
        }
    });
}

benchmark_group!(builder,
    old_path_build,
    old_path_build_prealloc,
    new_path_build,
    new_path_build_prealloc,
    newer_path_build,
    newer_id_path_build,
    newer_path_build_prealloc,
);

benchmark_group!(iter,
    old_path_iter,
    new_path_id_iter,
    new_path_iter,
    newer_path_id_iter,
    newer_path_iter,
);

#[cfg(not(feature = "libtess2"))]
benchmark_main!(builder, iter);


