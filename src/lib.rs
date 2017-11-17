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
//! * [![crate](http://meritbadge.herokuapp.com/lyon_tessellation)](https://crates.io/crates/lyon_tessellation)
//!   [![doc](https://docs.rs/lyon_tessellation/badge.svg)](https://docs.rs/lyon_tessellation) -
//!   **lyon_tessellation** - Path tessellation routines.
//! * [![crate](http://meritbadge.herokuapp.com/lyon_path)](https://crates.io/crates/lyon_path)
//!   [![doc](https://docs.rs/lyon_path/badge.svg)](https://docs.rs/lyon_path) -
//!   **lyon_path** - Tools to build and iterate over paths.
//! * [![crate](http://meritbadge.herokuapp.com/lyon_bezier)](https://crates.io/crates/lyon_bezier)
//!   [![doc](https://docs.rs/lyon_bezier/badge.svg)](https://docs.rs/lyon_bezier) -
//!   **lyon_bezier** - Cubic and quadratic 2d bézier math.
//! * [![crate](http://meritbadge.herokuapp.com/lyon_svg)](https://crates.io/crates/lyon_svg)
//!   [![doc](https://docs.rs/lyon_svg/badge.svg)](https://docs.rs/lyon_svg) -
//!   **lyon_svg** - Create paths using SVG's path syntax.
//! * [![crate](http://meritbadge.herokuapp.com/lyon_extra)](https://crates.io/crates/lyon_extra)
//!   [![doc](https://docs.rs/lyon_extra/badge.svg)](https://docs.rs/lyon_extra) -
//!   **lyon_extra** - Additional testing and debugging tools.
//! * [![crate](http://meritbadge.herokuapp.com/lyon_core)](https://crates.io/crates/lyon_core)
//!   [![doc](https://docs.rs/lyon_core/badge.svg)](https://docs.rs/lyon_core) -
//!   **lyon_core** - Common types to most lyon crates (mostly for internal use, reexported by the other crates).
//!
//! This meta-crate (`lyon`) mostly reexports the other lyon crates for convenience.
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
//! use lyon::path::builder::*;
//! use lyon::path::iterator::PathIterator;
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
//!             path.path_iter(),
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



pub extern crate lyon_core;
pub extern crate lyon_path;
pub extern crate lyon_tessellation;
pub extern crate lyon_bezier;
pub extern crate lyon_extra;
pub extern crate lyon_svg;
//pub extern crate lyon_renderer;

pub use lyon_core::*;

pub use lyon_tessellation as tessellation;
pub use lyon_path as path;
pub use lyon_bezier as bezier;
pub use lyon_extra as extra;
pub use lyon_svg as svg;
//pub use lyon_renderer as renderer;
