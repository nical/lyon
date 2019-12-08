use lyon::math::*;
use lyon::tessellation::geometry_builder::*;
use lyon::tessellation::basic_shapes::*;
use lyon::tessellation::{FillTessellator, StrokeTessellator, FillOptions};
use lyon::tessellation;
use lyon::algorithms::hatching::*;
use lyon::algorithms::aabb::bounding_rect;
use lyon::path::Path;
use commands::{TessellateCmd, AntiAliasing, RenderCmd, Tessellator, Background};
use lyon::tess2;

use gfx;
use gfx_window_glutin;
use gfx::traits::{Device, FactoryExt};
use glutin;
use glutin::{EventsLoop, KeyboardInput};
use glutin::ElementState::Pressed;
use glutin::dpi::LogicalSize;

const DEFAULT_WINDOW_WIDTH: f32 = 800.0;
const DEFAULT_WINDOW_HEIGHT: f32 = 800.0;

pub fn show_path(cmd: TessellateCmd, render_options: RenderCmd) {
    let mut geometry: VertexBuffers<GpuVertex, u16> = VertexBuffers::new();
    let mut stroke_width = 1.0;
    if let Some(options) = cmd.stroke {
        stroke_width = options.line_width;
        StrokeTessellator::new().tessellate_path(
            cmd.path.iter(),
            &options,
            &mut BuffersBuilder::new(&mut geometry, WithId(1))
        ).unwrap();
    }

    if let Some(hatch) = cmd.hatch {
        let mut path = Path::builder();
        let mut hatcher = Hatcher::new();
        hatcher.hatch_path(
            cmd.path.iter(),
            &hatch.options,
            &mut RegularHatchingPattern {
                interval: hatch.spacing,
                callback: &mut|segment: &HatchSegment| {
                    path.move_to(segment.a.position);
                    path.line_to(segment.b.position);
                }
            },

        );
        let hatched_path = path.build();

        StrokeTessellator::new().tessellate_path(
            hatched_path.iter(),
            &hatch.stroke,
            &mut BuffersBuilder::new(&mut geometry, WithId(1))
        ).unwrap();
    }

    if let Some(dots) = cmd.dots {
        let mut path = Path::builder();
        let mut hatcher = Hatcher::new();
        hatcher.dot_path(
            cmd.path.iter(),
            &dots.options,
            &mut RegularDotPattern {
                row_interval: dots.spacing,
                column_interval: dots.spacing,
                callback: &mut|dot: &Dot| {
                    path.move_to(dot.position);
                }
            },
        );
        let dotted_path = path.build();

        StrokeTessellator::new().tessellate_path(
            dotted_path.iter(),
            &dots.stroke,
            &mut BuffersBuilder::new(&mut geometry, WithId(1))
        ).unwrap();
    }

    if let Some(options) = cmd.fill {
        match cmd.tessellator {
            Tessellator::Default => {
                let mut tess = FillTessellator::new();

                tess.tessellate_path(
                    &cmd.path,
                    &options,
                    &mut BuffersBuilder::new(&mut geometry, WithId(0))
                ).unwrap();

                for (i, v) in geometry.vertices.iter().enumerate() {
                    println!("{}: {:?}", i, v.position);
                }
                for i in 0..(geometry.indices.len() / 3) {
                    println!("{}/{}/{}",
                        geometry.indices[i*3],
                        geometry.indices[i*3+1],
                        geometry.indices[i*3+2],
                    );
                }
            }
            Tessellator::Tess2 => {
                tess2::FillTessellator::new().tessellate_path(
                    cmd.path.iter(),
                    &options,
                    &mut BuffersBuilder::new(&mut geometry, WithId(0))
                ).unwrap();
            }
        }
    }

    let geom_split = geometry.indices.len() as u32;

    fill_circle(
        point(0.0, 0.0),
        1.0,
        &FillOptions::tolerance(0.01),
        &mut BuffersBuilder::new(&mut geometry, WithId(0)),
    ).unwrap();

    let (bg_color, vignette_color) = match render_options.background {
        Background::Blue => ([0.0, 0.47, 0.9, 1.0], [0.0, 0.1, 0.64, 1.0]),
        Background::Clear => ([0.9, 0.9, 0.9, 1.0], [0.5, 0.5, 0.5, 1.0]),
        Background::Dark => ([0.05, 0.05, 0.05, 1.0], [0.0, 0.0, 0.0, 1.0]),
    };

    if geometry.vertices.is_empty() {
        println!("No geometry to show");
        return;
    }

    let mut bg_geometry: VertexBuffers<BgVertex, u16> = VertexBuffers::new();
    fill_rectangle(
        &Rect::new(point(-1.0, -1.0), size(2.0, 2.0)),
        &FillOptions::default(),
        &mut BuffersBuilder::new(&mut bg_geometry, BgVertexCtor),
    ).unwrap();

    let glutin_builder = glutin::WindowBuilder::new()
        .with_dimensions(LogicalSize { width: DEFAULT_WINDOW_WIDTH as f64, height: DEFAULT_WINDOW_HEIGHT as f64 })
        .with_decorations(true)
        .with_title("lyon".to_string());

    let msaa = match render_options.aa {
        AntiAliasing::Msaa(samples) => samples,
        _ => 0,
    };
    let context = glutin::ContextBuilder::new()
        .with_multisampling(msaa)
        .with_vsync(true);

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
        &bg_geometry.indices[..]
    );

    let (path_vbo, vbo_range) = factory.create_vertex_buffer_with_slice(
        &geometry.vertices[..],
        &geometry.indices[..]
    );

    let (path_range, _) = vbo_range.split_at(geom_split);

    let gpu_primitives = factory.create_constant_buffer(3);
    let constants = factory.create_constant_buffer(1);

    let aabb = bounding_rect(cmd.path.iter());
    let center = aabb.origin.lerp(aabb.max(), 0.5).to_vector();

    let mut scene = SceneParams {
        target_zoom: 1.0,
        zoom: 0.1,
        target_scroll: center,
        scroll: center,
        show_points: false,
        show_wireframe: false,
        stroke_width,
        target_stroke_width: stroke_width,
        draw_background: true,
        cursor_position: (0.0, 0.0),
        window_size: (DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT),
    };

    let mut cmd_queue: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    
    while update_inputs(&mut events_loop, &mut scene) {

        gfx_window_glutin::update_views(&window, &mut main_fbo, &mut main_depth);
        let size = window.get_inner_size().unwrap();
        scene.window_size = (size.width as f32, size.height as f32);

        cmd_queue.clear(&main_fbo.clone(), [0.0, 0.0, 0.0, 0.0]);
        cmd_queue.clear_depth(&main_depth.clone(), 1.0);

        cmd_queue.update_constant_buffer(
            &constants,
            &Globals {
                resolution: [size.width as f32, size.height as f32],
                zoom: scene.zoom,
                scroll_offset: scene.scroll.to_array(),
                bg_color,
                vignette_color,
            },
        );

        cmd_queue.update_buffer(
            &gpu_primitives,
            &[
                Primitive {
                    color: [1.0, 1.0, 1.0, 1.0],
                    z_index: 0.1,
                    width: 0.0,
                    translation: [0.0, 0.0],
                },
                Primitive {
                    color: [0.0, 0.0, 0.0, 1.0],
                    z_index: 0.2,
                    width: scene.target_stroke_width,
                    translation: [0.0, 0.0],
                },
                // TODO: Debug edges. Color is hard-coded.
                Primitive {
                    color: [0.5, 0.0, 0.0, 1.0],
                    z_index: 0.4,
                    width: scene.target_stroke_width * 0.5,
                    translation: [0.0, 0.0],
                },
            ],
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
    }
}

gfx_defines!{
    constant Globals {
        bg_color: [f32; 4] = "u_bg_color",
        vignette_color: [f32; 4] = "u_vignette_color",
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
        translation: [f32; 2] = "translation",
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
        vec4 u_bg_color;
        vec4 u_vignette_color;
        vec2 u_resolution;
        vec2 u_scroll_offset;
        float u_zoom;
    };
    in vec2 v_position;
    out vec4 out_color;

    void main() {
        vec2 px_position = v_position * vec2(1.0, -1.0) * u_resolution * 0.5;

        float vignette = clamp(0.0, 1.0, (0.7*length(v_position)));
        out_color = mix(
            u_bg_color,
            u_vignette_color,
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
        vec4 u_bg_color;
        vec4 u_vignette_color;
        vec2 u_resolution;
        vec2 u_scroll_offset;
        float u_zoom;
    };

    struct Primitive {
        vec4 color;
        float z_index;
        float width;
        vec2 translation;
    };
    uniform u_primitives { Primitive primitives[2]; };

    in vec2 a_position;
    in vec2 a_normal;
    in int a_prim_id;

    out vec4 v_color;

    void main() {
        int id = a_prim_id + gl_InstanceID;
        Primitive prim = primitives[id];

        vec2 local_pos = a_position + a_normal * prim.width + prim.translation;
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

impl FillVertexConstructor<GpuVertex> for WithId {
    fn new_vertex(&mut self, position: Point, _: tessellation::FillAttributes) -> GpuVertex {
        debug_assert!(!position.x.is_nan());
        debug_assert!(!position.y.is_nan());
        GpuVertex {
            position: position.to_array(),
            normal: [0.0, 0.0],
            prim_id: self.0,
        }
    }
}

impl StrokeVertexConstructor<GpuVertex> for WithId {
    fn new_vertex(&mut self, position: Point, attributes: tessellation::StrokeAttributes) -> GpuVertex {
        debug_assert!(!position.x.is_nan());
        debug_assert!(!position.y.is_nan());
        debug_assert!(!attributes.normal.x.is_nan());
        debug_assert!(!attributes.normal.y.is_nan());
        debug_assert!(!attributes.advancement.is_nan());
        GpuVertex {
            position: position.to_array(),
            normal: attributes.normal.to_array(),
            prim_id: self.0,
        }
    }
}

impl BasicVertexConstructor<GpuVertex> for WithId {
    fn new_vertex(&mut self, position: Point) -> GpuVertex {
        debug_assert!(!position.x.is_nan());
        debug_assert!(!position.y.is_nan());
        GpuVertex {
            position: position.to_array(),
            normal: [0.0, 0.0],
            prim_id: self.0,
        }
    }
}

struct BgVertexCtor;

impl BasicVertexConstructor<BgVertex> for BgVertexCtor {
    fn new_vertex(&mut self, position: Point) -> BgVertex {
        debug_assert!(!position.x.is_nan());
        debug_assert!(!position.y.is_nan());
        BgVertex {
            position: position.to_array(),
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
                event: WindowEvent::CursorMoved { position, .. },
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
