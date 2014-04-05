use gl;
use glfw;
use gfx::opengl;
use gfx::renderer;
use gfx::shaders;
use gfx::mesh_utils::{
    generate_grid_indices, generate_grid_vertices, num_indices_for_grid, num_vertices_for_grid,
    PER_GRID_INDICES,
};
use std::rc::Rc;
use std::libc;

pub fn main_loop() {
    glfw::set_error_callback(~ErrorContext);

    glfw::start(proc() {
        glfw::window_hint(glfw::Resizable(true));

        glfw::window_hint(glfw::ContextVersion(3, 0));
        glfw::window_hint(glfw::OpenglForwardCompat(true));
        glfw::window_hint(glfw::OpenglDebugContext(true));

        let window = glfw::Window::create(800, 600, "vodk.", glfw::Windowed).unwrap();

        //let window = window_rc.borrow();
        window.set_sticky_keys(true);

        // Polling of events can be turned on and off by the specific event type
        window.set_pos_polling(true);
        window.set_all_polling(true);
        window.set_size_polling(true);
        window.set_close_polling(true);
        window.set_refresh_polling(true);
        window.set_focus_polling(true);
        window.set_iconify_polling(true);
        window.set_framebuffer_size_polling(true);
        window.set_key_polling(true);
        window.set_char_polling(true);
        window.set_mouse_button_polling(true);
        window.set_cursor_pos_polling(true);
        window.set_cursor_enter_polling(true);
        window.set_scroll_polling(true);

        // Alternatively, all event types may be set to poll at once. Note that
        // in this example, this call is redundant as all events have been set
        // to poll in the above code.
        window.set_all_polling(true);

        window.make_context_current();
        gl::load_with(glfw::get_proc_address);

        let mut ctx = ~opengl::RenderingContextGL::new() as ~renderer::RenderingContext;
        ctx.set_clear_color(1.0, 0.0, 0.0, 1.0);

        // TODO move into RenderingContext
        window.make_context_current();

        let grid_w: u32 = 32;
        let grid_h: u32 = 32;
        let vertex_stride = 5;
        let mut grid_vertices = Vec::from_fn( (grid_w*grid_h*vertex_stride) as uint, |idx|{ 0.0f32 });
        let mut grid_indices = Vec::from_fn(
            num_indices_for_grid(grid_w, grid_h, PER_GRID_INDICES) as uint,
            |idx|{ 0u32 }
        );

        generate_grid_indices(grid_w, grid_h, PER_GRID_INDICES, 
                              grid_indices.as_mut_slice(),
                              0, vertex_stride);
        generate_grid_vertices(grid_w, grid_h, grid_vertices.as_mut_slice(),
                               vertex_stride as uint, true,
            |x, y, vertex| {
                vertex[0] = x as f32;
                vertex[1] = y as f32;
                vertex[2] = 0.0;
                // tex coordinates
                vertex[3] = x as f32 / grid_w as f32;
                vertex[4] = x as f32 / grid_h as f32;
            }
        );


        let vertices : ~[f32] = ~[
            0.0, 0.0,
            1.0, 0.0,
            1.0, 1.0,
            0.0, 0.0,
            1.0, 1.0,
            0.0, 1.0,
        ];
        let quad = ctx.create_vertex_buffer();

        ctx.upload_vertex_data(quad, vertices, renderer::STATIC_UPDATE).map_err(
            |e| { fail!("Failed to upload the vertex data: {}", e); return; }
        );

        let geom = ctx.create_geometry();
        ctx.define_geometry(geom, [
            renderer::VertexAttribute {
                buffer: quad,
                attrib_type: renderer::F32,
                components: 2,
                location: 0,
                stride: 0,
                offset: 0,
                normalize: false,
            }
        ], None).map_err(|e| { fail!("{}", e); return; } );

        let vs = ctx.create_shader(renderer::VERTEX_SHADER);
        let fs = ctx.create_shader(renderer::FRAGMENT_SHADER);

        ctx.compile_shader(vs, shaders::BASIC_VERTEX_SHADER).map_err(
            |e| { fail!("Failed to compile the vertex shader: {}", e); return; }
        );

        ctx.compile_shader(fs, shaders::TEX_COORDS_FRAGMENT_SHADER).map_err(
            |e| { fail!("Failed to compile the fragment shader: {}", e); return; }
        );

        let program = ctx.create_shader_program();
        ctx.link_shader_program(program, [vs, fs]).map_err(
            |e| { fail!("Failed to link the shader program: {}", e); return; }
        );

        let cmd = ~[renderer::OpDraw(
            renderer::DrawCommand {
                target: ctx.get_default_render_target(),
                flags: 0,
                geometry: geom,
                shader_program: program,
                shader_inputs: ~[
                    renderer::ShaderInput {
                        location: ctx.get_shader_input_location(program, "u_color"),
                        value: renderer::INPUT_FLOATS(~[0.0, 0.5, 1.0, 1.0])
                    }
                ],
                first: 0,
                count: 6,
            }
        )];

        let texture_1 = ctx.create_texture(renderer::TEXTURE_FLAGS_DEFAULT);
        ctx.allocate_texture(texture_1, 512, 512, renderer::FORMAT_R8G8B8A8);
        let intermediate_target = ctx.create_render_target([texture_1], None, None).map_err(
            |e| {
                fail!("Failed to ceate a render target: {} - {}",
                      ctx.get_error_str(e.code),
                      match e.detail {
                        Some(s) => s,
                        None => ~"",
                      });
                return;
            }
        );

        while !window.should_close() {
            glfw::poll_events();
            for event in window.flush_events() {
                handle_window_event(&window, event);
            }
            ctx.clear();

            ctx.render(cmd).map_err(
                |e| { fail!("Rendering error: {}", e); return; }
            );

            window.swap_buffers();
        }

        ctx.destroy_shader(vs);
        ctx.destroy_shader(fs);
        ctx.destroy_shader_program(program);
        ctx.destroy_geometry(geom);
    });
}

struct ErrorContext;
impl glfw::ErrorCallback for ErrorContext {
    fn call(&self, _: glfw::Error, description: ~str) {
        println!("GLFW Error: {}", description);
    }
}

fn handle_window_event(window: &glfw::Window, (time, event): (f64, glfw::WindowEvent)) {
    match event {
        glfw::PosEvent(x, y)                => window.set_title(format!("Time: {}, Window pos: ({}, {})", time, x, y)),
        glfw::SizeEvent(w, h)               => window.set_title(format!("Time: {}, Window size: ({}, {})", time, w, h)),
        glfw::CloseEvent                    => println!("Time: {}, Window close requested.", time),
        glfw::RefreshEvent                  => println!("Time: {}, Window refresh callback triggered.", time),
        glfw::FocusEvent(true)              => println!("Time: {}, Window focus gained.", time),
        glfw::FocusEvent(false)             => println!("Time: {}, Window focus lost.", time),
        glfw::IconifyEvent(true)            => println!("Time: {}, Window was minimised", time),
        glfw::IconifyEvent(false)           => println!("Time: {}, Window was maximised.", time),
        glfw::FramebufferSizeEvent(w, h)    => println!("Time: {}, Framebuffer size: ({}, {})", time, w, h),
        glfw::CharEvent(character)          => println!("Time: {}, Character: {}", time, character),
        glfw::MouseButtonEvent(btn, action, mods) => println!("Time: {}, Button: {}, Action: {}, Modifiers: [{}]", time, btn, action, mods),
        glfw::CursorPosEvent(xpos, ypos)    => window.set_title(format!("Time: {}, Cursor position: ({}, {})", time, xpos, ypos)),
        glfw::CursorEnterEvent(true)        => println!("Time: {}, Cursor entered window.", time),
        glfw::CursorEnterEvent(false)       => println!("Time: {}, Cursor left window.", time),
        glfw::ScrollEvent(x, y)             => window.set_title(format!("Time: {}, Scroll offset: ({}, {})", time, x, y)),
        glfw::KeyEvent(key, scancode, action, mods) => {
            println!("Time: {}, Key: {}, ScanCode: {}, Action: {}, Modifiers: [{}]", time, key, scancode, action, mods);
            match (key, action) {
                (glfw::KeyEscape, glfw::Press) => window.set_should_close(true),
                (glfw::KeyR, glfw::Press) => {
                    // Resize should cause the window to "refresh"
                    let (window_width, window_height) = window.get_size();
                    window.set_size(window_width + 1, window_height);
                    window.set_size(window_width, window_height);
                }
                _ => {}
            }
        }
    }
}