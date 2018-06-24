#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]

//! 2D Path transformation and manipulation algorithms.
//!

// TODO doc!

pub extern crate lyon_path as path;

pub mod hatching;
pub mod aabb;

pub use path::math;
pub use path::geom;
