extern crate lyon;
#[macro_use]
extern crate bencher;

use lyon::path::Path;
use lyon::path::builder::*;
use lyon::path::iterator::PathIterator;
use lyon::extra::rust_logo::build_logo_path;
use lyon::tessellation::geometry_builder::{simple_builder, VertexBuffers};
use lyon::tessellation::{FillEvents, FillTessellator, FillOptions, FillVertex, LineJoin};
use lyon::tessellation::{StrokeTessellator, StrokeOptions, StrokeVertex};

use bencher::Bencher;

const N: usize = 10;

fn logo_simple_flattening_iter(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        for _ in 0..N {
            for _ in path.path_iter().flattened(0.05) {}
        }
    })
}

// This benchmark is a bit convoluted in order to be comparable to
// logo_flattening_builder below.
fn logo_flattening_iter(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        let mut builder = Path::builder();
        for _ in 0..N {
            for evt in path.path_iter().flattened(0.05) {
                builder.flat_event(evt);
            }
        }
    })
}

fn logo_flattening_builder(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        let mut builder = Path::builder().flattened(0.05);
        for _ in 0..N {
            for evt in path.path_iter() {
                builder.path_event(evt);
            }
        }
    })
}

fn fill_logo_tess_only(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = FillTessellator::new();
    let options = FillOptions::default();
    let events = FillEvents::from_path(0.05, path.path_iter());

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<FillVertex> = VertexBuffers::with_capacity(512, 1450);
            tess.tessellate_events(&events, &options, &mut simple_builder(&mut buffers)).unwrap();
        }
    })
}

fn fill_logo_tess_no_intersection(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = FillTessellator::new();
    let options = FillOptions::default().assume_no_intersections();
    let events = FillEvents::from_path(0.05, path.path_iter());

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<FillVertex> = VertexBuffers::new();
            tess.tessellate_events(&events, &options, &mut simple_builder(&mut buffers)).unwrap();
        }
    })
}

fn fill_logo_tess_no_curve(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = FillTessellator::new();
    let options = FillOptions::default();
    let events = FillEvents::from_path(1000000.0, path.path_iter());

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<FillVertex> = VertexBuffers::new();
            tess.tessellate_events(&events, &options, &mut simple_builder(&mut buffers)).unwrap();
        }
    })
}

fn fill_logo_events_and_tess(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = FillTessellator::new();
    let options = FillOptions::tolerance(0.05);

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<FillVertex> = VertexBuffers::new();
            tess.tessellate_path(path.path_iter(), &options, &mut simple_builder(&mut buffers)).unwrap();
        }
    })
}

fn fill_logo_events_only(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        for _ in 0..N {
            let _events = FillEvents::from_path(0.05, path.path_iter());
        }
    })
}

fn fill_logo_events_only_pre_flattened(bench: &mut Bencher) {
    let mut path = Path::builder().flattened(0.05).with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        for _ in 0..N {
            let _events = FillEvents::from_path(0.05, path.path_iter());
        }
    })
}

fn stroke_logo_miter(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = StrokeTessellator::new();
    let options = StrokeOptions::default();

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<StrokeVertex> = VertexBuffers::with_capacity(1024, 3000);
            tess.tessellate_path(path.path_iter(), &options, &mut simple_builder(&mut buffers));
        }
    })
}

fn stroke_logo_bevel(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = StrokeTessellator::new();
    let options = StrokeOptions::default().with_line_join(LineJoin::Bevel);

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<StrokeVertex> = VertexBuffers::with_capacity(1024, 3000);
            tess.tessellate_path(path.path_iter(), &options, &mut simple_builder(&mut buffers));
        }
    })
}

fn stroke_logo_round(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = StrokeTessellator::new();
    let options = StrokeOptions::default().with_line_join(LineJoin::Round);

    bench.iter(|| {
        for _ in 0..N {
            let mut buffers: VertexBuffers<StrokeVertex> = VertexBuffers::with_capacity(1024, 3000);
            tess.tessellate_path(path.path_iter(), &options, &mut simple_builder(&mut buffers));
        }
    })
}

benchmark_group!(stroke_tess,
  stroke_logo_miter,
  stroke_logo_bevel,
  stroke_logo_round
);

benchmark_group!(fill_tess,
  fill_logo_tess_only,
  fill_logo_tess_no_curve,
  fill_logo_tess_no_intersection
);

benchmark_group!(fill_events,
  fill_logo_events_and_tess,
  fill_logo_events_only,
  fill_logo_events_only_pre_flattened
);

benchmark_group!(flattening,
  logo_simple_flattening_iter,
  logo_flattening_iter,
  logo_flattening_builder
);

benchmark_main!(fill_tess, fill_events, stroke_tess, flattening);
