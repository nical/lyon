#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate gfx_device_gl;
extern crate glutin;
extern crate lyon;
extern crate lyon_renderer;

use lyon::path_builder::*;
use lyon::math::*;
use lyon::tessellation::geometry_builder::{VertexConstructor, VertexBuffers, BuffersBuilder};
use lyon::tessellation::basic_shapes::*;
use lyon::tessellation::{StrokeTessellator, StrokeOptions};
use lyon::tessellation;
use lyon::path::Path;
use lyon::path_iterator::PathIterator;
use lyon_renderer::buffer::{Id, CpuBuffer};
use lyon_renderer::glsl::*;
use lyon::bezier::{CubicBezierSegment, Line};

use gfx::traits::{Device, FactoryExt};
use std::mem;

type ColorFormat = gfx::format::Rgba8;
type DepthFormat = gfx::format::DepthStencil;
type Vbo<T> = gfx::handle::Buffer<gfx_device_gl::Resources, T>;
type BufferObject<T> = gfx::handle::Buffer<gfx_device_gl::Resources, T>;
type IndexSlice = gfx::Slice<gfx_device_gl::Resources>;
struct GpuGeometry<T> {
    vbo: Vbo<T>,
    ibo: IndexSlice,
}

gfx_defines!{
    constant Globals {
        resolution: [f32; 2] = "u_resolution",
        scroll_offset: [f32; 2] = "u_scroll_offset",
        zoom: f32 = "u_zoom",
    }

    constant GpuTransform {
        transform: [[f32; 4]; 4] = "transform",
    }

    // Per-vertex data.
    vertex GpuFillVertex {
        position: [f32; 2] = "a_position",
        normal: [f32; 2] = "a_normal",
        prim_id: i32 = "a_prim_id", // An id pointing to the PrimData struct above.
    }

    // Per fill primitive data.
    constant GpuFillPrimitive {
        color: [f32; 4] = "color",
        z_index: f32 = "z_index",
        local_transform: i32 = "local_transform",
        view_transform: i32 = "view_transform",
        width: f32 = "width",
    }

    // Per-vertex data.
    vertex GpuStrokeVertex {
        position: [f32; 2] = "a_position",
        normal: [f32; 2] = "a_normal",
        advancement: f32 = "a_advancement",
        prim_id: i32 = "a_prim_id", // An id pointing to the PrimData struct above.
    }

    // Per stroke primitive data.
    constant GpuStrokePrimitive {
        color: [f32; 4] = "color",
        z_index: f32 = "z_index",
        local_transform: i32 = "local_transform",
        view_transform: i32 = "view_transform",
        width: f32 = "width",
    }

    pipeline opaque_fill_pipeline {
        vbo: gfx::VertexBuffer<GpuFillVertex> = (),
        out_color: gfx::RenderTarget<ColorFormat> = "out_color",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
        constants: gfx::ConstantBuffer<Globals> = "Globals",
        transforms: gfx::ConstantBuffer<GpuTransform> = "u_transforms",
        primitives: gfx::ConstantBuffer<GpuFillPrimitive> = "u_primitives",
    }

    pipeline opaque_stroke_pipeline {
        vbo: gfx::VertexBuffer<GpuStrokeVertex> = (),
        out_color: gfx::RenderTarget<ColorFormat> = "out_color",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
        constants: gfx::ConstantBuffer<Globals> = "Globals",
        transforms: gfx::ConstantBuffer<GpuTransform> = "u_transforms",
        primitives: gfx::ConstantBuffer<GpuStrokePrimitive> = "u_primitives",
    }
}

impl GpuFillPrimitive {
    pub fn new(
        color: [f32; 4],
        z_index: f32,
        local_transform: TransformId,
        view_transform: TransformId,
    ) -> GpuFillPrimitive {
        GpuFillPrimitive {
            color: color,
            z_index: z_index,
            local_transform: local_transform.to_i32(),
            view_transform: view_transform.to_i32(),
            width: 0.0,
        }
    }
}

impl std::default::Default for GpuFillPrimitive {
    fn default() -> Self {
        GpuFillPrimitive::new([1.0, 1.0, 1.0, 1.0], 0.0, TransformId::new(0), TransformId::new(0))
    }
}

impl GpuStrokePrimitive {
    pub fn new(
        color: [f32; 4],
        z_index: f32,
        local_transform: TransformId,
        view_transform: TransformId,
    ) -> GpuStrokePrimitive {
        GpuStrokePrimitive {
            color: color,
            z_index: z_index,
            local_transform: local_transform.to_i32(),
            view_transform: view_transform.to_i32(),
            width: 1.0,
        }
    }
}

impl std::default::Default for GpuStrokePrimitive {
    fn default() -> Self {
        GpuStrokePrimitive::new([1.0, 1.0, 1.0, 1.0], 0.0, TransformId::new(0), TransformId::new(0))
    }
}


impl std::default::Default for GpuTransform {
    fn default() -> Self {
        GpuTransform {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
}

impl GpuTransform {
    pub fn new(mat: Transform3D) -> Self { GpuTransform { transform: mat.to_row_arrays() } }

    pub fn as_mat4(&self) -> &Transform3D { unsafe { mem::transmute(self) } }

    pub fn as_mut_mat4(&mut self) -> &mut Transform3D { unsafe { mem::transmute(self) } }
}

pub type FillPrimitiveId = Id<GpuFillPrimitive>;
pub type StrokePrimitiveId = Id<GpuStrokePrimitive>;


/// This vertex constructor forwards the positions and normals provided by the
/// tessellators and add a shape id.
pub struct WithId<T>(pub Id<T>);

// The fill tessellator does not implement normals yet, so this implementation
// just sets it to [0, 0], for now.
impl VertexConstructor<tessellation::FillVertex, GpuFillVertex> for WithId<GpuFillPrimitive> {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> GpuFillVertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());
        assert!(!vertex.normal.x.is_nan());
        assert!(!vertex.normal.y.is_nan());
        GpuFillVertex {
            position: vertex.position.to_array(),
            normal: vertex.normal.to_array(),
            prim_id: self.0.to_i32(),
        }
    }
}

impl VertexConstructor<tessellation::StrokeVertex, GpuStrokeVertex> for WithId<GpuStrokePrimitive> {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex) -> GpuStrokeVertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());
        assert!(!vertex.normal.x.is_nan());
        assert!(!vertex.normal.y.is_nan());
        assert!(!vertex.advancement.is_nan());
        GpuStrokeVertex {
            position: vertex.position.to_array(),
            normal: vertex.normal.to_array(),
            advancement: vertex.advancement,
            prim_id: self.0.to_i32(),
        }
    }
}

gfx_defines!{
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

pub type TransformId = Id<GpuTransform>;

pub fn split_gfx_slice<R: gfx::Resources>(
    slice: gfx::Slice<R>,
    at: u32,
) -> (gfx::Slice<R>, gfx::Slice<R>) {
    let mut first = slice.clone();
    let mut second = slice.clone();
    first.end = at;
    second.start = at;

    return (first, second);
}

pub fn gfx_sub_slice<R: gfx::Resources>(slice: gfx::Slice<R>, from: u32, to: u32) -> gfx::Slice<R> {
    let mut sub = slice.clone();
    sub.start = from;
    sub.end = to;

    return sub;
}

struct BgVertexCtor;
impl VertexConstructor<tessellation::FillVertex, BgVertex> for BgVertexCtor {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> BgVertex {
        BgVertex { position: vertex.position.to_array() }
    }
}

struct Cpu {
    transforms: CpuBuffer<GpuTransform>,
    fill_primitives: CpuBuffer<GpuFillPrimitive>,
    stroke_primitives: CpuBuffer<GpuStrokePrimitive>,
    fills: VertexBuffers<GpuFillVertex>,
    strokes: VertexBuffers<GpuStrokeVertex>,
}

struct Gpu {
    transforms: BufferObject<GpuTransform>,
    fill_primitives: BufferObject<GpuFillPrimitive>,
    stroke_primitives: BufferObject<GpuStrokePrimitive>,
}

fn main() {
    println!("== gfx-rs example ==");
    println!("Controls:");
    println!("  Arrow keys: scrolling");
    println!("  PgUp/PgDown: zoom in/out");
    println!("  w: toggle wireframe mode");
    println!("  b: toggle drawing the background");
    println!("  a/z: increase/decrease the stroke width");

    let bezier = CubicBezierSegment {
        from: point(10.0, 10.0),
        ctrl1: point(20.0, 100.0),
        ctrl2: point(80.0, 100.0),
        to: point(90.0, 10.0),
    };

    let line = Line {
        point: point(5.0, 20.0),
        vector: vec2(100.0, 45.0),
    };

    let intersections = bezier.line_intersections(&line);

    // Build a Path for the rust logo.
    let mut builder = SvgPathBuilder::new(Path::builder());
    builder.move_to(bezier.from);
    builder.cubic_bezier_to(bezier.ctrl1, bezier.ctrl2, bezier.to);
    let bezier_path = builder.build();

    let mut builder = SvgPathBuilder::new(Path::builder());
    builder.move_to(line.point);
    builder.relative_line_to(line.vector);
    let line_path = builder.build();


    // Create some CPU-side buffers that will contain the geometry.
    let mut cpu = Cpu {
        fills: VertexBuffers::new(),
        strokes: VertexBuffers::new(),
        transforms: CpuBuffer::new(PRIM_BUFFER_LEN as u16),
        fill_primitives: CpuBuffer::new(PRIM_BUFFER_LEN as u16),
        stroke_primitives: CpuBuffer::new(PRIM_BUFFER_LEN as u16),
    };

    let default_transform = cpu.transforms.push(GpuTransform::default());
    let view_transform =
        cpu.transforms.push(GpuTransform::new(Transform3D::create_rotation(0.0, 0.0, 1.0, Radians::new(2.0))));

    let bezier_id = cpu.stroke_primitives.push(
        GpuStrokePrimitive::new(
            [0.0, 0.0, 0.0, 0.1],
            0.2,
            default_transform,
            view_transform,
        )
    );
    let line_id = cpu.stroke_primitives.push(
        GpuStrokePrimitive::new(
            [0.0, 0.0, 0.0, 0.1],
            0.3,
            default_transform,
            view_transform,
        )
    );

    StrokeTessellator::new().tessellate_flattened_path(
        bezier_path.path_iter().flattened(0.01),
        &StrokeOptions::default().dont_apply_line_width(),
        &mut BuffersBuilder::new(&mut cpu.strokes, WithId(bezier_id)),
    );
    StrokeTessellator::new().tessellate_flattened_path(
        line_path.path_iter().flattened(0.01),
        &StrokeOptions::default().dont_apply_line_width(),
        &mut BuffersBuilder::new(&mut cpu.strokes, WithId(line_id)),
    );

    let num_points = intersections.len() as u16;

    //let stroke_ids = cpu.stroke_primitives.alloc_range(2);
    let point_transforms = cpu.transforms.alloc_range(num_points);
    let point_ids_1 = cpu.fill_primitives.alloc_range(num_points);
    let point_ids_2 = cpu.fill_primitives.alloc_range(num_points);

    let circle_indices_start = cpu.fills.indices.len() as u32;
    let circle_count = fill_circle(
        point(0.0, 0.0),
        2.0,
        0.01,
        &mut BuffersBuilder::new(
            &mut cpu.fills,
            WithId(point_ids_1.start())
        ),
    );
    fill_circle(
        point(0.0, 0.0),
        1.5,
        0.01,
        &mut BuffersBuilder::new(
            &mut cpu.fills,
            WithId(point_ids_2.start())
        ),
    );

    println!(" -- intersections {:?}", intersections);
    for (i, p) in intersections.iter().enumerate() {
        let i = i as u16;
        let transform_id = point_transforms.get(i);
        cpu.transforms[point_transforms.get(i)].transform =
            Transform3D::create_translation(p.x, p.y, 0.0).to_row_arrays();
        cpu.fill_primitives[point_ids_1.get(i)] = GpuFillPrimitive::new(
            [0.0, 0.2, 0.0, 1.0],
            0.3,
            transform_id,
            view_transform,
        );
        cpu.fill_primitives[point_ids_2.get(i)] = GpuFillPrimitive::new(
            [0.0, 1.0, 0.0, 1.0],
            0.4,
            transform_id,
            view_transform,
        );
    }

    let mut bg_mesh_cpu: VertexBuffers<BgVertex> = VertexBuffers::new();
    fill_rectangle(
        &Rect::new(point(-1.0, -1.0), size(2.0, 2.0)),
        &mut BuffersBuilder::new(&mut bg_mesh_cpu, BgVertexCtor),
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

    let gpu = Gpu {
        transforms: factory.create_constant_buffer(PRIM_BUFFER_LEN),
        fill_primitives: factory.create_constant_buffer(PRIM_BUFFER_LEN),
        stroke_primitives: factory.create_constant_buffer(PRIM_BUFFER_LEN),
    };

    let bg_pso = factory.create_pipeline_simple(
        BACKGROUND_VERTEX_SHADER.as_bytes(),
        BACKGROUND_FRAGMENT_SHADER.as_bytes(),
        bg_pipeline::new(),
    ).unwrap();

    let (bg_vbo, bg_range) = factory.create_vertex_buffer_with_slice(
        &bg_mesh_cpu.vertices[..],
        &bg_mesh_cpu.indices[..]
    );

    let fill_shader = factory.link_program(
        FILL_VERTEX_SHADER.as_bytes(),
        FILL_FRAGMENT_SHADER.as_bytes()
    ).unwrap();

    let stroke_shader = factory.link_program(
        STROKE_VERTEX_SHADER.as_bytes(),
        STROKE_FRAGMENT_SHADER.as_bytes()
    ).unwrap();

    let opaque_fill_pso = factory.create_pipeline_from_program(
        &fill_shader,
        gfx::Primitive::TriangleList,
        gfx::state::Rasterizer::new_fill(),
        opaque_fill_pipeline::new(),
    ).unwrap();

    let opaque_stroke_pso = factory.create_pipeline_from_program(
        &stroke_shader,
        gfx::Primitive::TriangleList,
        gfx::state::Rasterizer::new_fill(),
        opaque_stroke_pipeline::new(),
    ).unwrap();

    let mut fill_mode = gfx::state::Rasterizer::new_fill();
    fill_mode.method = gfx::state::RasterMethod::Line(1);
    let wireframe_stroke_pso = factory.create_pipeline_from_program(
        &stroke_shader,
        gfx::Primitive::TriangleList,
        fill_mode,
        opaque_stroke_pipeline::new(),
    ).unwrap();

    let mut init_queue: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let (vbo, ibo) = factory.create_vertex_buffer_with_slice(
        &cpu.fills.vertices[..],
        &cpu.fills.indices[..]
    );
    let gpu_fills = GpuGeometry { vbo: vbo, ibo: ibo };

    let (vbo, ibo) = factory.create_vertex_buffer_with_slice(
        &cpu.strokes.vertices[..],
        &cpu.strokes.indices[..]
    );
    let gpu_strokes = GpuGeometry { vbo: vbo, ibo: ibo };

    init_queue.update_buffer(&gpu.fill_primitives, cpu.fill_primitives.as_slice(), 0).unwrap();
    init_queue.update_buffer(&gpu.stroke_primitives, cpu.stroke_primitives.as_slice(), 0).unwrap();
    init_queue.update_buffer(&gpu.transforms, cpu.transforms.as_slice(), 0).unwrap();

    let split = circle_indices_start + (circle_count.indices as u32);
    let mut points_range_1 = gfx_sub_slice(gpu_fills.ibo.clone(), circle_indices_start, split);
    let mut points_range_2 =
        gfx_sub_slice(gpu_fills.ibo.clone(), split, split + circle_count.indices as u32);
    points_range_1.instances = Some((num_points as u32, 0));
    points_range_2.instances = Some((num_points as u32, 0));

    let mut scene = SceneParams {
        target_zoom: 5.0,
        zoom: 0.5,
        target_scroll: vec2(70.0, 70.0),
        scroll: vec2(70.0, 70.0),
        show_points: false,
        show_wireframe: false,
        stroke_width: 0.0,
        target_stroke_width: 1.0,
        draw_background: true,
    };

    let mut cmd_queue: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut frame_count: usize = 0;
    loop {
        if !update_inputs(&window, &mut scene) {
            break;
        }

        cpu.stroke_primitives[bezier_id].color =
            [
                (frame_count as f32 * 0.008 - 1.6).sin() * 0.1 + 0.1,
                (frame_count as f32 * 0.005 - 1.6).sin() * 0.1 + 0.1,
                (frame_count as f32 * 0.01 - 1.6).sin() * 0.1 + 0.1,
                1.0,
            ];
        cpu.stroke_primitives[bezier_id].width = scene.stroke_width;
        cpu.stroke_primitives[line_id].width = scene.stroke_width;

        gfx_window_glutin::update_views(&window, &mut main_fbo, &mut main_depth);
        let (w, h) = window.get_inner_size_pixels().unwrap();

        *cpu.transforms[view_transform].as_mut_mat4() = Transform3D::create_translation(
            -scene.scroll.x as f32,
            -scene.scroll.y as f32, 0.0
        ).post_scale(scene.zoom, scene.zoom, 1.0);

        cmd_queue.clear(&main_fbo.clone(), [0.0, 0.0, 0.0, 0.0]);
        cmd_queue.clear_depth(&main_depth.clone(), 1.0);

        cmd_queue.update_buffer(&gpu.fill_primitives, cpu.fill_primitives.as_slice(), 0).unwrap();
        cmd_queue.update_buffer(&gpu.stroke_primitives, cpu.stroke_primitives.as_slice(), 0).unwrap();
        cmd_queue.update_buffer(&gpu.transforms, cpu.transforms.as_slice(), 0).unwrap();

        cmd_queue.update_constant_buffer(
            &constants,
            &Globals {
                resolution: [w as f32, h as f32],
                zoom: scene.zoom,
                scroll_offset: scene.scroll.to_array(),
            },
        );

        cmd_queue.draw(
            &points_range_1,
            &opaque_fill_pso,
            &opaque_fill_pipeline::Data {
                vbo: gpu_fills.vbo.clone(),
                primitives: gpu.fill_primitives.clone(),
                transforms: gpu.transforms.clone(),
                constants: constants.clone(),
                out_color: main_fbo.clone(),
                out_depth: main_depth.clone(),
            },
        );
        cmd_queue.draw(
            &points_range_2,
            &opaque_fill_pso,
            &opaque_fill_pipeline::Data {
                vbo: gpu_fills.vbo.clone(),
                primitives: gpu.fill_primitives.clone(),
                transforms: gpu.transforms.clone(),
                constants: constants.clone(),
                out_color: main_fbo.clone(),
                out_depth: main_depth.clone(),
            },
        );

        let stroke_pso = if scene.show_wireframe { &wireframe_stroke_pso } else { &opaque_stroke_pso };

        cmd_queue.draw(
            &gpu_strokes.ibo,
            &stroke_pso,
            &opaque_stroke_pipeline::Data {
                vbo: gpu_strokes.vbo.clone(),
                primitives: gpu.stroke_primitives.clone(),
                transforms: gpu.transforms.clone(),
                constants: constants.clone(),
                out_color: main_fbo.clone(),
                out_depth: main_depth.clone(),
            },
        );

        if scene.draw_background {
            cmd_queue.draw(
                &bg_range,
                &bg_pso,
                &bg_pipeline::Data {
                    vbo: bg_vbo.clone(),
                    out_color: main_fbo.clone(),
                    out_depth: main_depth.clone(),
                    constants: constants.clone(),
                },
            );
        }

        // Non-opaque geometry should be drawn back to front here.
        // (there is none in this example)

        cmd_queue.flush(&mut device);

        window.swap_buffers().unwrap();

        device.cleanup();

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
    draw_background: bool,
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
                    VirtualKeyCode::B => {
                        scene.draw_background = !scene.draw_background;
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
    scene.stroke_width = scene.stroke_width +
        (scene.target_stroke_width - scene.stroke_width) / 5.0;

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
