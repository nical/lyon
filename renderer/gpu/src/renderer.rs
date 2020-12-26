use crate::transform2d::GpuTransform2D;
use crate::gpu_data::*;
use crate::registry::Registry;
use crate::shaders::*;
use crate::batching::BatchDescriptor;
use glue::*;

use pipe::WritableMemory;


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

    pub fn submit_batch<'l>(
        &mut self,
        pass: &mut wgpu::RenderPass<'l>,
        registry: &'l Registry,
        batch: &BatchDescriptor,
        instance_range: std::ops::Range<u32>,
    ) {
        if self.pipeline != Some(batch.pipeline) {
            let (pipeline, real_key) = registry.get_compatible_render_pipeline(batch.pipeline).unwrap();
            if self.pipeline != Some(real_key) {
                pass.set_pipeline(pipeline);
            }
            self.pipeline = Some(real_key);
        }

        if self.ibo != Some(batch.ibo) {
            pass.set_index_buffer(registry[batch.ibo].slice(..));
            self.ibo = Some(batch.ibo);
        }

        for i in 0..4 {
            if let Some(vbo) = batch.vbos[i] {
                if self.vbos[i] != Some(vbo) {
                    pass.set_vertex_buffer(i as u32, registry[vbo].slice(..));
                    self.vbos[i] = Some(vbo);
                }
            }
        }

        for i in 0..4 {
            if let Some(bind_group) = batch.bind_groups[i] {
                if self.bind_groups[i] != Some(bind_group) {
                    pass.set_bind_group(i as u32, &registry[bind_group], &[]);
                    self.bind_groups[i] = Some(bind_group);
                }
            }
        }

        pass.draw_indexed(batch.index_range.0..batch.index_range.1, batch.base_vertex, instance_range.clone());
    }
}

pub trait System {
    fn update(&mut self);
}

struct SystemEntry {
    type_id: std::any::TypeId,
    sys: Box<System>,
}

pub struct Systems {
    systems: Vec<SystemEntry>,
    lookup: std::collections::HashMap<std::any::TypeId, usize>,
}


/// ID of a staging buffer only valid during the current frame.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StagingBufferId(u16);

struct StagingBuffer {
    buffer: wgpu::Buffer,
    size: u64,
}

struct MappedStagingBuffer {
    buffer: wgpu::Buffer,
    size: u64,
    // We have to lie about the static lifetime here and rely on WritableMemory to dynamically
    // count live references to the mapped memory.
    view: wgpu::BufferViewMut<'static>,
    writable: WritableMemory<'static>,
}

impl StagingBuffer {
    fn new(size: u64, device: &wgpu::Device) -> Self {
        let descriptor = wgpu::BufferDescriptor {
            label: Some("Staging"),
            size,
            usage: wgpu::BufferUsage::MAP_WRITE | wgpu::BufferUsage::COPY_SRC,
            mapped_at_creation: true,
        };

        let buffer = device.create_buffer(&descriptor);

        StagingBuffer {
            buffer,
            size,
        }
    }

    fn map(mut self) -> MappedStagingBuffer {
        let mut view: wgpu::BufferViewMut<'static> = unsafe {
            std::mem::transmute(self.buffer.slice(..).get_mapped_range_mut())
        };

        let mem: &'static mut [u8] = unsafe { std::mem::transmute(&mut (*view)) };
        let writable = WritableMemory::new(mem, 0);

        MappedStagingBuffer {
            buffer: self.buffer,
            view,
            writable,
            size: self.size,
        }
    }

}

impl MappedStagingBuffer {
    fn unmap(mut self) -> StagingBuffer {
        assert!(!self.writable.has_writers());
        StagingBuffer {
            buffer: self.buffer,
            size: self.size,
        }
    }
}

pub struct StagingBuffers {
    // Staging buffers that are mapped during the current frame.
    current_frame: Vec<MappedStagingBuffer>,
    // Unmapped staging buffers that are ready to be mapped.
    pool: Vec<StagingBuffer>,
    // Staging buffers that were map in a recent previous frame and aren't ready to
    // be mapped again yet.
    in_flight: Vec<(FrameStamp, StagingBuffer)>,
    size: u64,
}

impl StagingBuffers {
    pub fn new(size: u64) -> Self {
        StagingBuffers {
            current_frame: Vec::new(),
            pool: Vec::new(),
            in_flight: Vec::new(),
            size,
        }
    }

    pub fn begin_frame(&mut self, current_frame: FrameStamp) {
        let num_frames = 2;
        if current_frame.0 > num_frames {
            let recycled_frame = current_frame.0 - num_frames;
            let mut idx = 0;
            while idx < self.in_flight.len() {
                if (self.in_flight[idx].0).0 <  recycled_frame {
                    self.pool.push(self.in_flight.swap_remove(idx).1);
                } else {
                    idx += 1;
                }
            }
        }
    }

    pub fn end_frame(&mut self, current_frame: FrameStamp) {
        while let Some(mut buffer) = self.current_frame.pop() {
            self.in_flight.push((current_frame, buffer.unmap()));
        }
    }

    pub fn map_staging_buffer(&mut self, device: &wgpu::Device) -> StagingBufferId {
        let buffer = self.pool.pop().unwrap_or_else(|| { StagingBuffer::new(self.size, device) });

        let id = StagingBufferId(self.current_frame.len() as u16);
        self.current_frame.push(buffer.map());

        id
    }

    pub fn get(&self, id: StagingBufferId) -> &WritableMemory<'static> {
        &self.current_frame[id.0 as usize].writable
    }
}

pub struct GpuPipe {
    staging_buffers: std::sync::Mutex<StagingBuffers>
}

