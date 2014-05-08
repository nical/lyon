use gl;
use glfw;
use glfw::Context;
use gfx::opengl;
use gfx::renderer;
use gfx::shaders;

use time;
use std::io::timer::sleep;

pub fn main_loop() {

    let glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    glfw.window_hint(glfw::ContextVersion(3, 1));
    glfw.window_hint(glfw::OpenglForwardCompat(true));

    let (window, _) = glfw.create_window(800, 600, "OpenGL", glfw::Windowed)
        .expect("Failed to create GLFW window.");

    // It is essential to make the context current before calling gl::load_with.
    window.make_current();

    // Load the OpenGL function pointers
    gl::load_with(|s| glfw.get_proc_address(s));

    let mut ctx = ~opengl::RenderingContextGL::new() as ~renderer::RenderingContext;
    ctx.set_clear_color(1.0, 0.0, 0.0, 1.0);

    let vertices : &[f32] = &[ 0.0, 0.0,  1.0, 0.0,  1.0, 1.0,  0.0, 1.0 ];
    let indices : &[u16] = &[0, 1, 2, 0, 2, 3];

    let vertices2 : &[f32] = &[ 0.0, 0.0,  1.0, 0.0,  1.0, 1.0,  
                                0.0, 0.0,  1.0, 1.0,  0.0, 1.0 ];

    let quad_vertices = ctx.create_buffer();
    let quad_indices = ctx.create_buffer();

    ctx.upload_buffer(quad_vertices, renderer::VERTEX_BUFFER, renderer::STATIC, 
                      renderer::as_bytes(vertices)).map_err(
        |e| { fail!("Failed to upload the vertex data: {}", e); return; }
    );
    ctx.upload_buffer(quad_indices, renderer::INDEX_BUFFER, renderer::STATIC,
                      renderer::as_bytes(indices)).map_err(
        |e| { fail!("Failed to upload the vertex data: {}", e); return; }
    );

    let geom_res = ctx.create_geometry([
        renderer::VertexAttribute {
            buffer: quad_vertices,
            attrib_type: renderer::F32,
            components: 2,
            location: 0,
            stride: 0,
            offset: 0,
            normalize: false,
        }
    ], Some(quad_indices));

    let geom = match geom_res {
        Ok(g) => g,
        Err(e) => fail!("Failed to create a Geometry object: {}", e),
    };

    let vs = ctx.create_shader(renderer::VERTEX_SHADER);
    let fs = ctx.create_shader(renderer::FRAGMENT_SHADER);
    let program = ctx.create_shader_program();

    ctx.compile_shader(fs, shaders::TEXTURED_FRAGMENT_SHADER).map_err(
        |e| { fail!("Failed to compile the fragment shader: {}", e); return; }
    );

    ctx.compile_shader(vs, shaders::BASIC_VERTEX_SHADER).map_err(
        |e| { fail!("Failed to compile the vertex shader: {}", e); return; }
    );

    ctx.link_shader_program(program, [vs, fs], None).map_err(
        |e| { fail!("Failed to link the shader program: {}", e); return; }
    );

    let u_color = ctx.get_shader_input_location(program, "u_color");
    let u_texture_0 = ctx.get_shader_input_location(program, "u_texture_0");
    println!("u_color: {}, u_texture_0: {}", u_color, u_texture_0);

    let w = 32;
    let h = 32;
    let checker_data : Vec<u8> = Vec::from_fn(w*h*4, |i|{
        (((i / 4 + (i/(4*h))) % 2)*255) as u8
    });
    let checker = ctx.create_texture(renderer::REPEAT|renderer::FILTER_NEAREST);
    ctx.upload_texture_data(checker, checker_data.as_slice(), w as u32, h as u32, renderer::R8G8B8A8);

    let screen = ctx.get_default_render_target();

    let mut avg_frame_time : u64 = 0;
    let mut frame_count : u64 = 0;
    let mut previous_time = time::precise_time_ns();
    while !window.should_close() {
        //glfw.poll_events();
        let frame_start_time = time::precise_time_ns();
        let elapsed_time = frame_start_time - previous_time;

        ctx.set_render_target(screen);

        ctx.clear(renderer::COLOR);

        ctx.set_shader(program);

        ctx.set_shader_input_float(u_color, [0.0, 0.5, 1.0, 1.0]);
        ctx.set_shader_input_texture(u_texture_0, 0, checker);

        ctx.draw(
            renderer::GeometryRange {
                geometry: geom,
                from: 0,
                to: 6,
                flags: renderer::TRIANGLES
            },
            renderer::COLOR
        ).map_err(
            |e| { fail!("Rendering error: {}", e); return; }
        );

        window.swap_buffers();

        previous_time = frame_start_time;
        
        let frame_time = time::precise_time_ns() - frame_start_time;
        frame_count += 1;
        avg_frame_time += frame_time;

        if (frame_count % 60 == 0) {
            println!("avg frame time: {}ms", avg_frame_time/(60*1000000));
            avg_frame_time = 0;
        }
        // glfw is already throttling to 60fps for us
        // let sleep_time : i64 = 16000000 - frame_time as i64;
        // if (sleep_time > 0) {
        //     sleep(sleep_time as u64/1000000 );
        // }
    }

    ctx.destroy_geometry(geom);
    ctx.destroy_buffer(quad_vertices);
    ctx.destroy_buffer(quad_indices);
    ctx.destroy_shader_program(program);
    ctx.destroy_shader(vs);
    ctx.destroy_shader(fs);
    ctx.destroy_texture(checker);
}

