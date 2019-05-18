use crate::{GpuData, GpuGlobals, GpuColor, RenderTargetState, BlendMode};


#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GpuQuad {
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub z: u32,
    pub color: GpuColor,
}

unsafe impl GpuData for GpuQuad {}

impl GpuQuad {
    pub const VERTEX_ATTRIBUTES: &'static [wgpu::VertexAttributeDescriptor] = &[
        wgpu::VertexAttributeDescriptor {
            offset: 0,
            format: wgpu::VertexFormat::Float2,
            attribute_index: 0,
        },
        wgpu::VertexAttributeDescriptor {
            offset: 8,
            format: wgpu::VertexFormat::Float2,
            attribute_index: 1,
        },
        wgpu::VertexAttributeDescriptor {
            offset: 16,
            format: wgpu::VertexFormat::Uint,
            attribute_index: 2,
        },
        wgpu::VertexAttributeDescriptor {
            offset: 20,
            format: wgpu::VertexFormat::Uint,
            attribute_index: 3,
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
pub struct QuadBatch {
    pub blend_mode: BlendMode,
    pub instances: std::ops::Range<u32>, 
}

pub struct QuadRenderer {
    pub opaque_pipeline: wgpu::RenderPipeline,
    pub alpha_pipeline: wgpu::RenderPipeline,
    pub index_buffer: wgpu::Buffer,
    pub instances: wgpu::Buffer,
    pub max_instances: usize,
    pub pipeline_layout: wgpu::PipelineLayout,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl QuadRenderer {
    pub fn new(device: &wgpu::Device, globals: &wgpu::Buffer) -> Self {
        let bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    GpuGlobals::bind_group_layout_binding(),
                ]
            }
        );

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
        });

        let vs_bytes = include_bytes!("./../shaders/quad.vert.spv");
        let fs_bytes = include_bytes!("./../shaders/quad.frag.spv");
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
                GpuQuad::vertex_buffer_descriptor(),
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
                GpuQuad::vertex_buffer_descriptor(),
            ],
            sample_count: 1,
        };

        let alpha_pipeline = device.create_render_pipeline(&alpha_pipeline_descriptor);

        let indices: &[u16] = &[0, 1, 2, 0, 2, 3];
        let index_buffer = device
            .create_buffer_mapped(indices.len(), wgpu::BufferUsageFlags::INDEX)
            .fill_from_slice(indices);

        let max_instances = 512;
        let instances = device.create_buffer(&wgpu::BufferDescriptor {
            size: (max_instances * std::mem::size_of::<GpuQuad>()) as u32,
            usage: wgpu::BufferUsageFlags::VERTEX,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: globals,
                        range: 0..(std::mem::size_of::<GpuGlobals>() as u32),
                    },
                },
            ],
        });

        QuadRenderer {
            pipeline_layout,
            bind_group_layout,
            opaque_pipeline,
            alpha_pipeline,
            index_buffer,
            instances,
            max_instances,
            bind_group,
        }
    }

    pub fn submit_batch(&self, batch: &QuadBatch, pass: &mut wgpu::RenderPass) {
        pass.set_pipeline(match batch.blend_mode {
            BlendMode::Alpha => &self.alpha_pipeline,
            BlendMode::None => &self.opaque_pipeline,
        });
        pass.set_bind_group(0, &self.bind_group);
        pass.set_index_buffer(&self.index_buffer, 0);
        pass.set_vertex_buffers(&[(&self.instances, 0)]);
        pass.draw_indexed(0..6, 0, batch.instances.clone());
    }

}
