#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate lyon;

use lyon::extra::rust_logo::build_logo_path;
use lyon::path_builder::*;
use lyon::math::*;
use lyon::tessellation::geometry_builder::{ VertexConstructor, VertexBuffers, BuffersBuilder };
use lyon::tessellation::basic_shapes::*;
use lyon::tessellation::path_fill::{ FillTessellator, FillOptions };
use lyon::tessellation::path_stroke::{ StrokeTessellator, StrokeOptions };
use lyon::tessellation::{ FillVertex, StrokeVertex };
use lyon::path::Path;
use lyon::path_iterator::PathIterator;

use gfx::traits::FactoryExt;
use gfx::Device;

use std::ops::Rem;

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

const SHAPE_DATA_LEN: usize = 1024;

// Describe the vertex, uniform data and pipeline states passed to gfx-rs.
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

    pipeline model_pipeline {
        vbo: gfx::VertexBuffer<Vertex> = (),
        out_color: gfx::RenderTarget<ColorFormat> = "out_color",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
        constants: gfx::ConstantBuffer<Globals> = "Globals",
        transforms: gfx::ConstantBuffer<ShapeTransform> = "u_transforms",
        shape_data: gfx::ConstantBuffer<ShapeData> = "u_shape_data",
    }

    // The background is drawn separately with its own shader.
    vertex BgVertex {
        position: [f32; 2] = "a_position",
    }

    pipeline bg_pipeline {
        vbo: gfx::VertexBuffer<BgVertex> = (),
        out_color: gfx::RenderTarget<ColorFormat> = "out_color",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
        constants: gfx::ConstantBuffer<Globals> = "Globals",
    }
}

pub fn split_gfx_slice<R:gfx::Resources>(slice: gfx::Slice<R>, at: u32) -> (gfx::Slice<R>, gfx::Slice<R>) {
    let mut first = slice.clone();
    let mut second = slice.clone();
    first.end = at;
    second.start = at;

    return (first, second);
}

impl ShapeData {
    fn new(color: [f32; 4], z_index: f32, transform_id: TransformId) -> ShapeData {
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

use std::marker::PhantomData;

pub struct Id<T> {
    handle: u16,
    _marker: PhantomData<T>,
}
impl<T> Copy for Id<T> {}
impl<T> Clone for Id<T> { fn clone(&self) -> Self { *self } }
impl<T> Id<T> {
    pub fn new(handle: u16) -> Self { Id { handle: handle, _marker: PhantomData  } }
    pub fn index(&self) -> usize { self.handle as usize }
    pub fn to_i32(&self) -> i32 { self.handle as i32 }
}

#[derive(Copy, Clone)]
pub struct IdRange<T> {
    first: Id<T>,
    count: u16,
}

impl<T> IdRange<T> {
    pub fn new(first: Id<T>, count: u16) -> Self {
        IdRange {
            first: first,
            count: count,
        }
    }
    pub fn first(&self) -> Id<T> { self.first }
    pub fn first_index(&self) -> usize { self.first.index() }
    pub fn count(&self) -> usize { self.count as usize }
    pub fn get(&self, n: usize) -> Id<T> {
        assert!(n < self.count(), "Shape id out of range.");
        Id::new(self.first.handle + n as u16)
    }
}

pub type ShapeId = Id<ShapeData>;

pub struct CpuBuffer<T> {
    data: Box<[T]>,
    next_id: u16,
}

impl<T: Default+Copy> CpuBuffer<T> {
    pub fn new() -> Self {
        CpuBuffer {
            data: Box::new([Default::default(); SHAPE_DATA_LEN]),
            next_id: 0,
        }
    }

    pub fn try_alloc_id(&mut self) -> Option<Id<T>> {
        let id = self.next_id;
        if id as usize >= self.data.len() {
            return None;
        }

        self.next_id += 1;
        return Some(Id::new(id));
    }

    pub fn alloc_id(&mut self) -> Id<T> { self.try_alloc_id().unwrap() }

    pub fn try_alloc_range(&mut self, count: usize) -> Option<IdRange<T>> {
        let id = self.next_id;
        if id as usize + count >= self.data.len() {
            return None;
        }

        self.next_id += count as u16;
        return Some(IdRange::new(Id::new(id), count as u16));
    }

    pub fn alloc_range(&mut self, count: usize) -> IdRange<T> {
        self.try_alloc_range(count).unwrap()
    }

    pub fn as_slice(&self) -> &[T] { &self.data[..] }

    pub fn len(&self) -> usize { self.data.len() }
}

impl<T> std::ops::Index<Id<T>> for CpuBuffer<T> {
    type Output = T;
    fn index(&self, id: Id<T>) -> &T {
        &self.data[id.index()]
    }
}

impl<T> std::ops::IndexMut<Id<T>> for CpuBuffer<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut T {
        &mut self.data[id.index()]
    }
}

// Implement a vertex constructor.
// The vertex constructor sits between the tessellator and the geometry builder.
// it is called every time a new vertex needs to be added and creates a the vertex
// from the information provided by the tessellator.
//
// This vertex constructor forwards the positions and normals provided by the
// tessellators and add a shape id.
struct WithShapeId(ShapeId);

impl VertexConstructor<StrokeVertex, Vertex> for WithShapeId {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> Vertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());
        assert!(!vertex.normal.x.is_nan());
        assert!(!vertex.normal.y.is_nan());
        Vertex {
            position: vertex.position.array(),
            normal: vertex.normal.array(),
            shape_id: self.0.to_i32(),
        }
    }
}

// The fill tessellator does not implement normals yet, so this implementation
// just sets it to [0, 0], for now.
impl VertexConstructor<FillVertex, Vertex> for WithShapeId {
    fn new_vertex(&mut self, vertex: FillVertex) -> Vertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());
        assert!(!vertex.normal.x.is_nan());
        assert!(!vertex.normal.y.is_nan());
        Vertex {
            position: vertex.position.array(),
            normal: vertex.normal.array(),
            shape_id: self.0.to_i32(),
        }
    }
}

struct BgWithShapeId ;
impl VertexConstructor<FillVertex, BgVertex> for BgWithShapeId  {
    fn new_vertex(&mut self, vertex: FillVertex) -> BgVertex {
        BgVertex { position: vertex.position.array() }
    }
}

pub type TransformId = Id<ShapeTransform>;

fn main() {
    println!("== gfx-rs example ==");
    println!("Controls:");
    println!("  Arrow keys: scrolling");
    println!("  PgUp/PgDown: zoom in/out");
    println!("  w: toggle wireframe mode");
    println!("  p: toggle sow points");
    println!("  a/z: increase/decrease the stroke width");

    let num_instances = 32;

    // Build a Path for the rust logo.
    let mut builder = SvgPathBuilder::new(Path::builder());
    build_logo_path(&mut builder);
    let path = builder.build();

    // Create some CPU-side buffers that will contain the geometry.
    let mut path_mesh_cpu: VertexBuffers<Vertex> = VertexBuffers::new();
    let mut points_mesh_cpu: VertexBuffers<Vertex> = VertexBuffers::new();
    let mut shape_data_cpu: CpuBuffer<ShapeData> = CpuBuffer::new();
    let mut shape_transforms_cpu: CpuBuffer<ShapeTransform> = CpuBuffer::new();

    let default_transform = shape_transforms_cpu.alloc_id();
    let logo_transforms = shape_transforms_cpu.alloc_range(num_instances);

    // Tessellate the fill
    let fill_ids = shape_data_cpu.alloc_range(num_instances);

    // Note that we flatten the path here. Since the flattening tolerance should
    // depend on the resolution/zoom it would make sense to re-tessellate when the
    // zoom level changes (not done here for simplicity).
    let fill_count = FillTessellator::new().tessellate_path(
        path.path_iter().flattened(0.09),
        &FillOptions::default(),
        &mut BuffersBuilder::new(&mut path_mesh_cpu, WithShapeId(fill_ids.first()))
    ).unwrap();

    shape_data_cpu[fill_ids.first()] = ShapeData::new([1.0, 1.0, 1.0, 1.0], 0.1, logo_transforms.first());
    for i in 1..num_instances {
        shape_data_cpu[fill_ids.get(i)] = ShapeData::new(
            [(0.1 * i as f32).rem(1.0), (0.5 * i as f32).rem(1.0), (0.9 * i as f32).rem(1.0), 1.0],
            0.1 - 0.001 * i as f32,
            logo_transforms.get(i)
        );
    }

    // Tessellate the stroke
    let stroke_id = shape_data_cpu.alloc_id();

    StrokeTessellator::new().tessellate(
        path.path_iter().flattened(0.022),
        &StrokeOptions::default(),
        &mut BuffersBuilder::new(&mut path_mesh_cpu, WithShapeId(stroke_id))
    ).unwrap();
    shape_data_cpu[stroke_id] = ShapeData::new([0.0, 0.0, 0.0, 0.1], 0.2, default_transform);

    let mut num_points = 0;
    for p in path.as_slice().iter() {
        if p.destination().is_some() {
            num_points += 1;
        }
    }

    let point_transforms = shape_transforms_cpu.alloc_range(num_points);
    let point_ids_1 = shape_data_cpu.alloc_range(num_points);
    let point_ids_2 = shape_data_cpu.alloc_range(num_points);

    let ellipsis_count = fill_ellipsis(
        vec2(0.0, 0.0), vec2(1.0, 1.0), 64,
        &mut BuffersBuilder::new(&mut points_mesh_cpu, WithShapeId(point_ids_1.first()))
    );
    fill_ellipsis(
        vec2(0.0, 0.0), vec2(0.5, 0.5), 64,
        &mut BuffersBuilder::new(&mut points_mesh_cpu, WithShapeId(point_ids_2.first()))
    );

    let mut i = 0;
    for p in path.as_slice().iter() {
        if let Some(to) = p.destination() {
            let transform_id = point_transforms.get(i);
            shape_transforms_cpu[transform_id].transform = Mat4::create_translation(
                to.x, to.y, 0.0
            ).to_row_arrays();
            shape_data_cpu[point_ids_1.get(i)] = ShapeData::new(
                [0.0, 0.2, 0.0, 1.0],
                0.3,
                transform_id
            );
            shape_data_cpu[point_ids_2.get(i)] = ShapeData::new(
                [0.0, 1.0, 0.0, 1.0],
                0.4,
                transform_id
            );
            i += 1;
        }
    }

    println!(" -- {} vertices {} indices", path_mesh_cpu.vertices.len(), path_mesh_cpu.indices.len());

    let mut bg_path_mesh_cpu: VertexBuffers<BgVertex> = VertexBuffers::new();
    fill_rectangle(
        &Rect::new(vec2(-1.0, -1.0), size(2.0, 2.0)),
        &mut BuffersBuilder::new(&mut bg_path_mesh_cpu, BgWithShapeId )
    );

    // Initialize glutin and gfx-rs (refer to gfx-rs examples for more details).

    let glutin_builder = glutin::WindowBuilder::new()
        .with_dimensions(700, 700)
        .with_decorations(true)
        .with_title("tessellation".to_string())
        .with_multisampling(8)
        .with_vsync();

    let (window, mut device, mut factory, mut main_fbo, mut main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(glutin_builder);

    let mut cmd_queue: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let constants = factory.create_constant_buffer(1);
    let shape_data_gpu = factory.create_constant_buffer(SHAPE_DATA_LEN);
    let shape_transforms_gpu = factory.create_constant_buffer(SHAPE_DATA_LEN);

    let bg_pso = factory.create_pipeline_simple(
        BACKGROUND_VERTEX_SHADER.as_bytes(),
        BACKGROUND_FRAGMENT_SHADER.as_bytes(),
        bg_pipeline::new()
    ).unwrap();

    let (bg_vbo, bg_range) = factory.create_vertex_buffer_with_slice(
        &bg_path_mesh_cpu.vertices[..],
        &bg_path_mesh_cpu.indices[..]
    );

    let model_shader = factory.link_program(
        MODEL_VERTEX_SHADER.as_bytes(),
        MODEL_FRAGMENT_SHADER.as_bytes(),
    ).unwrap();

    let model_pso = factory.create_pipeline_from_program(
        &model_shader,
        gfx::Primitive::TriangleList,
        gfx::state::Rasterizer::new_fill(),
        model_pipeline::new()
    ).unwrap();

    let mut fill_mode = gfx::state::Rasterizer::new_fill();
    fill_mode.method = gfx::state::RasterMethod::Line(1);
    let wireframe_model_pso = factory.create_pipeline_from_program(
        &model_shader,
        gfx::Primitive::TriangleList,
        fill_mode,
        model_pipeline::new()
    ).unwrap();

    /// Upload the tessellated geometry to the GPU.
    let (model_vbo, model_range) = factory.create_vertex_buffer_with_slice(
        &path_mesh_cpu.vertices[..],
        &path_mesh_cpu.indices[..]
    );

    let (points_vbo, points_range) = factory.create_vertex_buffer_with_slice(
        &points_mesh_cpu.vertices[..],
        &points_mesh_cpu.indices[..]
    );
    let (mut points_range_1, mut points_range_2) = split_gfx_slice(points_range.clone(), ellipsis_count.indices as u32);
    points_range_1.instances = Some((num_points as u32, 0));
    points_range_2.instances = Some((num_points as u32, 0));

    let (mut fill_range, stroke_range) = split_gfx_slice(model_range, fill_count.indices as u32);
    fill_range.instances = Some((num_instances as u32, 0));

    let mut scene = SceneParams {
        target_zoom: 5.0,
        zoom: 0.5,
        target_scroll: vec2(70.0, 70.0),
        scroll: vec2(70.0, 70.0),
        show_points: false,
        show_wireframe: false,
        stroke_width: 0.0,
        target_stroke_width: 1.0,
    };

    let mut init_queue: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    init_queue.update_buffer(&shape_data_gpu, shape_data_cpu.as_slice(), 0).unwrap();
    init_queue.update_buffer(&shape_transforms_gpu, shape_transforms_cpu.as_slice(), 0).unwrap();
    init_queue.flush(&mut device);

    let mut frame_count: usize = 0;
    loop {
        if !update_inputs(&window, &mut scene) {
            break;
        }

        // Set the color of the second shape (the outline) to some slowly changing
        // pseudo-random color.
        shape_data_cpu[stroke_id].color = [
            (frame_count as f32 * 0.008 - 1.6).sin() * 0.1 + 0.1,
            (frame_count as f32 * 0.005 - 1.6).sin() * 0.1 + 0.1,
            (frame_count as f32 * 0.01 - 1.6).sin() * 0.1 + 0.1,
            1.0
        ];
        shape_data_cpu[stroke_id].width = scene.stroke_width;

        for i in 1..num_instances {
            shape_transforms_cpu[logo_transforms.get(i)].transform = Mat4::create_translation(
                (frame_count as f32 * 0.001 * i as f32).sin() * (100.0 + i as f32 * 10.0),
                (frame_count as f32 * 0.002 * i as f32).sin() * (100.0 + i as f32 * 10.0),
                0.0
            ).to_row_arrays();
        }


        gfx_window_glutin::update_views(&window, &mut main_fbo, &mut main_depth);
        let (w, h) = window.get_inner_size_pixels().unwrap();

        cmd_queue.clear(&main_fbo.clone(), [0.0, 0.0, 0.0, 0.0]);
        cmd_queue.clear_depth(&main_depth.clone(), 1.0);

        cmd_queue.update_buffer(&shape_transforms_gpu, shape_transforms_cpu.as_slice(), 0).unwrap();
        cmd_queue.update_buffer(&shape_data_gpu, shape_data_cpu.as_slice(), 0).unwrap();
        cmd_queue.update_constant_buffer(&constants, &Globals {
            resolution: [w as f32, h as f32],
            zoom: scene.zoom,
            scroll_offset: scene.scroll.array(),
        });

        let default_pipeline_data = model_pipeline::Data {
            vbo: model_vbo.clone(),
            out_color: main_fbo.clone(),
            out_depth: main_depth.clone(),
            constants: constants.clone(),
            shape_data: shape_data_gpu.clone(),
            transforms: shape_transforms_gpu.clone(),
        };

pub type Buffer = gfx::handle::Buffer;

        // Draw the opaque geometry front to back with the depth buffer enabled.

        if scene.show_points {
            cmd_queue.draw(&points_range_1, &model_pso, &model_pipeline::Data {
                vbo: points_vbo.clone(),
                .. default_pipeline_data.clone()
            });
            cmd_queue.draw(&points_range_2, &model_pso, &model_pipeline::Data {
                vbo: points_vbo.clone(),
                .. default_pipeline_data.clone()
            });
        }

        let pso = if scene.show_wireframe { &wireframe_model_pso } else { &model_pso };

        cmd_queue.draw(&fill_range, &pso, &default_pipeline_data);

        cmd_queue.draw(&stroke_range, &pso, &default_pipeline_data);

        cmd_queue.draw(&bg_range, &bg_pso, &bg_pipeline::Data {
            vbo: bg_vbo.clone(),
            out_color: main_fbo.clone(),
            out_depth: main_depth.clone(),
            constants: constants.clone(),
        });

        // Non-opaque geometry should be drawn back to front here.
        // (there is none in this example)

        cmd_queue.flush(&mut device);

        window.swap_buffers().unwrap();
        device.cleanup();

        frame_count += 1;
    }
}

// The vertex shader for the tessellated geometry.
// The transform, color and stroke width are applied instead of during tessellation. This makes
// it possible to change these parameters without having to modify/upload the geometry.
// Per-shape data is stored in uniform buffer objects to keep the vertex buffer small.
static MODEL_VERTEX_SHADER: &'static str = &"
    #version 140
    #line 266

    #define SHAPE_DATA_LEN 64

    uniform Globals {
        vec2 u_resolution;
        vec2 u_scroll_offset;
        float u_zoom;
    };

    struct ShapeTransform { mat4 transform; };
    uniform u_transforms { ShapeTransform transforms[SHAPE_DATA_LEN]; };

    struct ShapeData {
        vec4 color;
        float z_index;
        int transform_id;
        float width;
        float _padding;
    };
    uniform u_shape_data { ShapeData shape_data[SHAPE_DATA_LEN]; };

    in vec2 a_position;
    in vec2 a_normal;
    in int a_shape_id;

    out vec4 v_color;

    void main() {
        int id = a_shape_id + gl_InstanceID;
        ShapeData data = shape_data[id];

        vec4 local_pos = vec4(a_position + a_normal * data.width, 0.0, 1.0);
        vec4 world_pos = transforms[data.transform_id].transform * local_pos;
        vec2 transformed_pos = (world_pos.xy / world_pos.w - u_scroll_offset)
            * u_zoom / (vec2(0.5, -0.5) * u_resolution);

        gl_Position = vec4(transformed_pos, 1.0 - data.z_index, 1.0);
        v_color = data.color;
    }
";

// The fragment shader is dead simple. It just applies the color computed in the vertex shader.
// A more advanced renderer would probably compute texture coordinates in the vertex shader and
// sample the color from a texture here.
static MODEL_FRAGMENT_SHADER: &'static str = &"
    #version 140
    in vec4 v_color;
    out vec4 out_color;

    void main() {
        out_color = v_color;
    }
";

static BACKGROUND_VERTEX_SHADER: &'static str = &"
    #version 140
    in vec2 a_position;
    out vec2 v_position;

    void main() {
        // TODO: fetch the z coordinate and a transform from a buffer.
        gl_Position = vec4(a_position, 1.0, 1.0);
        v_position = a_position;
    }
";

// The background.
// This shader is silly and slow, but it looks nice ;)
static BACKGROUND_FRAGMENT_SHADER: &'static str = &"
    #version 140
    uniform Globals {
        vec2 u_resolution;
        vec2 u_scroll_offset;
        float u_zoom;
    };
    in vec2 v_position;
    out vec4 out_color;

    void main() {
        vec2 px_position = v_position * vec2(1.0, -1.0) * u_resolution * 0.5;

        // #005fa4
        float vignette = clamp(0.0, 1.0, (0.7*length(v_position)));
        out_color = mix(
            vec4(0.0, 0.47, 0.9, 1.0),
            vec4(0.0, 0.1, 0.64, 1.0),
            vignette
        );

        // TODO: properly adapt the grid while zooming in and out.
        float grid_scale = 5.0;
        if (u_zoom < 2.5) {
            grid_scale = 1.0;
        }

        vec2 pos = px_position + u_scroll_offset * u_zoom;

        if (mod(pos.x, 20.0 / grid_scale * u_zoom) <= 1.0 ||
            mod(pos.y, 20.0 / grid_scale * u_zoom) <= 1.0) {
            out_color *= 1.2;
        }

        if (mod(pos.x, 100.0 / grid_scale * u_zoom) <= 2.0 ||
            mod(pos.y, 100.0 / grid_scale * u_zoom) <= 2.0) {
            out_color *= 1.2;
        }
    }
";

struct SceneParams {
    target_zoom: f32,
    zoom: f32,
    target_scroll: Vec2,
    scroll: Vec2,
    show_points: bool,
    show_wireframe: bool,
    stroke_width: f32,
    target_stroke_width: f32,
}

fn update_inputs(window: &glutin::Window, scene: &mut SceneParams) -> bool {
    for event in window.poll_events() {
        use glutin::Event::KeyboardInput;
        use glutin::ElementState::Pressed;
        use glutin::VirtualKeyCode;
        match event {
            glutin::Event::Closed => {
                return false;
            }
            KeyboardInput(Pressed, _, Some(key)) => {
                match key {
                    VirtualKeyCode::Escape => {
                        return false;
                    }
                    VirtualKeyCode::PageDown => {
                        scene.target_zoom *= 0.8;
                    }
                    VirtualKeyCode::PageUp => {
                        scene.target_zoom *= 1.25;
                    }
                    VirtualKeyCode::Left => {
                        scene.target_scroll.x -= 50.0 / scene.target_zoom;
                    }
                    VirtualKeyCode::Right => {
                        scene.target_scroll.x += 50.0 / scene.target_zoom;
                    }
                    VirtualKeyCode::Up => {
                        scene.target_scroll.y -= 50.0 / scene.target_zoom;
                    }
                    VirtualKeyCode::Down => {
                        scene.target_scroll.y += 50.0 / scene.target_zoom;
                    }
                    VirtualKeyCode::P => {
                        scene.show_points = !scene.show_points;
                    }
                    VirtualKeyCode::W => {
                        scene.show_wireframe = !scene.show_wireframe;
                    }
                    VirtualKeyCode::A => {
                        scene.target_stroke_width += 0.8;
                    }
                    VirtualKeyCode::Z => {
                        scene.target_stroke_width -= 0.8;
                    }
                    _key => {}
                }
                println!(" -- zoom: {}, scroll: {:?}", scene.target_zoom, scene.target_scroll);
            }
            _evt => {
                //println!("{:?}", _evt);
            }
        };
    }

    scene.zoom += (scene.target_zoom - scene.zoom) / 3.0;
    scene.scroll = scene.scroll + (scene.target_scroll - scene.scroll) / 3.0;
    scene.stroke_width = scene.stroke_width + (scene.target_stroke_width - scene.stroke_width) / 5.0;

    return true;
}
