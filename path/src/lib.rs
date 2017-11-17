#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]

//! # Lyon path
//!
//! A simple path data structure implementing the traits provided in the
//! [builder](https://docs.rs/lyon_path_builder) and
//! [iterator](https://docs.rs/lyon_path_iterator) modules.
//! TODO(nical) fix links!
//!
//! # Examples
//!
//! ```
//! # extern crate lyon_core;
//! # extern crate lyon_path;
//! # fn main() {
//! use lyon_path::Path;
//! use lyon_core::math::{point};
//! use lyon_path::builder::*;
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
extern crate lyon_bezier as bezier;

mod path;
pub mod iterator;
pub mod builder;

pub use path::*;
