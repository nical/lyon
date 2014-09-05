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
use math::vector;
use gfx2d::tesselation;
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

pub mod containers {
    pub mod copy_on_write;
    pub mod item_vector;
    pub mod id_lookup_table;
    pub mod freelist_vector;
    pub mod id;
}


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
    s: f32,
    t: f32,
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

    fn new(window: &mut Window, ctx: &mut gpu::RenderingContext) -> TestApp {
        ctx.set_clear_color(0.9, 0.9, 0.9, 1.0);

        let path_vbo = ctx.create_buffer(gpu::VERTEX_BUFFER);
        let path_ibo = ctx.create_buffer(gpu::INDEX_BUFFER);
        let path = &[
                world::vec2(-500.0, -500.0),
                world::vec2(-300.0, -500.0),
                world::vec2(-300.0, -300.0),
                world::vec2( 100.0, -300.0),
                world::vec2( 100.0, -500.0),
                world::vec2( 150.0, -300.0),
                world::vec2( 200.0, -500.0),
                world::vec2( 200.0,  200.0),
                world::vec2( 200.0,  500.0),
                world::vec2(-500.0,  200.0)
        ];
        let mut path_vertices = [
            tesselation::Pos2DNormal2DColorExtrusion {
                pos: world::vec2(0.0, 0.0),
                normal: world::vec2(0.0, 0.0),
                color: Rgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
                extrusion: 0.0,
            },
            .. 40 // n_points * 4
        ];
        let mut path_indices = [0 as u16, .. 180]; // n_points*18

        tesselation::path_to_line_vbo(
            path.as_slice(),
            true,
            tesselation::VERTEX_ANTIALIASING|tesselation::CONVEX_SHAPE,
            |_| { 10.0 },
            |_, ptype| { match ptype {
                tesselation::AntialiasPoint => Rgba { r: 0.0, g: 0.0, b: 0.3, a: 0.0 },
                _ => Rgba { r: 0.0, g: 0.0, b: 0.3, a: 1.0 },
            }},
            world::Mat3::identity(),
            path_vertices.as_mut_slice()
        );
        tesselation::path_to_line_ibo(
            path.len() as u32,
            true,
            tesselation::VERTEX_ANTIALIASING|tesselation::CONVEX_SHAPE,
            0,
            path_indices.as_mut_slice()
        );

        ctx.upload_buffer(
            path_vbo,
            gpu::VERTEX_BUFFER,
            gpu::STATIC,
            &data::DynamicallyTypedSlice::from_slice(path_vertices.slice(0, path.len() * 4))
        ).ok().expect("path vbo upload");

        ctx.upload_buffer(
            path_ibo,
            gpu::INDEX_BUFFER,
            gpu::STATIC,
            &index_buffer_slice(path_indices.slice(0, path.len() * 18))
        ).ok().expect("path ibo upload");

        let stride = mem::size_of::<tesselation::Pos2DNormal2DColorExtrusion>() as u16;
        assert!(stride == 36);
        let path_geom = match ctx.create_geometry(&[
            gpu::VertexAttribute {
                buffer: path_vbo,
                attrib_type: data::VEC2, location: app::a_position,
                stride: stride, offset: 0, normalize: false,
            },
            gpu::VertexAttribute {
                buffer: path_vbo,
                attrib_type: data::VEC2, location: app::a_normal,
                stride: stride, offset: 8, normalize: false,
            },
            gpu::VertexAttribute {
                buffer: path_vbo,
                attrib_type: data::VEC4, location: app::a_color,
                stride: stride, offset: 16, normalize: false,
            },
            gpu::VertexAttribute {
                buffer: path_vbo,
                attrib_type: data::F32, location: app::a_extrusion,
                stride: stride, offset: 32, normalize: false,
            }
        ], Some(path_ibo)) {
            Err(e) => { fail!("{}", e); }
            Ok(o) => { o }
        };

        let (shader, layout) = app::setup_shader(ctx,
            playground::basic_shaders::SHAPE_VERTEX_SHADER_2D,
            playground::basic_shaders::COLOR_FRAGMENT_SHADER
        );

        return TestApp {
            resolution: [800.0, 600.0],
            should_close: false,
            shader: shader,
            geom: path_geom,
            range: gpu::IndexRange(0, (path.len()*6*3) as u16),
            uniforms: layout,
            time : 0.0,
        }
    }

    fn update(&mut self, dt: f32, window: &mut Window, ctx: &mut gpu::RenderingContext) {
        let screen = ctx.get_default_render_target();
        ctx.set_render_target(screen);
        ctx.clear(gpu::COLOR);

        self.time += dt;
        let view_mat = world::Mat3::identity();
        let model_mat = world::Mat3::identity();
        ctx.set_shader(self.shader).ok().expect("set shader");
        ctx.set_shader_input_float(self.uniforms.u_resolution, self.resolution);
        ctx.set_shader_input_matrix(self.uniforms.u_view_mat, view_mat.as_slice(), 3, false);
        ctx.set_shader_input_matrix(self.uniforms.u_model_mat, model_mat.as_slice(), 3, false);
        ctx.draw(
            self.geom, self.range,
            gpu::TRIANGLES,
            gpu::ALPHA_BLENDING,
            gpu::COLOR
        ).ok().expect("draw");

    }

    fn shut_down(&mut self, window: &mut Window, ctx: &mut gpu::RenderingContext) {
        println!(" -- shut_down");
    }

    fn handle_events(&mut self, events: &[inputs::Event]) {
    }

    fn should_close(&mut self) -> bool { self.should_close }
}


fn main() {
    app::run::<TestApp>(800, 600, "cube test");
}

