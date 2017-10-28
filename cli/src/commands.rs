use std::io;
use lyon::path::Path;
use lyon::tessellation::{FillOptions, StrokeOptions};

#[derive(Clone, Debug)]
pub struct TessellateCmd {
    pub path: Path,
    pub fill: Option<FillOptions>,
    pub stroke: Option<StrokeOptions>,
}

pub struct FlattenCmd {
    pub path: Path,
    pub output: Box<io::Write>,
    pub tolerance: f32,
    pub count: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct FuzzCmd {
    pub fill: bool,
    pub stroke: bool,
    pub min_points: Option<u32>,
    pub max_points: Option<u32>,
}
