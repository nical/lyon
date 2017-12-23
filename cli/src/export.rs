use lyon::math::*;
use lyon::tessellation::geometry_builder::{VertexConstructor, VertexBuffers, BuffersBuilder};
use lyon::tessellation::basic_shapes::*;
use lyon::tessellation::{FillTessellator, StrokeTessellator};
use lyon::tessellation;
use commands::{TessellateCmd, RenderCmd, AntiAliasing};
use std::borrow::Borrow;
use std::io::Write;
use std::str::FromStr;
use show;
use std::f32;

use gfx;
use gfx_window_glutin;
use gfx::traits::{Device, FactoryExt};
use glutin;
use glutin::{GlContext, EventsLoop, KeyboardInput};
use glutin::ElementState::Pressed;

const DEFAULT_WINDOW_WIDTH: f32 = 800.0;
const DEFAULT_WINDOW_HEIGHT: f32 = 800.0;

pub struct FileFormatParseError;

pub enum FileFormat {
    JPG,
    PNG,
}

impl FromStr for FileFormat {
    type Err = FileFormatParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().borrow() {
            "PNG" => Ok(FileFormat::PNG),
            "JPG" => Ok(FileFormat::JPG),
            _ => {
                println!("Wrong format provided, falling back to PNG.");
                Ok(FileFormat::PNG)
            }
        }
    }
}

pub fn export_path(cmd: TessellateCmd, render_options: RenderCmd, format: FileFormat, output: Box<Write>) {
    render_window(cmd, render_options);
}

fn render_window(cmd: TessellateCmd, render_options: RenderCmd) {
    let mut geometry: VertexBuffers<GpuVertex> = VertexBuffers::new();
    let mut stroke_width = 1.0;
    if let Some(mut options) = cmd.stroke {
        stroke_width = options.line_width;
        options.apply_line_width = false;
        StrokeTessellator::new().tessellate_path(
            cmd.path.path_iter(),
            &options,
            &mut BuffersBuilder::new(&mut geometry, WithId(1)),
        );
    }

    if let Some(options) = cmd.fill {
        FillTessellator::new().tessellate_path(
            cmd.path.path_iter(),
            &options,
            &mut BuffersBuilder::new(&mut geometry, WithId(0)),
        ).unwrap();
    }

    println!("Geometry : {:?}", geometry);

    if geometry.vertices.is_empty() {
        println!("No geometry to show");
        return;
    }

    let mut bg_geometry: VertexBuffers<BgVertex> = VertexBuffers::new();
    fill_rectangle(
        &Rect::new(point(-1.0, -1.0), size(2.0, 2.0)),
        &mut BuffersBuilder::new(&mut bg_geometry, BgVertexCtor),
    );

    let window_position = compute_window_position(&geometry.vertices);

    let glutin_builder = glutin::WindowBuilder::new()
        .with_dimensions(window_position.width as u32, window_position.height as u32)
        .with_decorations(true)
        .with_title("lyon export".to_string());

    let msaa = match render_options.aa {
        AntiAliasing::Msaa(samples) => samples,
        _ => 0,
    };
    let context = glutin::ContextBuilder::new()
        .with_multisampling(msaa)
        .with_vsync(true);

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
        FRAGMENT_SHADER.as_bytes(),
    ).unwrap();

    let mut rasterizer_state = gfx::state::Rasterizer::new_fill().with_cull_back();
    if let AntiAliasing::Msaa(_) = render_options.aa {
        rasterizer_state.samples = Some(gfx::state::MultiSample);
    }
    let path_pso = factory.create_pipeline_from_program(
        &path_shader,
        gfx::Primitive::TriangleList,
        rasterizer_state,
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
        &bg_geometry.indices[..],
    );

    let (path_vbo, path_range) = factory.create_vertex_buffer_with_slice(
        &geometry.vertices[..],
        &geometry.indices[..],
    );

    let gpu_primitives = factory.create_constant_buffer(2);
    let constants = factory.create_constant_buffer(1);

    let mut scene = SceneParams {
        target_zoom: 1.0,
        zoom: 0.1,
        target_scroll: vector(window_position.center_x, window_position.center_y),
        scroll: vector(window_position.center_x, window_position.center_y),
        show_points: false,
        show_wireframe: false,
        stroke_width,
        target_stroke_width: stroke_width,
        draw_background: true,
        cursor_position: (window_position.center_x, window_position.center_y),
        window_size: (window_position.width, window_position.height),
    };

    let mut cmd_queue: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    while update_inputs(&mut events_loop, &mut scene) {
        gfx_window_glutin::update_views(&window, &mut main_fbo, &mut main_depth);
        let (w, h) = window.get_inner_size_pixels().unwrap();
        //scene.window_size = (w as f32, h as f32);

        cmd_queue.clear(&main_fbo.clone(), [0.0, 0.0, 0.0, 0.0]);
        cmd_queue.clear_depth(&main_depth.clone(), 1.0);

        cmd_queue.update_constant_buffer(
            &constants,
            &Globals {
                resolution: [window_position.width as f32, window_position.height as f32],
                zoom: scene.zoom,
                scroll_offset: scene.scroll.to_array(),
            },
        );

        cmd_queue.update_buffer(
            &gpu_primitives,
            &[
                Primitive {
                    color: [1.0, 1.0, 1.0, 1.0],
                    z_index: 0.1,
                    width: 0.0,
                    padding: [0.0, 0.0],
                },
                Primitive {
                    color: [0.0, 0.0, 0.0, 1.0],
                    z_index: 0.2,
                    width: scene.target_stroke_width,
                    padding: [0.0, 0.0],
                },
            ],
            0,
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
    }
}

struct WindowPosition {
    width: f32,
    height: f32,
    center_x: f32,
    center_y: f32,
}

fn compute_window_position(vertices: &Vec<GpuVertex>) -> WindowPosition {
    if vertices.len() == 0 {
        return WindowPosition {
            width: DEFAULT_WINDOW_WIDTH,
            height: DEFAULT_WINDOW_HEIGHT,
            center_x: 0.0,
            center_y: 0.0,
        };
    }

    let (min_x, max_x, min_y, max_y) = get_bounding_coordinates(&vertices);


    // Add 10% margin to each side to view the shape correctly
    let (x_margin, y_margin) = get_window_margins(min_x, max_x, min_y, max_y);

    // Compute the window size
    let (width, height) = get_window_size(min_x, max_x, min_y, max_y, x_margin, y_margin);

    // Adjust the scroll to center the shape
    let( center_x, center_y) = get_window_center(min_x, x_margin, width, min_y, y_margin, height);

    WindowPosition {
        width,
        height,
        center_x,
        center_y,
    }
}

fn get_window_center(min_x :f32, x_margin :f32, width :f32, min_y :f32, y_margin :f32, height :f32)->(f32, f32) {
    let center_x: f32 = width / 2.0 + min_x - x_margin;
    let center_y: f32 = height / 2.0 + min_y - y_margin;

    (center_x, center_y)
}

fn get_window_size(min_x :f32, max_x :f32, min_y :f32, max_y :f32, x_margin :f32, y_margin :f32) -> (f32, f32) {
    let width: f32 = (max_x - min_x) + 2.0 * x_margin;
    let height: f32 = (max_y - min_y) + 2.0 * y_margin;
    (width, height)
}

fn get_window_margins(min_x: f32, max_x: f32, min_y: f32, max_y: f32) -> (f32, f32) {
    let x_margin: f32 = (max_x - min_x) * 0.1;
    let y_margin: f32 = (max_y - min_y) * 0.1;
    (x_margin, y_margin)
}

fn get_bounding_coordinates(vertices: &Vec<GpuVertex>) -> (f32, f32, f32, f32) {
    let mut min_x: f32 = f32::MAX;
    let mut max_x: f32 = f32::MIN;
    let mut min_y: f32 = f32::MAX;
    let mut max_y: f32 = f32::MIN;
    for vertex in vertices.iter() {
        if vertex.position[0] < min_x { min_x = vertex.position[0]; }
        if vertex.position[0] > max_x { max_x = vertex.position[0]; }
        if vertex.position[1] < min_y { min_y = vertex.position[1]; }
        if vertex.position[1] > max_y { max_y = vertex.position[1]; }
    }

    (min_x, max_x, min_y, max_y)
}

gfx_defines! {
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
        z_index: f32 = "z_index",
        width: f32 = "width",
        padding: [f32; 2] = "padding",
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
        float z_index;
        float width;
        vec2 padding;
    };
    uniform u_primitives { Primitive primitives[2]; };

    in vec2 a_position;
    in vec2 a_normal;
    in int a_prim_id;

    out vec4 v_color;

    void main() {
        int id = a_prim_id + gl_InstanceID;
        Primitive prim = primitives[id];

        vec2 local_pos = a_position + a_normal * prim.width;
        vec2 world_pos = local_pos - u_scroll_offset;
        vec2 transformed_pos = world_pos * u_zoom / (vec2(0.5, -0.5) * u_resolution);

        gl_Position = vec4(transformed_pos, 1.0 - prim.z_index, 1.0);
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
            }
            Event::WindowEvent {
                event: WindowEvent::MouseInput {
                    state: glutin::ElementState::Pressed, button: glutin::MouseButton::Left,
                    ..
                },
                ..
            } => {
                let half_width = scene.window_size.0 * 0.5;
                let half_height = scene.window_size.1 * 0.5;
                println!("X: {}, Y: {}",
                         (scene.cursor_position.0 - half_width) / scene.zoom + scene.scroll.x,
                         (scene.cursor_position.1 - half_height) / scene.zoom + scene.scroll.y,
                );
            }
            Event::WindowEvent {
                event: WindowEvent::MouseMoved {
                    position: (x, y),
                    ..
                },
                ..
            } => {
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
            _evt => {
                //println!("{:?}", _evt);
            }
        }
        //println!(" -- zoom: {}, scroll: {:?}", scene.target_zoom, scene.target_scroll);
    });

    scene.zoom += (scene.target_zoom - scene.zoom) / 3.0;
    scene.scroll = scene.scroll + (scene.target_scroll - scene.scroll) / 3.0;
    scene.stroke_width = scene.stroke_width +
        (scene.target_stroke_width - scene.stroke_width) / 5.0;

    return status;
}
