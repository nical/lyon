use std::io;
use lyon::path::Path;
use lyon::tessellation::{FillOptions, StrokeOptions};
use lyon::algorithms::hatching::{HatchingOptions, DotOptions};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Tessellator {
    Default,
    Tess2,
    Experimental,
}

#[derive(Clone, Debug)]
pub struct TessellateCmd {
    pub path: Path,
    pub fill: Option<FillOptions>,
    pub stroke: Option<StrokeOptions>,
    pub hatch: Option<HatchingParams>,
    pub dots: Option<DotParams>,
    pub float_precision: Option<usize>,
    pub tessellator: Tessellator,
}

#[derive(Clone, Debug)]
pub struct HatchingParams {
    pub options: HatchingOptions,
    pub stroke: StrokeOptions,
    pub spacing: f32,
}

#[derive(Clone, Debug)]
pub struct DotParams {
    pub options: DotOptions,
    pub stroke: StrokeOptions,
    pub spacing: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AntiAliasing {
    Msaa(u16),
    None,
}

#[derive(Clone, Debug)]
pub struct RenderCmd {
    pub aa: AntiAliasing,
    pub background: Background,
    pub debugger: Option<u32>,
}

pub struct PathCmd {
    pub path: Path,
    pub output: Box<dyn io::Write>,
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
    pub tessellator: Tessellator,
    pub ignore_errors: bool,
}

#[derive(Copy, Clone, Debug)]
pub enum Background {
    Blue,
    Clear,
    Dark,
}
