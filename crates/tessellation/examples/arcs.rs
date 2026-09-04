//! Generate a mesh-rendered join comparison, without browser arcs support.
//! cargo run -p lyon_tessellation --example arcs -- joins.svg

use lyon_tessellation::path::{math::point, Path};
use lyon_tessellation::{
    ArcsClip, BuffersBuilder, LineJoin, StrokeOptions, StrokeTessellator, StrokeVertex,
    VertexBuffers,
};
use std::fmt::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let destination = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "joins.svg".into());
    let variants = [
        ("Round", LineJoin::Round, ArcsClip::Butt),
        ("Miter clip", LineJoin::MiterClip, ArcsClip::Butt),
        ("SVG 2 arcs", LineJoin::Arcs, ArcsClip::Butt),
        ("SVG 2 + round clip", LineJoin::Arcs, ArcsClip::Round),
    ];
    let mut svg = String::from(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="1440" height="580" viewBox="0 0 1440 580"><rect width="1440" height="580" fill="#16191d"/><g fill="#edf2f7" font-family="sans-serif">"##,
    );
    for (row, (name, path)) in cases().into_iter().enumerate() {
        for (column, (label, join, clip)) in variants.iter().enumerate() {
            let options = StrokeOptions::default()
                .with_line_join(*join)
                .with_arcs_clip(*clip)
                .with_line_width(40.0)
                .with_miter_limit(1.5)
                .with_tolerance(0.05);
            let mut mesh: VertexBuffers<_, u32> = VertexBuffers::new();
            StrokeTessellator::new().tessellate_path(
                &path,
                &options,
                &mut BuffersBuilder::new(&mut mesh, |v: StrokeVertex| v.position()),
            )?;
            let x = column * 360;
            let y = row * 280;
            write!(
                svg,
                r#"<g transform="translate({x} {y})"><text x="18" y="26">{label}: {name}</text>"#
            )?;
            svg.push_str(r##"<path fill="#40bed3" d=""##);
            for triangle in mesh.indices.chunks_exact(3) {
                let [a, b, c] = [triangle[0], triangle[1], triangle[2]]
                    .map(|index| mesh.vertices[index as usize]);
                write!(
                    svg,
                    "M{},{} L{},{} {},{} Z ",
                    a.x, a.y, b.x, b.y, c.x, c.y
                )?;
            }
            svg.push_str("\"/></g>");
        }
    }
    svg.push_str("</g></svg>");
    std::fs::write(&destination, svg)?;
    println!("{destination}");
    Ok(())
}

fn cases() -> Vec<(&'static str, Path)> {
    let mut curved = Path::builder();
    curved.begin(point(40.0, 205.0));
    curved.cubic_bezier_to(
        point(175.0, 250.0),
        point(210.0, 170.0),
        point(140.0, 110.0),
    );
    curved.cubic_bezier_to(point(205.0, 72.0), point(260.0, 80.0), point(320.0, 130.0));
    curved.end(false);
    let mut mixed = Path::builder();
    mixed.begin(point(310.0, 80.0));
    mixed.line_to(point(150.0, 120.0));
    mixed.cubic_bezier_to(
        point(260.0, 140.0),
        point(125.0, 215.0),
        point(325.0, 225.0),
    );
    mixed.end(false);
    vec![("curved", curved.build()), ("line / curve", mixed.build())]
}
