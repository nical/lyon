
#[macro_use]
extern crate clap;
extern crate gfx;
extern crate svgparser;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate rayon;
extern crate lyon;
extern crate lyon_svg;

use gfx::traits::FactoryExt;
use gfx::Device;
use gfx::format::{DepthStencil, Rgba8};

use lyon::path::Path;
use lyon::path_builder::*;
use lyon::math::*;
use lyon_svg::parser::RgbColor;

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

    let scene = if let Some(input_file) = matches.value_of("INPUT") {
        load_svg(input_file)
    } else {
        unimplemented!();
    };

    let builder = glutin::WindowBuilder::new().with_title("Svg renderer test".to_string());
    let (window, device, factory, rtv, stv) = gfx_window_glutin::init::<Rgba8, DepthStencil>(builder);

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
}

impl RenderItem {
    fn new() -> RenderItem {
        RenderItem {
            path: Path::new(),
            fill: None,
            stroke: None,
            stroke_width: 1.0,
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
            Ok(SvgToken::ElementEnd(ElementEnd::Close(b"path"))) => {
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
                break;
            }
            _ => {}
        }
    }

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
                        AttributeValue::None => { item.stroke = None; }
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

/*
gfx_defines!{
    vertex Vertex {
        pos: [f32; 2] = "a_position",
    }

    constant Locals {
        transform: [[f32; 4]; 4] = "u_transform",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        transform: gfx::Global<[[f32; 4]; 4]> = "u_transform",
        locals: gfx::ConstantBuffer<Locals> = "Locals",
        out_color: gfx::RenderTarget<ColorFormat> = "Target0",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

impl Vertex {
    fn new(pos: Point) -> Vertex { Vertex { pos: [pos.x, pos.y] } }
}
*/