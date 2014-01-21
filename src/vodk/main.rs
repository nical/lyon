extern mod native;
extern mod extra;
extern mod gl;

use json = io::json;


mod io {
    pub mod json;
}
pub mod gfx {
    pub mod renderer;
    pub mod opengl;
    pub mod window;
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
    foo();
    gfx::window::main_loop();
}

fn foo() {
    let src = ~"
    {
        \"pi\": 3.14,
        \"foo\": [[1],2,3,4,5],
        \"bar\": true,
        \"baz\": {
            \"plop\": \"hello world! \",
            \"hey\": null,
            \"x\": false
        }
    }  ";

    let shader = gfx::renderer::Shader { handle: 0 };

    let mut validator = json::Validator::new();
    println(src);

    println(" --------------- ");

    json::parse_with_handler(src.chars(), &mut validator as &mut json::Handler);

    match *validator.get_error() {
        Some(_) => {
            println("validation failed");
        }
        None => {
            println("validation suceeded");
        }
    }

    println(" --------------- ");

    for c in json::writer(json::parse_iter(json::tokenize(src.chars())), "  ", "\n") {
        print(c.to_str());
    }
}
