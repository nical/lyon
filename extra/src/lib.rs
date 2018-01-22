#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]

#![allow(dead_code)]
#![allow(unused_variables)]

extern crate lyon_path as path;
extern crate lyon_svg as svg;

pub use path::geom::math;
pub use path::geom::euclid;

pub mod rust_logo;
pub mod triangle_rasterizer;
pub mod debugging;
pub mod image;
