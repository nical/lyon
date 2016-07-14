#[macro_use]
extern crate glium;
extern crate lyon;

use glium::Surface;
use glium::glutin;
use glium::index::PrimitiveType;
use glium::DisplayBuild;

use lyon::extra::rust_logo::build_logo_path;
use lyon::path_builder::*;
use lyon::math::*;
use lyon::tessellation::vertex_builder::{ VertexConstructor, VertexBuffers, vertex_builder };
use lyon::tessellation::basic_shapes::*;
use lyon::tessellation::path_fill::FillTessellator;
use lyon::tessellation::path_stroke::StrokeTessellator;

#[derive(Copy, Clone, Debug)]
struct Vertex {
    a_position: [f32; 2],
    a_color: [f32; 3],
}

struct VertexCtor {
    color: [f32; 3]
}

impl VertexConstructor<Vec2, Vertex> for VertexCtor {
    fn new_vertex(&mut self, pos: Vec2) -> Vertex {
        Vertex {
            a_position: pos.array(),
            a_color: self.color,
        }
    }
}

implement_vertex!(Vertex, a_position, a_color);

#[derive(Copy, Clone, Debug)]
struct BgVertex {
    a_position: [f32; 2],
}

struct BgVertexCtor;
impl VertexConstructor<Vec2, BgVertex> for BgVertexCtor {
    fn new_vertex(&mut self, pos: Vec2) -> BgVertex {
        BgVertex { a_position: pos.array() }
    }
}

implement_vertex!(BgVertex, a_position);

fn uniform_matrix(m: &Mat4) -> [[f32; 4]; 4] {
    [
        [m.m11, m.m12, m.m13, m.m14],
        [m.m21, m.m22, m.m23, m.m24],
        [m.m31, m.m32, m.m33, m.m34],
        [m.m41, m.m42, m.m43, m.m44],
    ]
}

fn main() {

    let mut builder = flattened_path_builder(0.05);

    build_logo_path(&mut builder);

    builder.move_to(vec2(10.0, 30.0));
    builder.line_to(vec2(130.0, 30.0));
    builder.line_to(vec2(130.0, 60.0));
    builder.line_to(vec2(10.0, 60.0));
    builder.close();

    let path = builder.build();

    let mut buffers: VertexBuffers<Vertex> = VertexBuffers::new();

    FillTessellator::new().tessellate(
        path.as_slice(),
        &mut vertex_builder(&mut buffers, VertexCtor{ color: [0.9, 0.9, 1.0] })
    ).unwrap();

    StrokeTessellator::new().tessellate(
        path.as_slice(), 1.0,
        &mut vertex_builder(&mut buffers, VertexCtor{ color: [0.0, 0.0, 0.0] })
    ).unwrap();


    for p in path.vertices().as_slice() {
        tessellate_ellipsis(p.position, vec2(1.0, 1.0), 16,
            &mut vertex_builder(&mut buffers,
                VertexCtor{ color: [0.0, 0.0, 0.0] }
            )
        );
        tessellate_ellipsis(p.position, vec2(0.5, 0.5), 16,
            &mut vertex_builder(&mut buffers,
                VertexCtor{ color: [0.0, 1.0, 0.0] }
            )
        );
    }

    let (indices, vertices) = (buffers.indices, buffers.vertices);

    println!(" -- {} vertices {} indices", vertices.len(), indices.len());

    let mut bg_buffers: VertexBuffers<BgVertex> = VertexBuffers::new();
    tessellate_rectangle(
        &Rect::new(vec2(-1.0, -1.0), size(2.0, 2.0)),
        &mut vertex_builder(&mut bg_buffers, BgVertexCtor)
    );

    // building the display, ie. the main object
    let display = glutin::WindowBuilder::new()
        .with_dimensions(700, 700)
        .with_title("tessellation".to_string())
        .with_multisampling(8)
        .with_vsync()
        .build_glium().unwrap();

    let model_vbo = glium::VertexBuffer::new(&display, &vertices[..]).unwrap();
    let model_ibo = glium::IndexBuffer::new(
        &display, PrimitiveType::TrianglesList,
        &indices[..]
    ).unwrap();

    let bg_vbo = glium::VertexBuffer::new(&display, &bg_buffers.vertices[..]).unwrap();
    let bg_ibo = glium::IndexBuffer::new(
        &display, PrimitiveType::TrianglesList,
        &bg_buffers.indices[..]
    ).unwrap();

    // compiling shaders and linking them together
    let bg_program = program!(&display,
        140 => {
            vertex: "
                #version 140
                in vec2 a_position;
                out vec2 v_position;
                void main() {
                    gl_Position = vec4(a_position, 0.0, 1.0);
                    v_position = a_position;
                }
            ",
            fragment: "
                #version 140
                uniform vec2 u_resolution;
                in vec2 v_position;
                out vec4 f_color;
                void main() {
                    vec2 px_position = (v_position * vec2(1.0, -1.0)    + vec2(1.0, 1.0))
                                     * 0.5 * u_resolution;
                    // #005fa4
                    float vignette = clamp(0.0, 1.0, (0.7*length(v_position)));

                    f_color = mix(
                        vec4(0.0, 0.47, 0.9, 1.0),
                        vec4(0.0, 0.1, 0.64, 1.0),
                        vignette
                    );

                    if (mod(px_position.x, 20.0) <= 1.0 ||
                        mod(px_position.y, 20.0) <= 1.0) {
                        f_color *= 1.2;
                    }

                    if (mod(px_position.x, 100.0) <= 1.0 ||
                        mod(px_position.y, 100.0) <= 1.0) {
                        f_color *= 1.2;
                    }
                }
            "
        },
    ).unwrap();

    // compiling shaders and linking them together
    let model_program = program!(&display,
        140 => {
            vertex: "
                #version 140
                uniform vec2 u_resolution;
                uniform mat4 u_matrix;
                in vec2 a_position;
                in vec3 a_color;
                out vec3 v_color;
                void main() {
                    gl_Position = u_matrix * vec4(a_position, 0.0, 1.0);// / vec4(u_resolution, 1.0, 1.0);
                    v_color = a_color;
                }
            ",
            fragment: "
                #version 140
                in vec3 v_color;
                out vec4 f_color;
                void main() {
                    f_color = vec4(v_color, 1.0);
                }
            "
        },
    ).unwrap();

    let mut target_zoom = 1.0;
    let mut zoom = 1.0;
    let mut target_pos = vec2(0.0, 0.0);
    let mut pos = vec2(0.0, 0.0);
    loop {
        zoom += (target_zoom - zoom) / 3.0;
        pos = pos + (target_pos - pos) / 3.0;

        let mut target = display.draw();

        let (w, h) = target.get_dimensions();
        let resolution = vec2(w as f32, h as f32);

        let model_mat = Mat4::identity();
        let mut view_mat = Mat4::identity();

        view_mat = view_mat.translate(-1.0, 1.0, 0.0);
        view_mat = view_mat.scale(5.0 * zoom, 5.0 * zoom, 0.0);
        view_mat = view_mat.scale(2.0/resolution.x, -2.0/resolution.y, 1.0);
        view_mat = view_mat.translate(pos.x, pos.y, 0.0);

        let uniforms = uniform! {
            u_resolution: resolution.array(),
            u_matrix: uniform_matrix(&model_mat.mul(&view_mat))
        };

        target.clear_color(0.75, 0.75, 0.75, 1.0);
        target.draw(
            &bg_vbo, &bg_ibo,
            &bg_program, &uniforms,
            &Default::default()
        ).unwrap();
        target.draw(
            &model_vbo, &model_ibo,
            &model_program, &uniforms,
            &Default::default()
        ).unwrap();
        target.finish().unwrap();

        let mut should_close = false;
        for event in display.poll_events() {
            should_close |= match event {
                glutin::Event::Closed => true,
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Escape)) => true,
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::PageDown)) => {
                    target_zoom *= 0.8;
                    false
                }
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::PageUp)) => {
                    target_zoom *= 1.25;
                    false
                }
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Left)) => {
                    target_pos.x += 5.0 / target_zoom;
                    false
                }
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Right)) => {
                    target_pos.x -= 5.0 / target_zoom;
                    false
                }
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Up)) => {
                    target_pos.y += 5.0 / target_zoom;
                    false
                }
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Down)) => {
                    target_pos.y -= 5.0 / target_zoom;
                    false
                }
                _evt => {
                    //println!("{:?}", _evt);
                    false
                }
            };
        }
        if should_close {
            break;
        }
    }
}
