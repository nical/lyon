//! # Lyon
//!
//! 2d rendering on the GPU in rust.
//!
//! # Crates
//!
//! * lyon_tessellation ([Documentation](../lyon_tessellation/index.html)): Path tessellation routines.
//! * lyon_path_builder ([Documentation](../lyon_path_builder/index.html)): Tools to facilitate building paths.
//! * lyon_path_iterator ([Documentation](../lyon_path_iterator/index.html)): Tools to facilitate iteratring over paths.
//! * lyon_path ([Documentation](../lyon_path/index.html)): A simple optional path datat structure, provided for convenience.
//! * lyon_bezier ([Documentation](../lyon_bezier/index.html)): Cubic and Quadratic 2d bezier math.
//! * lyon_extra ([Documentation](../lyon_extra/index.html)): Additional testing and debugging tools.
//! * lyon_core ([Documentation](../lyon_core/index.html)): Common types to most lyon crates.
//!
//! This crate is just a meta-crate, reexporting the crates listed above.
//!
//! # Additional documentation
//!
//! The is some useful documentaion on the project's [wiki](https://github.com/nical/lyon/wiki).
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
