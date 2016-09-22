//! # Lyon
//!
//! 2d rendering on the GPU in rust.
//!
//! # Crates
//!
//! * [lyon_tessellation](https://crates.io/crates/lyon_tessellation) ([documentation](../lyon_tessellation/index.html)): Path tessellation routines.
//! * [lyon_path_builder](https://crates.io/crates/lyon_path_builder) ([documentation](../lyon_path_builder/index.html)): Tools to facilitate building paths.
//! * [lyon_path_iterator](https://crates.io/crates/lyon_path_iterator) ([documentation](../lyon_path_iterator/index.html)): Tools to facilitate iteratring over paths.
//! * [lyon_path](https://crates.io/crates/lyon_path) ([documentation](../lyon_path/index.html)): A simple optional path datat structure, provided for convenience.
//! * [lyon_bezier](https://crates.io/crates/lyon_bezier) ([documentation](../lyon_bezier/index.html)): Cubic and Quadratic 2d bezier math.
//! * [lyon_extra](https://crates.io/crates/lyon_extra) ([documentation](../lyon_extra/index.html)): Additional testing and debugging tools.
//! * [lyon_core](https://crates.io/crates/lyon_path) ([documentation](../lyon_core/index.html)): Common types to most lyon crates.
//!
//! [This crate](https://crates.io/crates/lyon) is just a meta-crate, reexporting the crates listed above.
//!
//! # Additional documentation and links
//!
//! * There is some useful documentaion on the project's [wiki](https://github.com/nical/lyon/wiki).
//! * The source code is available on the project's [git repository](https://github.com/nical/lyon).
//! * Interested in contributing? [This page is](https://github.com/nical/lyon/wiki/Contribute)
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
//pub extern crate lyon_svg;

pub use lyon_core::*;

pub use lyon_tessellation as tessellation;
pub use lyon_path as path;
pub use lyon_path_builder as path_builder;
pub use lyon_path_iterator as path_iterator;
pub use lyon_bezier as bezier;
pub use lyon_extra as extra;
//pub use lyon_svg as svg;
