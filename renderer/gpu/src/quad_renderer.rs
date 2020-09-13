use crate::{RenderTargetState, BlendMode, SharedResources};
use crate::bindings;
use crate::gpu_data::*;
use glue::{BindGroupId, BindGroupLayoutId, ResourceId};

pub struct QuadRenderer {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub opaque_pipeline: wgpu::RenderPipeline,
    pub alpha_pipeline: wgpu::RenderPipeline,
    pub pipeline_layout: wgpu::PipelineLayout,

    pub bind_group_layout_id: BindGroupLayoutId,
    pub textures_bind_group_layout_id: BindGroupLayoutId,
    // TODO: generate bind groups lazily
    pub bind_group_id: BindGroupId,
    pub textures_bind_group_id: BindGroupId,
}

impl QuadRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, resources: &mut SharedResources) -> Self {

        let bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("quad bind group layout"),
                entries: &[
                    GpuGlobals::bind_group_layout_entry(),
                    GpuPrimitiveRects::bind_group_layout_entry(),
                    GpuPrimitiveData::bind_group_layout_entry(),
                    GpuTransform2D::bind_group_layout_entry(),
                    GpuImageSource::bind_group_layout_entry(),
                ]
            }
        );


        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("quad bind group"),
            layout: &bind_group_layout,
            entries: &[
                GpuGlobals::bind_group_entry(&resources.globals),
                GpuPrimitiveRects::bind_group_entry(&resources.rects),
                GpuPrimitiveData::bind_group_entry(&resources.prim_data),
                GpuTransform2D::bind_group_entry(&resources.transforms),
                GpuImageSource::bind_group_entry(&resources.image_sources),
            ],
        });

        let textures_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("quad input textures layout"),
                entries: &[
                    ColorAtlasTexture::bind_group_layout_entry(),
                    U8AlphaMask::bind_group_layout_entry(),
                    DefaultSampler::bind_group_layout_entry(),
                ]
            }
        );

        let textures_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("quad textures bind group"),
            layout: &textures_bind_group_layout,
            entries: &[
                ColorAtlasTexture::bind_group_entry(&resources[resources.color_atlas_texture_id].view),
                U8AlphaMask::bind_group_entry(&resources[resources.mask_texture_id].view),
                DefaultSampler::bind_group_entry(&resources.default_sampler),
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&"quad layout"),
            bind_group_layouts: &[&bind_group_layout, &textures_bind_group_layout],
            push_constant_ranges: &[],
        });

        let bind_group_layout_id = resources.bind_groups.register_bind_group_layout(bind_group_layout);
        let textures_bind_group_layout_id = resources.bind_groups.register_bind_group_layout(textures_bind_group_layout);

        let bind_group_id = BindGroupId(ResourceId(0)); // TODO
        resources.bind_groups.add_bind_group(bind_group_id, bind_group);

        let textures_bind_group_id = BindGroupId(ResourceId(1));
        resources.bind_groups.add_bind_group(textures_bind_group_id, textures_bind_group);

        let vs: wgpu::ShaderModuleSource = wgpu::include_spirv!("./../shaders/quad.vert.spv");
        let fs: wgpu::ShaderModuleSource = wgpu::include_spirv!("./../shaders/quad.frag.spv");
        let vs_module = device.create_shader_module(vs);
        let fs_module = device.create_shader_module(fs);

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad vbo"),
            size: std::mem::size_of::<[f32; 2]>() as u64 * 4,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_buffer, 0, pipe::as_bytes(&[
            [0.0f32, 0.0],
            [1.0f32, 0.0],
            [1.0f32, 1.0],
            [0.0f32, 1.0],
        ]));

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad ibo"),
            size: std::mem::size_of::<u16>() as u64 * 6,
            usage: wgpu::BufferUsage::INDEX | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&index_buffer, 0, pipe::as_bytes(&[0u16, 1, 2, 0, 3, 2]));

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

        QuadRenderer {
            pipeline_layout,
            opaque_pipeline,
            alpha_pipeline,
            index_buffer,
            vertex_buffer,

            bind_group_layout_id,
            textures_bind_group_layout_id,

            bind_group_id,
            textures_bind_group_id,
        }
    }
}
