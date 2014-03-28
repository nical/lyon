use gl;
use glfw;
use gfx::opengl;
use gpu = gfx::renderer;
use gfx::shaders;
use std::rc::Rc;
use std::libc;

pub fn main_loop() {
    glfw::set_error_callback(~ErrorContext);

    glfw::start(proc() {
        glfw::window_hint(glfw::Resizable(true));

        glfw::window_hint(glfw::ContextVersion(3, 0));
        //glfw::window_hint::opengl_profile(glfw::OpenGlCoreProfile);
        glfw::window_hint(glfw::OpenglForwardCompat(true));
        glfw::window_hint(glfw::OpenglDebugContext(true));

        let window = //Rc::new(
            glfw::Window::create(800, 600, "vodk.", glfw::Windowed).unwrap();
        //);
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

        let mut ctx = ~opengl::RenderingContextGL::new() as ~gpu::RenderingContext;
        ctx.set_clear_color(1.0, 0.0, 0.0, 1.0);

        // TODO move into RenderingContext
        window.make_context_current();

        let vertices : ~[f32] = ~[
            0.0, 0.0,
            1.0, 0.0,
            1.0, 1.0,
            0.0, 0.0,
            1.0, 1.0,
            0.0, 1.0,
        ];
        let quad = ctx.create_vertex_buffer();
        match ctx.check_error() {
            Some(err) => { println!("A error {}", err); }
            None => {}
        }

        ctx.upload_vertex_data(quad, vertices, gpu::STATIC_UPDATE);
        let geom = ctx.create_geometry();
        ctx.define_geometry(geom, [
            gpu::VertexAttribute {
                buffer: quad,
                attrib_type: gpu::F32,
                components: 2,
                location: 0,
                stride: 0,
                offset: 0,
                normalize: false,
            }
        ], None);

        match ctx.check_error() {
            Some(err) => { println!("B rendering error {}", err); }
            None => {}
        }

        let vs = ctx.create_shader(gpu::VERTEX_SHADER);
        let fs = ctx.create_shader(gpu::FRAGMENT_SHADER);

        match ctx.compile_shader(vs, shaders::BASIC_VERTEX_SHADER) {
            Err(e) => { fail!("Failed to compile the vertex shader: {}", e); }
            _ => {}
        }
        match ctx.compile_shader(fs, shaders::SOLID_COLOR_FRAGMENT_SHADER) {
            Err(e) => { fail!("Failed to compile the fragment shader: {}", e); }
            _ => {}
        }

        let program = ctx.create_shader_program();
        match ctx.link_shader_program(program, [vs, fs]) {
            Err(e) => { fail!("Failed to link the shader program: {}", e); }
            _ => {}
        }

        let cmd = ~[gpu::OpDraw(
            gpu::DrawCommand {
                target: ctx.get_default_render_target(),
                mode: gpu::TRIANGLES,
                geometry: geom,
                shader_program: program,
                shader_inputs: ~[
                    gpu::ShaderInput {
                        location: ctx.get_shader_input_location(program, "u_color"),
                        value: gpu::INPUT_FLOATS(~[0.0, 0.5, 1.0, 1.0])
                    }
                ],
                first: 0,
                count: 6,
                use_indices: false,
            }
        )];

        while !window.should_close() {
            glfw::poll_events();
            for event in window.flush_events() {
                handle_window_event(&window, event);
            }
            ctx.clear();
            match ctx.check_error() {
                Some(err) => { println!("rendering error {}", err); }
                None => {}
            }
            ctx.render(cmd);
            match ctx.check_error() {
                Some(err) => { println!("rendering error {}", err); }
                None => {}
            }

            // TODO move into RenderingContext
            window.swap_buffers();
        }
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