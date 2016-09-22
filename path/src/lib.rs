//! # Lyon path
//!
//! A simple path data structure implementing the traits provided in the
//! [lyon_path_builder](../lyon_path_builder/index.html) and
//! [lyon_path_iterator](../lyon_path_iterator/index.html) modules.
//!
//! # Examples
//!
//! ```
//! # extern crate lyon_core;
//! # extern crate lyon_path;
//! # extern crate lyon_path_builder;
//! # extern crate lyon_path_iterator;
//! # fn main() {
//! use lyon_path::Path;
//! use lyon_core::math::{point};
//! use lyon_path_builder::*;
//!
//! // Create a builder object to build the path.
//! let mut builder = Path::builder();
//!
//! // Build a simple path.
//! let mut builder = Path::builder();
//! builder.move_to(point(0.0, 0.0));
//! builder.line_to(point(1.0, 2.0));
//! builder.line_to(point(2.0, 0.0));
//! builder.line_to(point(1.0, 1.0));
//! builder.close();
//!
//! // Generate the actual path object.
//! let path = builder.build();
//!
//! for event in &path {
//!     println!("{:?}", event);
//! }
//! # }
//! ```
//!

extern crate lyon_core as core;
extern crate lyon_path_builder as path_builder;
extern crate lyon_path_iterator as path_iterator;

mod path;

pub use path::*;
