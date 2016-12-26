use std::io;

pub struct TessellateCmd {
    pub input: String,
    pub output: Box<io::Write>,
    pub fill: bool,
    pub stroke: Option<f32>,
    pub tolerance: f32,
    pub count: bool,
}

pub struct FlattenCmd {
    pub input: String,
    pub output: Box<io::Write>,
    pub tolerance: f32,
    pub count: bool,
}

