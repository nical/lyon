extern mod native;
extern mod extra;
extern mod gl;


pub mod gfx {
    pub mod renderer;
    pub mod opengl;
    //pub mod window;
}
//mod util {
//    pub mod rand;
//}
mod logic {
    pub mod entity;
}


#[start]
fn start(argc: int, argv: **u8) -> int {
    native::start(argc, argv, main)
}

fn main() {
    std::io::println("vodk!");
    //gfx::window::main_loop();
}
