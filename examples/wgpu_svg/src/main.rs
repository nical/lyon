use clap::*;
use lyon::math::Point;
use lyon::path::PathEvent;
use lyon::tessellation::geometry_builder::*;
use lyon::tessellation::{self, FillOptions, FillTessellator, StrokeOptions, StrokeTessellator};
use usvg::prelude::*;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use futures::executor::block_on;

use wgpu::util::DeviceExt;

use std::f64::NAN;

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

fn main() {
    // Grab some parameters from the command line.

    let app = App::new("Lyon svg_render example")
        .version("0.1")
        .arg(
            Arg::with_name("MSAA")
                .long("msaa")
                .short("m")
                .help("Sets MSAA sample count (integer)")
                .value_name("SAMPLES")
                .takes_value(true)
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

    let msaa_samples = if let Some(msaa) = app.value_of("MSAA") {
        match msaa.parse::<u32>() {
            Ok(n) => n.max(1),
            Err(_) => {
                println!("ERROR: `{}` is not a number", msaa);
                std::process::exit(1);
            }
        }
    } else {
        1
    };

    // Parse and tessellate the geometry

    let filename = app.value_of("INPUT").unwrap();

    let mut fill_tess = FillTessellator::new();
    let mut stroke_tess = StrokeTessellator::new();
    let mut mesh: VertexBuffers<_, u32> = VertexBuffers::new();

    let opt = usvg::Options::default();
    let rtree = usvg::Tree::from_file(&filename, &opt).unwrap();
    let mut transforms = Vec::new();
    let mut primitives = Vec::new();

    let mut prev_transform = usvg::Transform {
        a: NAN,
        b: NAN,
        c: NAN,
        d: NAN,
        e: NAN,
        f: NAN,
    };
    let view_box = rtree.svg_node().view_box;
    for node in rtree.root().descendants() {
        if let usvg::NodeKind::Path(ref p) = *node.borrow() {
            let t = node.transform();
            if t != prev_transform {
                transforms.push(GpuTransform {
                    data0: [t.a as f32, t.b as f32, t.c as f32, t.d as f32],
                    data1: [t.e as f32, t.f as f32, 0.0, 0.0],
                });
            }
            prev_transform = t;

            let transform_idx = transforms.len() as u32 - 1;

            if let Some(ref fill) = p.fill {
                // fall back to always use color fill
                // no gradients (yet?)
                let color = match fill.paint {
                    usvg::Paint::Color(c) => c,
                    _ => FALLBACK_COLOR,
                };

                primitives.push(GpuPrimitive::new(
                    transform_idx,
                    color,
                    fill.opacity.value() as f32,
                ));

                fill_tess
                    .tessellate(
                        convert_path(p),
                        &FillOptions::tolerance(0.01),
                        &mut BuffersBuilder::new(
                            &mut mesh,
                            VertexCtor {
                                prim_id: primitives.len() as u32 - 1,
                            },
                        ),
                    )
                    .expect("Error during tesselation!");
            }

            if let Some(ref stroke) = p.stroke {
                let (stroke_color, stroke_opts) = convert_stroke(stroke);
                primitives.push(GpuPrimitive::new(
                    transform_idx,
                    stroke_color,
                    stroke.opacity.value() as f32,
                ));
                let _ = stroke_tess.tessellate(
                    convert_path(p),
                    &stroke_opts.with_tolerance(0.01),
                    &mut BuffersBuilder::new(
                        &mut mesh,
                        VertexCtor {
                            prim_id: primitives.len() as u32 - 1,
                        },
                    ),
                );
            }
        }
    }

    if app.is_present("TESS_ONLY") {
        return;
    }

    println!(
        "Finished tesselation: {} vertices, {} indices",
        mesh.vertices.len(),
        mesh.indices.len()
    );

    println!("Use arrow keys to pan, pageup and pagedown to zoom.");

    // Initialize wgpu and send some data to the GPU.

    let vb_width = view_box.rect.size().width as f32;
    let vb_height = view_box.rect.size().height as f32;
    let scale = vb_width / vb_height;

    let (width, height) = if scale < 1.0 {
        (WINDOW_SIZE, WINDOW_SIZE * scale)
    } else {
        (WINDOW_SIZE, WINDOW_SIZE / scale)
    };

    let pan = [vb_width / -2.0, vb_height / -2.0];
    let zoom = 2.0 / f32::max(vb_width, vb_height);
    let mut scene = SceneGlobals {
        zoom,
        pan,
        window_size: PhysicalSize::new(width as u32, height as u32),
        wireframe: false,
        size_changed: true,
    };

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();

    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

    let surface = unsafe { instance.create_surface(&window) };

    // create an adapter
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::Default,
        compatible_surface: Some(&surface),
    }))
    .unwrap();

    // create a device and a queue
    let (device, queue) = block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            features: wgpu::Features::default(),
            limits: wgpu::Limits::default(),
            shader_validation: true,
        },
        // trace_path can be used for API call tracing
        None,
    ))
    .unwrap();

    let size = window.inner_size();

    let mut swap_chain_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8Unorm,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    let mut swap_chain = None;
    let mut msaa_texture = None;

    let vbo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&mesh.vertices),
        usage: wgpu::BufferUsage::VERTEX,
    });

    let ibo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&mesh.indices),
        usage: wgpu::BufferUsage::INDEX,
    });

    let prim_buffer_byte_size = (MAX_PRIMITIVES * std::mem::size_of::<GpuPrimitive>()) as u64;
    let transform_buffer_byte_size = (MAX_TRANSFORMS * std::mem::size_of::<GpuTransform>()) as u64;
    let globals_buffer_byte_size = std::mem::size_of::<GpuGlobals>() as u64;

    let prims_ubo = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Prims ubo"),
        size: prim_buffer_byte_size,
        usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        mapped_at_creation: false,
    });

    let transforms_ubo = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Transforms ubo"),
        size: transform_buffer_byte_size,
        usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        mapped_at_creation: false,
    });

    let globals_ubo = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Globals ubo"),
        size: globals_buffer_byte_size,
        usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        mapped_at_creation: false,
    });
    let vs_module =
        &device.create_shader_module(wgpu::include_spirv!("../shaders/geometry.vert.spv"));
    let fs_module =
        &device.create_shader_module(wgpu::include_spirv!("../shaders/geometry.frag.spv"));
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Bind group layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::UniformBuffer {
                    dynamic: false,
                    min_binding_size: wgpu::BufferSize::new(globals_buffer_byte_size),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::UniformBuffer {
                    dynamic: false,
                    min_binding_size: wgpu::BufferSize::new(prim_buffer_byte_size),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::UniformBuffer {
                    dynamic: false,
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
                resource: wgpu::BindingResource::Buffer(globals_ubo.slice(..)),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(prims_ubo.slice(..)),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Buffer(transforms_ubo.slice(..)),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
        label: None,
    });

    let mut render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
        layout: Some(&pipeline_layout),
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &vs_module,
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &fs_module,
            entry_point: "main",
        }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::None,
            ..Default::default()
        }),
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        color_states: &[wgpu::ColorStateDescriptor {
            format: wgpu::TextureFormat::Bgra8Unorm,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state: None,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint32,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: std::mem::size_of::<GpuVertex>() as u64,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttributeDescriptor {
                        offset: 0,
                        format: wgpu::VertexFormat::Float2,
                        shader_location: 0,
                    },
                    wgpu::VertexAttributeDescriptor {
                        offset: 8,
                        format: wgpu::VertexFormat::Uint,
                        shader_location: 1,
                    },
                ],
            }],
        },
        sample_count: msaa_samples,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
        label: None,
    };

    let render_pipeline = device.create_render_pipeline(&render_pipeline_descriptor);

    // TODO: this isn't what we want: we'd need the equivalent of VK_POLYGON_MODE_LINE,
    // but it doesn't seem to be exposed by wgpu?
    render_pipeline_descriptor.primitive_topology = wgpu::PrimitiveTopology::LineList;
    let wireframe_render_pipeline = device.create_render_pipeline(&render_pipeline_descriptor);

    queue.write_buffer(&transforms_ubo, 0, bytemuck::cast_slice(&transforms));

    queue.write_buffer(&prims_ubo, 0, bytemuck::cast_slice(&primitives));

    // The main loop.

    event_loop.run(move |event, _, control_flow| {
        if update_inputs(event, control_flow, &mut scene) {
            // keep polling inputs.
            return;
        }

        if scene.size_changed || swap_chain.is_none() {
            scene.size_changed = false;
            let physical = scene.window_size;
            swap_chain_desc.width = physical.width;
            swap_chain_desc.height = physical.height;
            swap_chain = Some(device.create_swap_chain(&surface, &swap_chain_desc));
            if msaa_samples > 1 {
                msaa_texture = Some(
                    device
                        .create_texture(&wgpu::TextureDescriptor {
                            label: Some("Multisampled frame descriptor"),
                            size: wgpu::Extent3d {
                                width: swap_chain_desc.width,
                                height: swap_chain_desc.height,
                                depth: 1,
                            },
                            mip_level_count: 1,
                            sample_count: msaa_samples,
                            dimension: wgpu::TextureDimension::D2,
                            format: swap_chain_desc.format,
                            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                        })
                        .create_view(&wgpu::TextureViewDescriptor::default()),
                );
            }
        }

        let swap_chain = swap_chain.as_mut().unwrap();

        let frame = match swap_chain.get_current_frame() {
            Ok(frame) => frame,
            Err(e) => {
                println!("Swap-chain error: {:?}", e);
                return;
            }
        };

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Encoder"),
        });

        queue.write_buffer(
            &globals_ubo,
            0,
            bytemuck::cast_slice(&[GpuGlobals {
                aspect_ratio: scene.window_size.width as f32 / scene.window_size.height as f32,
                zoom: [scene.zoom, scene.zoom],
                pan: scene.pan,
            }]),
        );

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: msaa_texture.as_ref().unwrap_or(&frame.output.view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: true,
                    },
                    resolve_target: if msaa_texture.is_some() {
                        Some(&frame.output.view)
                    } else {
                        None
                    },
                }],
                depth_stencil_attachment: None,
            });

            if scene.wireframe {
                pass.set_pipeline(&wireframe_render_pipeline);
            } else {
                pass.set_pipeline(&render_pipeline);
            }
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_index_buffer(ibo.slice(..));
            pass.set_vertex_buffer(0, vbo.slice(..));

            pass.draw_indexed(0..(mesh.indices.len() as u32), 0, 0..1);
        }

        queue.submit(Some(encoder.finish()));
    });
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

// These mush match the uniform buffer sizes in the vertex shader.
pub static MAX_PRIMITIVES: usize = 512;
pub static MAX_TRANSFORMS: usize = 512;

// Default scene has all values set to zero
#[derive(Copy, Clone, Debug)]
pub struct SceneGlobals {
    pub zoom: f32,
    pub pan: [f32; 2],
    pub window_size: PhysicalSize<u32>,
    pub wireframe: bool,
    pub size_changed: bool,
}

fn update_inputs(
    event: Event<()>,
    control_flow: &mut ControlFlow,
    scene: &mut SceneGlobals,
) -> bool {
    match event {
        Event::MainEventsCleared => {
            return false;
        }
        Event::WindowEvent {
            event: WindowEvent::Destroyed,
            ..
        }
        | Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
            return false;
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(size),
            ..
        } => {
            scene.window_size = size;
            scene.size_changed = true
        }
        Event::WindowEvent {
            event:
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(key),
                            ..
                        },
                    ..
                },
            ..
        } => match key {
            VirtualKeyCode::Escape => {
                *control_flow = ControlFlow::Exit;
                return false;
            }
            VirtualKeyCode::PageDown => {
                scene.zoom *= 0.8;
            }
            VirtualKeyCode::PageUp => {
                scene.zoom *= 1.25;
            }
            VirtualKeyCode::Left => {
                scene.pan[0] -= 50.0 / scene.pan[0];
            }
            VirtualKeyCode::Right => {
                scene.pan[0] += 50.0 / scene.pan[0];
            }
            VirtualKeyCode::Up => {
                scene.pan[1] += 50.0 / scene.pan[1];
            }
            VirtualKeyCode::Down => {
                scene.pan[1] -= 50.0 / scene.pan[1];
            }
            VirtualKeyCode::W => {
                scene.wireframe = !scene.wireframe;
            }
            _key => {}
        },
        _evt => {
            //println!("{:?}", _evt);
        }
    }

    *control_flow = ControlFlow::Poll;

    return true;
}

/// Some glue between usvg's iterators and lyon's.

fn point(x: &f64, y: &f64) -> Point {
    Point::new((*x) as f32, (*y) as f32)
}

pub struct PathConvIter<'a> {
    iter: std::slice::Iter<'a, usvg::PathSegment>,
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
            Some(usvg::PathSegment::MoveTo { x, y }) => {
                if self.needs_end {
                    let last = self.prev;
                    let first = self.first;
                    self.needs_end = false;
                    self.prev = point(x, y);
                    self.deferred = Some(PathEvent::Begin { at: self.prev });
                    self.first = self.prev;
                    Some(PathEvent::End {
                        last,
                        first,
                        close: false,
                    })
                } else {
                    self.first = point(x, y);
                    Some(PathEvent::Begin { at: self.first })
                }
            }
            Some(usvg::PathSegment::LineTo { x, y }) => {
                self.needs_end = true;
                let from = self.prev;
                self.prev = point(x, y);
                Some(PathEvent::Line {
                    from,
                    to: self.prev,
                })
            }
            Some(usvg::PathSegment::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            }) => {
                self.needs_end = true;
                let from = self.prev;
                self.prev = point(x, y);
                Some(PathEvent::Cubic {
                    from,
                    ctrl1: point(x1, y1),
                    ctrl2: point(x2, y2),
                    to: self.prev,
                })
            }
            Some(usvg::PathSegment::ClosePath) => {
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

pub fn convert_path<'a>(p: &'a usvg::Path) -> PathConvIter<'a> {
    PathConvIter {
        iter: p.segments.iter(),
        first: Point::new(0.0, 0.0),
        prev: Point::new(0.0, 0.0),
        deferred: None,
        needs_end: false,
    }
}

pub fn convert_stroke(s: &usvg::Stroke) -> (usvg::Color, StrokeOptions) {
    let color = match s.paint {
        usvg::Paint::Color(c) => c,
        _ => FALLBACK_COLOR,
    };
    let linecap = match s.linecap {
        usvg::LineCap::Butt => tessellation::LineCap::Butt,
        usvg::LineCap::Square => tessellation::LineCap::Square,
        usvg::LineCap::Round => tessellation::LineCap::Round,
    };
    let linejoin = match s.linejoin {
        usvg::LineJoin::Miter => tessellation::LineJoin::Miter,
        usvg::LineJoin::Bevel => tessellation::LineJoin::Bevel,
        usvg::LineJoin::Round => tessellation::LineJoin::Round,
    };

    let opt = StrokeOptions::tolerance(0.01)
        .with_line_width(s.width.value() as f32)
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
