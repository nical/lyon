use glue::geom::euclid;
use glue::units::*;
use crate::bindings;


pub trait GpuData : Copy {}

pub const GPU_BLOCK_SIZE: usize = std::mem::size_of::<GpuBlock>();

pub type GpuBlock = [u32; 4];
pub type Index = u16;
pub type PrimitiveIndex = u16;
pub type ImageSourceIndex = u16;
pub type TransformIndex = u16;
pub type PrimitiveRectIndex = u16;

impl GpuData for GpuBlock {}
impl GpuData for [u32; 8] {}
impl GpuData for [u32; 16] {}
impl GpuData for [u32; 32] {}
impl GpuData for [f32; 4] {}
impl GpuData for [f32; 8] {}
impl GpuData for [f32; 16] {}
impl GpuData for [f32; 32] {}
impl GpuData for [i32; 4] {}
impl GpuData for [i32; 8] {}
impl GpuData for [i32; 16] {}
impl GpuData for [i32; 32] {}
impl GpuData for u8 {}
impl GpuData for u16 {}
impl GpuData for i16 {}
impl GpuData for u32 {}
impl GpuData for i32 {}
impl GpuData for u64 {}
impl GpuData for i64 {}
impl GpuData for f32 {}


#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GpuGlobals {
    pub resolution: [f32; 2],
}

impl GpuGlobals {
    pub const BUFFER_SIZE: u64 = (std::mem::size_of::<Self>() as u64);
    pub const BINDING: u32 = bindings::GLOBALS;

    pub fn bind_group_layout_entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: Self::BINDING,
            visibility: wgpu::ShaderStage::VERTEX,
            ty: wgpu::BindingType::UniformBuffer {
                dynamic: false,
                min_binding_size: wgpu::BufferSize::new(0), // TODO
            },
            count: None,
        }
    }

    pub fn bind_group_entry(buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: Self::BINDING,
            resource: wgpu::BindingResource::Buffer(buffer.slice(0..Self::BUFFER_SIZE)),
        }
    }

    pub fn buffer_descriptor() -> wgpu::BufferDescriptor<'static> {
        wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Self>() as u64,
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        }
    }
}

impl GpuData for GpuGlobals {}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GpuColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl GpuColor {
    pub const VERTEX_FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Uint;
}

pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> GpuColor {
    GpuColor { r, g, b, a }
}

pub fn rgb(r: u8, g: u8, b: u8) -> GpuColor {
    GpuColor { r, g, b, a: 255 }
}


#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Instance {
    pub rect_id: PrimitiveRectIndex,
    pub transform_id: TransformIndex,
    pub primitive_id: PrimitiveIndex,
    pub src_color_id: ImageSourceIndex,
    pub src_mask_id: ImageSourceIndex,
    pub user_data: u16,
    pub z: u32,
}

impl Instance {
    pub fn pack(&self) -> GpuInstance {
        GpuInstance([
            ((self.rect_id as u32) << 16) + self.transform_id as u32,
            ((self.primitive_id as u32) << 16) + self.src_color_id as u32,
            ((self.src_mask_id as u32) << 16) + self.user_data as u32,
            self.z,
        ])
    }
}

impl GpuInstance {
    pub const SIZE: u64 = std::mem::size_of::<Self>() as u64;

    pub const VERTEX_ATTRIBUTES: &'static [wgpu::VertexAttributeDescriptor] = &[
        wgpu::VertexAttributeDescriptor {
            offset: 0,
            format: wgpu::VertexFormat::Uint4,
            shader_location: bindings::A_INSTANCE,
        },
    ];

    pub fn vertex_buffer_descriptor() -> wgpu::VertexBufferDescriptor<'static> {
        wgpu::VertexBufferDescriptor {
            stride: Self::SIZE,
            step_mode: wgpu::InputStepMode::Instance,
            attributes: Self::VERTEX_ATTRIBUTES,
        }
    }

    pub fn buffer_descriptor(count: u32) -> wgpu::BufferDescriptor<'static> {
        wgpu::BufferDescriptor {
            label: Some("instances"),
            size: count as u64 * Self::SIZE,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        }        
    }
}

impl GpuInstance {
    pub fn unpack(&self) -> Instance {
        let data = &self.0;
        Instance {
            rect_id: (data[0] >> 16) as u16,
            transform_id: (data[0] & 0x0000ffff) as u16,
            primitive_id: (data[1] >> 16) as u16,
            src_color_id: (data[1] & 0x0000ffff) as u16,
            src_mask_id: (data[2] >> 16) as u16,
            user_data: (data[2] & 0x0000ffff) as u16,
            z: data[3],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GpuInstance(pub [u32; 4]);

impl GpuData for GpuInstance {}


#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GpuLocalRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GpuDeviceRect {
    pub min: [u16; 2],
    pub max: [u16; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GpuImageSource {
    pub rect: [f32; 4],
    pub parameters: [f32; 4],
}

impl GpuImageSource {
    pub const BINDING: u32 = bindings::IMAGE_SOURCES;
    pub const SIZE: u64 = (std::mem::size_of::<Self>() as u64);

    pub fn bind_group_layout_entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: Self::BINDING,
            visibility: wgpu::ShaderStage::VERTEX,
            ty: wgpu::BindingType::StorageBuffer {
                dynamic: false,
                min_binding_size: wgpu::BufferSize::new(0), // TODO
                readonly: true,
            },
            count: None,
        }
    }

    pub fn bind_group_entry(buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: Self::BINDING,
            resource: wgpu::BindingResource::Buffer(buffer.slice(..)),
        }
    }

    pub fn buffer_descriptor(count: u64) -> wgpu::BufferDescriptor<'static> {
        wgpu::BufferDescriptor {
            label: Some("render tasks"),
            size: count * Self::SIZE,
            usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        }        
    }

    pub fn repeat(mut self, x: f32, y: f32, offset: DevicePoint) -> Self {
        self.parameters = [x, y, offset.x, offset.y];

        self
    }
}

pub struct GpuPrimitiveRects;

impl GpuPrimitiveRects {
    pub const SIZE: u64 = (std::mem::size_of::<[f32; 4]>() as u64);
    pub const BINDING: u32 = bindings::PRIMITIVE_RECTS;

    pub fn bind_group_layout_entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: Self::BINDING,
            visibility: wgpu::ShaderStage::VERTEX,
            ty: wgpu::BindingType::StorageBuffer {
                dynamic: false,
                min_binding_size: wgpu::BufferSize::new(0), // TODO
                readonly: true,
            },
            count: None,
        }
    }

    pub fn bind_group_entry(buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: Self::BINDING,
            resource: wgpu::BindingResource::Buffer(buffer.slice(..)),
        }
    }

    pub fn buffer_descriptor(count: u64) -> wgpu::BufferDescriptor<'static> {
        wgpu::BufferDescriptor {
            label: Some("prim rects"),
            size: count * Self::SIZE,
            usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        }
    }
}

pub struct GpuPrimitiveData;
impl GpuPrimitiveData {
    pub const SIZE: u64 = (std::mem::size_of::<[f32; 4]>() as u64);
    pub const BINDING: u32 = bindings::PRIMITIVE_DATA;

    pub fn bind_group_layout_entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: Self::BINDING,
            visibility: wgpu::ShaderStage::VERTEX,
            ty: wgpu::BindingType::UniformBuffer {
                dynamic: false,
                min_binding_size: wgpu::BufferSize::new(0), // TODO
            },
            count: None,
        }
    }

    pub fn bind_group_entry(buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: Self::BINDING,
            resource: wgpu::BindingResource::Buffer(buffer.slice(..)),
        }
    }

    pub fn buffer_descriptor() -> wgpu::BufferDescriptor<'static> {
        wgpu::BufferDescriptor {
            label: Some("prim data"),
            size: 8192,
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        }
    }
}

pub struct GpuF32Data;

impl GpuF32Data {
    pub const SIZE: u64 = (std::mem::size_of::<[f32;4]>() as u64);
    pub const BINDING: u32 = bindings::F32_DATA;

    pub fn bind_group_layout_entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: Self::BINDING,
            visibility: wgpu::ShaderStage::VERTEX,
            ty: wgpu::BindingType::UniformBuffer {
                dynamic: false,
                min_binding_size: wgpu::BufferSize::new(0), // TODO
            },
            count: None,
        }
    }

    pub fn bind_group_entry(buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: Self::BINDING,
            resource: wgpu::BindingResource::Buffer(buffer.slice(..)),
        }
    }
}

pub struct GpuU32Data;

impl GpuU32Data {
    pub const SIZE: u64 = (std::mem::size_of::<[u32;4]>() as u64);
    pub const BINDING: u32 = bindings::U32_DATA;

    pub fn bind_group_layout_entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: Self::BINDING,
            visibility: wgpu::ShaderStage::VERTEX,
            ty: wgpu::BindingType::UniformBuffer {
                dynamic: false,
                min_binding_size: wgpu::BufferSize::new(0), // TODO
            },
            count: None,
        }
    }

    pub fn bind_group_entry(buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: Self::BINDING,
            resource: wgpu::BindingResource::Buffer(buffer.slice(..)),
        }
    }
}

pub struct U8AlphaMask;

impl U8AlphaMask {
    pub const BINDING: u32 = bindings::U8_MASK;

    pub const SIZE: wgpu::Extent3d = wgpu::Extent3d { width: 1024, height: 1024, depth: 1 };

    pub fn bind_group_layout_entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: Self::BINDING,
            visibility: wgpu::ShaderStage::FRAGMENT,
            ty: wgpu::BindingType::SampledTexture {
                multisampled: false,
                component_type: wgpu::TextureComponentType::Float,
                dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        }
    }

    pub fn bind_group_entry(texture: &wgpu::TextureView) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: Self::BINDING,
            resource: wgpu::BindingResource::TextureView(texture),
        }
    }    
}

pub struct ColorAtlasTexture;

impl ColorAtlasTexture {
    pub const BINDING: u32 = bindings::INPUT_COLOR_0;

    pub const SIZE: wgpu::Extent3d = wgpu::Extent3d { width: 4096, height: 4096, depth: 1 };

    pub fn bind_group_layout_entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: Self::BINDING,
            visibility: wgpu::ShaderStage::FRAGMENT,
            ty: wgpu::BindingType::SampledTexture {
                multisampled: false,
                component_type: wgpu::TextureComponentType::Float,
                dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        }
    }

    pub fn bind_group_entry(texture: &wgpu::TextureView) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: Self::BINDING,
            resource: wgpu::BindingResource::TextureView(texture),
        }
    }    
}

pub struct DefaultSampler;

impl DefaultSampler {
    pub const BINDING: u32 = bindings::DEFAULT_SAMPLER;

    pub fn bind_group_layout_entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: Self::BINDING,
            visibility: wgpu::ShaderStage::FRAGMENT,
            ty: wgpu::BindingType::Sampler { comparison: false },
            count: None,
        }
    }

    pub fn bind_group_entry(sampler: &wgpu::Sampler) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: Self::BINDING,
            resource: wgpu::BindingResource::Sampler(sampler),
        }
    }    
}
