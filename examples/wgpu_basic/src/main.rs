use lyon::extra::rust_logo::build_logo_path;
use lyon::path::builder::*;
use lyon::path::Path;
use lyon::math::*;
use lyon::tessellation::geometry_builder::{VertexConstructor, VertexBuffers, BuffersBuilder};
use lyon::tessellation::{FillTessellator, FillOptions};
use lyon::tessellation;

use wgpu::winit::{ElementState, Event, EventsLoop, KeyboardInput, VirtualKeyCode, Window, WindowEvent};

#[derive(Copy, Clone, Debug)]
struct GpuFillVertex {
    position: [f32; 2],
}

// A very simple vertex constructor that only outputs the vertex position
struct VertexCtor;
impl VertexConstructor<tessellation::FillVertex, GpuFillVertex> for VertexCtor {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> GpuFillVertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());
        GpuFillVertex {
            // (ugly hack) tweak the vertext position so that the logo fits roughly
            // within the (-1.0, 1.0) range.
            position: (vertex.position * 0.0145 - vector(1.0, 1.0)).to_array(),
        }
    }
}

fn main() {
    // Build a Path for the rust logo.
    let mut builder = SvgPathBuilder::new(Path::builder());
    build_logo_path(&mut builder);
    let path = builder.build();

    let mut tessellator = FillTessellator::new();

    let mut mesh: VertexBuffers<GpuFillVertex, u16> = VertexBuffers::new();

    tessellator.tessellate_path(
        &path,
        &FillOptions::tolerance(0.01),
        &mut BuffersBuilder::new(&mut mesh, VertexCtor),
    ).unwrap();

    println!(" -- fill: {} vertices {} indices", mesh.vertices.len(), mesh.indices.len());

    let instance = wgpu::Instance::new();
    let adapter = instance.get_adapter(&wgpu::AdapterDescriptor {
        power_preference: wgpu::PowerPreference::LowPower,
    });
    let mut device = adapter.create_device(&wgpu::DeviceDescriptor {
        extensions: wgpu::Extensions {
            anisotropic_filtering: false,
        },
    });

    let vbo = device
        .create_buffer_mapped(mesh.vertices.len(), wgpu::BufferUsageFlags::VERTEX)
        .fill_from_slice(&mesh.vertices);

    let ibo = device
        .create_buffer_mapped(mesh.indices.len(), wgpu::BufferUsageFlags::INDEX)
        .fill_from_slice(&mesh.indices);

    let vs_bytes = include_bytes!("./../shaders/basic.vert.spv");
    let fs_bytes = include_bytes!("./../shaders/basic.frag.spv");
    let vs_module = device.create_shader_module(vs_bytes);
    let fs_module = device.create_shader_module(fs_bytes);

    let bind_group_layout = device.create_bind_group_layout(
        &wgpu::BindGroupLayoutDescriptor { bindings: &[] }
    );
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        bindings: &[],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&bind_group_layout],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: &pipeline_layout,
        vertex_stage: wgpu::PipelineStageDescriptor {
            module: &vs_module,
            entry_point: "main",
        },
        fragment_stage: wgpu::PipelineStageDescriptor {
            module: &fs_module,
            entry_point: "main",
        },
        rasterization_state: wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
        },
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        color_states: &[wgpu::ColorStateDescriptor {
            format: wgpu::TextureFormat::Bgra8Unorm,
            color: wgpu::BlendDescriptor::REPLACE,
            alpha: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWriteFlags::ALL,
        }],
        depth_stencil_state: None,
        index_format: wgpu::IndexFormat::Uint16,
        vertex_buffers: &[
            wgpu::VertexBufferDescriptor {
                stride: 8,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttributeDescriptor {
                        offset: 0,
                        format: wgpu::VertexFormat::Float2,
                        attribute_index: 0,
                    },
                ],
            },
        ],
        sample_count: 1,
    });

    let mut events_loop = EventsLoop::new();
    let window = Window::new(&events_loop).unwrap();
    let size = window
        .get_inner_size()
        .unwrap()
        .to_physical(window.get_hidpi_factor());

    let window_surface = instance.create_surface(&window);
    let mut swap_chain = device.create_swap_chain(
        &window_surface,
        &wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsageFlags::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width: size.width.round() as u32,
            height: size.height.round() as u32,
        },
    );

    loop {
        if !update_inputs(&mut events_loop) {
            break;
        }

        let frame = swap_chain.get_next_texture();
        let mut encoder = device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { todo: 0 }
        );

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    load_op: wgpu::LoadOp::Clear,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color::WHITE,
                }],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&render_pipeline);
            rpass.set_bind_group(0, &bind_group);
            rpass.set_index_buffer(&ibo, 0);
            rpass.set_vertex_buffers(&[(&vbo, 0)]);
            rpass.draw_indexed(0..(mesh.indices.len() as u32), 0, 0..1);
        }

        device.get_queue().submit(&[encoder.finish()]);
    }
}

fn update_inputs(event_loop: &mut EventsLoop) -> bool {
    let mut status = true;

    event_loop.poll_events(|event| {
        match event {
            Event::WindowEvent { event: WindowEvent::Destroyed, .. } => {
                println!("Window Closed!");
                status = false;
            },
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput {
                    input: KeyboardInput { state: ElementState::Pressed, virtual_keycode: Some(VirtualKeyCode::Escape), .. },
                    ..
                },
                ..
            } => {
                println!("Closing");
                status = false;
            },
            _ => {}
        }
    });

    status
}
