use glue::*;
use glue::geom::euclid;
use crate::bindings;
use crate::Registry;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GpuTransform2D {
    pub data: [f32; 8],
}

impl GpuTransform2D {
    pub const SIZE: u64 = (std::mem::size_of::<Self>() as u64);
    pub const BINDING: u32 = bindings::TRANSFORMS;

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
            label: Some("transforms"),
            size: count * Self::SIZE,
            usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        }
    }

    pub fn identity() -> Self {
        GpuTransform2D {
            data: [
                1.0, 0.0,
                0.0, 1.0,
                0.0, 0.0,
                0.0, 0.0,
            ],
        }
    }
}

impl<T1, T2> From<euclid::Transform2D<f32, T1, T2>> for GpuTransform2D {
    fn from(t: euclid::Transform2D<f32, T1, T2>) -> Self {
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

pub struct TransformSystem {
    buffer_id: BufferId,
}

impl TransformSystem {
    pub fn new(count: u32, device: &wgpu::Device, resources: &mut Registry) -> Self {
        let kind = resources.register_buffer_kind(GpuTransform2D::buffer_descriptor(count as u64));
        let buffer_id = kind.buffer_id(ResourceIndex(0, 0));

        resources.allocate_buffer(device, buffer_id);

        TransformSystem {
            buffer_id
        }
    }
}
