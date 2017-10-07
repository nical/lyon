use std::io;
use lyon::path::Path;

pub struct TessellateCmd {
    pub path: Path,
    pub fill: bool,
    pub stroke: Option<f32>,
    pub tolerance: f32,
}

pub struct FlattenCmd {
    pub input: String,
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
