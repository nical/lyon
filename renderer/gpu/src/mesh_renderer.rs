use crate::{RenderTargetState, BlendMode, Registry, CommonResources};
use crate::transform2d::GpuTransform2D;
use crate::bindings;
use crate::gpu_data::*;
use glue::*;

#[derive(Copy, Clone, Debug)]
pub struct MeshInstance {
    pub sub_mesh_offset: u16,
    pub transform_id: TransformIndex,
    pub user_data: u16,
    pub z: u32,
}

impl MeshInstance {
    pub fn pack(&self) -> GpuInstance {
        GpuInstance([
            self.sub_mesh_offset as u32,
            self.transform_id as u32,
            self.user_data as u32,
            self.z
        ])
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SubMeshData {
    pub transform_id: TransformIndex,
    pub src_color_id: ImageSourceIndex,
    pub dest_color_rect: units::LocalBox,
    pub opacity: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GpuSubMesh {
    pub dest_color_rect: [f32; 4],
    pub ids: [u32; 3],
    // TODO: can compress to a single byte if need be.
    pub opacity: f32,
}

impl SubMeshData {
    pub fn pack(&self) -> GpuSubMesh {
        GpuSubMesh {
            dest_color_rect: [
                self.dest_color_rect.min.x,
                self.dest_color_rect.min.y,
                self.dest_color_rect.max.x,
                self.dest_color_rect.max.y,
            ],
            ids: [
                self.transform_id as u32,
                self.src_color_id as u32,
                0,
            ],
            opacity: self.opacity,
        }
    }
}

impl GpuSubMesh {
    pub const BINDING: u32 = bindings::SUB_MESHES;
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
            label: Some("sub-meshes"),
            size: count * Self::SIZE,
            usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        }        
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GpuMeshVertex {
    pub x: f32,
    pub y: f32,
    pub sub_mesh: u16,
}

impl GpuMeshVertex {
    pub const SIZE: u64 = std::mem::size_of::<Self>() as u64;

    pub const VERTEX_ATTRIBUTES: &'static [wgpu::VertexAttributeDescriptor] = &[
        wgpu::VertexAttributeDescriptor {
            offset: 0,
            format: wgpu::VertexFormat::Float2,
            shader_location: bindings::A_POSITION,
        },
        wgpu::VertexAttributeDescriptor {
            offset: 8,
            format: wgpu::VertexFormat::Uint4,
            shader_location: bindings::A_SUB_MESH,
        },
    ];

    pub fn vertex_buffer_descriptor() -> wgpu::VertexBufferDescriptor<'static> {
        wgpu::VertexBufferDescriptor {
            stride: Self::SIZE,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: Self::VERTEX_ATTRIBUTES,
        }
    }

    pub fn buffer_descriptor(count: u32) -> wgpu::BufferDescriptor<'static> {
        wgpu::BufferDescriptor {
            label: Some("mesh vertices"),
            size: count as u64 * Self::SIZE,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        }        
    }
}

pub struct MeshRenderer {
    pub vbo_kind: BufferKind,
    pub ibo_kind: BufferKind,
    pub sub_meshes: BufferKind,
    pub opaque_pipeline_key: PipelineKey,
    pub alpha_pipeline_key: PipelineKey,
    pub pipeline_layout: wgpu::PipelineLayout,
    pub mesh_bind_group_layout: BindGroupLayoutId,
}

impl MeshRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, common: &CommonResources, resources: &mut Registry) -> Self {

        let sub_meshes = resources.register_buffer_kind(GpuSubMesh::buffer_descriptor(512));
        let mesh_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("mesh group layout"),
                entries: &[
                    GpuSubMesh::bind_group_layout_entry(),
                ]
            }
        );
        let mesh_bind_group_layout = resources.register_bind_group_layout(mesh_bind_group_layout);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&"mesh layout"),
            bind_group_layouts: &[
                &resources[common.base_bind_group_layout],
                &resources[common.textures_bind_group_layout],
                &resources[mesh_bind_group_layout],
            ],
            push_constant_ranges: &[],
        });

        let vbo_kind = resources.register_buffer_kind(wgpu::BufferDescriptor {
            label: Some("mesh vbo"),
            size: std::mem::size_of::<GpuMeshVertex>() as u64 * 16384,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let ibo_kind = resources.register_buffer_kind(wgpu::BufferDescriptor {
            label: Some("mesh ibo"),
            size: std::mem::size_of::<u32>() as u64 * 16384,
            usage: wgpu::BufferUsage::INDEX | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let vs: wgpu::ShaderModuleSource = wgpu::include_spirv!("./../shaders/mesh.vert.spv");
        let fs: wgpu::ShaderModuleSource = wgpu::include_spirv!("./../shaders/mesh.frag.spv");
        let vs_module = device.create_shader_module(vs);
        let fs_module = device.create_shader_module(fs);

        let opaque_pipeline_kind = resources.add_render_pipeline_kind();
        let alpha_pipeline_kind = resources.add_render_pipeline_kind();

        let opaque_pipeline_key = opaque_pipeline_kind.with_no_feature();
        let alpha_pipeline_key = alpha_pipeline_kind.with_no_feature();

        let vertex_buffers = &[
            GpuInstance::vertex_buffer_descriptor(),
            GpuMeshVertex::vertex_buffer_descriptor(),
        ];
        let opaque_target = RenderTargetState::opaque_pass_with_depth_test();
        let opaque_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: Some(&"mesh opaque pipeline"),
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
                index_format: wgpu::IndexFormat::Uint32,
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
        resources.add_render_pipeline(opaque_pipeline_key, opaque_pipeline);

        let alpha_target = RenderTargetState::blend_pass(BlendMode::Alpha);
        //let alpha_target = RenderTargetState::blend_pass_with_depth_test(BlendMode::Alpha);
        let alpha_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: Some(&"mesh alpha pipeline"),
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
                index_format: wgpu::IndexFormat::Uint32,
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
        resources.add_render_pipeline(alpha_pipeline_key, alpha_pipeline);

        MeshRenderer {
            pipeline_layout,
            opaque_pipeline_key,
            alpha_pipeline_key,
            mesh_bind_group_layout,
            vbo_kind,
            ibo_kind,
            sub_meshes,
        }
    }
}
