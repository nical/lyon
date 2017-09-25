#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]

//! # Lyon path builder
//!
//! Tools to iterate over path objects.
//!
//! ## Overview
//!
//! This crate provides a collection of traits to extend the Iterator trait with
//! information about the state of the cursor moving along the path. This is useful
//! because the way some events are described require to have information about the
//! previous events. For example the event `LinTo` gives the next position and it is
//! generally useful to have access to the current position in order to make something
//! out of it. Likewise, Some Svg events are given in relative coordinates and/or
//! are expressed in a way that the first control point is deduced from the position
//! of the previous control point.
//!
//! All of this extra information is conveniently exposed in the `PathState` struct
//! that can be accessed by `PathIterator`, `SvgIterator` and `FlattenedIterator`.
//!
//! The `PathIter<Iter>` adapter automatically implements `PathIterator` for
//! any `Iter` that implements `Iterator<PathEvent>`
//!
//! This crate provides adapters between these iterator types. For example iterating
//! over a sequence of SVG events can be automatically translated into iterating over
//! simpler path events which express all positions with absolute coordinates, among
//! other things.
//!
//! The trait `FlattenedIterator` is what some of the tessellation algorithms
//! of the `lyon_tessellation` crate take as input.
//!
//! ## Examples
//!
//! ```
//! extern crate lyon_path_iterator;
//! use lyon_path_iterator::*;
//! use math::{point, vec2};
//!
//! fn main() {
//!     let events = vec![
//!         SvgEvent::MoveTo(point(1.0, 1.0)),
//!         SvgEvent::RelativeQuadraticTo(vec2(4.0, 5.0), vec2(-1.0, 4.0)),
//!         SvgEvent::SmoothCubicTo(point(3.0, 1.0), point(10.0, -3.0)),
//!         SvgEvent::Close,
//!     ];
//!
//!     // A simple std::iter::Iterator<SvgEvent>,
//!     let simple_iter = events.iter().cloned();
//!
//!     // Make it a SvgIterator (keeps tracks of the path state).
//!     let svg_path_iter = SvgPathIter::new(simple_iter);
//!
//!     // Make it a PathIterator (iterates on simpler PathEvents).
//!     let path_iter = svg_path_iter.path_events();
//!     // Equivalent to:
//!     // let path_iter = PathEvents::new(svg_path_iter);
//!
//!     // Make it an iterator over even simpler primitives: FlattenedEvent,
//!     // which do not contain any curve. To do so we approximate each curve
//!     // linear segments according to a tolerance threashold which controls
//!     // the tradeoff between fidelity of the approximation and amount of
//!     // generated events. Let's use a tolerance threshold of 0.01.
//!     // The beauty of this approach is that the flattening happens lazily
//!     // while iterating with no memory allocation.
//!     let flattened_iter = path_iter.flattened(0.01);
//!     // equivalent to:
//!     // let flattened = Flattened::new(0.01, path_iter);
//!
//!     for evt in flattened_iter {
//!         match evt {
//!             FlattenedEvent::MoveTo(p) => { println!(" - move to {:?}", p); }
//!             FlattenedEvent::LineTo(p) => { println!(" - line to {:?}", p); }
//!             FlattenedEvent::Close => { println!(" - close"); }
//!         }
//!     }
//! }
//! ```
//!
//! An equivalent (but shorter) version of the above code takes advantage of the
//! fact you can get a flattening iterator directly from an `SvgIterator`:
//!
//! ```
//! extern crate lyon_path_iterator;
//! use lyon_path_iterator::*;
//! use math::{point, vec2};
//!
//! fn main() {
//!     let events = vec![
//!         SvgEvent::MoveTo(point(1.0, 1.0)),
//!         SvgEvent::RelativeQuadraticTo(vec2(4.0, 5.0), vec2(-1.0, 4.0)),
//!         SvgEvent::SmoothCubicTo(point(3.0, 1.0), point(10.0, -3.0)),
//!         SvgEvent::Close,
//!     ];
//!
//!     for evt in SvgPathIter::new(events.iter().cloned()).flattened(0.01) {
//!         // ...
//!     }
//! }
//! ```

extern crate lyon_core as core;
extern crate lyon_bezier as bezier;

mod path_iterator;

#[doc(inline)]
pub use path_iterator::*;

pub use core::*;
