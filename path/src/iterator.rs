//! # Lyon path iterators
//!
//! Tools to iterate over path objects.
//!
//! ## Overview
//!
//! This module provides a collection of traits to extend the Iterator trait with
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
//! This module provides adapters between these iterator types. For example iterating
//! over a sequence of SVG events can be automatically translated into iterating over
//! simpler path events which express all positions with absolute coordinates, among
//! other things.
//!
//! The trait `PathIterator` is what some of the tessellation algorithms
//! of the `lyon_tessellation` crate take as input.
//!
//! ## Examples
//!
//! ```
//! extern crate lyon_path;
//! use lyon_path::iterator::*;
//! use lyon_path::math::{point, vector};
//! use lyon_path::{PathEvent, SvgEvent, FlattenedEvent};
//!
//! fn main() {
//!     let events = vec![
//!         SvgEvent::MoveTo(point(1.0, 1.0)),
//!         SvgEvent::RelativeQuadraticTo(vector(4.0, 5.0), vector(-1.0, 4.0)),
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
//! extern crate lyon_path;
//! use lyon_path::iterator::*;
//! use lyon_path::math::{point, vector};
//! use lyon_path::SvgEvent;
//!
//! fn main() {
//!     let events = vec![
//!         SvgEvent::MoveTo(point(1.0, 1.0)),
//!         SvgEvent::RelativeQuadraticTo(vector(4.0, 5.0), vector(-1.0, 4.0)),
//!         SvgEvent::SmoothCubicTo(point(3.0, 1.0), point(10.0, -3.0)),
//!         SvgEvent::Close,
//!     ];
//!
//!     for evt in SvgPathIter::new(events.iter().cloned()).flattened(0.01) {
//!         // ...
//!     }
//! }
//! ```

use std::iter;

use core::math::*;
use {PathEvent, SvgEvent, FlattenedEvent, PathState};
use bezier::QuadraticBezierSegment;
use bezier::quadratic_bezier;
use bezier::CubicBezierSegment;
use bezier::cubic_bezier;
use bezier::utils::vector_angle;
use bezier::arc;

/// An extension to the common Iterator interface, that adds information which is useful when
/// chaining path-specific iterators.
pub trait PathIterator: Iterator<Item = PathEvent> + Sized {
    /// The returned structure exposes the current position, the first position in the current
    /// sub-path, and the position of the last control point.
    fn get_state(&self) -> &PathState;

    /// Returns an iterator that turns curves into line segments.
    fn flattened(self, tolerance: f32) -> Flattened<Self> {
        Flattened::new(tolerance, self)
    }
}

/// An extension to the common Iterator interface, that adds information which is useful when
/// chaining path-specific iterators.
pub trait SvgIterator: Iterator<Item = SvgEvent> + Sized {
    /// The returned structure exposes the current position, the first position in the current
    /// sub-path, and the position of the last control point.
    fn get_state(&self) -> &PathState;

    /// Returns an iterator of FlattenedEvents, turning curves into sequences of line segments.
    fn flattened(self, tolerance: f32) -> Flattened<PathEvents<Self>> {
        self.path_events().flattened(tolerance)
    }

    /// Returns an iterator of path events.
    fn path_events(self) -> PathEvents<Self> { PathEvents::new(self) }
}

/// An extension to the common Iterator interface, that adds information which is useful when
/// chaining path-specific iterators.
pub trait FlattenedIterator: Iterator<Item = FlattenedEvent> + Sized {
    /// The returned structure exposes the current position, the first position in the current
    /// sub-path, and the position of the last control point.
    fn get_state(&self) -> &PathState;

    /// Returns an iterator of path events.
    fn path_events(self) -> iter::Map<Self, fn(FlattenedEvent) -> PathEvent> {
        self.map(flattened_to_path_event)
    }

    /// Returns an iterator of svg events.
    fn svg_events(self) -> iter::Map<Self, fn(FlattenedEvent) -> SvgEvent> {
        self.map(flattened_to_svg_event)
    }
}

pub struct PathEvents<SvgIter> {
    it: SvgIter,
}

impl<SvgIter> PathEvents<SvgIter> {
    pub fn new(it: SvgIter) -> Self { PathEvents { it: it } }
}

impl<SvgIter> PathIterator for PathEvents<SvgIter>
where
    SvgIter: SvgIterator,
{
    fn get_state(&self) -> &PathState { self.it.get_state() }
}

impl<SvgIter> Iterator for PathEvents<SvgIter>
where
    SvgIter: SvgIterator,
{
    type Item = PathEvent;
    fn next(&mut self) -> Option<PathEvent> {
        match self.it.next() {
            Some(svg_evt) => Some(self.get_state().svg_to_path_event(svg_evt)),
            None => None,
        }
    }
}

/// An iterator that consumes an PathIterator and yields FlattenedEvents.
pub struct Flattened<Iter> {
    it: Iter,
    current_curve: TmpFlatteningIter,
    tolerance: f32,
}

enum TmpFlatteningIter {
    Quadratic(quadratic_bezier::Flattened),
    Cubic(cubic_bezier::Flattened),
    Arc(arc::Flattened),
    None,
}

impl<Iter: PathIterator> Flattened<Iter> {
    /// Create the iterator.
    pub fn new(tolerance: f32, it: Iter) -> Self {
        Flattened {
            it: it,
            current_curve: TmpFlatteningIter::None,
            tolerance: tolerance,
        }
    }
}

impl<Iter> FlattenedIterator for Flattened<Iter>
where
    Iter: PathIterator,
{
    fn get_state(&self) -> &PathState { self.it.get_state() }
}

impl<Iter> Iterator for Flattened<Iter>
where
    Iter: PathIterator,
{
    type Item = FlattenedEvent;
    fn next(&mut self) -> Option<FlattenedEvent> {
        match self.current_curve {
            TmpFlatteningIter::Quadratic(ref mut it) => {
                if let Some(point) = it.next() {
                    return Some(FlattenedEvent::LineTo(point));
                }
            }
            TmpFlatteningIter::Cubic(ref mut it) => {
                if let Some(point) = it.next() {
                    return Some(FlattenedEvent::LineTo(point));
                }
            }
            TmpFlatteningIter::Arc(ref mut it) => {
                if let Some(point) = it.next() {
                    return Some(FlattenedEvent::LineTo(point));
                }
            }
            _ => {}
        }
        self.current_curve = TmpFlatteningIter::None;
        let current = self.get_state().current;
        return match self.it.next() {
            Some(PathEvent::MoveTo(to)) => Some(FlattenedEvent::MoveTo(to)),
            Some(PathEvent::LineTo(to)) => Some(FlattenedEvent::LineTo(to)),
            Some(PathEvent::Close) => Some(FlattenedEvent::Close),
            Some(PathEvent::QuadraticTo(ctrl, to)) => {
                self.current_curve = TmpFlatteningIter::Quadratic(
                    QuadraticBezierSegment {
                            from: current,
                            ctrl: ctrl,
                            to: to,
                    }.flattened(self.tolerance)
                );
                return self.next();
            }
            Some(PathEvent::CubicTo(ctrl1, ctrl2, to)) => {
                self.current_curve = TmpFlatteningIter::Cubic(
                    CubicBezierSegment {
                        from: current,
                        ctrl1: ctrl1,
                        ctrl2: ctrl2,
                        to: to,
                    }.flattened(self.tolerance)
                );
                return self.next();
            }
            Some(PathEvent::Arc(center, radii, sweep_angle, x_rotation)) => {
                let start_angle = vector_angle(current - center);
                self.current_curve = TmpFlatteningIter::Arc(
                    arc::Arc {
                        center, radii,
                        start_angle, sweep_angle,
                        x_rotation
                    }.flattened(self.tolerance)
                );
                return self.next();
            }
            None => None,
        };
    }
}

// TODO: SvgPathIter and PathIter should be merged into a single struct using
// specialization to implement the Iterator trait depending on the type of
// event but specialization isn't stable in rust yet.

/// An adapater iterator that implements SvgIterator on top of an Iterator<Item=SvgEvent>.
pub struct SvgPathIter<Iter> {
    it: Iter,
    state: PathState,
}

impl<E, Iter> SvgPathIter<Iter>
where
    E: Into<SvgEvent>,
    Iter: Iterator<Item = E>
{
    pub fn new(it: Iter) -> Self {
        SvgPathIter {
            it: it,
            state: PathState::new(),
        }
    }
}

impl<E, Iter> SvgIterator for SvgPathIter<Iter>
where
    E: Into<SvgEvent>,
    Iter: Iterator<Item = E>
{
    fn get_state(&self) -> &PathState { &self.state }
}

impl<E, Iter> Iterator for SvgPathIter<Iter>
where
    E: Into<SvgEvent>,
    Iter: Iterator<Item = E>
{
    type Item = SvgEvent;
    fn next(&mut self) -> Option<SvgEvent> {
        if let Some(evt) = self.it.next() {
            let svg_evt = evt.into();
            self.state.svg_event(svg_evt);
            return Some(svg_evt);
        }
        return None;
    }
}

/// An adapater iterator that implements PathIterator on top of an Iterator<Item=PathEvent>.
pub struct PathIter<Iter> {
    it: Iter,
    state: PathState,
}

impl<E, Iter> PathIter<Iter>
where
    E: Into<PathEvent>,
    Iter: Iterator<Item = E>
{
    pub fn new(it: Iter) -> Self {
        PathIter {
            it: it,
            state: PathState::new(),
        }
    }
}


impl<E, Iter> PathIterator for PathIter<Iter>
where
    E: Into<PathEvent>,
    Iter: Iterator<Item = E>
{
    fn get_state(&self) -> &PathState { &self.state }
}

impl<E, Iter> Iterator for PathIter<Iter>
where
    E: Into<PathEvent>,
    Iter: Iterator<Item = E>
{
    type Item = PathEvent;
    fn next(&mut self) -> Option<PathEvent> {
        if let Some(evt) = self.it.next() {
            let path_evt = evt.into();
            self.state.path_event(path_evt);
            return Some(path_evt);
        }
        return None;
    }
}

fn flattened_to_path_event(evt: FlattenedEvent) -> PathEvent { evt.to_path_event() }
fn flattened_to_svg_event(evt: FlattenedEvent) -> SvgEvent { evt.to_svg_event() }

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
    first: bool,
    done: bool,
    close: bool,
}

impl<Iter: Iterator<Item = Point>> FromPolyline<Iter> {
    pub fn new(closed: bool, iter: Iter) -> Self {
        FromPolyline {
            iter: iter,
            first: true,
            done: false,
            close: closed,
        }
    }

    pub fn closed(iter: Iter) -> Self { FromPolyline::new(true, iter) }

    pub fn open(iter: Iter) -> Self { FromPolyline::new(false, iter) }

    /// Consumes self and returns an adapter that implements PathIterator.
    pub fn path_iter(self) -> PathIter<Self> { PathIter::new(self) }
}

impl<Iter> Iterator for FromPolyline<Iter>
where
    Iter: Iterator<Item = Point>,
{
    type Item = FlattenedEvent;

    fn next(&mut self) -> Option<FlattenedEvent> {
        if self.done {
            return None;
        }

        if let Some(next) = self.iter.next() {
            return Some(
                if self.first {
                    self.first = false;
                    FlattenedEvent::MoveTo(next)
                } else {
                    FlattenedEvent::LineTo(next)
                }
            );
        }

        self.done = true;
        if self.close {
            return Some(FlattenedEvent::Close);
        }

        return None;
    }
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

    assert_eq!(evts.next(), Some(FlattenedEvent::MoveTo(point(1.0, 1.0))));
    assert_eq!(evts.next(), Some(FlattenedEvent::LineTo(point(3.0, 1.0))));
    assert_eq!(evts.next(), Some(FlattenedEvent::LineTo(point(4.0, 5.0))));
    assert_eq!(evts.next(), Some(FlattenedEvent::LineTo(point(5.0, 2.0))));
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

    assert_eq!(evts.next(), Some(FlattenedEvent::MoveTo(point(1.0, 1.0))));
    assert_eq!(evts.next(), Some(FlattenedEvent::LineTo(point(3.0, 1.0))));
    assert_eq!(evts.next(), Some(FlattenedEvent::LineTo(point(4.0, 5.0))));
    assert_eq!(evts.next(), Some(FlattenedEvent::LineTo(point(5.0, 2.0))));
    assert_eq!(evts.next(), Some(FlattenedEvent::Close));
}
