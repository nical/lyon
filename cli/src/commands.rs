use std::io;
use lyon::path::default::Path;
use lyon::tessellation::{FillOptions, StrokeOptions};

#[derive(Clone, Debug)]
pub struct TessellateCmd {
    pub path: Path,
    pub fill: Option<FillOptions>,
    pub stroke: Option<StrokeOptions>,
    pub float_precision: Option<usize>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AntiAliasing {
    Msaa(u16),
    None,
}

#[derive(Clone, Debug)]
pub struct RenderCmd {
    pub aa: AntiAliasing,
}

pub struct PathCmd {
    pub path: Path,
    pub output: Box<io::Write>,
    pub tolerance: f32,
    pub count: bool,
    pub flatten: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct FuzzCmd {
    pub fill: bool,
    pub stroke: bool,
    pub min_points: Option<u32>,
    pub max_points: Option<u32>,
}
