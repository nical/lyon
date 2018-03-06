extern crate cgmath;
#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate lyon;
extern crate resvg;

use resvg::tree::Color;

mod path_convert;
mod stroke_convert;
pub mod render;

pub use self::path_convert::convert_path;
pub use self::stroke_convert::convert_stroke;

pub const FALLBACK_COLOR: Color = Color {
    red: 0,
    green: 0,
    blue: 0,
};
