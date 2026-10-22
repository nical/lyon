#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]
#![deny(bare_trait_objects)]
#![deny(unconditional_recursion)]
#![allow(clippy::float_cmp)]
#![allow(mismatched_lifetime_syntaxes)]
#![no_std]

//! Path offsetting and stroke expansion.
//!
//! This crate provides two related algorithms:
//!
//! - [`OffsetBuilder`] builds a path offset by a given distance, with
//!   configurable joins. See [`OffsetOptions`].
//! - [`Stroker`] converts a stroked path into a fillable outline. See
//!   [`StrokeToFillOptions`].
//!
//! The stroke-to-fill output has self-overlapping contours and must be filled
//! with the non-zero fill rule.
//!
//! This crate is reexported in [lyon](https://docs.rs/lyon/).
//!
//! # Examples
//!
//! ```
//! use lyon_offset::path::Path;
//! use lyon_offset::path::math::point;
//! use lyon_offset::{LineCap, LineJoin, StrokeToFillOptions, Stroker};
//!
//! let mut builder = Path::builder();
//! builder.begin(point(0.0, 0.0));
//! builder.line_to(point(100.0, 0.0));
//! builder.line_to(point(100.0, 100.0));
//! builder.end(false);
//! let source = builder.build();
//!
//! let options = StrokeToFillOptions::new(10.0)
//!     .with_line_join(LineJoin::Round)
//!     .with_line_cap(LineCap::Butt);
//!
//! // The output is a single contour going along one side of the path, around
//! // the end cap, back along the other side and around the start cap.
//! let mut builder = Path::builder();
//! Stroker::new().stroke_to_fill(source.iter(), &options, &mut builder);
//! let outline = builder.build();
//! ```

extern crate alloc;

#[cfg(any(test, feature = "std"))]
extern crate std;

#[cfg(feature = "serialization")]
#[macro_use]
pub extern crate serde;

pub use lyon_geom as geom;
pub use lyon_path as path;

mod offset;
mod stroke;
#[cfg(test)]
mod tests;

pub use offset::*;
pub use stroke::*;

pub(crate) type Point = geom::Point<f32>;
pub(crate) type Vector = geom::Vector<f32>;

/// Line join as defined by the SVG specification.
///
/// See: <https://svgwg.org/specs/strokes/#StrokeLinejoinProperty>
#[non_exhaustive]
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
    /// A beveled corner is to be used to join path segments.
    /// The bevel shape is a triangle that fills the area between the two stroked
    /// segments.
    Bevel,
}

impl From<path::LineJoin> for LineJoin {
    fn from(join: path::LineJoin) -> Self {
        match join {
            path::LineJoin::Miter => LineJoin::Miter,
            path::LineJoin::MiterClip => LineJoin::MiterClip,
            path::LineJoin::Round => LineJoin::Round,
            path::LineJoin::Bevel => LineJoin::Bevel,
        }
    }
}

/// End cap as defined by the SVG specification, plus a few extensions.
///
/// See: <https://svgwg.org/specs/strokes/#StrokeLinecapProperty>
///
/// This is a superset of [`lyon_path::LineCap`], which only contains the three
/// caps defined by the SVG specification. Use [`From`] to convert a
/// [`lyon_path::LineCap`] into a `LineCap`.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
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
    /// At each end of each sub-path, the shape representing the stroke will be
    /// extended by a half circle with a radius equal to the stroke width.
    /// If a sub-path has zero length, then the resulting effect is that the
    /// stroke for that sub-path consists solely of a full circle centered at
    /// the sub-path's point.
    Round,
    /// A half circle like [`LineCap::Round`], but carved into the stroke
    /// instead of extending past the end of the sub-path.
    ///
    /// The outline keeps the same endpoints as a [`LineCap::Butt`] cap, so the
    /// stroke never extends beyond the path. A zero length sub-path has no
    /// stroke to carve into, so this behaves like [`LineCap::Butt`] there.
    RoundInverted,
    /// A triangle pointing past the end of the sub-path, with its base at the
    /// end of the sub-path and its tip at half the stroke width past it.
    ///
    /// A zero length sub-path draws a triangle centered on its point.
    Triangle,
    /// A triangle pointing back into the stroke, carved out of it instead of
    /// extending past the end of the sub-path.
    ///
    /// The outline keeps the same endpoints as a [`LineCap::Butt`] cap. A zero
    /// length sub-path has no stroke to carve into, so this behaves like
    /// [`LineCap::Butt`] there.
    TriangleInverted,
}

impl From<path::LineCap> for LineCap {
    fn from(cap: path::LineCap) -> Self {
        match cap {
            path::LineCap::Butt => LineCap::Butt,
            path::LineCap::Square => LineCap::Square,
            path::LineCap::Round => LineCap::Round,
        }
    }
}
