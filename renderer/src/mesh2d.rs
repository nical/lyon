use crate::{GpuData, GpuGlobals, RenderTargetState, BlendMode, GpuColor};


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

pub struct MeshRenderer {
    pub opaque_pipeline: wgpu::RenderPipeline,
    pub alpha_pipeline: wgpu::RenderPipeline,

    pub indices: wgpu::Buffer,
    pub max_indices: usize,

    pub vertices: wgpu::Buffer,
    pub max_vertices: usize,

    pub instances: wgpu::Buffer,
    pub max_instances: usize,

    pub primitives: wgpu::Buffer,

    pub pipeline_layout: wgpu::PipelineLayout,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl MeshRenderer {
    pub fn new(device: &wgpu::Device, globals: &wgpu::Buffer) -> Self {
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

        let max_instances = 512;
        let instances = device.create_buffer(&wgpu::BufferDescriptor {
            size: (max_instances * std::mem::size_of::<GpuMeshInstance>()) as u32,
            usage: wgpu::BufferUsageFlags::VERTEX,
        });

        let default_vbo_size = 4096 * 16;
        let max_vertices = default_vbo_size / std::mem::size_of::<GpuMeshVertex>();
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            size: default_vbo_size as u32,
            usage: wgpu::BufferUsageFlags::VERTEX,
        });

        let default_ibo_size = 4096 * 4;
        let max_indices = default_ibo_size / std::mem::size_of::<u16>();
        let indices = device.create_buffer(&wgpu::BufferDescriptor {
            size: default_ibo_size as u32,
            usage: wgpu::BufferUsageFlags::VERTEX,
        });

        let primitives = device.create_buffer(&wgpu::BufferDescriptor {
            size: GpuMeshPrimitive::BUFFER_SIZE,
            usage: wgpu::BufferUsageFlags::UNIFORM,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[
                GpuGlobals::binding(globals),
                GpuMeshPrimitive::binding(&primitives),
            ],
        });

        MeshRenderer {
            pipeline_layout,
            bind_group_layout,
            opaque_pipeline,
            alpha_pipeline,
            indices,
            max_indices,
            vertices,
            max_vertices,
            instances,
            max_instances,
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
        pass.set_index_buffer(&self.indices, 0);
        pass.set_vertex_buffers(&[
            (&self.vertices, 0),
            (&self.instances, 0),
        ]);
        pass.draw_indexed(batch.indices.clone(), batch.base_vertex, batch.instances.clone());
    }
}
