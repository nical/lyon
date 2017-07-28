#[macro_use]
extern crate glium;
#[macro_use]
extern crate clap;
extern crate svgparser;
extern crate glutin;
extern crate rayon;
extern crate lyon;
extern crate lyon_svg;

use glium::Surface;
use glium::index::PrimitiveType;
use glium::DisplayBuild;
use glium::backend::glutin_backend::GlutinFacade as Display;

use lyon::path::Path;
use lyon::path_builder::*;
use lyon::math::*;
use lyon_svg::parser::RgbColor;
use lyon::tessellation::geometry_builder::{ VertexConstructor, VertexBuffers, BuffersBuilder };
use lyon::tessellation::basic_shapes::*;
use lyon::tessellation::path_fill::{ FillEvents, FillTessellator, FillOptions };
use lyon::tessellation::path_stroke::{ StrokeTessellator, StrokeOptions };
use lyon::tessellation::StrokeVertex;
use lyon::path_iterator::PathIterator;

use clap::{Arg, ArgMatches};

fn main() {
    let matches = clap::App::new("SVG renderer test")
        .version("0.1")
        .about("Renders a SVG passed as parameter")
        .arg(Arg::with_name("INPUT")
             .help("Sets the input SVG file")
             .takes_value(true)
             .index(1)
             .required(true))
        .arg(Arg::with_name("OUTPUT")
             .help("Sets the output file to export the tessellated geometry (optional).")
             .takes_value(true)
             .short("o")
             .long("output")
             .required(false))
        .get_matches();

    if let Some(output) = matches.value_of("OUTPUT") {
        println!("output {:?}", output);
    } else {
        println!("Rendering to a window");
    }

    let mut scene = if let Some(input_file) = matches.value_of("INPUT") {
        load_svg(input_file)
    } else {
        unimplemented!();
    };

    tessellate_scene(&mut scene[..]);

    let display = glutin::WindowBuilder::new()
        .with_dimensions(700, 700)
        .with_title("tessellation".to_string())
        .with_multisampling(8)
        .with_vsync()
        .build_glium().unwrap();

    upload_geometry(&mut scene[..], &display);

/*
    let model_vbo = glium::VertexBuffer::new(&display, &vertices[..]).unwrap();
    let model_ibo = glium::IndexBuffer::new(
        &display, PrimitiveType::TrianglesList,
        &indices[..]
    ).unwrap();

    let bg_vbo = glium::VertexBuffer::new(&display, &bg_buffers.vertices[..]).unwrap();
    let bg_ibo = glium::IndexBuffer::new(
        &display, PrimitiveType::TrianglesList,
        &bg_buffers.indices[..]
    ).unwrap();
*/
    // compiling shaders and linking them together
    let bg_program = program!(&display,
        140 => {
            vertex: "
                #version 140
                in vec2 a_position;
                out vec2 v_position;
                void main() {
                    gl_Position = vec4(a_position, 0.0, 1.0);
                    v_position = a_position;
                }
            ",
            fragment: "
                #version 140
                uniform vec2 u_resolution;
                in vec2 v_position;
                out vec4 f_color;
                void main() {
                    vec2 px_position = (v_position * vec2(1.0, -1.0)    + vec2(1.0, 1.0))
                                     * 0.5 * u_resolution;
                    // #005fa4
                    float vignette = clamp(0.0, 1.0, (0.7*length(v_position)));

                    f_color = mix(
                        vec4(0.0, 0.47, 0.9, 1.0),
                        vec4(0.0, 0.1, 0.64, 1.0),
                        vignette
                    );

                    if (mod(px_position.x, 20.0) <= 1.0 ||
                        mod(px_position.y, 20.0) <= 1.0) {
                        f_color *= 1.2;
                    }

                    if (mod(px_position.x, 100.0) <= 1.0 ||
                        mod(px_position.y, 100.0) <= 1.0) {
                        f_color *= 1.2;
                    }
                }
            "
        },
    ).unwrap();

    // compiling shaders and linking them together
    let model_program = program!(&display,
        140 => {
            vertex: "
                #version 140
                uniform vec2 u_resolution;
                uniform mat4 u_matrix;
                in vec2 a_position;
                in vec3 a_color;
                out vec3 v_color;
                void main() {
                    gl_Position = u_matrix * vec4(a_position, 0.0, 1.0);// / vec4(u_resolution, 1.0, 1.0);
                    v_color = a_color;
                }
            ",
            fragment: "
                #version 140
                in vec3 v_color;
                out vec4 f_color;
                void main() {
                    f_color = vec4(v_color, 1.0);
                }
            "
        },
    ).unwrap();

    let mut target_zoom = 1.0;
    let mut zoom = 1.0;
    let mut target_pos = vec2(0.0, 0.0);
    let mut pos = vec2(0.0, 0.0);
    loop {
        zoom += (target_zoom - zoom) / 3.0;
        pos = pos + (target_pos - pos) / 3.0;

        let mut target = display.draw();

        let (w, h) = target.get_dimensions();
        let resolution = vec2(w as f32, h as f32);

        let model_mat = Transform3D::identity();
        let mut view_mat = Transform3D::identity();

        view_mat = view_mat.pre_translated(-1.0, 1.0, 0.0);
        view_mat = view_mat.pre_scaled(5.0 * zoom, 5.0 * zoom, 0.0);
        view_mat = view_mat.pre_scaled(2.0/resolution.x, -2.0/resolution.y, 1.0);
        view_mat = view_mat.pre_translated(pos.x, pos.y, 0.0);

        let uniforms = uniform! {
            u_resolution: resolution.array(),
            u_matrix: uniform_matrix(&model_mat.pre_mul(&view_mat))
        };

        target.clear_color(0.75, 0.75, 0.75, 1.0);

        for item in &scene[..] {
            if let &Some((ref vbo, ref ibo)) = &item.uploaded {
                target.draw(
                    vbo, ibo,
                    &model_program, &uniforms,
                    &Default::default()
                ).unwrap();
            }
        }

        target.finish().unwrap();

        let mut should_close = false;
        for event in display.poll_events() {
            should_close |= match event {
                glutin::Event::Closed => true,
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Escape)) => true,
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::PageDown)) => {
                    target_zoom *= 0.8;
                    false
                }
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::PageUp)) => {
                    target_zoom *= 1.25;
                    false
                }
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Left)) => {
                    target_pos.x += 5.0 / target_zoom;
                    false
                }
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Right)) => {
                    target_pos.x -= 5.0 / target_zoom;
                    false
                }
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Up)) => {
                    target_pos.y += 5.0 / target_zoom;
                    false
                }
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Down)) => {
                    target_pos.y -= 5.0 / target_zoom;
                    false
                }
                _evt => {
                    //println!("{:?}", _evt);
                    false
                }
            };
        }
        if should_close {
            break;
        }
    }
}

use std::fs;
use std::io::Read;
use svgparser::svg as svg_parser;
use svgparser::path as path_parser;
use svgparser::svg::Token as SvgToken;
use svgparser::svg::ElementEnd;

struct RenderItem {
    path: Path,
    fill: Option<RgbColor>,
    stroke: Option<RgbColor>,
    stroke_width: f32,
    geometry: Option<VertexBuffers<Vertex>>,
    uploaded: Option<(glium::VertexBuffer<Vertex>, glium::IndexBuffer<u16>)>,
}

impl RenderItem {
    fn new() -> RenderItem {
        RenderItem {
            path: Path::new(),
            fill: None,
            stroke: None,
            stroke_width: 1.0,
            geometry: None,
            uploaded: None,
        }

    }
}

fn tessellate_scene(scene: &mut[RenderItem]) {
    println!(" -- The scene contains {} items to tessellate", scene.len());
    let mut fill_tessellator = FillTessellator::new();
    let mut fill_events = FillEvents::new();
    let mut stroke_tessellator = StrokeTessellator::new();

    for item in scene {
        if item.geometry.is_none() {
            let mut buffers: VertexBuffers<Vertex> = VertexBuffers::new();
            if let Some(color) = item.fill {
                println!(" -- tessellate fill");
                //fill_events.set_path_iter(item.path.path_iter().flattened(0.03));
                //fill_tessellator.tessellate_events(
                //    &fill_events,
                //    &FillOptions::default(),
                //    &mut BuffersBuilder::new(&mut buffers, WithColor(
                //        [color.red as f32 / 255.0, color.green as f32 / 255.0, color.blue as f32 / 255.0]
                //    ))
                //).unwrap();
            }
            if let Some(color) = item.stroke {
                println!(" -- tessellate stroke");
                stroke_tessellator.tessellate_flattened_path(
                    item.path.path_iter().flattened(0.03),
                    &StrokeOptions::default(),
                    &mut BuffersBuilder::new(&mut buffers, WithColorAndStrokeWidth([
                            color.red as f32 / 255.0,
                            color.green as f32 / 255.0,
                            color.blue as f32 / 255.0
                        ],
                        item.stroke_width
                    ))
                ).unwrap();
                item.geometry = Some(buffers);
                item.uploaded = None;
            }
            //item.geometry = Some(buffers);
            //item.uploaded = None;
        }
    }
}

fn upload_geometry(scene: &mut[RenderItem], display: &Display) {
    for item in scene {
        let uploaded = match (&item.geometry, &item.uploaded) {
            (&Some(ref geom), &None) => {
                let vbo = glium::VertexBuffer::new(display, &geom.vertices[..]).unwrap();
                let ibo = glium::IndexBuffer::new(
                    display, PrimitiveType::TrianglesList,
                    &geom.indices[..]
                ).unwrap();

                Some((vbo, ibo))
            }
            _ => { None }
        };

        if uploaded.is_some() {
            println!(" -- upload geometry");
            item.uploaded = uploaded;
        }
    }
}

fn load_svg(file_name: &str) -> Vec<RenderItem> {
    println!("-- loading {:?}", file_name);

    // Read a file to the buffer.
    let mut file = fs::File::open(file_name).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    let mut render_items = Vec::new();
    let mut current_item = RenderItem::new();

    let mut p = svg_parser::Tokenizer::new(&buffer);
    while let Some(item) = p.next() {
        match item {
            Ok(SvgToken::ElementStart(b"path")) => {
                current_item = RenderItem::new();
            }
            Ok(SvgToken::ElementEnd(_)) => {
                println!(" -- close path");
                if current_item.fill.is_some() || current_item.stroke.is_some() {
                    let mut tmp = RenderItem::new();
                    ::std::mem::swap(&mut current_item, &mut tmp);
                    render_items.push(tmp);
                }
            }
            Ok(SvgToken::Attribute(b"style", stream)) => {
                parse_style(stream, &mut current_item);
            }
            Ok(SvgToken::Attribute(b"d", stream)) => {
                current_item.path = parse_path_data(stream);
            }
            Err(e) => {
                panic!("Error: {:?}.", e);
            }
            _ => {}
        }
    }

    println!(" -- loaded {} paths", render_items.len());

    return render_items;
}

fn parse_path_data(stream: svgparser::Stream) -> Path {
    let mut builder = Path::builder().with_svg();

    for item in lyon_svg::parser::path::PathTokenizer::from_stream(stream) {
        match item {
            Ok(evt) => { builder.svg_event(evt) }
            Err(e) => { panic!("Warning: {:?}.", e); }
        }
    }

    return builder.build();
}

fn parse_style(stream: svgparser::Stream, item: &mut RenderItem) {
    use lyon_svg::parser::{AttributeId, AttributeValue, ValueId};

    for attr in lyon_svg::parser::style::StyleTokenizer::from_stream(stream) {
        if let Ok(attr) = attr {
            match attr.id {
                AttributeId::Fill => {
                    match attr.value {
                        AttributeValue::RgbColor(rgb) => { item.fill = Some(rgb); }
                        AttributeValue::KeyWord(ValueId::None) => { item.fill = None; }
                        _ => { item.fill = Some(RgbColor { red: 255, green: 0, blue: 0 }) }
                    }
                }
                AttributeId::Stroke => {
                    match attr.value {
                        AttributeValue::RgbColor(rgb) => { item.stroke = Some(rgb); }
                        AttributeValue::KeyWord(ValueId::None) => { item.stroke = None; }
                        _ => {
                            panic!(" Unimplemented ! stroke: {:?}", attr.value);
                        }
                    }
                }
                AttributeId::StrokeWidth => {
                    match attr.value {
                        AttributeValue::Number(n) => {
                            item.stroke_width = n as f32;
                        }
                        AttributeValue::Length(lyon_svg::parser::Length { num, unit }) => {
                            item.stroke_width = num as f32;
                        }
                        _=> {
                            panic!(" stroke-width: {:?}", attr.value);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Vertex {
    a_position: [f32; 2],
    a_color: [f32; 3],
}

struct WithColor([f32; 3]);

impl VertexConstructor<Vec2, Vertex> for WithColor {
    fn new_vertex(&mut self, pos: Vec2) -> Vertex {
        assert!(!pos.x.is_nan());
        assert!(!pos.y.is_nan());
        Vertex {
            a_position: pos.array(),
            a_color: self.0,
        }
    }
}

struct WithColorAndStrokeWidth([f32; 3], f32);

impl VertexConstructor<StrokeVertex, Vertex> for WithColorAndStrokeWidth {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> Vertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());
        assert!(!vertex.normal.x.is_nan());
        assert!(!vertex.normal.y.is_nan());
        Vertex {
            a_position: (vertex.position + vertex.normal * self.1).array(),
            a_color: self.0,
        }
    }
}

implement_vertex!(Vertex, a_position, a_color);

#[derive(Copy, Clone, Debug)]
struct BgVertex {
    a_position: [f32; 2],
}

struct BgWithColor ;
impl VertexConstructor<Vec2, BgVertex> for BgWithColor  {
    fn new_vertex(&mut self, pos: Vec2) -> BgVertex {
        BgVertex { a_position: pos.array() }
    }
}

implement_vertex!(BgVertex, a_position);

fn uniform_matrix(m: &Transform3D) -> [[f32; 4]; 4] {
    [
        [m.m11, m.m12, m.m13, m.m14],
        [m.m21, m.m22, m.m23, m.m24],
        [m.m31, m.m32, m.m33, m.m34],
        [m.m41, m.m42, m.m43, m.m44],
    ]
}
