extern crate shaderc;

use std::path::PathBuf;
use std::io::Write;
use std::fs::File;
use std::fs;
use std::env;

fn main() {
    let mut compiler = shaderc::Compiler::new().unwrap();
    let options = shaderc::CompileOptions::new().unwrap();

    let glsl = fs::read_to_string("data/shader.vert.glsl").unwrap();
    let spirv = compiler.compile_into_spirv(
        &glsl, shaderc::ShaderKind::Vertex,
        "shader.vert.glsl", "main",
        Some(&options)
    ).unwrap();

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut output = File::create(out_path.join("shader.vert.spirv")).unwrap();
    output.write_all(spirv.as_binary_u8()).unwrap();

    let glsl = fs::read_to_string("data/shader.frag.glsl").unwrap();
    let spirv = compiler.compile_into_spirv(
        &glsl, shaderc::ShaderKind::Fragment,
        "shader.frag.glsl", "main",
        Some(&options)
    ).unwrap();

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut output = File::create(out_path.join("shader.frag.spirv")).unwrap();
    output.write_all(spirv.as_binary_u8()).unwrap();
}
