extern crate lyon;
#[macro_use]
extern crate bencher;

use lyon::extra::rust_logo::build_logo_path;
use lyon::path_builder::*;
use lyon::tessellation::geometry_builder::{ simple_builder, VertexBuffers };
use lyon::tessellation::{ FillEvents, FillTessellator, FillOptions, FillVertex };
use lyon::path::Path;
use lyon::path_iterator::PathIterator;

use bencher::Bencher;

const N: usize = 100;

fn logo_tess_only(bench: &mut Bencher) {
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

fn logo_tess_no_intersection(bench: &mut Bencher) {
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

fn logo_tess_no_curve(bench: &mut Bencher) {
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

fn logo_events_and_tess(bench: &mut Bencher) {
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

fn logo_events_only(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        for _ in 0..N {
            let _events = FillEvents::from_iterator(path.path_iter().flattened(0.05));
        }
    })
}

fn logo_events_only_pre_flattened(bench: &mut Bencher) {
    let mut path = Path::builder().flattened(0.05).with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        for _ in 0..N {
            let _events = FillEvents::from_iterator(path.path_iter().flattened(0.05));
        }
    })
}

fn logo_flattening(bench: &mut Bencher) {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    bench.iter(|| {
        for _ in 0..N {
            for _ in path.path_iter().flattened(0.05) {}
        }
    })
}

benchmark_group!(tess,
  logo_tess_only,
  logo_tess_no_curve,
  logo_tess_no_intersection
);

benchmark_group!(events,
  logo_events_and_tess,
  logo_events_only,
  logo_events_only_pre_flattened
);

benchmark_group!(flattening,
  logo_flattening
);


benchmark_main!(tess, events, flattening);
