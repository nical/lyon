use gl;
use glfw;
use glfw::Context;
use gfx::opengl;
use gpu = gfx::renderer;
use gfx::shaders;

pub fn main_loop() {

    let glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    // Choose a GL profile that is compatible with OS X 10.7+
    glfw.window_hint(glfw::ContextVersion(3, 1));
    glfw.window_hint(glfw::OpenglForwardCompat(true));
    //glfw.window_hint(glfw::OpenglProfile(glfw::OpenGlCoreProfile));

    let (window, _) = glfw.create_window(800, 600, "OpenGL", glfw::Windowed)
        .expect("Failed to create GLFW window.");

    // It is essential to make the context current before calling gl::load_with.
    window.make_current();

    // Load the OpenGL function pointers
    gl::load_with(|s| glfw.get_proc_address(s));

    let mut ctx = ~opengl::RenderingContextGL::new() as ~gpu::RenderingContext;
    ctx.set_clear_color(1.0, 0.0, 0.0, 1.0);

    let vertices : ~[f32] = ~[
        0.0, 0.0,  1.0, 0.0,  1.0, 1.0,
        0.0, 0.0,  1.0, 1.0,  0.0, 1.0,
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
    match ctx.compile_shader(fs, shaders::TEXTURED_FRAGMENT_SHADER) {
        Err(e) => { fail!("Failed to compile the fragment shader: {}", e); }
        _ => {}
    }

    let program = ctx.create_shader_program();
    match ctx.link_shader_program(program, [vs, fs], None) {
        Err(e) => { fail!("Failed to link the shader program: {}", e); }
        _ => {}
    }

    let u_color = ctx.get_shader_input_location(program, "u_color");
    let u_texture_0 = ctx.get_shader_input_location(program, "u_texture_0");
    println!("u_color: {}, u_texture_0: {}", u_color, u_texture_0);

    let texture_0 = ctx.create_texture();
    ctx.allocate_texture(texture_0, gpu::FORMAT_R8G8B8A8, 800, 600);
    ctx.set_texture_flags(texture_0, gpu::TEXTURE_CLAMP|gpu::TEXTURE_FILTER_NEAREST);

    let checker = ctx.create_texture();
    let checker_data : Vec<u8> = Vec::from_fn(64*64*4, |i|{ (((i / 4) % 2)*255) as u8 });
    ctx.set_texture_flags(checker, gpu::TEXTURE_REPEAT|gpu::TEXTURE_FILTER_NEAREST);
    ctx.upload_texture_data(checker, checker_data.as_slice(), gpu::FORMAT_R8G8B8A8, 64, 64);

    let intermediate_target = match ctx.create_render_target([texture_0], None, None) {
        Ok(rt) => rt,
        Err(s) => fail!(s),
    };
    let screen = ctx.get_default_render_target();

    while !window.should_close() {
        //glfw.poll_events();

        ctx.set_render_target(screen);

        ctx.clear();

        ctx.set_shader(program);

        ctx.set_shader_input_float(u_color, [0.0, 0.5, 1.0, 1.0]);
        ctx.set_shader_input_texture(u_texture_0, 0, checker);

        ctx.draw(gpu::TRIANGLES,
            gpu::GeometryRange {
                geometry: geom,
                from: 0,
                to: 6,
                indexed: false,
            }
        );

        match ctx.check_error() {
            Some(err) => { println!("rendering error {}", err); }
            None => {}
        }

        // TODO move into RenderingContext
        window.swap_buffers();
    }
}

