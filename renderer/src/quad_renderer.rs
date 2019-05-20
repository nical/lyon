use crate::{
    GpuData, GpuGlobals, GpuColor, RenderTargetState, BlendMode, BufferId, GpuDataPipe,
    AllocError, write_to_pipe, CommonBuffers, Buffer,
};


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

impl QuadBatch {
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

pub struct QuadRenderer {
    pub opaque_pipeline: wgpu::RenderPipeline,
    pub alpha_pipeline: wgpu::RenderPipeline,
    pub index_buffer: Buffer,
    pub instances: Buffer,
    pub pipeline_layout: wgpu::PipelineLayout,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl QuadRenderer {
    pub fn new(device: &wgpu::Device, common: &CommonBuffers) -> Self {
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

        let index_buffer = Buffer::from_data(
            device,
            wgpu::BufferUsageFlags::INDEX,
            &[0u16, 1, 2, 0, 2, 3],
        );

        let instances = Buffer::new(
            device,
            wgpu::BufferDescriptor {
                size: 4096,
                usage: wgpu::BufferUsageFlags::VERTEX,
            },
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[
                GpuGlobals::binding(&common.globals.handle),
            ],
        });

        QuadRenderer {
            pipeline_layout,
            bind_group_layout,
            opaque_pipeline,
            alpha_pipeline,
            index_buffer,
            instances,
            bind_group,
        }
    }

    pub fn submit_batch(&self, batch: &QuadBatch, pass: &mut wgpu::RenderPass) {
        pass.set_pipeline(match batch.blend_mode {
            BlendMode::Alpha => &self.alpha_pipeline,
            BlendMode::None => &self.opaque_pipeline,
        });
        pass.set_bind_group(0, &self.bind_group);
        pass.set_index_buffer(&self.index_buffer.handle, 0);
        pass.set_vertex_buffers(&[(&self.instances.handle, 0)]);
        pass.draw_indexed(0..6, 0, batch.instances.clone());
    }

    pub fn upload_instances(pipe: &mut GpuDataPipe, instances: &[GpuQuad]) -> Result<QuadBatch, AllocError> {
        let range = write_to_pipe(pipe, BufferId::QuadInstances, instances)?;
        Ok(QuadBatch {
            instances: range.1.range,
            blend_mode: BlendMode::None,
        })
    }
}
