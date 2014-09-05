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

pub mod data;

pub mod math {
    pub mod units;
    pub mod vector;
}

pub mod gpu;

pub mod io {
    pub mod window;
    pub mod inputs;
}

pub mod containers {
    pub mod copy_on_write;
    pub mod item_vector;
    pub mod id_lookup_table;
    pub mod freelist_vector;
    pub mod id;
}

pub mod playground {
    pub mod app;
    pub mod basic_shaders;
}




struct TestApp {
    resolution: [f32, ..2],
    cube_geom: gpu::Geometry,
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

        let cube_vertices: &[f32] = &[
          // Front face     |     normals     | tex coords
          -1.0, -1.0, 1.0,    0.0, 0.0, 1.0,    1.0, 0.0,
           1.0, -1.0, 1.0,    0.0, 0.0, 1.0,    1.0, 1.0,
           1.0,  1.0, 1.0,    0.0, 0.0, 1.0,    0.0, 1.0,
          -1.0,  1.0, 1.0,    0.0, 0.0, 1.0,    0.0, 0.0,
          // Back face
          -1.0, -1.0, -1.0,   0.0, 0.0, -1.0,   1.0, 0.0,
          -1.0,  1.0, -1.0,   0.0, 0.0, -1.0,   1.0, 1.0,
           1.0,  1.0, -1.0,   0.0, 0.0, -1.0,   0.0, 1.0,
           1.0, -1.0, -1.0,   0.0, 0.0, -1.0,   0.0, 0.0,
          // Top face
          -1.0, 1.0, -1.0,    0.0, 1.0, 1.0,    1.0, 0.0,
          -1.0, 1.0,  1.0,    0.0, 1.0, 1.0,    1.0, 1.0,
           1.0, 1.0,  1.0,    0.0, 1.0, 1.0,    0.0, 1.0,
           1.0, 1.0, -1.0,    0.0, 1.0, 1.0,    0.0, 0.0,
          // Bottom face
          -1.0, -1.0, -1.0,   0.0, -1.0, 1.0,   1.0, 0.0,
           1.0, -1.0, -1.0,   0.0, -1.0, 1.0,   1.0, 1.0,
           1.0, -1.0,  1.0,   0.0, -1.0, 1.0,   0.0, 1.0,
          -1.0, -1.0,  1.0,   0.0, -1.0, 1.0,   0.0, 0.0,
          // Right face
           1.0, -1.0, -1.0,   1.0, 0.0, 1.0,    1.0, 0.0,
           1.0,  1.0, -1.0,   1.0, 0.0, 1.0,    1.0, 1.0,
           1.0,  1.0,  1.0,   1.0, 0.0, 1.0,    0.0, 1.0,
           1.0, -1.0,  1.0,   1.0, 0.0, 1.0,    0.0, 0.0,
          // Left face
          -1.0, -1.0, -1.0,   -1.0, 0.0, 1.0,   1.0, 0.0,
          -1.0, -1.0,  1.0,   -1.0, 0.0, 1.0,   1.0, 1.0,
          -1.0,  1.0,  1.0,   -1.0, 0.0, 1.0,   0.0, 1.0,
          -1.0,  1.0, -1.0,   -1.0, 0.0, 1.0,   0.0, 0.0
        ];

        let cube_indices: &[u16] = &[
          0, 1, 2, 0, 2, 3,         // Front face
          4, 5, 6, 4, 6, 7,         // Back face
          8, 9, 10, 8, 10, 11,      // Top face
          12, 13, 14, 12, 14, 15,   // Bottom face
          16, 17, 18, 16, 18, 19,   // Right face
          20, 21, 22, 20, 22, 23    // Left face
        ];

        let cube_vbo = ctx.create_buffer(gpu::VERTEX_BUFFER);
        let cube_ibo = ctx.create_buffer(gpu::INDEX_BUFFER);

        ctx.upload_buffer(
            cube_vbo,
            gpu::VERTEX_BUFFER,
            gpu::STATIC,
            &data::DynamicallyTypedSlice::new(cube_vertices, &[data::VEC3, data::VEC3, data::VEC2])
        ).ok().expect("cube vbo upload");
        ctx.upload_buffer(
            cube_ibo,
            gpu::INDEX_BUFFER,
            gpu::STATIC,
            &index_buffer_slice(cube_indices)
        ).ok().expect("cube ibo upload");

        let cube_geom = ctx.create_geometry([
            gpu::VertexAttribute {
                buffer: cube_vbo,
                attrib_type: data::VEC3,
                location: app::a_position,
                stride: 32,
                offset: 0,
                normalize: false,
            },
            gpu::VertexAttribute {
                buffer: cube_vbo,
                attrib_type: data::VEC3,
                location: app::a_normal,
                stride: 32,
                offset: 12,
                normalize: false,
            },
            gpu::VertexAttribute {
                buffer: cube_vbo,
                attrib_type: data::VEC2,
                location: app::a_tex_coords,
                stride: 32,
                offset: 24,
                normalize: false,
            }],
            Some(cube_ibo)
        ).ok().expect("cube geom definition");

        let (shader, layout) = app::setup_shader(ctx,
            playground::basic_shaders::BASIC_VERTEX_SHADER_3D,
            playground::basic_shaders::NORMALS_FRAGMENT_SHADER
        );

        return TestApp {
            resolution: [800.0, 600.0],
            should_close: false,
            shader: shader,
            cube_geom: cube_geom,
            range: gpu::IndexRange(0, cube_indices.len() as u16),
            uniforms: layout,
            time : 0.0,
        }
    }

    fn update(&mut self, dt: f32, window: &mut Window, ctx: &mut gpu::RenderingContext) {
        let screen = ctx.get_default_render_target();
        ctx.set_render_target(screen);
        ctx.clear(gpu::COLOR|gpu::DEPTH);

        self.time += dt;
        ctx.set_shader(self.shader).ok().expect("set 3d shader");
        let mut proj_mat = world::Mat4::identity();
        world::Mat4::perspective(
            45.0,
            self.resolution[0] / self.resolution[1],
            0.5,
            1000.0,
            &mut proj_mat
        );
        let model_mat = world::Mat4::identity();
        let mut view_mat = world::Mat4::identity();
        view_mat.translate(&world::vec3(0.0, 0.0, -10.0));
        view_mat.rotate(vector::PI * (self.time * 0.000000000001).sin(), &world::vec3(0.0, 1.0, 0.0));

        ctx.set_shader_input_matrix(self.uniforms.u_model_mat, model_mat.as_slice(), 4, false);
        ctx.set_shader_input_matrix(self.uniforms.u_view_mat, view_mat.as_slice(), 4, false);
        ctx.set_shader_input_matrix(self.uniforms.u_proj_mat, proj_mat.as_slice(), 4, false);
        ctx.draw(
            self.cube_geom, self.range,
            gpu::TRIANGLES,
            gpu::ALPHA_BLENDING,
            gpu::DEPTH|gpu::COLOR
        ).ok().expect("draw(checker texture)");

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

