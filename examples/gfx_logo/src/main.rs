#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate lyon;

use lyon::extra::rust_logo::build_logo_path;
use lyon::path_builder::*;
use lyon::math::*;
use lyon::tessellation::geometry_builder::{ VertexConstructor, VertexBuffers, BuffersBuilder };
use lyon::tessellation::basic_shapes::*;
use lyon::tessellation::path_fill::{ FillEvents, FillTessellator, FillOptions };
use lyon::tessellation::path_stroke::{ StrokeTessellator, StrokeOptions };
use lyon::path::Path;
use lyon::path_iterator::PathIterator;

use gfx::traits::FactoryExt;
use gfx::Device;

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

gfx_defines!{
    constant Constants {
        transform: [[f32; 4]; 4] = "u_transform",
        resolution: [f32; 2] = "u_resolution",
    }

    vertex Vertex {
        a_position: [f32; 2] = "a_position",
        a_color: [f32; 3] = "a_color",
    }

    vertex BgVertex {
        a_position: [f32; 2] = "a_position",
    }

    pipeline model_pipeline {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        out: gfx::RenderTarget<ColorFormat> = "out_color",
        constants: gfx::ConstantBuffer<Constants> = "Constants",
    }

    pipeline bg_pipeline {
        vbuf: gfx::VertexBuffer<BgVertex> = (),
        out: gfx::RenderTarget<ColorFormat> = "out_color",
        constants: gfx::ConstantBuffer<Constants> = "Constants",
    }
}

struct WithColor([f32; 3]);

impl VertexConstructor<Vec2, Vertex> for WithColor {
    fn new_vertex(&mut self, pos: Vec2) -> Vertex {
        assert!(!pos.x.is_nan());
        assert!(!pos.y.is_nan());
        Vertex {
            a_position: pos.array(),
            a_color: self.0,
        }
    }
}

struct BgVertexCtor ;
impl VertexConstructor<Vec2, BgVertex> for BgVertexCtor  {
    fn new_vertex(&mut self, pos: Vec2) -> BgVertex {
        BgVertex { a_position: pos.array() }
    }
}

fn uniform_matrix(m: &Mat4) -> [[f32; 4]; 4] {
    [
        [m.m11, m.m12, m.m13, m.m14],
        [m.m21, m.m22, m.m23, m.m24],
        [m.m31, m.m32, m.m33, m.m34],
        [m.m41, m.m42, m.m43, m.m44],
    ]
}

fn main() {
    let mut builder = SvgPathBuilder::new(Path::builder());

    build_logo_path(&mut builder);

    let path = builder.build();

    let mut buffers: VertexBuffers<Vertex> = VertexBuffers::new();

    let events = FillEvents::from_iter(path.path_iter().flattened(0.03));

    FillTessellator::new().tessellate_events(
        &events,
        &FillOptions::default(),
        &mut BuffersBuilder::new(&mut buffers, WithColor([0.9, 0.9, 1.0]))
    ).unwrap();

    StrokeTessellator::new().tessellate(
        path.path_iter().flattened(0.03),
        &StrokeOptions::stroke_width(1.0),
        &mut BuffersBuilder::new(&mut buffers, WithColor([0.0, 0.0, 0.0]))
    ).unwrap();

    let show_points = false;

    if show_points {
        for p in path.as_slice().iter() {
            if let Some(to) = p.destination() {
                tessellate_ellipsis(to, vec2(1.0, 1.0), 16,
                    &mut BuffersBuilder::new(&mut buffers,
                        WithColor([0.0, 0.0, 0.0])
                    )
                );
                tessellate_ellipsis(to, vec2(0.5, 0.5), 16,
                    &mut BuffersBuilder::new(&mut buffers,
                        WithColor([0.0, 1.0, 0.0])
                    )
                );
            }
        }
    }

    println!(" -- {} vertices {} indices", buffers.vertices.len(), buffers.indices.len());

    let mut bg_buffers: VertexBuffers<BgVertex> = VertexBuffers::new();
    tessellate_rectangle(
        &Rect::new(vec2(-1.0, -1.0), size(2.0, 2.0)),
        &mut BuffersBuilder::new(&mut bg_buffers, BgVertexCtor )
    );

    // building the display, ie. the main object
    let glutin_builder = glutin::WindowBuilder::new()
        .with_dimensions(700, 700)
        .with_title("tessellation".to_string())
        .with_multisampling(8)
        .with_vsync();

    let (window, mut device, mut factory, mut main_fbo, mut _main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(glutin_builder);

    let mut cmd_queue: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let constants = factory.create_constant_buffer(1);

    let bg_pso = factory.create_pipeline_simple(
        BACKGROUND_VERTEX_SHADER.as_bytes(),
        BACKGROUND_FRAGMENT_SHADER.as_bytes(),
        bg_pipeline::new()
    ).unwrap();

    let (bg_vbo, bg_range) = factory.create_vertex_buffer_with_slice(
        &bg_buffers.vertices[..],
        &bg_buffers.indices[..]
    );

    let model_pso = factory.create_pipeline_simple(
        MODEL_VERTEX_SHADER.as_bytes(),
        MODEL_FRAGMENT_SHADER.as_bytes(),
        model_pipeline::new()
    ).unwrap();

    let (model_vbo, model_range) = factory.create_vertex_buffer_with_slice(
        &buffers.vertices[..],
        &buffers.indices[..]
    );

    let mut target_zoom = 1.0;
    let mut zoom = 1.0;
    let mut target_pos = vec2(0.0, 0.0);
    let mut pos = vec2(0.0, 0.0);
    let mut resolution;
    loop {
        zoom += (target_zoom - zoom) / 3.0;
        pos = pos + (target_pos - pos) / 3.0;

        gfx_window_glutin::update_views(&window, &mut main_fbo, &mut _main_depth);
        let (w, h) = window.get_inner_size_pixels().unwrap();

        resolution = vec2(w as f32, h as f32);

        let model_mat = Mat4::identity();
        let mut view_mat = Mat4::identity();

        view_mat = view_mat.pre_translated(-1.0, 1.0, 0.0);
        view_mat = view_mat.pre_scaled(5.0 * zoom, 5.0 * zoom, 0.0);
        view_mat = view_mat.pre_scaled(2.0/resolution.x, -2.0/resolution.y, 1.0);
        view_mat = view_mat.pre_translated(pos.x, pos.y, 0.0);

        cmd_queue.clear(&main_fbo.clone(), [0.0, 0.0, 0.0, 0.0]);
        cmd_queue.update_constant_buffer(&constants, &Constants {
            resolution: resolution.array(),
            transform: uniform_matrix(&model_mat.pre_mul(&view_mat))
        });
        cmd_queue.draw(&bg_range, &bg_pso, &bg_pipeline::Data {
            vbuf: bg_vbo.clone(),
            out: main_fbo.clone(),
            constants: constants.clone(),
        });
        cmd_queue.draw(&model_range, &model_pso, &model_pipeline::Data {
            vbuf: model_vbo.clone(),
            out: main_fbo.clone(),
            constants: constants.clone(),
        });
        cmd_queue.flush(&mut device);

        window.swap_buffers().unwrap();
        device.cleanup();

        let mut should_close = false;
        for event in window.poll_events() {
            match event {
                glutin::Event::Closed => {
                    should_close = true;
                }
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Escape)) => {
                    should_close = true;
                }
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::PageDown)) => {
                    target_zoom *= 0.8;
                }
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::PageUp)) => {
                    target_zoom *= 1.25;
                }
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Left)) => {
                    target_pos.x += 5.0 / target_zoom;
                }
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Right)) => {
                    target_pos.x -= 5.0 / target_zoom;
                }
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Up)) => {
                    target_pos.y += 5.0 / target_zoom;
                }
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Down)) => {
                    target_pos.y -= 5.0 / target_zoom;
                }
                _evt => {
                    //println!("{:?}", _evt);
                }
            };
        }
        if should_close {
            break;
        }
    }
}

static MODEL_VERTEX_SHADER: &'static str = &"
    #version 140
    uniform Constants {
        mat4 u_transform;
        vec2 u_resolution;
    };
    in vec2 a_position;
    in vec3 a_color;
    out vec3 v_color;

    void main() {
        gl_Position = u_transform * vec4(a_position, 0.0, 1.0);// / vec4(u_resolution, 1.0, 1.0);
        v_color = a_color;
    }
";

static MODEL_FRAGMENT_SHADER: &'static str = &"
    #version 140
    in vec3 v_color;
    out vec4 out_color;
    void main() {
        out_color = vec4(v_color, 1.0);
    }
";

static BACKGROUND_VERTEX_SHADER: &'static str = &"
    #version 140
    in vec2 a_position;
    out vec2 v_position;
    void main() {
        gl_Position = vec4(a_position, 0.0, 1.0);
        v_position = a_position;
    }
";

static BACKGROUND_FRAGMENT_SHADER: &'static str = &"
    #version 140
    //uniform vec2 u_resolution;
    //uniform mat4 u_transform;
    uniform Constants {
        mat4 u_transform;
        vec2 u_resolution;
    };
    in vec2 v_position;
    out vec4 out_color;
    void main() {
        vec2 px_position = (v_position * vec2(1.0, -1.0)    + vec2(1.0, 1.0))
                         * 0.5 * u_resolution;
        // #005fa4
        float vignette = clamp(0.0, 1.0, (0.7*length(v_position)));

        out_color = mix(
            vec4(0.0, 0.47, 0.9, 1.0),
            vec4(0.0, 0.1, 0.64, 1.0),
            vignette
        );

        if (mod(px_position.x, 20.0) <= 1.0 ||
            mod(px_position.y, 20.0) <= 1.0) {
            out_color *= 1.2;
        }

        if (mod(px_position.x, 100.0) <= 1.0 ||
            mod(px_position.y, 100.0) <= 1.0) {
            out_color *= 1.2;
        }
    }
";
