use gfx::traits::FactoryExt;
//use gfx_device_gl;
use gfx;

use tessellation;
use tessellation::geometry_builder::VertexConstructor;
use core::math::*;
use buffer::*;
pub use gfx_types::*;
use shaders::*;
use prim_store::{ PrimStore, BufferStore, GeometryStore };

use frame::*;

use std;
//use std::sync::Arc;
use std::collections::HashMap;

pub type OpaquePso = Pso<opaque_pipeline::Meta>;
pub type TransparentPso = Pso<transparent_pipeline::Meta>;

pub type GpuStrokeVertex = Vertex;
pub type GpuFillVertex = Vertex;
pub type GpuStrokePrimitive = PrimData;
pub type GpuFillPrimitive = PrimData;

gfx_defines!{
    constant Globals {
        resolution: [f32; 2] = "u_resolution",
        scroll_offset: [f32; 2] = "u_scroll_offset",
        zoom: f32 = "u_zoom",
    }

    constant GpuTransform {
        transform: [[f32; 4]; 4] = "transform",
    }

    // Per-shape data.
    // It would probably make sense to have different structures for fills and strokes,
    // but using the same struct helps with keeping things simple for now.
    constant PrimData {
        color: [f32; 4] = "color",
        z_index: f32 = "z_index",
        transform_id: i32 = "transform_id",
        width: f32 = "width",
        _padding: f32 = "_padding",
    }

    // Per-vertex data.
    // Again, the same data is used for fill and strokes for simplicity.
    // Ideally this should stay as small as possible.
    vertex Vertex {
        position: [f32; 2] = "a_position",
        normal: [f32; 2] = "a_normal",
        shape_id: i32 = "a_prim_id", // An id pointing to the PrimData struct above.
    }

    pipeline opaque_pipeline {
        vbo: gfx::VertexBuffer<Vertex> = (),
        out_color: gfx::RenderTarget<ColorFormat> = "out_color",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
        constants: gfx::ConstantBuffer<Globals> = "Globals",
        transforms: gfx::ConstantBuffer<GpuTransform> = "u_transforms",
        shape_data: gfx::ConstantBuffer<PrimData> = "u_shape_data",
    }

    pipeline transparent_pipeline {
        vbo: gfx::VertexBuffer<Vertex> = (),
        out_color: gfx::RenderTarget<ColorFormat> = "out_color",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_TEST,
        constants: gfx::ConstantBuffer<Globals> = "Globals",
        transforms: gfx::ConstantBuffer<GpuTransform> = "u_transforms",
        shape_data: gfx::ConstantBuffer<PrimData> = "u_shape_data",
    }
}

pub type TransformId = Id<GpuTransform>;

impl PrimData {
    pub fn new(color: [f32; 4], z_index: f32, transform_id: TransformId) -> PrimData {
        PrimData {
            color: color,
            z_index: z_index,
            transform_id: transform_id.to_i32(),
            width: 1.0,
            _padding: 0.0,
        }
    }
}

impl std::default::Default for PrimData {
    fn default() -> Self { PrimData::new([1.0, 1.0, 1.0, 1.0], 0.0, TransformId::new(0)) }
}

impl std::default::Default for GpuTransform {
    fn default() -> Self {
        GpuTransform { transform: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]}
    }
}

pub type PrimitiveId = Id<PrimData>;

// Implement a vertex constructor.
// The vertex constructor sits between the tessellator and the geometry builder.
// it is called every time a new vertex needs to be added and creates a the vertex
// from the information provided by the tessellator.
//
// This vertex constructor forwards the positions and normals provided by the
// tessellators and add a shape id.
pub struct WithPrimitiveId(pub PrimitiveId);

impl VertexConstructor<tessellation::StrokeVertex, GpuStrokeVertex> for WithPrimitiveId {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex) -> GpuStrokeVertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());
        assert!(!vertex.normal.x.is_nan());
        assert!(!vertex.normal.y.is_nan());
        GpuStrokeVertex {
            position: vertex.position.array(),
            normal: vertex.normal.array(),
            shape_id: self.0.to_i32(),
        }
    }
}

// The fill tessellator does not implement normals yet, so this implementation
// just sets it to [0, 0], for now.
impl VertexConstructor<tessellation::FillVertex, GpuFillVertex> for WithPrimitiveId {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> GpuFillVertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());
        assert!(!vertex.normal.x.is_nan());
        assert!(!vertex.normal.y.is_nan());
        GpuFillVertex {
            position: vertex.position.array(),
            normal: vertex.normal.array(),
            shape_id: self.0.to_i32(),
        }
    }
}

pub enum SurfaceFormat {
    Rgb,
    Rgba,
    Alpha,
    Stencil,
}

pub struct RenderTarget {
    pub color: ColorTarget,
    pub depth: DepthTarget,
}

pub struct Geometry<T> {
    vbo: Vbo<T>,
    ibo: IndexSlice,
}

pub struct Renderer {
    fill_data: GpuStore<GpuFillVertex, PrimData>,
    stroke_data: GpuStore<GpuFillVertex, PrimData>,
    transform_buffers: GpuBufferStore<GpuTransform>,
    render_targets: HashMap<RenderTargetId, RenderTarget>,

    opaque_fill_pso: [OpaquePso; 2],
    opaque_stroke_pso: [OpaquePso; 2],
    transparent_fill_pso: [TransparentPso; 2],
    transparent_stroke_pso: [TransparentPso; 2],

    constants_buffer: BufferObject<Globals>,

    device: GlDevice,
    factory: GlFactory,
}

pub enum InitializationError {
    ShaderCompilation,
    PipelineCreation,
    BufferAllocation,
}

impl Renderer {
    pub fn new(mut config: RendererConfig) -> Result<Self, InitializationError> {

        let shader = if let Ok(program) = config.factory.link_program(
            FILL_VERTEX_SHADER.as_bytes(),
            FILL_FRAGMENT_SHADER.as_bytes(),
        ) { program } else { return Err(InitializationError::ShaderCompilation); };

        let opaque_fill_pso = if let Ok(pso) = config.factory.create_pipeline_from_program(
            &shader,
            gfx::Primitive::TriangleList,
            gfx::state::Rasterizer::new_fill(),
            opaque_pipeline::new()
        ) { pso } else { return Err(InitializationError::PipelineCreation); };
        let opaque_stroke_pso = if let Ok(pso) = config.factory.create_pipeline_from_program(
            &shader,
            gfx::Primitive::TriangleList,
            gfx::state::Rasterizer::new_fill(),
            opaque_pipeline::new()
        ) { pso } else { return Err(InitializationError::PipelineCreation); };
        let transparent_fill_pso = if let Ok(pso) = config.factory.create_pipeline_from_program(
            &shader,
            gfx::Primitive::TriangleList,
            gfx::state::Rasterizer::new_fill(),
            transparent_pipeline::new()
        ) { pso } else { return Err(InitializationError::PipelineCreation); };
        let transparent_stroke_pso = if let Ok(pso) = config.factory.create_pipeline_from_program(
            &shader,
            gfx::Primitive::TriangleList,
            gfx::state::Rasterizer::new_fill(),
            transparent_pipeline::new()
        ) { pso } else { return Err(InitializationError::PipelineCreation); };

        let dbg_opaque_fill_pso;
        let dbg_opaque_stroke_pso;
        let dbg_transparent_fill_pso;
        let dbg_transparent_stroke_pso;
        if config.debug {
            let mut fill_mode = gfx::state::Rasterizer::new_fill();
            fill_mode.method = gfx::state::RasterMethod::Line(1);
            dbg_opaque_fill_pso = if let Ok(pso) = config.factory.create_pipeline_from_program(
                &shader,
                gfx::Primitive::TriangleList,
                fill_mode,
                opaque_pipeline::new()
            ) { pso } else { return Err(InitializationError::PipelineCreation); };
            dbg_opaque_stroke_pso = if let Ok(pso) = config.factory.create_pipeline_from_program(
                &shader,
                gfx::Primitive::TriangleList,
                fill_mode,
                opaque_pipeline::new()
            ) { pso } else { return Err(InitializationError::PipelineCreation); };
            dbg_transparent_fill_pso = if let Ok(pso) = config.factory.create_pipeline_from_program(
                &shader,
                gfx::Primitive::TriangleList,
                fill_mode,
                transparent_pipeline::new()
            ) { pso } else { return Err(InitializationError::PipelineCreation); };
            dbg_transparent_stroke_pso = if let Ok(pso) = config.factory.create_pipeline_from_program(
                &shader,
                gfx::Primitive::TriangleList,
                fill_mode,
                transparent_pipeline::new()
            ) { pso } else { return Err(InitializationError::PipelineCreation); };
        } else {
            dbg_opaque_fill_pso = if let Ok(pso) = config.factory.create_pipeline_from_program(
                &shader,
                gfx::Primitive::TriangleList,
                gfx::state::Rasterizer::new_fill(),
                opaque_pipeline::new()
            ) { pso } else { return Err(InitializationError::PipelineCreation); };
            dbg_opaque_stroke_pso = if let Ok(pso) = config.factory.create_pipeline_from_program(
                &shader,
                gfx::Primitive::TriangleList,
                gfx::state::Rasterizer::new_fill(),
                opaque_pipeline::new()
            ) { pso } else { return Err(InitializationError::PipelineCreation); };
            dbg_transparent_fill_pso = if let Ok(pso) = config.factory.create_pipeline_from_program(
                &shader,
                gfx::Primitive::TriangleList,
                gfx::state::Rasterizer::new_fill(),
                transparent_pipeline::new()
            ) { pso } else { return Err(InitializationError::PipelineCreation); };
            dbg_transparent_stroke_pso = if let Ok(pso) = config.factory.create_pipeline_from_program(
                &shader,
                gfx::Primitive::TriangleList,
                gfx::state::Rasterizer::new_fill(),
                transparent_pipeline::new()
            ) { pso } else { return Err(InitializationError::PipelineCreation); };
        }

        return Ok(Renderer {
            fill_data: GpuStore::new(),
            stroke_data: GpuStore::new(),
            transform_buffers: GpuBufferStore::new(),
            render_targets: HashMap::new(),

            opaque_fill_pso: [opaque_fill_pso, dbg_opaque_fill_pso],
            opaque_stroke_pso: [opaque_stroke_pso, dbg_opaque_stroke_pso],
            transparent_fill_pso: [transparent_fill_pso, dbg_transparent_fill_pso],
            transparent_stroke_pso: [transparent_stroke_pso, dbg_transparent_stroke_pso],

            constants_buffer: config.factory.create_constant_buffer(1),

            device: config.device,
            factory: config.factory,
        });
    }
}

pub struct GpuStore<Vertex, Primitive> {
    geometry: GpuGeometryStore<Vertex>,
    primitives: GpuBufferStore<Primitive>,
}

impl<Vertex, Primitive> GpuStore<Vertex, Primitive>
where
    Vertex: Copy + gfx::traits::Pod + gfx::pso::buffer::Structure<gfx::format::Format>,
    Primitive: Copy + Default + gfx::traits::Pod
{
    pub fn new() -> Self {
        GpuStore {
            geometry: GpuGeometryStore::new(),
            primitives: GpuBufferStore::new(),
        }
    }

    pub fn update(&mut self, data: &mut PrimStore<Vertex, Primitive>, factory: &mut GlFactory, queue: &mut CmdEncoder) {
        self.geometry.update(&mut data.geometry, factory, queue);
        self.primitives.update(&mut data.primitives, factory, queue);
    }
}

pub struct GpuBufferStore<Primitive> {
    buffers: Vec<BufferObject<Primitive>>,
}

impl<Primitive> GpuBufferStore<Primitive>
where  Primitive: Copy + Default + gfx::traits::Pod {
    pub fn new() -> Self { GpuBufferStore { buffers: Vec::new() } }

    pub fn update(&mut self, cpu: &mut BufferStore<Primitive>, factory: &mut GlFactory, queue: &mut CmdEncoder) {
        for i in 0..cpu.buffers.len() {
            if i >= self.buffers.len() {
                let buffer = factory.create_constant_buffer(PRIM_BUFFER_LEN);
                self.buffers.push(buffer);
            }
            queue.update_buffer(&self.buffers[i], cpu.buffers[i].as_slice(), 0).unwrap();
        }
    }
}

pub struct GpuGeometryStore<Vertex> {
    buffers: Vec<Geometry<Vertex>>,
}

impl<Vertex> GpuGeometryStore<Vertex>
where Vertex: Copy + gfx::traits::Pod + gfx::pso::buffer::Structure<gfx::format::Format> {
    pub fn new() -> Self { GpuGeometryStore { buffers: Vec::new() } }

    pub fn update(&mut self, cpu: &mut GeometryStore<Vertex>, factory: &mut GlFactory, queue: &mut CmdEncoder) {
        for i in 0..cpu.buffers.len() {
            let cpu_geom = &cpu.buffers[i];
            let (vbo, ibo) = factory.create_vertex_buffer_with_slice(
                &cpu_geom.vertices[..],
                &cpu_geom.indices[..],
            );
            self.buffers.push(Geometry { vbo: vbo, ibo: ibo });
        }
    }
}



pub struct RenderOptions {
    pub wireframe: bool,
}

pub struct RendererConfig {
    pub device: GlDevice,
    pub factory: GlFactory,
    pub debug: bool,
}

impl RenderOptions {
    pub fn new() -> Self {
        RenderOptions {
            wireframe: false,
        }
    }

    pub fn with_wireframe(mut self, enabled: bool) -> Self {
        self.wireframe = enabled;
        return self;
    }
}

pub fn create_index_buffer(factory: &mut GlFactory, data: &[u16]) -> Ibo {
    use gfx::IntoIndexBuffer;
    return data.into_index_buffer(factory);
}
