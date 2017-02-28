use std::sync::Arc;

use renderer::{FillVertex, StrokeVertex, StrokeShapeData, FillShapeData, ShapeTransform};
use api::Color;
use frame_builder::{MeshData};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RenderTargetId(pub u32);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextureId(pub u32);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct UniformBufferId(pub u32);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct VertexBufferId(pub u16);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct IndexBufferId(pub u16);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BufferRange<ID> {
    pub buffer: ID,
    pub first: u16,
    pub count: u16,
}

impl<ID> BufferRange<ID> {
    pub fn buffer_id(self) -> ID { self.buffer }

    pub fn range(self) -> (u16, u16) { (self.first, self.first + self.count) }

    pub fn first(&self) -> u32 { self.first as u32 }

    pub fn count(&self) -> u32 { self.count as u32 }

    pub fn end(&self) -> u32 { self.first() + self.count() }
}

pub type IndexBufferRange = BufferRange<IndexBufferId>;
pub type VertexBufferRange = BufferRange<VertexBufferId>;

#[derive(Clone)]
pub struct DrawParams {
    pub vbo: VertexBufferId,
    pub indices: IndexBufferRange,
    pub instances: Option<u16>,
    pub texture: Option<TextureId>,
    pub shape_data: UniformBufferId,
    pub transforms: UniformBufferId,
}

#[derive(Clone)]
pub enum UploadCmd {
    FillShapeData(UniformBufferId, u16, Vec<FillShapeData>),
    StrokeShapeData(UniformBufferId, u16, Vec<StrokeShapeData>),
    Transform(UniformBufferId, u16, Vec<ShapeTransform>),
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
    AddFillShapeBuffer(UniformBufferId, Arc<Vec<FillShapeData>>),
    AddStrokeShapeBuffer(UniformBufferId, Arc<Vec<StrokeShapeData>>),
    AddTransformBuffer(UniformBufferId, Arc<Vec<ShapeTransform>>),
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
pub struct DrawCmd {
    pub mesh: MeshData,
    pub instances: u32,
    pub prim_data: UniformBufferId,
    pub transforms: UniformBufferId,
}

#[derive(Clone)]
pub struct RenderTargetCmds {
    pub fbo: RenderTargetId,
    pub opaque_fills: Vec<DrawCmd>,
    pub opaque_strokes: Vec<DrawCmd>,
    pub transparent_fills: Vec<DrawCmd>,
    pub transparent_strokes: Vec<DrawCmd>,
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


#[derive(Clone)]
pub struct FrameCmds {
    pub allocations: Vec<AllocCmd>,
    pub uploads: Vec<UploadCmd>,
    pub targets: Vec<RenderTargetCmds>,
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

//    pub fn add_opaque(mut self, cmd: DrawCmd) -> Self {
//        self.target.opaque_cmds.push(cmd);
//        return self;
//    }

//    pub fn add_transparent(mut self, cmd: DrawCmd) -> Self {
//        self.target.transparent_cmds.push(cmd);
//        return self;
//    }

    pub fn build(mut self) -> RenderTargetCmds { self.target }
}

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

    pub fn update_fill_shapes(mut self, id: UniformBufferId, offset: u16, data: Vec<FillShapeData>) -> Self {
        self.frame.uploads.push(UploadCmd::FillShapeData(id, offset, data));
        return self;
    }

    pub fn update_stroke_shapes(mut self, id: UniformBufferId, offset: u16, data: Vec<StrokeShapeData>) -> Self {
        self.frame.uploads.push(UploadCmd::StrokeShapeData(id, offset, data));
        return self;
    }

    pub fn update_transforms(mut self, id: UniformBufferId, offset: u16, data: Vec<ShapeTransform>) -> Self {
        self.frame.uploads.push(UploadCmd::Transform(id, offset, data));
        return self;
    }

    pub fn build(self) -> FrameCmds { self.frame }
}
