use gfx::traits::FactoryExt;
//use gfx_device_gl;
use gfx;

use tessellation;
use tessellation::geometry_builder::VertexConstructor;
use core::math::*;
use buffer::*;
pub use gfx_types::*;
use shaders::*;

use frame::*;

use std;
//use std::sync::Arc;
use std::collections::HashMap;

pub type OpaquePso = Pso<opaque_pipeline::Meta>;
pub type TransparentPso = Pso<transparent_pipeline::Meta>;

pub type StrokeVertex = Vertex;
pub type FillVertex = Vertex;
pub type StrokeShapeData = ShapeData;
pub type FillShapeData = ShapeData;

gfx_defines!{
    constant Globals {
        resolution: [f32; 2] = "u_resolution",
        scroll_offset: [f32; 2] = "u_scroll_offset",
        zoom: f32 = "u_zoom",
    }

    constant ShapeTransform {
        transform: [[f32; 4]; 4] = "transform",
    }

    // Per-shape data.
    // It would probably make sense to have different structures for fills and strokes,
    // but using the same struct helps with keeping things simple for now.
    constant ShapeData {
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
        shape_id: i32 = "a_shape_id", // An id pointing to the ShapeData struct above.
    }

    pipeline opaque_pipeline {
        vbo: gfx::VertexBuffer<Vertex> = (),
        out_color: gfx::RenderTarget<ColorFormat> = "out_color",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
        constants: gfx::ConstantBuffer<Globals> = "Globals",
        transforms: gfx::ConstantBuffer<ShapeTransform> = "u_transforms",
        shape_data: gfx::ConstantBuffer<ShapeData> = "u_shape_data",
    }

    pipeline transparent_pipeline {
        vbo: gfx::VertexBuffer<Vertex> = (),
        out_color: gfx::RenderTarget<ColorFormat> = "out_color",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_TEST,
        constants: gfx::ConstantBuffer<Globals> = "Globals",
        transforms: gfx::ConstantBuffer<ShapeTransform> = "u_transforms",
        shape_data: gfx::ConstantBuffer<ShapeData> = "u_shape_data",
    }
}

pub type TransformId = Id<ShapeTransform>;

impl ShapeData {
    pub fn new(color: [f32; 4], z_index: f32, transform_id: TransformId) -> ShapeData {
        ShapeData {
            color: color,
            z_index: z_index,
            transform_id: transform_id.to_i32(),
            width: 1.0,
            _padding: 0.0,
        }
    }
}

impl std::default::Default for ShapeData {
    fn default() -> Self { ShapeData::new([1.0, 1.0, 1.0, 1.0], 0.0, TransformId::new(0)) }
}

impl std::default::Default for ShapeTransform {
    fn default() -> Self {
        ShapeTransform { transform: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]}
    }
}

pub type ShapeDataId = Id<ShapeData>;

// Implement a vertex constructor.
// The vertex constructor sits between the tessellator and the geometry builder.
// it is called every time a new vertex needs to be added and creates a the vertex
// from the information provided by the tessellator.
//
// This vertex constructor forwards the positions and normals provided by the
// tessellators and add a shape id.
pub struct WithShapeDataId(pub ShapeDataId);

impl VertexConstructor<tessellation::StrokeVertex, StrokeVertex> for WithShapeDataId {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex) -> StrokeVertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());
        assert!(!vertex.normal.x.is_nan());
        assert!(!vertex.normal.y.is_nan());
        StrokeVertex {
            position: vertex.position.array(),
            normal: vertex.normal.array(),
            shape_id: self.0.to_i32(),
        }
    }
}

// The fill tessellator does not implement normals yet, so this implementation
// just sets it to [0, 0], for now.
impl VertexConstructor<tessellation::FillVertex, FillVertex> for WithShapeDataId {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> FillVertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());
        assert!(!vertex.normal.x.is_nan());
        assert!(!vertex.normal.y.is_nan());
        FillVertex {
            position: vertex.position.array(),
            normal: vertex.normal.array(),
            shape_id: self.0.to_i32(),
        }
    }
}

//pub struct Range { first: u16, count: u16 }
//
//pub struct Ranges {
//    vertices: Range,
//    indices: Range,
//}

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
    fill_geometries: HashMap<VertexBufferId, Geometry<FillVertex>>,
    stroke_geometries: HashMap<VertexBufferId, Geometry<StrokeVertex>>,
    shape_data_buffers: HashMap<UniformBufferId, BufferObject<ShapeData>>,
    transform_buffers: HashMap<UniformBufferId, BufferObject<ShapeTransform>>,
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
            fill_geometries: HashMap::new(),
            stroke_geometries: HashMap::new(),
            shape_data_buffers: HashMap::new(),
            transform_buffers: HashMap::new(),
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

    pub fn render_frame(&mut self, options: &RenderOptions, mut frame: FrameCmds) -> Result<(), ()> {
        for alloc_cmd in frame.allocations {
            match alloc_cmd {
                AllocCmd::AddFillVertexBuffer(id, vertices, indices) => {
                    self.add_fill_geometry(id, &vertices[..], &indices[..]);
                }
                AllocCmd::AddStrokeVertexBuffer(id, vertices, indices) => {
                    self.add_stroke_geometry(id, &vertices[..], &indices[..]);
                }
                AllocCmd::AddFillShapeBuffer(id, buffer) => {
                    self.add_fill_shape_buffer(id, &buffer[..]);
                }
                AllocCmd::AddStrokeShapeBuffer(id, buffer) => {
                    self.add_stroke_shape_buffer(id, &buffer[..]);
                }
                AllocCmd::AddTransformBuffer(id, buffer) => {
                    self.add_transform_buffer(id, &buffer[..])
                }
                AllocCmd::AddTexture(id, descriptor, buffer) => {
                    unimplemented!();
                }
                AllocCmd::RemoveFillShapeBuffer(id) => {
                    self.shape_data_buffers.remove(&id);
                }
                AllocCmd::RemoveStrokeShapeBuffer(id) => {
                    self.shape_data_buffers.remove(&id);
                }
                AllocCmd::RemoveTransformBuffer(id) => {
                    self.transform_buffers.remove(&id);
                }
                AllocCmd::RemoveTexture(id) => {
                    unimplemented!();
                }
            }
        }

        let mut init_queue: CmdEncoder = self.factory.create_command_buffer().into();
        for upload_cmd in frame.uploads {
            match upload_cmd {
                UploadCmd::FillShapeData(id, offset, data) => {
                    if let Some(buffer_object) = self.shape_data_buffers.get(&id) {
                        init_queue.update_buffer(
                            buffer_object,
                            &data[..],
                            offset as usize
                        ).unwrap();
                    }
                }
                UploadCmd::StrokeShapeData(id, offset, data) => {
                    if let Some(buffer_object) = self.shape_data_buffers.get(&id) {
                        init_queue.update_buffer(
                            buffer_object,
                            &data[..],
                            offset as usize
                        ).unwrap();
                    }
                }
                UploadCmd::Transform(id, offset, data) => {
                    if let Some(buffer_object) = self.transform_buffers.get(&id) {
                        init_queue.update_buffer(
                            buffer_object,
                            &data[..],
                            offset as usize
                        ).unwrap();
                    }
                }
                UploadCmd::FillVertex(id, offset, data) => {
                    if let Some(geom) = self.fill_geometries.get(&id) {
                        init_queue.update_buffer(
                            &geom.vbo,
                            &data[..],
                            offset as usize,
                        ).unwrap();
                    }
                }
                UploadCmd::StrokeVertex(id, offset, data) => {
                    if let Some(geom) = self.stroke_geometries.get(&id) {
                        init_queue.update_buffer(
                            &geom.vbo,
                            &data[..],
                            offset as usize,
                        ).unwrap();
                    }
                }
            }
        }
        init_queue.flush(&mut self.device);

        let mut draw_queue: CmdEncoder = self.factory.create_command_buffer().into();
        for target_cmds in frame.targets {
            try!{ self.render_target(target_cmds, options, &mut draw_queue) };
        }
        draw_queue.flush(&mut self.device);

        return Ok(());
    }

    pub fn render_target(
        &mut self,
        target_cmds: RenderTargetCmds,
        options: &RenderOptions,
        draw_queue: &mut CmdEncoder
    ) -> Result<(), ()> {
        let pso_index = if options.wireframe { 1 } else { 0 };

        let target = self.render_targets.get(&target_cmds.fbo).unwrap();
        if let Some(color) = target_cmds.clear_color {
            draw_queue.clear(&target.color.clone(), color.f32_array());
        }
        if let Some(value) = target_cmds.clear_depth {
            draw_queue.clear_depth(&target.depth.clone(), value);
        }
        for cmd in target_cmds.opaque_fills {
            let geom = self.fill_geometries.get(&cmd.mesh.vertices.buffer).unwrap();
            let (start, end) = cmd.mesh.indices.range();
            let mut indices = geom.ibo.clone();
            indices.start = start as u32;
            indices.end = end as u32;
            if cmd.instances > 1 {
                indices.instances = Some((cmd.instances as u32, 0));
            }

            draw_queue.draw(
                &indices,
                &self.opaque_fill_pso[pso_index],
                &opaque_pipeline::Data {
                    vbo: geom.vbo.clone(),
                    out_color: target.color.clone(),
                    out_depth: target.depth.clone(),
                    constants: self.constants_buffer.clone(),
                    shape_data: self.shape_data_buffers.get(&cmd.prim_data).unwrap().clone(),
                    transforms: self.transform_buffers.get(&cmd.transforms).unwrap().clone(),
                }
            );
        }

        /*
        for cmd in target_cmds.opaque_cmds {
            match cmd {
                DrawCmd::Fill(params) => {
                    let geom = self.fill_geometries.get(&params.vbo).unwrap();
                    let mut indices = geom.ibo.clone();
                    let (start, end) = params.indices.range();
                    indices.start = start as u32;
                    indices.end = end as u32;
                    if let Some(instances) = params.instances {
                        indices.instances = Some((instances as u32, 0));
                    }

                    draw_queue.draw(
                        &indices,
                        &self.opaque_fill_pso[pso_index],
                        &opaque_pipeline::Data {
                            vbo: geom.vbo.clone(),
                            out_color: target.color.clone(),
                            out_depth: target.depth.clone(),
                            constants: self.constants_buffer.clone(),
                            shape_data: self.shape_data_buffers.get(&params.shape_data).unwrap().clone(),
                            transforms: self.transform_buffers.get(&params.transforms).unwrap().clone(),
                        }
                    );
                }
                DrawCmd::Stroke(params) => {
                    let geom = self.stroke_geometries.get(&params.vbo).unwrap();
                    let mut indices = geom.ibo.clone();
                    let (start, end) = params.indices.range();
                    indices.start = start as u32;
                    indices.end = end as u32;
                    if let Some(instances) = params.instances {
                        indices.instances = Some((instances as u32, 0));
                    }

                    draw_queue.draw(
                        &indices,
                        &self.opaque_stroke_pso[pso_index],
                        &opaque_pipeline::Data {
                            vbo: geom.vbo.clone(),
                            out_color: target.color.clone(),
                            out_depth: target.depth.clone(),
                            constants: self.constants_buffer.clone(),
                            shape_data: self.shape_data_buffers.get(&params.shape_data).unwrap().clone(),
                            transforms: self.transform_buffers.get(&params.transforms).unwrap().clone(),
                        }
                    );
                }
                DrawCmd::External(params) => {
                    unimplemented!();
                }
            }
        }
        */

        return Ok(());
    }

    pub fn add_render_target(&mut self, id: RenderTargetId, target: RenderTarget) {
        self.render_targets.insert(id, target);
    }

    pub fn add_fill_geometry(&mut self, id: VertexBufferId, vertices: &[FillVertex], indices: &[u16]) {
        let (vbo, ibo) = self.factory.create_vertex_buffer_with_slice(
            &vertices[..],
            &indices[..],
        );
        self.fill_geometries.insert(id, Geometry { vbo: vbo, ibo: ibo });
    }

    pub fn add_stroke_geometry(&mut self, id: VertexBufferId, vertices: &[StrokeVertex], indices: &[u16]) {
        let (vbo, ibo) = self.factory.create_vertex_buffer_with_slice(
            &vertices[..],
            &indices[..],
        );
        self.stroke_geometries.insert(id, Geometry { vbo: vbo, ibo: ibo });
    }

    pub fn add_fill_shape_buffer(&mut self, id: UniformBufferId, data: &[FillShapeData]) {
        let buffer = self.factory.create_constant_buffer(data.len());
        self.shape_data_buffers.insert(id, buffer);
    }

    pub fn add_stroke_shape_buffer(&mut self, id: UniformBufferId, data: &[StrokeShapeData]) {
        let buffer = self.factory.create_constant_buffer(data.len());
        self.shape_data_buffers.insert(id, buffer);
    }

    pub fn add_transform_buffer(&mut self, id: UniformBufferId, data: &[ShapeTransform]) {
        let buffer = self.factory.create_constant_buffer(data.len());
        self.transform_buffers.insert(id, buffer);
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
