use gl;
use glfw;
use glfw::Context;
use gfx::opengl;
use gpu = gfx::renderer;
use gfx::shaders;

pub fn main_loop() {

    let glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    // Choose a GL profile that is compatible with OS X 10.7+
    glfw.window_hint(glfw::ContextVersion(3, 0));
    glfw.window_hint(glfw::OpenglForwardCompat(true));
    //glfw.window_hint(glfw::OpenglProfile(glfw::OpenGlCoreProfile));

    let (window, _) = glfw.create_window(800, 600, "OpenGL", glfw::Windowed)
        .expect("Failed to create GLFW window.");

    // It is essential to make the context current before calling gl::load_with.
    window.make_current();

    // Load the OpenGL function pointers
    gl::load_with(|s| glfw.get_proc_address(s));

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

    let mut ctx = ~opengl::RenderingContextGL::new() as ~gpu::RenderingContext;
    ctx.set_clear_color(1.0, 0.0, 0.0, 1.0);

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
        //glfw.poll_events();

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
}

