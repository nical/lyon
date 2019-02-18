//! Tools to iterate over paths.
//!
//! # Lyon path iterators
//!
//! ## Overview
//!
//! This module provides a collection of traits to extend the `Iterator` trait when
//! iterating over paths.
//!
//! ## Examples
//!
//! ```
//! use lyon_path::iterator::*;
//! use lyon_path::math::{point, vector};
//! use lyon_path::geom::BezierSegment;
//! use lyon_path::{Path, PathEvent, FlattenedEvent};
//!
//! fn main() {
//!     // Start with a path.
//!     let mut builder = Path::builder();
//!     builder.move_to(point(0.0, 0.0));
//!     builder.line_to(point(10.0, 0.0));
//!     builder.cubic_bezier_to(point(10.0, 10.0), point(0.0, 10.0), point(0.0, 5.0));
//!     builder.close();
//!     let path = builder.build();
//!
//!     // A simple std::iter::Iterator<PathEvent<Point, Point>>,
//!     let simple_iter = path.iter();
//!
//!     // Make it an iterator over simpler primitives: FlattenedEvent,
//!     // which do not contain any curve. To do so we approximate each curve
//!     // linear segments according to a tolerance threshold which controls
//!     // the tradeoff between fidelity of the approximation and amount of
//!     // generated events. Let's use a tolerance threshold of 0.01.
//!     // The beauty of this approach is that the flattening happens lazily
//!     // while iterating without allocating memory for the path.
//!     let flattened_iter = path.iter().flattened(0.01);
//!
//!     for evt in flattened_iter {
//!         match evt {
//!             FlattenedEvent::Begin { at } => { println!(" - move to {:?}", at); }
//!             FlattenedEvent::Line { from, to } => { println!(" - line {:?} -> {:?}", from, to); }
//!             FlattenedEvent::End { last, first, close } => {
//!                 if close {
//!                     println!(" - close {:?} -> {:?}", last, first);
//!                 } else {
//!                     println!(" - end");
//!                 }
//!             }
//!         }
//!     }
//!
//!     // Sometimes, working with segments directly without dealing with Begin/End events
//!     // can be more convenient:
//!     for segment in path.iter().bezier_segments() {
//!         match segment {
//!             BezierSegment::Linear(segment) => { println!("{:?}", segment); }
//!             BezierSegment::Quadratic(segment) => { println!("{:?}", segment); }
//!             BezierSegment::Cubic(segment) => { println!("{:?}", segment); }
//!         }
//!     }
//!
//!     // It is also possible to iterate over line segments directly with flattened paths.
//!     for segment in path.iter().flattened(0.1).line_segments() {
//!         println!("line segment {:?} -> {:?}", segment.from, segment.to);
//!     }
//! }
//! ```
//!
//! Chaining the provided iterators allow performing some path manipulations lazily
//! without allocating actual path objects to hold the result of the transformations.
//!
//! ```
//! extern crate lyon_path;
//! use lyon_path::iterator::*;
//! use lyon_path::geom::euclid::{Angle, Transform2D};
//! use lyon_path::math::point;
//! use lyon_path::Path;
//!
//! fn main() {
//!     // In practice it is more common to iterate over Path objects than vectors
//!     // of SVG commands (the former can be constructed from the latter).
//!     let mut builder = Path::builder();
//!     builder.move_to(point(1.0, 1.0));
//!     builder.line_to(point(2.0, 1.0));
//!     builder.quadratic_bezier_to(point(2.0, 2.0), point(1.0, 2.0));
//!     builder.cubic_bezier_to(point(0.0, 2.0), point(0.0, 0.0), point(1.0, 0.0));
//!     builder.close();
//!     let path = builder.build();
//!
//!     let mut transform = Transform2D::create_rotation(Angle::radians(1.0));
//!
//!     for evt in path.iter().transformed(&transform).bezier_segments() {
//!         // ...
//!     }
//! }
//! ```

use std::iter;

use crate::math::*;
use crate::{PathEvent, FlattenedEvent};
use crate::geom::{BezierSegment, QuadraticBezierSegment, CubicBezierSegment, LineSegment, quadratic_bezier, cubic_bezier};

/// An extension trait for `PathEvent` iterators.
pub trait PathIterator: Iterator<Item = PathEvent<Point, Point>> + Sized {

    /// Returns an iterator that turns curves into line segments.
    fn flattened(self, tolerance: f32) -> Flattened<Self> {
        Flattened::new(tolerance, self)
    }

    /// Returns an iterator applying a 2D transform to all of its events.
    fn transformed(self, mat: &Transform2D) -> Transformed<Self> {
        Transformed::new(mat, self)
    }

    /// Returns an iterator of segments.
    fn bezier_segments(self) -> BezierSegments<Self> {
        BezierSegments { iter: self }
    }
}

impl<Iter> PathIterator for Iter
where
    Iter: Iterator<Item = PathEvent<Point, Point>>,
{}

/// An extension to the common Iterator interface, that adds information which is useful when
/// chaining path-specific iterators.
pub trait FlattenedIterator: Iterator<Item = FlattenedEvent<Point>> + Sized {

    /// Returns an iterator of path events.
    fn path_events(self) -> iter::Map<Self, fn(FlattenedEvent<Point>) -> PathEvent<Point, Point>> {
        self.map(flattened_to_path_event)
    }

    /// Returns an iterator applying a 2D transform to all of its events.
    fn transformed(self, mat: &Transform2D) -> Transformed<Self> {
        Transformed::new(mat, self)
    }

    /// Consumes the iterator and returns the length of the path.
    fn length(self) -> f32 {
        flattened_path_length(self)
    }

    /// Returns an iterator of line segments.
    fn line_segments(self) -> LineSegments<Self> {
        LineSegments { iter: self }
    }
}

impl<Iter> FlattenedIterator for Iter
where
    Iter: Iterator<Item = FlattenedEvent<Point>>,
{}

/// An iterator that consumes `PathEvent` iterator and yields FlattenedEvents.
pub struct Flattened<Iter> {
    it: Iter,
    current_position: Point,
    current_curve: TmpFlatteningIter,
    tolerance: f32,
}

enum TmpFlatteningIter {
    Quadratic(quadratic_bezier::Flattened<f32>),
    Cubic(cubic_bezier::Flattened<f32>),
    None,
}

impl<Iter: Iterator<Item = PathEvent<Point, Point>>> Flattened<Iter> {
    /// Create the iterator.
    pub fn new(tolerance: f32, it: Iter) -> Self {
        Flattened {
            it,
            current_position: point(0.0, 0.0),
            current_curve: TmpFlatteningIter::None,
            tolerance,
        }
    }
}

impl<Iter> Iterator for Flattened<Iter>
where
    Iter: Iterator<Item = PathEvent<Point, Point>>,
{
    type Item = FlattenedEvent<Point>;
    fn next(&mut self) -> Option<FlattenedEvent<Point>> {
        match self.current_curve {
            TmpFlatteningIter::Quadratic(ref mut it) => {
                if let Some(to) = it.next() {
                    let from = self.current_position;
                    self.current_position = to;
                    return Some(FlattenedEvent::Line { from, to });
                }
            }
            TmpFlatteningIter::Cubic(ref mut it) => {
                if let Some(to) = it.next() {
                    let from = self.current_position;
                    self.current_position = to;
                    return Some(FlattenedEvent::Line { from, to });
                }
            }
            _ => {}
        }
        self.current_curve = TmpFlatteningIter::None;
        match self.it.next() {
            Some(PathEvent::Begin { at }) => Some(FlattenedEvent::Begin { at }),
            Some(PathEvent::Line { from, to }) => Some(FlattenedEvent::Line { from, to }),
            Some(PathEvent::End { last, first, close }) => Some(FlattenedEvent::End { last, first, close }),
            Some(PathEvent::Quadratic { from, ctrl, to }) => {
                self.current_position = from;
                self.current_curve = TmpFlatteningIter::Quadratic(
                    QuadraticBezierSegment { from, ctrl, to }.flattened(self.tolerance)
                );
                self.next()
            }
            Some(PathEvent::Cubic { from, ctrl1, ctrl2, to }) => {
                self.current_position = from;
                self.current_curve = TmpFlatteningIter::Cubic(
                    CubicBezierSegment { from, ctrl1, ctrl2, to }.flattened(self.tolerance)
                );
                self.next()
            }
            None => None,
        }
    }
}

#[inline]
fn flattened_to_path_event(evt: FlattenedEvent<Point>) -> PathEvent<Point, Point> { evt.to_path_event() }

/// Applies a 2D transform to a path iterator and yields the resulting path iterator.
pub struct Transformed<I> {
    it: I,
    transform: Transform2D,
}

impl<I, Event> Transformed<I>
where
    I: Iterator<Item = Event>,
    Event: Transform
{
    /// Creates a new transformed path iterator from a path iterator.
    #[inline]
    pub fn new(transform: &Transform2D, it: I) -> Transformed<I> {
        Transformed {
            it,
            transform: *transform,
        }
    }
}

impl<I, Event> Iterator for Transformed<I>
where
    I: Iterator<Item = Event>,
    Event: Transform
{
    type Item = Event;
    fn next(&mut self) -> Option<Event> {
        match self.it.next() {
            None => None,
            Some(ref evt) => Some(evt.transform(&self.transform)),
        }
    }
}


/// An iterator that consumes an iterator of `Point`s and produces `FlattenedEvent`s.
///
/// # Example
///
/// ```
/// # extern crate lyon_path;
/// # use lyon_path::iterator::FromPolyline;
/// # use lyon_path::math::point;
/// # fn main() {
/// let points = [
///     point(1.0, 1.0),
///     point(2.0, 1.0),
///     point(1.0, 2.0)
/// ];
/// let iter = FromPolyline::closed(points.iter().cloned());
/// # }
/// ```
pub struct FromPolyline<Iter> {
    iter: Iter,
    current: Point,
    first: Point,
    is_first: bool,
    done: bool,
    close: bool,
}

impl<Iter: Iterator<Item = Point>> FromPolyline<Iter> {
    pub fn new(close: bool, iter: Iter) -> Self {
        FromPolyline {
            iter,
            current: point(0.0, 0.0),
            first: point(0.0, 0.0),
            is_first: true,
            done: false,
            close,
        }
    }

    pub fn closed(iter: Iter) -> Self { FromPolyline::new(true, iter) }

    pub fn open(iter: Iter) -> Self { FromPolyline::new(false, iter) }
}

impl<Iter> Iterator for FromPolyline<Iter>
where
    Iter: Iterator<Item = Point>,
{
    type Item = FlattenedEvent<Point>;

    fn next(&mut self) -> Option<FlattenedEvent<Point>> {
        if self.done {
            return None;
        }

        if let Some(next) = self.iter.next() {
            debug_assert!(next.x.is_finite());
            debug_assert!(next.y.is_finite());
            let from = self.current;
            self.current = next;
            return if self.is_first {
                self.is_first = false;
                self.first = next;
                Some(FlattenedEvent::Begin { at: next })
            } else {
                Some(FlattenedEvent::Line { from, to: next })
            }
        }

        self.done = true;

        Some(FlattenedEvent::End {
            last: self.current,
            first: self.first,
            close: self.close,
        })
    }
}

/// Turns an iterator of `PathEvent` into an iterator of `BezierSegment<f32>`.
pub struct BezierSegments<Iter> {
    iter: Iter
}

impl<Iter> Iterator for BezierSegments<Iter>
where Iter: Iterator<Item = PathEvent<Point, Point>> {
    type Item = BezierSegment<f32>;
    fn next(&mut self) -> Option<BezierSegment<f32>> {
        match self.iter.next() {
            Some(PathEvent::Line { from, to }) => Some(BezierSegment::Linear(LineSegment { from, to })),
            Some(PathEvent::End { last, first, close: true })=> Some(BezierSegment::Linear(LineSegment { from: last, to: first })),
            Some(PathEvent::End { close: false, .. }) => self.next(),
            Some(PathEvent::Quadratic { from, ctrl, to }) => Some(BezierSegment::Quadratic(QuadraticBezierSegment { from, ctrl, to })),
            Some(PathEvent::Cubic { from, ctrl1, ctrl2, to }) => Some(BezierSegment::Cubic(CubicBezierSegment { from, ctrl1, ctrl2, to })),
            Some(PathEvent::Begin { .. }) => self.next(),
            None => None,
        }
    }
}

/// Turns an iterator of `FlattenedEvent` into an iterator of `LineSegment<f32>`.
pub struct LineSegments<Iter> {
    iter: Iter
}

impl<Iter> Iterator for LineSegments<Iter>
where Iter: Iterator<Item = FlattenedEvent<Point>> {
    type Item = LineSegment<f32>;
    fn next(&mut self) -> Option<LineSegment<f32>> {
        match self.iter.next() {
            Some(FlattenedEvent::Line { from, to }) => Some(LineSegment { from, to }),
            Some(FlattenedEvent::End { last, first, close: true }) => Some(LineSegment { from: last, to: first }),
            Some(FlattenedEvent::End { close: false, .. }) => self.next(),
            Some(FlattenedEvent::Begin { .. }) => self.next(),
            None => None,
        }
    }
}

/// Computes the length of a flattened path.
fn flattened_path_length<T>(iter: T) -> f32
where T: Iterator<Item = FlattenedEvent<Point>> {
    let mut length = 0.0;
    for evt in iter {
        match evt {
            FlattenedEvent::Begin { .. } => {}
            FlattenedEvent::Line { from, to } => { length += (to - from).length(); }
            FlattenedEvent::End { last, first, .. } => { length += (first - last).length(); }
        }
    }

    length
}

#[test]
fn test_from_polyline_open() {
    let points = &[
        point(1.0, 1.0),
        point(3.0, 1.0),
        point(4.0, 5.0),
        point(5.0, 2.0),
    ];

    let mut evts = FromPolyline::open(points.iter().cloned());

    assert_eq!(evts.next(), Some(FlattenedEvent::Begin { at: point(1.0, 1.0) }));
    assert_eq!(evts.next(), Some(FlattenedEvent::Line { from: point(1.0, 1.0), to: point(3.0, 1.0) }));
    assert_eq!(evts.next(), Some(FlattenedEvent::Line { from: point(3.0, 1.0), to: point(4.0, 5.0) }));
    assert_eq!(evts.next(), Some(FlattenedEvent::Line { from: point(4.0, 5.0), to: point(5.0, 2.0) }));
    assert_eq!(evts.next(), Some(FlattenedEvent::End { last: point(5.0, 2.0), first: point(1.0, 1.0), close: false }));
    assert_eq!(evts.next(), None);
}

#[test]
fn test_from_polyline_closed() {
    let points = &[
        point(1.0, 1.0),
        point(3.0, 1.0),
        point(4.0, 5.0),
        point(5.0, 2.0),
    ];

    let mut evts = FromPolyline::closed(points.iter().cloned());

    assert_eq!(evts.next(), Some(FlattenedEvent::Begin { at: point(1.0, 1.0) }));
    assert_eq!(evts.next(), Some(FlattenedEvent::Line { from: point(1.0, 1.0), to: point(3.0, 1.0) }));
    assert_eq!(evts.next(), Some(FlattenedEvent::Line { from: point(3.0, 1.0), to: point(4.0, 5.0) }));
    assert_eq!(evts.next(), Some(FlattenedEvent::Line { from: point(4.0, 5.0), to: point(5.0, 2.0) }));
    assert_eq!(evts.next(), Some(FlattenedEvent::End { last: point(5.0, 2.0), first: point(1.0, 1.0), close: true }));
    assert_eq!(evts.next(), None);
}
