#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]
#![deny(bare_trait_objects)]

//! Simple 2D geometric primitives on top of euclid.
//!
//! This crate is reexported in [lyon](https://docs.rs/lyon/).
//!
//! # Overview.
//!
//! This crate implements some of the maths to work with:
//!
//! - lines and line segments,
//! - quadratic and cubic bézier curves,
//! - elliptic arcs,
//! - triangles.
//!
//! # Flattening
//!
//! Flattening is the action of approximating a curve with a succession of line segments.
//!
//! <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 30" height="30mm" width="120mm">
//!   <path d="M26.7 24.94l.82-11.15M44.46 5.1L33.8 7.34" fill="none" stroke="#55d400" stroke-width=".5"/>
//!   <path d="M26.7 24.94c.97-11.13 7.17-17.6 17.76-19.84M75.27 24.94l1.13-5.5 2.67-5.48 4-4.42L88 6.7l5.02-1.6" fill="none" stroke="#000"/>
//!   <path d="M77.57 19.37a1.1 1.1 0 0 1-1.08 1.08 1.1 1.1 0 0 1-1.1-1.08 1.1 1.1 0 0 1 1.08-1.1 1.1 1.1 0 0 1 1.1 1.1" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M77.57 19.37a1.1 1.1 0 0 1-1.08 1.08 1.1 1.1 0 0 1-1.1-1.08 1.1 1.1 0 0 1 1.08-1.1 1.1 1.1 0 0 1 1.1 1.1" color="#000" fill="#fff"/>
//!   <path d="M80.22 13.93a1.1 1.1 0 0 1-1.1 1.1 1.1 1.1 0 0 1-1.08-1.1 1.1 1.1 0 0 1 1.1-1.08 1.1 1.1 0 0 1 1.08 1.08" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M80.22 13.93a1.1 1.1 0 0 1-1.1 1.1 1.1 1.1 0 0 1-1.08-1.1 1.1 1.1 0 0 1 1.1-1.08 1.1 1.1 0 0 1 1.08 1.08" color="#000" fill="#fff"/>
//!   <path d="M84.08 9.55a1.1 1.1 0 0 1-1.08 1.1 1.1 1.1 0 0 1-1.1-1.1 1.1 1.1 0 0 1 1.1-1.1 1.1 1.1 0 0 1 1.08 1.1" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M84.08 9.55a1.1 1.1 0 0 1-1.08 1.1 1.1 1.1 0 0 1-1.1-1.1 1.1 1.1 0 0 1 1.1-1.1 1.1 1.1 0 0 1 1.08 1.1" color="#000" fill="#fff"/>
//!   <path d="M89.1 6.66a1.1 1.1 0 0 1-1.08 1.1 1.1 1.1 0 0 1-1.08-1.1 1.1 1.1 0 0 1 1.08-1.08 1.1 1.1 0 0 1 1.1 1.08" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M89.1 6.66a1.1 1.1 0 0 1-1.08 1.1 1.1 1.1 0 0 1-1.08-1.1 1.1 1.1 0 0 1 1.08-1.08 1.1 1.1 0 0 1 1.1 1.08" color="#000" fill="#fff"/>
//!   <path d="M94.4 5a1.1 1.1 0 0 1-1.1 1.1A1.1 1.1 0 0 1 92.23 5a1.1 1.1 0 0 1 1.08-1.08A1.1 1.1 0 0 1 94.4 5" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M94.4 5a1.1 1.1 0 0 1-1.1 1.1A1.1 1.1 0 0 1 92.23 5a1.1 1.1 0 0 1 1.08-1.08A1.1 1.1 0 0 1 94.4 5" color="#000" fill="#fff"/>
//!   <path d="M76.44 25.13a1.1 1.1 0 0 1-1.1 1.1 1.1 1.1 0 0 1-1.08-1.1 1.1 1.1 0 0 1 1.1-1.1 1.1 1.1 0 0 1 1.08 1.1" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M76.44 25.13a1.1 1.1 0 0 1-1.1 1.1 1.1 1.1 0 0 1-1.08-1.1 1.1 1.1 0 0 1 1.1-1.1 1.1 1.1 0 0 1 1.08 1.1" color="#000" fill="#fff"/>
//!   <path d="M27.78 24.9a1.1 1.1 0 0 1-1.08 1.08 1.1 1.1 0 0 1-1.1-1.08 1.1 1.1 0 0 1 1.1-1.1 1.1 1.1 0 0 1 1.08 1.1" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M27.78 24.9a1.1 1.1 0 0 1-1.08 1.08 1.1 1.1 0 0 1-1.1-1.08 1.1 1.1 0 0 1 1.1-1.1 1.1 1.1 0 0 1 1.08 1.1" color="#000" fill="#fff"/>
//!   <path d="M45.4 5.14a1.1 1.1 0 0 1-1.08 1.1 1.1 1.1 0 0 1-1.1-1.1 1.1 1.1 0 0 1 1.1-1.08 1.1 1.1 0 0 1 1.1 1.08" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M45.4 5.14a1.1 1.1 0 0 1-1.08 1.1 1.1 1.1 0 0 1-1.1-1.1 1.1 1.1 0 0 1 1.1-1.08 1.1 1.1 0 0 1 1.1 1.08" color="#000" fill="#fff"/>
//!   <path d="M28.67 13.8a1.1 1.1 0 0 1-1.1 1.08 1.1 1.1 0 0 1-1.08-1.08 1.1 1.1 0 0 1 1.08-1.1 1.1 1.1 0 0 1 1.1 1.1" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M28.67 13.8a1.1 1.1 0 0 1-1.1 1.08 1.1 1.1 0 0 1-1.08-1.08 1.1 1.1 0 0 1 1.08-1.1 1.1 1.1 0 0 1 1.1 1.1" color="#000" fill="#fff"/>
//!   <path d="M35 7.32a1.1 1.1 0 0 1-1.1 1.1 1.1 1.1 0 0 1-1.08-1.1 1.1 1.1 0 0 1 1.1-1.1A1.1 1.1 0 0 1 35 7.33" color="#000" fill="none" stroke="#030303" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M35 7.32a1.1 1.1 0 0 1-1.1 1.1 1.1 1.1 0 0 1-1.08-1.1 1.1 1.1 0 0 1 1.1-1.1A1.1 1.1 0 0 1 35 7.33" color="#000" fill="#fff"/>
//!   <text style="line-height:6.61458302px" x="35.74" y="284.49" font-size="5.29" font-family="Sans" letter-spacing="0" word-spacing="0" fill="#b3b3b3" stroke-width=".26" transform="translate(19.595 -267)">
//!     <tspan x="35.74" y="284.49" font-size="10.58">→</tspan>
//!   </text>
//! </svg>
//!
//! The flattening algorithm implemented in this crate is based on the paper
//! [Fast, Precise Flattening of Cubic Bézier Segment Offset Curves](http://cis.usouthal.edu/~hain/general/Publications/Bezier/Bezier%20Offset%20Curves.pdf).
//! It tends to produce a better approximations than the usual recursive subdivision approach (or
//! in other words, it generates less segments for a given tolerance threshold).
//!
//! The tolerance threshold taken as input by the flattening algorithms corresponds
//! to the maximum distance between the curve and its linear approximation.
//! The smaller the tolerance is, the more precise the approximation and the more segments
//! are generated. This value is typically chosen in function of the zoom level.
//!
//! <svg viewBox="0 0 47.5 13.2" height="100" width="350" xmlns="http://www.w3.org/2000/svg">
//!   <path d="M-2.44 9.53c16.27-8.5 39.68-7.93 52.13 1.9" fill="none" stroke="#dde9af" stroke-width="4.6"/>
//!   <path d="M-1.97 9.3C14.28 1.03 37.36 1.7 49.7 11.4" fill="none" stroke="#00d400" stroke-width=".57" stroke-linecap="round" stroke-dasharray="4.6, 2.291434"/>
//!   <path d="M-1.94 10.46L6.2 6.08l28.32-1.4 15.17 6.74" fill="none" stroke="#000" stroke-width=".6"/>
//!   <path d="M6.83 6.57a.9.9 0 0 1-1.25.15.9.9 0 0 1-.15-1.25.9.9 0 0 1 1.25-.15.9.9 0 0 1 .15 1.25" color="#000" stroke="#000" stroke-width=".57" stroke-linecap="round" stroke-opacity=".5"/>
//!   <path d="M35.35 5.3a.9.9 0 0 1-1.25.15.9.9 0 0 1-.15-1.25.9.9 0 0 1 1.25-.15.9.9 0 0 1 .15 1.24" color="#000" stroke="#000" stroke-width=".6" stroke-opacity=".5"/>
//!   <g fill="none" stroke="#ff7f2a" stroke-width=".26">
//!     <path d="M20.4 3.8l.1 1.83M19.9 4.28l.48-.56.57.52M21.02 5.18l-.5.56-.6-.53" stroke-width=".2978872"/>
//!   </g>
//! </svg>
//!
//! The figure above shows a close up on a curve (the dotted line) and its linear
//! approximation (the black segments). The tolerance threshold is represented by
//! the light green area and the orange arrow.
//!

//#![allow(needless_return)] // clippy

// Reexport dependencies.
pub use arrayvec;
pub use euclid;

#[cfg(feature = "serialization")]
#[macro_use]
pub extern crate serde;

#[macro_use] mod segment;
pub mod quadratic_bezier;
pub mod cubic_bezier;
pub mod arc;
pub mod utils;
pub mod cubic_to_quadratic;
mod cubic_bezier_intersections;
mod flatten_cubic;
mod triangle;
mod line;
mod monotonic;

#[doc(inline)]
pub use crate::quadratic_bezier::QuadraticBezierSegment;
#[doc(inline)]
pub use crate::cubic_bezier::CubicBezierSegment;
#[doc(inline)]
pub use crate::triangle::{Triangle};
#[doc(inline)]
pub use crate::line::{LineSegment, Line, LineEquation};
#[doc(inline)]
pub use crate::arc::{Arc, SvgArc, ArcFlags};
#[doc(inline)]
pub use crate::segment::{Segment, BezierSegment};
#[doc(inline)]
pub use crate::monotonic::Monotonic;

mod scalar {
    pub(crate) use num_traits::{Float, FloatConst, NumCast};
    pub(crate) use num_traits::One;
    pub(crate) use num_traits::cast::cast;
    pub(crate) use euclid::Trig;

    use std::fmt::{Display, Debug};
    use std::ops::{AddAssign, SubAssign, MulAssign, DivAssign};

    pub trait Scalar
        : Float
        + NumCast
        + FloatConst
        + Sized
        + Display
        + Debug
        + Trig
        + AddAssign
        + SubAssign
        + MulAssign
        + DivAssign
    {
        const HALF: Self;
        const ZERO: Self;
        const ONE: Self;
        const TWO: Self;
        const THREE: Self;
        const FOUR: Self;
        const FIVE: Self;
        const SIX: Self;
        const SEVEN: Self;
        const EIGHT: Self;
        const NINE: Self;
        const TEN: Self;

        const EPSILON: Self;

        fn value(v: f32) -> Self;
    }

    impl Scalar for f32 {
        const HALF: Self = 0.5;
        const ZERO: Self = 0.0;
        const ONE: Self = 1.0;
        const TWO: Self = 2.0;
        const THREE: Self = 3.0;
        const FOUR: Self = 4.0;
        const FIVE: Self = 5.0;
        const SIX: Self = 6.0;
        const SEVEN: Self = 7.0;
        const EIGHT: Self = 8.0;
        const NINE: Self = 9.0;
        const TEN: Self = 10.0;

        const EPSILON: Self = 1e-5;

        #[inline]
        fn value(v: f32) -> Self { v }
    }

    impl Scalar for f64 {
        const HALF: Self = 0.5;
        const ZERO: Self = 0.0;
        const ONE: Self = 1.0;
        const TWO: Self = 2.0;
        const THREE: Self = 3.0;
        const FOUR: Self = 4.0;
        const FIVE: Self = 5.0;
        const SIX: Self = 6.0;
        const SEVEN: Self = 7.0;
        const EIGHT: Self = 8.0;
        const NINE: Self = 9.0;
        const TEN: Self = 10.0;

        const EPSILON: Self = 1e-8;

        #[inline]
        fn value(v: f32) -> Self { v as f64 }
    }
}

mod generic_math {
    /// Alias for `euclid::Point2D`.
    pub use euclid::Point2D as Point;

    /// Alias for `euclid::Vector2D`.
    pub use euclid::Vector2D as Vector;

    /// Alias for `euclid::Size2D`.
    pub use euclid::Size2D as Size;

    /// Alias for `euclid::Rect`
    pub use euclid::Rect;

    /// Alias for `euclid::Transform2D`
    pub use euclid::Transform2D;

    /// Alias for `euclid::Rotation2D`
    pub use euclid::Rotation2D;

    /// An angle in radians.
    pub use euclid::Angle;

    /// Shorthand for `Rect::new(Point::new(x, y), Size::new(w, h))`.
    pub use euclid::rect;

    /// Shorthand for `Vector::new(x, y)`.
    pub use euclid::vec2 as vector;

    /// Shorthand for `Point::new(x, y)`.
    pub use euclid::point2 as point;

    /// Shorthand for `Size::new(x, y)`.
    pub use euclid::size2 as size;
}

pub mod math {
    //! Basic types that are used everywhere. Most other lyon crates
    //! reexport them.

    use euclid;

    /// Alias for ```euclid::Point2D<f32>```.
    pub type Point = euclid::Point2D<f32>;

    /// Alias for ```euclid::Point2D<f64>```.
    pub type F64Point = euclid::Point2D<f64>;

    /// Alias for ```euclid::Point2D<f32>```.
    pub type Vector = euclid::Vector2D<f32>;

    /// Alias for ```euclid::Size2D<f32>```.
    pub type Size = euclid::Size2D<f32>;

    /// Alias for ```euclid::Rect<f32>```
    pub type Rect = euclid::Rect<f32>;

    /// Alias for ```euclid::Transform2D<f32>```
    pub type Transform2D = euclid::Transform2D<f32>;

    /// Alias for ```euclid::Rotation2D<f32>```
    pub type Rotation2D = euclid::Rotation2D<f32>;

    /// An angle in radians (f32).
    pub type Angle = euclid::Angle<f32>;

    /// Shorthand for `Rect::new(Point::new(x, y), Size::new(w, h))`.
    pub use euclid::rect;

    /// Shorthand for `Vector::new(x, y)`.
    pub use euclid::vec2 as vector;

    /// Shorthand for `Point::new(x, y)`.
    pub use euclid::point2 as point;

    /// Shorthand for `Size::new(x, y)`.
    pub use euclid::size2 as size;

    /// Anything that can be transformed in 2D.
    pub trait Transform {
        fn transform(&self, mat: &Transform2D) -> Self;
    }
}


pub mod traits {
    pub use crate::segment::{Segment, FlattenedForEach, FlatteningStep};
    //pub use monotonic::MonotonicSegment;
}
