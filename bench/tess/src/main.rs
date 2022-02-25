extern crate lyon;
#[macro_use]
extern crate bencher;

use lyon::extra::rust_logo::build_logo_path;
use lyon::math::Point;
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
            tess.tessellate_path(&path, &options, &mut simple_builder(&mut buffers))
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
            tess.tessellate_path(&path, &options, &mut simple_builder(&mut buffers))
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

benchmark_main!(fill_tess, fill_events, stroke_tess, flattening);
