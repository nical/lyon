#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]

//! 2d rendering on the GPU in rust.
//!
//! ![logo](https://nical.github.io/lyon-doc/lyon-logo.svg)
//!
//! [![crate](http://meritbadge.herokuapp.com/lyon)](https://crates.io/crates/lyon)
//! [![ci](https://img.shields.io/travis/nical/lyon/master.svg)](https://travis-ci.org/nical/lyon)
//!
//! # Crates
//!
//! * [![doc](https://docs.rs/lyon_tessellation/badge.svg)](https://docs.rs/lyon_tessellation) - [lyon_tessellation](https://crates.io/crates/lyon_tessellation) - Path tessellation routines.
//! * [![doc](https://docs.rs/lyon_path_builder/badge.svg)](https://docs.rs/lyon_path_builder) - [lyon_path_builder](https://crates.io/crates/lyon_path_builder) - Tools to facilitate building paths.
//! * [![doc](https://docs.rs/lyon_path_iterator/badge.svg)](https://docs.rs/lyon_path_iterator) - [lyon_path_iterator](https://crates.io/crates/lyon_path_iterator) - Tools to facilitate iteratring over paths.
//! * [![doc](https://docs.rs/lyon_path/badge.svg)](https://docs.rs/lyon_path) - [lyon_path](https://crates.io/crates/lyon_path) - A simple optional path data structure, provided for convenience.
//! * [![doc](https://docs.rs/lyon_bezier/badge.svg)](https://docs.rs/lyon_bezier) - [lyon_bezier](https://crates.io/crates/lyon_bezier) - Cubic and quadratic 2d bezier math.
//! * [![doc](https://docs.rs/lyon_extra/badge.svg)](https://docs.rs/lyon_extra) - [lyon_extra](https://crates.io/crates/lyon_extra) - Additional testing and debugging tools.
//! * [![doc](https://docs.rs/lyon_core/badge.svg)](https://docs.rs/lyon_path_core) - [lyon_core](https://crates.io/crates/lyon_core) - Common types to most lyon crates.
//!
//! [This crate](https://crates.io/crates/lyon) is just a meta-crate, reexporting the crates listed above.
//!
//! # Additional documentation and links
//!
//! * [very basic gfx-rs example](https://github.com/nical/lyon/tree/master/examples/gfx_basic).
//! * [advanced gfx-rs example](https://github.com/nical/lyon/tree/master/examples/gfx_advanced).
//! * There is some useful documentaion on the project's [wiki](https://github.com/nical/lyon/wiki).
//! * The source code is available on the project's [git repository](https://github.com/nical/lyon).
//! * Interested in contributing? [This page](https://github.com/nical/lyon/wiki/Contribute)
//!   is probably what you are looking for! You can also look at the list of
//!   [issues marked as easy](https://github.com/nical/lyon/issues?q=is%3Aissue+is%3Aopen+label%3Aeasy),
//!   they are a good place to start.
//!


pub extern crate lyon_core;
pub extern crate lyon_path;
pub extern crate lyon_path_builder;
pub extern crate lyon_path_iterator;
pub extern crate lyon_tessellation;
pub extern crate lyon_bezier;
pub extern crate lyon_extra;
pub extern crate lyon_svg;
//pub extern crate lyon_renderer;

pub use lyon_core::*;

pub use lyon_tessellation as tessellation;
pub use lyon_path as path;
pub use lyon_path_builder as path_builder;
pub use lyon_path_iterator as path_iterator;
pub use lyon_bezier as bezier;
pub use lyon_extra as extra;
pub use lyon_svg as svg;
//pub use lyon_renderer as renderer;
