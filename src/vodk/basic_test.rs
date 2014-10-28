#![feature(macro_rules, globs)]
#![feature(default_type_params)]
#![feature(unsafe_destructor)]

extern crate native;
extern crate gl;
extern crate glfw;
extern crate time;
extern crate libc;
extern crate core;

//use data;
use gpu::context::RenderingContext;
use io::window::Window;
use playground::app;
use math::units::world;

pub mod data;

pub mod math {
    pub mod units;
    pub mod vector;
}

pub mod gpu;

pub mod gfx2d;

pub mod io;

pub mod containers;

pub mod playground {
    pub mod app;
}




struct TestApp {
    resolution: [f32, ..2],
}

#[deriving(Show)]
#[repr(C)]
struct Pos2DTex2D {
    x: f32,
    y: f32,
    s: f32,
    t: f32,
}

static vec2_vec2_slice_type : &'static[data::Type] = &[data::VEC2, data::VEC2];

impl Pos2DTex2D {
    fn dynamically_typed_slice<'l>(data: &'l[Pos2DTex2D]) -> data::DynamicallyTypedSlice<'l> {
        data::DynamicallyTypedSlice::new(data, vec2_vec2_slice_type)
    }
}

impl app::App for TestApp {

    fn new(window: &mut Window, ctx: &mut RenderingContext) -> TestApp {
        TestApp {
            resolution: [800.0, 600.0],
        }
    }

    fn update(&mut self, dt: f32, window: &mut Window, ctx: &mut RenderingContext) {
        println!(" -- update");
    }

    fn shut_down(&mut self, window: &mut Window, ctx: &mut RenderingContext) {
        println!(" -- shut_down");
    }

    fn handle_events(&mut self, events: &[io::inputs::Event]) {
        println!(" -- handle_events");        
    }

    fn should_close(&mut self) -> bool { true }
}


fn main() {
    app::run::<TestApp>(800, 600, "GL test");
}
