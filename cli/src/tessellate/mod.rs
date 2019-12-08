use commands::{TessellateCmd, Tessellator};
use lyon::math::*;
use lyon::tessellation::geometry_builder::*;
use lyon::tessellation::{StrokeTessellator, FillTessellator};
use lyon::tess2;
use std::io;

mod format;
use self::format::format_output;

#[derive(Debug)]
pub enum TessError {
    Io(io::Error),
    Fill,
    Stroke,
}

impl ::std::convert::From<::std::io::Error> for TessError {
    fn from(err: io::Error) -> Self { TessError::Io(err) }
}

pub fn tessellate_path(cmd: TessellateCmd) -> Result<VertexBuffers<Point, u16>, TessError> {

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    if let Some(options) = cmd.fill {

        let ok = match cmd.tessellator {
            Tessellator::Default => {
                FillTessellator::new().tessellate_path(
                    &cmd.path,
                    &options,
                    &mut BuffersBuilder::new(&mut buffers, Positions)
                ).is_ok()
            }
            Tessellator::Tess2 => {
                tess2::FillTessellator::new().tessellate_path(
                    &cmd.path,
                    &options,
                    &mut BuffersBuilder::new(&mut buffers, Positions)
                ).is_ok()
            }
        };


        if !ok {
            return Err(TessError::Fill);
        }
    }

    if let Some(options) = cmd.stroke {
        let ok = StrokeTessellator::new().tessellate_path(
            &cmd.path,
            &options,
            &mut BuffersBuilder::new(&mut buffers, Positions)
        ).is_ok();

        if !ok {
            return Err(TessError::Stroke);
        }
    }

    Ok(buffers)
}

pub fn write_output(
    buffers: VertexBuffers<Point, u16>,
    count: bool,
    fmt_string: Option<&str>,
    float_precision: Option<usize>,
    mut output: Box<dyn io::Write>
) -> Result<(), io::Error> {

    if count {
        writeln!(&mut *output, "vertices: {}", buffers.vertices.len())?;
        writeln!(&mut *output, "indices: {}", buffers.indices.len())?;
        writeln!(&mut *output, "triangles: {}", buffers.indices.len() / 3)?;

        return Ok(());
    }

    writeln!(&mut *output, "{}", format_output(fmt_string, float_precision, &buffers))?;
    Ok(())
}
