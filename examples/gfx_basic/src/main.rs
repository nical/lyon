#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate gfx_device_gl;
extern crate glutin;
extern crate lyon;

use lyon::extra::rust_logo::build_logo_path;
use lyon::path_builder::*;
use lyon::math::*;
use lyon::tessellation::geometry_builder::{VertexConstructor, VertexBuffers, BuffersBuilder};
use lyon::tessellation::{FillTessellator, FillOptions};
use lyon::tessellation;
use lyon::path::Path;

use gfx::traits::{Device, FactoryExt};

type ColorFormat = gfx::format::Rgba8;
type DepthFormat = gfx::format::DepthStencil;

gfx_defines!{
    vertex GpuFillVertex {
        position: [f32; 2] = "a_position",
    }

    pipeline fill_pipeline {
        vbo: gfx::VertexBuffer<GpuFillVertex> = (),
        out_color: gfx::RenderTarget<ColorFormat> = "out_color",
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
        }
    }
}

fn main() {
    // Build a Path for the rust logo.
    let mut builder = SvgPathBuilder::new(Path::builder());
    build_logo_path(&mut builder);
    let path = builder.build();

    let mut tessellator = FillTessellator::new();

    let mut mesh = VertexBuffers::new();

    tessellator.tessellate_path(
        path.path_iter(),
        &FillOptions::tolerance(0.01),
        &mut BuffersBuilder::new(&mut mesh, VertexCtor),
    ).unwrap();

    println!(" -- fill: {} vertices {} indices", mesh.vertices.len(), mesh.indices.len());

    // Initialize glutin and gfx-rs (refer to gfx-rs examples for more details).

    let glutin_builder = glutin::WindowBuilder::new()
        .with_dimensions(700, 700)
        .with_decorations(true)
        .with_title("Simple tessellation".to_string())
        .with_vsync();

    let (window, mut device, mut factory, mut main_fbo, mut main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(glutin_builder);

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

    loop {
        if !update_inputs(&window) {
            break;
        }

        gfx_window_glutin::update_views(&window, &mut main_fbo, &mut main_depth);

        cmd_queue.clear(&main_fbo.clone(), [0.8, 0.8, 0.8, 1.0]);
        cmd_queue.draw(
            &ibo,
            &pso,
            &fill_pipeline::Data {
                vbo: vbo.clone(),
                out_color: main_fbo.clone(),
            },
        );
        cmd_queue.flush(&mut device);

        window.swap_buffers().unwrap();

        device.cleanup();
    }
}

fn update_inputs(window: &glutin::Window) -> bool {
    for event in window.poll_events() {
        use glutin::Event::KeyboardInput;
        use glutin::ElementState::Pressed;
        use glutin::VirtualKeyCode;
        match event {
            glutin::Event::Closed => {
                return false;
            }
            KeyboardInput(Pressed, _, Some(key)) => {
                match key {
                    VirtualKeyCode::Escape => {
                        return false;
                    }
                    _key => {}
                }
            }
            _evt => {
                //println!("{:?}", _evt);
            }
        };
    }

    return true;
}

pub static VERTEX_SHADER: &'static str = &"
    #version 140
    #line 266

    in vec2 a_position;

    out vec4 v_color;

    void main() {
        gl_Position = vec4(a_position, 0.0, 1.0);
        gl_Position.y *= -1.0;
        v_color = vec4(0.0, 0.0, 0.0, 1.0);
    }
";

// The fragment shader is dead simple. It just applies the color computed in the vertex shader.
// A more advanced renderer would probably compute texture coordinates in the vertex shader and
// sample the color from a texture here.
pub static FRAGMENT_SHADER: &'static str = &"
    #version 140
    in vec4 v_color;
    out vec4 out_color;

    void main() {
        out_color = v_color;
    }
";
