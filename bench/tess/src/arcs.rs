//! Same-input comparisons with reused tessellator and output allocations.

use bencher::Bencher;
use lyon::math::{point, Point};
use lyon::path::Path;
use lyon::tessellation::geometry_builder::{simple_builder, VertexBuffers};
use lyon::tessellation::{ArcsClip, LineJoin, StrokeOptions, StrokeTessellator};

fn arcs_curved(bench: &mut Bencher) {
    run(bench, false, LineJoin::Arcs, 4.0, ArcsClip::Butt);
}

fn arcs_clipped(bench: &mut Bencher) {
    run(bench, false, LineJoin::Arcs, 1.5, ArcsClip::Butt);
}

fn arcs_round_clip(bench: &mut Bencher) {
    run(bench, false, LineJoin::Arcs, 1.5, ArcsClip::Round);
}

fn arcs_lines(bench: &mut Bencher) {
    run(bench, true, LineJoin::Arcs, 4.0, ArcsClip::Butt);
}

fn round_curved(bench: &mut Bencher) {
    run(bench, false, LineJoin::Round, 4.0, ArcsClip::Butt);
}

fn miter_clip_lines(bench: &mut Bencher) {
    run(bench, true, LineJoin::MiterClip, 4.0, ArcsClip::Butt);
}

fn run(bench: &mut Bencher, lines: bool, join: LineJoin, limit: f32, clip: ArcsClip) {
    let path = path(lines);
    let options = StrokeOptions::default()
        .with_line_join(join)
        .with_arcs_clip(clip)
        .with_line_width(8.0)
        .with_miter_limit(limit)
        .with_tolerance(0.1);
    let mut tessellator = StrokeTessellator::new();
    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
    bench.iter(|| {
        buffers.clear();
        tessellator
            .tessellate_path(&path, &options, &mut simple_builder(&mut buffers))
            .unwrap();
        buffers.indices.len()
    });
}

fn path(lines: bool) -> Path {
    let mut builder = Path::builder();
    for i in 0..64 {
        let x = i as f32 * 40.0;
        builder.begin(point(x, 0.0));
        if lines {
            builder.line_to(point(x + 20.0, 0.0));
            builder.line_to(point(x + 5.0, 10.0));
        } else {
            builder.cubic_bezier_to(
                point(x + 12.0, -12.0),
                point(x + 25.0, -8.0),
                point(x + 20.0, 0.0),
            );
            builder.cubic_bezier_to(
                point(x + 32.0, -4.0),
                point(x + 36.0, 15.0),
                point(x + 5.0, 10.0),
            );
        }
        builder.end(false);
    }
    builder.build()
}

benchmark_group!(
    arcs_tess,
    arcs_curved,
    arcs_round_clip,
    arcs_clipped,
    arcs_lines,
    round_curved,
    miter_clip_lines
);
