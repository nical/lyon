#![feature(macro_rules, globs)]
#![feature(default_type_params)]
#![feature(unsafe_destructor)]

extern crate native;
extern crate gl;
extern crate glfw;
extern crate libc;
extern crate core;
extern crate time;

//use data;
use io::window::Window;
use io::inputs;
use playground::app;
use math::units::world;
use math::units::texels;
use math::vector;
use gfx2d::tesselation;
use gfx2d::shapes;
use gfx2d::bezier::BezierSegment;
use gfx2d::color::Rgba;
use std::mem;
use containers::copy_on_write;

pub mod data;

pub mod math {
    pub mod units;
    pub mod vector;
}

pub mod gpu;

pub mod gfx2d;

pub mod io {
    pub mod window;
    pub mod inputs;
}

pub mod playground {
    pub mod app;
    pub mod basic_shaders;
}

pub mod containers;

struct TestApp {
    resolution: [f32, ..2],
    geom: gpu::Geometry,
    shader: gpu::Shader,
    uniforms: app::UniformLayout,
    range: gpu::Range,
    should_close: bool,
    time: f32,
}

#[deriving(Show)]
#[repr(C)]
struct Pos2DTex2D {
    x: f32,
    y: f32,
    u: f32,
    v: f32,
}

impl tesselation::VertexType2D for Pos2DTex2D {
    fn from_pos(pos: &world::Vec2) -> Pos2DTex2D {
        Pos2DTex2D {
            x: pos.x,
            y: pos.y,
            u: 0.0,
            v: 0.0,
        }
    }
    fn set_pos(&mut self, pos: &world::Vec2) { self.x = pos.x; self.y = pos.y; }
    fn set_uv(&mut self, uv: &texels::Vec2) { self.u = uv.x; self.v = uv.y; }
    fn set_color(&mut self, _color: &Rgba<u8>) { fail!() }
}

static vec2_vec2_slice_type : &'static[data::Type] = &[data::VEC2, data::VEC2];
static u16_slice_type : &'static[data::Type] = &[data::U16];

impl Pos2DTex2D {
    fn dynamically_typed_slice<'l>(data: &'l[Pos2DTex2D]) -> data::DynamicallyTypedSlice<'l> {
        data::DynamicallyTypedSlice::new(data, vec2_vec2_slice_type)
    }
}

fn index_buffer_slice<'l>(data: &'l[u16]) -> data::DynamicallyTypedSlice<'l> {
    data::DynamicallyTypedSlice::new(data, u16_slice_type)
}

impl app::App for TestApp {

    fn new(_window: &mut Window, ctx: &mut gpu::RenderingContext) -> TestApp {
        ctx.set_clear_color(0.9, 0.9, 0.9, 1.0);

        let vbo = ctx.create_buffer(gpu::VERTEX_BUFFER);
        let ibo = ctx.create_buffer(gpu::INDEX_BUFFER);

        let mut vertices = [Pos2DTex2D { x: 0.0, y: 0.0, u: 0.0, v: 0.0 }, .. 256];
        let mut indices = [0 as u16, .. 512];

        {
            let mut vertex_stream = tesselation::VertexStream {
                vertices: vertices.as_mut_slice(),
                indices: indices.as_mut_slice(),
                vertex_cursor: 0,
                index_cursor: 0,
                base_vertex: 0
            };

            let uv_transform = texels::Mat3::identity();

            tesselation::fill_rectangle(
                &mut vertex_stream,
                &world::rect(10.0, 10.0, 100.0, 200.0),
                &world::Mat3::identity(),
                tesselation::FillTexture(&uv_transform),
            );

            tesselation::fill_rectangle(
                &mut vertex_stream,
                &world::rect(300.0, 10.0, 100.0, 100.0),
                &world::Mat3::identity(),
                tesselation::FillTexture(&uv_transform),
            );

            tesselation::fill_circle(
                &mut vertex_stream,
                &shapes::Circle {
                    center: world::vec2(100.0, 400.0),
                    radius: 100.0
                },
                64,
                &world::Mat3::identity(),
                tesselation::FillTexture(&uv_transform),
            );

            let uv_columns: &[f32] = &[0.0, 0.3, 0.7, 1.0];
            let uv_lines: &[f32] = &[0.0, 0.3, 0.7, 1.0];
            tesselation::fill_grid(
                &mut vertex_stream,
                &[400.0, 420.0, 680.0, 700.0],
                &[400.0, 420.0, 680.0, 700.0],
                &world::Mat3::identity(),
                tesselation::FillTexture(&uv_transform),
                Some((uv_columns, uv_lines))
            );

            tesselation::fill_convex_path(
                &mut vertex_stream,
                &[
                    world::vec2(1000.0, 500.0),
                    world::vec2(1200.0, 500.0),
                    world::vec2(1300.0, 700.0),
                    world::vec2(1200.0, 900.0),
                    world::vec2(1000.0, 900.0),
                ],
                &world::rect(1000.0, 500.0, 300.0, 400.0),
                &world::Mat3::identity(),
                tesselation::FillTexture(&uv_transform)
            );

            tesselation::stroke_path(
                &mut vertex_stream,
                &[
                    world::vec2(1000.0, 500.0),
                    world::vec2(1200.0, 500.0),
                    world::vec2(1300.0, 700.0),
                    world::vec2(1200.0, 900.0),
                    world::vec2(1000.0, 900.0),
                ],
                &world::rect(1000.0, 500.0, 300.0, 400.0),
                &world::Mat3::identity(),
                tesselation::NoStroke,
                30.0,
                tesselation::STROKE_CLOSED
            );

            let bezier_curve = BezierSegment {
                p0: world::vec2(10.0, 800.0),
                p1: world::vec2(400.0, 800.0),
                p2: world::vec2(400.0, 1100.0),
                p3: world::vec2(800.0, 1100.0)
            };
            let mut bezier_linearized = [world::vec2(0.0, 0.0), ..16];
            bezier_curve.linearize(bezier_linearized.as_mut_slice());

            tesselation::stroke_path(
                &mut vertex_stream,
                bezier_linearized.as_slice(),
                &world::rect(1000.0, 500.0, 300.0, 400.0),
                &world::Mat3::identity(),
                tesselation::NoStroke,
                20.0,
                tesselation::STROKE_DEFAULT
            );
        }

        println!("{}", vertices.as_slice());
        println!("{}", indices.as_slice());

        ctx.upload_buffer(
            vbo,
            gpu::VERTEX_BUFFER,
            gpu::STATIC,
            &data::DynamicallyTypedSlice::new(vertices.as_slice(), vec2_vec2_slice_type)
        ).ok().expect("path vbo upload");

        ctx.upload_buffer(
            ibo,
            gpu::INDEX_BUFFER,
            gpu::STATIC,
            &index_buffer_slice(indices.as_slice())
        ).ok().expect("path ibo upload");

        let stride = mem::size_of::<Pos2DTex2D>() as u16;
        let path_geom = match ctx.create_geometry(&[
            gpu::VertexAttribute {
                buffer: vbo,
                attrib_type: data::VEC2, location: app::a_position,
                stride: stride, offset: 0, normalize: false,
            },
            gpu::VertexAttribute {
                buffer: vbo,
                attrib_type: data::VEC2, location: app::a_tex_coords,
                stride: stride, offset: 8, normalize: false,
            },
        ], Some(ibo)) {
            Err(e) => { fail!("{}", e); }
            Ok(o) => { o }
        };

        let (shader, layout) = app::setup_shader(ctx,
            playground::basic_shaders::BASIC_VERTEX_SHADER_2D,
            playground::basic_shaders::SHOW_UV_FRAGMENT_SHADER
        );

        return TestApp {
            resolution: [800.0, 600.0],
            should_close: false,
            shader: shader,
            geom: path_geom,
            range: gpu::IndexRange(0, indices.len() as u16),
            uniforms: layout,
            time : 0.0,
        }
    }

    fn update(&mut self, dt: f32, _window: &mut Window, ctx: &mut gpu::RenderingContext) {
        let screen = ctx.get_default_render_target();
        ctx.set_render_target(screen);
        ctx.clear(gpu::COLOR);

        self.time += dt;
        ctx.set_shader(self.shader).ok().expect("set shader");
        ctx.set_shader_input_float(self.uniforms.u_resolution, self.resolution);
        ctx.draw(
            self.geom, self.range,
            gpu::TRIANGLES,
            gpu::ALPHA_BLENDING,
            gpu::COLOR
        ).ok().expect("draw");
    }

    fn shut_down(&mut self, _window: &mut Window, _ctx: &mut gpu::RenderingContext) {
        println!(" -- shut_down");
    }

    fn handle_events(&mut self, _events: &[inputs::Event]) {
    }

    fn should_close(&mut self) -> bool { self.should_close }
}


fn main() {
    app::run::<TestApp>(800, 600, "cube test");
}

