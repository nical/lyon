use renderer::{GpuFillVertex, GpuStrokeVertex, GpuStrokePrimitive, GpuFillPrimitive, GpuTransform};
use buffer::*;

// TODO: remove all of this

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RenderTargetId(pub u32);
