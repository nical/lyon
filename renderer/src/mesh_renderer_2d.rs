use crate::{
    GpuData, GpuGlobals, RenderTargetState, BlendMode, GpuColor, GpuDataPipe, BufferId, write_to_pipe,
    AllocError, CommonBuffers, Buffer,
};


#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GpuMeshVertex {
    pub position: [f32; 2],
    pub prim_id: u32,
}

unsafe impl GpuData for GpuMeshVertex {}

impl GpuMeshVertex {
    pub const VERTEX_ATTRIBUTES: &'static [wgpu::VertexAttributeDescriptor] = &[
        wgpu::VertexAttributeDescriptor {
            offset: 0,
            format: wgpu::VertexFormat::Float2,
            attribute_index: 0,
        },
        wgpu::VertexAttributeDescriptor {
            offset: 8,
            format: wgpu::VertexFormat::Uint,
            attribute_index: 1,
        },
    ];

    pub fn vertex_buffer_descriptor() -> wgpu::VertexBufferDescriptor<'static> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<Self>() as u32,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: Self::VERTEX_ATTRIBUTES,
        }
    }
}


#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GpuMeshPrimitive {
    pub z_index: u32,
    pub color: GpuColor,
}

unsafe impl GpuData for GpuMeshPrimitive {}

impl GpuMeshPrimitive {
    // Must match the constant in mesh2d.vert.glsl
    pub const BUFFER_LEN: u32 = 1024;
    pub const BUFFER_SIZE: u32 = Self::BUFFER_LEN * (std::mem::size_of::<Self>() as u32);
    pub const BINDING: u32 = 1;

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

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GpuMeshInstance {
    pub transform: u32,
    pub prim_offset: u32,
    pub layer: u32,
    pub z_index: u32,
}

unsafe impl GpuData for GpuMeshInstance {}

impl GpuMeshInstance {
    pub const VERTEX_ATTRIBUTES: &'static [wgpu::VertexAttributeDescriptor] = &[
        wgpu::VertexAttributeDescriptor {
            offset: 0,
            format: wgpu::VertexFormat::Uint,
            attribute_index: 2,
        },
        wgpu::VertexAttributeDescriptor {
            offset: 4,
            format: wgpu::VertexFormat::Uint,
            attribute_index: 3,
        },
        wgpu::VertexAttributeDescriptor {
            offset: 8,
            format: wgpu::VertexFormat::Uint,
            attribute_index: 4,
        },
        wgpu::VertexAttributeDescriptor {
            offset: 12,
            format: wgpu::VertexFormat::Uint,
            attribute_index: 5,
        },
    ];

    pub fn vertex_buffer_descriptor() -> wgpu::VertexBufferDescriptor<'static> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<Self>() as u32,
            step_mode: wgpu::InputStepMode::Instance,
            attributes: Self::VERTEX_ATTRIBUTES,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MeshBatch {
    pub indices: std::ops::Range<u32>,
    pub instances: std::ops::Range<u32>,
    pub blend_mode: BlendMode,
    pub base_vertex: i32,
}

impl MeshBatch {
    pub fn with_blend_mode(&self, mode: BlendMode) -> Self {
        let mut batch = self.clone();
        batch.blend_mode = mode;
        batch
    }

    pub fn instances(&self, mut range: std::ops::Range<u32>) -> Self {
        range.end = self.instances.end.min(range.end);
        range.start = self.instances.start.max(range.start);
        let mut batch = self.clone();
        batch.instances = range;
        batch
    }
}

pub struct MeshRenderer {
    pub opaque_pipeline: wgpu::RenderPipeline,
    pub alpha_pipeline: wgpu::RenderPipeline,

    pub indices: Buffer,
    pub vertices: Buffer,
    pub instances: Buffer,
    pub primitives: Buffer,

    pub pipeline_layout: wgpu::PipelineLayout,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl MeshRenderer {
    pub fn new(device: &wgpu::Device, common: &CommonBuffers) -> Self {
        let bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    GpuGlobals::bind_group_layout_binding(),
                    GpuMeshPrimitive::bind_group_layout_binding(),
                ]
            }
        );

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
        });

        let vs_bytes = include_bytes!("./../shaders/mesh2d.vert.spv");
        let fs_bytes = include_bytes!("./../shaders/mesh2d.frag.spv");
        let vs_module = device.create_shader_module(vs_bytes);
        let fs_module = device.create_shader_module(fs_bytes);

        let opaque_target = RenderTargetState::opaque_pass();
        let opaque_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::PipelineStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: wgpu::PipelineStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            },
            rasterization_state: wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            },
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[opaque_target.color],
            depth_stencil_state: opaque_target.depth_stencil, 
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[
                GpuMeshVertex::vertex_buffer_descriptor(),
                GpuMeshInstance::vertex_buffer_descriptor(),
            ],
            sample_count: 1,
        };

        let opaque_pipeline = device.create_render_pipeline(&opaque_pipeline_descriptor);

        let alpha_target = RenderTargetState::blend_pass_with_depth_test(BlendMode::Alpha);
        let alpha_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::PipelineStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: wgpu::PipelineStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            },
            rasterization_state: wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            },
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[alpha_target.color],
            depth_stencil_state: alpha_target.depth_stencil, 
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[
                GpuMeshVertex::vertex_buffer_descriptor(),
                GpuMeshInstance::vertex_buffer_descriptor(),
            ],
            sample_count: 1,
        };

        let alpha_pipeline = device.create_render_pipeline(&alpha_pipeline_descriptor);

        let instances = Buffer::new(
            device,
            wgpu::BufferDescriptor {
                size: 4096 * 4,
                usage: wgpu::BufferUsageFlags::VERTEX,
            },
        );

        let vertices = Buffer::new(
            device,
            wgpu::BufferDescriptor {
                size: 4096 * 16,
                usage: wgpu::BufferUsageFlags::VERTEX,
            },
        );

        let indices = Buffer::new(
            device,
            wgpu::BufferDescriptor {
                size: 4096 * 4,
                usage: wgpu::BufferUsageFlags::VERTEX,
            },
        );

        let primitives = Buffer::new(
            device,
            wgpu::BufferDescriptor {
                size: GpuMeshPrimitive::BUFFER_SIZE,
                usage: wgpu::BufferUsageFlags::UNIFORM,
            },
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[
                GpuGlobals::binding(&common.globals.handle),
                GpuMeshPrimitive::binding(&primitives.handle),
            ],
        });

        MeshRenderer {
            pipeline_layout,
            bind_group_layout,
            opaque_pipeline,
            alpha_pipeline,
            indices,
            vertices,
            instances,
            primitives,
            bind_group,
        }
    }

    pub fn submit_batch(&self, batch: &MeshBatch, pass: &mut wgpu::RenderPass) {
        pass.set_pipeline(match batch.blend_mode {
            BlendMode::Alpha => &self.alpha_pipeline,
            BlendMode::None => &self.opaque_pipeline,
        });
        pass.set_bind_group(0, &self.bind_group);
        pass.set_index_buffer(&self.indices.handle, 0);
        pass.set_vertex_buffers(&[
            (&self.vertices.handle, 0),
            (&self.instances.handle, 0),
        ]);
        pass.draw_indexed(batch.indices.clone(), batch.base_vertex, batch.instances.clone());
    }

    pub fn upload_mesh(pipe: &mut GpuDataPipe, mesh: CpuMeshSlice) -> Result<MeshOffsets, AllocError> {
        Ok(MeshOffsets {
            vertices: write_to_pipe(pipe, BufferId::MeshVertices, mesh.vertices)?.1.range,
            indices: write_to_pipe(pipe, BufferId::MeshIndices, mesh.indices)?.1.range,
            default_primitives: write_to_pipe(pipe, BufferId::MeshPrimitives, mesh.default_primitives)?.1.range,
        })
    }

    pub fn upload_instances(pipe: &mut GpuDataPipe, mesh: &MeshOffsets, instances: &[GpuMeshInstance]) -> Result<MeshBatch, AllocError> {
        let range = write_to_pipe(pipe, BufferId::MeshInstances, instances)?;
        Ok(MeshBatch {
            indices: mesh.indices.clone(),
            instances: range.1.range,
            blend_mode: BlendMode::None,
            base_vertex: mesh.vertices.start as i32,
        })
    }
}

pub struct CpuMeshSlice<'l> {
    pub vertices: &'l[GpuMeshVertex],
    pub indices: &'l[u16],
    pub default_primitives: &'l[GpuMeshPrimitive],
}

use std::ops::Range;
pub struct MeshOffsets {
    pub vertices: Range<u32>,
    pub indices: Range<u32>,
    pub default_primitives: Range<u32>,
}
