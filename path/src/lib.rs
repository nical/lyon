#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]

//! Data structures and traits to work with paths (vector graphics).
//!
//! To build and consume paths, see the [builder](builder/index.html) and
//! [iterator](iterator/index.html) modules.
//!
//! # Examples
//!
//! ```
//! # extern crate lyon_path;
//! # fn main() {
//! use lyon_path::default::Path;
//! use lyon_path::math::{point};
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

pub extern crate lyon_geom as geom;

#[cfg(feature = "serialization")]
#[macro_use]
pub extern crate serde;

mod events;
mod path_state;
pub mod default;
pub mod iterator;
pub mod builder;

pub use events::*;
pub use path_state::*;
pub use geom::ArcFlags;
pub use geom::math as math;
