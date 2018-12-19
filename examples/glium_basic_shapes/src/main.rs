#[macro_use]
extern crate glium;
extern crate lyon;

use glium::Surface;
use glium::glutin::dpi::LogicalSize;

use lyon::math::*;
use lyon::tessellation::geometry_builder::{VertexConstructor, VertexBuffers, BuffersBuilder};
use lyon::tessellation::{StrokeOptions, FillOptions};
use lyon::tessellation::basic_shapes::*;
use lyon::tessellation;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}

implement_vertex!(Vertex, position);


// A very simple vertex constructor that only outputs the vertex position
struct VertexCtor;
impl VertexConstructor<tessellation::FillVertex, Vertex> for VertexCtor {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> Vertex {
        Vertex { position: vertex.position.to_array(), }
    }
}
impl VertexConstructor<tessellation::StrokeVertex, Vertex> for VertexCtor {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex) -> Vertex {
        Vertex { position: vertex.position.to_array(), }
    }
}

fn main() {


    let mut mesh: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    let fill_options = FillOptions::tolerance(0.01);

    fill_triangle(
        point(3.0, 1.0),
        point(1.0, 4.0),
        point(5.0, 4.0),
        &fill_options,
        &mut BuffersBuilder::new(&mut mesh, VertexCtor),
    );

    fill_rectangle(
        &rect(6.0, 1.0, 4.0, 3.0),
        &fill_options,
        &mut BuffersBuilder::new(&mut mesh, VertexCtor),
    );

    fill_quad(
        point(11.0, 1.0),
        point(13.0, 2.0),
        point(14.0, 5.0),
        point(12.0, 4.0),
        &fill_options,
        &mut BuffersBuilder::new(&mut mesh, VertexCtor),
    );

    fill_rounded_rectangle(
        &rect(15.0, 1.0, 4.0, 3.0),
        &BorderRadii {
            top_left: 0.0,
            top_right: 0.5,
            bottom_right: 1.0,
            bottom_left: 1.5,
        },
        &fill_options,
        &mut BuffersBuilder::new(&mut mesh, VertexCtor),
    );

    fill_circle(
        point(22.0, 3.0),
        2.0,
        &fill_options,
        &mut BuffersBuilder::new(&mut mesh, VertexCtor),
    );

    fill_ellipse(
        point(27.0, 3.0),
        vector(2.2, 1.5),
        Angle::radians(0.6),
        &fill_options,
        &mut BuffersBuilder::new(&mut mesh, VertexCtor),
    );


    let stroke_options = StrokeOptions::tolerance(0.01)
        .with_line_width(0.2);

    stroke_triangle(
        point(3.0, 6.0),
        point(1.0, 9.0),
        point(5.0, 9.0),
        &stroke_options,
        &mut BuffersBuilder::new(&mut mesh, VertexCtor),
    );

    stroke_rectangle(
        &rect(6.0, 6.0, 4.0, 3.0),
        &stroke_options,
        &mut BuffersBuilder::new(&mut mesh, VertexCtor),
    );

    stroke_quad(
        point(11.0, 6.0),
        point(13.0, 7.0),
        point(14.0, 10.0),
        point(12.0, 9.0),
        &stroke_options,
        &mut BuffersBuilder::new(&mut mesh, VertexCtor),
    );

    stroke_rounded_rectangle(
        &rect(15.0, 6.0, 4.0, 3.0),
        &BorderRadii {
            top_left: 0.0,
            top_right: 0.5,
            bottom_right: 1.0,
            bottom_left: 1.5,
        },
        &stroke_options,
        &mut BuffersBuilder::new(&mut mesh, VertexCtor),
    );

    stroke_circle(
        point(22.0, 8.0),
        2.0,
        &stroke_options,
        &mut BuffersBuilder::new(&mut mesh, VertexCtor),
    );

    stroke_ellipse(
        point(27.0, 8.0),
        vector(2.2, 1.5),
        Angle::radians(0.6),
        &stroke_options,
        &mut BuffersBuilder::new(&mut mesh, VertexCtor),
    );

    println!(
        " -- fill: {} vertices {} indices",
        mesh.vertices.len(),
        mesh.indices.len()
    );

    let mut events_loop = glium::glutin::EventsLoop::new();
    let context = glium::glutin::ContextBuilder::new().with_vsync(true);
    let window = glium::glutin::WindowBuilder::new()
        .with_dimensions(LogicalSize { width: 400.0, height: 400.0 })
        .with_decorations(true)
        .with_title("basic shapes");
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    let vertex_buffer = glium::VertexBuffer::new(&display, &mesh.vertices).unwrap();
    let indices = glium::IndexBuffer::new(
        &display,
        glium::index::PrimitiveType::TrianglesList,
        &mesh.indices,
    ).unwrap();
    let program = glium::Program::from_source(&display, VERTEX_SHADER, FRAGMENT_SHADER, None)
        .unwrap();

    let mut status = true;
    loop {
        if !status {
            break;
        }

        let mut target = display.draw();
        target.clear_color(0.8, 0.8, 0.8, 1.0);
        target.draw(
            &vertex_buffer,
            &indices,
            &program,
            &glium::uniforms::EmptyUniforms,
            &Default::default(),
        ).unwrap();

        target.finish().unwrap();

        events_loop.poll_events(|event| {
            match event {
                glium::glutin::Event::WindowEvent { event, .. } => match event {
                    glium::glutin::WindowEvent::Destroyed => { status = false }
                    _ => (),
                }
                _ => (),
            }
        });
    }
}


pub static VERTEX_SHADER: &'static str = r#"
    #version 140

    in vec2 position;

    void main() {
        vec2 pos = position.xy * 0.065 - vec2(1.0, 1.0);
        gl_Position = vec4(pos, 0.0, 1.0);
        gl_Position.y *= -1.0;
    }
"#;

pub static FRAGMENT_SHADER: &'static str = r#"
    #version 140

    out vec4 color;

    void main() {
        color = vec4(0.0, 0.0, 0.0, 1.0);
    }
"#;
