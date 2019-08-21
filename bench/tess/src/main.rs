extern crate lyon;
#[macro_use]
extern crate bencher;
#[cfg(feature = "libtess2")]
extern crate tess2_sys as tess2;

use lyon::path::Path;
use lyon::path::builder::*;
use lyon::path::iterator::PathIterator;
use lyon::extra::rust_logo::build_logo_path;
use lyon::tessellation::geometry_builder::{simple_builder, VertexBuffers};
use lyon::tessellation::{FillEvents, FillTessellator, FillOptions, FillVertex, LineJoin};
use lyon::tessellation::{StrokeTessellator, StrokeOptions, StrokeVertex};

#[cfg(feature = "experimental")]
use lyon::tessellation::experimental;

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
                builder.flat_event(evt);
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
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = FillTessellator::new();
    let options = FillOptions::default();
    let events = FillEvents::from_path(0.05, path.iter());

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<FillVertex, u16> = VertexBuffers::with_capacity(512, 1450);
            tess.tessellate_events(&events, &options, &mut simple_builder(&mut buffers)).unwrap();
        }
    })
}

#[cfg(feature = "experimental")]
fn fill_new_tess_01_logo(bench: &mut Bencher) {
    let mut path = Path::builder().flattened(0.05).with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = experimental::FillTessellator::new();
    let options = FillOptions::default();

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<experimental::Vertex, u16> = VertexBuffers::with_capacity(512, 1450);
            tess.tessellate_path(&path, &options, &mut simple_builder(&mut buffers));
        }
    })
}

fn fill_tess_02_logo_no_normals(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = FillTessellator::new();
    let options = FillOptions::default().with_normals(false);
    let events = FillEvents::from_path(0.05, path.iter());

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<FillVertex, u16> = VertexBuffers::with_capacity(512, 1450);
            tess.tessellate_events(&events, &options, &mut simple_builder(&mut buffers)).unwrap();
        }
    })
}

fn fill_tess_03_logo_no_intersections(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = FillTessellator::new();
    let options = FillOptions::default().assume_no_intersections();
    let events = FillEvents::from_path(0.05, path.iter());

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<FillVertex, u16> = VertexBuffers::new();
            tess.tessellate_events(&events, &options, &mut simple_builder(&mut buffers)).unwrap();
        }
    })
}

fn fill_tess_04_logo_no_normals_no_intersections(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = FillTessellator::new();
    let options = FillOptions::default()
        .with_normals(false)
        .assume_no_intersections();
    let events = FillEvents::from_path(0.05, path.iter());

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<FillVertex, u16> = VertexBuffers::with_capacity(512, 1450);
            tess.tessellate_events(&events, &options, &mut simple_builder(&mut buffers)).unwrap();
        }
    })
}

fn fill_tess_05_logo_no_curve(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = FillTessellator::new();
    let options = FillOptions::default();
    let events = FillEvents::from_path(1000000.0, path.iter());

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<FillVertex, u16> = VertexBuffers::new();
            tess.tessellate_events(&events, &options, &mut simple_builder(&mut buffers)).unwrap();
        }
    })
}

#[cfg(feature = "libtess2")]
fn cmp_01_libtess2_rust_logo(bench: &mut Bencher) {
    use tess2::*;
    use std::slice;
    use std::os::raw::c_void;

    use lyon::path::FlattenedEvent;

    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut contours = Vec::new();

    let tolerance = FillOptions::default().tolerance;
    for evt in path.iter().flattened(tolerance) {
        match evt {
            FlattenedEvent::MoveTo(p) => {
                contours.push(vec![p]);
            }
            FlattenedEvent::LineTo(p) => {
                contours.last_mut().unwrap().push(p);
            }
            FlattenedEvent::Close => {}
        }
    }

    bench.iter(|| {
        unsafe {
            let tess = tessNewTess(0 as *mut TESSalloc);
            for _ in 0..N {
                for contour in &contours {
                    tessAddContour(
                        tess,
                        2,
                        (&contour[0].x as *const f32) as *const c_void,
                        8,
                        contour.len() as i32
                    );
                }
                let res = tessTesselate(tess,
                    TessWindingRule::TESS_WINDING_ODD,
                    TessElementType::TESS_POLYGONS,
                    3,
                    2,
                    0 as *mut TESSreal
                );
                assert!(res == 1);

                let raw_triangle_count = tessGetElementCount(tess);
                let triangle_count = raw_triangle_count as usize;
                assert!(triangle_count > 1);

                let _vertex_buffer = slice::from_raw_parts(tessGetVertices(tess),
                                                          tessGetVertexCount(tess) as usize * 2);
                let _triangle_buffer = slice::from_raw_parts(tessGetElements(tess), triangle_count * 3);
            }
        }
    });
}

#[cfg(feature = "libtess2")]
fn cmp_02_lyon_rust_logo(bench: &mut Bencher) {
    // To get this test case as comparable as possible with the libtess2 one:
    // - The path is built and pre-flattened beforehand.
    // - The tessellator and other allocations are not recycled between runs.
    // - No normals.

    let options = FillOptions::default().with_normals(false);
    let mut path = Path::builder().flattened(options.tolerance).with_svg();
    build_logo_path(&mut path);
    let path = path.build();


    bench.iter(|| {
        for _ in 0..N {
            let mut tess = FillTessellator::new();
            let mut buffers: VertexBuffers<FillVertex, u16> = VertexBuffers::new();
            tess.tessellate_path(&path, &options, &mut simple_builder(&mut buffers)).unwrap();
        }
    })
}

fn fill_events_01_logo(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        for _ in 0..N {
            let _events = FillEvents::from_path(0.05, path.iter());
        }
    })
}

fn fill_events_02_logo_pre_flattened(bench: &mut Bencher) {
    let mut path = Path::builder().flattened(0.05).with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        for _ in 0..N {
            let _events = FillEvents::from_path(0.05, path.iter());
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
            let mut buffers: VertexBuffers<FillVertex, u16> = VertexBuffers::new();
            tess.tessellate_path(&path, &options, &mut simple_builder(&mut buffers)).unwrap();
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
            let mut buffers: VertexBuffers<StrokeVertex, u16> = VertexBuffers::with_capacity(1024, 3000);
            tess.tessellate_path(&path, &options, &mut simple_builder(&mut buffers)).unwrap();
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
            let mut buffers: VertexBuffers<StrokeVertex, u16> = VertexBuffers::with_capacity(1024, 3000);
            tess.tessellate_path(&path, &options, &mut simple_builder(&mut buffers)).unwrap();
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
            let mut buffers: VertexBuffers<StrokeVertex, u16> = VertexBuffers::with_capacity(1024, 3000);
            tess.tessellate_path(&path, &options, &mut simple_builder(&mut buffers)).unwrap();
        }
    })
}

benchmark_group!(stroke_tess,
  stroke_01_logo_miter,
  stroke_02_logo_bevel,
  stroke_03_logo_round
);

benchmark_group!(fill_tess,
  fill_tess_01_logo,
  fill_tess_02_logo_no_normals,
  fill_tess_03_logo_no_intersections,
  fill_tess_04_logo_no_normals_no_intersections,
  fill_tess_05_logo_no_curve
);

#[cfg(feature = "experimental")]
benchmark_group!(new_tess,
    fill_new_tess_01_logo
);

#[cfg(feature = "libtess2")]
benchmark_group!(cmp_tess2,
  cmp_01_libtess2_rust_logo,
  cmp_02_lyon_rust_logo
);

benchmark_group!(fill_events,
  fill_events_01_logo,
  fill_events_02_logo_pre_flattened,
  fill_events_03_logo_with_tess
);

benchmark_group!(flattening,
  flattening_01_logo_simple_iter,
  flattening_02_logo_iter,
  flattening_03_logo_builder
);

#[cfg(all(feature = "libtess2", not(feature = "experimental")))]
benchmark_main!(
    fill_tess,
    cmp_tess2,
    fill_events,
    stroke_tess,
    flattening
);

#[cfg(feature = "experimental")]
benchmark_main!(
    fill_tess,
    new_tess,
    fill_events,
    stroke_tess,
    flattening
);

#[cfg(all(not(feature = "experimental"), not(feature = "libtess2")))]
benchmark_main!(
    fill_tess,
    fill_events,
    stroke_tess,
    flattening
);
