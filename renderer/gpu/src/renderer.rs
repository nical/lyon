use crate::transform2d::GpuTransform2D;
use crate::gpu_data::*;
use crate::registry::Registry;
use crate::shaders::*;
use glue::*;


// TODO: don't really want that here.
pub struct CommonResources {
    pub instances: BufferKind,
    pub globals: BufferKind,
    pub transforms: BufferKind,
    pub rects: BufferKind,
    pub image_sources: BufferKind,

    pub mask_texture_kind: TextureKind,
    pub color_atlas_texture_kind: TextureKind,

    pub base_bind_group_layout: BindGroupLayoutId,
    pub textures_bind_group_layout: BindGroupLayoutId,

    pub default_sampler: wgpu::Sampler,
}

pub struct Renderer {
    pub common: CommonResources,
    pub resources: Registry,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {

        let mut resources = Registry::new();

        let globals = resources.register_buffer_kind(GpuGlobals::buffer_descriptor());
        let rects = resources.register_buffer_kind(GpuPrimitiveRects::buffer_descriptor(2048));
        let transforms = resources.register_buffer_kind(GpuTransform2D::buffer_descriptor(512));
        let image_sources = resources.register_buffer_kind(GpuImageSource::buffer_descriptor(512));
        let instances = resources.register_buffer_kind(GpuInstance::buffer_descriptor(16384));

        let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Default sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });


        let color_atlas_texture_kind= resources.register_texture_kind(wgpu::TextureDescriptor {
            label: Some("Atlas texture"),
            size: ColorAtlasTexture::SIZE,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            mip_level_count: 1,
            sample_count: 1,
        });

        let mask_texture_kind = resources.register_texture_kind(wgpu::TextureDescriptor {
            label: Some("Mask texture"),
            size: U8AlphaMask::SIZE,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            mip_level_count: 1,
            sample_count: 1,
        });

        let base_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("quad bind group layout"),
                entries: &[
                    GpuGlobals::bind_group_layout_entry(),
                    GpuPrimitiveRects::bind_group_layout_entry(),
                    GpuTransform2D::bind_group_layout_entry(),
                    GpuImageSource::bind_group_layout_entry(),
                ]
            }
        );

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

        let base_bind_group_layout = resources.register_bind_group_layout(base_bind_group_layout);
        let textures_bind_group_layout = resources.register_bind_group_layout(textures_bind_group_layout);

        let common = CommonResources {
            instances,
            globals,
            transforms,
            rects,
            image_sources,
            default_sampler,

            color_atlas_texture_kind,
            mask_texture_kind,
            base_bind_group_layout,
            textures_bind_group_layout,
        };

        Renderer {
            common,
            resources,
        }
    }
}

pub struct BatchKey {
    pub pipeline: PipelineKey,
    pub ibo: BufferId,
    pub vbos: [Option<BufferId>; 4],
    pub bind_groups: [Option<BindGroupId>; 4],
}

pub struct Batch {
    pub key: BatchKey,
    pub index_range: std::ops::Range<u32>,
    pub base_vertex: i32,
    pub instance_range: std::ops::Range<u32>,
}

pub struct DrawState {
    pipeline: Option<PipelineKey>,
    ibo: Option<BufferId>,
    bind_groups: [Option<BindGroupId>; 4],
    vbos: [Option<BufferId>; 4],
}

impl DrawState {
    pub fn new() -> Self {
        DrawState {
            pipeline: None,
            bind_groups: [None; 4],
            vbos: [None; 4],
            ibo: None,
        }
    }

    pub fn submit_batch<'l>(&mut self, pass: &mut wgpu::RenderPass<'l>, registry: &'l Registry, batch: &Batch) {
        if self.pipeline != Some(batch.key.pipeline) {
            let (pipeline, real_key) = registry.get_compatible_render_pipeline(batch.key.pipeline).unwrap();
            if self.pipeline != Some(real_key) {
                pass.set_pipeline(pipeline);
            }
            self.pipeline = Some(real_key);
        }

        if self.ibo != Some(batch.key.ibo) {
            pass.set_index_buffer(registry[batch.key.ibo].slice(..));
            self.ibo = Some(batch.key.ibo);
        }

        for i in 0..4 {
            if let Some(vbo) = batch.key.vbos[i] {
                if self.vbos[i] != Some(vbo) {
                    pass.set_vertex_buffer(i as u32, registry[vbo].slice(..));
                    self.vbos[i] = Some(vbo);
                }
            }
        }

        for i in 0..4 {
            if let Some(bind_group) = batch.key.bind_groups[i] {
                if self.bind_groups[i] != Some(bind_group) {
                    pass.set_bind_group(i as u32, &registry[bind_group], &[]);
                    self.bind_groups[i] = Some(bind_group);
                }
            }
        }

        pass.draw_indexed(batch.index_range.clone(), batch.base_vertex, batch.instance_range.clone());
    }
}
