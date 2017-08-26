use gfx;
use gfx_device_gl;

use std::collections::HashMap;

gfx_defines!{
    constant Globals {
        resolution: [f32; 2] = "u_resolution",
    }

    vertex GpuFillVertex {
        position: [f32; 2] = "a_position",
        normal: [f32; 2] = "a_normal",
        prim_id: i32 = "a_prim_id",
    }

    vertex GpuStrokeVertex {
        position: [f32; 2] = "a_position",
        normal: [f32; 2] = "a_normal",
        advancement: f32 = "a_advancement",
        prim_id: i32 = "a_prim_id",
    }

    pipeline fill_pipeline {
        vbo: gfx::VertexBuffer<GpuFillVertex> = (),
        out_color: gfx::RenderTarget<ColorFormat> = "out_color",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
        constants: gfx::ConstantBuffer<Globals> = "Globals",
        gpu_data: gfx::TextureSampler<[f32; 4]> = "gpu_data",
    }

    pipeline stroke_pipeline {
        vbo: gfx::VertexBuffer<GpuStrokeVertex> = (),
        out_color: gfx::RenderTarget<ColorFormat> = "out_color",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
        constants: gfx::ConstantBuffer<Globals> = "Globals",
        gpu_data: gfx::TextureSampler<[f32; 4]> = "gpu_data",
    }
}


pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;
pub type DataTexFormat = (gfx::format::R32_G32_B32_A32, gfx::format::Float);

pub type CmdEncoder = gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>;
pub type BufferObject<T> = gfx::handle::Buffer<gfx_device_gl::Resources, T>;
pub type Vbo<T> = gfx::handle::Buffer<gfx_device_gl::Resources, T>;
pub type Ibo = gfx::IndexBuffer<gfx_device_gl::Resources>;
pub type Pso<T> = gfx::PipelineState<gfx_device_gl::Resources, T>;
pub type IndexSlice = gfx::Slice<gfx_device_gl::Resources>;
pub type ColorTarget = gfx::handle::RenderTargetView<gfx_device_gl::Resources,
                                                     (gfx::format::R8_G8_B8_A8,
                                                      gfx::format::Unorm)>;
pub type DepthTarget = gfx::handle::DepthStencilView<gfx_device_gl::Resources,
                                                     (gfx::format::D24_S8, gfx::format::Unorm)>;
pub type GlDevice = gfx_device_gl::Device;
pub type GlFactory = gfx_device_gl::Factory;
pub type GlRgbaTexture = gfx::handle::Texture<gfx_device_gl::Resources, ColorFormat>;
pub type GlDataTexture = gfx::handle::Texture<gfx_device_gl::Resources, DataTexFormat>;

pub struct RenderTarget {
    pub color: ColorTarget,
    pub depth: DepthTarget,
}

pub struct GpuGeometry<T> {
    pub vbo: Vbo<T>,
    pub ibo: IndexSlice,
}

//use gfx::Factory;
use gfx::traits::FactoryExt;
use vector_image_renderer::{DrawCmd, RenderPassOptions, GeometryBuilder, GeometryId, Device};
use gpu_data::{GpuData, GpuAddressRange, GpuAddress, GpuOffset};

pub struct GfxDevice {
    pub device: GlDevice,
    pub factory: GlFactory,

    alloc: GpuOffset,

    fill_geom: HashMap<GeometryId, GpuGeometry<GpuFillVertex>>,
    stroke_geom: HashMap<GeometryId, GpuGeometry<GpuStrokeVertex>>,
    _data_texture: GlDataTexture,
}

impl Device for GfxDevice {
    fn allocate_gpu_data(&mut self, size: u32) -> GpuAddressRange {
        let start = GpuAddress::global(self.alloc);
        self.alloc = self.alloc + GpuOffset(size);
        let end = GpuAddress::global(self.alloc);
        GpuAddressRange { start, end }
    }

    fn set_gpu_data(&mut self, range: GpuAddressRange, _data: &GpuData) {
        assert!(self.alloc.as_u32() >= range.end.offset().as_u32());

        // TODO
    }

    fn submit_geometry(&mut self, geom: GeometryBuilder) {
        let (fill_vbo, fill_range) = self.factory.create_vertex_buffer_with_slice(
            &geom.fill().vertices[..],
            &geom.fill().indices[..]
        );
        self.fill_geom.insert(geom.id(), GpuGeometry {
            vbo: fill_vbo,
            ibo: fill_range,
        });

        let (stroke_vbo, stroke_range) = self.factory.create_vertex_buffer_with_slice(
            &geom.stroke().vertices[..],
            &geom.stroke().indices[..]
        );
        self.stroke_geom.insert(geom.id(), GpuGeometry {
            vbo: stroke_vbo,
            ibo: stroke_range,
        });
    }

    fn render_pass(&mut self, cmds: &[DrawCmd], options: &RenderPassOptions) {
        println!("{:?}", options);
        for cmd in cmds {
            println!("{:?}", cmd);
        }
        // TODO
    }
}
