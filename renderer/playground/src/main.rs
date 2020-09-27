use glue::units::*;
use glue::*;
use gpu::*;
use pipe::*;
use cache::*;
use futures::executor::block_on;

use winit::event::{WindowEvent, Event};
use winit::event_loop::{EventLoop, ControlFlow};

use lyon::tessellation::*;
use lyon::extra::rust_logo::build_logo_path;

struct Image {
    data: Vec<u8>,
    descriptor: SourceTexture,
}

struct Scene {
    instances: Vec<GpuInstance>,
    rects: Vec<[f32; 4]>,
    transforms: Vec<GpuTransform2D>,
    mask_texture: Image,
    atlas_texture: Image,
    image_sources: Vec<GpuImageSource>,
}

impl Scene {
    fn new() -> Self {
        let mask_texture = mask_tex();
        let atlas_texture = color_tex();
        Scene {
            instances: vec![
                Instance {
                    rect_id: 0,
                    primitive_id: 0,
                    transform_id: 0,
                    src_color_id: 4,
                    src_mask_id: 0,
                    user_data: 10,
                    z: 1,
                }.pack(),
                Instance {
                    rect_id: 1,
                    primitive_id: 1,
                    transform_id: 0,
                    src_color_id: 5,
                    src_mask_id: 0,
                    user_data: 20,
                    z: 2,
                }.pack(),
                Instance {
                    rect_id: 2,
                    primitive_id: 2,
                    transform_id: 0,
                    src_color_id: 6,
                    src_mask_id: 3,
                    user_data: 30,
                    z: 3,
                }.pack(),
                Instance {
                    rect_id: 3,
                    primitive_id: 3,
                    transform_id: 0,
                    src_color_id: 7,
                    src_mask_id: 3,
                    user_data: 40,
                    z: 4,
                }.pack(),
            ],
            rects: vec![
                [0.0, 0.0, 200.0, 200.0],
                [300.0, 0.0, 500.0, 300.0],
                [500.0, 0.0,  700.0, 200.0],
                [0.0, 300.0, 400.0, 700.0],
            ],
            transforms: vec![
                GpuTransform2D::identity(),
            ],
            image_sources: vec![
                // clip sources
                mask_texture.descriptor.pixel_src(point2(0, 0)),
                mask_texture.descriptor.pixel_src(point2(1, 0)),
                mask_texture.descriptor.sub_image_src(&Box2D { min: point2(16, 16), max: point2(32, 32), }),
                mask_texture.descriptor.sub_image_src(&Box2D { min: point2(32, 32), max: point2(64, 64), }),
                // color sources
                atlas_texture.descriptor.pixel_src(point2(4, 0)),
                atlas_texture.descriptor.pixel_src(point2(5, 0)),
                atlas_texture.descriptor.pixel_src(point2(6, 0)),
                atlas_texture.descriptor.pixel_src(point2(7, 0)),
            ],
            mask_texture,
            atlas_texture,
        }
    }
}

fn mask_tex() -> Image {
    let descriptor = SourceTexture {
        size: U8AlphaMask::SIZE,
        format: wgpu::TextureFormat::R8Unorm,
    };

    let w = descriptor.size.width;
    let h = descriptor.size.height;

    let mut data = Vec::with_capacity((w * h) as usize);

    data.push(255);
    data.push(0);
    for _ in 2..w {
        data.push(0);
    }

    for _ in 1..16 {
        for x in 0..256 {
            data.push(x as u8);
        }

        for _ in 256..w {
            data.push(255);
        }
    }

    for y in 16..h {
        for x in 0..w {
            let checker =  if (y % 2 == 0) ^ (x % 2 == 0) {
                255
            } else {
                0
            };
            data.push(checker);
        }
    }

    Image { data, descriptor }
}

fn color_tex() -> Image {
    let descriptor = SourceTexture {
        size: ColorAtlasTexture::SIZE,
        format: wgpu::TextureFormat::Rgba8Unorm,
    };

    let width = descriptor.size.width;
    let height = descriptor.size.height;


    let mut data = Vec::with_capacity((width * height) as usize);

    let add_pixel = &mut|r, g, b, a| {
        data.push(r);
        data.push(g);
        data.push(b);
        data.push(a);
    };

    for _ in 0..5 {
        add_pixel(0, 0, 0, 0);
        add_pixel(0, 0, 0, 255);
        add_pixel(255, 255, 255, 255);
        add_pixel(255, 255, 255, 0);
        add_pixel(255, 0, 0, 255);
        add_pixel(0, 255, 0, 255);
        add_pixel(0, 0, 255, 255);
        add_pixel(255, 255, 0, 255);
        add_pixel(0, 255, 255, 255);
        add_pixel(255, 0, 255, 255);
        for _ in 10 .. width {
            add_pixel(255, 0, 255, 255);
        }
    }


    for _ in (5 * width) .. (width * height) {
        add_pixel(255, 255, 0, 255);
    }

    Image { data, descriptor }
}

fn mesh() -> (VertexBuffers<GpuMeshVertex, u32>, Vec<GpuSubMesh>, Vec<GpuInstance>) {
    use lyon::tessellation::geometry_builder::*;
    use lyon::path::builder::Build;

    let mut geometry: VertexBuffers<GpuMeshVertex, u32> = VertexBuffers::new();

    let mut fill_tess = FillTessellator::new();

    let mut builder = lyon::path::Path::builder().with_svg();
    build_logo_path(&mut builder);
    let path = builder.build();

    let mut sub_meshes = Vec::new();
    let mut mesh_instances = Vec::new();
    fill_tess.tessellate_path(
        &path,
        &FillOptions::tolerance(0.01).with_fill_rule(FillRule::NonZero),
        &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
            let p = vertex.position();
            GpuMeshVertex {
                x: p.x,
                y: p.y,
                sub_mesh: 0,
            }
        }),
    ).unwrap();

    sub_meshes.push(SubMeshData {
        transform_id: 0,
        src_color_id: 0,
        dest_color_rect: Box2D { min: point2(0.0, 0.0), max: point2(100.0, 100.0)  },
        opacity: 1.0,
    }.pack());
    mesh_instances.push(MeshInstance {
        sub_mesh_offset: 0,
        transform_id: 0,
        user_data: 0,
        z: 1,
    }.pack());

    (geometry, sub_meshes, mesh_instances)
}

fn main() {
    //std::env::set_var("WINIT_UNIX_BACKEND", "x11");

    let event_loop = EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();

    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let window_surface = unsafe { instance.create_surface(&window) };

    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::Default,
        compatible_surface: Some(&window_surface),
    })).unwrap();

    let (device, queue) = block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            features: wgpu::Features::default(),
            limits: wgpu::Limits::default(),
            shader_validation: true,
        },
        None,
    ))
    .unwrap();

    let size = window.inner_size();
    let mut physical_width = size.width as u32;
    let mut physical_height = size.height as u32;
    println!("window size {:?}", size);

    let swap_chain_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: DEFAULT_COLOR_FORMAT,
        width: physical_width,
    height: physical_height,
        present_mode: wgpu::PresentMode::Fifo,
    };

    let mut swap_chain = device.create_swap_chain(
        &window_surface,
        &swap_chain_desc,
    );

    let mut renderer = Renderer::new(&device, &queue);
    let quad_renderer = QuadRenderer::new(&device, &queue, &renderer.common, &mut renderer.resources);
    let mesh_renderer = MeshRenderer::new(&device, &queue, &renderer.common, &mut renderer.resources);

    let mut common_bind_group_cache = BindGroupCache::new(renderer.common.base_bind_group_layout);
    let mut textures_bind_group_cache = BindGroupCache::new(renderer.common.textures_bind_group_layout);
    let mut mesh_bind_group_cache = BindGroupCache::new(mesh_renderer.mesh_bind_group_layout);

    let cpu = Scene::new();

    let (cpu_geometry, cpu_sub_meshes, cpu_mesh_instances) = mesh();

    let cpu_globals = &[
        GpuGlobals {
            resolution: [
                physical_width as f32,
                physical_height as f32,
            ],
        }
    ];

    let globals = renderer.common.globals.buffer_id(ResourceIndex(0, 0));
    let rects = renderer.common.rects.buffer_id(ResourceIndex(0, 0));
    let transforms = renderer.common.transforms.buffer_id(ResourceIndex(0, 0));
    let image_sources = renderer.common.image_sources.buffer_id(ResourceIndex(0, 0));
    let instances = renderer.common.instances.buffer_id(ResourceIndex(0, 0));

    renderer.resources.allocate_buffer(&device, globals);
    renderer.resources.allocate_buffer(&device, rects);
    renderer.resources.allocate_buffer(&device, transforms);
    renderer.resources.allocate_buffer(&device, image_sources);
    renderer.resources.allocate_buffer(&device, instances);

    let mask = renderer.common.mask_texture_kind.texture_id(ResourceIndex(0, 0));
    let color_atlas = renderer.common.color_atlas_texture_kind.texture_id(ResourceIndex(0, 0));

    renderer.resources.allocate_texture(&device, mask);
    renderer.resources.allocate_texture(&device, color_atlas);

    let base_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("base bind group"),
        layout: &renderer.resources[renderer.common.base_bind_group_layout],
        entries: &[
            GpuGlobals::bind_group_entry(&renderer.resources[globals]),
            GpuPrimitiveRects::bind_group_entry(&renderer.resources[rects]),
            GpuTransform2D::bind_group_entry(&renderer.resources[transforms]),
            GpuImageSource::bind_group_entry(&renderer.resources[image_sources]),
        ],
    });

    let textures_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("textures bind group"),
        layout: &renderer.resources[renderer.common.textures_bind_group_layout],
        entries: &[
            ColorAtlasTexture::bind_group_entry(&renderer.resources[color_atlas].view),
            U8AlphaMask::bind_group_entry(&renderer.resources[mask].view),
            DefaultSampler::bind_group_entry(&renderer.common.default_sampler),
        ],
    });

    let base_bind_group_id = renderer.common.base_bind_group_layout.bind_group_id(ResourceIndex(0, 0)); // TODO
    renderer.resources.add_bind_group(base_bind_group_id, base_bind_group);

    let textures_bind_group_id = renderer.common.textures_bind_group_layout.bind_group_id(ResourceIndex(1, 0));
    renderer.resources.add_bind_group(textures_bind_group_id, textures_bind_group);

    let mesh_vertex_buffer = mesh_renderer.vbo_kind.buffer_id(ResourceIndex(0, 0));
    let mesh_index_buffer = mesh_renderer.ibo_kind.buffer_id(ResourceIndex(0, 0));
    let sub_meshes = mesh_renderer.sub_meshes.buffer_id(ResourceIndex(0, 0));
    renderer.resources.allocate_buffer(&device, mesh_vertex_buffer);
    renderer.resources.allocate_buffer(&device, mesh_index_buffer);
    renderer.resources.allocate_buffer(&device, sub_meshes);

    let mesh_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("mesh bind group"),
        layout: &renderer.resources[mesh_renderer.mesh_bind_group_layout],
        entries: &[
            GpuSubMesh::bind_group_entry(&renderer.resources[sub_meshes]),
        ],
    });
    let mesh_bind_group_id = mesh_renderer.mesh_bind_group_layout.bind_group_id(ResourceIndex(1, 0));
    renderer.resources.add_bind_group(mesh_bind_group_id, mesh_bind_group);

    queue.write_buffer(&renderer.resources[instances], 0, as_bytes(&cpu.instances));
    queue.write_buffer(&renderer.resources[globals], 0, as_bytes(cpu_globals));
    queue.write_buffer(&renderer.resources[transforms], 0, as_bytes(&cpu.transforms));
    queue.write_buffer(&renderer.resources[rects], 0, as_bytes(&cpu.rects));
    queue.write_buffer(&renderer.resources[image_sources], 0, as_bytes(&cpu.image_sources));

    queue.write_buffer(&renderer.resources[instances], as_bytes(&cpu.instances).len() as u64, as_bytes(&cpu_mesh_instances));
    queue.write_buffer(&renderer.resources[mesh_vertex_buffer], 0, as_bytes(&cpu_geometry.vertices));
    queue.write_buffer(&renderer.resources[mesh_index_buffer], 0, as_bytes(&cpu_geometry.indices));
    queue.write_buffer(&renderer.resources[sub_meshes], 0, as_bytes(&cpu_sub_meshes));

    let mut depth_buffer = DepthBuffer::new(
        physical_width,
        physical_height,
        &device,
    );

    queue.write_texture(
        wgpu::TextureCopyView {
            texture: &renderer.resources[mask].texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        &cpu.mask_texture.data,
        wgpu::TextureDataLayout {
            offset: 0,
            bytes_per_row: cpu.mask_texture.descriptor.size.width,
            rows_per_image: cpu.mask_texture.descriptor.size.height,
        },
        U8AlphaMask::SIZE,
    );

    queue.write_texture(
        wgpu::TextureCopyView {
            texture: &renderer.resources[color_atlas].texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        as_bytes(&cpu.atlas_texture.data),
        wgpu::TextureDataLayout {
            offset: 0,
            bytes_per_row: cpu.atlas_texture.descriptor.size.width * 4,
            rows_per_image: cpu.atlas_texture.descriptor.size.height,
        },
        ColorAtlasTexture::SIZE,
    );

    let mut framestamp = FrameStamp(0);

    event_loop.run(move|event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event: WindowEvent::Destroyed, .. }
            | Event::WindowEvent { event: WindowEvent::CloseRequested, .. }
            => {
                *control_flow = ControlFlow::Exit;
            },
            Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                let swap_chain_desc = wgpu::SwapChainDescriptor {
                    usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                    format: DEFAULT_COLOR_FORMAT,
                    width: size.width as u32,
                    height: size.height as u32,
                    present_mode: wgpu::PresentMode::Fifo,
                };

                physical_width = size.width as u32;
                physical_height = size.height as u32;

                swap_chain = device.create_swap_chain(&window_surface, &swap_chain_desc);
            }
            _ => {}
        }

        framestamp.advance();

        if depth_buffer.width != physical_width || depth_buffer.height != physical_height {
            depth_buffer = DepthBuffer::new(
                physical_width,
                physical_height,
                &device,
            );
        };

        let mut state = DrawState::new();

        let frame = swap_chain.get_current_frame().unwrap();
        let mut encoder = device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some(&"drawing commands") }
        );

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.output.view,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.2, g: 0.2, b: 0.2, a: 1.0 }),
                        store: true,
                    },
                    resolve_target: None,
                }], 
                depth_stencil_attachment: None,
                //depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                //    attachment: &depth_buffer.view,
                //    depth_ops: Some(wgpu::Operations {
                //        load: wgpu::LoadOp::Clear(0.0),
                //        store: true,
                //    }),
                //    stencil_ops: None,
                //}),
            });

            let batch = Batch {
                key: BatchKey {
                    pipeline: quad_renderer.alpha_pipeline_key,
                    ibo: quad_renderer.index_buffer,
                    vbos: [
                        Some(instances),
                        Some(quad_renderer.vertex_buffer),
                        None,
                        None,
                    ],
                    bind_groups: [
                        Some(base_bind_group_id),
                        Some(textures_bind_group_id),
                        None,
                        None,
                    ],
                },
                base_vertex: 0,
                index_range: 0..6,
                instance_range: 0..4,
            };

            state.submit_batch(&mut pass, &renderer.resources, &batch);

            let batch = Batch {
                key: BatchKey {
                    pipeline: mesh_renderer.alpha_pipeline_key,
                    ibo: mesh_index_buffer,
                    vbos: [
                        Some(instances),
                        Some(mesh_vertex_buffer),
                        None,
                        None,
                    ],
                    bind_groups: [
                        Some(base_bind_group_id),
                        Some(textures_bind_group_id),
                        Some(mesh_bind_group_id),
                        None,
                    ],
                },
                base_vertex: 0,
                index_range: 0..(cpu_geometry.indices.len() as u32),
                instance_range: 6..7,
            };

            state.submit_batch(&mut pass, &renderer.resources, &batch);
        }

        queue.submit(Some(encoder.finish()));
    });
}
