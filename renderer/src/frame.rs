use std::sync::Arc;

use renderer::{ GpuFillVertex, GpuStrokeVertex, GpuStrokePrimitive, GpuFillPrimitive, GpuTransform };
use api::Color;
use batch_builder::{ FillGeometryRanges, StrokeGeometryRanges };
use buffer::*;

pub type Index = u16;
pub type IndexBufferId = BufferId<Index>;
pub type IndexBufferRange = BufferRange<Index>;
pub type TransformBufferId = BufferId<GpuTransform>;
pub type TransformBufferRange = BufferRange<GpuTransform>;
pub type FillVertexBufferId = BufferId<GpuFillVertex>;
pub type FillVertexBufferRange = BufferRange<GpuFillVertex>;
pub type StrokeVertexBufferId = BufferId<GpuStrokeVertex>;
pub type StrokeVertexBufferRange = BufferRange<GpuStrokeVertex>;
pub type FillPrimBufferId = BufferId<GpuFillPrimitive>;
pub type FillPrimBufferRange = BufferRange<GpuFillPrimitive>;
pub type StrokePrimBufferId = BufferId<GpuStrokePrimitive>;
pub type StrokePrimBufferRange = BufferRange<GpuStrokePrimitive>;

#[derive(Clone)]
pub struct FillCmd {
    pub geometry: FillGeometryRanges,
    pub instances: u32,
    pub prim_data: FillPrimBufferId,
    pub transforms: TransformBufferId,
}

impl FillCmd {
    pub fn default() -> Self {
        FillCmd {
            geometry: FillGeometryRanges {
                vertices: FillVertexBufferRange {
                    buffer: BufferId::new(0),
                    range: IdRange::empty(),
                },
                indices: IndexBufferRange {
                    buffer: IndexBufferId::new(0),
                    range: IdRange::empty(),
                },
            },
            instances: 1,
            prim_data: FillPrimBufferId::new(0),
            transforms: TransformBufferId::new(0),
        }
    }
}

#[derive(Clone)]
pub struct StrokeCmd {
    pub geometry: StrokeGeometryRanges,
    pub instances: u32,
    pub prim_data: StrokePrimBufferId,
    pub transforms: TransformBufferId,
}

impl StrokeCmd {
    pub fn default() -> Self {
        StrokeCmd {
            geometry: StrokeGeometryRanges {
                vertices: StrokeVertexBufferRange {
                    buffer: BufferId::new(0),
                    range: IdRange::empty(),
                },
                indices: IndexBufferRange {
                    buffer: IndexBufferId::new(0),
                    range: IdRange::empty(),
                },
            },
            instances: 1,
            prim_data: StrokePrimBufferId::new(0),
            transforms: TransformBufferId::new(0),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RenderTargetId(pub u32);

#[derive(Clone)]
pub struct RenderTargetCmds {
    pub fbo: RenderTargetId,
    pub opaque_fills: Vec<FillCmd>,
    pub opaque_strokes: Vec<StrokeCmd>,
    pub transparent_fills: Vec<FillCmd>,
    pub transparent_strokes: Vec<StrokeCmd>,
    pub clear_color: Option<Color>,
    pub clear_depth: Option<f32>,
}

unsafe impl Send for RenderTargetCmds {}

impl RenderTargetCmds {
    pub fn new(fbo: RenderTargetId) -> Self {
        RenderTargetCmds {
            fbo: fbo,
            opaque_fills: Vec::new(),
            opaque_strokes: Vec::new(),
            transparent_fills: Vec::new(),
            transparent_strokes: Vec::new(),
            clear_depth: Some(1.0),
            clear_color: Some(Color::black()),
        }
    }
}

pub struct TargetCmdBuilder {
    target: RenderTargetCmds,
}

impl TargetCmdBuilder {
    pub fn new(id: RenderTargetId) -> Self {
        TargetCmdBuilder {
            target: RenderTargetCmds {
                fbo: id,
                clear_color: None,
                clear_depth: None,
                opaque_fills: Vec::new(),
                opaque_strokes: Vec::new(),
                transparent_fills: Vec::new(),
                transparent_strokes: Vec::new(),
            }
        }
    }

    pub fn clear_color(mut self, color: Color) -> Self {
        self.target.clear_color = Some(color);
        return self;
    }

    pub fn clear_depth(mut self, depth: f32) -> Self {
        self.target.clear_depth = Some(depth);
        return self;
    }

    pub fn build(mut self) -> RenderTargetCmds { self.target }
}
