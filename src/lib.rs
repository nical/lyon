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
//! * Interested in contributing? Pull requests are welcome. If you would like to help but don't know
//!   what to do specifically, have a look at the [github issues](https://github.com/nical/lyon/issues),
//!   some of which are tagged as [easy](https://github.com/nical/lyon/issues?q=is%3Aissue+is%3Aopen+label%3Aeasy).
//!
//! # Examples
//!
//! ## Tessellating a rounded rectangle
//!
//! The `lyon_tessellation` crate provides a collection of tessellation routines
//! for common shapes such as rectangles and circles. Let's have a look at how
//! to obtain the fill tessellation a rectangle with rounded corners:
//!
//! ```
//! extern crate lyon;
//! use lyon::math::rect;
//! use lyon::tessellation::VertexBuffers;
//! use lyon::tessellation::basic_shapes::*;
//! use lyon::tessellation::geometry_builder::simple_builder;
//!
//! fn main() {
//!     let mut geometry = VertexBuffers::new();
//!
//!     let tolerance = 0.1;
//!
//!     fill_rounded_rectangle(
//!         &rect(0.0, 0.0, 100.0, 50.0),
//!         &BorderRadii {
//!             top_left: 10.0,
//!             top_right: 5.0,
//!             bottom_left: 20.0,
//!             bottom_right: 25.0,
//!         },
//!         tolerance,
//!         &mut simple_builder(&mut geometry),
//!     );
//!
//!     // The tessellated geometry is ready to be uploaded to the GPU.
//!     println!(" -- {} vertices {} indices",
//!         geometry.vertices.len(),
//!         geometry.indices.len()
//!     );
//! }
//!
//! ```
//!
//! ## Building and tessellating an arbitrary path
//!
//! ```
//! extern crate lyon;
//! use lyon::math::point;
//! use lyon::path::Path;
//! use lyon::path_builder::*;
//! use lyon::path_iterator::PathIterator;
//! use lyon::tessellation::{FillTessellator, FillOptions, VertexBuffers};
//! use lyon::tessellation::geometry_builder::simple_builder;
//!
//! fn main() {
//!     // Build a Path.
//!     let mut builder = Path::builder();
//!     builder.move_to(point(0.0, 0.0));
//!     builder.line_to(point(1.0, 0.0));
//!     builder.quadratic_bezier_to(point(2.0, 0.0), point(2.0, 1.0));
//!     builder.cubic_bezier_to(point(1.0, 1.0), point(0.0, 1.0), point(0.0, 0.0));
//!     builder.close();
//!     let path = builder.build();
//!
//!     // Will contain the result of the tessellation.
//!     let mut geometry = VertexBuffers::new();
//!
//!     let mut tessellator = FillTessellator::new();
//!
//!     {
//!         let mut geom_builder = simple_builder(&mut geometry);
//!
//!         let tolerance = 0.1;
//!
//!         // Compute the tessellation.
//!         tessellator.tessellate_path(
//!             path.path_iter().flattened(tolerance),
//!             &FillOptions::default(),
//!             &mut geom_builder
//!         ).unwrap();
//!     }
//!
//!     // The tessellated geometry is ready to be uploaded to the GPU.
//!     println!(" -- {} vertices {} indices",
//!         geometry.vertices.len(),
//!         geometry.indices.len()
//!     );
//! }
//! ```
//!
//! ## What is the tolerance variable in these examples?
//!
//! The tessellator operates on flattened paths (that only contains line segments)
//! so we have to approximate the curves segments with sequences of line segments.
//! To do so we pick a tolerance threshold which is the maximum distance allowed
//! between the curve and its approximation.
//! The documentation of the [lyon_bezier](https://docs.rs/lyon_bezier) crate provides
//! more detailed explanations about this tolerance parameter.
//!
//! ## Rendering the tessellated geometry
//!
//! Lyon does not provide with any GPU abstraction or rendering backend (for now).
//! It is up to the user of this crate to decide whether to use OpenGL, vulkan, gfx-rs,
//! glium, or any low level graphics API and how to render it.
//! The [basic](https://github.com/nical/lyon/tree/master/examples/gfx_basic) and
//! [advanced](https://github.com/nical/lyon/tree/master/examples/gfx_advanced) gfx-rs
//! examples can be used to get an idea of how to render the geometry (in this case
//! using gfx-rs).
//!
//! ## Which crates do I need?
//!
//! The meta-crate (`lyon`) mostly reexports the other lyon crates for convenience.
//!
//! ```ignore
//! extern crate lyon;
//! use lyon::tessellation::FillTessellator;
//! ```
//!
//! Is equivalent to:
//!
//! ```ignore
//! extern crate lyon_tessellation;
//! use lyon_tessellation::FillTessellator;
//! ```
//!
//! - The `lyon_tessellation` crate is the most interesting crate so is what most people using
//!   lyon are interested in. The tessellation algorithms don't depend on a specific data
//!   structure. Instead they work on iterators of path. When using the `lyon_tessellation`
//!   crate you'll almost always want to use the `lyon_path_iterator` crate as well.
//! - The `lyon_path_iterator` crate contains a colletion of tools to chain iterators
//!   of path events. These adapters are very useful to convert an iterator of SVG events
//!   (which contains various types of curves in relative and absolute coordinates) into
//!   iterator of simpler path events (every thing in absolute coordinates) all the way to
//!   flattened events (only line segments in absolute corrdinates).
//! - The `lyon_path` crate is completely optional. It contains a path data structure
//!   which work with the `lyon_path_iterator` (and thus works with `lyon_tessellation`) and
//!   `lyon_path_builder` crates. Various examples use it but anyone can implement a custom
//!   path data structure that works with the tessellators as long as it provides an iterator
//!   of path events.
//! - The `lyon_path_builder` crate is also optional, but provide useful abstractions to
//!   build path objects from sequences of function calls like `move_to`, `cubic_bezier_to`, etc.
//!   Just like `lyon_path_iterator` this crate provides adapters between the different types of
//!   path events, making it easy to use the full set of SVG events to build a path object that
//!   does not actually support all of them by converting events to lower level primitives on
//!   the fly.
//! - The `lyon_bezier` crate is really standalone as it does not depend on any other `lyon_*` crate.
//!   It implements useful quadratic and cubic bezier curve math, including the flattening
//!   algorithm that is used by `lyon_path_iterator` and `lyon_path_builder`.
//! - The `lyon_svg` crate contains utilities to interface with SVG. At the moment it is mostly
//!   a collection of wrappers around the excellent `svgparser` crate.
//! - The `lyon_core` crate contains internal details that are useful to all other lyon crates
//!   (except `lyon_bezier`). It is reexported by all crates and you should not have to interact
//!   directly with it.
//!
//! These crates are not very big, it's usually fine for most use-case to simply import the `lyon`
//! meta-crate, unless you are only interested in the bezier tools.
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
