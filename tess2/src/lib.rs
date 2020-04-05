#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]
#![deny(bare_trait_objects)]

//! Alternative fill tessellation implementation using
//! [libtess2](https://github.com/memononen/libtess2).
//!
//! # Lyon libtess2 wrapper
//!
//! This crate provides an alternative path fill tessellator implemented
//! as a wrapper of the [libtess2](https://github.com/memononen/libtess2)
//! C library.
//!
//! The goal of this crate is to provide an alternative tessellator for
//! the potential cases where lyon_tessellation::FillTessellator is lacking
//! in features or robustness, and have something to compare the latter
//! against.
//!
//! ## Comparison with [lyon_tessellation::FillTessellator](https://docs.rs/lyon_tessellation/)
//!
//! Advantages:
//!
//! - Supports the `NonZero` fill rule.
//! - More robust against precision errors when paths have many self
//!   intersections very close to each other.
//!
//! Disadvantages:
//!
//! - About twice slower than lyon_tessellation's fill tessellator.
//! - Does not support computing vertex normals.
//! - Wrapper around a C library (as opposed to pure rust with no
//!   unsafe code).
//!
//! ## API
//!
//! In order to avoid any overhead, this crate introduces the
//! FlattenedPath type which stores already-flattened paths
//! in the memory layout expected by libtess2.
//! Instead of working with a `GeometryBuilder` like the tessellators
//! in `lyon_tessellation`, this tessellator uses a `GeometryReceiver`
//! trait that corresponds to the way libtess2 exposes its output.
//!
//! ## Example
//!
//! ```
//! extern crate lyon_tess2 as tess2;
//! use tess2::{FillTessellator, FillOptions};
//! use tess2::math::{Point, point};
//! use tess2::path::Path;
//! use tess2::path::builder::*;
//! use tess2::path::iterator::*;
//! use tess2::flattened_path::FlattenedPath;
//! use tess2::tessellation::geometry_builder::*;
//!
//! fn main() {
//!     // Create a simple path.
//!     let mut path_builder = Path::builder();
//!     path_builder.begin(point(0.0, 0.0));
//!     path_builder.line_to(point(1.0, 2.0));
//!     path_builder.line_to(point(2.0, 0.0));
//!     path_builder.line_to(point(1.0, 1.0));
//!     path_builder.end(true);
//!     let path = path_builder.build();
//!
//!     // Create the destination vertex and index buffers.
//!     let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
//!
//!     {
//!         // Create the tessellator.
//!         let mut tessellator = FillTessellator::new();
//!
//!         // Compute the tessellation.
//!         let result = tessellator.tessellate(
//!             &path,
//!             &FillOptions::default(),
//!             &mut simple_builder(&mut buffers)
//!         );
//!         assert!(result.is_ok());
//!     }
//!     println!("The generated vertices are: {:?}.", &buffers.vertices[..]);
//!     println!("The generated indices are: {:?}.", &buffers.indices[..]);
//!
//! }
//! ```

pub extern crate lyon_tessellation as tessellation;
pub extern crate tess2_sys;
pub use tessellation::geom;
pub use tessellation::math;
pub use tessellation::path;

pub mod flattened_path;
mod tessellator;

pub use crate::tessellation::FillOptions;
pub use crate::tessellator::FillTessellator;
