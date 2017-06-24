use commands::TessellateCmd;
use lyon::math::*;
use lyon::svg::parser::{build_path, ParserError};
use lyon::path::Path;
use lyon::path_iterator::*;
use lyon::tessellation::geometry_builder::{VertexBuffers, BuffersBuilder, VertexConstructor};
use lyon::tessellation::path_fill::*;
use lyon::tessellation::path_stroke::*;
use lyon::tessellation::{FillVertex, StrokeVertex};
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

impl ::std::convert::From<ParserError> for TessError {
    fn from(_err: ParserError) -> Self { TessError::Parse }
}

pub fn tessellate(mut cmd: TessellateCmd) -> Result<(), TessError> {

    let path = try!{ build_path(Path::builder().with_svg(), &cmd.input) };

    let mut buffers: VertexBuffers<Point> = VertexBuffers::new();

    if cmd.fill {
        if FillTessellator::new().tessellate_path(
            path.path_iter().flattened(cmd.tolerance),
            &FillOptions::default(),
            &mut BuffersBuilder::new(&mut buffers, ApplyNormal)
        ).is_err() {
            return Err(TessError::Fill);
        }
    }

    if let Some(width) = cmd.stroke {
        if StrokeTessellator::new().tessellate(
            path.path_iter().flattened(cmd.tolerance),
            &StrokeOptions::default(),
            &mut BuffersBuilder::new(&mut buffers, StrokeWidth(width))
        ).is_err() {
            return Err(TessError::Stroke);
        }
    }

    if cmd.count {
        try!{ writeln!(&mut *cmd.output, "vertices: {}", buffers.vertices.len()) };
        try!{ writeln!(&mut *cmd.output, "indices: {}", buffers.indices.len()) };
        try!{ writeln!(&mut *cmd.output, "triangles: {}", buffers.indices.len() / 3) };

        return Ok(());
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

    try!{ write!(&mut *cmd.output, "indices: [") };
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

struct StrokeWidth(f32);

impl VertexConstructor<StrokeVertex, Point> for StrokeWidth {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> Point {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());
        assert!(!vertex.normal.x.is_nan());
        assert!(!vertex.normal.y.is_nan());

        vertex.position + vertex.normal * self.0
    }
}

struct ApplyNormal;

impl VertexConstructor<FillVertex, Point> for ApplyNormal {
    fn new_vertex(&mut self, vertex: FillVertex) -> Point {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());

        vertex.position + vertex.normal
    }
}

