use crate::{RenderTargetState, BlendMode, Registry, Pipeline, CommonResources};
use crate::transform2d::GpuTransform2D;
use crate::bindings;
use crate::gpu_data::*;
use glue::*;

pub struct QuadRenderer {
    pub vertex_buffer: BufferId,
    pub index_buffer: BufferId,
    pub opaque_pipeline_key: PipelineKey,
    pub alpha_pipeline_key: PipelineKey,
    pub pipeline_layout: wgpu::PipelineLayout,
}


impl QuadRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, common: &CommonResources, resources: &mut Registry) -> Self {

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&"quad layout"),
            bind_group_layouts: &[
                &resources[common.base_bind_group_layout],
                &resources[common.textures_bind_group_layout],
            ],
            push_constant_ranges: &[],
        });

        let vertex_buffer = resources.register_buffer_kind(wgpu::BufferDescriptor {
            label: Some("quad vbo"),
            size: std::mem::size_of::<[f32; 2]>() as u64 * 4,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        }).buffer_id(ResourceIndex(0, 0));

        let index_buffer = resources.register_buffer_kind(wgpu::BufferDescriptor {
            label: Some("quad ibo"),
            size: std::mem::size_of::<u16>() as u64 * 6,
            usage: wgpu::BufferUsage::INDEX | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        }).buffer_id(ResourceIndex(0, 0));

        resources.allocate_buffer(device, vertex_buffer);
        resources.allocate_buffer(device, index_buffer);

        queue.write_buffer(&resources[vertex_buffer], 0, pipe::as_bytes(&[
            [0.0f32, 0.0],
            [1.0f32, 0.0],
            [1.0f32, 1.0],
            [0.0f32, 1.0],
        ]));

        queue.write_buffer(&resources[index_buffer], 0, pipe::as_bytes(&[0u16, 1, 2, 0, 3, 2]));

        let vs: wgpu::ShaderModuleSource = wgpu::include_spirv!("./../shaders/quad.vert.spv");
        let fs: wgpu::ShaderModuleSource = wgpu::include_spirv!("./../shaders/quad.frag.spv");
        let vs_module = device.create_shader_module(vs);
        let fs_module = device.create_shader_module(fs);

        let vbo_desc = wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<[f32; 2]>() as u64,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    format: wgpu::VertexFormat::Float2,
                    shader_location: bindings::A_POSITION,
                },
            ],
        };

        let vertex_buffers = &[
            GpuInstance::vertex_buffer_descriptor(),
            vbo_desc,
        ];
        let opaque_target = RenderTargetState::opaque_pass_with_depth_test();
        let opaque_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: Some(&"quads opaque pipeline"),
            layout: Some(&pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                ..Default::default()
            }),
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers,
            },
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[opaque_target.color],
            depth_stencil_state: opaque_target.depth_stencil, 
            sample_count: 1,
            alpha_to_coverage_enabled: false,
            sample_mask: !0,
        };

        let opaque_pipeline = device.create_render_pipeline(&opaque_pipeline_descriptor);

        let alpha_target = RenderTargetState::blend_pass(BlendMode::Alpha);
        //let alpha_target = RenderTargetState::blend_pass_with_depth_test(BlendMode::Alpha);
        let alpha_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: Some(&"quads blend pipeline"),
            layout: Some(&pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                ..Default::default()
            }),
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers,

            },
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[alpha_target.color],
            depth_stencil_state: alpha_target.depth_stencil, 
            sample_count: 1,
            alpha_to_coverage_enabled: false,
            sample_mask: !0,
        };

        let alpha_pipeline = device.create_render_pipeline(&alpha_pipeline_descriptor);

        let opaque_pipeline_kind = resources.add_render_pipeline_kind(vec![
            Pipeline::from_handle(PipelineFeatures::opaque(), opaque_pipeline),
        ]);
        let alpha_pipeline_kind = resources.add_render_pipeline_kind(vec![
            Pipeline::from_handle(PipelineFeatures::blending(), alpha_pipeline)
        ]);
        let opaque_pipeline_key = opaque_pipeline_kind.with_no_feature();
        let alpha_pipeline_key = alpha_pipeline_kind.with_no_feature();

        QuadRenderer {
            pipeline_layout,
            opaque_pipeline_key,
            alpha_pipeline_key,
            index_buffer,
            vertex_buffer,
        }
    }
}
