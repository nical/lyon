#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]
#![deny(bare_trait_objects)]

//! 2d Path transformation and manipulation algorithms.
//!
//! This crate is reexported in [lyon](https://docs.rs/lyon/).

// TODO doc!

pub extern crate lyon_path as path;

pub(crate) mod advanced_path;
pub mod splitter;
pub mod hatching;
pub mod raycast;
pub mod walk;
pub mod aabb;
pub mod fit;

pub use crate::path::math;
pub use crate::path::geom;
