#![feature(macro_rules, globs)]

extern crate glfw;
extern crate gl;
extern crate data;
extern crate gpu;
extern crate gfx2d;
extern crate math;
extern crate time;

use data::*;
use gpu::device::*;
use gpu::constants::*;
use gpu::opengl;
use gfx2d::tesselation;
use gfx2d::color::Rgba;

use std::mem;
use std::io::timer::sleep;
use std::time::duration::Duration;
use glfw::Context;

use math::units::world;
use math::units::texels;
use math::vector;

#[deriving(Show)]
struct TransformsBlock {
  model: gpu::std140::Mat3,
  view:  gpu::std140::Mat3,
}

fn to_std_140_mat3<T>(from: &vector::Matrix3x3<T>) -> gpu::std140::Mat3 {
    return gpu::std140::Mat3 {
        _11: from._11, _21: from._21, _31: from._31, _pad1: 0,
        _12: from._12, _22: from._22, _32: from._32, _pad2: 0,
        _13: from._13, _23: from._23, _33: from._33, _pad3: 0,
    }
}

fn main() {
    let glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 1));
    glfw.window_hint(glfw::WindowHint::OpenglForwardCompat(true));

    let win_width = 800;
    let win_height = 600;

    let (window, events) = glfw.create_window(
        win_width, win_height,
        "Cube test",
        glfw::WindowMode::Windowed
    ).expect("Failed to create the window.");

    window.set_size_polling(true);
    window.set_close_polling(true);
    window.set_refresh_polling(true);
    window.set_focus_polling(true);
    window.set_framebuffer_size_polling(true);
    window.set_mouse_button_polling(true);
    window.set_cursor_pos_polling(true);
    window.set_scroll_polling(true);

    window.make_current();
    gl::load_with(|s| window.get_proc_address(s));

    let mut ctx = opengl::create_debug_device(LOG_ERRORS|CRASH_ERRORS);

    let path = &[
        world::vec2(-200.0,  -200.0),
        world::vec2(-200.0,   200.0),
        world::vec2( 200.0,   200.0),
        world::vec2( 200.0,  -200.0),
        world::vec2(  25.0,  -200.0),
        world::vec2(  25.0,  -100.0),
        world::vec2( -25.0,  -100.0),
        world::vec2( -25.0,  -200.0),
    ];

    let n_points = path.len();
    let indices_per_point = 18;
    let vertices_per_point = 4;
    let n_indices = indices_per_point * n_points;

    let vbo_desc = BufferDescriptor {
        size: (n_points * vertices_per_point *
              mem::size_of::<tesselation::Pos2DNormal2DColorExtrusion>()) as u32,
        buffer_type: BufferType::VERTEX,
        update_hint: UpdateHint::STATIC,
    };

    let ibo_desc = BufferDescriptor {
        size: (mem::size_of::<u16>()  * n_indices) as u32,
        buffer_type: BufferType::INDEX,
        update_hint: UpdateHint::STATIC,
    };

    let vbo = ctx.create_buffer(&vbo_desc).ok().unwrap();
    let ibo = ctx.create_buffer(&ibo_desc).ok().unwrap();

    ctx.with_write_only_mapped_buffer(
      vbo, |mapped_vbo| {
        tesselation::path_to_line_vbo(
            path.as_slice(),
            true,
            tesselation::VERTEX_ANTIALIASING|tesselation::CONVEX_SHAPE,
            |_| { 50.0 },
            |_, ptype| { match ptype {
                tesselation::PointType::Antialias => Rgba { r: 0.0, g: 0.0, b: 0.3, a: 0.0 },
                _ => Rgba { r: 0.0, g: 0.0, b: 0.3, a: 1.0 },
            }},
            world::Mat3::rotation(1.0),
            mapped_vbo
        );
      }
    );

    ctx.with_write_only_mapped_buffer(
      ibo, |mapped_ibo| {
        tesselation::path_to_line_ibo(
            path.len() as u32,
            true,
            tesselation::VERTEX_ANTIALIASING|tesselation::CONVEX_SHAPE,
            0,
            mapped_ibo
        );
      }
    );

    let a_position = VertexAttributeLocation { index: 0 };
    let a_normal = VertexAttributeLocation { index: 1 };
    let a_color = VertexAttributeLocation { index: 2 };
    let a_extrusion = VertexAttributeLocation { index: 3 };

    let stride = mem::size_of::<tesselation::Pos2DNormal2DColorExtrusion>() as u16;
    let geom_desc = GeometryDescriptor{
        attributes: &[
            VertexAttribute {
                buffer: vbo,
                attrib_type: data::VEC2, location: a_position,
                stride: stride, offset: 0, normalize: false,
            },
            VertexAttribute {
                buffer: vbo,
                attrib_type: data::VEC2, location: a_normal,
                stride: stride, offset: 8, normalize: false,
            },
            VertexAttribute {
                buffer: vbo,
                attrib_type: data::VEC4, location: a_color,
                stride: stride, offset: 16, normalize: false,
            },
            VertexAttribute {
                buffer: vbo,
                attrib_type: data::F32, location: a_extrusion,
                stride: stride, offset: 32, normalize: false,
            }
        ],
        index_buffer: Some(ibo)
    };

    let geom = ctx.create_geometry(&geom_desc).ok().unwrap();

    let vertex_stage_desc = ShaderStageDescriptor {
        stage_type: ShaderType::VERTEX_SHADER,
        src: &[shaders::VERTEX]
    };

    let vertex_shader = ctx.create_shader_stage(&vertex_stage_desc).ok().unwrap();
    match ctx.get_shader_stage_result(vertex_shader) {
        Err((_code, msg)) => { panic!("{}\nshader build failed - {}\n", shaders::VERTEX, msg); }
        _ => {}
    }

    let fragment_stage_desc = ShaderStageDescriptor {
        stage_type: ShaderType::FRAGMENT_SHADER,
        src: &[shaders::PIXEL]
    };
    let fragment_shader = ctx.create_shader_stage(&fragment_stage_desc).ok().unwrap();
    match ctx.get_shader_stage_result(fragment_shader) {
        Err((_code, msg)) => { panic!("{}\nshader build failed - {}\n", shaders::PIXEL, msg); }
        _ => {}
    }

    let pipeline_desc = ShaderPipelineDescriptor {
        stages: &[vertex_shader, fragment_shader],
        attrib_locations: &[
            ("a_position", a_position),
            ("a_normal", a_normal),
            ("a_color", a_color),
            ("a_extrusion", a_extrusion),
        ]
    };

    let pipeline = ctx.create_shader_pipeline(&pipeline_desc).ok().unwrap();

    match ctx.get_shader_pipeline_result(pipeline) {
        Err((_code, msg)) => { panic!("Shader link failed - {}\n", msg); }
        _ => {}
    }

    ctx.set_clear_color(0.9, 0.9, 0.9, 1.0);
    ctx.set_viewport(0, 0, win_width as i32, win_height as i32);

    let transforms_ubo_desc = BufferDescriptor {
        buffer_type: BufferType::UNIFORM,
        update_hint: UpdateHint::DYNAMIC,
        size: mem::size_of::<gpu::std140::Mat3>() as u32 * 2,
    };

    let static_ubo_desc = BufferDescriptor {
        buffer_type: BufferType::UNIFORM,
        update_hint: UpdateHint::DYNAMIC,
        size: mem::size_of::<texels::Vec2>() as u32,
    };

    let transforms_ubo = ctx.create_buffer(&transforms_ubo_desc).ok().unwrap();
    ctx.with_write_only_mapped_buffer::<TransformsBlock>(
      transforms_ubo, |mapped_data| {
        mapped_data[0].model = to_std_140_mat3(&world::Mat3::identity());
        mapped_data[0].view = to_std_140_mat3(&world::Mat3::identity());
      }
    );

    let static_ubo = ctx.create_buffer(&static_ubo_desc).ok().unwrap();
    ctx.with_write_only_mapped_buffer::<texels::Vec2>(
      static_ubo, |mapped_data| {
        mapped_data[0].x = win_width as f32;
        mapped_data[0].y = win_height as f32;
      }
    );

    let transforms_binding_index = 0;
    let static_binding_index = 1;

    ctx.bind_uniform_buffer(transforms_binding_index, transforms_ubo, None);
    let u_transforms = ctx.get_uniform_block_location(pipeline, "u_transforms");
    assert!(u_transforms.index >= 0);
    ctx.set_uniform_block(pipeline, u_transforms, transforms_binding_index);

    ctx.bind_uniform_buffer(static_binding_index, static_ubo, None);
    let u_static = ctx.get_uniform_block_location(pipeline, "u_static");
    assert!(u_static.index >= 0);
    ctx.set_uniform_block(pipeline, u_static, static_binding_index);

    ctx.set_shader(pipeline);

    //let mut avg_frame_time: u64 = 0;
    //let mut frame_count: u64 = 0;
    let mut previous_time = time::precise_time_ns();
    let mut time: f32 = 0.0;
    while !window.should_close() {
        glfw.poll_events();
        for (_, _event) in glfw::flush_messages(&events) {
            // handle events
        }

        let frame_start_time = time::precise_time_ns();
        let elapsed_time = frame_start_time - previous_time;
        time += elapsed_time as f32;

        ctx.clear(COLOR|DEPTH);
        ctx.draw(
            geom,
            Range::IndexRange(0, n_indices as u16),
            TRIANGLES, BlendMode::NONE, COLOR|DEPTH
        );

        window.swap_buffers();

        previous_time = frame_start_time;
        let frame_time = time::precise_time_ns() - frame_start_time;
        //frame_count += 1;
        //avg_frame_time += frame_time;

        let sleep_time: i64 = 16000000 - frame_time as i64;
        if sleep_time > 0 {
            sleep(Duration::milliseconds(sleep_time/1000000));
        }
    }
}

pub mod shaders {
pub const VERTEX: &'static str = "
#version 150
attribute vec2 a_position;
attribute vec2 a_normal;
attribute vec4 a_color;
attribute float a_extrusion;

uniform u_static {
    vec2 resolution;
};
uniform u_transforms {
    mat3 model;
    mat3 view;
};

varying vec4 v_color;

void main() {
  mat3 transform = model;// * model;
  float scale = length(transform * vec3(1.0,0.0,0.0));
  vec2 pos = (a_position + a_normal * a_extrusion / scale) / resolution;
  gl_Position = vec4(transform * vec3(pos, 0.0), 1.0);
  v_color = a_color;
}
";

pub static PIXEL: &'static str = "
varying vec4 v_color;
void main() {
    gl_FragColor = v_color;
}
";
}

