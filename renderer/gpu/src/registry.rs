use std::num::{NonZeroU16, NonZeroU32};
use std::ops::Index;
use std::collections::HashMap;

use glue::*;
use crate::shaders::*;

pub struct TextureEntry {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

pub struct Registry {
    buffer_descriptors: Vec<wgpu::BufferDescriptor<'static>>,
    buffers: HashMap<BufferId, wgpu::Buffer>,

    texture_descriptors: Vec<wgpu::TextureDescriptor<'static>>,
    textures: HashMap<TextureId, TextureEntry>,

    bind_groups: HashMap<BindGroupId, wgpu::BindGroup>,
    bind_group_layouts: Vec<wgpu::BindGroupLayout>,

    pipeline_kinds: HashMap<PipelineKind, Vec<Pipeline>>,
    next_pipeline_key: u32,
}

impl Registry {
    pub fn new() -> Self {
        Registry {
            buffer_descriptors: Vec::new(),
            buffers: HashMap::new(),
            texture_descriptors: Vec::new(),
            textures: HashMap::new(),
            bind_group_layouts: Vec::new(),
            bind_groups: HashMap::new(),
            pipeline_kinds: HashMap::new(),
            next_pipeline_key: 1,

        }
    }

    pub fn register_bind_group_layout(&mut self, layout: wgpu::BindGroupLayout) -> BindGroupLayoutId {
        self.bind_group_layouts.push(layout);

        BindGroupLayoutId(NonZeroU16::new(self.bind_group_layouts.len() as u16).unwrap())
    }

    pub fn add_bind_group(&mut self, id: BindGroupId, bind_group: wgpu::BindGroup) {
        self.bind_groups.insert(id, bind_group);
    }

    pub fn get_bind_group(&self, id: BindGroupId) -> &wgpu::BindGroup {
        self.bind_groups.get(&id).unwrap()
    }

    pub fn get_bind_group_layout(&self, id: BindGroupLayoutId) -> &wgpu::BindGroupLayout {
        &self.bind_group_layouts[id.0.get() as usize - 1]
    }

    pub fn register_buffer_kind(&mut self, descriptor: wgpu::BufferDescriptor<'static>) -> BufferKind {
        self.buffer_descriptors.push(descriptor);

        BufferKind(NonZeroU16::new(self.buffer_descriptors.len() as u16).unwrap())
    }

    pub fn allocate_buffer(&mut self, device: &wgpu::Device, id: BufferId) {
        let descriptor = &self.buffer_descriptors[id.kind.0.get() as usize - 1];
        let buffer = device.create_buffer(descriptor);
        self.buffers.insert(id, buffer);
    }

    pub fn deallocate_buffer(&mut self, id: BufferId) {
        self.buffers.remove(&id);
    }

    pub fn register_texture_kind(&mut self, descriptor: wgpu::TextureDescriptor<'static>) -> TextureKind {
        self.texture_descriptors.push(descriptor);

        TextureKind(NonZeroU16::new(self.texture_descriptors.len() as u16).unwrap())
    }

    pub fn allocate_texture(&mut self, device: &wgpu::Device, id: TextureId) {
        let descriptor = &self.texture_descriptors[id.kind.0.get() as usize - 1];
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

    pub fn get_compatible_render_pipeline(&self, key: PipelineKey) -> Option<(&wgpu::RenderPipeline, PipelineKey)> {
        let kind = self.pipeline_kinds.get(&key.kind)?;

        for p in kind {
            if p.features & key.features == key.features {
                if let Some(handle) = &p.handle {
                    return Some((handle, PipelineKey { features: p.features, .. key }));
                }
            }
        }

        None
    }

    pub fn ensure_compatible_render_pipeline(
        &mut self,
        key: PipelineKey,
        device: &wgpu::Device,
    ) -> bool {
        // TODO: ugly borrow-ck dance.
        let mut pipeline_kinds = std::mem::take(&mut self.pipeline_kinds);
        let mut exists = false;

        if let Some(kind) = pipeline_kinds.get_mut(&key.kind) {
            for p in kind {
                if p.features & key.features == key.features {
                    p.ensure_pipeline(device, self);
                    exists = true;
                    break;
                }
            }
        }

        self.pipeline_kinds = pipeline_kinds;

        exists
    }

    pub fn add_render_pipeline_kind(&mut self, pipelines: Vec<Pipeline>) -> PipelineKind {
        let key = PipelineKind(NonZeroU32::new(self.next_pipeline_key).unwrap());

        self.pipeline_kinds.insert(key, pipelines);

        self.next_pipeline_key += 1;

        key
    }

    pub fn remove_pipeline_kind(&mut self, key: PipelineKind) {
        self.pipeline_kinds.remove(&key);
    }
}

pub struct Pipeline {
    handle: Option<wgpu::RenderPipeline>,
    lazy_init: Option<Box<FnOnce(&wgpu::Device, &Registry) -> wgpu::RenderPipeline>>,
    features: PipelineFeatures,
}

impl Pipeline {
    pub fn from_handle(features: PipelineFeatures, handle: wgpu::RenderPipeline) -> Self {
        Pipeline {
            handle: Some(handle),
            lazy_init: None,
            features,
        }
    }

    pub fn lazily_initialized(features: PipelineFeatures, callback: Box<dyn FnOnce(&wgpu::Device, &Registry) -> wgpu::RenderPipeline>) -> Self {
        Pipeline {
            handle: None,
            lazy_init: Some(callback),
            features,
        }
    }

    fn ensure_pipeline(&mut self, device: &wgpu::Device, resources: &Registry) {
        if self.handle.is_some() {
            return;
        }

        let cb = self.lazy_init.take().unwrap();

        self.handle = Some(cb(device, resources));
    }
}

impl Index<TextureId> for Registry {
    type Output = TextureEntry;
    fn index(&self, id: TextureId) -> &TextureEntry {
        self.textures.get(&id).unwrap()
    }
}

impl Index<BufferId> for Registry {
    type Output = wgpu::Buffer;
    fn index(&self, id: BufferId) -> &wgpu::Buffer {
        self.buffers.get(&id).unwrap()
    }
}

impl Index<BindGroupId> for Registry {
    type Output = wgpu::BindGroup;
    fn index(&self, id: BindGroupId) -> &wgpu::BindGroup {
        self.get_bind_group(id)
    }
}

impl Index<BindGroupLayoutId> for Registry {
    type Output = wgpu::BindGroupLayout;
    fn index(&self, id: BindGroupLayoutId) -> &wgpu::BindGroupLayout {
        self.get_bind_group_layout(id)
    }
}

