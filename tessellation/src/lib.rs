#![allow(dead_code)]

extern crate lyon_core as core;

#[cfg(test)]
extern crate lyon_path as path;
#[cfg(test)]
extern crate lyon_path_builder as path_builder;
#[cfg(test)]
extern crate lyon_path_iterator as path_iterator;
#[cfg(test)]
extern crate lyon_extra as extra;

pub mod basic_shapes;
pub mod path_fill;
pub mod path_stroke;
pub mod geometry_builder;

pub use core::*;
