#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate gfx_device_gl;
extern crate glutin;
extern crate lyon;

use lyon::extra::rust_logo::build_logo_path;
use lyon::path::Path;
use lyon::path::builder::*;
use lyon::path::iterator::*;
use lyon::math::*;
use lyon::tessellation::geometry_builder::{VertexConstructor, VertexBuffers, BuffersBuilder};
use lyon::tessellation::basic_shapes::*;
use lyon::tessellation::{FillTessellator, FillOptions};
use lyon::algorithms::walk;

use gfx::traits::{Device, FactoryExt};

use glutin::{EventsLoop, KeyboardInput};
use glutin::ElementState::Pressed;
use glutin::dpi::LogicalSize;

use std::ops::Rem;

const DEFAULT_WINDOW_WIDTH: f32 = 800.0;
const DEFAULT_WINDOW_HEIGHT: f32 = 800.0;

fn main() {
    println!("== Lyon path walking example ==");
    println!("Controls:");
    println!("  Arrow keys: scrolling");
    println!("  PgUp/PgDown: zoom in/out");
    println!("  w: toggle wireframe mode");
    println!("  b: toggle drawing the background");
    println!("  a/z: increase/decrease the arrow spacing");

    let tolerance = 0.002;

    // Build a Path for the rust logo.
    let mut builder = SvgPathBuilder::new(Path::builder());
    build_logo_path(&mut builder);
    let logo_path = builder.build();

    // Build a Path for the arrow.
    let mut builder = Path::builder();
    builder.move_to(point(-1.0, -0.3));
    builder.line_to(point(0.0, -0.3));
    builder.line_to(point(0.0, -1.0));
    builder.line_to(point(1.5, 0.0));
    builder.line_to(point(0.0, 1.0));
    builder.line_to(point(0.0, 0.3));
    builder.line_to(point(-1.0, 0.3));
    builder.close();

    let arrow_path = builder.build();

    let mut geometry: VertexBuffers<GpuVertex, u16> = VertexBuffers::new();

    FillTessellator::new().tessellate_path(
        &arrow_path,
        &FillOptions::tolerance(tolerance),
        &mut BuffersBuilder::new(&mut geometry, WithId(0))
    ).unwrap();

    let mut bg_geometry: VertexBuffers<BgVertex, u16> = VertexBuffers::new();
    fill_rectangle(
        &Rect::new(point(-1.0, -1.0), size(2.0, 2.0)),
        &FillOptions::default(),
        &mut BuffersBuilder::new(&mut bg_geometry, BgVertexCtor),
    ).unwrap();

    let mut cpu_primitives = Vec::with_capacity(PRIM_BUFFER_LEN);
    for _ in 0..PRIM_BUFFER_LEN {
        cpu_primitives.push(Primitive {
            position: [0.0, 0.0],
            angle: 0.0,
            z_index: 0,
        });
    }

    let glutin_builder = glutin::WindowBuilder::new()
        .with_dimensions(LogicalSize { width: DEFAULT_WINDOW_WIDTH as f64, height: DEFAULT_WINDOW_HEIGHT as f64 })
        .with_decorations(true)
        .with_title("lyon".to_string());

    let context = glutin::ContextBuilder::new().with_vsync(true);

    let mut events_loop = glutin::EventsLoop::new();

    let (window, mut device, mut factory, mut main_fbo, mut main_depth) =
        gfx_window_glutin::init::<gfx::format::Rgba8, gfx::format::DepthStencil>(glutin_builder, context, &events_loop).unwrap();

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

    let (path_vbo, mut path_range) = factory.create_vertex_buffer_with_slice(
        &geometry.vertices[..],
        &geometry.indices[..]
    );

    let gpu_primitives = factory.create_constant_buffer(PRIM_BUFFER_LEN);
    let constants = factory.create_constant_buffer(1);

    let mut scene = SceneParams {
        target_zoom: 5.0,
        zoom: 0.1,
        target_scroll: vector(70.0, 70.0),
        scroll: vector(70.0, 70.0),
        show_points: false,
        show_wireframe: false,
        arrow_spacing: 0.0,
        target_arrow_spacing: 3.0,
        draw_background: true,
        cursor_position: (0.0, 0.0),
        window_size: (DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT),
    };

    let mut cmd_queue: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut frame_count: usize   = 0;
    while update_inputs(&mut events_loop, &mut scene) {
        gfx_window_glutin::update_views(&window, &mut main_fbo, &mut main_depth);
        let size = window.get_inner_size().unwrap();
        scene.window_size = (size.width as f32, size.height as f32);

        cmd_queue.clear(&main_fbo.clone(), [1.0, 1.0, 1.0, 1.0]);
        cmd_queue.clear_depth(&main_depth.clone(), 1.0);

        let mut i = 0;
        {
            let offset = (frame_count as f32 * 0.1).rem(3.0+3.0+scene.arrow_spacing);
            // Walk along the logo and apply the pattern. This will invoke
            // the pattern's callback that fills the primitive buffer.
            walk::walk_along_path(
                logo_path.iter().flattened(0.01),
                offset,
                &mut walk::RepeatedPattern {
                    callback: |pos: Point, tangent: Vector, _| {
                        if i >= PRIM_BUFFER_LEN {
                            // Don't want to overflow the primitive buffer,
                            // just skip the remaining arrows.
                            return false;
                        }
                        cpu_primitives[i] = Primitive {
                            position: pos.to_array(),
                            angle: tangent.angle_from_x_axis().get(),
                            z_index: 1,
                        };
                        i += 1;
                        true
                    },
                    intervals: &[scene.arrow_spacing, 3.0, 3.0],
                    index: 0,
                }
            );
        }
        path_range.instances = Some((i as u32, 0));

        cmd_queue.update_constant_buffer(
            &constants,
            &Globals {
                resolution: [size.width as f32, size.height as f32],
                zoom: scene.zoom,
                scroll_offset: scene.scroll.to_array(),
                arrow_scale: 1.0,
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
            &path_range,
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

        frame_count += 1;
    }
}

gfx_defines!{
    constant Globals {
        resolution: [f32; 2] = "u_resolution",
        scroll_offset: [f32; 2] = "u_scroll_offset",
        zoom: f32 = "u_zoom",
        arrow_scale: f32 = "u_arrow_scale",
    }

    vertex GpuVertex {
        position: [f32; 2] = "a_position",
        prim_id: i32 = "a_prim_id",
    }

    constant Primitive {
        position: [f32; 2] = "position",
        angle: f32 = "angle",
        z_index: i32 = "z_index",
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
impl VertexConstructor<Point, BgVertex> for BgVertexCtor {
    fn new_vertex(&mut self, vertex: Point) -> BgVertex {
        BgVertex { position: vertex.to_array() }
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
    layout(std140) uniform Globals {
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

const PRIM_BUFFER_LEN: usize = 1024;

pub static VERTEX_SHADER: &'static str = &"
    #version 140

    #define PRIM_BUFFER_LEN 1024

    layout(std140) uniform Globals {
        vec2 u_resolution;
        vec2 u_scroll_offset;
        float u_zoom;
        float u_arrow_scale;
    };

    struct Primitive {
        vec2 position;
        float angle;
        int z_index;
    };
    layout(std140) uniform u_primitives { Primitive primitives[PRIM_BUFFER_LEN]; };

    in vec2 a_position;
    in int a_prim_id;

    void main() {
        int id = a_prim_id + gl_InstanceID;
        Primitive prim = primitives[id];
        mat2 rotation = mat2(
            cos(prim.angle), -sin(prim.angle),
            sin(prim.angle), cos(prim.angle)
        );
        vec2 local_pos = a_position * rotation * u_arrow_scale;
        vec2 world_pos = local_pos - u_scroll_offset + prim.position;
        vec2 transformed_pos = world_pos * u_zoom / (vec2(0.5, -0.5) * u_resolution);

        float z = float(prim.z_index) / 4096.0;
        gl_Position = vec4(transformed_pos, 1.0 - z, 1.0);
    }
";

pub static FRAGMENT_SHADER: &'static str = &"
    #version 140
    out vec4 out_color;

    void main() {
        out_color = vec4(0.0, 0.0, 0.0, 1.0);
    }
";

/// This vertex constructor forwards the positions and normals provided by the
/// tessellators and add a shape id.
pub struct WithId(pub i32);

impl VertexConstructor<Point, GpuVertex> for WithId {
    fn new_vertex(&mut self, vertex: Point) -> GpuVertex {
        debug_assert!(!vertex.x.is_nan());
        debug_assert!(!vertex.y.is_nan());
        GpuVertex {
            position: vertex.to_array(),
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
    arrow_spacing: f32,
    target_arrow_spacing: f32,
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
                event: WindowEvent::Destroyed,
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
                    position,
                    ..},
            ..} => {
                scene.cursor_position = (position.x as f32, position.y as f32);
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
                        scene.target_arrow_spacing /= 0.9;
                    }
                    VirtualKeyCode::Z => {
                        scene.target_arrow_spacing *= 0.9;
                    }
                    _key => {}
                }
            }
            _evt => {
                //println!("{:?}", _evt);
            }
        }
        //println!(" -- zoom: {}, scroll: {:?}", scene.target_zoom, scene.target_scroll);
    });

    scene.zoom += (scene.target_zoom - scene.zoom) / 3.0;
    scene.scroll = scene.scroll + (scene.target_scroll - scene.scroll) / 3.0;
    scene.arrow_spacing = scene.arrow_spacing +
        (scene.target_arrow_spacing - scene.arrow_spacing) / 5.0;

    return status;
}
