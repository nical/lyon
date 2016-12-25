#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate lyon;

use lyon::extra::rust_logo::build_logo_path;
use lyon::path_builder::*;
use lyon::math::*;
use lyon::tessellation::geometry_builder::{ VertexConstructor, VertexBuffers, BuffersBuilder };
use lyon::tessellation::basic_shapes::*;
use lyon::tessellation::path_fill::{ FillEvents, FillTessellator, FillOptions };
use lyon::tessellation::path_stroke::{ StrokeTessellator, StrokeOptions };
use lyon::path::Path;
use lyon::path_iterator::PathIterator;

use gfx::traits::FactoryExt;
use gfx::Device;

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

const SHAPE_DATA_LEN: usize = 64;

gfx_defines!{
    constant Constants {
        resolution: [f32; 2] = "u_resolution",
        scroll_offset: [f32; 2] = "u_scroll_offset",
        zoom: f32 = "u_zoom",
    }

    constant ShapeTransform {
        transform: [[f32; 4]; 4] = "transform",
    }

    constant ShapeData {
        color: [f32; 4] = "color",
        z_index: f32 = "z_index",
        _padding_0: f32 = "_padding_0",
        _padding_1: f32 = "_padding_1",
        _padding_2: f32 = "_padding_2",
    }

    vertex Vertex {
        position: [f32; 2] = "a_position",
        color: [f32; 3] = "a_color",
        shape_id: i32 = "a_shape_id",
    }

    vertex BgVertex {
        position: [f32; 2] = "a_position",
    }

    pipeline model_pipeline {
        vbo: gfx::VertexBuffer<Vertex> = (),
        out_color: gfx::RenderTarget<ColorFormat> = "out_color",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
        constants: gfx::ConstantBuffer<Constants> = "Constants",
        transforms: gfx::ConstantBuffer<ShapeTransform> = "u_transforms",
        shape_data: gfx::ConstantBuffer<ShapeData> = "u_shape_data",
    }

    pipeline bg_pipeline {
        vbo: gfx::VertexBuffer<BgVertex> = (),
        out_color: gfx::RenderTarget<ColorFormat> = "out_color",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
        constants: gfx::ConstantBuffer<Constants> = "Constants",
    }
}

impl ShapeData {
    fn new(color: [f32; 4], z_index: f32) -> ShapeData {
        ShapeData {
            color: color,
            z_index: z_index,
            _padding_0: 0.0,
            _padding_1: 0.0,
            _padding_2: 0.0,
        }
    }
}

struct VertexCtor([f32; 3], i32);

impl VertexConstructor<Vec2, Vertex> for VertexCtor {
    fn new_vertex(&mut self, pos: Vec2) -> Vertex {
        assert!(!pos.x.is_nan());
        assert!(!pos.y.is_nan());
        Vertex {
            position: pos.array(),
            color: self.0,
            shape_id: self.1,
        }
    }
}

struct BgVertexCtor ;
impl VertexConstructor<Vec2, BgVertex> for BgVertexCtor  {
    fn new_vertex(&mut self, pos: Vec2) -> BgVertex {
        BgVertex { position: pos.array() }
    }
}

fn main() {
    let mut builder = SvgPathBuilder::new(Path::builder());

    build_logo_path(&mut builder);

    let path = builder.build();

    let mut buffers: VertexBuffers<Vertex> = VertexBuffers::new();
    let mut shape_data_cpu = &mut [ShapeData::new([1.0, 0.0, 0.0, 1.0], 1.0); SHAPE_DATA_LEN];
    let shape_transforms_cpu = &[
        ShapeTransform { transform: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]}; SHAPE_DATA_LEN
    ];

    let events = FillEvents::from_iter(path.path_iter().flattened(0.03));

    FillTessellator::new().tessellate_events(
        &events,
        &FillOptions::default(),
        &mut BuffersBuilder::new(&mut buffers, VertexCtor([0.9, 0.9, 1.0], 1))
    ).unwrap();
    shape_data_cpu[1] = ShapeData::new([1.0, 1.0, 1.0, 1.0], 0.2);

    StrokeTessellator::new().tessellate(
        path.path_iter().flattened(0.03),
        &StrokeOptions::stroke_width(1.0),
        &mut BuffersBuilder::new(&mut buffers, VertexCtor([0.0, 0.0, 0.0], 2))
    ).unwrap();
    shape_data_cpu[2] = ShapeData::new([0.0, 0.0, 0.0, 0.1], 0.1);

    let show_points = false;

    if show_points {
        for p in path.as_slice().iter() {
            if let Some(to) = p.destination() {
                tessellate_ellipsis(to, vec2(1.0, 1.0), 16,
                    &mut BuffersBuilder::new(&mut buffers,
                        VertexCtor([0.0, 0.0, 0.0], 3)
                    )
                );
                tessellate_ellipsis(to, vec2(0.5, 0.5), 16,
                    &mut BuffersBuilder::new(&mut buffers,
                        VertexCtor([0.0, 1.0, 0.0], 4)
                    )
                );
            }
        }
    }

    println!(" -- {} vertices {} indices", buffers.vertices.len(), buffers.indices.len());

    let mut bg_buffers: VertexBuffers<BgVertex> = VertexBuffers::new();
    tessellate_rectangle(
        &Rect::new(vec2(-1.0, -1.0), size(2.0, 2.0)),
        &mut BuffersBuilder::new(&mut bg_buffers, BgVertexCtor )
    );

    let glutin_builder = glutin::WindowBuilder::new()
        .with_dimensions(700, 700)
        .with_decorations(true)
        .with_title("tessellation".to_string())
        .with_multisampling(8)
        .with_vsync();

    let (window, mut device, mut factory, mut main_fbo, mut main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(glutin_builder);

    println!(" -- hidpi factor: {}", window.hidpi_factor());

    let mut cmd_queue: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let constants = factory.create_constant_buffer(1);
    let shape_data_gpu = factory.create_constant_buffer(SHAPE_DATA_LEN);
    let shape_transforms_gpu = factory.create_constant_buffer(SHAPE_DATA_LEN);

    let bg_pso = factory.create_pipeline_simple(
        BACKGROUND_VERTEX_SHADER.as_bytes(),
        BACKGROUND_FRAGMENT_SHADER.as_bytes(),
        bg_pipeline::new()
    ).unwrap();

    let (bg_vbo, bg_range) = factory.create_vertex_buffer_with_slice(
        &bg_buffers.vertices[..],
        &bg_buffers.indices[..]
    );

    let model_pso = factory.create_pipeline_simple(
        MODEL_VERTEX_SHADER.as_bytes(),
        MODEL_FRAGMENT_SHADER.as_bytes(),
        model_pipeline::new()
    ).unwrap();

    let (model_vbo, model_range) = factory.create_vertex_buffer_with_slice(
        &buffers.vertices[..],
        &buffers.indices[..]
    );

    let mut view = Viewport {
        target_zoom: 5.0,
        zoom: 0.5,
        target_scroll: vec2(70.0, 70.0),
        scroll: vec2(70.0, 70.0),
    };

    let mut init_queue: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    init_queue.update_buffer(&shape_data_gpu, shape_data_cpu, 0).unwrap();
    init_queue.update_buffer(&shape_transforms_gpu, shape_transforms_cpu, 0).unwrap();
    init_queue.flush(&mut device);

    let mut frame_count: usize = 0;
    loop {
        if !update_viewport(&window, &mut view) {
            break;
        }

        // Set the color of the second shape (the outline) to some slowly changing
        // pseudo-random color.
        shape_data_cpu[2].color = [
            (frame_count as f32 * 0.008 - 1.6).sin() * 0.1 + 0.1,
            (frame_count as f32 * 0.005 - 1.6).sin() * 0.1 + 0.1,
            (frame_count as f32 * 0.01 - 1.6).sin() * 0.1 + 0.1,
            1.0
        ];

        gfx_window_glutin::update_views(&window, &mut main_fbo, &mut main_depth);
        let (w, h) = window.get_inner_size_pixels().unwrap();

        cmd_queue.clear(&main_fbo.clone(), [0.0, 0.0, 0.0, 0.0]);
        cmd_queue.clear_depth(&main_depth.clone(), 1.0);

        cmd_queue.update_buffer(&shape_data_gpu, shape_data_cpu, 0).unwrap();
        cmd_queue.update_constant_buffer(&constants, &Constants {
            resolution: [w as f32, h as f32],
            zoom: view.zoom,
            scroll_offset: view.scroll.array(),
        });
        cmd_queue.draw(&model_range, &model_pso, &model_pipeline::Data {
            vbo: model_vbo.clone(),
            out_color: main_fbo.clone(),
            out_depth: main_depth.clone(),
            constants: constants.clone(),
            shape_data: shape_data_gpu.clone(),
            transforms: shape_transforms_gpu.clone(),
        });
        cmd_queue.draw(&bg_range, &bg_pso, &bg_pipeline::Data {
            vbo: bg_vbo.clone(),
            out_color: main_fbo.clone(),
            out_depth: main_depth.clone(),
            constants: constants.clone(),
        });
        cmd_queue.flush(&mut device);

        window.swap_buffers().unwrap();
        device.cleanup();

        frame_count += 1;
    }
}

static MODEL_VERTEX_SHADER: &'static str = &"
    #version 140
    #line 266
    uniform Constants {
        vec2 u_resolution;
        vec2 u_scroll_offset;
        float u_zoom;
    };

    #define SHAPE_DATA_LEN 64

    struct ShapeTransform { mat4 transform; };
    uniform u_transforms { ShapeTransform transforms[SHAPE_DATA_LEN]; };

    struct ShapeData {
        vec4 color;
        float z_index;
        float _padding_0;
        float _padding_1;
        float _padding_2;
    };
    uniform u_shape_data { ShapeData shape_data[SHAPE_DATA_LEN]; };

    in vec2 a_position;
    in int a_shape_id;

    out vec4 v_color;

    void main() {
        ShapeData data = shape_data[a_shape_id];

        vec4 world_pos = transforms[a_shape_id].transform * vec4(a_position, 0.0, 1.0);
        vec2 transformed_pos = (world_pos.xy - u_scroll_offset)
            * u_zoom / (vec2(0.5, -0.5) * u_resolution);

        gl_Position = vec4(
            transformed_pos,
            world_pos.z + data.z_index,
            world_pos.w
        );
        v_color = data.color;
    }
";

static MODEL_FRAGMENT_SHADER: &'static str = &"
    #version 140
    in vec4 v_color;
    out vec4 out_color;

    void main() {
        out_color = v_color;
    }
";

static BACKGROUND_VERTEX_SHADER: &'static str = &"
    #version 140
    in vec2 a_position;
    out vec2 v_position;

    void main() {
        // TODO: fetch the z coordinate and a transform from a buffer.
        gl_Position = vec4(a_position, 0.9, 1.0);
        v_position = a_position;
    }
";

static BACKGROUND_FRAGMENT_SHADER: &'static str = &"
    #version 140
    uniform Constants {
        vec2 u_resolution;
        vec2 u_scroll_offset;
        float u_zoom;
    };
    in vec2 v_position;
    out vec4 out_color;

    void main() {
        vec2 px_position = v_position * vec2(1.0, -1.0) * u_resolution * 0.5;

        // #005fa4
        float vignette = clamp(0.0, 1.0, (0.7*length(v_position)));
        out_color = mix(
            vec4(0.0, 0.47, 0.9, 1.0),
            vec4(0.0, 0.1, 0.64, 1.0),
            vignette
        );

        // TODO: properly adapt the grid while zooming in and out.
        float grid_scale = 5.0;
        if (u_zoom < 2.5) {
            grid_scale = 1.0;
        }

        vec2 pos = px_position + u_scroll_offset * u_zoom;

        if (mod(pos.x, 20.0 / grid_scale * u_zoom) <= 1.0 ||
            mod(pos.y, 20.0 / grid_scale * u_zoom) <= 1.0) {
            out_color *= 1.2;
        }

        if (mod(pos.x, 100.0 / grid_scale * u_zoom) <= 2.0 ||
            mod(pos.y, 100.0 / grid_scale * u_zoom) <= 2.0) {
            out_color *= 1.2;
        }
    }
";

struct Viewport {
    target_zoom: f32,
    zoom: f32,
    target_scroll: Vec2,
    scroll: Vec2,
}

fn update_viewport(window: &glutin::Window, view: &mut Viewport) -> bool {
    for event in window.poll_events() {
        use glutin::Event::KeyboardInput;
        use glutin::ElementState::Pressed;
        use glutin::VirtualKeyCode;
        match event {
            glutin::Event::Closed => {
                return false;
            }
            KeyboardInput(Pressed, _, Some(key)) => {
                match key {
                    VirtualKeyCode::Escape => {
                        return false;
                    }
                    VirtualKeyCode::PageDown => {
                        view.target_zoom *= 0.8;
                    }
                    VirtualKeyCode::PageUp => {
                        view.target_zoom *= 1.25;
                    }
                    VirtualKeyCode::Left => {
                        view.target_scroll.x -= 20.0 / view.target_zoom;
                    }
                    VirtualKeyCode::Right => {
                        view.target_scroll.x += 20.0 / view.target_zoom;
                    }
                    VirtualKeyCode::Up => {
                        view.target_scroll.y -= 20.0 / view.target_zoom;
                    }
                    VirtualKeyCode::Down => {
                        view.target_scroll.y += 20.0 / view.target_zoom;
                    }
                    _key => {}
                }
                println!(" -- zoom: {}, scroll: {:?}", view.target_zoom, view.target_scroll);
            }
            _evt => {
                //println!("{:?}", _evt);
            }
        };
    }

    view.zoom += (view.target_zoom - view.zoom) / 3.0;
    view.scroll = view.scroll + (view.target_scroll - view.scroll) / 3.0;

    return true;
}
