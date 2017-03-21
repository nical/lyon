use std::sync::Arc;

use renderer::{ GpuFillVertex, GpuStrokeVertex, GpuStrokePrimitive, GpuFillPrimitive, GpuTransform };
use api::Color;
use frame_builder::{ FillGeometryRanges, StrokeGeometryRanges };
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

/*
#[derive(Clone)]
pub struct DrawParams {
    pub vbo: VertexBufferId,
    pub indices: IndexBufferRange,
    pub instances: Option<u16>,
    pub texture: Option<TextureId>,
    pub shape_data: UniformBufferId,
    pub transforms: TransformBufferId,
}

#[derive(Clone)]
pub enum UploadCmd {
    GpuFillPrimitive(UniformBufferId, u16, Vec<GpuFillPrimitive>),
    GpuStrokePrimitive(UniformBufferId, u16, Vec<GpuStrokePrimitive>),
    Transform(UniformBufferId, u16, Vec<GpuTransform>),
    FillVertex(VertexBufferId, u16, Vec<FillVertex>),
    StrokeVertex(VertexBufferId, u16, Vec<StrokeVertex>),
}

#[derive(Clone)]
pub enum AllocCmd {
    //AllocStrokeVertexBuffer(VertexBufferId, Arc<Vec<FillVertex>>),
    //AllocFillVertexBuffer(VertexBufferId, Arc<Vec<FillVertex>>),
    //AllocIndexBuffer(IndexBufferId, Arc<Vec<FillVertex>>),
    AddFillVertexBuffer(VertexBufferId, Arc<Vec<FillVertex>>, Arc<Vec<u16>>),
    AddStrokeVertexBuffer(VertexBufferId, Arc<Vec<StrokeVertex>>, Arc<Vec<u16>>),
    AddFillShapeBuffer(UniformBufferId, Arc<Vec<GpuFillPrimitive>>),
    AddStrokeShapeBuffer(UniformBufferId, Arc<Vec<GpuStrokePrimitive>>),
    AddTransformBuffer(UniformBufferId, Arc<Vec<GpuTransform>>),
    AddTexture(TextureId, ImageDescriptor, Arc<Vec<u8>>),
    RemoveFillShapeBuffer(UniformBufferId),
    RemoveStrokeShapeBuffer(UniformBufferId),
    RemoveTransformBuffer(UniformBufferId),
    RemoveTexture(TextureId),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    Rgba8,
    A8,
    RgbaF32,
    AF32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ImageDescriptor {
    width: u32,
    height: u32,
    stride: Option<u32>,
    format: ImageFormat,
}

#[derive(Clone)]
pub struct FrameCmds {
    pub allocations: Vec<AllocCmd>,
    pub uploads: Vec<UploadCmd>,
    pub targets: Vec<RenderTargetCmds>,
}

*/

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

/*
pub struct FrameCmdBuilder {
    frame: FrameCmds,
}

impl FrameCmdBuilder {
    pub fn new() -> Self {
        FrameCmdBuilder {
            frame: FrameCmds {
                allocations: Vec::new(),
                uploads: Vec::new(),
                targets: Vec::new(),
            }
        }
    }

    pub fn add_target(mut self, target: RenderTargetCmds) -> Self {
        self.frame.targets.push(target);
        return self;
    }

    pub fn update_fill_shapes(mut self, id: UniformBufferId, offset: u16, data: Vec<GpuFillPrimitive>) -> Self {
        self.frame.uploads.push(UploadCmd::GpuFillPrimitive(id, offset, data));
        return self;
    }

    pub fn update_stroke_shapes(mut self, id: UniformBufferId, offset: u16, data: Vec<GpuStrokePrimitive>) -> Self {
        self.frame.uploads.push(UploadCmd::GpuStrokePrimitive(id, offset, data));
        return self;
    }

    pub fn update_transforms(mut self, id: UniformBufferId, offset: u16, data: Vec<GpuTransform>) -> Self {
        self.frame.uploads.push(UploadCmd::Transform(id, offset, data));
        return self;
    }

    pub fn build(self) -> FrameCmds { self.frame }
}
*/