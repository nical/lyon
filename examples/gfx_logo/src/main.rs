#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate gfx_device_gl;
extern crate glutin;
extern crate lyon;
extern crate lyon_renderer;

use lyon::extra::rust_logo::build_logo_path;
use lyon::path_builder::*;
use lyon::math::*;
use lyon::tessellation::geometry_builder::{ VertexConstructor, VertexBuffers, BuffersBuilder };
use lyon::tessellation::basic_shapes::*;
use lyon::tessellation::path_fill::{ FillTessellator, FillOptions };
use lyon::tessellation::path_stroke::{ StrokeTessellator, StrokeOptions };
use lyon::tessellation;
use lyon::path::Path;
use lyon::path_iterator::PathIterator;
use lyon_renderer::frame;
use lyon_renderer::buffer::{Id, CpuBuffer};
use lyon_renderer::shaders::*;
use lyon_renderer::api::{Color};
// make  public so that the module in gfx_defines can see the types.
pub use lyon_renderer::gfx_types::*;

use gfx::traits::FactoryExt;

use std::ops::Rem;

type FillVertex = Vertex;
type StrokeVertex = Vertex;

type OpaquePso = Pso<opaque_pipeline::Meta>;
type TransparentPso = Pso<transparent_pipeline::Meta>;

// Describe the vertex, uniform data and pipeline states passed to gfx-rs.
gfx_defines!{
    constant Globals {
        resolution: [f32; 2] = "u_resolution",
        scroll_offset: [f32; 2] = "u_scroll_offset",
        zoom: f32 = "u_zoom",
    }

    constant PrimTransform {
        transform: [[f32; 4]; 4] = "transform",
    }

    // Per-shape data.
    // It would probably make sense to have different structures for fills and strokes,
    // but using the same struct helps with keeping things simple for now.
    constant PrimData {
        // TODO: sample the color from a texture.
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
        prim_id: i32 = "a_prim_id", // An id pointing to the PrimData struct above.
    }

    pipeline opaque_pipeline {
        vbo: gfx::VertexBuffer<Vertex> = (),
        out_color: gfx::RenderTarget<ColorFormat> = "out_color",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
        constants: gfx::ConstantBuffer<Globals> = "Globals",
        transforms: gfx::ConstantBuffer<PrimTransform> = "u_transforms",
        prim_data: gfx::ConstantBuffer<PrimData> = "u_prim_data",
    }

    pipeline transparent_pipeline {
        vbo: gfx::VertexBuffer<Vertex> = (),
        out_color: gfx::RenderTarget<ColorFormat> = "out_color",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_TEST,
        constants: gfx::ConstantBuffer<Globals> = "Globals",
        transforms: gfx::ConstantBuffer<PrimTransform> = "u_transforms",
        prim_data: gfx::ConstantBuffer<PrimData> = "u_prim_data",
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

pub type TransformId = Id<PrimTransform>;

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

impl std::default::Default for PrimTransform {
    fn default() -> Self {
        PrimTransform { transform: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]}
    }
}

pub fn split_gfx_slice<R:gfx::Resources>(slice: gfx::Slice<R>, at: u32) -> (gfx::Slice<R>, gfx::Slice<R>) {
    let mut first = slice.clone();
    let mut second = slice.clone();
    first.end = at;
    second.start = at;

    return (first, second);
}

pub fn gfx_sub_slice<R:gfx::Resources>(slice: gfx::Slice<R>, from: u32, to: u32) -> gfx::Slice<R> {
    let mut sub = slice.clone();
    sub.start = from;
    sub.end = to;

    return sub;
}


pub type PrimId = Id<PrimData>;

// Implement a vertex constructor.
// The vertex constructor sits between the tessellator and the geometry builder.
// it is called every time a new vertex needs to be added and creates a the vertex
// from the information provided by the tessellator.
//
// This vertex constructor forwards the positions and normals provided by the
// tessellators and add a prim id.
pub struct WithPrimId(pub PrimId);

impl VertexConstructor<tessellation::StrokeVertex, StrokeVertex> for WithPrimId {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex) -> StrokeVertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());
        assert!(!vertex.normal.x.is_nan());
        assert!(!vertex.normal.y.is_nan());
        StrokeVertex {
            position: vertex.position.array(),
            normal: vertex.normal.array(),
            prim_id: self.0.to_i32(),
        }
    }
}

// The fill tessellator does not implement normals yet, so this implementation
// just sets it to [0, 0], for now.
impl VertexConstructor<tessellation::FillVertex, FillVertex> for WithPrimId {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> FillVertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());
        assert!(!vertex.normal.x.is_nan());
        assert!(!vertex.normal.y.is_nan());
        FillVertex {
            position: vertex.position.array(),
            normal: vertex.normal.array(),
            prim_id: self.0.to_i32(),
        }
    }
}

struct BgWithPrimId ;
impl VertexConstructor<tessellation::FillVertex, BgVertex> for BgWithPrimId  {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> BgVertex {
        BgVertex { position: vertex.position.array() }
    }
}

fn main() {
    println!("== gfx-rs example ==");
    println!("Controls:");
    println!("  Arrow keys: scrolling");
    println!("  PgUp/PgDown: zoom in/out");
    println!("  w: toggle wireframe mode");
    println!("  p: toggle show points");
    println!("  a/z: increase/decrease the stroke width");

    let num_instances = 32;

    // Build a Path for the rust logo.
    let mut builder = SvgPathBuilder::new(Path::builder());
    build_logo_path(&mut builder);
    let path = builder.build();

    // Create some CPU-side buffers that will contain the geometry.
    let mut fill_mesh_cpu: VertexBuffers<Vertex> = VertexBuffers::new();
    let mut stroke_mesh_cpu: VertexBuffers<Vertex> = VertexBuffers::new();
    let mut prim_data_cpu: CpuBuffer<PrimData> = CpuBuffer::new(PRIM_BUFFER_LEN as u16);
    let mut prim_transforms_cpu: CpuBuffer<PrimTransform> = CpuBuffer::new(PRIM_BUFFER_LEN as u16);

    let default_transform = prim_transforms_cpu.push(PrimTransform::default());
    let logo_transforms = prim_transforms_cpu.alloc_range(num_instances);

    // Tessellate the fill
    let fill_ids = prim_data_cpu.alloc_range(num_instances);

    // Note that we flatten the path here. Since the flattening tolerance should
    // depend on the resolution/zoom it would make sense to re-tessellate when the
    // zoom level changes (not done here for simplicity).
    let fill_count = FillTessellator::new().tessellate_path(
        path.path_iter().flattened(0.09),
        &FillOptions::default(),
        &mut BuffersBuilder::new(&mut fill_mesh_cpu, WithPrimId(fill_ids.first()))
    ).unwrap();

    prim_data_cpu[fill_ids.first()] = PrimData::new([1.0, 1.0, 1.0, 1.0], 0.1, logo_transforms.first());
    for i in 1..num_instances {
        prim_data_cpu[fill_ids.get(i)] = PrimData::new(
            [(0.1 * i as f32).rem(1.0), (0.5 * i as f32).rem(1.0), (0.9 * i as f32).rem(1.0), 1.0],
            0.1 - 0.001 * i as f32,
            logo_transforms.get(i)
        );
    }

    // Tessellate the stroke
    let stroke_id = prim_data_cpu.push(PrimData::new([0.0, 0.0, 0.0, 0.1], 0.2, default_transform));

    StrokeTessellator::new().tessellate(
        path.path_iter().flattened(0.022),
        &StrokeOptions::default(),
        &mut BuffersBuilder::new(&mut stroke_mesh_cpu, WithPrimId(stroke_id))
    ).unwrap();

    let mut num_points = 0;
    for p in path.as_slice().iter() {
        if p.destination().is_some() {
            num_points += 1;
        }
    }

    let point_transforms = prim_transforms_cpu.alloc_range(num_points);
    let point_ids_1 = prim_data_cpu.alloc_range(num_points);
    let point_ids_2 = prim_data_cpu.alloc_range(num_points);

    let ellipse_vertices_start = fill_mesh_cpu.vertices.len() as u32;
    let ellipse_indices_start = fill_mesh_cpu.indices.len() as u32;
    let ellipsis_count = fill_ellipse(
        vec2(0.0, 0.0), vec2(1.0, 1.0), 64,
        &mut BuffersBuilder::new(&mut fill_mesh_cpu, WithPrimId(point_ids_1.first()))
    );
    fill_ellipse(
        vec2(0.0, 0.0), vec2(0.5, 0.5), 64,
        &mut BuffersBuilder::new(&mut fill_mesh_cpu, WithPrimId(point_ids_2.first()))
    );

    let mut i = 0;
    for p in path.as_slice().iter() {
        if let Some(to) = p.destination() {
            let transform_id = point_transforms.get(i);
            prim_transforms_cpu[transform_id].transform = Mat4::create_translation(
                to.x, to.y, 0.0
            ).to_row_arrays();
            prim_data_cpu[point_ids_1.get(i)] = PrimData::new(
                [0.0, 0.2, 0.0, 1.0],
                0.3,
                transform_id
            );
            prim_data_cpu[point_ids_2.get(i)] = PrimData::new(
                [0.0, 1.0, 0.0, 1.0],
                0.4,
                transform_id
            );
            i += 1;
        }
    }

    println!(" -- fill: {} vertices {} indices", fill_mesh_cpu.vertices.len(), fill_mesh_cpu.indices.len());
    println!(" -- stroke: {} vertices {} indices", stroke_mesh_cpu.vertices.len(), stroke_mesh_cpu.indices.len());

    let mut bg_mesh_cpu: VertexBuffers<BgVertex> = VertexBuffers::new();
    fill_rectangle(
        &Rect::new(vec2(-1.0, -1.0), size(2.0, 2.0)),
        &mut BuffersBuilder::new(&mut bg_mesh_cpu, BgWithPrimId )
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

    let constants = factory.create_constant_buffer(1);
    let prim_data_gpu = factory.create_constant_buffer(PRIM_BUFFER_LEN);
    let prim_transforms_gpu = factory.create_constant_buffer(PRIM_BUFFER_LEN);

    let bg_pso = factory.create_pipeline_simple(
        BACKGROUND_VERTEX_SHADER.as_bytes(),
        BACKGROUND_FRAGMENT_SHADER.as_bytes(),
        bg_pipeline::new()
    ).unwrap();

    let (bg_vbo, bg_range) = factory.create_vertex_buffer_with_slice(
        &bg_mesh_cpu.vertices[..],
        &bg_mesh_cpu.indices[..]
    );

    let model_shader = factory.link_program(
        FILL_VERTEX_SHADER.as_bytes(),
        FILL_FRAGMENT_SHADER.as_bytes(),
    ).unwrap();

    let opaque_pso = factory.create_pipeline_from_program(
        &model_shader,
        gfx::Primitive::TriangleList,
        gfx::state::Rasterizer::new_fill(),
        opaque_pipeline::new()
    ).unwrap();

    let transparent_pso = factory.create_pipeline_from_program(
        &model_shader,
        gfx::Primitive::TriangleList,
        gfx::state::Rasterizer::new_fill(),
        transparent_pipeline::new()
    ).unwrap();

    let mut fill_mode = gfx::state::Rasterizer::new_fill();
    fill_mode.method = gfx::state::RasterMethod::Line(1);
    let wireframe_opaque_pso = factory.create_pipeline_from_program(
        &model_shader,
        gfx::Primitive::TriangleList,
        fill_mode,
        opaque_pipeline::new()
    ).unwrap();

    let wireframe_transparent_pso = factory.create_pipeline_from_program(
        &model_shader,
        gfx::Primitive::TriangleList,
        fill_mode,
        transparent_pipeline::new()
    ).unwrap();

    /// Upload the tessellated geometry to the GPU.
    let (fill_vbo, mut fill_range) = factory.create_vertex_buffer_with_slice(
        &fill_mesh_cpu.vertices[..],
        &fill_mesh_cpu.indices[..]
    );
    let (stroke_vbo, stroke_range) = factory.create_vertex_buffer_with_slice(
        &stroke_mesh_cpu.vertices[..],
        &stroke_mesh_cpu.indices[..]
    );

    let split = ellipse_indices_start + (ellipsis_count.indices as u32);
    let mut points_range_1 = gfx_sub_slice(fill_range.clone(), ellipse_indices_start, split);
    let mut points_range_2 = gfx_sub_slice(fill_range.clone(), split, split + ellipsis_count.indices as u32);
    points_range_1.instances = Some((num_points as u32, 0));
    points_range_2.instances = Some((num_points as u32, 0));

    fill_range.instances = Some((num_instances as u32, 0));
    fill_range.end = fill_count.indices as u32;

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
    init_queue.update_buffer(&prim_data_gpu, prim_data_cpu.as_slice(), 0).unwrap();
    init_queue.update_buffer(&prim_transforms_gpu, prim_transforms_cpu.as_slice(), 0).unwrap();
    init_queue.flush(&mut device);

    let mut cmd_queue: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut frame_count: usize = 0;
    loop {
        if !update_inputs(&window, &mut scene) {
            break;
        }

        // Set the color of the second shape (the outline) to some slowly changing
        // pseudo-random color.
        prim_data_cpu[stroke_id].color = [
            (frame_count as f32 * 0.008 - 1.6).sin() * 0.1 + 0.1,
            (frame_count as f32 * 0.005 - 1.6).sin() * 0.1 + 0.1,
            (frame_count as f32 * 0.01 - 1.6).sin() * 0.1 + 0.1,
            1.0
        ];
        prim_data_cpu[stroke_id].width = scene.stroke_width;

        for i in 1..num_instances {
            prim_transforms_cpu[logo_transforms.get(i)].transform = Mat4::create_translation(
                (frame_count as f32 * 0.001 * i as f32).sin() * (100.0 + i as f32 * 10.0),
                (frame_count as f32 * 0.002 * i as f32).sin() * (100.0 + i as f32 * 10.0),
                0.0
            ).to_row_arrays();
        }


        gfx_window_glutin::update_views(&window, &mut main_fbo, &mut main_depth);
        let (w, h) = window.get_inner_size_pixels().unwrap();

        cmd_queue.clear(&main_fbo.clone(), [0.0, 0.0, 0.0, 0.0]);
        cmd_queue.clear_depth(&main_depth.clone(), 1.0);

        cmd_queue.update_buffer(&prim_transforms_gpu, prim_transforms_cpu.as_slice(), 0).unwrap();
        cmd_queue.update_buffer(&prim_data_gpu, prim_data_cpu.as_slice(), 0).unwrap();
        cmd_queue.update_constant_buffer(&constants, &Globals {
            resolution: [w as f32, h as f32],
            zoom: scene.zoom,
            scroll_offset: scene.scroll.array(),
        });

        let default_pipeline_data = opaque_pipeline::Data {
            vbo: fill_vbo.clone(),
            out_color: main_fbo.clone(),
            out_depth: main_depth.clone(),
            constants: constants.clone(),
            prim_data: prim_data_gpu.clone(),
            transforms: prim_transforms_gpu.clone(),
        };

        // Draw the opaque geometry front to back with the depth buffer enabled.

        if scene.show_points {
            cmd_queue.draw(&points_range_1, &opaque_pso, &opaque_pipeline::Data {
                vbo: fill_vbo.clone(),
                .. default_pipeline_data.clone()
            });
            cmd_queue.draw(&points_range_2, &opaque_pso, &opaque_pipeline::Data {
                vbo: fill_vbo.clone(),
                .. default_pipeline_data.clone()
            });
        }

        let pso = if scene.show_wireframe { &wireframe_opaque_pso }
                  else { &opaque_pso };

        cmd_queue.draw(&fill_range, &pso, &opaque_pipeline::Data {
            vbo: fill_vbo.clone(),
            .. default_pipeline_data.clone()
        });

        cmd_queue.draw(&stroke_range, &pso, &opaque_pipeline::Data {
            vbo: stroke_vbo.clone(),
            .. default_pipeline_data.clone()
        });

        cmd_queue.draw(&bg_range, &bg_pso, &bg_pipeline::Data {
            vbo: bg_vbo.clone(),
            out_color: main_fbo.clone(),
            out_depth: main_depth.clone(),
            constants: constants.clone(),
        });

        //let pso = if scene.show_wireframe { &wireframe_transparent_pso }
        //          else { &transparent_pso };
        // Non-opaque geometry should be drawn back to front here.
        // (there is none in this example)


        cmd_queue.flush(&mut device);

        window.swap_buffers().unwrap();

        //device.cleanup(); // TODO

        frame_count += 1;
    }
}

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

static BACKGROUND_VERTEX_SHADER: &'static str = &"
    #version 140
    in vec2 a_position;
    out vec2 v_position;

    void main() {
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
