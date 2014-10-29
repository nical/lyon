#![feature(macro_rules, globs)]

extern crate glfw;
extern crate gl;
extern crate gpu;
extern crate data;
extern crate time;


use data::*;
use gpu::context::*;
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
    gl::load_with(|s| glfw.get_proc_address(s));

    let mut ctx = opengl::create_debug_device(LOG_ERRORS|CRASH_ERRORS);

    let mut avg_frame_time: u64 = 0;
    let mut frame_count: u64 = 0;
    let mut previous_time = time::precise_time_ns();

    let vbo_desc = BufferDescriptor {
        size: 3*5*4,
        buffer_type: VERTEX_BUFFER,
        update_hint: STATIC_UPDATE,
    };

    let ibo_desc = BufferDescriptor {
        size: 3*2,
        buffer_type: INDEX_BUFFER,
        update_hint: STATIC_UPDATE,
    };

    let mut res;

    let mut vbo = BufferObject::new();
    res = ctx.create_buffer(&vbo_desc, &mut vbo);
    assert!(res == OK);

    {
        let mut mapped_vbo: &mut [Vertex] = [];
        res = ctx.map_buffer(vbo, VERTEX_BUFFER, WRITE_MAP, &mut mapped_vbo);
        assert!(res == OK);
     
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

        ctx.unmap_buffer(vbo);
    }

    {
        let mut mapped_vbo: &mut [Vertex] = [];
        res = ctx.map_buffer(vbo, VERTEX_BUFFER, READ_MAP, &mut mapped_vbo);
        assert!(res == OK);
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

        let mut ibo = BufferObject::new();
        res = ctx.create_buffer(&ibo_desc, &mut ibo);
        assert!(res == OK);
        ctx.unmap_buffer(vbo);
    }

    let geom_desc = GeometryDescriptor {
        attributes: &[
            VertexAttribute {
                buffer: vbo,
                attrib_type: VEC3,
                location: 0,
                stride: 20,
                offset: 0,
                normalize: false
            },
            VertexAttribute {
                buffer: vbo,
                attrib_type: VEC2,
                location: 1,
                stride: 20,
                offset: 12,
                normalize: false
            },
        ],
        index_buffer: None
    };
    let mut geom = GeometryObject::new();
    res = ctx.create_geometry(&geom_desc, &mut geom);
    assert_eq!(res, OK);

    let mut vertex_shader = ShaderStageObject::new();
    let mut fragment_shader = ShaderStageObject::new();

    let vertex_stage_desc = ShaderStageDescriptor {
        stage_type: VERTEX_SHADER,
        src: &[shaders::vs::BASIC_VERTEX]
    };

    let mut shader_result = ShaderBuildResult::new();
    res = ctx.create_shader_stage(&vertex_stage_desc, &mut vertex_shader);
    assert_eq!(res, OK);
    if ctx.get_shader_stage_result(vertex_shader, &mut shader_result) != OK {
        fail!(
            "{}\nshader build failed with error {}\n",
            shaders::vs::BASIC_VERTEX,
            shader_result.details
        );
    }

    res = ctx.create_shader_stage(&vertex_stage_desc, &mut vertex_shader);
    assert_eq!(res, OK);
    if ctx.get_shader_stage_result(vertex_shader, &mut shader_result) != OK {
        fail!(
            "{}\nshader build failed with error {}\n",
            shaders::fs::BASIC_FRAGMENT,
            shader_result.details
        );
    }

    let fragment_stage_desc = ShaderStageDescriptor {
        stage_type: FRAGMENT_SHADER,
        src: &[shaders::fs::BASIC_FRAGMENT]
    };
    res = ctx.create_shader_stage(&fragment_stage_desc, &mut fragment_shader);
    assert_eq!(res, OK);

    let pipeline_desc = ShaderPipelineDescriptor {
        stages: &[
            vertex_shader,
            fragment_shader
        ],
        attrib_locations: &[
            ("a_position", 0),
            ("a_uv", 1)
        ]
    };
    let mut pipeline = ShaderPipelineObject::new();
    res = ctx.create_shader_pipeline(
        &pipeline_desc,
        &mut pipeline
    );
    assert_eq!(res, OK);
    
    if ctx.get_shader_pipeline_result(pipeline, &mut shader_result) != OK {
        fail!("Pipline link failed with error {}\n", shader_result.details);
    }

    ctx.set_clear_color(0.9, 0.9, 0.9, 1.0);
    ctx.set_viewport(0, 0, 800, 600);

    while !window.should_close() {
        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            // handle events
        }

        let frame_start_time = time::precise_time_ns();
        let elapsed_time = frame_start_time - previous_time;

        res = ctx.clear(COLOR);
        assert_eq!(res, OK);
        res = ctx.set_shader(pipeline);
        assert_eq!(res, OK);
        res = ctx.draw(geom, VertexRange(0, 3), TRIANGLES, NO_BLENDING, COLOR);
        assert_eq!(res, OK);
        res = ctx.flush();
        assert_eq!(res, OK);

        window.swap_buffers();

        previous_time = frame_start_time;
        let frame_time = time::precise_time_ns() - frame_start_time;
        frame_count += 1;
        avg_frame_time += frame_time;

        let sleep_time: i64 = 16000000 - frame_time as i64;
        if sleep_time > 0 {
            sleep(Duration::milliseconds(sleep_time/1000000));
        }
    }
}

pub mod shaders {
    pub mod vs {
        pub const BASIC_VERTEX: &'static str = "
            attribute vec3 a_position;
            attribute vec2 a_uv;
            varying vec2 v_uv;
            void main() {
                gl_Position = vec4(a_position, 1.0);
                v_uv = a_uv;
            }
        ";
    }
    pub mod fs {
        pub const BASIC_FRAGMENT: &'static str = "
            varying vec2 v_uv;
            void main() {
                gl_Color = vec4(v_uv, 1.0, 1.0);
            }
        ";
    }
}