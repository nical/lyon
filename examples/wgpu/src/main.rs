use lyon::extra::rust_logo::build_logo_path;
use lyon::math::*;
use lyon::path::builder::*;
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::geometry_builder::*;
use lyon::tessellation::{FillOptions, FillTessellator};
use lyon::tessellation::{StrokeOptions, StrokeTessellator};

use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use futures::executor::block_on;
use std::ops::Rem;

//use log;

const PRIM_BUFFER_LEN: usize = 64;

#[repr(C)]
#[derive(Copy, Clone)]
struct Globals {
    resolution: [f32; 2],
    scroll_offset: [f32; 2],
    zoom: f32,
}

unsafe impl bytemuck::Pod for Globals {}
unsafe impl bytemuck::Zeroable for Globals {}

#[repr(C)]
#[derive(Copy, Clone)]
struct GpuVertex {
    position: [f32; 2],
    normal: [f32; 2],
    prim_id: i32,
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
struct BgPoint {
    point: [f32; 2],
}
unsafe impl bytemuck::Pod for BgPoint {}
unsafe impl bytemuck::Zeroable for BgPoint {}

const DEFAULT_WINDOW_WIDTH: f32 = 800.0;
const DEFAULT_WINDOW_HEIGHT: f32 = 800.0;

/// Creates a texture that uses MSAA and fits a given swap chain
fn create_multisampled_framebuffer(
    device: &wgpu::Device,
    sc_desc: &wgpu::SwapChainDescriptor,
    sample_count: u32,
) -> wgpu::TextureView {
    let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
        label: Some("Multisampled frame descriptor"),
        size: wgpu::Extent3d {
            width: sc_desc.width,
            height: sc_desc.height,
            depth: 1,
        },
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: sc_desc.format,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
    };

    device
        .create_texture(multisampled_frame_descriptor)
        .create_default_view()
}

fn main() {
    env_logger::init();
    println!("== wgpu example ==");
    println!("Controls:");
    println!("  Arrow keys: scrolling");
    println!("  PgUp/PgDown: zoom in/out");
    println!("  w: toggle wireframe mode");
    println!("  b: toggle drawing the background");
    println!("  a/z: increase/decrease the stroke width");

    // Number of samples for anti-aliasing
    // Set to 1 to disable
    let sample_count = 4;

    let num_instances: u32 = PRIM_BUFFER_LEN as u32 - 1;
    let tolerance = 0.02;

    let stroke_prim_id = 0;
    let fill_prim_id = 1;

    let mut geometry: VertexBuffers<GpuVertex, u16> = VertexBuffers::new();

    let mut fill_tess = FillTessellator::new();
    let mut stroke_tess = StrokeTessellator::new();

    // Build a Path for the rust logo.
    let mut builder = Path::builder().with_svg();
    build_logo_path(&mut builder);
    let path = builder.build();

    let fill_count = fill_tess
        .tessellate_path(
            &path,
            &FillOptions::tolerance(tolerance).with_fill_rule(tessellation::FillRule::NonZero),
            &mut BuffersBuilder::new(&mut geometry, WithId(fill_prim_id as i32)),
        )
        .unwrap();

    stroke_tess
        .tessellate_path(
            &path,
            &StrokeOptions::tolerance(tolerance),
            &mut BuffersBuilder::new(&mut geometry, WithId(stroke_prim_id as i32)),
        )
        .unwrap();

    let fill_range = 0..fill_count.indices;
    let stroke_range = fill_range.end..(geometry.indices.len() as u32);
    let mut bg_geometry: VertexBuffers<BgPoint, u16> = VertexBuffers::new();

    fill_tess
        .tessellate_rectangle(
            &Rect::new(point(-1.0, -1.0), size(2.0, 2.0)),
            &FillOptions::DEFAULT,
            &mut BuffersBuilder::new(&mut bg_geometry, Custom),
        )
        .unwrap();

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
    for idx in (fill_prim_id + 1)..(fill_prim_id + num_instances as usize) {
        cpu_primitives[idx].z_index = (idx as u32 + 1) as i32;
        cpu_primitives[idx].color = [
            (0.1 * idx as f32).rem(1.0),
            (0.5 * idx as f32).rem(1.0),
            (0.9 * idx as f32).rem(1.0),
            1.0,
        ];
    }

    let mut scene = SceneParams {
        target_zoom: 5.0,
        zoom: 5.0,
        target_scroll: vector(70.0, 70.0),
        scroll: vector(70.0, 70.0),
        show_points: false,
        show_wireframe: false,
        stroke_width: 1.0,
        target_stroke_width: 1.0,
        draw_background: true,
        cursor_position: (0.0, 0.0),
        window_size: LogicalSize::new(DEFAULT_WINDOW_WIDTH as f64, DEFAULT_WINDOW_HEIGHT as f64),
        size_changed: true,
    };

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();

    let surface = wgpu::Surface::create(&window);

    let adapter = block_on(wgpu::Adapter::request(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
        },
        wgpu::BackendBit::PRIMARY,
    ))
    .unwrap();

    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        extensions: wgpu::Extensions {
            anisotropic_filtering: false,
        },
        limits: wgpu::Limits::default(),
    }));

    let vbo = device.create_buffer_with_data(
        bytemuck::cast_slice(&geometry.vertices),
        wgpu::BufferUsage::VERTEX,
    );

    let ibo = device.create_buffer_with_data(
        bytemuck::cast_slice(&geometry.indices),
        wgpu::BufferUsage::INDEX,
    );

    let bg_vbo = device.create_buffer_with_data(
        bytemuck::cast_slice(&bg_geometry.vertices),
        wgpu::BufferUsage::VERTEX,
    );

    let bg_ibo = device.create_buffer_with_data(
        bytemuck::cast_slice(&bg_geometry.indices),
        wgpu::BufferUsage::INDEX,
    );

    let prim_buffer_byte_size = (PRIM_BUFFER_LEN * std::mem::size_of::<Primitive>()) as u64;
    let globals_buffer_byte_size = std::mem::size_of::<Globals>() as u64;

    let prims_ubo = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Prims ubo"),
        size: prim_buffer_byte_size,
        usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    });

    let globals_ubo = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Globals ubo"),
        size: globals_buffer_byte_size,
        usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    });

    let vs_bytes = include_bytes!("./../shaders/geometry.vert.spv");
    let fs_bytes = include_bytes!("./../shaders/geometry.frag.spv");
    let vs_spv = wgpu::read_spirv(std::io::Cursor::new(&vs_bytes[..])).unwrap();
    let fs_spv = wgpu::read_spirv(std::io::Cursor::new(&fs_bytes[..])).unwrap();
    let bg_vs_bytes = include_bytes!("./../shaders/background.vert.spv");
    let bg_fs_bytes = include_bytes!("./../shaders/background.frag.spv");
    let bg_vs_spv = wgpu::read_spirv(std::io::Cursor::new(&bg_vs_bytes[..])).unwrap();
    let bg_fs_spv = wgpu::read_spirv(std::io::Cursor::new(&bg_fs_bytes[..])).unwrap();
    let vs_module = device.create_shader_module(&vs_spv);
    let fs_module = device.create_shader_module(&fs_spv);
    let bg_vs_module = device.create_shader_module(&bg_vs_spv);
    let bg_fs_module = device.create_shader_module(&bg_fs_spv);

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Bind group layout"),
        bindings: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::UniformBuffer { dynamic: false },
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::UniformBuffer { dynamic: false },
            },
        ],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Bind group"),
        layout: &bind_group_layout,
        bindings: &[
            wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &globals_ubo,
                    range: 0..globals_buffer_byte_size,
                },
            },
            wgpu::Binding {
                binding: 1,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &prims_ubo,
                    range: 0..prim_buffer_byte_size,
                },
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&bind_group_layout],
    });

    let depth_stencil_state = Some(wgpu::DepthStencilStateDescriptor {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::Greater,
        stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
        stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
        stencil_read_mask: 0,
        stencil_write_mask: 0,
    });

    let mut render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
        layout: &pipeline_layout,
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
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
        }),
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        color_states: &[wgpu::ColorStateDescriptor {
            format: wgpu::TextureFormat::Bgra8Unorm,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state: depth_stencil_state.clone(),
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
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
                        format: wgpu::VertexFormat::Float2,
                        shader_location: 1,
                    },
                    wgpu::VertexAttributeDescriptor {
                        offset: 16,
                        format: wgpu::VertexFormat::Int,
                        shader_location: 2,
                    },
                ],
            }],
        },
        sample_count: sample_count,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    };

    let render_pipeline = device.create_render_pipeline(&render_pipeline_descriptor);

    // TODO: this isn't what we want: we'd need the equivalent of VK_POLYGON_MODE_LINE,
    // but it doesn't seem to be exposed by wgpu?
    render_pipeline_descriptor.primitive_topology = wgpu::PrimitiveTopology::LineList;
    let wireframe_render_pipeline = device.create_render_pipeline(&render_pipeline_descriptor);

    let bg_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: &pipeline_layout,
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &bg_vs_module,
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &bg_fs_module,
            entry_point: "main",
        }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
        }),
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        color_states: &[wgpu::ColorStateDescriptor {
            format: wgpu::TextureFormat::Bgra8Unorm,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state: depth_stencil_state.clone(),
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: std::mem::size_of::<Point>() as u64,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &[wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    format: wgpu::VertexFormat::Float2,
                    shader_location: 0,
                }],
            }],
        },
        sample_count: sample_count,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    });

    let size = window.inner_size().to_physical(window.hidpi_factor());

    let mut swap_chain_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8Unorm,
        width: size.width.round() as u32,
        height: size.height.round() as u32,
        present_mode: wgpu::PresentMode::Fifo,
    };

    let mut multisampled_render_target = None;

    let window_surface = wgpu::Surface::create(&window);
    let mut swap_chain = device.create_swap_chain(&window_surface, &swap_chain_desc);

    let mut depth_texture_view = None;

    let mut frame_count: f32 = 0.0;
    event_loop.run(move |event, _, control_flow| {
        if update_inputs(event, control_flow, &mut scene) {
            // keep polling inputs.
            return;
        }

        if scene.size_changed {
            scene.size_changed = false;
            let physical = scene.window_size.to_physical(window.hidpi_factor());
            swap_chain_desc.width = physical.width.round() as u32;
            swap_chain_desc.height = physical.height.round() as u32;
            swap_chain = device.create_swap_chain(&window_surface, &swap_chain_desc);

            let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth texture"),
                size: wgpu::Extent3d {
                    width: swap_chain_desc.width,
                    height: swap_chain_desc.height,
                    depth: 1,
                },
                array_layer_count: 1,
                mip_level_count: 1,
                sample_count: sample_count,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            });

            depth_texture_view = Some(depth_texture.create_default_view());

            multisampled_render_target = if sample_count > 1 {
                Some(create_multisampled_framebuffer(
                    &device,
                    &swap_chain_desc,
                    sample_count,
                ))
            } else {
                None
            };
        }

        let frame = swap_chain.get_next_texture().unwrap();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Encoder"),
        });

        cpu_primitives[stroke_prim_id as usize].width = scene.stroke_width;
        cpu_primitives[stroke_prim_id as usize].color = [
            (frame_count * 0.008 - 1.6).sin() * 0.1 + 0.1,
            (frame_count * 0.005 - 1.6).sin() * 0.1 + 0.1,
            (frame_count * 0.01 - 1.6).sin() * 0.1 + 0.1,
            1.0,
        ];

        for idx in 2..(num_instances + 1) {
            cpu_primitives[idx as usize].translate = [
                (frame_count * 0.001 * idx as f32).sin() * (100.0 + idx as f32 * 10.0),
                (frame_count * 0.002 * idx as f32).sin() * (100.0 + idx as f32 * 10.0),
            ];
        }

        let globals_transfer_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&[Globals {
                resolution: [
                    scene.window_size.width as f32,
                    scene.window_size.height as f32,
                ],
                zoom: scene.zoom,
                scroll_offset: scene.scroll.to_array(),
            }]),
            wgpu::BufferUsage::COPY_SRC,
        );

        let prim_transfer_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&cpu_primitives),
            wgpu::BufferUsage::COPY_SRC,
        );

        encoder.copy_buffer_to_buffer(
            &globals_transfer_buffer,
            0,
            &globals_ubo,
            0,
            globals_buffer_byte_size,
        );

        encoder.copy_buffer_to_buffer(
            &prim_transfer_buffer,
            0,
            &prims_ubo,
            0,
            prim_buffer_byte_size,
        );

        {
            // A resolve target is only supported if the attachment actually uses anti-aliasing
            // So if sample_count == 1 then we must render directly to the swapchain's buffer
            let color_attachment = if let Some(msaa_target) = &multisampled_render_target {
                wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: msaa_target,
                    load_op: wgpu::LoadOp::Clear,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color::WHITE,
                    resolve_target: Some(&frame.view),
                }
            } else {
                wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    load_op: wgpu::LoadOp::Clear,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color::WHITE,
                    resolve_target: None,
                }
            };

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[color_attachment],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: depth_texture_view.as_ref().unwrap(),
                    depth_load_op: wgpu::LoadOp::Clear,
                    depth_store_op: wgpu::StoreOp::Store,
                    stencil_load_op: wgpu::LoadOp::Clear,
                    stencil_store_op: wgpu::StoreOp::Store,
                    clear_depth: 0.0,
                    clear_stencil: 0,
                }),
            });

            if scene.show_wireframe {
                pass.set_pipeline(&wireframe_render_pipeline);
            } else {
                pass.set_pipeline(&render_pipeline);
            }
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_index_buffer(&ibo, 0, 0);
            pass.set_vertex_buffer(0, &vbo, 0, 0);

            pass.draw_indexed(fill_range.clone(), 0, 0..(num_instances as u32));
            pass.draw_indexed(stroke_range.clone(), 0, 0..1);

            if scene.draw_background {
                pass.set_pipeline(&bg_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_index_buffer(&bg_ibo, 0, 0);
                pass.set_vertex_buffer(0, &bg_vbo, 0, 0);

                pass.draw_indexed(0..6, 0, 0..1);
            }
        }

        queue.submit(&[encoder.finish()]);

        frame_count += 1.0;
    });
}

/// This vertex constructor forwards the positions and normals provided by the
/// tessellators and add a shape id.
pub struct WithId(pub i32);

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
    window_size: LogicalSize,
    size_changed: bool,
}

fn update_inputs(
    event: Event<()>,
    control_flow: &mut ControlFlow,
    scene: &mut SceneParams,
) -> bool {
    match event {
        Event::EventsCleared => {
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
            event: WindowEvent::CursorMoved { position, .. },
            ..
        } => {
            scene.cursor_position = (position.x as f32, position.y as f32);
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
        },
        _evt => {
            //println!("{:?}", _evt);
        }
    }
    //println!(" -- zoom: {}, scroll: {:?}", scene.target_zoom, scene.target_scroll);

    scene.zoom += (scene.target_zoom - scene.zoom) / 3.0;
    scene.scroll = scene.scroll + (scene.target_scroll - scene.scroll) / 3.0;
    scene.stroke_width =
        scene.stroke_width + (scene.target_stroke_width - scene.stroke_width) / 5.0;

    *control_flow = ControlFlow::Poll;

    return true;
}
