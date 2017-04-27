//! # Lyon Tessellation
//!
//! This crate implements tools to compute the tessellation of 2d paths fill and stroke operations, in order to render them efficiently on the GPU.
//!
//! <svg viewBox="0 0 600.0 300.0" height="300" width="600">
//!   <g transform="translate(0,-752.36216)">
//!     <path style="fill:none;stroke:#ff9955;" d="m 346.4,790.7 186.8,11.2 -213.6,25.9 196.8,12.7 -185.1,41.4 192.4,24.4 -205.5,10.3 194.1,33.4 -97.3,25.5 -96.9,-58.9"/>
//!     <path style="fill:#ffb380;stroke:none;" d="m 59.1,965.3 -15.1,-48.7 13.0,-34.4 -11.5,-54.2 26.7,-37.1 73.2,-19.2 114.4,30.4 -17.0,38.0 1.1,15.3 13.4,-8.1 14.8,6.8 -5.2,48.2 -16.8,4.8 -11.4,43.5 -40.4,29.9 -31.3,-3.0 0,28.2 19.0,8.3 -44.2,19.0 -10.6,-25.1 9.9,-3.8 0,-28.2 z"/>
//!     <path style="fill:#de8787;stroke:none;" d="m 106.4,853.1 2.2,67.9 49.6,0.7 13.7,-22.9 -20.6,-3.8 -4.5,-44.2 -17.5,-13.7 z"/>
//!     <path style="fill:#a02c2c;stroke:none;" d="m 108.74845,940.94089 61.08369,3.05419 -13.74383,15.27092 -33.59604,-3.05418 z"/>
//!     <path style="fill:#784421;stroke:none;" d="m 176.93475,845.6345 20.51653,4.85918 -5.39908,30.7748 -12.68786,-1.88968 z"/>
//!     <path style="fill:#784421;stroke:none;" d="m 78.4,882.0 9.4,-3.2 3.7,-30.7 -13.2,8.6 z"/>
//!     <path style="fill:none;stroke:#ff9955;stroke-linecap:round;stroke-linejoin:round;" d="m 333.2,965.3 -15.1,-48.7 13.0,-34.4 -11.5,-54.2 26.7,-37.1 73.2,-19.2 114.4,30.4 -17.0,38.0 1.2,15.3 13.2,-8.0 14.8,6.8 -5.2,48.2 -16.8,4.8 -11.4,43.5 -40.4,29.9 -31.3,-3.0 0,28.2 19.0,8.3 -44.2,19.0 -10.6,-25.1 9.9,-3.8 0,-28.2 z"/>
//!     <path style="fill:none;stroke:#de8787;stroke-linecap:round;stroke-linejoin:round;" d="m 380.5,853.1 2.2,67.9 49.6,0.7 13.7,-22.9 -20.6,-3.8 -4.5,-44.2 -17.5,-13.7 z"/>
//!     <path style="fill:none;stroke:#a02c2c;stroke-linecap:round;stroke-linejoin:round;" d="m 382.8,940.9 61.0,3.0 -13.7,15.2 -33.5,-3.0 z"/>
//!     <path style="fill:none;stroke:#803300;stroke-linecap:round;stroke-linejoin:round;" d="m 451.0,845.6 20.5,4.8 -5.3,30.7 -12.6,-1.8 z"/>
//!     <path style="fill:none;stroke:#803300;stroke-linecap:round;stroke-linejoin:round;" d="m 352.5,882.0 9.4,-3.2 3.7,-30.7 -13.2,8.6 z"/>
//!     <path style="fill:none;stroke:#803300;stroke-linecap:round;stroke-linejoin:round;" d="m 352.5,856.4 9.3,22.2"/>
//!     <path style="fill:none;stroke:#de8787;stroke-linecap:round;stroke-linejoin:round;" d="m 380.8,853.3 40.2,-2.5 -38.3,70.1 42.6,-25.9 6.6,26.5"/>
//!     <path style="fill:none;stroke:#803300;stroke-linecap:round;stroke-linejoin:round;" d="m 471.4,850.4 -17.6,28.6"/>
//!     <path style="fill:none;stroke:#a02c2c;stroke-linecap:round;stroke-linejoin:round;" d="m 443.3,943.9 -46.7,12.2"/>
//!     <path style="fill:none;stroke:#ff9955;stroke-linecap:round;stroke-linejoin:round;" d="m 540.7,902.0 -22.5,-46.9 5.7,51.7"/>
//!     <path style="fill:none;stroke:#ff9955;stroke-linecap:round;stroke-linejoin:round;" d="m 512.2,950.2 -71.0,27.1 -26.1,-1.7 25.3,30.1 -25.1,-1.7 44.0,10.3 -53.8,-6.1"/>
//!     <path style="fill:none;stroke:#ff9955;stroke-linecap:round;stroke-linejoin:round;" d="m 518.5,8.3 27.5,-1.2"/>
//!   </g>
//! </svg>
//!
//! ## Overview
//!
//! The most interesting modules of this crate are:
//!
//! * [path_fill](path_fill/index.html) - Implementing the tessellation of complex path fill
//!   operations.
//! * [path_storke](path_storke/index.html) - Implementing the tessellation of complex path
//!   stroke operations.
//! * [geometry_builder](geometry_builder/index.html) - Which the above two are built on. It
//!   provides traits to facilitate generating arbitrary vertex and index buffers.
//!
//! ### The input: iterators
//!
//! The path tessellators are not tied to a particular data structure. Instead they consume
//! iterators of [path events](https://docs.rs/lyon_core/*/lyon_core/events/index.html).
//! A [Path struct](https://docs.rs/lyon_path/*/lyon_path/struct.Path.html) in the crate
//! [lyon_path](https://docs.rs/lyon_path/*/lyon_path/) is provided for convenience
//! (but is not mandatory).
//!
//! The fill tessellator builds a [FillEvents object](path_fill/struct.FillEvents.html) from
//! the iterator. It is an intermediate representation which can be cached if the path needs
//! to be tessellated again.
//!
//! ### The output: geometry builders
//!
//! The tessellators are parametrized over a type implementing the
//! [BezierGeometryBuilder trait](geometry_builder/trait.GeometryBuilder.html).
//! This trait provides some simple methods to add vertices and triangles, without enforcing
//! any particular representation for the resulting geometry. This is important because each
//! application has its own internal representation for the vertex and index buffers sent to
//! the GPU, and the tessellator needs to be able to write directly into these buffers without
//! enforcing a particular vertex layout.
//!
//! Each application will implement the ```BezierGeometryBuilder<Point>``` trait in order to
//! generate vertex buffers and index buffers any type of vertex they want taking a 2d Point
//! as input for each vertex.
//! The structs [VertexBuffers](geometry_builder/struct.VertexBuffers.html) and
//! [geometry_buider::BuffersBuilder](geometry_builder/struct.BuffersBuilder.html) are provided
//! for convenience.
//!
//! ## Examples
//!
//! See the examples in the [path_fill](path_fill/index.html) and [path_stroke](path_stroke/index.html)
//! modules documentation.
//!

#![allow(dead_code)]

extern crate lyon_core as core;
extern crate lyon_path_builder as path_builder;
extern crate lyon_bezier as bezier;

#[cfg(test)]
extern crate lyon_path as path;
#[cfg(test)]
extern crate lyon_path_iterator as path_iterator;
#[cfg(test)]
extern crate lyon_extra as extra;

pub mod basic_shapes;
pub mod path_fill;
pub mod path_stroke;
pub mod geometry_builder;

pub use core::*;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Side {
    Left,
    Right
}

/// Vertex produced by the stroke tessellators.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct StrokeVertex {
    /// Position of the vertex (on the path, the consumer should move the point along
    /// the provided normal in order to give the stroke a width).
    pub position: math::Point,
    /// Normal at this vertex such that extruding the vertices along the normal would
    /// produce a stroke of width 2.0 (1.0 on each side). This vector is not normalized.
    ///
    /// Note that some tessellators aren't fully implemented and don't provide the
    /// normal (a nil vector is provided instead). Refer the documentation of each tessellator.
    pub normal: math::Vec2,
    /// Whether the vertex is on the left or right side of the path.
    pub side: Side,
}

/// Vertex produced by the fill tessellators.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FillVertex {
    /// Position of the vertex (on the path).
    pub position: math::Point,
    /// Normal at this vertex such that extruding the vertices along the normal would
    /// produce a stroke of width 2.0 (1.0 on each side). This vector is not normalized.
    ///
    /// Note that some tessellators aren't fully implemented and don't provide the
    /// normal (a nil vector is provided instead). Refer the documentation of each tessellator.
    pub normal: math::Vec2,
}

