#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]
#![deny(bare_trait_objects)]

//! Tessellation of 2D fill and stroke operations.
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
//! This crate is reexported in [lyon](https://docs.rs/lyon/).
//!
//! ## Overview
//!
//! The most interesting types and traits of this crate are:
//!
//! * [FillTessellator](struct.FillTessellator.html) - Tessellator for complex path fill operations.
//! * [StrokeTessellator](struct.StrokeTessellator.html) - Tessellator for complex path stroke operations.
//! * [`GeometryBuilder`](geometry_builder/trait.GeometryBuilder.html) - (See the documentation of the
//!   [geometry_builder module](geometry_builder/index.html)) which the above two are built on. This trait
//!   provides an interface for types that help with building and assembling the vertices and triangles that
//!   form the tessellation, usually in the form of arbitrary vertex and index buffers.
//! * The various specialized tessellators in the [`basic_shapes`](basic_shapes/index.html) modules.
//!
//! ## The tessellation pipeline
//!
//! <svg xmlns="http://www.w3.org/2000/svg" width="280mm" height="42mm" viewBox="0 0 280 42">
//!   <defs>
//!     <marker id="e" orient="auto" overflow="visible">
//!       <path fill="#59f" fill-rule="evenodd" stroke="#59f" stroke-width=".532" d="M-4 0l-2 2 7-2-7-2z"/>
//!     </marker>
//!     <marker id="d" orient="auto" overflow="visible">
//!       <path fill-rule="evenodd" stroke="#000" stroke-width=".532" d="M-4 0l-2 2 7-2-7-2z"/>
//!     </marker>
//!     <marker id="c" orient="auto" overflow="visible">
//!       <path fill="#59f" fill-rule="evenodd" stroke="#59f" stroke-width=".532" d="M-4 0l-2 2 7-2-7-2z"/>
//!     </marker>
//!     <marker id="b" orient="auto" overflow="visible">
//!       <path fill-rule="evenodd" stroke="#000" stroke-width=".532" d="M-4 0l-2 2 7-2-7-2z"/>
//!     </marker>
//!     <marker id="a" orient="auto" overflow="visible">
//!       <path fill-rule="evenodd" stroke="#000" stroke-width=".532" d="M-4 0l-2 2 7-2-7-2z"/>
//!     </marker>
//!   </defs>
//!   <path fill="#fff" stroke="#000" stroke-opacity=".56" stroke-width=".26" stroke-miterlimit="4.27" d="M39.55 17.37h15.8l2.15-1.7 2.06 1.7h15.36V38.8H39.55zM194.65 31.3h21.58l2.1-1.83 2.04 1.82h35.07v7.07h-60.8zM77.7 19.5h54.6l3.3-2.58 3.17 2.57h52.56v19H77.7z" color="#000" overflow="visible" stroke-linecap="round" stroke-linejoin="round"/>
//!   <g color="#000">
//!     <path fill="#80b3ff" d="M194.6 20.37h50.65v8.73H194.6z" overflow="visible"/>
//!     <path fill="#d5f6ff" d="M194.6 19.3h50.65v8.74H194.6z" overflow="visible"/>
//!   </g>
//!   <g color="#000">
//!     <path fill="#2a7fff" d="M221.6 5.74h21.56v8.73H221.6z" overflow="visible"/>
//!     <path fill="#d5f6ff" d="M221.6 4.68h21.56v8.73H221.6z" overflow="visible"/>
//!   </g>
//!   <g color="#000">
//!     <path fill="#2a7fff" d="M154.38 5.74h47.4v8.73h-47.4z" overflow="visible"/>
//!     <path fill="#d5f6ff" d="M154.38 4.68h47.4v8.73h-47.4z" overflow="visible"/>
//!   </g>
//!   <g color="#000">
//!     <path fill="#2a7fff" d="M91.94 5.74h39.34v8.73H91.94z" overflow="visible"/>
//!     <path fill="#d5f6ff" d="M91.94 4.68h39.34v8.73H91.94z" overflow="visible"/>
//!   </g>
//!   <g color="#000">
//!     <path fill="#2a7fff" d="M3.04 5.74H75.2v8.73H3.03z" overflow="visible"/>
//!     <path fill="#d5f6ff" d="M3.04 4.68H75.2v8.73H3.03z" overflow="visible"/>
//!   </g>
//!   <text x="93.73" y="266.09" stroke-width=".26" style="line-height:6.61458302px" font-size="5.29" font-family="Sans" letter-spacing="0" word-spacing="0" transform="translate(0 -255)">
//!     <tspan x="93.73" y="266.09">FillTessellator</tspan>
//!   </text>
//!   <text x="155.37" y="265.58" stroke-width=".26" style="line-height:6.61458302px" font-size="5.29" font-family="Sans" letter-spacing="0" word-spacing="0" transform="translate(0 -255)">
//!     <tspan x="155.37" y="265.58">GeometryBuilder</tspan>
//!   </text>
//!   <text x="223.1" y="266.02" stroke-width=".26" style="line-height:6.61458302px" font-size="5.29" font-family="Sans" letter-spacing="0" word-spacing="0" transform="translate(0 -255)">
//!     <tspan x="223.1" y="266.02">output</tspan>
//!   </text>
//!   <text x="196.17" y="280.9" stroke-width=".26" style="line-height:6.61458302px" font-size="5.29" font-family="Sans" letter-spacing="0" word-spacing="0" transform="translate(0 -255)">
//!     <tspan x="196.17" y="280.9">VertexConstructor</tspan>
//!   </text>
//!   <text x="5.13" y="266.09" stroke-width=".26" style="line-height:6.61458302px" font-size="5.29" font-family="Sans" letter-spacing="0" word-spacing="0" transform="translate(0 -255)">
//!     <tspan x="5.13" y="266.09">Iterator&lt;PathEvent&gt;</tspan>
//!   </text>
//!   <text x="79.79" y="282.2" stroke-width=".26" style="line-height:6.61458302px" font-size="5.29" font-family="Sans" letter-spacing="0" word-spacing="0" transform="translate(0 -255)">
//!     <tspan x="79.79" y="282.2" fill="navy" font-size="4.23">builder.add_vertex(FillVertex) -&gt; VertexId;</tspan><tspan x="79.79" y="289.09" fill="navy" font-size="4.23">builder.add_triangle(VertexId, <tspan stroke-width=".07" style="line-height:1.75010836px;font-variant-ligatures:normal;font-variant-position:normal;font-variant-caps:normal;font-variant-numeric:normal;font-variant-alternates:normal;font-variant-east-asian:normal;font-feature-settings:normal;text-indent:0;text-align:start;text-decoration-line:none;text-decoration-style:solid;text-decoration-color:#000000;text-transform:none;text-orientation:mixed;shape-padding:0" white-space="normal">VertexId, VertexId);</tspan></tspan>
//!   </text>
//!   <path fill="none" stroke="#000" stroke-width=".3" stroke-miterlimit="4.4" d="M76.94 265l13.64-.1" marker-end="url(#a)" transform="translate(0 -255)"/>
//!   <path fill="none" stroke="#000" stroke-width=".3" stroke-miterlimit="4.4" d="M132.86 265l19.55-.1" marker-end="url(#b)" transform="translate(0 -255)"/>
//!   <path fill="#59f" fill-rule="evenodd" stroke="#59f" stroke-width=".3" stroke-miterlimit="4.4" d="M203.38 264.53l8.27 8.26" marker-end="url(#c)" transform="translate(0 -255)"/>
//!   <path fill="none" stroke="#000" stroke-width=".3" stroke-miterlimit="4.4" d="M203.38 264.53l16 .06" marker-end="url(#d)" transform="translate(0 -255)"/>
//!   <text x="196.69" y="291.41" stroke-width=".26" style="line-height:6.61458302px" font-size="5.29" font-family="Sans" letter-spacing="0" word-spacing="0" transform="translate(0 -255)">
//!     <tspan x="196.69" y="291.41" fill="navy" font-size="4.23">FillVertex -&gt; CustomVertex</tspan>
//!   </text>
//!   <path fill="#59f" fill-rule="evenodd" stroke="#59f" stroke-width=".3" stroke-miterlimit="4.4" d="M212.97 272.98l6.75-6.5" marker-end="url(#e)" transform="translate(0 -255)"/>
//!   <g fill="none" stroke="#000" stroke-width=".26">
//!     <path d="M7.2 30.1l2.98 1.72h3.24l1.78-1.8 2.62-.75 2.08 1.83-1.6 2.87-5.64 1.54-3.5-1.62zM32.6 30.1l-3 1.72H26.4l-1.78-1.8-2.62-.75-2.08 1.83 1.6 2.87 5.64 1.54 3.5-1.62zM15 20.67l-.5 4.42 1.34 1 1.63-1.57-1.06-4.03zM24.53 20.67l.5 4.42-1.33 1-1.63-1.57 1.06-4.03z"/>
//!   </g>
//!   <path fill="#b7c8c4" fill-rule="evenodd" stroke="#000" stroke-width=".15" d="M251.68 19.5l2.98 1.74h3.23l1.78-1.8 2.62-.75 2.07 1.82-1.6 2.87-5.63 1.53-3.5-1.63z" stroke-linecap="round" stroke-linejoin="round"/>
//!   <path fill="#b7c8c4" fill-rule="evenodd" stroke="#000" stroke-width=".15" d="M277.07 19.5l-2.98 1.74h-3.24l-1.8-1.8-2.6-.75-2.1 1.82L266 23.4l5.63 1.53 3.5-1.63zM259.48 10.08l-.5 4.42 1.33 1 1.65-1.55-1.07-4.03zM269 10.08l.52 4.42-1.34 1-1.64-1.55 1.07-4.03z" stroke-linecap="round" stroke-linejoin="round"/>
//!   <path fill="none" stroke="#000" stroke-width=".15" d="M258.97 14.5l2.98-.55-2.47-3.87M266.54 13.95l2.98.55-1.9-4.58M254.66 21.24l-1 2.06 4.23-2.06-.76 3.7 2.54-5.5 3.1 3.95-.48-4.7M275.1 23.3l-1-2.06-2.5 3.7-.74-3.7-4.4-2.55-.48 4.7 4.88-2.16" stroke-linecap="round" stroke-linejoin="round"/>
//!   <text x="43.5" y="277.68" stroke-width=".26" style="line-height:6.61458349px" font-size="5.29" font-family="Sans" letter-spacing="0" word-spacing="0" transform="translate(0 -255)">
//!     <tspan x="43.5" y="277.68" fill="navy" font-size="3.88">MoveTo(Point)</tspan><tspan x="43.5" y="284.66" fill="navy" font-size="3.88">LineTo(Point)</tspan><tspan x="43.5" y="291.65" fill="navy" font-size="3.88">Close</tspan>
//!   </text>
//! </svg>
//!
//! The figure above shows each step of the fill tessellation pipeline.
//! Tessellating strokes works the same way using `StrokeVertex` instead of `FillVertex`.
//!
//! ### The input: iterators
//!
//! The path tessellators are not tied to a particular data structure. Instead they consume
//! iterators of flattened path events.
//! A [Path struct](https://docs.rs/lyon_path/*/lyon_path/struct.Path.html) in the crate
//! [lyon_path](https://docs.rs/lyon_path/*/lyon_path/) is provided for convenience
//! (but is optional).
//!
//! ### The output: geometry builders
//!
//! The tessellators are parametrized over a type implementing the
//! [GeometryBuilder trait](geometry_builder/trait.GeometryBuilder.html).
//! This trait provides some simple methods to add vertices and triangles, without enforcing
//! any particular representation for the resulting geometry. This is important because each
//! application will usually want to work with its own vertex type tailored a certain rendering
//! model.
//!
//! Applications can implement the ```GeometryBuilder<Point>``` trait in order to
//! generate vertex buffers and index buffers with custom vertex types.
//!
//! The structs [VertexBuffers](geometry_builder/struct.VertexBuffers.html) and
//! [geometry_buider::BuffersBuilder](geometry_builder/struct.BuffersBuilder.html) are provided
//! for convenience. `VertexBuffers<T>` is contains a `Vec<T>` for the vertices and a `Vec<u16>`
//! for the indices.
//!
//! `BuffersBuilder` is generic over a `VertexConstructor<InputVertex, OutputVertex>` trait which
//! creates the application's output vertices from the tessellator input vertices (either `FillVertex`
//! or `StrokeVertex`).
//!
//! ### Rendering the tessellated geometry
//!
//! The tessellators produce geometry in the form of vertex and index buffers which are expected
//! to be rendered using the equivalent of OpenGL's `glDrawElements` with mode `GL_TRIANGLES` available
//! under various names in the different graphics APIs.
//! There is a [basic example](https://github.com/nical/lyon/tree/master/examples/gfx_basic) showing how
//! it can be done with gfx-rs.
//!
//! ### Flattening and tolerance
//!
//! Most tessellators in this crate currently operate on flattened paths (paths or shapes represented
//! by sequences of line segments). when paths contain bézier curves or arcs, the latter need to be
//! approximated with sequences of line segments. This approximation depends on a `tolerance` parameter
//! which represents the maximum distance between a curve and its flattened approximation.
//!
//! More explanation about flattening and tolerance in the [lyon_geom crate](https://docs.rs/lyon_geom/#flattening).
//!
//! ## Examples
//!
//! - [Tessellating path fills](path_fill/struct.FillTessellator.html#examples).
//! - [Tessellating path strokes](path_stroke/struct.StrokeTessellator.html#examples).
//! - [Generating custom vertices](geometry_builder/index.html#generating-custom-vertices).
//! - [Generating completely custom output](geometry_builder/index.html#generating-a-completely-custom-output).
//! - [Writing a tessellator](geometry_builder/index.html#writing-a-tessellator).
//!

#![allow(dead_code)]
//#![allow(needless_return, new_without_default_derive)] // clippy

pub use lyon_path as path;

#[cfg(test)] use lyon_extra as extra;

#[cfg(feature = "serialization")]
#[macro_use]
pub extern crate serde;

pub mod basic_shapes;
pub mod geometry_builder;
pub mod debugger;
mod path_fill;
mod path_stroke;
mod math_utils;
mod fixed;

#[cfg(feature = "experimental")]
pub mod experimental;

#[cfg(test)]
mod earcut_tests;
#[cfg(test)]
mod fill_tests;
#[cfg(test)]
mod fuzz_tests;

pub use crate::path::math;

pub use crate::path::geom;

#[doc(inline)]
pub use crate::path_fill::*;

#[doc(inline)]
pub use crate::path_stroke::*;

#[doc(inline)]
pub use crate::geometry_builder::{GeometryBuilder, GeometryReceiver, VertexBuffers, BuffersBuilder, VertexConstructor, Count};

pub use crate::path::FillRule;

/// The fill tessellator's result type.
pub type TessellationResult = Result<Count, TessellationError>;

/// The fill tessellator's error enumeration.
#[derive(Clone, Debug, PartialEq)]
pub enum TessellationError {
    UnsupportedParamater,
    InvalidVertex,
    TooManyVertices,
    Internal(InternalError)
}

/// Something unexpectedly put the tessellator in a bad state.
///
/// If you run into this error code, please [file an issue](https://github.com/nical/lyon/issues).
#[derive(Clone, Debug, PartialEq)]
pub enum InternalError {
    E01,
    E02,
    E03,
    E04,
}

/// Left or right.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub fn opposite(self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    pub fn is_left(self) -> bool { self == Side::Left }

    pub fn is_right(self) -> bool { self == Side::Right }
}

/// Before or After. Used to describe position relative to a join.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum Order {
    Before,
    After,
}

impl Order {
    pub fn opposite(self) -> Self {
        match self {
            Order::Before => Order::After,
            Order::After => Order::Before,
        }
    }

    pub fn is_before(self) -> bool { self == Order::Before }

    pub fn is_after(self) -> bool { self == Order::After }
}

/// Vertex produced by the stroke tessellators.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct StrokeVertex {
    /// Position of the vertex (on the path, the consumer should move the point along
    /// the provided normal in order to give the stroke a width).
    pub position: math::Point,
    /// Normal at this vertex such that extruding the vertices along the normal would
    /// produce a stroke of width 2.0 (1.0 on each side). This vector is not normalized.
    pub normal: math::Vector,
    /// How far along the path this vertex is.
    pub advancement: f32,
    /// Whether the vertex is on the left or right side of the path.
    pub side: Side,
}

/// Vertex produced by the fill tessellators.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct FillVertex {
    /// Position of the vertex (on the path).
    pub position: math::Point,
    /// Normal at this vertex such that extruding the vertices along the normal would
    /// produce a stroke of width 2.0 (1.0 on each side). This vector is not normalized.
    ///
    /// Note that some tessellators aren't fully implemented and don't provide the
    /// normal (a nil vector is provided instead). Refer the documentation of each tessellator.
    pub normal: math::Vector,
}

/// Line cap as defined by the SVG specification.
///
/// See: https://svgwg.org/specs/strokes/#StrokeLinecapProperty
///
/// <svg viewBox="0 0 400 399.99998" height="400" width="400">
///   <g transform="translate(0,-652.36229)">
///     <path style="opacity:1;fill:#80b3ff;stroke:#000000;stroke-width:1;stroke-linejoin:round;" d="m 240,983 a 30,30 0 0 1 -25,-15 30,30 0 0 1 0,-30.00001 30,30 0 0 1 25.98076,-15 l 0,30 z"/>
///     <path style="fill:#80b3ff;stroke:#000000;stroke-width:1px;stroke-linecap:butt;" d="m 390,782.6 -150,0 0,-60 150,0.5"/>
///     <circle style="opacity:1;fill:#ff7f2a;stroke:#000000;stroke-width:1;stroke-linejoin:round;" r="10" cy="752.89227" cx="240.86813"/>
///     <path style="fill:none;stroke:#000000;stroke-width:1px;stroke-linejoin:round;" d="m 240,722.6 150,60"/>
///     <path style="fill:#80b3ff;stroke:#000000;stroke-width:1px;stroke-linecap:butt;" d="m 390,882 -180,0 0,-60 180,0.4"/>
///     <circle style="opacity:1;fill:#ff7f2a;stroke:#000000;stroke-width:1;stroke-linejoin:round;" cx="239.86813" cy="852.20868" r="10" />
///     <path style="fill:none;stroke:#000000;stroke-width:1px;stroke-linejoin:round;" d="m 210.1,822.3 180,60"/>
///     <path style="fill:#80b3ff;stroke:#000000;stroke-width:1px;stroke-linecap:butt;" d="m 390,983 -150,0 0,-60 150,0.4"/>
///     <circle style="opacity:1;fill:#ff7f2a;stroke:#000000;stroke-width:1;stroke-linejoin:round;" cx="239.86813" cy="953.39734" r="10" />
///     <path style="fill:none;stroke:#000000;stroke-width:1px;stroke-linejoin:round;" d="m 390,983 -150,-60 L 210,953 l 30,30 -21.5,-9.5 L 210,953 218.3,932.5 240,923.4"/>
///     <text y="757.61273" x="183.65314" style="font-style:normal;font-weight:normal;font-size:20px;line-height:125%;font-family:Sans;text-align:end;text-anchor:end;fill:#000000;stroke:none;">
///        <tspan y="757.61273" x="183.65314">LineCap::Butt</tspan>
///        <tspan y="857.61273" x="183.65314">LineCap::Square</tspan>
///        <tspan y="957.61273" x="183.65314">LineCap::Round</tspan>
///      </text>
///   </g>
/// </svg>
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum LineCap {
    /// The stroke for each sub-path does not extend beyond its two endpoints.
    /// A zero length sub-path will therefore not have any stroke.
    Butt,
    /// At the end of each sub-path, the shape representing the stroke will be
    /// extended by a rectangle with the same width as the stroke width and
    /// whose length is half of the stroke width. If a sub-path has zero length,
    /// then the resulting effect is that the stroke for that sub-path consists
    /// solely of a square with side length equal to the stroke width, centered
    /// at the sub-path's point.
    Square,
    /// At each end of each sub-path, the shape representing the stroke will be extended
    /// by a half circle with a radius equal to the stroke width.
    /// If a sub-path has zero length, then the resulting effect is that the stroke for
    /// that sub-path consists solely of a full circle centered at the sub-path's point.
    Round,
}

/// Line join as defined by the SVG specification.
///
/// See: https://svgwg.org/specs/strokes/#StrokeLinejoinProperty
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum LineJoin {
    /// A sharp corner is to be used to join path segments.
    Miter,
    /// Same as a miter join, but if the miter limit is exceeded,
    /// the miter is clipped at a miter length equal to the miter limit value
    /// multiplied by the stroke width.
    MiterClip,
    /// A round corner is to be used to join path segments.
    Round,
    /// A bevelled corner is to be used to join path segments.
    /// The bevel shape is a triangle that fills the area between the two stroked
    /// segments.
    Bevel,
}

/// Parameters for the tessellator.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct StrokeOptions {
    /// What cap to use at the start of each sub-path.
    ///
    /// Default value: `LineCap::Butt`.
    pub start_cap: LineCap,

    /// What cap to use at the end of each sub-path.
    ///
    /// Default value: `LineCap::Butt`.
    pub end_cap: LineCap,

    /// See the SVG specification.
    ///
    /// Default value: `LineJoin::Miter`.
    pub line_join: LineJoin,

    /// Line width
    ///
    /// Default value: `StrokeOptions::DEFAULT_LINE_WIDTH`.
    pub line_width: f32,

    /// See the SVG specification.
    ///
    /// Must be greater than or equal to 1.0.
    /// Default value: `StrokeOptions::DEFAULT_MITER_LIMIT`.
    pub miter_limit: f32,

    /// Maximum allowed distance to the path when building an approximation.
    ///
    /// See [Flattening and tolerance](index.html#flattening-and-tolerance).
    /// Default value: `StrokeOptions::DEFAULT_TOLERANCE`.
    pub tolerance: f32,

    /// Apply line width
    ///
    /// When set to false, the generated vertices will all be positioned in the centre
    /// of the line. The width can be applied later on (eg in a vertex shader) by adding
    /// the vertex normal multiplied by the line with to each vertex position.
    ///
    /// Default value: `true`.
    pub apply_line_width: bool,

    // To be able to add fields without making it a breaking change, add an empty private field
    // which makes it impossible to create a StrokeOptions without calling the constructor.
    _private: (),
}

impl StrokeOptions {
    /// Minimum miter limit as defined by the SVG specification.
    ///
    /// See [StrokeMiterLimitProperty](https://svgwg.org/specs/strokes/#StrokeMiterlimitProperty)
    pub const MINIMUM_MITER_LIMIT: f32 = 1.0;
    /// Default miter limit as defined by the SVG specification.
    ///
    /// See [StrokeMiterLimitProperty](https://svgwg.org/specs/strokes/#StrokeMiterlimitProperty)
    pub const DEFAULT_MITER_LIMIT: f32 = 4.0;
    pub const DEFAULT_LINE_CAP: LineCap = LineCap::Butt;
    pub const DEFAULT_LINE_JOIN: LineJoin = LineJoin::Miter;
    pub const DEFAULT_LINE_WIDTH: f32 = 1.0;
    pub const DEFAULT_TOLERANCE: f32 = 0.1;

    pub const DEFAULT: Self = StrokeOptions {
        start_cap: Self::DEFAULT_LINE_CAP,
        end_cap: Self::DEFAULT_LINE_CAP,
        line_join: Self::DEFAULT_LINE_JOIN,
        line_width: Self::DEFAULT_LINE_WIDTH,
        miter_limit: Self::DEFAULT_MITER_LIMIT,
        tolerance: Self::DEFAULT_TOLERANCE,
        apply_line_width: true,
        _private: (),
    };

    #[inline]
    pub fn tolerance(tolerance: f32) -> Self {
        Self::DEFAULT.with_tolerance(tolerance)
    }

    #[inline]
    pub fn with_tolerance(mut self, tolerance: f32) -> Self {
        self.tolerance = tolerance;
        self
    }

    #[inline]
    pub fn with_line_cap(mut self, cap: LineCap) -> Self {
        self.start_cap = cap;
        self.end_cap = cap;
        self
    }

    #[inline]
    pub fn with_start_cap(mut self, cap: LineCap) -> Self {
        self.start_cap = cap;
        self
    }

    #[inline]
    pub fn with_end_cap(mut self, cap: LineCap) -> Self {
        self.end_cap = cap;
        self
    }

    #[inline]
    pub fn with_line_join(mut self, join: LineJoin) -> Self {
        self.line_join = join;
        self
    }

    #[inline]
    pub fn with_line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }

    #[inline]
    pub fn with_miter_limit(mut self, limit: f32) -> Self {
        assert!(limit >= Self::MINIMUM_MITER_LIMIT);
        self.miter_limit = limit;
        self
    }

    #[inline]
    pub fn dont_apply_line_width(mut self) -> Self {
        self.apply_line_width = false;
        self
    }
}

/// Parameters for the fill tessellator.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct FillOptions {
    /// Maximum allowed distance to the path when building an approximation.
    ///
    /// See [Flattening and tolerance](index.html#flattening-and-tolerance).
    ///
    /// Default value: `FillOptions::DEFAULT_TOLERANCE`.
    pub tolerance: f32,

    /// Set the fill rule.
    ///
    /// See the [SVG specification](https://www.w3.org/TR/SVG/painting.html#FillRuleProperty).
    /// Currently, only the `EvenOdd` rule is implemented.
    ///
    /// Default value: `EvenOdd`.
    pub fill_rule: FillRule,

    /// Whether or not to compute the normal vector at each vertex.
    ///
    /// When set to false, all generated vertex normals are equal to `vector(0.0, 0.0)`.
    /// Not computing vertex normals can speed up tessellation and enable generating less vertices
    /// at intersections.
    ///
    /// Default value: `true`.
    pub compute_normals: bool,

    /// A fast path to avoid some expensive operations if the path is known to
    /// not have any self-intersections.
    ///
    /// Do not set this to `true` if the path may have intersecting edges else
    /// the tessellator may panic or produce incorrect results. In doubt, do not
    /// change the default value.
    ///
    /// Default value: `false`.
    pub assume_no_intersections: bool,

    /// What to do if the tessellator detects an error.
    pub on_error: OnError,

    // To be able to add fields without making it a breaking change, add an empty private field
    // which makes it impossible to create a FillOptions without the calling constructor.
    _private: (),
}

impl Default for StrokeOptions {
    fn default() -> Self { Self::DEFAULT }
}

impl FillOptions {
    /// Default flattening tolerance.
    pub const DEFAULT_TOLERANCE: f32 = 0.1;
    /// Default Fill rule.
    pub const DEFAULT_FILL_RULE: FillRule = FillRule::EvenOdd;

    pub const DEFAULT: Self = FillOptions {
        tolerance: Self::DEFAULT_TOLERANCE,
        fill_rule: Self::DEFAULT_FILL_RULE,
        compute_normals: true,
        assume_no_intersections: false,
        on_error: OnError::DEFAULT,
        _private: (),
    };

    #[inline]
    pub fn even_odd() -> Self { Self::DEFAULT }

    #[inline]
    pub fn tolerance(tolerance: f32) -> Self {
        Self::DEFAULT.with_tolerance(tolerance)
    }

    #[inline]
    pub fn non_zero() -> Self {
        let mut options = Self::DEFAULT;
        options.fill_rule = FillRule::NonZero;
        options
    }

    #[inline]
    pub fn with_tolerance(mut self, tolerance: f32) -> Self {
        self.tolerance = tolerance;
        self
    }

    #[inline]
    pub fn with_normals(mut self, normals: bool) -> Self {
        self.compute_normals = normals;
        self
    }

    #[inline]
    pub fn assume_no_intersections(mut self) -> Self {
        self.assume_no_intersections = true;
        self
    }

    #[inline]
    pub fn on_error(mut self, policy: OnError) -> Self {
        self.on_error = policy;
        self
    }
}

impl Default for FillOptions {
    fn default() -> Self { Self::DEFAULT }
}

/// Defines the tessellator the should try to behave when detecting
/// an error.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum OnError {
    /// Panic as soon as the error is detected.
    ///
    /// Most suitable for testing.
    Panic,
    /// Interrupt tessellation and return an error.
    Stop,
    /// Attempt to continue if possible, stop otherwise.
    ///
    /// The resulting tessellation may be locally incorrect.
    Recover,
}

impl OnError {
    #[cfg(test)]
    pub const DEFAULT: Self = OnError::Panic;
    #[cfg(not(test))]
    pub const DEFAULT: Self = OnError::Stop;
}

impl Default for OnError {
    fn default() -> Self { Self::DEFAULT }
}


#[test]
fn test_without_miter_limit(){
    let expected_limit = 4.0;
    let stroke_options = StrokeOptions::default();

    assert_eq!(expected_limit, stroke_options.miter_limit);
}

#[test]
fn test_with_miter_limit(){
    let expected_limit = 3.0;
    let stroke_options = StrokeOptions::default().with_miter_limit(expected_limit);

    assert_eq!(expected_limit, stroke_options.miter_limit);
}

#[test]
#[should_panic]
fn test_with_invalid_miter_limit(){
    let _ = StrokeOptions::default().with_miter_limit(0.0);
}
