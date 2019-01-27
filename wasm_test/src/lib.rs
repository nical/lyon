extern crate lyon;

use lyon::tessellation::{FillVertex, FillTessellator, FillOptions};
use lyon::tessellation::geometry_builder::{VertexBuffers, simple_builder};
use lyon::path::builder::*;
use lyon::path::Path;
use lyon::extra::rust_logo::build_logo_path;

#[no_mangle]
pub extern fn run_tests() {
    test_logo();
}

fn test_logo() {
    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut buffers: VertexBuffers<FillVertex, u16> = VertexBuffers::new();
    let mut tess = FillTessellator::new();

    tess.tessellate_path(
        path.path_iter(),
        &FillOptions::tolerance(0.05),
        &mut simple_builder(&mut buffers)
    ).unwrap();
}
