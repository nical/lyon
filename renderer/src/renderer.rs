use crate::{
    QuadRenderer, QuadBatch, MeshRenderer, MeshBatch, GpuGlobals, BufferId,
    GpuBufferAllocator, BumpAllocator, GpuData,
};
use std::ops::Range;
use std::sync::Arc;

pub enum Batch {
    Quads(QuadBatch),
    Meshes(MeshBatch),
    Custom(Box<CustomBatch>),
}

pub trait CustomBatch {
    fn submit_batch(&self, renderer: &Renderer, pass: &mut wgpu::RenderPass);
}

pub struct Buffer {
    pub handle: wgpu::Buffer,
    pub descriptor: wgpu::BufferDescriptor,
}

impl Buffer {
    pub fn new(device: &wgpu::Device, descriptor: wgpu::BufferDescriptor) -> Self {
        Buffer {
            handle: device.create_buffer(&descriptor),
            descriptor,
        }
    }

    pub fn from_data<T: GpuData + 'static>(
        device: &wgpu::Device,
        usage: wgpu::BufferUsageFlags,
        data: &[T]
    ) -> Self {
        Buffer {
            descriptor: wgpu::BufferDescriptor {
                size: (data.len() * std::mem::size_of::<T>()) as u32,
                usage,
            },
            handle: device.create_buffer_mapped(data.len(), usage)
                .fill_from_slice(data),
        }
    }

    pub fn resize(&mut self, new_size: u32, device: &wgpu::Device) {
        self.descriptor.size = new_size;
        self.handle = device.create_buffer(&self.descriptor);
    }
}

pub struct CommonBuffers {
    pub globals: Buffer,
    pub transforms: Buffer,
}

pub struct Renderer {
    pub quads: QuadRenderer,
    pub meshes: MeshRenderer,
    pub common: CommonBuffers,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, cpu_globals: GpuGlobals) -> Self {
        let common = CommonBuffers {
            globals: Buffer::from_data(
                device,
                wgpu::BufferUsageFlags::VERTEX,
                &[cpu_globals],
            ),
            transforms: Buffer::new(
                device,
                wgpu::BufferDescriptor {
                    size: 4096,
                    usage: wgpu::BufferUsageFlags::VERTEX,
                },
            ),
        };

        Renderer {
            quads: QuadRenderer::new(&device, &common),
            meshes: MeshRenderer::new(&device, &common),
            common,
        }
    }

    pub fn submit_batches(&self, batches: &[Batch], pass: &mut wgpu::RenderPass) {
        for batch in batches {
            match *batch {
                Batch::Quads(ref batch) => {
                    self.quads.submit_batch(batch, pass);
                }
                Batch::Meshes(ref batch) => {
                    self.meshes.submit_batch(batch, pass);
                }
                Batch::Custom(ref custom_batch) => {
                    custom_batch.submit_batch(self, pass);
                }
            }
        }
    }

    pub fn render_target(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth_buffer: Option<&wgpu::TextureView>,
        target_passes: &TargetPasses,
    ) {
        {
            let depth_stencil_attachment = depth_buffer.map(|texure_view| {
                wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: texure_view,
                    depth_load_op: wgpu::LoadOp::Clear,
                    depth_store_op: wgpu::StoreOp::Store,
                    stencil_load_op: wgpu::LoadOp::Clear,
                    stencil_store_op: wgpu::StoreOp::Store,
                    clear_depth: 0.0,
                    clear_stencil: 0,
                }
            });

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &target,
                    load_op: if target_passes.clear { wgpu::LoadOp::Clear } else { wgpu::LoadOp::Load },
                    store_op: wgpu::StoreOp::Store,
                    clear_color: target_passes.clear_color,
                }],
                depth_stencil_attachment,
            });

            self.submit_batches(&target_passes.opaque_pass, &mut pass);
        }

        {
            let depth_stencil_attachment = depth_buffer.map(|texture_view| {
                wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: texture_view,
                    depth_load_op: wgpu::LoadOp::Load,
                    depth_store_op: wgpu::StoreOp::Store,
                    stencil_load_op: wgpu::LoadOp::Clear,
                    stencil_store_op: wgpu::StoreOp::Store,
                    clear_depth: 0.0,
                    clear_stencil: 0,
                }
            });

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &target,
                    load_op: wgpu::LoadOp::Load,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color::WHITE,
                }],
                depth_stencil_attachment,
            });

            self.submit_batches(&target_passes.blend_pass, &mut pass);        
        }
    }
}

impl std::ops::Index<BufferId> for Renderer {
    type Output = wgpu::Buffer;
    fn index(&self, id: BufferId) -> &wgpu::Buffer {
        match id {
            BufferId::Transfer(_idx) => unimplemented!(),
            BufferId::QuadInstances => &self.quads.instances.handle,
            BufferId::MeshVertices => &self.meshes.vertices.handle,
            BufferId::MeshIndices => &self.meshes.indices.handle,
            BufferId::MeshInstances => &self.meshes.instances.handle,
            BufferId::MeshPrimitives => &self.meshes.primitives.handle,
            BufferId::Globals => &self.common.globals.handle,
            BufferId::Transforms => &self.common.transforms.handle,
            BufferId::Custom(_idx) => unimplemented!(),
        }
    }    
}

pub struct TargetPasses {
    pub clear: bool,
    pub clear_color: wgpu::Color,
    pub opaque_pass: Vec<Batch>,
    pub blend_pass: Vec<Batch>,
}

#[derive(Clone)]
pub struct FrameDataAllocators {
    pub mesh_vertices: GpuBufferAllocator,
    pub mesh_indices: GpuBufferAllocator,
    pub mesh_primitives: GpuBufferAllocator,
    pub mesh_instances: GpuBufferAllocator,
    pub quad_instances: GpuBufferAllocator,
    pub transforms: GpuBufferAllocator,
}

impl FrameDataAllocators {
    pub fn new(
        mesh_vertices: Range<u32>,
        mesh_indices: Range<u32>,
        mesh_primitives: Range<u32>,
        mesh_instances: Range<u32>,
        quad_instances: Range<u32>,
        transforms: Range<u32>,
    ) -> Self {
        FrameDataAllocators {
            mesh_vertices: GpuBufferAllocator::new(BufferId::MeshVertices, Arc::new(BumpAllocator::new(mesh_vertices))),
            mesh_indices: GpuBufferAllocator::new(BufferId::MeshIndices, Arc::new(BumpAllocator::new(mesh_indices))),
            mesh_primitives: GpuBufferAllocator::new(BufferId::MeshPrimitives, Arc::new(BumpAllocator::new(mesh_primitives))),
            mesh_instances: GpuBufferAllocator::new(BufferId::MeshInstances, Arc::new(BumpAllocator::new(mesh_instances))),
            quad_instances: GpuBufferAllocator::new(BufferId::QuadInstances, Arc::new(BumpAllocator::new(quad_instances))),
            transforms: GpuBufferAllocator::new(BufferId::Transforms, Arc::new(BumpAllocator::new(transforms))),
        }
    }

    pub fn select(&self, id: BufferId) -> &GpuBufferAllocator {
        match id {
            BufferId::MeshVertices => &self.mesh_vertices,
            BufferId::MeshIndices => &self.mesh_indices,
            BufferId::MeshPrimitives => &self.mesh_primitives,
            BufferId::MeshInstances => &self.mesh_instances,
            BufferId::QuadInstances => &self.quad_instances,
            BufferId::Transforms => &self.transforms,
            _ => panic!("unsupported destination buffer"),
        }
    }
}
