use std::ops::{Range, Rem};
use std::sync::Arc;
use std::time::{Duration, Instant};

use lyon::extra::rust_logo::build_logo_path;
use lyon::math::*;
use lyon::path::{Path, Polygon, NO_ATTRIBUTES};
use lyon::tessellation;
use lyon::tessellation::geometry_builder::*;
use lyon::tessellation::{FillOptions, FillTessellator};
use lyon::tessellation::{StrokeOptions, StrokeTessellator};

use lyon::algorithms::{rounded_polygon, walk};

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

// For create_buffer_init()
use wgpu::util::DeviceExt;

use bytemuck::{Pod, Zeroable};
use futures::executor::block_on;

const PRIM_BUFFER_LEN: usize = 256;

/// Number of samples for anti-aliasing. Set to 1 to disable.
const SAMPLE_COUNT: u32 = 4;
const NUM_INSTANCES: u32 = 32;

/// The first primitive we have is
const LOGO_STROKE_PRIM_ID: usize = 0;
/// The second primitive
const LOGO_FILL_PRIM_ID: usize = 1;

const ARROWS_PRIM_ID: u32 = NUM_INSTANCES + 1;

const DEFAULT_WINDOW_WIDTH: f32 = 800.0;
const DEFAULT_WINDOW_HEIGHT: f32 = 800.0;

/// Globals stored in a uniform buffer
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Globals {
    resolution: [f32; 2],
    scroll_offset: [f32; 2],
    zoom: f32,
    _pad: f32,
}

/// Data shared in the vertex buffer
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct GpuVertex {
    /// Center position of the vertex
    position: [f32; 2],
    /// Direction the position needs to be moved from center, if any.
    ///
    /// This is used when stroking a line; the tessellator could calculate the exact vertex
    /// position for us, but it is cheaper to only calculate the normal on the CPU and offload
    /// the final position calculation to the GPU.
    normal: [f32; 2],
    /// Reference to a [`Primitive`] descriptor
    prim_id: u32,
}

impl GpuVertex {
    fn desc() -> &'static [wgpu::VertexAttribute] {
        &[
            wgpu::VertexAttribute {
                offset: 0,
                format: wgpu::VertexFormat::Float32x2,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                offset: 8,
                format: wgpu::VertexFormat::Float32x2,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                offset: 16,
                format: wgpu::VertexFormat::Uint32,
                shader_location: 2,
            },
        ]
    }
}

/// Descriptor for primitives, which will be stored in an array in a uniform buffer.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Primitive {
    color: [f32; 4],
    translate: [f32; 2],
    z_index: i32,
    width: f32,
    angle: f32,
    scale: f32,
    _pad1: i32,
    _pad2: i32,
}

impl Default for Primitive {
    fn default() -> Self {
        Self {
            color: [0.0; 4],
            translate: [0.0; 2],
            z_index: 0,
            width: 0.0,
            angle: 0.0,
            scale: 1.0,
            _pad1: 0,
            _pad2: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct BgPoint {
    point: [f32; 2],
}

fn main() {
    env_logger::init();
    println!("== wgpu example ==");
    println!("Controls:");
    println!("  Arrow keys: scrolling");
    println!("  PgUp/PgDown: zoom in/out");
    println!("  b: toggle drawing the background");
    println!("  a/z: increase/decrease the stroke width");

    let app_state = AppState::new();
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        window: None,
        state: app_state,
        gfx_state: None,
    };

    event_loop.run_app(&mut app).unwrap();
}

/// This vertex constructor forwards the positions and normals provided by the
/// tessellators and add a shape id.
pub struct WithId(pub u32);

impl FillVertexConstructor<GpuVertex> for WithId {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> GpuVertex {
        GpuVertex {
            position: vertex.position().to_array(),
            normal: [0.0, 0.0],
            prim_id: self.0,
        }
    }
}

impl StrokeVertexConstructor<GpuVertex> for WithId {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex) -> GpuVertex {
        GpuVertex {
            position: vertex.position_on_path().to_array(),
            normal: vertex.normal().to_array(),
            prim_id: self.0,
        }
    }
}

pub struct Custom;

impl FillVertexConstructor<BgPoint> for Custom {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> BgPoint {
        BgPoint {
            point: vertex.position().to_array(),
        }
    }
}

/// The configured state of our application
struct SceneParams {
    target_zoom: f32,
    zoom: f32,
    target_scroll: Vector,
    scroll: Vector,
    show_points: bool,
    stroke_width: f32,
    target_stroke_width: f32,
    draw_background: bool,
    window_size: PhysicalSize<u32>,
    size_changed: bool,
    render: bool,
}

impl Default for SceneParams {
    fn default() -> Self {
        Self {
            target_zoom: 5.0,
            zoom: 5.0,
            target_scroll: vector(70.0, 70.0),
            scroll: vector(70.0, 70.0),
            show_points: false,
            stroke_width: 1.0,
            target_stroke_width: 1.0,
            draw_background: true,
            window_size: PhysicalSize::new(
                DEFAULT_WINDOW_WIDTH as u32,
                DEFAULT_WINDOW_HEIGHT as u32,
            ),
            size_changed: true,
            render: false,
        }
    }
}

/// The application itself
struct App {
    /// We need to store an `Arc` because both `App` (for `ApplicationHandler`) and `GfxState`
    /// (for the surface) require references to the `Window`.
    window: Option<Arc<Window>>,
    gfx_state: Option<GfxState>,
    state: AppState,
}

impl ApplicationHandler for App {
    /// Create a new window
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let win_attrs = Window::default_attributes()
            .with_title("Lyon tessellation example")
            .with_inner_size(self.state.scene.window_size);
        let window = Arc::new(event_loop.create_window(win_attrs).unwrap());

        let gfx_state = GfxState::new(Arc::clone(&window), &mut self.state);
        window.request_redraw();

        self.window = Some(window);
        self.gfx_state = Some(gfx_state);
    }

    /// Handle redraw requests and other state changes
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let scene = &mut self.state.scene;

        match event {
            WindowEvent::RedrawRequested => {
                self.window.as_ref().unwrap().request_redraw();
                scene.render = true;
            }
            WindowEvent::Destroyed | WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                scene.window_size = size;
                scene.size_changed = true;
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match key_code {
                KeyCode::Escape => event_loop.exit(),
                KeyCode::PageDown => scene.target_zoom *= 0.8,
                KeyCode::PageUp => scene.target_zoom *= 1.25,
                KeyCode::ArrowLeft => scene.target_scroll.x += 50.0 / scene.target_zoom,
                KeyCode::ArrowRight => scene.target_scroll.x -= 50.0 / scene.target_zoom,
                KeyCode::ArrowUp => scene.target_scroll.y -= 50.0 / scene.target_zoom,
                KeyCode::ArrowDown => scene.target_scroll.y += 50.0 / scene.target_zoom,
                KeyCode::KeyP => scene.show_points = !scene.show_points,
                KeyCode::KeyB => scene.draw_background = !scene.draw_background,
                KeyCode::KeyA => scene.target_stroke_width /= 0.8,
                KeyCode::KeyZ => scene.target_stroke_width *= 0.8,
                _key => {}
            },
            _evt => {}
        };

        if event_loop.exiting() {
            return;
        }

        scene.zoom += (scene.target_zoom - scene.zoom) / 3.0;
        scene.scroll = scene.scroll + (scene.target_scroll - scene.scroll) / 3.0;
        scene.stroke_width =
            scene.stroke_width + (scene.target_stroke_width - scene.stroke_width) / 5.0;

        self.gfx_state.as_mut().unwrap().paint(&mut self.state);
    }
}

/// Store values that are reused throughout drawings
struct AppState {
    scene: SceneParams,
    path: Path,
    /// Tessellated vertices for the main geometry
    geometry: VertexBuffers<GpuVertex, u16>,
    /// Tessellated vertices for the background
    bg_geometry: VertexBuffers<BgPoint, u16>,
    cpu_primitives: Vec<Primitive>,

    // Ranges of different primitive types in the buffer
    logo_fill_range: Range<u32>,
    logo_stroke_range: Range<u32>,
    arrow_range: Range<u32>,

    // Values for FPS reporting
    start: Instant,
    next_report: Instant,
    frame_count: u32,
    time_secs: f32,
}

impl AppState {
    /// Tessellate the example geometry to create an initial application state
    fn new() -> Self {
        let scene = SceneParams::default();
        let tolerance = 0.02;
        let mut geometry: VertexBuffers<GpuVertex, u16> = VertexBuffers::new();

        let mut fill_tess = FillTessellator::new();
        let mut stroke_tess = StrokeTessellator::new();

        /* Tessellate the fill and stroke of the Rust logo */

        // Build a Path for the rust logo.
        let mut logo_builder = Path::builder().with_svg();
        build_logo_path(&mut logo_builder);
        let logo_path = logo_builder.build();

        fill_tess
            .tessellate_path(
                &logo_path,
                &FillOptions::tolerance(tolerance).with_fill_rule(tessellation::FillRule::NonZero),
                &mut BuffersBuilder::new(&mut geometry, WithId(LOGO_FILL_PRIM_ID as u32)),
            )
            .unwrap();

        let logo_fill_range = 0..(geometry.indices.len() as u32);

        stroke_tess
            .tessellate_path(
                &logo_path,
                &StrokeOptions::tolerance(tolerance),
                &mut BuffersBuilder::new(&mut geometry, WithId(LOGO_STROKE_PRIM_ID as u32)),
            )
            .unwrap();

        let logo_stroke_range = logo_fill_range.end..(geometry.indices.len() as u32);

        /* Tessellate an arrow primitive  */

        // Create an arrow shape that we can reuse
        let arrow_points = [
            point(-1.0, -0.3),
            point(0.0, -0.3),
            point(0.0, -1.0),
            point(1.5, 0.0),
            point(0.0, 1.0),
            point(0.0, 0.3),
            point(-1.0, 0.3),
        ];

        let arrow_polygon = Polygon {
            points: &arrow_points,
            closed: true,
        };

        // Build a Path for the arrow.
        let mut arrow_builder = Path::builder();
        rounded_polygon::add_rounded_polygon(&mut arrow_builder, arrow_polygon, 0.2, NO_ATTRIBUTES);
        let arrow_path = arrow_builder.build();

        fill_tess
            .tessellate_path(
                &arrow_path,
                &FillOptions::tolerance(tolerance),
                &mut BuffersBuilder::new(&mut geometry, WithId(ARROWS_PRIM_ID)),
            )
            .unwrap();

        let arrow_range = logo_stroke_range.end..(geometry.indices.len() as u32);

        /* Create the background grid  */

        let mut bg_geometry: VertexBuffers<BgPoint, u16> = VertexBuffers::new();

        fill_tess
            .tessellate_rectangle(
                &Box2D {
                    min: point(-1.0, -1.0),
                    max: point(1.0, 1.0),
                },
                &FillOptions::DEFAULT,
                &mut BuffersBuilder::new(&mut bg_geometry, Custom),
            )
            .unwrap();

        /* Build all primitives */

        // Create red primitives by default
        let mut cpu_primitives = Vec::with_capacity(PRIM_BUFFER_LEN);
        for _ in 0..PRIM_BUFFER_LEN {
            cpu_primitives.push(Primitive {
                color: [1.0, 0.0, 0.0, 1.0],
                z_index: 0,
                width: 0.0,
                translate: [0.0, 0.0],
                angle: 0.0,
                ..Primitive::default()
            });
        }

        // Stroke the logo with black
        cpu_primitives[LOGO_STROKE_PRIM_ID] = Primitive {
            color: [0.0, 0.0, 0.0, 1.0],
            z_index: NUM_INSTANCES as i32 + 2,
            width: 1.0,
            ..Primitive::default()
        };

        // Fill the logo with white
        cpu_primitives[LOGO_FILL_PRIM_ID] = Primitive {
            color: [1.0, 1.0, 1.0, 1.0],
            z_index: NUM_INSTANCES as i32 + 1,
            ..Primitive::default()
        };

        // Instance primitives
        for (idx, cpu_prim) in cpu_primitives
            .iter_mut()
            .enumerate()
            .skip(LOGO_FILL_PRIM_ID + 1)
            .take(NUM_INSTANCES as usize - 1)
        {
            cpu_prim.z_index = (idx as u32 + 1) as i32;
            cpu_prim.color = [
                (0.1 * idx as f32).rem(1.0),
                (0.5 * idx as f32).rem(1.0),
                (0.9 * idx as f32).rem(1.0),
                1.0,
            ];
        }

        let start = Instant::now();

        Self {
            scene,
            cpu_primitives,
            logo_fill_range,
            logo_stroke_range,
            arrow_range,
            geometry,
            bg_geometry,
            start,
            next_report: start + Duration::from_secs(1),
            frame_count: 0,
            time_secs: 0.0,
            path: logo_path,
        }
    }
}

/// Everything needed for wgpu graphics
struct GfxState {
    device: wgpu::Device,
    surface_desc: wgpu::SurfaceConfiguration,
    /// Drawable surface, which contains an `Arc<Window>`
    surface: wgpu::Surface<'static>,
    /// Primitive uniform buffer
    prims_ubo: wgpu::Buffer,
    /// Globals uniform buffer
    globals_ubo: wgpu::Buffer,
    /// Index buffer object for the main shader
    geo_ibo: wgpu::Buffer,
    /// Vertex buffer object for the main shader
    geo_vbo: wgpu::Buffer,
    /// Main shader render pipeline
    geo_pipeline: wgpu::RenderPipeline,
    /// Index buffer object for the background shader
    bg_ibo: wgpu::Buffer,
    /// Vertex buffer object for the backgroundshader
    bg_vbo: wgpu::Buffer,
    /// Background shader render pipeline
    bg_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    queue: wgpu::Queue,
    depth_texture_view: Option<wgpu::TextureView>,
    multisampled_render_target: Option<wgpu::TextureView>,
}

impl GfxState {
    // impl<'win> GfxState<'win> {
    fn new(window: Arc<Window>, app_state: &mut AppState) -> Self {
        // Create an instance
        let instance = wgpu::Instance::default();
        let size = window.inner_size();

        // Create a surface
        let surface = instance.create_surface(window).unwrap();

        let (device, queue) = block_on(async {
            // Create an adapter
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::LowPower,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                })
                .await
                .unwrap();

            // Create a device and a queue
            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: None,
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::default(),
                    },
                    None,
                )
                .await
                .unwrap()
        });

        let surface_desc = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            // defaults from `surface.get_default_config(...)``
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };

        surface.configure(&device, &surface_desc);

        // Geometry shader vertex buffer
        let geo_vbo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("geo_vbo"),
            contents: bytemuck::cast_slice(&app_state.geometry.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Geometry shader index buffer
        let geo_ibo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("geo_ibo"),
            contents: bytemuck::cast_slice(&app_state.geometry.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Background shader vertex buffer
        let bg_vbo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bg_vbo"),
            contents: bytemuck::cast_slice(&app_state.bg_geometry.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Background shader index buffer
        let bg_ibo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bg_ibo"),
            contents: bytemuck::cast_slice(&app_state.bg_geometry.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let prim_buffer_byte_size = (PRIM_BUFFER_LEN * std::mem::size_of::<Primitive>()) as u64;
        let globals_buffer_byte_size = std::mem::size_of::<Globals>() as u64;

        // Primitive uniform buffer
        let prims_ubo = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Prims ubo"),
            size: prim_buffer_byte_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Globals uniform buffer
        let globals_ubo = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Globals ubo"),
            size: globals_buffer_byte_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(globals_buffer_byte_size),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(prim_buffer_byte_size),
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(globals_ubo.as_entire_buffer_binding()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(prims_ubo.as_entire_buffer_binding()),
                },
            ],
        });

        // Geometry shaders
        let geo_vs_module =
            &device.create_shader_module(wgpu::include_wgsl!("./../shaders/geometry.vs.wgsl"));
        let geo_fs_module =
            &device.create_shader_module(wgpu::include_wgsl!("./../shaders/geometry.fs.wgsl"));

        // Background shaders
        let bg_vs_module =
            &device.create_shader_module(wgpu::include_wgsl!("./../shaders/background.vs.wgsl"));
        let bg_fs_module =
            &device.create_shader_module(wgpu::include_wgsl!("./../shaders/background.fs.wgsl"));

        let depth_stencil_state = Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Greater,
            stencil: wgpu::StencilState {
                front: wgpu::StencilFaceState::IGNORE,
                back: wgpu::StencilFaceState::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
            bias: wgpu::DepthBiasState::default(),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
            label: Some("pl_layout"),
        });

        let mut render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: geo_vs_module,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: GpuVertex::desc(),
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: geo_fs_module,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                polygon_mode: wgpu::PolygonMode::Fill,
                front_face: wgpu::FrontFace::Ccw,
                strip_index_format: None,
                cull_mode: Some(wgpu::Face::Back),
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: depth_stencil_state.clone(),
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        };

        let geo_pipeline = device.create_render_pipeline(&render_pipeline_descriptor);

        // TODO: this isn't what we want: we'd need the equivalent of VK_POLYGON_MODE_LINE,
        // but it doesn't seem to be exposed by wgpu?
        render_pipeline_descriptor.primitive.topology = wgpu::PrimitiveTopology::LineList;

        let bg_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: bg_vs_module,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Point>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        format: wgpu::VertexFormat::Float32x2,
                        shader_location: 0,
                    }],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: bg_fs_module,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                polygon_mode: wgpu::PolygonMode::Fill,
                front_face: wgpu::FrontFace::Ccw,
                strip_index_format: None,
                cull_mode: None,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: depth_stencil_state,
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            device,
            surface_desc,
            surface,
            prims_ubo,
            globals_ubo,
            geo_ibo,
            geo_vbo,
            bind_group,
            geo_pipeline,
            bg_ibo,
            bg_vbo,
            bg_pipeline,
            queue,
            depth_texture_view: None,
            multisampled_render_target: None,
        }
    }

    fn paint(&mut self, state: &mut AppState) {
        if state.scene.size_changed {
            self.update_scene_size(state);
        }

        if !state.scene.render {
            return;
        }

        state.scene.render = false;

        let frame = match self.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(e) => {
                println!("Swap-chain error: {e:?}");
                return;
            }
        };

        let frame_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder"),
            });

        state.cpu_primitives[LOGO_STROKE_PRIM_ID].width = state.scene.stroke_width;
        state.cpu_primitives[LOGO_STROKE_PRIM_ID].color = [
            (state.time_secs * 0.8 - 1.6).sin() * 0.1 + 0.1,
            (state.time_secs * 0.5 - 1.6).sin() * 0.1 + 0.1,
            (state.time_secs - 1.6).sin() * 0.1 + 0.1,
            1.0,
        ];

        for idx in 2..(NUM_INSTANCES + 1) {
            state.cpu_primitives[idx as usize].translate = [
                (state.time_secs * 0.05 * idx as f32).sin() * (100.0 + idx as f32 * 10.0),
                (state.time_secs * 0.1 * idx as f32).sin() * (100.0 + idx as f32 * 10.0),
            ];
        }

        let mut arrow_count = 0;
        let offset = (state.time_secs * 10.0).rem(5.0);
        let prims = &mut state.cpu_primitives;

        walk::walk_along_path(
            state.path.iter(),
            offset,
            0.1,
            &mut walk::RepeatedPattern {
                callback: |event: walk::WalkerEvent| {
                    if arrow_count + NUM_INSTANCES as usize + 1 >= PRIM_BUFFER_LEN {
                        // Don't want to overflow the primitive buffer,
                        // just skip the remaining arrows.
                        return false;
                    }
                    prims[ARROWS_PRIM_ID as usize + arrow_count] = Primitive {
                        color: [0.7, 0.9, 0.8, 1.0],
                        translate: (event.position * 2.3 - vector(80.0, 80.0)).to_array(),
                        angle: event.tangent.angle_from_x_axis().get(),
                        scale: 2.0,
                        z_index: ARROWS_PRIM_ID as i32,
                        ..Primitive::default()
                    };
                    arrow_count += 1;
                    true
                },
                intervals: &[5.0, 5.0, 5.0],
                index: 0,
            },
        );

        self.queue.write_buffer(
            &self.globals_ubo,
            0,
            bytemuck::cast_slice(&[Globals {
                resolution: [
                    state.scene.window_size.width as f32,
                    state.scene.window_size.height as f32,
                ],
                zoom: state.scene.zoom,
                scroll_offset: state.scene.scroll.to_array(),
                _pad: 0.0,
            }]),
        );

        self.queue.write_buffer(
            &self.prims_ubo,
            0,
            bytemuck::cast_slice(&state.cpu_primitives),
        );

        {
            // A resolve target is only supported if the attachment actually uses anti-aliasing
            // So if sample_count == 1 then we must render directly to the surface's buffer
            let color_attachment = if let Some(msaa_target) = &self.multisampled_render_target {
                wgpu::RenderPassColorAttachment {
                    view: msaa_target,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                    resolve_target: Some(&frame_view),
                }
            } else {
                wgpu::RenderPassColorAttachment {
                    view: &frame_view,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                    resolve_target: None,
                }
            };

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: self.depth_texture_view.as_ref().unwrap(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.geo_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_index_buffer(self.geo_ibo.slice(..), wgpu::IndexFormat::Uint16);
            pass.set_vertex_buffer(0, self.geo_vbo.slice(..));

            pass.draw_indexed(state.logo_fill_range.clone(), 0, 0..NUM_INSTANCES);
            pass.draw_indexed(state.logo_stroke_range.clone(), 0, 0..1);
            pass.draw_indexed(state.arrow_range.clone(), 0, 0..(arrow_count as u32));

            if state.scene.draw_background {
                pass.set_pipeline(&self.bg_pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_index_buffer(self.bg_ibo.slice(..), wgpu::IndexFormat::Uint16);
                pass.set_vertex_buffer(0, self.bg_vbo.slice(..));

                pass.draw_indexed(0..6, 0, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        let now = Instant::now();
        state.frame_count += 1;
        state.time_secs = (now - state.start).as_secs_f32();
        if now >= state.next_report {
            println!("{} FPS", state.frame_count);
            state.frame_count = 0;
            state.next_report = now + Duration::from_secs(1);
        }
    }

    fn update_scene_size(&mut self, state: &mut AppState) {
        state.scene.size_changed = false;
        let physical = state.scene.window_size;
        self.surface_desc.width = physical.width;
        self.surface_desc.height = physical.height;
        self.surface.configure(&self.device, &self.surface_desc);

        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth texture"),
            size: wgpu::Extent3d {
                width: self.surface_desc.width,
                height: self.surface_desc.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        self.depth_texture_view =
            Some(depth_texture.create_view(&wgpu::TextureViewDescriptor::default()));

        self.multisampled_render_target = if SAMPLE_COUNT > 1 {
            Some(create_multisampled_framebuffer(
                &self.device,
                &self.surface_desc,
            ))
        } else {
            None
        };
    }
}

/// Creates a texture that uses MSAA and fits a given swap chain
fn create_multisampled_framebuffer(
    device: &wgpu::Device,
    desc: &wgpu::SurfaceConfiguration,
) -> wgpu::TextureView {
    let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
        label: Some("Multisampled frame descriptor"),
        size: wgpu::Extent3d {
            width: desc.width,
            height: desc.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: SAMPLE_COUNT,
        dimension: wgpu::TextureDimension::D2,
        format: desc.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    };

    device
        .create_texture(multisampled_frame_descriptor)
        .create_view(&wgpu::TextureViewDescriptor::default())
}
