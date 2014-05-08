#![crate_id = "vodk#0.1"]
#![feature(macro_rules, globs)]

extern crate native;
extern crate gl;
extern crate glfw;

pub mod gfx {
    pub mod renderer;
    pub mod opengl;
    pub mod window;
    pub mod shaders;
    pub mod mesh_utils;
    pub mod geom;
    pub mod test_renderer;
}
pub mod logic {
    pub mod entity;
}
pub mod base {
	pub mod containers;
}
pub mod data {
    //pub mod layout;
}
pub mod kiwi {
    //pub mod graph;
}

fn main() {
    std::io::println("vodk!");
    gfx::window::main_loop();
}
