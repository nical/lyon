use gl;
use glfw;
use glfw::Context;
use gfx::opengl;
use gfx::renderer;
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

    let mut ctx = ~opengl::RenderingContextGL::new() as ~renderer::RenderingContext;
    ctx.set_clear_color(1.0, 0.0, 0.0, 1.0);

    let vertices : ~[f32] = ~[
        0.0, 0.0,  1.0, 0.0,  1.0, 1.0,
        0.0, 0.0,  1.0, 1.0,  0.0, 1.0,
    ];

    let quad = ctx.create_vertex_buffer();

    ctx.upload_vertex_data(quad, vertices, renderer::STATIC).map_err(
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
    ], None);

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

    let texture_0 = ctx.create_texture(renderer::CLAMP|renderer::FILTER_NEAREST);
    ctx.allocate_texture(texture_0, 800, 600, renderer::R8G8B8A8);

    let checker_data : Vec<u8> = Vec::from_fn(64*64*4, |i|{ (((i / 4) % 2)*255) as u8 });
    let checker = ctx.create_texture(renderer::REPEAT|renderer::FILTER_NEAREST);
    ctx.upload_texture_data(checker, checker_data.as_slice(), 64, 64, renderer::R8G8B8A8);

    let mut checker_read_back : Vec<u8> = Vec::from_fn(64*64*4, |i|{ 1 as u8 });
    assert!(checker_data != checker_read_back);
    ctx.read_back_texture(checker, renderer::R8G8B8A8,
                          checker_read_back.as_mut_slice());

    assert_eq!(checker_data, checker_read_back);

    let intermediate_target = ctx.create_render_target([texture_0], None, None).map_err(
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

    let screen = ctx.get_default_render_target();

    while !window.should_close() {
        //glfw.poll_events();

        ctx.set_render_target(screen);

        ctx.clear();

        ctx.set_shader(program);

        ctx.set_shader_input_float(u_color, [0.0, 0.5, 1.0, 1.0]);
        ctx.set_shader_input_texture(u_texture_0, 0, checker);

        ctx.draw(
            renderer::GeometryRange {
                geometry: geom,
                from: 0,
                to: 6,
                indexed: false,
            },
            renderer::RENDER_DEFAULT
        ).map_err(
            |e| { fail!("Rendering error: {}", e); return; }
        );

        window.swap_buffers();
    }
}
