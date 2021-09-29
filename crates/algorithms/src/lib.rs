#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]
#![deny(bare_trait_objects)]
#![allow(clippy::float_cmp)]

//! 2d Path transformation and manipulation algorithms.
//!
//! This crate is reexported in [lyon](https://docs.rs/lyon/).

// TODO doc!

pub extern crate lyon_path as path;

pub mod aabb;
pub(crate) mod advanced_path;
pub mod fit;
pub mod hatching;
pub mod hit_test;
pub mod raycast;
pub mod splitter;
pub mod walk;
pub mod length;
pub mod winding;
pub mod area;

pub use crate::path::geom;
pub use crate::path::math;
