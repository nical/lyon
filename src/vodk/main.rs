#![crate_id = "vodk#0.1"]
#![feature(macro_rules, globs)]
#![feature(default_type_params)]

extern crate native;
extern crate gl;
extern crate glfw;
extern crate time;
extern crate png;
extern crate libc;
extern crate core;

use gfx::renderer;
use gfx::shaders;
use gfx::text;
use gfx::window;
use math::vector;
use math::units::pixels;
use math::units::texels;
use math::units::world;
use io::inputs;
use gfx::locations::*;
use gfx::ui;
use gfx::scenegraph;
use gfx::tesselation;
use gfx::color::Rgba;
use std::mem;

use std::io::timer::sleep;

pub mod gfx {
    pub mod renderer;
    pub mod opengl;
    pub mod window;
    pub mod shaders;
    pub mod mesh_utils;
    pub mod geom;
    pub mod text;
    pub mod test_renderer;
    pub mod ui;
    pub mod color;
    pub mod locations;
    pub mod scenegraph;
    pub mod tesselation;
}
pub mod logic {
    pub mod entity;
}
pub mod base {
    pub mod containers;
}

pub mod kiwi {
    pub mod graph;
}

pub mod math {
    pub mod vector;
    pub mod units;
}

pub mod io {
    pub mod inputs;
}

pub mod app;
pub mod data;

#[deriving(Show)]
struct Pos2DTex2D {
    x: f32,
    y: f32,
    s: f32,
    t: f32,
}

static vec2_vec2_slice_type : &'static[data::Type] = &[data::VEC2, data::VEC2];
static u8_slice_type : &'static[data::Type] = &[data::U8];
static u16_slice_type : &'static[data::Type] = &[data::U16];

impl Pos2DTex2D {
    fn dynamically_typed_slice<'l>(data: &'l[Pos2DTex2D]) -> data::DynamicallyTypedSlice<'l> {
        data::DynamicallyTypedSlice::new(data, vec2_vec2_slice_type)
    }
}

fn dynamically_typed_byte_slice<'l>(data: &'l[u8]) -> data::DynamicallyTypedSlice<'l> {
    data::DynamicallyTypedSlice::new(data, u8_slice_type)
}

fn index_buffer_slice<'l>(data: &'l[u16]) -> data::DynamicallyTypedSlice<'l> {
    data::DynamicallyTypedSlice::new(data, u16_slice_type)
}

struct TestApp {
    shaders: Vec<(renderer::ShaderProgram, UniformLayout)>,
    draw_calls: Vec<DrawCall>,
    textures: Vec<renderer::Texture>,
    ctx: Box<renderer::RenderingContext>,
    width: u32,
    height: u32,
}

pub struct DrawCall {
    geom: renderer::Geometry,
    first: u32,
    count: u32,
    flags: renderer::GeometryFlags,
    targets: renderer::TargetTypes,
}

impl app::Application for TestApp {
    fn handle_events(&mut self, events: &[inputs::Event]) {
        for e in events.iter() {
            match *e {
                inputs::CursorPosEvent(x, y) => {
                    println!("cursor: {} {}", x, y);
                }
                inputs::MouseButtonEvent(button, action) => {
                    println!("MouseButtonEvent: {} {}", button, action);
                }
                inputs::ScrollEvent(dx, dy) => {
                    println!("ScrollEvent: {} {}", dx, dy);
                }
                inputs::FocusEvent(focused) => {
                    println!("FocusEvent: {}", focused);
                }
                inputs::CloseEvent => {
                    println!("CloseEvent");
                }
                inputs::FramebufferSizeEvent(w, h) => {
                    self.width = w as u32;
                    self.height = h as u32;
                    self.ctx.set_viewport(0, 0, w, h);
                    self.update(0.16, 0);
                    println!("FramebufferSizeEvent {} {}", w, h);
                }
                inputs::DummyEvent => {}
            }
        }
    }

    fn update(&mut self, _dt: f32, frame_count: u64) {
        let ctx: &mut renderer::RenderingContext = self.ctx;
        let screen = ctx.get_default_render_target();
        ctx.set_render_target(screen);
        ctx.clear(renderer::COLOR|renderer::DEPTH);

        let resolution = &[self.width as f32, self.height as f32];

        let mut i = 0;

        let &(shader, uniforms) = self.shaders.get(i);
        let dc = self.draw_calls.get(i);
        let view_mat = world::Mat3::identity();
        let model_mat = world::Mat3::identity();
        ctx.set_shader(shader).ok().expect("set path shader");
        ctx.set_shader_input_float(uniforms.u_resolution, resolution);
        ctx.set_shader_input_matrix(uniforms.u_view_mat, view_mat.as_slice(), 3, false);
        ctx.set_shader_input_matrix(uniforms.u_model_mat, model_mat.as_slice(), 3, false);
        ctx.draw(dc.geom, dc.first, dc.count, dc.flags, renderer::ALPHA_BLENDING, dc.targets).ok().expect("draw(path)");

        i+=1;
        let &(shader, uniforms) = self.shaders.get(i);
        let dc = self.draw_calls.get(i);
        ctx.set_shader(shader).ok().expect("set ui shader");
        ctx.set_shader_input_texture(uniforms.u_texture_0, 0, *self.textures.get(0));
        ctx.set_shader_input_float(uniforms.u_resolution, resolution);
        ctx.draw(dc.geom, dc.first, dc.count, dc.flags, renderer::ALPHA_BLENDING, dc.targets).ok().expect("draw(ui)");

        i+=1;
        //let &(shader, uniforms) = self.shaders.get(i);
        //let dc = self.draw_calls.get(i);
        //ctx.set_shader(shader).ok().expect("set texturing shader");
        //ctx.set_shader_input_texture(uniforms.u_texture_0, 0, *self.textures.get(0));
        //ctx.set_shader_input_float(uniforms.u_resolution, [self.width as f32, self.height as f32]);
        //ctx.draw(dc.geom, dc.first, dc.count, dc.flags, renderer::ALPHA_BLENDING, dc.targets).ok().expect("draw(checker texture)");

        i+=1;
        let &(shader, uniforms) = self.shaders.get(i);
        let dc = self.draw_calls.get(i);
        ctx.set_shader(shader).ok().expect("set text shader");
        ctx.set_shader_input_float(uniforms.u_color, [1.0, 0.0, 0.0, 1.0]);
        ctx.set_shader_input_float(uniforms.u_resolution, resolution);
        ctx.set_shader_input_texture(uniforms.u_texture_0, 0, *self.textures.get(1));
        ctx.draw(dc.geom, dc.first, dc.count, dc.flags, renderer::ALPHA_BLENDING, dc.targets).ok().expect("draw(text)");


        i+=1;
        let &(shader, uniforms) = self.shaders.get(i);
        let dc = self.draw_calls.get(i);
        ctx.set_shader(shader).ok().expect("set 3d shader");
        let mut proj_mat = world::Mat4::identity();
        world::Mat4::perspective(45.0, self.width as f32 / self.height as f32, 0.5, 1000.0, &mut proj_mat);
        let model_mat = world::Mat4::identity();
        let mut view_mat = world::Mat4::identity();
        view_mat.translate(&world::vec3(0.0, 0.0, -10.0));
        view_mat.rotate(vector::PI * (frame_count as f32 * 0.01).sin(), &world::vec3(0.0, 1.0, 0.0));

        ctx.set_shader_input_matrix(uniforms.u_model_mat, model_mat.as_slice(), 4, false);
        ctx.set_shader_input_matrix(uniforms.u_view_mat, view_mat.as_slice(), 4, false);
        ctx.set_shader_input_matrix(uniforms.u_proj_mat, proj_mat.as_slice(), 4, false);
        ctx.draw(dc.geom, dc.first, dc.count, dc.flags, renderer::ALPHA_BLENDING, dc.targets).ok().expect("draw(checker texture)");

        ctx.swap_buffers();
    }

    fn get_help(&self) -> String { "Vodk!".to_string() }

    fn shut_down(&mut self) {
        println!("bye");
    }
}

impl TestApp {
    fn new(
        window: &mut window::Window,
        ctx: Box<renderer::RenderingContext>
    ) -> TestApp {
        TestApp {
            textures: Vec::new(),
            draw_calls: Vec::new(),
            shaders: Vec::new(),
            ctx: ctx,
            width: 800,
            height: 600,
        }
    }

    fn init(&mut self) {
        let ctx: &mut renderer::RenderingContext = self.ctx;
        ctx.set_clear_color(0.8, 0.8, 0.8, 1.0);

        let path_vbo = ctx.create_buffer(renderer::VERTEX_BUFFER);
        let path_ibo = ctx.create_buffer(renderer::INDEX_BUFFER);
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

        //for v in path_vertices.slice(0, path.len() * 4).iter() {
        //    println!("{}", v);
        //}
        //for i in range(0, path.len() * 6) {
        //    println!("({} {} {})", path_indices[i*3], path_indices[i*3+1], path_indices[i*3+2]);
        //}

        ctx.upload_buffer(
            path_vbo,
            renderer::VERTEX_BUFFER,
            renderer::STATIC,
            &data::DynamicallyTypedSlice::from_slice(path_vertices.slice(0, path.len() * 4))
        ).ok().expect("path vbo upload");

        ctx.upload_buffer(
            path_ibo,
            renderer::INDEX_BUFFER,
            renderer::STATIC,
            &index_buffer_slice(path_indices.slice(0, path.len() * 18))
        ).ok().expect("path ibo upload");

        let stride = mem::size_of::<tesselation::Pos2DNormal2DColorExtrusion>() as u16;
        assert!(stride == 36);
        let path_geom = match ctx.create_geometry(&[
            renderer::VertexAttribute {
                buffer: path_vbo,
                attrib_type: data::VEC2, location: a_position,
                stride: stride, offset: 0, normalize: false,
            },
            renderer::VertexAttribute {
                buffer: path_vbo,
                attrib_type: data::VEC2, location: a_normal,
                stride: stride, offset: 8, normalize: false,
            },
            renderer::VertexAttribute {
                buffer: path_vbo,
                attrib_type: data::VEC4, location: a_color,
                stride: stride, offset: 16, normalize: false,
            },
            renderer::VertexAttribute {
                buffer: path_vbo,
                attrib_type: data::F32, location: a_extrusion,
                stride: stride, offset: 32, normalize: false,
            }
        ], Some(path_ibo)) {
            Err(e) => { fail!("{}", e); }
            Ok(o) => { o }
        };

        self.shaders.push(setup_shader(ctx,
            shaders::SHAPE_VERTEX_SHADER_2D,
            shaders::COLOR_FRAGMENT_SHADER
        ));
        self.draw_calls.push(
            DrawCall {
                geom: path_geom,
                first: 0,
                count: (path.len()*6*3)  as u32,
                flags: renderer::TRIANGLES,
                targets: renderer::COLOR,
            }
        );


        let mut ui_vertices = [0.0 as f32, .. 512];
        let mut ui_indices = [0 as u16, .. 256];
        let ui_vbo = ctx.create_buffer(renderer::VERTEX_BUFFER);
        let ui_ibo = ctx.create_buffer(renderer::INDEX_BUFFER);

        let ui_attribs = &[
            renderer::VertexAttribute {
                buffer: ui_vbo,
                attrib_type: data::VEC2,
                location: a_position,
                stride: 16,
                offset: 0,
                normalize: false,
            },
            renderer::VertexAttribute {
                buffer: ui_vbo,
                attrib_type: data::VEC2,
                location: a_tex_coords,
                stride: 16,
                offset: 8,
                normalize: false,
            }
        ];
        let (ui_vbo_size, ui_ibo_size) = {
            let mut ui_batch = ui::IndexedBatch::new(
                ui_vertices.as_mut_slice(),
                ui_indices.as_mut_slice(),
                0, 4, ui_attribs
            );
            ui::push_rect(
                &mut ui_batch,
                pixels::rect(500.0, 0.0, 100.0, 100.0),
                Some(texels::rect(0.0, 0.0, 1.0, 1.0)),
                None
            );
            ui::push_circle(
                &mut ui_batch,
                300.0, 300.0, 100.0, 33,
                Some(texels::rect(0.0, 0.0, 1.0, 1.0)),
                None
            );
            ui::push_circle(
                &mut ui_batch,
                600.0, 300.0, 50.0, 33,
                Some(texels::rect(0.0, 0.0, 1.0, 1.0)),
                None
            );
            ui::push_rect(
                &mut ui_batch,
                pixels::rect(-0.0, 0.0, 100.0, 100.0),
                Some(texels::rect(0.0, 0.0, 1.0, 1.0 )),
                None
            );

            (ui_batch.vertex_cursor*ui_batch.vertex_stride, ui_batch.index_cursor)
        };

        ctx.upload_buffer(
            ui_vbo,
            renderer::VERTEX_BUFFER,
            renderer::STATIC,
            &data::DynamicallyTypedSlice::new(ui_vertices, &[data::VEC2, data::VEC2])
        );
        ctx.upload_buffer(
            ui_ibo,
            renderer::INDEX_BUFFER,
            renderer::STATIC,
            &index_buffer_slice(ui_indices)
        );

        let ui_geom = ctx.create_geometry(ui_attribs, Some(ui_ibo)).ok().expect("ui_geom");

        self.shaders.push(setup_shader(ctx,
            shaders::BASIC_VERTEX_SHADER_2D,
            shaders::TEXTURED_FRAGMENT_SHADER
        ));
        self.draw_calls.push(
            DrawCall {
                geom: ui_geom,
                first: 0, count: ui_ibo_size as u32,
                flags: renderer::TRIANGLES,
                targets: renderer::COLOR
            }
        );

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

        let cube_vbo = ctx.create_buffer(renderer::VERTEX_BUFFER);
        let cube_ibo = ctx.create_buffer(renderer::INDEX_BUFFER);

        ctx.upload_buffer(
            cube_vbo,
            renderer::VERTEX_BUFFER,
            renderer::STATIC,
            &data::DynamicallyTypedSlice::new(cube_vertices, &[data::VEC3, data::VEC3, data::VEC2])
        ).ok().expect("cube vbo upload");
        ctx.upload_buffer(
            cube_ibo,
            renderer::INDEX_BUFFER,
            renderer::STATIC,
            &index_buffer_slice(cube_indices)
        ).ok().expect("cube ibo upload");

        let cube_geom = ctx.create_geometry([
            renderer::VertexAttribute {
                buffer: cube_vbo,
                attrib_type: data::VEC3,
                location: a_position,
                stride: 32,
                offset: 0,
                normalize: false,
            },
            renderer::VertexAttribute {
                buffer: cube_vbo,
                attrib_type: data::VEC3,
                location: a_normal,
                stride: 32,
                offset: 12,
                normalize: false,
            },
            renderer::VertexAttribute {
                buffer: cube_vbo,
                attrib_type: data::VEC2,
                location: a_tex_coords,
                stride: 32,
                offset: 24,
                normalize: false,
            }],
            Some(cube_ibo)
        ).ok().expect("cube geom definition");

        let quad_vertices: &[Pos2DTex2D] = & [
            Pos2DTex2D { x:   0.0, y:   0.0,    s: 0.0, t: 0.0 },
            Pos2DTex2D { x: 200.0, y:   0.0,    s: 1.0, t: 0.0 },
            Pos2DTex2D { x: 200.0, y: 200.0,    s: 1.0, t: 1.0 },
            Pos2DTex2D { x:   0.0, y: 200.0,    s: 0.0, t: 1.0 },
        ];

        let quad_indices: &[u16] = &[0, 1, 2, 0, 2, 3];

        let quad_vbo = ctx.create_buffer(renderer::VERTEX_BUFFER);
        let quad_ibo = ctx.create_buffer(renderer::INDEX_BUFFER);

        ctx.upload_buffer(
            quad_vbo,
            renderer::VERTEX_BUFFER,
            renderer::STATIC,
            &Pos2DTex2D::dynamically_typed_slice(quad_vertices)
        ).ok().expect("vbo upload");

        ctx.upload_buffer(
            quad_ibo,
            renderer::INDEX_BUFFER,
            renderer::STATIC,
            &index_buffer_slice(quad_indices)
        ).ok().expect("ibo upload");

        let geom = ctx.create_geometry([
            renderer::VertexAttribute {
                buffer: quad_vbo,
                attrib_type: data::VEC2,
                location: a_position,
                stride: 16,
                offset: 0,
                normalize: false,
            },
            renderer::VertexAttribute {
                buffer: quad_vbo,
                attrib_type: data::VEC2,
                location: a_tex_coords,
                stride: 16,
                offset: 8,
                normalize: false,
            }],
            Some(quad_ibo)
        ).ok().expect("geom creation");

        let text = "vodk! - Hello World\n--test--";
        let mut text_vertices = Vec::from_fn(
            text.len()*24,
            |_|{ 0.0 as f32 }
        );
        text::text_to_vertices(text,
            pixels::vec2(400.0, 300.0),
            pixels::vec2(32.0, 32.0),
            pixels::vec2(-10.0, 10.0),
            texels::rect(0.0, 0.0, 1.0, 1.0),
            16, 8, // vertex stride in bytes, tex coords offset in bytes
            text_vertices.as_mut_slice()
        );
        let text_vbo = ctx.create_buffer(renderer::VERTEX_BUFFER);
        ctx.upload_buffer(
            text_vbo,
            renderer::VERTEX_BUFFER,
            renderer::STATIC,
            &data::DynamicallyTypedSlice::new(text_vertices.as_mut_slice(), &[data::VEC2, data::VEC2])
        ).ok().expect("text vbo upload");

        let text_geom = ctx.create_geometry([
            renderer::VertexAttribute {
                buffer: text_vbo,
                attrib_type: data::VEC2,
                location: a_position,
                stride: 4*4,
                offset: 0,
                normalize: false,
            },
            renderer::VertexAttribute {
                buffer: text_vbo,
                attrib_type: data::VEC2,
                location: a_tex_coords,
                stride: 4*4,
                offset: 2*4,
                normalize: false,
            }],
            None
        ).ok().expect("text geom creation");

        let ascii_atlas = match png::load_png(&Path::new("assets/ascii_atlas.png")) {
            Ok(img) => img,
            Err(e) => fail!("Failed to load the ascii atlas image {}", e)
        };

        let ascii_tex = ctx.create_texture(renderer::REPEAT|renderer::FILTER_LINEAR);
        ctx.upload_texture_data(
            ascii_tex,
            &dynamically_typed_byte_slice(ascii_atlas.pixels.as_slice()),
            ascii_atlas.width, ascii_atlas.height,
            renderer::R8G8B8A8
        ).ok().expect("Ascii atlas texture upload");

        let checker = create_checker_texture(10, 10, ctx);

        self.shaders.push(setup_shader(ctx,
            shaders::BASIC_VERTEX_SHADER_2D,
            shaders::TEXTURED_FRAGMENT_SHADER
        ));
        self.draw_calls.push(
            DrawCall {
                geom: geom,
                first: 0, count: 6,
                flags: renderer::TRIANGLES,
                targets: renderer::COLOR
            }
        );

        self.shaders.push(setup_shader(ctx,
            shaders::BASIC_VERTEX_SHADER_2D,
            shaders::TEXT_FRAGMENT_SHADER
        ));
        self.draw_calls.push(
            DrawCall {
                geom: text_geom,
                first: 0, count: (text.len()*6) as u32,
                flags: renderer::TRIANGLES,
                targets: renderer::COLOR
            }
        );

        self.shaders.push(setup_shader(ctx,
            shaders::BASIC_VERTEX_SHADER_3D,
            shaders::NORMALS_FRAGMENT_SHADER
        ));
        self.draw_calls.push(
            DrawCall {
                geom: cube_geom,
                first: 0,
                count: cube_indices.len() as u32,
                flags: renderer::TRIANGLES,
                targets: renderer::COLOR|renderer::DEPTH,
            }
        );

        self.textures.push(checker);
        self.textures.push(ascii_tex);
    }
}

fn setup_shader(
    ctx: &mut renderer::RenderingContext,
    vs_src: &str,
    fs_src: &str
) -> (renderer::ShaderProgram, UniformLayout) {
    let vs = ctx.create_shader(renderer::VERTEX_SHADER);
    let fs = ctx.create_shader(renderer::FRAGMENT_SHADER);
    let program = ctx.create_shader_program();

    ctx.compile_shader(vs, &[vs_src]).map_err(
        |e| { fail!("Failed to compile the vertex shader: {}", e); return; }
    );

    ctx.compile_shader(fs, &[fs_src]).map_err(
        |e| { fail!("Failed to compile the fragment shader: {}", e); return; }
    );

    ctx.link_shader_program(program, [vs, fs], &[
        ("a_position", a_position),
        ("a_normal", a_normal),
        ("a_tex_coords", a_tex_coords),
        ("a_color", a_color),
        ("a_extrusion", a_extrusion),
    ]).map_err(
        |e| { fail!("Failed to link the text's shader program: {}", e); return; }
    );

    let uniforms = UniformLayout::new(ctx, program);
    ctx.destroy_shader(vs);
    ctx.destroy_shader(fs);
    return (program, uniforms);
}

fn main() {
    std::io::println("vodk!");

    scenegraph::test_cow_scene_graph();

    let mut window = gfx::window::Window::create(800, 600, "vodk");
    let ctx = window.create_rendering_context();
    let mut app = TestApp::new(&mut window, ctx);
    app.init();
    let app = &mut app as &mut app::Application;

    let mut input_events: Vec<inputs::Event> = Vec::new();

    let mut avg_frame_time: u64 = 0;
    let mut frame_count: u64 = 0;
    let mut previous_time = time::precise_time_ns();
    let mut i = 0;
    while !window.should_close() {
        input_events.clear();
        window.poll_events(&mut input_events);
        app.handle_events(input_events.as_slice());
        let frame_start_time = time::precise_time_ns();
        let elapsed_time = frame_start_time - previous_time;

        app.update(elapsed_time as f32 / 1000000.0 , i);

        i+=1;
        previous_time = frame_start_time;
        let frame_time = time::precise_time_ns() - frame_start_time;
        frame_count += 1;
        avg_frame_time += frame_time;

        if frame_count % 60 == 0 {
            println!("avg frame time: {}ms", avg_frame_time as f64/(60.0*1000000.0));
            avg_frame_time = 0;
        }

        let sleep_time: i64 = 16000000 - frame_time as i64;
        if sleep_time > 0 {
            sleep(sleep_time as u64/1000000 );
        }
    }

    app.shut_down();
}

fn create_checker_texture(w: uint, h: uint, ctx: &mut renderer::RenderingContext) -> renderer::Texture {
    let checker_data: Vec<u8> = Vec::from_fn(w*h*4, |i|{
        (((i / 4 + (i/(4*h))) % 2)*255) as u8
    });
    let checker = ctx.create_texture(renderer::REPEAT|renderer::FILTER_NEAREST);
    ctx.upload_texture_data(
        checker,
        &dynamically_typed_byte_slice(checker_data.as_slice()),
        w as u32, h as u32,
        renderer::R8G8B8A8
    ).ok().expect("checker texture upload");
    return checker;
}
