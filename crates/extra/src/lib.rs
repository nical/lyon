#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]
#![deny(bare_trait_objects)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![no_std]

extern crate lyon_path as path;

extern crate alloc;

#[cfg(any(test, feature = "std"))]
#[macro_use]
extern crate std;


pub use path::geom::euclid;
pub use path::math;

#[cfg(any(test, feature = "std"))]
pub mod debugging;
pub mod parser;
pub mod rust_logo;
