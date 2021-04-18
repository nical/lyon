#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]
#![deny(bare_trait_objects)]
#![allow(dead_code)]
#![allow(unused_variables)]

extern crate lyon_path as path;
extern crate lyon_svg as svg;

pub use path::geom::euclid;
pub use path::math;

pub mod debugging;
pub mod image;
pub mod rust_logo;
pub mod triangle_rasterizer;
