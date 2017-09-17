extern crate lyon;
#[macro_use]
extern crate bencher;

use lyon::extra::rust_logo::build_logo_path;
use lyon::path_builder::*;
use lyon::tessellation::geometry_builder::{simple_builder, VertexBuffers};
use lyon::tessellation::{FillEvents, FillTessellator, FillOptions, FillVertex};
use lyon::tessellation::{StrokeTessellator, StrokeOptions, StrokeVertex};
use lyon::path::Path;
use lyon::path_iterator::PathIterator;

use bencher::Bencher;

const N: usize = 100;

fn fill_logo_tess_only(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = FillTessellator::new();
    let options = FillOptions::default();
    let events = FillEvents::from_iterator(path.path_iter().flattened(0.05));

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
    let events = FillEvents::from_iterator(path.path_iter().flattened(0.05));

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
    let events = FillEvents::from_iterator(path.path_iter().flattened(1000000.0));

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
    let options = FillOptions::default();

    let mut events = FillEvents::new();

    bench.iter(|| {
        for _ in 0..N {
            events.set_path_iter(path.path_iter().flattened(0.05));
            let mut buffers: VertexBuffers<FillVertex> = VertexBuffers::new();
            tess.tessellate_events(&events, &options, &mut simple_builder(&mut buffers)).unwrap();
        }
    })
}

fn fill_logo_events_only(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        for _ in 0..N {
            let _events = FillEvents::from_iterator(path.path_iter().flattened(0.05));
        }
    })
}

fn fill_logo_events_only_pre_flattened(bench: &mut Bencher) {
    let mut path = Path::builder().flattened(0.05).with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        for _ in 0..N {
            let _events = FillEvents::from_iterator(path.path_iter().flattened(0.05));
        }
    })
}

fn fill_logo_flattening(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        for _ in 0..N {
            for _ in path.path_iter().flattened(0.05) {}
        }
    })
}

fn stroke_logo(bench: &mut Bencher) {
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

benchmark_group!(stroke_tess,
  stroke_logo
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

benchmark_group!(fill_flattening,
  fill_logo_flattening
);

benchmark_main!(fill_tess, fill_events, fill_flattening, stroke_tess);
