use resvg::tree::{self, Color, Paint, Stroke};
use lyon::tessellation::{self, StrokeOptions};

use super::FALLBACK_COLOR;

pub fn convert_stroke(s: &Stroke) -> (Color, StrokeOptions) {
    let color = match s.paint {
        Paint::Color(c) => c,
        _ => FALLBACK_COLOR,
    };
    let linecap = match s.linecap {
        tree::LineCap::Butt => tessellation::LineCap::Butt,
        tree::LineCap::Square => tessellation::LineCap::Square,
        tree::LineCap::Round => tessellation::LineCap::Round,
    };
    let linejoin = match s.linejoin {
        tree::LineJoin::Miter => tessellation::LineJoin::Miter,
        tree::LineJoin::Bevel => tessellation::LineJoin::Bevel,
        tree::LineJoin::Round => tessellation::LineJoin::Round,
    };

    let opt = StrokeOptions::tolerance(0.01)
        .with_line_width(s.width as f32)
        .with_line_cap(linecap)
        .with_line_join(linejoin);

    (color, opt)
}
