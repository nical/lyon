extern crate lyon;
#[macro_use]
extern crate bencher;
#[cfg(feature = "libtess2")]
extern crate tess2_sys as tess2;

use lyon::extra::rust_logo::build_logo_path;
use lyon::math::Point;
use lyon::path::builder::*;
use lyon::path::iterator::PathIterator;
use lyon::path::Path;
use lyon::tessellation::geometry_builder::{simple_builder, VertexBuffers};
use lyon::tessellation::{EventQueue, FillTessellator};
use lyon::tessellation::{FillOptions, LineJoin};
use lyon::tessellation::{StrokeOptions, StrokeTessellator};

use bencher::Bencher;

#[cfg(feature = "profiling")]
const N: usize = 100;
#[cfg(not(feature = "profiling"))]
const N: usize = 1;

fn flattening_01_logo_simple_iter(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        for _ in 0..N {
            for _ in path.iter().flattened(0.05) {}
        }
    })
}

// This benchmark is a bit convoluted in order to be comparable to
// flattening_03_logo_builder below.
fn flattening_02_logo_iter(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        let mut builder = Path::builder();
        for _ in 0..N {
            for evt in path.iter().flattened(0.05) {
                builder.path_event(evt);
            }
        }
    })
}

fn flattening_03_logo_builder(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        let mut builder = Path::builder().flattened(0.05);
        for _ in 0..N {
            for evt in path.iter() {
                builder.path_event(evt);
            }
        }
    })
}

fn fill_tess_01_logo(bench: &mut Bencher) {
    let mut path = Path::builder().flattened(0.05).with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = FillTessellator::new();
    let options = FillOptions::default();

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::with_capacity(512, 1450);
            tess.tessellate(&path, &options, &mut simple_builder(&mut buffers))
                .unwrap();
        }
    })
}

fn fill_tess_06_logo_with_ids(bench: &mut Bencher) {
    let mut path = Path::builder().flattened(0.05).with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = FillTessellator::new();
    let options = FillOptions::default();

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::with_capacity(512, 1450);
            tess.tessellate_with_ids(
                path.id_iter(),
                &path,
                None,
                &options,
                &mut simple_builder(&mut buffers),
            )
            .unwrap();
        }
    })
}

fn fill_tess_03_logo_no_intersections(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = FillTessellator::new();
    let options = FillOptions::default()
        .with_tolerance(0.05)
        .with_intersections(false);

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
            tess.tessellate_path(
                &path,
                &options,
                &mut simple_builder(&mut buffers),
            )
            .unwrap();
        }
    })
}

fn fill_tess_05_logo_no_curve(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = FillTessellator::new();
    let options = FillOptions::default().with_tolerance(1000000.0);

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
            tess.tessellate_path(
                &path,
                &options,
                &mut simple_builder(&mut buffers),
            )
            .unwrap();
        }
    })
}

#[cfg(feature = "libtess2")]
fn cmp_01_libtess2_rust_logo(bench: &mut Bencher) {
    use lyon::path::PathEvent;
    use std::os::raw::c_void;
    use std::slice;
    use tess2::*;

    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut contours = Vec::new();

    let tolerance = FillOptions::default().tolerance;
    for evt in path.iter().flattened(tolerance) {
        match evt {
            PathEvent::Begin { at } => {
                contours.push(vec![at]);
            }
            PathEvent::Line { to, .. } => {
                contours.last_mut().unwrap().push(to);
            }
            _ => {}
        }
    }

    bench.iter(|| unsafe {
        let tess = tessNewTess(0 as *mut TESSalloc);
        for _ in 0..N {
            for contour in &contours {
                tessAddContour(
                    tess,
                    2,
                    (&contour[0].x as *const f32) as *const c_void,
                    8,
                    contour.len() as i32,
                );
            }
            let res = tessTesselate(
                tess,
                TessWindingRule::TESS_WINDING_ODD,
                TessElementType::TESS_POLYGONS,
                3,
                2,
                0 as *mut TESSreal,
            );
            assert!(res == 1);

            let raw_triangle_count = tessGetElementCount(tess);
            let triangle_count = raw_triangle_count as usize;
            assert!(triangle_count > 1);

            let _vertex_buffer =
                slice::from_raw_parts(tessGetVertices(tess), tessGetVertexCount(tess) as usize * 2);
            let _triangle_buffer = slice::from_raw_parts(tessGetElements(tess), triangle_count * 3);
        }
    });
}

#[cfg(feature = "libtess2")]
fn cmp_02_lyon_rust_logo(bench: &mut Bencher) {
    // To get this test case as comparable as possible with the libtess2 one:
    // - The path is built and pre-flattened beforehand.
    // - The tessellator and other allocations are not recycled between runs.
    // - No normals.

    let options = FillOptions::default();
    let mut path = Path::builder().flattened(options.tolerance).with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        for _ in 0..N {
            let mut tess = FillTessellator::new();
            let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
            tess.tessellate(&path, &options, &mut simple_builder(&mut buffers))
                .unwrap();
        }
    })
}

fn fill_events_01_logo(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        for _ in 0..N {
            let _events = EventQueue::from_path(0.05, path.iter());
        }
    })
}

fn fill_events_02_logo_pre_flattened(bench: &mut Bencher) {
    let mut path = Path::builder().flattened(0.05).with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        for _ in 0..N {
            let _events = EventQueue::from_path(0.05, path.iter());
        }
    })
}

fn fill_events_03_logo_with_tess(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = FillTessellator::new();
    let options = FillOptions::tolerance(0.05);

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
            tess.tessellate(&path, &options, &mut simple_builder(&mut buffers))
                .unwrap();
        }
    })
}

fn stroke_01_logo_miter(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = StrokeTessellator::new();
    let options = StrokeOptions::default();

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::with_capacity(1024, 3000);
            tess.tessellate(&path, &options, &mut simple_builder(&mut buffers))
                .unwrap();
        }
    })
}

fn stroke_02_logo_bevel(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = StrokeTessellator::new();
    let options = StrokeOptions::default().with_line_join(LineJoin::Bevel);

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::with_capacity(1024, 3000);
            tess.tessellate(&path, &options, &mut simple_builder(&mut buffers))
                .unwrap();
        }
    })
}

fn stroke_03_logo_round(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = StrokeTessellator::new();
    let options = StrokeOptions::default().with_line_join(LineJoin::Round);

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::with_capacity(1024, 3000);
            tess.tessellate(&path, &options, &mut simple_builder(&mut buffers))
                .unwrap();
        }
    })
}

benchmark_group!(
    stroke_tess,
    stroke_01_logo_miter,
    stroke_02_logo_bevel,
    stroke_03_logo_round
);

benchmark_group!(
    fill_tess,
    fill_tess_01_logo,
    fill_tess_06_logo_with_ids,
    fill_tess_03_logo_no_intersections,
    fill_tess_05_logo_no_curve
);

#[cfg(feature = "libtess2")]
benchmark_group!(cmp_tess2, cmp_01_libtess2_rust_logo, cmp_02_lyon_rust_logo);

benchmark_group!(
    fill_events,
    fill_events_01_logo,
    fill_events_02_logo_pre_flattened,
    fill_events_03_logo_with_tess
);

benchmark_group!(
    flattening,
    flattening_01_logo_simple_iter,
    flattening_02_logo_iter,
    flattening_03_logo_builder
);

#[cfg(feature = "libtess2")]
benchmark_main!(fill_tess, cmp_tess2, fill_events, stroke_tess, flattening);

#[cfg(not(feature = "libtess2"))]
benchmark_main!(fill_tess, fill_events, stroke_tess, flattening);
