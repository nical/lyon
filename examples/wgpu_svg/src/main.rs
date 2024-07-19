use clap::*;
use lyon::math::Point;
use lyon::path::PathEvent;
use lyon::tessellation::geometry_builder::*;
use lyon::tessellation::{self, FillOptions, FillTessellator, StrokeOptions, StrokeTessellator};
use usvg::*;
use wgpu::include_wgsl;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};
use std::sync::Arc;
use futures::executor::block_on;

use wgpu::util::DeviceExt;

const WINDOW_SIZE: f32 = 800.0;

pub const FALLBACK_COLOR: usvg::Color = usvg::Color {
    red: 0,
    green: 0,
    blue: 0,
};

// This example renders a very tiny subset of SVG (only filled and stroke paths with solid color
// patterns and transforms).
//
// Parsing is done via the usvg crate. In this very simple example, paths are all tessellated directly
// into a static mesh during parsing.
// vertices embed a primitive ID which lets the vertex shader fetch the per-path information such like
// the color from uniform buffer objects.
// No occlusion culling optimization here (see the wgpu example).
//
// Most of the code in this example is related to working with the GPU.

struct App {
    window: Option<Arc<Window>>,
    gfx: Option<GfxState>,
}

pub struct GfxState {
    scene: SceneGlobals,
    mesh: VertexBuffers<GpuVertex, u32>,
    surface_desc: wgpu::SurfaceConfiguration,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    vbo: wgpu::Buffer,
    ibo: wgpu::Buffer,
    globals_ubo: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    wireframe_render_pipeline: wgpu::RenderPipeline,
    msaa_texture: Option<wgpu::TextureView>,
    msaa_samples: u32,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let win_attrs = Window::default_attributes()
            .with_title("Basic SVG example");
        let window = Arc::new(event_loop.create_window(win_attrs).unwrap());

        self.gfx = init(window.clone());
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let gfx = if let Some(gfx) = &mut self.gfx {
            gfx
        } else {
            event_loop.exit();
            return;
        };

        let scene = &mut gfx.scene;
        let mut should_render = false;

        let initial_pan = scene.pan;
        let initial_zoom = scene.zoom;

        match event {
            WindowEvent::RedrawRequested => {
                should_render = true;
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
                KeyCode::PageDown => scene.zoom *= 0.8,
                KeyCode::PageUp => scene.zoom *= 1.25,
                KeyCode::ArrowLeft => scene.pan[0] += 50.0 / scene.zoom,
                KeyCode::ArrowRight => scene.pan[0] -= 50.0 / scene.zoom,
                KeyCode::ArrowUp => scene.pan[1] -= 50.0 / scene.zoom,
                KeyCode::ArrowDown => scene.pan[1] += 50.0 / scene.zoom,
                _key => {}
            },
            _evt => {}
        };

        if event_loop.exiting() {
            return;
        }

        should_render |= scene.pan != initial_pan || scene.zoom != initial_zoom;

        if should_render {
            render(gfx);
        }
    }
}

fn main() {
    // Grab some parameters from the command line.

    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App {
        window: None,
        gfx: None,
    };

    event_loop.run_app(&mut app).unwrap();
}

fn init(window: Arc<Window>) -> Option<GfxState> {
    let args = clap::App::new("Lyon svg_render example")
        .version("0.1")
        .arg(
            Arg::with_name("MSAA")
                .long("msaa")
                .short("m")
                .help("Enable msaa")
                .value_name("SAMPLES")
                .takes_value(false)
                .required(false),
        )
        .arg(
            Arg::with_name("INPUT")
                .help("SVG or SVGZ file")
                .value_name("INPUT")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("TESS_ONLY")
                .help("Perform the tessellation and exit without rendering")
                .value_name("TESS_ONLY")
                .long("tessellate-only")
                .short("t")
                .takes_value(false)
                .required(false),
        )
        .get_matches();

    let msaa_samples = if args.is_present("MSAA") { 4 } else { 1 };

    // Parse and tessellate the geometry

    let filename = args.value_of("INPUT").unwrap();

    let mut fill_tess = FillTessellator::new();
    let mut stroke_tess = StrokeTessellator::new();
    let mut mesh: VertexBuffers<_, u32> = VertexBuffers::new();

    let opt = usvg::Options::default();
    let file_data = std::fs::read(filename).unwrap();
    let db = usvg::fontdb::Database::new();
    let rtree = usvg::Tree::from_data(&file_data, &opt, &db).unwrap();
    let mut transforms = Vec::new();
    let mut primitives = Vec::new();

    let mut prev_transform = usvg::Transform {
        sx: f32::NAN,
        kx: f32::NAN,
        ky: f32::NAN,
        sy: f32::NAN,
        tx: f32::NAN,
        ty: f32::NAN,
    };
    let view_box = rtree.view_box();
    collect_geom(
        &rtree.root(),
        &mut prev_transform,
        &mut transforms,
        &mut primitives,
        &mut fill_tess,
        &mut mesh,
        &mut stroke_tess,
    );

    if args.is_present("TESS_ONLY") {
        return None;
    }

    println!(
        "Finished tessellation: {} vertices, {} indices",
        mesh.vertices.len(),
        mesh.indices.len()
    );

    println!("Use arrow keys to pan, pageup and pagedown to zoom.");

    // Initialize wgpu and send some data to the GPU.

    let vb_width = view_box.rect.size().width() as f32;
    let vb_height = view_box.rect.size().height() as f32;
    let scale = vb_width / vb_height;

    let (width, height) = if scale < 1.0 {
        (WINDOW_SIZE, WINDOW_SIZE * scale)
    } else {
        (WINDOW_SIZE, WINDOW_SIZE / scale)
    };

    let pan = [vb_width / -2.0, vb_height / -2.0];
    let zoom = 2.0 / f32::max(vb_width, vb_height);
    let scene = SceneGlobals {
        zoom,
        pan,
        window_size: PhysicalSize::new(width as u32, height as u32),
        wireframe: false,
        size_changed: true,
    };

    let instance = wgpu::Instance::default();

    let surface = instance.create_surface(window.clone()).unwrap();

    // create an adapter
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .unwrap();

    // create a device and a queue
    let (device, queue) = block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::default() | wgpu::Features::POLYGON_MODE_LINE,
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
        },
        // trace_path can be used for API call tracing
        None,
    ))
    .unwrap();

    let size = window.inner_size();

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

    let vbo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&mesh.vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let ibo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&mesh.indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    let prim_buffer_byte_size = (primitives.len() * std::mem::size_of::<GpuPrimitive>()) as u64;
    let transform_buffer_byte_size =
        (transforms.len() * std::mem::size_of::<GpuTransform>()) as u64;
    let globals_buffer_byte_size = std::mem::size_of::<GpuGlobals>() as u64;

    let prims_ssbo = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Prims ssbo"),
        size: prim_buffer_byte_size,
        usage: wgpu::BufferUsages::VERTEX
            | wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let transforms_ssbo = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Transforms ssbo"),
        size: transform_buffer_byte_size,
        usage: wgpu::BufferUsages::VERTEX
            | wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let globals_ubo = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Globals ubo"),
        size: globals_buffer_byte_size,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let vs_module = device.create_shader_module(include_wgsl!("../shaders/geometry.vs.wgsl"));
    let fs_module = device.create_shader_module(include_wgsl!("../shaders/geometry.fs.wgsl"));
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
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(prim_buffer_byte_size),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(transform_buffer_byte_size),
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
                resource: wgpu::BindingResource::Buffer(prims_ssbo.as_entire_buffer_binding()),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Buffer(transforms_ssbo.as_entire_buffer_binding()),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
        label: None,
    });

    let mut render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &vs_module,
            entry_point: "main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<GpuVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        offset: 0,
                        format: wgpu::VertexFormat::Float32x2,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        offset: 8,
                        format: wgpu::VertexFormat::Uint32,
                        shader_location: 1,
                    },
                ],
            }],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &fs_module,
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
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: msaa_samples,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    };

    let render_pipeline = device.create_render_pipeline(&render_pipeline_descriptor);

    // TODO: this isn't what we want: we'd need the equivalent of VK_POLYGON_MODE_LINE,
    // but it doesn't seem to be exposed by wgpu?
    render_pipeline_descriptor.primitive.polygon_mode = wgpu::PolygonMode::Line;
    let wireframe_render_pipeline = device.create_render_pipeline(&render_pipeline_descriptor);

    queue.write_buffer(&transforms_ssbo, 0, bytemuck::cast_slice(&transforms));

    queue.write_buffer(&prims_ssbo, 0, bytemuck::cast_slice(&primitives));

    window.request_redraw();

    return Some(GfxState {
        scene,
        mesh,
        device,
        queue,
        vbo,
        ibo,
        globals_ubo,
        surface_desc,
        surface,
        bind_group,
        render_pipeline,
        wireframe_render_pipeline,
        msaa_samples,
        msaa_texture: None,
    });
}

fn render(gfx: &mut GfxState) {
    if gfx.scene.size_changed {
        gfx.scene.size_changed = false;
        let physical = gfx.scene.window_size;
        gfx.surface_desc.width = physical.width;
        gfx.surface_desc.height = physical.height;
        gfx.surface.configure(&gfx.device, &gfx.surface_desc);
        if gfx.msaa_samples > 1 {
            gfx.msaa_texture = Some(
                gfx.device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("Multisampled frame descriptor"),
                        size: wgpu::Extent3d {
                            width: gfx.surface_desc.width,
                            height: gfx.surface_desc.height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: gfx.msaa_samples,
                        dimension: wgpu::TextureDimension::D2,
                        format: gfx.surface_desc.format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        view_formats: &[],
                    })
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            );
        }
    }

    let frame = match gfx.surface.get_current_texture() {
        Ok(frame) => frame,
        Err(e) => {
            println!("Swap-chain error: {e:?}");
            return;
        }
    };

    let frame_view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    let mut encoder = gfx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Encoder"),
    });

    gfx.queue.write_buffer(
        &gfx.globals_ubo,
        0,
        bytemuck::cast_slice(&[GpuGlobals {
            aspect_ratio: gfx.scene.window_size.width as f32 / gfx.scene.window_size.height as f32,
            zoom: [gfx.scene.zoom, gfx.scene.zoom],
            pan: gfx.scene.pan,
            _pad: 0.0,
        }]),
    );

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: gfx.msaa_texture.as_ref().unwrap_or(&frame_view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: if gfx.msaa_texture.is_some() {
                    Some(&frame_view)
                } else {
                    None
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if gfx.scene.wireframe {
            pass.set_pipeline(&gfx.wireframe_render_pipeline);
        } else {
            pass.set_pipeline(&gfx.render_pipeline);
        }
        pass.set_bind_group(0, &gfx.bind_group, &[]);
        pass.set_index_buffer(gfx.ibo.slice(..), wgpu::IndexFormat::Uint32);
        pass.set_vertex_buffer(0, gfx.vbo.slice(..));

        pass.draw_indexed(0..(gfx.mesh.indices.len() as u32), 0, 0..1);
    }

    gfx.queue.submit(Some(encoder.finish()));
    frame.present();
}

fn collect_geom(
    group: &Group,
    prev_transform: &mut Transform,
    transforms: &mut Vec<GpuTransform>,
    primitives: &mut Vec<GpuPrimitive>,
    fill_tess: &mut FillTessellator,
    mesh: &mut VertexBuffers<GpuVertex, u32>,
    stroke_tess: &mut StrokeTessellator,
) {
    for node in group.children() {
        if let usvg::Node::Group(group) = node {
            collect_geom(
                group,
                prev_transform,
                transforms,
                primitives,
                fill_tess,
                mesh,
                stroke_tess,
            )
        } else if let usvg::Node::Path(p) = &node {
            let t = node.abs_transform();
            if t != *prev_transform {
                transforms.push(GpuTransform {
                    data0: [t.sx, t.kx, t.ky, t.sy],
                    data1: [t.tx, t.ty, 0.0, 0.0],
                });
            }
            *prev_transform = t;

            let transform_idx = transforms.len() as u32 - 1;

            if let Some( fill) = p.fill() {
                // fall back to always use color fill
                // no gradients (yet?)
                let color = match fill.paint() {
                    usvg::Paint::Color(c) => *c,
                    _ => FALLBACK_COLOR,
                };

                primitives.push(GpuPrimitive::new(
                    transform_idx,
                    color,
                    fill.opacity().get(),
                ));

                fill_tess
                    .tessellate(
                        convert_path(p),
                        &FillOptions::tolerance(0.01),
                        &mut BuffersBuilder::new(
                            mesh,
                            VertexCtor {
                                prim_id: primitives.len() as u32 - 1,
                            },
                        ),
                    )
                    .expect("Error during tessellation!");
            }

            if let Some(stroke) = p.stroke() {
                let (stroke_color, stroke_opts) = convert_stroke(stroke);
                primitives.push(GpuPrimitive::new(
                    transform_idx,
                    stroke_color,
                    stroke.opacity().get(),
                ));
                let _ = stroke_tess.tessellate(
                    convert_path(p),
                    &stroke_opts.with_tolerance(0.01),
                    &mut BuffersBuilder::new(
                        mesh,
                        VertexCtor {
                            prim_id: primitives.len() as u32 - 1,
                        },
                    ),
                );
            }
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GpuVertex {
    pub position: [f32; 2],
    pub prim_id: u32,
}

// A 2x3 matrix (last two members of data1 unused).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct GpuTransform {
    pub data0: [f32; 4],
    pub data1: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GpuPrimitive {
    pub transform: u32,
    pub color: u32,
    pub _pad: [u32; 2],
}

impl GpuPrimitive {
    pub fn new(transform_idx: u32, color: usvg::Color, alpha: f32) -> Self {
        GpuPrimitive {
            transform: transform_idx,
            color: ((color.red as u32) << 24)
                + ((color.green as u32) << 16)
                + ((color.blue as u32) << 8)
                + (alpha * 255.0) as u32,
            _pad: [0; 2],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GpuGlobals {
    pub zoom: [f32; 2],
    pub pan: [f32; 2],
    pub aspect_ratio: f32,
    pub _pad: f32,
}

pub struct VertexCtor {
    pub prim_id: u32,
}

impl FillVertexConstructor<GpuVertex> for VertexCtor {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> GpuVertex {
        GpuVertex {
            position: vertex.position().to_array(),
            prim_id: self.prim_id,
        }
    }
}

impl StrokeVertexConstructor<GpuVertex> for VertexCtor {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex) -> GpuVertex {
        GpuVertex {
            position: vertex.position().to_array(),
            prim_id: self.prim_id,
        }
    }
}

// Default scene has all values set to zero
#[derive(Copy, Clone, Debug)]
pub struct SceneGlobals {
    pub zoom: f32,
    pub pan: [f32; 2],
    pub window_size: PhysicalSize<u32>,
    pub wireframe: bool,
    pub size_changed: bool,
}

/// Some glue between usvg's iterators and lyon's.
pub struct PathConvIter<'a> {
    iter: tiny_skia_path::PathSegmentsIter<'a>,
    prev: Point,
    first: Point,
    needs_end: bool,
    deferred: Option<PathEvent>,
}

impl<'l> Iterator for PathConvIter<'l> {
    type Item = PathEvent;
    fn next(&mut self) -> Option<PathEvent> {
        if self.deferred.is_some() {
            return self.deferred.take();
        }

        let next = self.iter.next();
        match next {
            Some(tiny_skia_path::PathSegment::MoveTo(pt)) => {
                if self.needs_end {
                    let last = self.prev;
                    let first = self.first;
                    self.needs_end = false;
                    self.prev = Point::new(pt.x, pt.y);
                    self.deferred = Some(PathEvent::Begin { at: self.prev });
                    self.first = self.prev;
                    Some(PathEvent::End {
                        last,
                        first,
                        close: false,
                    })
                } else {
                    self.first = Point::new(pt.x, pt.y);
                    self.needs_end = true;
                    Some(PathEvent::Begin { at: self.first })
                }
            }
            Some(tiny_skia_path::PathSegment::LineTo(pt)) => {
                self.needs_end = true;
                let from = self.prev;
                self.prev = Point::new(pt.x, pt.y);
                Some(PathEvent::Line {
                    from,
                    to: self.prev,
                })
            }
            Some(tiny_skia_path::PathSegment::CubicTo(p1, p2, p0)) => {
                self.needs_end = true;
                let from = self.prev;
                self.prev = Point::new(p0.x, p0.y);
                Some(PathEvent::Cubic {
                    from,
                    ctrl1: Point::new(p1.x, p1.y),
                    ctrl2: Point::new(p2.x, p2.y),
                    to: self.prev,
                })
            }
            Some(tiny_skia_path::PathSegment::QuadTo(p1, p0)) => {
                self.needs_end = true;
                let from = self.prev;
                self.prev = Point::new(p1.x, p1.y);
                Some(PathEvent::Quadratic {
                    from,
                    ctrl: Point::new(p0.x, p0.y),
                    to: self.prev,
                })
            }
            Some(tiny_skia_path::PathSegment::Close) => {
                self.needs_end = false;
                self.prev = self.first;
                Some(PathEvent::End {
                    last: self.prev,
                    first: self.first,
                    close: true,
                })
            }
            None => {
                if self.needs_end {
                    self.needs_end = false;
                    let last = self.prev;
                    let first = self.first;
                    Some(PathEvent::End {
                        last,
                        first,
                        close: false,
                    })
                } else {
                    None
                }
            }
        }
    }
}

pub fn convert_path(p: &usvg::Path) -> PathConvIter {
    PathConvIter {
        iter: p.data().segments(),
        first: Point::new(0.0, 0.0),
        prev: Point::new(0.0, 0.0),
        deferred: None,
        needs_end: false,
    }
}

pub fn convert_stroke(s: &usvg::Stroke) -> (usvg::Color, StrokeOptions) {
    let color = match s.paint() {
        usvg::Paint::Color(c) => *c,
        _ => FALLBACK_COLOR,
    };
    let linecap = match s.linecap() {
        usvg::LineCap::Butt => tessellation::LineCap::Butt,
        usvg::LineCap::Square => tessellation::LineCap::Square,
        usvg::LineCap::Round => tessellation::LineCap::Round,
    };
    let linejoin = match s.linejoin() {
        usvg::LineJoin::Miter => tessellation::LineJoin::Miter,
        usvg::LineJoin::MiterClip => tessellation::LineJoin::MiterClip,
        usvg::LineJoin::Bevel => tessellation::LineJoin::Bevel,
        usvg::LineJoin::Round => tessellation::LineJoin::Round,
    };

    let opt = StrokeOptions::tolerance(0.01)
        .with_line_width(s.width().get())
        .with_line_cap(linecap)
        .with_line_join(linejoin);

    (color, opt)
}

unsafe impl bytemuck::Pod for GpuGlobals {}
unsafe impl bytemuck::Zeroable for GpuGlobals {}
unsafe impl bytemuck::Pod for GpuVertex {}
unsafe impl bytemuck::Zeroable for GpuVertex {}
unsafe impl bytemuck::Pod for GpuPrimitive {}
unsafe impl bytemuck::Zeroable for GpuPrimitive {}
unsafe impl bytemuck::Pod for GpuTransform {}
unsafe impl bytemuck::Zeroable for GpuTransform {}
