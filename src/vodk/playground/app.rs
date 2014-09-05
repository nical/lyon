
use gpu::context as gpu;
use gpu::opengl;
use io::inputs;
use io::window;
use std::io::timer::sleep;
use time;
use std::time::duration::Duration;

pub trait App {
    fn new(window: &mut window::Window, ctx: &mut gpu::RenderingContext) -> Self;
    fn update(&mut self, dt: f32, window: &mut window::Window, ctx: &mut gpu::RenderingContext);
    fn shut_down(&mut self, window: &mut window::Window, ctx: &mut gpu::RenderingContext);
    fn handle_events(&mut self, events: &[inputs::Event]);
    fn should_close(&mut self) -> bool;
}

pub fn run<T: App>(w: u32, h: u32, title: &str) {

    let mut window = window::Window::new(w, h, title);
    window.init_opengl();
    let mut ctx = opengl::RenderingContextGL::new();

    let mut app: T = App::new(&mut window, &mut ctx);

    let mut input_events: Vec<inputs::Event> = Vec::new();

    let mut avg_frame_time: u64 = 0;
    let mut frame_count: u64 = 0;
    let mut previous_time = time::precise_time_ns();

    while !window.should_close() {
        input_events.clear();
        window.poll_events(&mut input_events);
        app.handle_events(input_events.as_slice());
        
        if app.should_close() {
            break;
        }

        let frame_start_time = time::precise_time_ns();
        let elapsed_time = frame_start_time - previous_time;

        app.update((elapsed_time * 1000) as f32, &mut window, &mut ctx);
        window.swap_buffers();

        previous_time = frame_start_time;
        let frame_time = time::precise_time_ns() - frame_start_time;
        frame_count += 1;
        avg_frame_time += frame_time;

        if frame_count % 60 == 0 {
            println!("avg frame time: {}ms", avg_frame_time as f64/(60.0*1000000.0));
            avg_frame_time = 0;
        }

        let sleep_time: i64 = 16000000 - frame_time as i64;
        if sleep_time > 0 {
            sleep(Duration::milliseconds(sleep_time/1000000));
        }

    }

    app.shut_down(&mut window, &mut ctx);
}

pub static a_position:   gpu::VertexAttributeLocation = 0;
pub static a_normal:     gpu::VertexAttributeLocation = 1;
pub static a_tex_coords: gpu::VertexAttributeLocation = 2;
pub static a_color:      gpu::VertexAttributeLocation = 3;
// for antialiased shape rendering
pub static a_extrusion:  gpu::VertexAttributeLocation = 4;

#[deriving(Show)]
pub struct UniformLayout {
    pub u_resolution: gpu::ShaderInputLocation,
    pub u_color: gpu::ShaderInputLocation,
    pub u_texture_0: gpu::ShaderInputLocation,
    pub u_texture_1: gpu::ShaderInputLocation,
    pub u_texture_2: gpu::ShaderInputLocation,
    pub u_texture_3: gpu::ShaderInputLocation,
    pub u_model_mat: gpu::ShaderInputLocation,
    pub u_view_mat: gpu::ShaderInputLocation,
    pub u_proj_mat: gpu::ShaderInputLocation,
}

impl UniformLayout {
    pub fn new(ctx: &mut gpu::RenderingContext, p: gpu::Shader) -> UniformLayout{
        return UniformLayout {
            u_resolution: ctx.get_shader_input_location(p, "u_resolution"),
            u_texture_0: ctx.get_shader_input_location(p, "u_texture_0"),
            u_texture_1: ctx.get_shader_input_location(p, "u_texture_1"),
            u_texture_2: ctx.get_shader_input_location(p, "u_texture_2"),
            u_texture_3: ctx.get_shader_input_location(p, "u_texture_3"),
            u_model_mat: ctx.get_shader_input_location(p, "u_model_mat"),
            u_view_mat: ctx.get_shader_input_location(p, "u_view_mat"),
            u_proj_mat: ctx.get_shader_input_location(p, "u_proj_mat"),
            u_color: ctx.get_shader_input_location(p, "u_color"),
        }
    }
}


pub fn setup_shader(
    ctx: &mut gpu::RenderingContext,
    vs_src: &str,
    fs_src: &str
) -> (gpu::Shader, UniformLayout) {
    let vs = ctx.create_shader_stage(gpu::VERTEX_SHADER);
    let fs = ctx.create_shader_stage(gpu::FRAGMENT_SHADER);
    let program = ctx.create_shader();

    ctx.compile_shader_stage(vs, &[vs_src]).map_err(
        |e| { fail!("Failed to compile the vertex shader: {}", e); return; }
    );

    ctx.compile_shader_stage(fs, &[fs_src]).map_err(
        |e| { fail!("Failed to compile the fragment shader: {}", e); return; }
    );

    ctx.link_shader(program, [vs, fs], &[
        ("a_position", a_position),
        ("a_normal", a_normal),
        ("a_tex_coords", a_tex_coords),
        ("a_color", a_color),
        ("a_extrusion", a_extrusion),
    ]).map_err(
        |e| { fail!("Failed to link the text's shader program: {}", e); return; }
    );

    let uniforms = UniformLayout::new(ctx, program);
    ctx.destroy_shader_stage(vs);
    ctx.destroy_shader_stage(fs);
    return (program, uniforms);
}