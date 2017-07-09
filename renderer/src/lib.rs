#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]

//! Very experimental high level api on top of lyon_tessellator
//!
//! Don't use it.

#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;

extern crate lyon_core as core;
extern crate lyon_path as path;
extern crate lyon_path_builder as path_builder;
extern crate lyon_bezier as bezier;
extern crate lyon_path_iterator as path_iterator;
extern crate lyon_tessellation as tessellation;

pub mod buffer;
pub mod gfx_renderer;
pub mod glsl;
pub mod gpu_data;
pub mod vector_image_renderer;
