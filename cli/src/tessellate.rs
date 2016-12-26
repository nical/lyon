use commands::TessellateCmd;
use lyon::math::Vec2;
use lyon::svg::parser;
use lyon::path::Path;
use lyon::path_builder::*;
use lyon::path_iterator::*;
use lyon::tessellation::geometry_builder::{simple_builder, VertexBuffers};
use lyon::tessellation::path_fill::*;
use lyon::tessellation::path_stroke::*;
use std::io;

#[derive(Debug)]
pub enum TessError {
    Io(io::Error),
    Fill,
    Stroke,
    Parse,
}

impl ::std::convert::From<::std::io::Error> for TessError {
    fn from(err: io::Error) -> Self { TessError::Io(err) }
}

pub fn tessellate(mut cmd: TessellateCmd) -> Result<(), TessError> {

    let mut builder = SvgPathBuilder::new(Path::builder());
    let mut buffers: VertexBuffers<Vec2> = VertexBuffers::new();

    for item in parser::path::PathTokenizer::new(&cmd.input) {
        if let Ok(event) = item {
            builder.svg_event(event)
        } else {
            return Err(TessError::Parse);
        }
    }

    let path = builder.build();

    if cmd.fill {
        let events = FillEvents::from_iter(path.path_iter().flattened(cmd.tolerance));
        if FillTessellator::new().tessellate_events(
            &events,
            &FillOptions::default(),
            &mut simple_builder(&mut buffers)
        ).is_err() {
            return Err(TessError::Fill);
        }
    }

    if cmd.stroke {
        if StrokeTessellator::new().tessellate(
            path.path_iter().flattened(cmd.tolerance),
            &StrokeOptions::stroke_width(1.0),
            &mut simple_builder(&mut buffers)
        ).is_err() {
            return Err(TessError::Stroke);
        }
    }

    try!{ write!(&mut *cmd.output, "vertices: [") };
    let mut is_first = true;
    for vertex in buffers.vertices {
        if !is_first {
            try!{ write!(&mut *cmd.output, ", ") };
        }
        try!{ write!(&mut *cmd.output, "({}, {})", vertex.x, vertex.y) };
        is_first = false;
    }
    try!{ writeln!(&mut *cmd.output, "]") };

    try!{ write!(&mut *cmd.output, "inices: [") };
    let mut is_first = true;
    for index in buffers.indices {
        if !is_first {
            try!{ write!(&mut *cmd.output, ", ") };
        }
        try!{ write!(&mut *cmd.output, "{}", index) };
        is_first = false;
    }
    try!{ writeln!(&mut *cmd.output, "]") };

    Ok(())
}
