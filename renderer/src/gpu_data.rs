use crate::geom::euclid;

use std::ops::Range;

pub unsafe trait GpuData : Copy {}

pub const GPU_BLOCK_SIZE: usize = std::mem::size_of::<GpuBlock>();

pub type GpuBlock = [u32; 4];
pub type Index = u16;

unsafe impl GpuData for GpuBlock {}
unsafe impl GpuData for [u32; 8] {}
unsafe impl GpuData for [u32; 16] {}
unsafe impl GpuData for [u32; 32] {}
unsafe impl GpuData for [f32; 4] {}
unsafe impl GpuData for [f32; 8] {}
unsafe impl GpuData for [f32; 16] {}
unsafe impl GpuData for [f32; 32] {}
unsafe impl GpuData for [i32; 4] {}
unsafe impl GpuData for [i32; 8] {}
unsafe impl GpuData for [i32; 16] {}
unsafe impl GpuData for [i32; 32] {}
unsafe impl GpuData for u8 {}
unsafe impl GpuData for u16 {}
unsafe impl GpuData for i16 {}
unsafe impl GpuData for u32 {}
unsafe impl GpuData for i32 {}
unsafe impl GpuData for u64 {}
unsafe impl GpuData for i64 {}
unsafe impl GpuData for f32 {}


#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BufferId {
    Transfer(u16),
    QuadInstances,
    Globals,
    MeshVertices,
    MeshIndices,
    MeshInstances,
    MeshPrimitives,
    Transforms,
    Custom(u16),
}

impl BufferId {
    pub fn range(self, range: Range<u32>) -> BufferRange {
        BufferRange { buffer: self, range }
    }

    pub fn at_offset(self, offset: u32) -> BufferOffset {
        BufferOffset { buffer: self, offset }
    }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct BufferRange {
    pub buffer: BufferId,
    // In bytes
    pub range: Range<u32>,
}

impl BufferRange {
    pub fn new(buffer: BufferId, range: Range<u32>) -> Self {
        BufferRange { buffer, range }
    }

    pub fn len(&self) -> u32 {
        self.range.end - self.range.start
    }

    pub fn start(&self) -> u32 {
        self.range.start
    }

    pub fn end(&self) -> u32 {
        self.range.end
    }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct BufferOffset {
    pub buffer: BufferId,
    pub offset: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GpuGlobals {
    pub resolution: [f32; 2],
}

impl GpuGlobals {
    pub const BUFFER_SIZE: u32 = (std::mem::size_of::<Self>() as u32);
    pub const BINDING: u32 = 0;  

    pub fn bind_group_layout_binding() -> wgpu::BindGroupLayoutBinding {
        wgpu::BindGroupLayoutBinding {
            binding: Self::BINDING,
            visibility: wgpu::ShaderStageFlags::VERTEX,
            ty: wgpu::BindingType::UniformBuffer,
        }
    }

    pub fn binding(buffer: &wgpu::Buffer) -> wgpu::Binding {
        wgpu::Binding {
            binding: Self::BINDING,
            resource: wgpu::BindingResource::Buffer {
                buffer,
                range: 0..Self::BUFFER_SIZE,
            },
        }
    }
}

unsafe impl GpuData for GpuGlobals {}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GpuColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> GpuColor {
    GpuColor { r, g, b, a }
}

pub fn rgb(r: u8, g: u8, b: u8) -> GpuColor {
    GpuColor { r, g, b, a: 255 }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GpuTransform2D {
    pub data: [f32; 8],
}

impl<T1, T2> From<euclid::TypedTransform2D<f32, T1, T2>> for GpuTransform2D {
    fn from(t: euclid::TypedTransform2D<f32, T1, T2>) -> Self {
        GpuTransform2D {
            data: [
                t.m11, t.m12,
                t.m21, t.m22,
                t.m31, t.m32,
                0.0, 0.0,
            ]
        }
    }
}
