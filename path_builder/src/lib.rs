//! # Lyon path builder
//!
//! Tools to build path objects from a sequence of imperative commands.
//!
//! ## Examples
//!
//! The following example shows the Builder struct from the
//! [lyon_path](https://docs.rs/lyon_path/*/lyon_path) crate using the
//! [BaseBuilder](traits.BaseBuilder.html) interface.
//!
//! ```ignore
//! use lyon_path::Path;
//! use lyon_core::math::{point};
//! use lyon_path_builder::*;
//!
//! // Create a builder object to build the path.
//! let mut builder = Path::builder();
//!
//! // Build a simple path using the BaseBuilder interface.
//! builder.move_to(point(0.0, 0.0));
//! builder.line_to(point(1.0, 2.0));
//! builder.line_to(point(2.0, 0.0));
//! builder.line_to(point(1.0, 1.0));
//! builder.close();
//!
//! // Finish building and create the actual path object.
//! let path = builder.build();
//! ```
//!
//! The next example uses the [PathBuilder](traits.PathBuilder.html) trait, which adds
//! some simple curves to the [BaseBuilder](traits.BaseBuilder.html) trait.
//!
//! ```ignore
//! let mut builder = Path::builder();
//!
//! builder.move_to(point(0.0, 0.0));
//! builder.line_to(point(1.0, 0.0));
//! builder.quadratic_bezier_to(point(2.0, 0.0), point(2.0, 1.0));
//! builder.cubic_bezier_to(point(2.0, 2.0), point(0.0, 2.0), point(0.0, 0.0));
//! builder.close();
//!
//! let path = builder.build();
//! ```
//!
//! The [SvgBuilder](trait.SvgBuilder.html) Adds to [PathBuilder](traits.PathBuilder.html)
//! the rest of the [SVG path](https://svgwg.org/specs/paths/) commands.
//!
//! These SVG commands can approximated with the simpler set of commands supported by
//! [PathBuilder](traits.PathBuilder.html). Therefore it is possible to create an SvgBuilder
//! adapter on top of a PathBuilder using the with_svg method:
//!
//! ```ignore
//! let mut builder = Path::builder().with_svg();
//!
//! builder.move_to(point(0.0, 0.0));
//! builder.horizontal_line_to(1.0);
//! builder.relative_quadratic_bezier_to(point(1.0, 0.0), point(1.0, 1.0));
//! builder.smooth_relative_quadratic_bezier_to(point(-1.0, 1.0));
//!
//! let path = builder.build();
//! ```
//!
//! To build a path that approximates curves with a sequence of line segments, use the
//! flattened method:
//!
//! ```ignore
//! let tolerance = 0.05;// maximum distance between a curve and its approximation.
//! let mut builder = Path::builder().flattened(tolerance);
//!
//! builder.move_to(point(0.0, 0.0));
//! builder.quadratic_bezier_to(point(1.0, 0.0), point(1.0, 1.0));
//! builder.close();
//!
//! // The resulting path contains only MoveTo, LineTo and Close events.
//! let path = builder.build();
//! ```
//!

extern crate lyon_core as core;
extern crate lyon_bezier as bezier;

mod path_builder;
mod arc;

pub use path_builder::*;