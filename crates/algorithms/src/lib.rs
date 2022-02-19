#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]
#![deny(bare_trait_objects)]
#![allow(clippy::float_cmp)]

//! 2d Path transformation and manipulation algorithms.
//!
//! This crate is reexported in [lyon](https://docs.rs/lyon/).

pub extern crate lyon_path as path;

pub mod aabb;
pub mod area;
pub mod fit;
pub mod hatching;
pub mod hit_test;
pub mod length;
pub mod raycast;
pub mod walk;
pub mod winding;
pub mod rect;

pub use crate::path::geom;
pub use crate::path::math;
