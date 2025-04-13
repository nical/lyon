use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

// For create_buffer_init()
use wgpu::util::DeviceExt;

use futures::executor::block_on;
use std::ops::Rem;
use std::sync::Arc;

use crate::commands::{AntiAliasing, Background, RenderCmd, TessellateCmd};
use lyon::algorithms::aabb::bounding_box;
use lyon::algorithms::hatching::*;
use lyon::geom::LineSegment;
use lyon::math::*;
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::geometry_builder::*;
use lyon::tessellation::{FillOptions, FillTessellator, StrokeTessellator};

const PRIM_BUFFER_LEN: usize = 64;

#[repr(C)]
#[derive(Copy, Clone)]
struct Globals {
    resolution: [f32; 2],
    scroll_offset: [f32; 2],
    bg_color: [f32; 4],
    vignette_color: [f32; 4],
    zoom: f32,
    _pad: [f32; 3],
}

unsafe impl bytemuck::Pod for Globals {}
unsafe impl bytemuck::Zeroable for Globals {}

#[repr(C)]
#[derive(Copy, Clone)]
struct GpuVertex {
    position: [f32; 2],
    normal: [f32; 2],
    prim_id: u32,
}
unsafe impl bytemuck::Pod for GpuVertex {}
unsafe impl bytemuck::Zeroable for GpuVertex {}

#[repr(C)]
#[derive(Copy, Clone)]
struct Primitive {
    color: [f32; 4],
    translate: [f32; 2],
    z_index: i32,
    width: f32,
}
unsafe impl bytemuck::Pod for Primitive {}
unsafe impl bytemuck::Zeroable for Primitive {}

#[repr(C)]
#[derive(Copy, Clone)]
struct BgVertex {
    point: [f32; 2],
}
unsafe impl bytemuck::Pod for BgVertex {}
unsafe impl bytemuck::Zeroable for BgVertex {}

const DEFAULT_WINDOW_WIDTH: f32 = 800.0;
const DEFAULT_WINDOW_HEIGHT: f32 = 800.0;

struct App {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_desc: wgpu::SurfaceConfiguration,
    vbo: wgpu::Buffer,
    ibo: wgpu::Buffer,
    wireframe_ibo: wgpu::Buffer,
    bg_vbo: wgpu::Buffer,
    bg_ibo: wgpu::Buffer,
    prims_ubo: wgpu::Buffer,
    globals_ubo: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    wireframe_render_pipeline: wgpu::RenderPipeline,
    bg_pipeline: wgpu::RenderPipeline,
    multisampled_render_target: Option<wgpu::TextureView>,
    geometry: VertexBuffers<GpuVertex, u32>,
    wireframe_indices: Vec<u32>,
    cpu_primitives: Vec<Primitive>,
    scene: SceneParams,
    depth_texture_view: Option<wgpu::TextureView>,
    sample_count: u32,
    frame_count: f32,
    stroke_prim_id: usize,
    num_instances: u32,
    bg_color: [f32; 4],
    vignette_color: [f32; 4],
}

enum AppState {
    Initializing { cmd: TessellateCmd, render_options: RenderCmd },
    Running(App),
    Closing,
}

impl ApplicationHandler for AppState {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let win_attrs = Window::default_attributes()
            .with_title("Basic SVG example");
        let window = Arc::new(event_loop.create_window(win_attrs).unwrap());

        match self {
            AppState::Initializing { cmd, render_options } => {
                if let Some(app) = App::init(window, cmd, render_options) {
                    *self = AppState::Running(app);
                    return;
                }
            }
            _ => {
                // TODO
            }
        }

        event_loop.exit();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let this = match self {
            AppState::Running(this) => this,
            _ => {
                event_loop.exit();
                return;
            }
        };

        this.update_inputs(event_loop, id, event);

        if this.scene.render {
            this.render();
        }

        if event_loop.exiting() {
            *self = AppState::Closing;
        }
    }
}

impl App {
    fn update_inputs(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let scene = &mut self.scene;
        let initial_scroll = scene.scroll;
        let initial_zoom = scene.zoom;
        match event {
            WindowEvent::RedrawRequested => {
                scene.render = true;
            }
            WindowEvent::Destroyed | WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                scene.window_size = size;
                scene.size_changed = true;
                scene.render = true;
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
                KeyCode::KeyW => scene.show_wireframe = !scene.show_wireframe,
                KeyCode::KeyB => scene.draw_background = !scene.draw_background,
                KeyCode::KeyA => scene.target_stroke_width /= 0.8,
                KeyCode::KeyZ => scene.target_stroke_width *= 0.8,
                _key => {}
            },
            _evt => {}
        };

        if event_loop.exiting() {
            scene.render = false;
            return;
        }

        scene.zoom += (scene.target_zoom - scene.zoom) / 3.0;
        scene.scroll = scene.scroll + (scene.target_scroll - scene.scroll) / 3.0;
        scene.stroke_width =
            scene.stroke_width + (scene.target_stroke_width - scene.stroke_width) / 5.0;

        scene.render |= scene.scroll != initial_scroll || scene.zoom != initial_zoom;

        if scene.render {
            self.window.request_redraw();
        }
    }
}

/// Creates a texture that uses MSAA and fits a given swap chain
fn create_multisampled_framebuffer(
    device: &wgpu::Device,
    desc: &wgpu::SurfaceConfiguration,
    sample_count: u32,
) -> wgpu::TextureView {
    let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
        label: Some("Multisampled frame descriptor"),
        size: wgpu::Extent3d {
            width: desc.width,
            height: desc.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: desc.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    };

    device
        .create_texture(multisampled_frame_descriptor)
        .create_view(&wgpu::TextureViewDescriptor::default())
}

pub fn show_path(cmd: TessellateCmd, render_options: RenderCmd) {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = AppState::Initializing { cmd, render_options };
    event_loop.run_app(&mut app).unwrap();
}

impl App {
    fn init(window: Arc<Window>, cmd: &TessellateCmd, render_options: &RenderCmd) -> Option<App> {
        let mut geometry: VertexBuffers<GpuVertex, u32> = VertexBuffers::new();

        let fill_prim_id = 0;
        let stroke_prim_id = 1;

        let mut fill = FillTessellator::new();
        let mut stroke = StrokeTessellator::new();

        if let Some(options) = cmd.fill {
            fill.tessellate(
                &cmd.path,
                &options,
                &mut BuffersBuilder::new(&mut geometry, WithId(fill_prim_id)),
            )
            .unwrap();

            //for (i, v) in geometry.vertices.iter().enumerate() {
            //    println!("{}: {:?}", i, v.position);
            //}
            //for i in 0..(geometry.indices.len() / 3) {
            //    println!(
            //        "{}/{}/{}",
            //        geometry.indices[i * 3],
            //        geometry.indices[i * 3 + 1],
            //        geometry.indices[i * 3 + 2],
            //    );
            //}
        }

        if let Some(options) = cmd.stroke {
            stroke
                .tessellate_path(
                    &cmd.path,
                    &options,
                    &mut BuffersBuilder::new(&mut geometry, WithId(stroke_prim_id)),
                )
                .unwrap();
        }

        if let Some(hatch) = &cmd.hatch {
            let mut path = Path::builder();
            let mut hatcher = Hatcher::new();
            hatcher.hatch_path(
                cmd.path.iter(),
                &hatch.options,
                &mut RegularHatchingPattern {
                    interval: hatch.spacing,
                    callback: &mut |segment: &HatchSegment| {
                        path.add_line_segment(&LineSegment {
                            from: segment.a.position,
                            to: segment.b.position,
                        });
                    },
                },
            );
            let hatched_path = path.build();

            stroke
                .tessellate(
                    hatched_path.iter(),
                    &hatch.stroke,
                    &mut BuffersBuilder::new(&mut geometry, WithId(stroke_prim_id)),
                )
                .unwrap();
        }

        if let Some(dots) = &cmd.dots {
            let mut path = Path::builder();
            let mut hatcher = Hatcher::new();
            hatcher.dot_path(
                cmd.path.iter(),
                &dots.options,
                &mut RegularDotPattern {
                    row_interval: dots.spacing,
                    column_interval: dots.spacing,
                    callback: &mut |dot: &Dot| {
                        path.add_point(dot.position);
                    },
                },
            );
            let dotted_path = path.build();

            stroke
                .tessellate(
                    dotted_path.iter(),
                    &dots.stroke,
                    &mut BuffersBuilder::new(&mut geometry, WithId(stroke_prim_id)),
                )
                .unwrap();
        }

        let (bg_color, vignette_color) = match render_options.background {
            Background::Blue => ([0.0, 0.47, 0.9, 1.0], [0.0, 0.1, 0.64, 1.0]),
            Background::Clear => ([0.9, 0.9, 0.9, 1.0], [0.5, 0.5, 0.5, 1.0]),
            Background::Dark => ([0.05, 0.05, 0.05, 1.0], [0.0, 0.0, 0.0, 1.0]),
        };

        if geometry.vertices.is_empty() {
            println!("No geometry to show");
            return None;
        }

        let mut bg_geometry: VertexBuffers<BgVertex, u32> = VertexBuffers::new();

        fill.tessellate_rectangle(
            &Box2D {
                min: point(-1.0, -1.0),
                max: point(1.0, 1.0),
            },
            &FillOptions::DEFAULT,
            &mut BuffersBuilder::new(&mut bg_geometry, BgVertexCtor),
        )
        .unwrap();

        let sample_count = match render_options.aa {
            AntiAliasing::Msaa(samples) => samples as u32,
            _ => 1,
        };

        let num_instances: u32 = PRIM_BUFFER_LEN as u32 - 1;

        let mut cpu_primitives = Vec::with_capacity(PRIM_BUFFER_LEN);
        for _ in 0..PRIM_BUFFER_LEN {
            cpu_primitives.push(Primitive {
                color: [1.0, 0.0, 0.0, 1.0],
                z_index: 0,
                width: 0.0,
                translate: [0.0, 0.0],
            });
        }

        // Stroke primitive
        cpu_primitives[stroke_prim_id] = Primitive {
            color: [0.0, 0.0, 0.0, 1.0],
            z_index: num_instances as i32 + 2,
            width: 1.0,
            translate: [0.0, 0.0],
        };
        // Main fill primitive
        cpu_primitives[fill_prim_id] = Primitive {
            color: [1.0, 1.0, 1.0, 1.0],
            z_index: num_instances as i32 + 1,
            width: 0.0,
            translate: [0.0, 0.0],
        };
        // Instance primitives
        for (idx, cpu_prim) in cpu_primitives
            .iter_mut()
            .enumerate()
            .skip(fill_prim_id + 1)
            .take(num_instances as usize - 1)
        {
            cpu_prim.z_index = (idx as u32 + 1) as i32;
            cpu_prim.color = [
                (0.1 * idx as f32).rem(1.0),
                (0.5 * idx as f32).rem(1.0),
                (0.9 * idx as f32).rem(1.0),
                1.0,
            ];
        }

        let aabb = bounding_box(cmd.path.iter());
        let center = aabb.center().to_vector();

        let scene = SceneParams {
            target_zoom: 5.0,
            zoom: 5.0,
            target_scroll: center,
            scroll: center,
            show_points: false,
            show_wireframe: false,
            stroke_width: 1.0,
            target_stroke_width: 1.0,
            draw_background: true,
            cursor_position: (0.0, 0.0),
            window_size: PhysicalSize::new(DEFAULT_WINDOW_WIDTH as u32, DEFAULT_WINDOW_HEIGHT as u32),
            size_changed: true,
            render: true,
        };


        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).unwrap();
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
                required_features: wgpu::Features::default(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            },
        ))
        .unwrap();

        let vbo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&geometry.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let ibo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&geometry.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let bg_vbo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&bg_geometry.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let bg_ibo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&bg_geometry.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let prim_buffer_byte_size = (PRIM_BUFFER_LEN * std::mem::size_of::<Primitive>()) as u64;
        let globals_buffer_byte_size = std::mem::size_of::<Globals>() as u64;

        let prims_ubo = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Prims ubo"),
            size: prim_buffer_byte_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let globals_ubo = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Globals ubo"),
            size: globals_buffer_byte_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let vs_module = &device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Geometry vs"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./../shaders/geometry.vs.wgsl").into()),
        });
        let fs_module = &device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Geometry fs"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./../shaders/geometry.fs.wgsl").into()),
        });
        let bg_vs_module = &device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Background vs"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./../shaders/background.vs.wgsl").into()),
        });
        let bg_fs_module = &device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Background fs"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./../shaders/background.fs.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
            label: None,
        });

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

        let mut render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: vs_module,
                entry_point: Some("main"),
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
                            format: wgpu::VertexFormat::Float32x2,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            offset: 16,
                            format: wgpu::VertexFormat::Uint32,
                            shader_location: 2,
                        },
                    ],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: fs_module,
                entry_point: Some("main"),
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
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: depth_stencil_state.clone(),
            multisample: wgpu::MultisampleState {
                count: sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        };

        let render_pipeline = device.create_render_pipeline(&render_pipeline_descriptor);

        render_pipeline_descriptor.primitive.topology = wgpu::PrimitiveTopology::LineList;
        let wireframe_render_pipeline = device.create_render_pipeline(&render_pipeline_descriptor);

        let wireframe_indices = build_wireframe_indices(&geometry.indices);
        let wireframe_ibo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&wireframe_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let bg_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: bg_vs_module,
                entry_point: Some("main"),
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
                entry_point: Some("main"),
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
                count: sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let size = window.inner_size();

        let surface_desc = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };

        surface.configure(&device, &surface_desc);

        Some(App {
            window,
            scene,
            device,
            queue,
            surface,
            surface_desc,
            vbo,
            ibo,
            wireframe_ibo,
            bg_vbo,
            bg_ibo,
            wireframe_indices,
            prims_ubo,
            globals_ubo,
            bind_group,
            render_pipeline,
            wireframe_render_pipeline,
            bg_pipeline,
            multisampled_render_target: None,
            geometry,
            cpu_primitives,
            depth_texture_view: None,
            frame_count: 0.0,
            sample_count,
            stroke_prim_id,
            num_instances,
            bg_color,
            vignette_color,
        })
    }

    fn render(&mut self) {
        if self.scene.size_changed {
            self.scene.size_changed = false;
            let physical = self.scene.window_size;
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
                sample_count: self.sample_count,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            self.depth_texture_view =
                Some(depth_texture.create_view(&wgpu::TextureViewDescriptor::default()));

            self.multisampled_render_target = if self.sample_count > 1 {
                Some(create_multisampled_framebuffer(
                    &self.device,
                    &self.surface_desc,
                    self.sample_count,
                ))
            } else {
                None
            };
        }

        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                println!("Swap-chain error: {e:?}");
                return;
            }
        };

        let frame_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Encoder"),
        });

        self.cpu_primitives[self.stroke_prim_id].width = self.scene.stroke_width;
        self.cpu_primitives[self.stroke_prim_id].color = [
            (self.frame_count * 0.008 - 1.6).sin() * 0.1 + 0.1,
            (self.frame_count * 0.005 - 1.6).sin() * 0.1 + 0.1,
            (self.frame_count * 0.01 - 1.6).sin() * 0.1 + 0.1,
            1.0,
        ];

        for idx in 2..(self.num_instances + 1) {
            self.cpu_primitives[idx as usize].translate = [
                (self.frame_count * 0.001 * idx as f32).sin() * (100.0 + idx as f32 * 10.0),
                (self.frame_count * 0.002 * idx as f32).sin() * (100.0 + idx as f32 * 10.0),
            ];
        }

        self.queue.write_buffer(
            &self.globals_ubo,
            0,
            bytemuck::cast_slice(&[Globals {
                resolution: [
                    self.scene.window_size.width as f32,
                    self.scene.window_size.height as f32,
                ],
                zoom: self.scene.zoom,
                scroll_offset: self.scene.scroll.to_array(),
                bg_color: self.bg_color,
                vignette_color: self.vignette_color,
                _pad: [0.0; 3],
            }]),
        );

        self.queue.write_buffer(&self.prims_ubo, 0, bytemuck::cast_slice(&self.cpu_primitives));

        {
            // A resolve target is only supported if the attachment actually uses anti-aliasing
            // So if sample_count == 1 then we must render directly to the swapchain's buffer
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

            let index_range = if self.scene.show_wireframe {
                pass.set_pipeline(&self.wireframe_render_pipeline);
                pass.set_index_buffer(self.wireframe_ibo.slice(..), wgpu::IndexFormat::Uint32);
                0..(self.wireframe_indices.len() as u32)
            } else {
                pass.set_pipeline(&self.render_pipeline);
                pass.set_index_buffer(self.ibo.slice(..), wgpu::IndexFormat::Uint32);
                0..(self.geometry.indices.len() as u32)
            };

            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.vbo.slice(..));

            pass.draw_indexed(index_range, 0, 0..1);

            if self.scene.draw_background {
                pass.set_pipeline(&self.bg_pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_index_buffer(self.bg_ibo.slice(..), wgpu::IndexFormat::Uint32);
                pass.set_vertex_buffer(0, self.bg_vbo.slice(..));

                pass.draw_indexed(0..6, 0, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        self.frame_count += 1.0;
        self.scene.render = false;
    }
}

fn build_wireframe_indices(indices: &[u32]) -> Vec<u32> {
    let mut set = std::collections::HashSet::new();
    let check = &mut |a: u32, b: u32| {
        let (i1, i2) = if a < b { (a, b) } else { (b, a) };

        set.insert((i1, i2))
    };

    let mut output = Vec::new();

    for triangle in indices.chunks(3) {
        let a = triangle[0];
        let b = triangle[1];
        let c = triangle[2];
        if check(a, b) {
            output.push(a);
            output.push(b);
        }
        if check(b, c) {
            output.push(b);
            output.push(c);
        }
        if check(a, c) {
            output.push(a);
            output.push(c);
        }
    }

    output
}

/// This vertex constructor forwards the positions and normals provided by the
/// tessellators and add a shape id.
pub struct WithId(pub usize);

impl FillVertexConstructor<GpuVertex> for WithId {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> GpuVertex {
        GpuVertex {
            position: vertex.position().to_array(),
            normal: [0.0, 0.0],
            prim_id: self.0 as u32,
        }
    }
}

impl StrokeVertexConstructor<GpuVertex> for WithId {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex) -> GpuVertex {
        let p = vertex.position_on_path();
        GpuVertex {
            position: p.to_array(),
            normal: (vertex.position() - p).to_array(),
            prim_id: self.0 as u32,
        }
    }
}

pub struct BgVertexCtor;

impl FillVertexConstructor<BgVertex> for BgVertexCtor {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> BgVertex {
        BgVertex {
            point: vertex.position().to_array(),
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
    window_size: PhysicalSize<u32>,
    size_changed: bool,
    render: bool,
}
