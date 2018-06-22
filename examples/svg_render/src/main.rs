extern crate clap;
#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate lyon;
extern crate usvg;

mod path_convert;
mod stroke_convert;
mod render;

use clap::*;
use gfx::traits::{Device, FactoryExt};
use glutin::GlContext;
use lyon::tessellation::geometry_builder::{BuffersBuilder, VertexBuffers};
use lyon::tessellation::{FillOptions, FillTessellator, StrokeTessellator};
pub use lyon::geom::euclid::Transform3D;
use usvg::Color;
use usvg::prelude::*;

use path_convert::convert_path;
use stroke_convert::convert_stroke;
use render::{
    fill_pipeline, ColorFormat, DepthFormat, Scene, VertexCtor, Transform, Primitive
};
use std::f64::NAN;

const WINDOW_SIZE: f32 = 800.0;

pub const FALLBACK_COLOR: Color = Color {
    red: 0,
    green: 0,
    blue: 0,
};

fn main() {
    let app = App::new("Lyon svg_render example")
        .version("0.1")
        .arg(Arg::with_name("MSAA")
            .long("msaa")
            .short("m")
            .help("Sets MSAA sample count (integer)")
            .value_name("SAMPLES")
            .takes_value(true)
            .required(false))
        .arg(Arg::with_name("INPUT")
             .help("SVG or SVGZ file")
             .value_name("INPUT")
             .takes_value(true)
             .required(true))
        .get_matches();

    let msaa = if let Some(msaa) = app.value_of("MSAA") {
        match msaa.parse::<u16>() {
            Ok(n) => Some(n),
            Err(_) => {
                println!("ERROR: `{}` is not a number", msaa);
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    let filename = app.value_of("INPUT").unwrap();

    let mut fill_tess = FillTessellator::new();
    let mut stroke_tess = StrokeTessellator::new();
    let mut mesh = VertexBuffers::new();


    let opt = usvg::Options::default();
    let rtree = usvg::Tree::from_file(&filename, &opt).unwrap();
    let mut transforms = Vec::new();
    let mut primitives = Vec::new();

    let mut prev_transform = usvg::Transform {
        a: NAN, b: NAN,
        c: NAN, d: NAN,
        e: NAN, f: NAN,
    };
    let view_box = rtree.svg_node().view_box;
    for node in rtree.root().descendants() {
        if let usvg::NodeKind::Path(ref p) = *node.borrow() {
            let t = node.transform();
            if t != prev_transform {
                println!(" push transform");
                transforms.push(Transform {
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

                primitives.push(Primitive::new(
                    transform_idx,
                    color,
                    fill.opacity.value() as f32
                ));

                fill_tess.tessellate_path(
                    convert_path(p).path_iter(),
                    &FillOptions::tolerance(0.01),
                    &mut BuffersBuilder::new(
                        &mut mesh,
                        VertexCtor { prim_id: primitives.len() as u32 - 1 }
                    ),
                ).expect("Error during tesselation!");
            }

            if let Some(ref stroke) = p.stroke {
                let (stroke_color, stroke_opts) = convert_stroke(stroke);
                primitives.push(Primitive::new(
                    transform_idx,
                    stroke_color,
                    stroke.opacity.value() as f32
                ));
                let _ = stroke_tess.tessellate_path(
                    convert_path(p).path_iter(),
                    &stroke_opts.with_tolerance(0.01),
                    &mut BuffersBuilder::new(
                        &mut mesh,
                        VertexCtor { prim_id: primitives.len() as u32 - 1 },
                    ),
                );
            }
        }
    }

    println!(
        "Finished tesselation: {} vertices, {} indices",
        mesh.vertices.len(),
        mesh.indices.len()
    );
    println!("Use arrow keys to pan, pageup and pagedown to zoom.");

    // get svg view box parameters
    let vb_width = view_box.rect.size().width as f32;
    let vb_height = view_box.rect.size().height as f32;
    let scale = vb_width / vb_height;

    // set window scale
    let (width, height) = if scale < 1.0 {
        (WINDOW_SIZE, WINDOW_SIZE * scale)
    } else {
        (WINDOW_SIZE, WINDOW_SIZE / scale)
    };

    // init the scene object
    // use the viewBox, if available, to set the initial zoom and pan
    let pan = [vb_width / -2.0, vb_height / -2.0];
    let zoom = 2.0 / f32::max(vb_width, vb_height);
    let mut scene = Scene::new(zoom, pan, width / height);

    // set up event processing and rendering
    let mut event_loop = glutin::EventsLoop::new();
    let glutin_builder = glutin::WindowBuilder::new()
        .with_dimensions(width as u32, height as u32)
        .with_decorations(true)
        .with_title("SVG Renderer");

    let msaa_samples = msaa.unwrap_or(0);

    let context = glutin::ContextBuilder::new()
        .with_multisampling(msaa_samples)
        .with_vsync(true);

    let (window, mut device, mut factory, mut main_fbo, mut main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(glutin_builder, context, &event_loop);

    let shader = factory.link_program(
        render::VERTEX_SHADER.as_bytes(),
        render::FRAGMENT_SHADER.as_bytes(),
    ).unwrap();

    let mut rasterizer_state = gfx::state::Rasterizer::new_fill();
    if msaa.is_some() {
        rasterizer_state.samples = Some(gfx::state::MultiSample);
    }
    let pso = factory.create_pipeline_from_program(
        &shader,
        gfx::Primitive::TriangleList,
        rasterizer_state,
        fill_pipeline::new(),
    ).unwrap();


    let mut rasterizer_state = gfx::state::Rasterizer::new_fill();
    rasterizer_state.method = gfx::state::RasterMethod::Line(1);
    let wireframe_pso = factory.create_pipeline_from_program(
        &shader,
        gfx::Primitive::TriangleList,
        rasterizer_state,
        fill_pipeline::new(),
    ).unwrap();

    let (vbo, ibo) = factory.create_vertex_buffer_with_slice(&mesh.vertices[..], &mesh.indices[..]);

    let mut cmd_queue: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let constants = factory.create_constant_buffer(1);
    let gpu_transforms = factory.create_constant_buffer(render::MAX_TRANSFORMS);
    let gpu_primtives = factory.create_constant_buffer(render::MAX_PRIMITIVES);
    cmd_queue.update_buffer(&gpu_transforms, &transforms[..], 0).unwrap();
    cmd_queue.update_buffer(&gpu_primtives, &primitives[..], 0).unwrap();

    loop {
        if !update_inputs(&mut scene, &mut event_loop) {
            break;
        }

        gfx_window_glutin::update_views(&window, &mut main_fbo, &mut main_depth);

        cmd_queue.clear(&main_fbo.clone(), [1.0, 1.0, 1.0, 1.0]);

        cmd_queue.update_constant_buffer(&constants, &scene.into());
        cmd_queue.draw(
            &ibo,
            if scene.wireframe { &wireframe_pso } else { &pso },
            &fill_pipeline::Data {
                vbo: vbo.clone(),
                out_color: main_fbo.clone(),
                constants: constants.clone(),
                transforms: gpu_transforms.clone(),
                primitives: gpu_primtives.clone(),
            },
        );
        cmd_queue.flush(&mut device);

        window.swap_buffers().unwrap();

        device.cleanup();
    }
}



fn update_inputs(scene: &mut Scene, event_loop: &mut glutin::EventsLoop) -> bool {
    use glutin::Event;
    use glutin::VirtualKeyCode;
    use glutin::ElementState::Pressed;

    let mut status = true;

    event_loop.poll_events(|event| match event {
        Event::WindowEvent {
            event: glutin::WindowEvent::Closed,
            ..
        } => {
            status = false;
        }
        Event::WindowEvent {
            event: glutin::WindowEvent::Resized(w, h),
            ..
        } => {
            scene.aspect_ratio = w as f32 / h as f32;
        }
        Event::WindowEvent {
            event:
                glutin::WindowEvent::KeyboardInput {
                    input:
                        glutin::KeyboardInput {
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
                    scene.zoom *= 0.8;
                }
                VirtualKeyCode::PageUp => {
                    scene.zoom *= 1.2;
                }
                VirtualKeyCode::Left => {
                    scene.pan[0] += 0.2 / scene.zoom;
                }
                VirtualKeyCode::Right => {
                    scene.pan[0] -= 0.2 / scene.zoom;
                }
                VirtualKeyCode::Up => {
                    scene.pan[1] += 0.2 / scene.zoom;
                }
                VirtualKeyCode::Down => {
                    scene.pan[1] -= 0.2 / scene.zoom;
                }
                VirtualKeyCode::W => {
                    scene.wireframe = !scene.wireframe;
                }
                _key => {}
            };
        }
        _ => {}
    });

    status
}
