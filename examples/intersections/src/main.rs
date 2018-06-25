#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate gfx_device_gl;
extern crate glutin;
extern crate lyon;

use lyon::path::builder::*;
use lyon::geom::{Line, CubicBezierSegment, Arc};
use lyon::math::*;
use lyon::tessellation::geometry_builder::{VertexConstructor, VertexBuffers, BuffersBuilder};
use lyon::tessellation::basic_shapes::*;
use lyon::tessellation::{StrokeTessellator, StrokeOptions, FillOptions};
use lyon::tessellation;
use lyon::path::default::Path;

use gfx::traits::{Device, FactoryExt};

use glutin::{GlContext, EventsLoop, KeyboardInput};
use glutin::ElementState::Pressed;

pub fn split_gfx_slice<R: gfx::Resources>(
    slice: gfx::Slice<R>,
    at: u32,
) -> (gfx::Slice<R>, gfx::Slice<R>) {
    let mut first = slice.clone();
    let mut second = slice.clone();
    first.end = at;
    second.start = at;

    (first, second)
}

pub fn gfx_sub_slice<R: gfx::Resources>(slice: gfx::Slice<R>, from: u32, to: u32) -> gfx::Slice<R> {
    let mut sub = slice.clone();
    sub.start = from;
    sub.end = to;

    sub
}

const DEFAULT_WINDOW_WIDTH: f32 = 800.0;
const DEFAULT_WINDOW_HEIGHT: f32 = 800.0;

fn main() {
    println!("== gfx-rs example ==");
    println!("Controls:");
    println!("  Arrow keys: scrolling");
    println!("  PgUp/PgDown: zoom in/out");
    println!("  w: toggle wireframe mode");
    println!("  b: toggle drawing the background");
    println!("  a/z: increase/decrease the stroke width");


    let tolerance = 0.001;

    let bezier = CubicBezierSegment {
        from: point(10.0, 10.0),
        ctrl1: point(20.0, 100.0),
        ctrl2: point(80.0, 100.0),
        to: point(90.0, 10.0),
    };

    let line = Line {
        point: point(5.0, 20.0),
        vector: vector(100.0, 45.0),
    };

    //let mut builder = SvgPathBuilder::new(Path::builder());
    //builder.move_to(bezier.from);
    //builder.cubic_bezier_to(bezier.ctrl1, bezier.ctrl2, bezier.to);
    //let bezier_path = builder.build();

    let arc = Arc {
        center: point(0.0, 0.0),
        radii: vector(100.0, 100.0),
        start_angle: Angle::pi(),
        sweep_angle: Angle::pi(),
        x_rotation: -Angle::pi() * 0.25,
    };

    let r = arc.bounding_rect();

    let mut builder = Path::builder();
    builder.move_to(arc.from());
    builder.arc(arc.center, arc.radii, arc.sweep_angle, arc.x_rotation);
    builder.close();
    builder.move_to(r.origin);
    builder.line_to(r.top_right());
    builder.line_to(r.bottom_right());
    builder.line_to(r.bottom_left());
    builder.close();
    let bezier_path = builder.build();


    let mut builder = SvgPathBuilder::new(Path::builder());
    builder.move_to(line.point);
    builder.relative_line_to(line.vector);
    let line_path = builder.build();

    //let intersections = bezier.line_intersections(&line);
    let mut intersections = Vec::new();
    arc.for_each_local_x_extremum_t(&mut|t| {
        intersections.push(arc.sample(t));
    });
    arc.for_each_local_y_extremum_t(&mut|t| {
        intersections.push(arc.sample(t));
    });
    let num_points = intersections.len() as u16;

    let mut geometry: VertexBuffers<GpuVertex, u16> = VertexBuffers::new();

    let line_id = 0;
    let bezier_id = 1;
    let point_ids_1 = 2;
    let point_ids_2 = point_ids_1 + num_points as i32;

    let stroke_options = StrokeOptions::tolerance(tolerance).dont_apply_line_width();
    StrokeTessellator::new().tessellate_path(
        bezier_path.path_iter(),
        &stroke_options,
        &mut BuffersBuilder::new(
            &mut geometry,
            WithId(bezier_id)
        ),
    );
    StrokeTessellator::new().tessellate_path(
        line_path.path_iter(),
        &stroke_options,
        &mut BuffersBuilder::new(
            &mut geometry,
            WithId(line_id)
        ),
    );


    let circle_indices_start = geometry.indices.len() as u32;

    fill_circle(
        point(0.0, 0.0),
        1.0,
        &FillOptions::tolerance(0.01),
        &mut BuffersBuilder::new(
            &mut geometry,
            WithId(point_ids_1)
        ),
    );

    let mut bg_geometry: VertexBuffers<BgVertex, u16> = VertexBuffers::new();
    fill_rectangle(
        &Rect::new(point(-1.0, -1.0), size(2.0, 2.0)),
        &FillOptions::default(),
        &mut BuffersBuilder::new(&mut bg_geometry, BgVertexCtor),
    );

    let mut cpu_primitives = Vec::with_capacity(PRIM_BUFFER_LEN);
    for _ in 0..PRIM_BUFFER_LEN {
        cpu_primitives.push(
            Primitive {
                color: [1.0, 0.0, 0.0, 1.0],
                z_index: 0,
                width: 0.0,
                translate: [0.0, 0.0],
            },
        );
    }

    cpu_primitives[line_id as usize] = Primitive {
        color: [0.0, 0.0, 0.0, 1.0],
        z_index: 1,
        width: 0.2,
        translate: [0.0, 0.0],
    };

    cpu_primitives[bezier_id as usize] = Primitive {
        color: [0.0, 0.0, 0.0, 1.0],
        z_index: 2,
        width: 0.2,
        translate: [0.0, 0.0],
    };

    // Intance primitives
    println!(" -- intersections {:?}", intersections);
    for (i, intersection) in intersections.iter().enumerate() {
        let pos = intersection.to_array();
        cpu_primitives[point_ids_1 as usize + i] = Primitive {
            color: [0.0, 0.2, 0.0, 1.0],
            z_index: 3,
            width: 2.0,
            translate: pos,
        };
        cpu_primitives[point_ids_2 as usize + i] = Primitive {
            color: [0.0, 1.0, 0.0, 1.0],
            z_index: 4,
            width: 1.0,
            translate: pos,
        };
    }

    let glutin_builder = glutin::WindowBuilder::new()
        .with_dimensions(DEFAULT_WINDOW_WIDTH as u32, DEFAULT_WINDOW_HEIGHT as u32)
        .with_decorations(true)
        .with_title("lyon".to_string());

    let context = glutin::ContextBuilder::new().with_vsync(true);

    let mut events_loop = glutin::EventsLoop::new();

    let (window, mut device, mut factory, mut main_fbo, mut main_depth) =
        gfx_window_glutin::init::<gfx::format::Rgba8, gfx::format::DepthStencil>(glutin_builder, context, &events_loop);

    let bg_pso = factory.create_pipeline_simple(
        BACKGROUND_VERTEX_SHADER.as_bytes(),
        BACKGROUND_FRAGMENT_SHADER.as_bytes(),
        bg_pipeline::new(),
    ).unwrap();

    let path_shader = factory.link_program(
        VERTEX_SHADER.as_bytes(),
        FRAGMENT_SHADER.as_bytes()
    ).unwrap();

    let path_pso = factory.create_pipeline_from_program(
        &path_shader,
        gfx::Primitive::TriangleList,
        gfx::state::Rasterizer::new_fill().with_cull_back(),
        path_pipeline::new(),
    ).unwrap();

    let mut wireframe_fill_mode = gfx::state::Rasterizer::new_fill();
    wireframe_fill_mode.method = gfx::state::RasterMethod::Line(1);
    let wireframe_pso = factory.create_pipeline_from_program(
        &path_shader,
        gfx::Primitive::TriangleList,
        wireframe_fill_mode,
        path_pipeline::new(),
    ).unwrap();

    let (bg_vbo, bg_range) = factory.create_vertex_buffer_with_slice(
        &bg_geometry.vertices[..],
        &bg_geometry.indices[..]
    );

    let (path_vbo, path_range) = factory.create_vertex_buffer_with_slice(
        &geometry.vertices[..],
        &geometry.indices[..]
    );

    let (stroke_range, mut points_range) = split_gfx_slice(path_range, circle_indices_start);
    points_range.instances = Some((2 * num_points as u32, 0));

    let gpu_primitives = factory.create_constant_buffer(PRIM_BUFFER_LEN);
    let constants = factory.create_constant_buffer(1);

    let mut scene = SceneParams {
        target_zoom: 5.0,
        zoom: 0.1,
        target_scroll: vector(70.0, 70.0),
        scroll: vector(70.0, 70.0),
        show_points: false,
        show_wireframe: false,
        stroke_width: 1.0,
        target_stroke_width: 1.0,
        draw_background: true,
        cursor_position: (0.0, 0.0),
        window_size: (DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT),
    };

    let mut cmd_queue: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    while update_inputs(&mut events_loop, &mut scene) {
        gfx_window_glutin::update_views(&window, &mut main_fbo, &mut main_depth);
        let (w, h) = window.get_inner_size().unwrap();
        scene.window_size = (w as f32, h as f32);

        cmd_queue.clear(&main_fbo.clone(), [0.0, 0.0, 0.0, 0.0]);
        cmd_queue.clear_depth(&main_depth.clone(), 1.0);

        cpu_primitives[line_id as usize].width = scene.stroke_width;
        cpu_primitives[bezier_id as usize].width = scene.stroke_width;

        cmd_queue.update_constant_buffer(
            &constants,
            &Globals {
                resolution: [w as f32, h as f32],
                zoom: scene.zoom,
                scroll_offset: scene.scroll.to_array(),
            },
        );

        cmd_queue.update_buffer(
            &gpu_primitives,
            &cpu_primitives[..],
            0
        ).unwrap();

        let pso = if scene.show_wireframe {
            &wireframe_pso
        } else {
            &path_pso
        };

        cmd_queue.draw(
            &stroke_range,
            &pso,
            &path_pipeline::Data {
                vbo: path_vbo.clone(),
                primitives: gpu_primitives.clone(),
                constants: constants.clone(),
                out_color: main_fbo.clone(),
                out_depth: main_depth.clone(),
            },
        );

        cmd_queue.draw(
            &points_range,
            &pso,
            &path_pipeline::Data {
                vbo: path_vbo.clone(),
                primitives: gpu_primitives.clone(),
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

        cmd_queue.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();
    }
}

gfx_defines!{
    constant Globals {
        resolution: [f32; 2] = "u_resolution",
        scroll_offset: [f32; 2] = "u_scroll_offset",
        zoom: f32 = "u_zoom",
    }

    vertex GpuVertex {
        position: [f32; 2] = "a_position",
        normal: [f32; 2] = "a_normal",
        prim_id: i32 = "a_prim_id", // An id pointing to the PrimData struct above.
    }

    constant Primitive {
        color: [f32; 4] = "color",
        z_index: i32 = "z_index",
        width: f32 = "width",
        translate: [f32; 2] = "translate",
    }

    pipeline path_pipeline {
        vbo: gfx::VertexBuffer<GpuVertex> = (),
        out_color: gfx::RenderTarget<gfx::format::Rgba8> = "out_color",
        out_depth: gfx::DepthTarget<gfx::format::DepthStencil> = gfx::preset::depth::LESS_EQUAL_WRITE,
        constants: gfx::ConstantBuffer<Globals> = "Globals",
        primitives: gfx::ConstantBuffer<Primitive> = "u_primitives",
    }

    vertex BgVertex {
        position: [f32; 2] = "a_position",
    }

    pipeline bg_pipeline {
        vbo: gfx::VertexBuffer<BgVertex> = (),
        out_color: gfx::RenderTarget<gfx::format::Rgba8> = "out_color",
        out_depth: gfx::DepthTarget<gfx::format::DepthStencil> = gfx::preset::depth::LESS_EQUAL_WRITE,
        constants: gfx::ConstantBuffer<Globals> = "Globals",
    }
}

struct BgVertexCtor;
impl VertexConstructor<tessellation::FillVertex, BgVertex> for BgVertexCtor {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> BgVertex {
        BgVertex { position: vertex.position.to_array() }
    }
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

const PRIM_BUFFER_LEN: usize = 64;

pub static VERTEX_SHADER: &'static str = &"
    #version 140

    #define PRIM_BUFFER_LEN 64

    uniform Globals {
        vec2 u_resolution;
        vec2 u_scroll_offset;
        float u_zoom;
    };

    struct Primitive {
        vec4 color;
        int z_index;
        float width;
        vec2 translate;
    };
    uniform u_primitives { Primitive primitives[PRIM_BUFFER_LEN]; };

    in vec2 a_position;
    in vec2 a_normal;
    in int a_prim_id;

    out vec4 v_color;

    void main() {
        int id = a_prim_id + gl_InstanceID;
        Primitive prim = primitives[id];

        vec2 local_pos = a_position + a_normal * prim.width;
        vec2 world_pos = local_pos - u_scroll_offset + prim.translate;
        vec2 transformed_pos = world_pos * u_zoom / (vec2(0.5, -0.5) * u_resolution);

        float z = float(prim.z_index) / 4096.0;
        gl_Position = vec4(transformed_pos, 1.0 - z, 1.0);
        v_color = prim.color;
    }
";

pub static FRAGMENT_SHADER: &'static str = &"
    #version 140
    in vec4 v_color;
    out vec4 out_color;

    void main() {
        out_color = v_color;
    }
";

/// This vertex constructor forwards the positions and normals provided by the
/// tessellators and add a shape id.
pub struct WithId(pub i32);

impl VertexConstructor<tessellation::FillVertex, GpuVertex> for WithId {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> GpuVertex {
        debug_assert!(!vertex.position.x.is_nan());
        debug_assert!(!vertex.position.y.is_nan());
        debug_assert!(!vertex.normal.x.is_nan());
        debug_assert!(!vertex.normal.y.is_nan());
        GpuVertex {
            position: vertex.position.to_array(),
            normal: vertex.normal.to_array(),
            prim_id: self.0,
        }
    }
}

impl VertexConstructor<tessellation::StrokeVertex, GpuVertex> for WithId {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex) -> GpuVertex {
        debug_assert!(!vertex.position.x.is_nan());
        debug_assert!(!vertex.position.y.is_nan());
        debug_assert!(!vertex.normal.x.is_nan());
        debug_assert!(!vertex.normal.y.is_nan());
        debug_assert!(!vertex.advancement.is_nan());
        GpuVertex {
            position: vertex.position.to_array(),
            normal: vertex.normal.to_array(),
            prim_id: self.0,
        }
    }
}

struct SceneParams {
    target_zoom: f32,
    zoom: f32,
    target_scroll: Vector,
    scroll: Vector,
    show_points: bool,
    show_wireframe: bool,
    stroke_width: f32,
    target_stroke_width: f32,
    draw_background: bool,
    cursor_position: (f32, f32),
    window_size: (f32, f32),
}

fn update_inputs(events_loop: &mut EventsLoop, scene: &mut SceneParams) -> bool {
    use glutin::Event;
    use glutin::VirtualKeyCode;
    use glutin::WindowEvent;

    let mut status = true;

    events_loop.poll_events(|event| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::Closed,
                ..
            } => {
                status = false;
            },
            Event::WindowEvent {
                event: WindowEvent::MouseInput {
                    state: glutin::ElementState::Pressed, button: glutin::MouseButton::Left,
                ..},
            ..} => {
                let half_width = scene.window_size.0 * 0.5;
                let half_height = scene.window_size.1 * 0.5;
                println!("X: {}, Y: {}",
                    (scene.cursor_position.0 - half_width) / scene.zoom + scene.scroll.x,
                    (scene.cursor_position.1 - half_height) / scene.zoom + scene.scroll.y,
                );
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved {
                    position: (x, y),
                    ..},
            ..} => {
                scene.cursor_position = (x as f32, y as f32);
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput {
                    input: KeyboardInput {
                        state: Pressed,
                        virtual_keycode: Some(key),
                        ..
                    },
                    ..
                },
                ..
            } => {
                match key {
                    VirtualKeyCode::Escape => {
                        status = false;
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
                        scene.target_stroke_width /= 0.8;
                    }
                    VirtualKeyCode::Z => {
                        scene.target_stroke_width *= 0.8;
                    }
                    _key => {}
                }
            }
            _evt => {}
        }
    });

    scene.zoom += (scene.target_zoom - scene.zoom) / 3.0;
    scene.scroll = scene.scroll + (scene.target_scroll - scene.scroll) / 3.0;
    scene.stroke_width = scene.stroke_width +
        (scene.target_stroke_width - scene.stroke_width) / 5.0;

    return status;
}
