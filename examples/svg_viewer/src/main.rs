#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate gfx_device_gl;
extern crate glutin;
extern crate lyon;
extern crate clap;
extern crate svgparser;

// use lyon::extra::rust_logo::build_logo_path;
// use lyon::path_builder::*;
use lyon::math::*;
use lyon::tessellation::geometry_builder::{VertexConstructor, VertexBuffers, BuffersBuilder};
use lyon::tessellation::{FillTessellator, FillOptions};
use lyon::tessellation;
use lyon::path::Path;
use lyon::svg::parser::build_path;
// use lyon::lyon_path_builder::SvgBuilder;

use gfx::traits::{Device, FactoryExt};
use glutin::GlContext;

use clap::{Arg, App};

use std::fs::File;
use std::io::{Read};
// use std::borrow::Borrow;

use svgparser::svg::{Tokenizer, Token};
use svgparser::{Tokenize, ElementId, AttributeId};

type ColorFormat = gfx::format::Rgba8;
type DepthFormat = gfx::format::DepthStencil;

gfx_defines!{
    vertex GpuFillVertex {
        position: [f32; 2] = "a_position",
        color: [f32; 4] = "a_color",
    }

    constant Globals {
        zoom: [f32; 2] = "u_zoom",
        pan: [f32; 2] = "u_pan",
    }

    pipeline fill_pipeline {
        vbo: gfx::VertexBuffer<GpuFillVertex> = (),
        out_color: gfx::RenderTarget<ColorFormat> = "out_color",
        constants: gfx::ConstantBuffer<Globals> = "Globals",
    }
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
            position: (vertex.position * 0.0145 - vec2(1.0, 1.0)).to_array(),
            color: [0.2, 0.2, 0.2, 1.0]
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Scene {
    zoom: f32,
    pan: [f32; 2]
}

// extract the relevant globals from the scene struct
impl From<Scene> for Globals {
    fn from(scene: Scene) -> Self {
        Globals {
            zoom: [scene.zoom, scene.zoom],
            pan: scene.pan
        }
    }
}

fn main() {
    // Parse CLI arguments
    let cli_args = App::new("Lyon CLI Renderer")
                          .arg(Arg::with_name("input")
                               .index(1)
                               .value_name("FILE")
                               .help("SVG file to render")
                               .takes_value(true))
                          .get_matches();

    let filepath = cli_args.value_of("input").expect("Missing input file");

    // read the SVG file into a string
    let mut input_buffer = String::new();
    let mut file = File::open(filepath)
        .expect("Error opening input file!");
    file.read_to_string(&mut input_buffer)
        .expect("Error reading input file!");

    // iterate over the SVG paths and tesselate each one (i.e. tokens)
    let mut last_tag = None;
    // let mut svg_paths = Vec::new();
    let mut tessellator = FillTessellator::new();
    let mut mesh = VertexBuffers::new();
    let mut tokens = Tokenizer::from_str(&input_buffer).tokens();
    for token in &mut tokens {
        match token {
            // start of new tag
            Token::SvgElementStart(tag) => {
                last_tag = Some(tag);
            }
            // start of an SVG attribute
            Token::SvgAttribute(name, value) => {
                // check if we're dealing with a path definition
                if last_tag == Some(ElementId::Path) && name == AttributeId::D {
                    // parse/build the path
                    let path = build_path(Path::builder().with_svg(), value.slice())
                        .expect("Error parsing SVG!");

                    // tesselate and add to the shared mesh
                    tessellator.tessellate_path(
                        path.path_iter(),
                        &FillOptions::tolerance(0.01),
                        &mut BuffersBuilder::new(&mut mesh, VertexCtor),
                    ).expect("Error during tesselation");
                }
            }
            _ => {}
        }
    }

    println!("Finished tesselation: {} vertices, {} indices", mesh.vertices.len(), mesh.indices.len());
    println!("Use arrow keys to pan, square brackes to zoom.");

    // init the scene object
    let mut scene = Scene {
        // coordinate shift factor (applied before scale)
        pan: [0.0, 0.0],
        // coordinate multiplication factor.  Zero means infinitely zoomed out.
        zoom: 1.0
    };

    // Initialize glutin and gfx-rs (refer to gfx-rs examples for more details).
    let mut events_loop = glutin::EventsLoop::new();

    let glutin_builder = glutin::WindowBuilder::new()
        .with_dimensions(700, 700)
        .with_decorations(true)
        .with_title("SVG Viewer".to_string());

    let context = glutin::ContextBuilder::new().with_vsync(true);

    let (window, mut device, mut factory, mut main_fbo, mut main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(glutin_builder, context, &events_loop);

    let shader = factory.link_program(
        VERTEX_SHADER.as_bytes(),
        FRAGMENT_SHADER.as_bytes()
    ).unwrap();

    let pso = factory.create_pipeline_from_program(
        &shader,
        gfx::Primitive::TriangleList,
        gfx::state::Rasterizer::new_fill(),
        fill_pipeline::new(),
    ).unwrap();

    let (vbo, ibo) = factory.create_vertex_buffer_with_slice(
        &mesh.vertices[..],
        &mesh.indices[..]
    );

    let mut cmd_queue: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let constants = factory.create_constant_buffer(1);

    loop {
        if !update_inputs(&mut scene, &mut events_loop) {
            break;
        }

        gfx_window_glutin::update_views(&window, &mut main_fbo, &mut main_depth);

        cmd_queue.clear(&main_fbo.clone(), [1.0, 1.0, 1.0, 1.0]);

        cmd_queue.update_constant_buffer(&constants, &scene.into());
        cmd_queue.draw(
            &ibo,
            &pso,
            &fill_pipeline::Data {
                vbo: vbo.clone(),
                out_color: main_fbo.clone(),
                constants: constants.clone()
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

    event_loop.poll_events(|event| {
        match event {
            Event::WindowEvent {event: glutin::WindowEvent::Closed, ..} => {
                println!("Window Closed!");
                status = false;
            },
            Event::WindowEvent {event: glutin::WindowEvent::KeyboardInput {input: glutin::KeyboardInput {state: Pressed, virtual_keycode: Some(key), ..}, ..}, ..} => {
                match key {
                    VirtualKeyCode::Escape => {
                        println!("Closing");
                        status = false;
                    }
                    VirtualKeyCode::LBracket => {
                        scene.zoom *= 0.8;
                    }
                    VirtualKeyCode::RBracket => {
                        scene.zoom *= 1.2;
                    }
                    VirtualKeyCode::Left => {
                        scene.pan[0] -= 0.2 / scene.zoom;
                    }
                    VirtualKeyCode::Right => {
                        scene.pan[0] += 0.2 / scene.zoom;
                    }
                    VirtualKeyCode::Up => {
                        scene.pan[1] -= 0.2 / scene.zoom;
                    }
                    VirtualKeyCode::Down => {
                        scene.pan[1] += 0.2 / scene.zoom;
                    }
                    _key => {}
                }
            },
            _ => {}
        }
    });

    status
}


pub static VERTEX_SHADER: &'static str = "
    #version 140
    #line 266

    uniform Globals {
        vec2 u_zoom;
        vec2 u_pan;
    };

    in vec2 a_position;
    in vec4 a_color;

    out vec4 v_color;

    void main() {
        gl_Position = vec4((a_position + u_pan) * u_zoom, 0.0, 1.0);
        gl_Position.y *= -1.0;
        v_color = a_color;
    }
";

// The fragment shader is dead simple. It just applies the color computed in the vertex shader.
// A more advanced renderer would probably compute texture coordinates in the vertex shader and
// sample the color from a texture here.
pub static FRAGMENT_SHADER: &'static str = "
    #version 140
    in vec4 v_color;
    out vec4 out_color;

    void main() {
        out_color = v_color;
    }
";
