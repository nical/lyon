#![feature(macro_rules, globs)]

extern crate glfw;
extern crate gl;
extern crate gpu;
extern crate data;
extern crate time;

use data::*;
use gpu::device::*;
use gpu::constants::*;
use gpu::opengl;

use std::io::timer::sleep;
use std::time::duration::Duration;
use glfw::Context;

#[deriving(Show, PartialEq)]
struct Vertex {
    x: f32,
    y: f32,
    z: f32,
    u: f32,
    v: f32,
}

fn main() {
    let glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    glfw.window_hint(glfw::ContextVersion(3, 1));
    glfw.window_hint(glfw::OpenglForwardCompat(true));

    let (window, events) = glfw.create_window(800, 600, "Triangle test", glfw::Windowed)
        .expect("Failed to create GLFW window.");

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

    // interesting stuff starts here

    let vbo_desc = BufferDescriptor {
        size: 3*5*4,
        buffer_type: VERTEX_BUFFER,
        update_hint: STATIC_UPDATE,
    };

    let vbo = ctx.create_buffer(&vbo_desc).ok().unwrap();

    ctx.with_write_only_mapped_buffer(
        vbo, |mapped_vbo| {
            mapped_vbo[0] = Vertex {
                x: -1.0, y: -1.0, z: 0.0,
                u: 0.0, v: 0.0
            };
            mapped_vbo[1] = Vertex {
                x: 0.0, y: 1.0, z: 0.0,
                u: 0.5, v: 1.0
            };
            mapped_vbo[2] = Vertex {
                x: 1.0, y: -1.0, z: 0.0,
                u: 1.0, v: 0.0
            };
        }
    );

    ctx.with_read_only_mapped_buffer::<Vertex>(
        vbo, |mapped_vbo| {
            assert_eq!(mapped_vbo[0],
                Vertex {
                    x: -1.0, y: -1.0, z: 0.0,
                    u: 0.0, v: 0.0
                }
            );
            assert_eq!(mapped_vbo[1],
                Vertex {
                    x: 0.0, y: 1.0, z: 0.0,
                    u: 0.5, v: 1.0
                }
            );
            assert_eq!(mapped_vbo[2],
                Vertex {
                    x: 1.0, y: -1.0, z: 0.0,
                    u: 1.0, v: 0.0
                }
            );
        }
    );

    let a_position = VertexAttributeLocation { index: 0 };
    let a_uv = VertexAttributeLocation { index: 1 };

    let geom_desc = GeometryDescriptor {
        attributes: &[
            VertexAttribute {
                buffer: vbo,
                attrib_type: VEC3,
                location: a_position,
                stride: 20,
                offset: 0,
                normalize: false
            },
            VertexAttribute {
                buffer: vbo,
                attrib_type: VEC2,
                location: a_uv,
                stride: 20,
                offset: 12,
                normalize: false
            },
        ],
        index_buffer: None
    };

    let geom = ctx.create_geometry(&geom_desc).ok().unwrap();

    let vertex_stage_desc = ShaderStageDescriptor {
        stage_type: VERTEX_SHADER,
        src: &[shaders::BASIC_VERTEX]
    };

    let vertex_shader = ctx.create_shader_stage(&vertex_stage_desc).ok().unwrap();

    match ctx.get_shader_stage_result(vertex_shader) {
        Err((_code, msg)) => {
            fail!(
                "{}\nshader build failed - {}\n",
                shaders::BASIC_VERTEX, msg
            );
        }
        _ => {}
    }

    let fragment_stage_desc = ShaderStageDescriptor {
        stage_type: FRAGMENT_SHADER,
        src: &[shaders::BASIC_FRAGMENT]
    };
    let fragment_shader = ctx.create_shader_stage(&fragment_stage_desc).ok().unwrap();
    match ctx.get_shader_stage_result(fragment_shader) {
        Err((_code, msg)) => {
            fail!(
                "{}\nshader build failed - {}\n",
                shaders::BASIC_FRAGMENT, msg
            );
        }
        _ => {}
    }

    let pipeline_desc = ShaderPipelineDescriptor {
        stages: &[
            vertex_shader,
            fragment_shader
        ],
        attrib_locations: &[
            ("a_position", a_position),
            ("a_uv", a_uv)
        ]
    };
    let pipeline = ctx.create_shader_pipeline(&pipeline_desc).ok().unwrap();
    
    match ctx.get_shader_pipeline_result(pipeline) {
        Err((_code, msg)) => {
            fail!("Pipline link failed - {}\n", msg);
        }
        _ => {}
    }

    ctx.set_clear_color(0.9, 0.9, 0.9, 1.0);
    ctx.set_viewport(0, 0, 800, 600);

    let dyn_ubo_desc = BufferDescriptor {
        buffer_type: UNIFORM_BUFFER,
        update_hint: DYNAMIC_UPDATE,
        size: 4,
    };

    let dyn_ubo = ctx.create_buffer(&dyn_ubo_desc).ok().unwrap();
    ctx.with_write_only_mapped_buffer(
        dyn_ubo, |mapped_ubo| { mapped_ubo[0] = 0.5f32; }
    );

    let ubo_binding_index = 0;
    ctx.bind_uniform_buffer(ubo_binding_index, dyn_ubo, None);
    let u_dynamic = ctx.get_uniform_block_location(pipeline, "u_dynamic");
    assert!(u_dynamic.index >= 0);
    ctx.set_uniform_block(pipeline, u_dynamic, ubo_binding_index);


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
        ctx.with_write_only_mapped_buffer(
            dyn_ubo, |mapped_ubo| { mapped_ubo[0] = time * 0.000000001; }
        );

        ctx.clear(COLOR);
        ctx.set_shader(pipeline);
        ctx.draw(geom, VertexRange(0, 3), TRIANGLES, NO_BLENDING, COLOR);
        ctx.flush();

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
    pub const BASIC_VERTEX: &'static str = "
        attribute vec3 a_position;
        attribute vec2 a_uv;
        varying vec2 v_uv;
        void main() {
            gl_Position = vec4(a_position, 1.0);
            v_uv = a_uv;
        }
    ";
    pub const BASIC_FRAGMENT: &'static str = "
        #version 150
        layout(std140)
        uniform u_dynamic {
            float time;
        };
        varying vec2 v_uv;
        void main() {
            gl_FragColor = vec4(v_uv, 0.5+0.5*sin(time), 1.0);
        }
    ";
}