use lyon::math::*;
use lyon::tessellation::geometry_builder::{VertexConstructor, VertexBuffers, BuffersBuilder};
use lyon::tessellation::basic_shapes::*;
use lyon::tessellation::{FillTessellator, StrokeTessellator, FillOptions, StrokeOptions};
use lyon::tessellation::debugger::*;
use lyon::tessellation;
use lyon::algorithms::hatching::*;
use lyon::algorithms::aabb::bounding_rect;
use lyon::path::Path;
use commands::{TessellateCmd, AntiAliasing, RenderCmd, Tessellator, Background};
use lyon::tess2;
#[cfg(feature = "experimental")]
use lyon::tessellation::experimental;

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

    let mut debug_trace = Trace::new();
    if let Some(options) = cmd.fill {
        match cmd.tessellator {
            Tessellator::Default => {
                let mut tess = FillTessellator::new();
                let dbg_receiver = render_options.debugger.map(|flags| {
                    let (dbg_tx, dbg_rx) = debugger_channel();
                    tess.install_debugger(Box::new(Filter::new(flags, dbg_tx)));
                    dbg_rx
                });
                tess.tessellate_path(
                    cmd.path.iter(),
                    &options,
                    &mut BuffersBuilder::new(&mut geometry, WithId(0))
                ).unwrap();
                if let Some(dbg) = dbg_receiver {
                    dbg.write_trace(&mut debug_trace);
                }
            }
            Tessellator::Tess2 => {
                tess2::FillTessellator::new().tessellate_path(
                    cmd.path.iter(),
                    &options,
                    &mut BuffersBuilder::new(&mut geometry, WithId(0))
                ).unwrap();
            }
            Tessellator::Experimental => {
                #[cfg(feature = "experimental")] {
                    use lyon::path::builder::*;
                    use lyon::path::iterator::*;

                    println!(" -- running the experimental tessellator.");

                    let mut builder = Path::builder();
                    for e in cmd.path.iter().flattened(options.tolerance) {
                        println!("{:?}", e);
                        builder.flat_event(e);
                    }

                    let mut tess = experimental::FillTessellator::new();
                    let dbg_receiver = render_options.debugger.map(|flags| {
                        let (dbg_tx, dbg_rx) = debugger_channel();
                        tess.install_debugger(Box::new(Filter::new(flags, dbg_tx)));
                        tess.enable_logging();
                        dbg_rx
                    });

                    tess.tessellate_path(
                        &builder.build(),
                        &options,
                        &mut BuffersBuilder::new(&mut geometry, WithId(0))
                    );
                    if let Some(dbg) = dbg_receiver {
                        dbg.write_trace(&mut debug_trace);
                    }
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

    let (path_range, mut point_range) = vbo_range.split_at(geom_split);

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
        update_debugger: true,
    };

    let mut cmd_queue: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let mut point_primitives = factory.create_constant_buffer(1);
    let mut debug_points = Vec::new();
    let mut debug_edges = VertexBuffers::new();
    let mut gpu_debug_edges = None;

    while update_inputs(&mut events_loop, &mut scene) {
        if scene.update_debugger && render_options.debugger.is_some() {
            scene.update_debugger = false;
            debug_points.clear();
            debug_edges.vertices.clear();
            debug_edges.indices.clear();
            get_debug_geometry(
                &debug_trace,
                &scene,
                None,
                0.5,
                &mut debug_points,
                &mut debug_edges
            );
            if !debug_points.is_empty() {
                point_primitives = factory.create_constant_buffer(debug_points.len());
                cmd_queue.update_buffer(&point_primitives, &debug_points, 0).unwrap();
            }
            point_range.instances = Some((debug_points.len() as u32, 0));
            if !debug_edges.indices.is_empty() {
                gpu_debug_edges = Some(factory.create_vertex_buffer_with_slice(
                    &debug_edges.vertices[..],
                    &debug_edges.indices[..]
                ));
            }
        }

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


        if !debug_points.is_empty() {
            cmd_queue.draw(
                &point_range,
                &pso,
                &path_pipeline::Data {
                    vbo: path_vbo.clone(),
                    primitives: point_primitives.clone(),
                    constants: constants.clone(),
                    out_color: main_fbo.clone(),
                    out_depth: main_depth.clone(),
                },
            );
        }

        if let Some((ref vbo, ref ibo)) = gpu_debug_edges {
            cmd_queue.draw(
                &ibo,
                &path_pso,
                &path_pipeline::Data {
                    vbo: vbo.clone(),
                    primitives: gpu_primitives.clone(),
                    constants: constants.clone(),
                    out_color: main_fbo.clone(),
                    out_depth: main_depth.clone(),
                },
            );
        }

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

fn get_debug_geometry(
    debug_trace: &Trace,
    scene: &SceneParams,
    target_frame: Option<u32>,
    z_index: f32,
    points: &mut Vec<Primitive>,
    edges: &mut VertexBuffers<GpuVertex, u16>,
) {
    let mut edge_path = Path::builder();
    let mut frame = 0;
    for msg in &debug_trace.messages {
        match msg {
            DebuggerMsg::Point { position, color, .. } => {
                if target_frame == Some(frame) || target_frame.is_none() {
                    points.push(Primitive {
                        color: [
                            color.r as f32 * 255.0,
                            color.g as f32 * 255.0,
                            color.b as f32 * 255.0,
                            color.a as f32 * 255.0,
                        ],
                        z_index,
                        width: scene.target_stroke_width,
                        translation: position.to_array(),
                    });
                    points.push(Primitive {
                        color: [1.0, 1.0, 1.0, 1.0],
                        z_index: z_index + 0.1,
                        width: scene.target_stroke_width * 0.5,
                        translation: position.to_array(),
                    });
                }
            }
            DebuggerMsg::Edge { from, to, .. } => {
                if target_frame == Some(frame) || target_frame.is_none() {
                    edge_path.move_to(*from);
                    edge_path.line_to(*to);
                }
            }
            DebuggerMsg::NewFrame { .. } => {
                frame += 1;
            }
            DebuggerMsg::String { string, .. } => {
                if target_frame == Some(frame) || target_frame.is_none() {
                    println!("[debugger]: {}", string);
                }
            }
            DebuggerMsg::Error { .. } => {
                println!("[debugger]: received an error event");
            }
        }
    }

    let path = edge_path.build();

    StrokeTessellator::new().tessellate_path(
        path.iter(),
        &StrokeOptions::default().dont_apply_line_width(),
        &mut BuffersBuilder::new(edges, WithId(2)),
    ).unwrap();
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

impl VertexConstructor<Point, GpuVertex> for WithId {
    fn new_vertex(&mut self, point: Point) -> GpuVertex {
        debug_assert!(!point.x.is_nan());
        debug_assert!(!point.y.is_nan());
        GpuVertex {
            position: point.to_array(),
            normal: [0.0, 0.0],
            prim_id: self.0,
        }
    }
}

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
    update_debugger: bool,
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
