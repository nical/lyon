use lyon::math::*;
use lyon::tessellation::geometry_builder::{VertexConstructor, VertexBuffers, BuffersBuilder};
use lyon::tessellation::basic_shapes::*;
use lyon::tessellation::{FillTessellator, StrokeTessellator};
use lyon::tessellation;
use commands::{TessellateCmd, RenderCmd};
use std::borrow::Borrow;
use std::io::Write;
use std::str::FromStr;

pub struct FileFormatParseError;

pub enum FileFormat {
    JPG,
    PNG
}

impl FromStr for FileFormat {
    type Err = FileFormatParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().borrow() {
            "PNG" => Ok(FileFormat::PNG),
            "JPG" => Ok(FileFormat::JPG),
            _ => {
                println!("Wrong format provided, falling back to PNG.");
                Ok(FileFormat::PNG)
            }
        }
    }
}

pub fn export_path(cmd: TessellateCmd, render_options: RenderCmd, format: FileFormat, output: Box<Write>) {
    panic!("Not implemented yet !")
}
