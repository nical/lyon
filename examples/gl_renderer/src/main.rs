extern crate lyon_renderer;
extern crate glutin;
extern crate gl;
extern crate lyon;

use lyon_renderer::gl_renderer::*;
use lyon_renderer::gpu_data::{GpuAddress, GpuOffset, GpuFillVertex};
use lyon::math::*;
use glutin::{WindowBuilder, GlRequest, GlWindow, EventsLoop, ContextBuilder, GlContext};
use std::mem;

fn main() {
    let win_width: u32 = 800;
    let win_height: u32 = 600;

    let mut events_loop = EventsLoop::new();
    let window = WindowBuilder::new()
        .with_title(format!("renderer"))
        .with_dimensions(800,600);
    let context = ContextBuilder::new().with_gl(GlRequest::Latest);
    let gl_window = GlWindow::new(window, context, &events_loop).unwrap();

    unsafe {
        gl_window.make_current().unwrap();
        gl::load_with(|symbol| { mem::transmute(gl_window.get_proc_address(symbol)) });
    }

    let mut renderer = Renderer::new().unwrap();

    let normal = vec2(0.0, 0.0);
    let prim_id = GpuAddress::global(GpuOffset(0));
    let vertices = &[
        GpuFillVertex { position: point(0.0, 0.0), normal, prim_id },
        GpuFillVertex { position: point(1.0, 0.0), normal, prim_id },
        GpuFillVertex { position: point(1.0, 1.0), normal, prim_id },
        GpuFillVertex { position: point(0.0, 1.0), normal, prim_id },
    ];
    let indices = &[
        0, 1, 2,
        0, 2, 3,
    ];

    let geom = renderer.alloc_fill_geometry(vertices.len() as u32, indices.len() as u32);

    let mut mapped_vertices: &mut[GpuFillVertex] = &mut[];
    renderer.device.map_buffer(geom.vbo, None, map::WRITE, &mut mapped_vertices);
    assert_eq!(mapped_vertices.len(), vertices.len());
    for i in 0..vertices.len() {
        mapped_vertices[i] = vertices[i];
    }
    renderer.device.unmap_buffer(geom.vbo);

    let mut mapped_indices: &mut[u16] = &mut[];
    renderer.device.map_buffer(geom.ibo, None, map::WRITE, &mut mapped_indices);
    assert_eq!(mapped_indices.len(), indices.len());
    for i in 0..indices.len() {
        mapped_indices[i] = indices[i];
    }
    renderer.device.unmap_buffer(geom.ibo);

    let a_position = VertexAttributeLocation { index: 0 };
    let a_normal = VertexAttributeLocation { index: 1 };
    let a_prim_id = VertexAttributeLocation { index: 2 };
    let a_advancement = VertexAttributeLocation { index: 3 };

    let vs = renderer.device.create_shader_stage(
        &ShaderStageDescriptor {
            stage_type: ShaderType::Vertex,
            src: &[VERTEX_SHADER],
        },
    ).unwrap();
    renderer.device.get_shader_stage_result(vs).unwrap();

    let fs = renderer.device.create_shader_stage(
        &ShaderStageDescriptor {
            stage_type: ShaderType::Fragment,
            src: &[FRAGMENT_SHADER],
        },
    ).unwrap();
    renderer.device.get_shader_stage_result(fs).unwrap();

    let shader = renderer.device.create_shader_pipeline(
        &ShaderPipelineDescriptor {
            stages: &[vs, fs],
            attrib_locations: &[
                ("a_position", a_position),
                ("a_normal", a_normal),
                ("a_prim_id", a_prim_id),
            ]
        }
    ).unwrap();

    renderer.device.get_shader_pipeline_result(shader).unwrap();

    let mut should_close = false;
    loop {
        events_loop.poll_events(|event| {
            match event {
                glutin::Event::WindowEvent{ event: glutin::WindowEvent::Closed, .. } => {
                    should_close = true;
                }

                // process events here
                _ => ()
            }
        });

        renderer.device.set_shader(shader).unwrap();
        renderer.device.draw(geom.geom, Range::IndexRange(0, 6), TRIANGLES, Blending::None, COLOR|DEPTH).unwrap();

        // draw everything here

        gl_window.swap_buffers();
        std::thread::sleep(std::time::Duration::from_millis(16));

        if should_close {
            break;
        }
    }
}

static VERTEX_SHADER: &'static str = &"
    #version 140
    #line 115

    in vec2 a_position;
    in vec2 a_normal;
    in int a_prim_id;

    out vec4 v_color;

    void main() {
        gl_Position = vec4(a_position, 0.0, 1.0);
        v_color = vec4(0.5, 0.8, 1.0, 1.0);
    }
";

pub static FRAGMENT_SHADER: &'static str = &"
    #version 140
    in vec4 v_color;
    out vec4 out_color;

    void main() {
        out_color = v_color;
    }
";
