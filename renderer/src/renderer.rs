use crate::{QuadRenderer, QuadBatch, MeshRenderer, MeshBatch, GpuGlobals, BufferId};

pub enum Batch {
    Quads(QuadBatch),
    Meshes(MeshBatch),
    Custom(Box<CustomBatch>),
}

pub trait CustomBatch {
    fn submit_batch(&self, renderer: &Renderer, pass: &mut wgpu::RenderPass);
}

pub struct Renderer {
    pub quads: QuadRenderer,
    pub meshes: MeshRenderer,
    pub globals: wgpu::Buffer,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, cpu_globals: GpuGlobals) -> Self {
        let globals = device
            .create_buffer_mapped(1, wgpu::BufferUsageFlags::VERTEX)
            .fill_from_slice(&[cpu_globals]);

        Renderer {
            quads: QuadRenderer::new(&device, &globals),
            meshes: MeshRenderer::new(&device, &globals),
            globals,
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
            BufferId::QuadInstances => &self.quads.instances,
            BufferId::Globals => &self.globals,
            BufferId::MeshVertices => &self.meshes.vertices,
            BufferId::MeshIndices => &self.meshes.indices,
            BufferId::MeshInstances => &self.meshes.instances,
            BufferId::MeshPrimitives => &self.meshes.primitives,
            BufferId::Layers => unimplemented!(),
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
