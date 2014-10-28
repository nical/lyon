#![feature(macro_rules, globs)]

extern crate glfw;
extern crate gl;
extern crate gpu;
extern crate data;
extern crate time;


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

    let commands = [
        SetClearColor(0.9, 0.9, 0.9, 1.0),
        Clear(COLOR),
        Flush,
    ];

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


    while !window.should_close() {
        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            // handle events
        }

        let frame_start_time = time::precise_time_ns();
        let elapsed_time = frame_start_time - previous_time;

        ctx.execute_command_list(commands);
        assert!(res == OK);

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
