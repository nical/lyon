#![feature(macro_rules, globs)]

extern crate glfw;
extern crate gl;
extern crate gpu;
extern crate data;
extern crate time;
extern crate math;

use data::*;
use gpu::device::*;
use gpu::constants::*;
use gpu::opengl;

use std::io::timer::sleep;
use std::time::duration::Duration;
use glfw::Context;

use math::units::world;
use math::vector;

use std::num::FloatMath;

struct TransformsBlock {
  model: world::Mat4,
  view: world::Mat4,
  projection: world::Mat4,
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

    let vbo_desc = BufferDescriptor {
        size: 8*4*4*6,
        buffer_type: BufferType::VERTEX,
        update_hint: UpdateHint::STATIC,
    };

    let ibo_desc = BufferDescriptor {
        size: 8*4*4*6,
        buffer_type: BufferType::INDEX,
        update_hint: UpdateHint::STATIC,
    };

    let vbo = ctx.create_buffer(&vbo_desc).ok().unwrap();
    let ibo = ctx.create_buffer(&ibo_desc).ok().unwrap();

    ctx.with_write_only_mapped_buffer(
      vbo, |mapped_vbo| {
          for i in range(0, cube_vertices.len()) {
            mapped_vbo[i] = cube_vertices[i];
          }
      }
    );

    ctx.with_write_only_mapped_buffer(
      ibo, |mapped_ibo| {
          for i in range(0, cube_indices.len()) {
            mapped_ibo[i] = cube_indices[i];
          }
      }
    );

    let a_position = VertexAttributeLocation { index: 0 };
    let a_normal = VertexAttributeLocation { index: 1 };
    let a_tex_coords = VertexAttributeLocation { index: 2 };

    let geom_desc = GeometryDescriptor{
      attributes: &[
        VertexAttribute {
            buffer: vbo,
            attrib_type: data::VEC3,
            location: a_position,
            stride: 32,
            offset: 0,
            normalize: false,
        },
        VertexAttribute {
            buffer: vbo,
            attrib_type: data::VEC3,
            location: a_normal,
            stride: 32,
            offset: 12,
            normalize: false,
        },
        VertexAttribute {
            buffer: vbo,
            attrib_type: data::VEC2,
            location: a_tex_coords,
            stride: 32,
            offset: 24,
            normalize: false,
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
        Err((_code, msg)) => {
            panic!("{}\nshader build failed - {}\n", shaders::VERTEX, msg);
        }
        _ => {}
    }

    let fragment_stage_desc = ShaderStageDescriptor {
        stage_type: ShaderType::FRAGMENT_SHADER,
        src: &[shaders::FRAGMENT]
    };
    let fragment_shader = ctx.create_shader_stage(&fragment_stage_desc).ok().unwrap();
    match ctx.get_shader_stage_result(fragment_shader) {
        Err((_code, msg)) => {
            panic!("{}\nshader build failed - {}\n", shaders::FRAGMENT, msg);
        }
        _ => {}
    }

    let pipeline_desc = ShaderPipelineDescriptor {
        stages: &[vertex_shader, fragment_shader],
        attrib_locations: &[
            ("a_position", a_position),
            ("a_normal", a_normal),
            ("a_uv_tex_coords", a_tex_coords),
        ]
    };

    let pipeline = ctx.create_shader_pipeline(&pipeline_desc).ok().unwrap();

    match ctx.get_shader_pipeline_result(pipeline) {
        Err((_code, msg)) => {
            panic!("Shader link failed - {}\n", msg);
        }
        _ => {}
    }

    ctx.set_clear_color(0.9, 0.9, 0.9, 1.0);
    ctx.set_viewport(0, 0, win_width as i32, win_height as i32);

    let ubo_desc = BufferDescriptor {
        buffer_type: BufferType::UNIFORM,
        update_hint: UpdateHint::DYNAMIC,
        size: 4*16*3,
    };

    let ubo = ctx.create_buffer(&ubo_desc).ok().unwrap();

    let ubo_binding_index = 0;
    ctx.bind_uniform_buffer(ubo_binding_index, ubo, None);
    let u_transforms = ctx.get_uniform_block_location(pipeline, "u_transforms");
    assert!(u_transforms.index >= 0);
    ctx.set_uniform_block(pipeline, u_transforms, ubo_binding_index);

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
        ctx.with_write_only_mapped_buffer::<TransformsBlock>(
            ubo, |mapped_ubo| {
                mapped_ubo[0].projection = world::Mat4::perspective(
                    45.0,
                    win_width as f32 / win_height as f32,
                    0.5,
                    1000.0,
                );
                let view = &mut mapped_ubo[0].view;
                *view = world::Mat4::identity();
                view.translate(&world::vec3(0.0, 0.0, -10.0));
                view.rotate(
                    vector::PI * (time * 0.000000001).sin(),
                    &world::vec3(0.0, 1.0, 0.0)
                );
                mapped_ubo[0].model = world::Mat4::identity();
            }
        );

        ctx.clear(COLOR|DEPTH);
        ctx.set_shader(pipeline);
        ctx.draw(
            geom,
            Range::IndexRange(0, cube_indices.len() as u16),
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
layout(std140)
uniform u_transforms {
  mat4 model;
  mat4 view;
  mat4 projection;
};
attribute vec3 a_position;
attribute vec3 a_normal;
attribute vec2 a_tex_coords;
varying vec3 v_normal;
varying vec2 v_tex_coords;
void main() {
    v_tex_coords = a_tex_coords;
    v_normal = a_normal;
    gl_Position = projection
                * view
                * model
                * vec4(a_position, 1.0);
}
";

pub static FRAGMENT: &'static str = "
varying vec3 v_normal;
varying vec2 v_tex_coords;
void main() {
    vec3 normals = v_normal * 0.5 + vec3(0.5, 0.5, 0.5);
    gl_FragColor = vec4(normals, 1.0);
}
";
}
