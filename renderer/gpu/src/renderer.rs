use crate::QuadRenderer;
use crate::gpu_data::*;
use glue::*;
use std::ops::Index;
use std::collections::HashMap;

pub struct SharedResources {
    pub bind_groups: BindGroupRegistry,
    pub buffers: BufferRegistry,
    pub textures: TextureRegistry,

    pub globals: wgpu::Buffer,

    // TODO: use texture and buffer caches.
    pub transforms: wgpu::Buffer,
    pub rects: wgpu::Buffer,
    pub prim_data: wgpu::Buffer,
    pub image_sources: wgpu::Buffer,
    pub instances: wgpu::Buffer,
    pub default_sampler: wgpu::Sampler,
    pub mask_texture_id: TextureId,
    pub color_atlas_texture_id: TextureId,
}

impl Index<TextureId> for SharedResources {
    type Output = TextureEntry;
    fn index(&self, id: TextureId) -> &TextureEntry {
        self.textures.get(id).unwrap()
    }
}

impl Index<BufferId> for SharedResources {
    type Output = wgpu::Buffer;
    fn index(&self, id: BufferId) -> &wgpu::Buffer {
        self.buffers.get(id).unwrap()
    }
}

impl Index<BindGroupId> for SharedResources {
    type Output = wgpu::BindGroup;
    fn index(&self, id: BindGroupId) -> &wgpu::BindGroup {
        self.bind_groups.get_bind_group(id)
    }
}

pub struct Renderer {
    pub quads: QuadRenderer,
    pub resources: SharedResources,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {

        let mut textures = TextureRegistry::new();
        let bind_groups = BindGroupRegistry::new();
        let buffers = BufferRegistry::new();

        let globals = device.create_buffer(&GpuGlobals::buffer_descriptor());
        let prim_data = device.create_buffer(&GpuPrimitiveData::buffer_descriptor());
        let rects = device.create_buffer(&GpuPrimitiveRects::buffer_descriptor());
        let transforms = device.create_buffer(&GpuTransform2D::buffer_descriptor());
        let image_sources = device.create_buffer(&GpuImageSource::buffer_descriptor());
        let instances = device.create_buffer(&GpuInstance::buffer_descriptor(16384));

        let color_atlas_texture_kind = TextureKindId(0);
        let mask_texture_kind = TextureKindId(1);

        textures.register_texture_kind(mask_texture_kind, wgpu::TextureDescriptor {
            label: Some("Mask texture"),
            size: U8AlphaMask::SIZE,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            mip_level_count: 1,
            sample_count: 1,
        });

        textures.register_texture_kind(color_atlas_texture_kind, wgpu::TextureDescriptor {
            label: Some("Atlas texture"),
            size: ColorAtlasTexture::SIZE,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            mip_level_count: 1,
            sample_count: 1,
        });

        let mask_texture_id = mask_texture_kind.texture_id(ResourceId(0));
        let color_atlas_texture_id = color_atlas_texture_kind.texture_id(ResourceId(0));

        textures.allocate_texture(device, mask_texture_id);
        textures.allocate_texture(device, color_atlas_texture_id);

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

        let mut resources = SharedResources {
            globals,
            transforms,
            rects,
            prim_data,
            image_sources,
            instances,
            default_sampler,

            mask_texture_id,
            color_atlas_texture_id,

            bind_groups,
            buffers,
            textures,
        };

        Renderer {
            quads: QuadRenderer::new(device, queue, &mut resources),
            resources,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BindGroupInput {
    Buffer(BufferId),
    Texture(TextureId),
}


pub struct BindGroupRegistry {
    bind_groups: HashMap<BindGroupId, wgpu::BindGroup>,
    layouts: Vec<wgpu::BindGroupLayout>,
}

impl BindGroupRegistry {
    pub fn new() -> Self {
        BindGroupRegistry {
            bind_groups: HashMap::new(),
            layouts: Vec::new(),
        }
    }

    pub fn register_bind_group_layout(&mut self, layout: wgpu::BindGroupLayout) -> BindGroupLayoutId {
        let id = BindGroupLayoutId(ResourceId(self.layouts.len() as u16));
        self.layouts.push(layout);

        id
    }

    pub fn add_bind_group(&mut self, id: BindGroupId, bind_group: wgpu::BindGroup) {
        self.bind_groups.insert(id, bind_group);
    }

    pub fn get_bind_group(&self, id: BindGroupId) -> &wgpu::BindGroup {
        self.bind_groups.get(&id).unwrap()
    }
}

pub struct BufferRegistry {
    descriptors: HashMap<BufferKindId, wgpu::BufferDescriptor<'static>>,
    buffers: HashMap<BufferId, wgpu::Buffer>,
}

impl BufferRegistry {
    pub fn new() -> Self {
        BufferRegistry {
            descriptors: HashMap::new(),
            buffers: HashMap::new(),
        }
    }

    pub fn register_buffer_kind(&mut self, id: BufferKindId, descriptor: wgpu::BufferDescriptor<'static>) {
        self.descriptors.insert(id, descriptor);
    }

    pub fn allocate_buffer(&mut self, device: &wgpu::Device, id: BufferId) {
        let descriptor = self.descriptors.get(&id.kind).unwrap();
        let buffer = device.create_buffer(descriptor);
        self.buffers.insert(id, buffer);
    }

    pub fn deallocate_buffer(&mut self, id: BufferId) {
        self.buffers.remove(&id);
    }

    pub fn get(&self, id: BufferId) -> Option<&wgpu::Buffer> {
        self.buffers.get(&id)
    }
}

pub struct TextureEntry {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

pub struct TextureRegistry {
    descriptors: HashMap<TextureKindId, wgpu::TextureDescriptor<'static>>,
    textures: HashMap<TextureId, TextureEntry>,
}

impl TextureRegistry {
    pub fn new() -> Self {
        TextureRegistry {
            descriptors: HashMap::new(),
            textures: HashMap::new(),
        }
    }

    pub fn register_texture_kind(&mut self, id: TextureKindId, descriptor: wgpu::TextureDescriptor<'static>) {
        self.descriptors.insert(id, descriptor);
    }

    pub fn allocate_texture(&mut self, device: &wgpu::Device, id: TextureId) {
        let descriptor = self.descriptors.get(&id.kind).unwrap();
        let texture = device.create_texture(descriptor);
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: descriptor.label,
            format: Some(descriptor.format),
            dimension: None,
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });
        self.textures.insert(id, TextureEntry { texture, view });
    }

    pub fn deallocate_texture(&mut self, id: TextureId) {
        self.textures.remove(&id);
    }

    pub fn get(&self, id: TextureId) -> Option<&TextureEntry> {
        self.textures.get(&id)
    }
}

