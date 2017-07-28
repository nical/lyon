use commands::TessellateCmd;
use lyon::math::*;
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
}

impl ::std::convert::From<::std::io::Error> for TessError {
    fn from(err: io::Error) -> Self { TessError::Io(err) }
}

pub fn.tessellate_flattened_path(cmd: TessellateCmd) -> Result<VertexBuffers<Point>, TessError> {

    let mut buffers: VertexBuffers<Point> = VertexBuffers::new();

    if cmd.fill {
        if FillTessellator::new().tessellate_path(
            cmd.path.path_iter(),
            &FillOptions::tolerance(cmd.tolerance),
            &mut BuffersBuilder::new(&mut buffers, ApplyNormal)
        ).is_err() {
            return Err(TessError::Fill);
        }
    }

    if let Some(width) = cmd.stroke {
        StrokeTessellator::new().tessellate_flattened_path(
            cmd.path.path_iter().flattened(cmd.tolerance),
            &StrokeOptions::default(),
            &mut BuffersBuilder::new(&mut buffers, StrokeWidth(width))
        );
    }

    Ok(buffers)
}

pub fn write_output(
    buffers: VertexBuffers<Point>,
    count: bool,
    mut output: Box<io::Write>
) -> Result<(), io::Error>{

    if count {
        try!{ writeln!(&mut *output, "vertices: {}", buffers.vertices.len()) };
        try!{ writeln!(&mut *output, "indices: {}", buffers.indices.len()) };
        try!{ writeln!(&mut *output, "triangles: {}", buffers.indices.len() / 3) };

        return Ok(());
    }

    try!{ write!(&mut *output, "vertices: [") };
    let mut is_first = true;
    for vertex in &buffers.vertices {
        if !is_first {
            try!{ write!(&mut *output, ", ") };
        }
        try!{ write!(&mut *output, "({}, {})", vertex.x, vertex.y) };
        is_first = false;
    }
    try!{ writeln!(&mut *output, "]") };

    try!{ write!(&mut *output, "indices: [") };
    let mut is_first = true;
    for index in &buffers.indices {
        if !is_first {
            try!{ write!(&mut *output, ", ") };
        }
        try!{ write!(&mut *output, "{}", index) };
        is_first = false;
    }
    try!{ writeln!(&mut *output, "]") };

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

