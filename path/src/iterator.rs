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
//! extern crate lyon_path;
//! use lyon_path::iterator::*;
//! use lyon_path::math::{point, vector};
//! use lyon_path::{PathEvent, SvgEvent, FlattenedEvent};
//!
//! fn main() {
//!     let events = vec![
//!         SvgEvent::MoveTo(point(1.0, 1.0)),
//!         SvgEvent::RelativeQuadraticTo(vector(4.0, 5.0), vector(-1.0, 4.0)),
//!         SvgEvent::CubicTo(point(3.0, 0.0), point(3.0, 1.0), point(10.0, -3.0)),
//!         SvgEvent::Close,
//!     ];
//!
//!     // A simple std::iter::Iterator<SvgEvent>,
//!     let simple_iter = events.iter().cloned();
//!
//!     // Make it a SvgIterator (keeps tracks of the path state).
//!     let svg_path_iter = SvgPathIter::new(simple_iter);
//!
//!     // Make it a PathEvent iterator.
//!     let path_iter = svg_path_iter.path_events();
//!
//!     // Make it an iterator over even simpler primitives: FlattenedEvent,
//!     // which do not contain any curve. To do so we approximate each curve
//!     // linear segments according to a tolerance threshold which controls
//!     // the tradeoff between fidelity of the approximation and amount of
//!     // generated events. Let's use a tolerance threshold of 0.01.
//!     // The beauty of this approach is that the flattening happens lazily
//!     // while iterating without allocating memory for the path.
//!     let flattened_iter = path_iter.flattened(0.01);
//!
//!     for evt in flattened_iter {
//!         match evt {
//!             FlattenedEvent::MoveTo(p) => { println!(" - move to {:?}", p); }
//!             FlattenedEvent::Line(segment) => { println!(" - line {:?}", segment); }
//!             FlattenedEvent::Close(segment) => { println!(" - close {:?}", segment); }
//!         }
//!     }
//! }
//! ```
//!
//! An equivalent (shorter) version of the above code takes advantage of the
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
//!
//! Sometimes, working with segments directly without dealing with MoveTo/Close events
//! can be more convenient:
//!
//! ```
//! extern crate lyon_path;
//! use lyon_path::iterator::*;
//! use lyon_path::math::{point, vector};
//! use lyon_path::geom::BezierSegment;
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
//!     let path = builder.build();
//!
//!     // Iterate over bézier segments directly.
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
use crate::{PathEvent, SvgEvent, FlattenedEvent, QuadraticEvent, PathState};
use crate::geom::{BezierSegment, QuadraticBezierSegment, CubicBezierSegment, LineSegment, quadratic_bezier, cubic_bezier};
use crate::geom::arc::*;
use crate::geom::arrayvec::ArrayVec;
use crate::builder::SvgBuilder;

/// An extension trait for `PathEvent` iterators.
pub trait PathIterator: Iterator<Item = PathEvent> + Sized {

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
    Iter: Iterator<Item = PathEvent>,
{}

/// An extension to the common Iterator interface, that adds information which is useful when
/// chaining path-specific iterators.
pub trait SvgIterator: Iterator<Item = SvgEvent> + Sized {
    /// The returned structure exposes the current position, the first position in the current
    /// sub-path, and the position of the last control point.
    fn path_state(&self) -> &PathState;

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

    /// Returns an iterator of path events.
    fn path_events(self) -> iter::Map<Self, fn(FlattenedEvent) -> PathEvent> {
        self.map(flattened_to_path_event)
    }

    /// Returns an iterator of svg events.
    fn svg_events(self) -> iter::Map<Self, fn(FlattenedEvent) -> SvgEvent> {
        self.map(flattened_to_svg_event)
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
    Iter: Iterator<Item = FlattenedEvent>,
{}

/// An extension to the common Iterator interface, that adds information which is useful when
/// chaining path-specific iterators.
pub trait QuadraticPathIterator: Iterator<Item = QuadraticEvent> + Sized {

    /// Returns an iterator of path events.
    fn path_events(self) -> iter::Map<Self, fn(QuadraticEvent) -> PathEvent> {
        self.map(quadratic_to_path_event)
    }

    /// Returns an iterator of svg events.
    fn svg_events(self) -> iter::Map<Self, fn(QuadraticEvent) -> SvgEvent> {
        self.map(quadratic_to_svg_event)
    }

    /// Returns an iterator applying a 2D transform to all of its events.
    fn transformed(self, mat: &Transform2D) -> Transformed<Self> {
        Transformed::new(mat, self)
    }
}

impl<Iter> QuadraticPathIterator for Iter
where
    Iter: Iterator<Item = QuadraticEvent>,
{}

/// Turns an iterator of SVG path commands into an iterator of `PathEvent`.
pub struct PathEvents<SvgIter> {
    it: SvgIter,
    arc_to_cubics: Vec<CubicBezierSegment<f32>>,
}

impl<SvgIter> PathEvents<SvgIter> {
    pub fn new(it: SvgIter) -> Self {
        PathEvents {
            it,
            arc_to_cubics: Vec::new(),
        }
    }
}

impl<SvgIter> Iterator for PathEvents<SvgIter>
where
    SvgIter: SvgIterator,
{
    type Item = PathEvent;
    fn next(&mut self) -> Option<PathEvent> {
        if let Some(segment) = self.arc_to_cubics.pop() {
            return Some(PathEvent::Cubic(segment));
        }
        match self.it.next() {
            Some(svg_evt) => Some(
                svg_to_path_event(
                    svg_evt,
                    &self.it.path_state().clone(),
                    &mut self.arc_to_cubics
                )
            ),
            None => None,
        }
    }
}

fn svg_to_path_event(
    event: SvgEvent,
    ps: &PathState,
    arcs_to_cubic: &mut Vec<CubicBezierSegment<f32>>
) -> PathEvent {
    let from = ps.current_position();
    match event {
        SvgEvent::MoveTo(to) => PathEvent::MoveTo(to),
        SvgEvent::LineTo(to) => PathEvent::Line(LineSegment { from, to }),
        SvgEvent::QuadraticTo(ctrl, to) => PathEvent::Quadratic(QuadraticBezierSegment {
            from, ctrl, to
        }),
        SvgEvent::CubicTo(ctrl1, ctrl2, to) => PathEvent::Cubic(CubicBezierSegment {
            from, ctrl1, ctrl2, to
        }),
        SvgEvent::Close => PathEvent::Close(LineSegment {
            from: ps.current_position(),
            to: ps.start_position(),
        }),
        SvgEvent::RelativeMoveTo(to) => PathEvent::MoveTo(ps.relative_to_absolute(to)),
        SvgEvent::RelativeLineTo(to) => PathEvent::Line(LineSegment {
            from,
            to: ps.relative_to_absolute(to)
        }),
        SvgEvent::RelativeQuadraticTo(ctrl, to) => {
            PathEvent::Quadratic(QuadraticBezierSegment {
                from,
                ctrl: ps.relative_to_absolute(ctrl),
                to: ps.relative_to_absolute(to),
            })
        }
        SvgEvent::RelativeCubicTo(ctrl1, ctrl2, to) => {
            PathEvent::Cubic(CubicBezierSegment {
                from,
                ctrl1: ps.relative_to_absolute(ctrl1),
                ctrl2: ps.relative_to_absolute(ctrl2),
                to: ps.relative_to_absolute(to),
            })
        }
        SvgEvent::HorizontalLineTo(x) => {
            PathEvent::Line(LineSegment {
                from,
                to: point(x, ps.current_position().y)
            })
        }
        SvgEvent::VerticalLineTo(y) => {
            PathEvent::Line(LineSegment {
                from,
                to: point(ps.current_position().x, y)
            })
        }
        SvgEvent::RelativeHorizontalLineTo(x) => {
            PathEvent::Line(LineSegment {
                from,
                to: point(ps.current_position().x + x, ps.current_position().y)
            })
        }
        SvgEvent::RelativeVerticalLineTo(y) => {
            PathEvent::Line(LineSegment {
                from,
                to: point(ps.current_position().x, ps.current_position().y + y)
            })
        }
        SvgEvent::SmoothQuadraticTo(to) => {
            PathEvent::Quadratic(QuadraticBezierSegment {
                from,
                ctrl: ps.get_smooth_quadratic_ctrl(),
                to
            })
        }
        SvgEvent::SmoothCubicTo(ctrl2, to) => {
            PathEvent::Cubic(CubicBezierSegment {
                from,
                ctrl1: ps.get_smooth_cubic_ctrl(),
                ctrl2,
                to
            })
        }
        SvgEvent::SmoothRelativeQuadraticTo(to) => {
            PathEvent::Quadratic(QuadraticBezierSegment {
                from,
                ctrl: ps.get_smooth_quadratic_ctrl(),
                to: ps.relative_to_absolute(to),
            })
        }
        SvgEvent::SmoothRelativeCubicTo(ctrl2, to) => {
            PathEvent::Cubic(CubicBezierSegment {
                from,
                ctrl1: ps.get_smooth_cubic_ctrl(),
                ctrl2: ps.relative_to_absolute(ctrl2),
                to: ps.relative_to_absolute(to),
            })
        }
        SvgEvent::ArcTo(radii, x_rotation, flags, to) => {
            arc_to_path_events(
                &Arc::from_svg_arc(&SvgArc {
                    from: ps.current_position(),
                    to,
                    radii,
                    x_rotation,
                    flags,
                }),
                arcs_to_cubic,
            )
        }
        SvgEvent::RelativeArcTo(radii, x_rotation, flags, to) => {
            arc_to_path_events(
                &Arc::from_svg_arc(&SvgArc {
                    from: ps.current_position(),
                    to: ps.current_position() + to,
                    radii,
                    x_rotation,
                    flags,
                }),
                arcs_to_cubic,
            )
        }
    }
}

fn arc_to_path_events(arc: &Arc<f32>, arcs_to_cubic: &mut Vec<CubicBezierSegment<f32>>) -> PathEvent {
    let mut curves: ArrayVec<[CubicBezierSegment<f32>; 4]> = ArrayVec::new();
    arc.for_each_cubic_bezier(&mut|curve: &CubicBezierSegment<f32>| {
        curves.push(*curve);
    });
    while curves.len() > 1 {
        // Append in reverse order.
        arcs_to_cubic.push(curves.pop().unwrap());
    }
    PathEvent::Cubic(curves[0])
}

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

impl<Iter: Iterator<Item = PathEvent>> Flattened<Iter> {
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
    Iter: Iterator<Item = PathEvent>,
{
    type Item = FlattenedEvent;
    fn next(&mut self) -> Option<FlattenedEvent> {
        match self.current_curve {
            TmpFlatteningIter::Quadratic(ref mut it) => {
                if let Some(to) = it.next() {
                    let from = self.current_position;
                    self.current_position = to;
                    return Some(FlattenedEvent::Line(LineSegment { from, to }));
                }
            }
            TmpFlatteningIter::Cubic(ref mut it) => {
                if let Some(to) = it.next() {
                    let from = self.current_position;
                    self.current_position = to;
                    return Some(FlattenedEvent::Line(LineSegment { from, to }));
                }
            }
            _ => {}
        }
        self.current_curve = TmpFlatteningIter::None;
        match self.it.next() {
            Some(PathEvent::MoveTo(to)) => Some(FlattenedEvent::MoveTo(to)),
            Some(PathEvent::Line(segment)) => Some(FlattenedEvent::Line(segment)),
            Some(PathEvent::Close(segment)) => Some(FlattenedEvent::Close(segment)),
            Some(PathEvent::Quadratic(segment)) => {
                self.current_position = segment.from;
                self.current_curve = TmpFlatteningIter::Quadratic(
                    segment.flattened(self.tolerance)
                );
                self.next()
            }
            Some(PathEvent::Cubic(segment)) => {
                self.current_position = segment.from;
                self.current_curve = TmpFlatteningIter::Cubic(
                    segment.flattened(self.tolerance)
                );
                self.next()
            }
            None => None,
        }
    }
}

// TODO: SvgPathIter and PathIter should be merged into a single struct using
// specialization to implement the Iterator trait depending on the type of
// event but specialization isn't stable in rust yet.

/// An adapter iterator that implements SvgIterator on top of an Iterator<Item=SvgEvent>.
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
            it,
            state: PathState::new(),
        }
    }
}

impl<E, Iter> SvgIterator for SvgPathIter<Iter>
where
    E: Into<SvgEvent>,
    Iter: Iterator<Item = E>
{
    fn path_state(&self) -> &PathState { &self.state }
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

        None
    }
}

#[inline]
fn quadratic_to_path_event(evt: QuadraticEvent) -> PathEvent { evt.to_path_event() }
#[inline]
fn quadratic_to_svg_event(evt: QuadraticEvent) -> SvgEvent { evt.to_svg_event() }
#[inline]
fn flattened_to_path_event(evt: FlattenedEvent) -> PathEvent { evt.to_path_event() }
#[inline]
fn flattened_to_svg_event(evt: FlattenedEvent) -> SvgEvent { evt.to_svg_event() }

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
    type Item = FlattenedEvent;

    fn next(&mut self) -> Option<FlattenedEvent> {
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
                Some(FlattenedEvent::MoveTo(next))
            } else {
                Some(FlattenedEvent::Line(LineSegment { from, to: next }))
            }
        }

        self.done = true;
        if self.close {
            return Some(FlattenedEvent::Close(LineSegment {
                from: self.current,
                to: self.first,
            }));
        }

        None
    }
}

/// Turns an iterator of `PathEvent` into an iterator of `BezierSegment<f32>`.
pub struct BezierSegments<Iter> {
    iter: Iter
}

impl<Iter> Iterator for BezierSegments<Iter>
where Iter: Iterator<Item = PathEvent> {
    type Item = BezierSegment<f32>;
    fn next(&mut self) -> Option<BezierSegment<f32>> {
        match self.iter.next() {
            Some(PathEvent::Line(segment))
            | Some(PathEvent::Close(segment))
            => Some(BezierSegment::Linear(segment)),
            Some(PathEvent::Quadratic(segment)) => Some(BezierSegment::Quadratic(segment)),
            Some(PathEvent::Cubic(segment)) => Some(BezierSegment::Cubic(segment)),
            Some(PathEvent::MoveTo(..)) => self.next(),
            None => None,
        }
    }
}

/// Turns an iterator of `FlattenedEvent` into an iterator of `LineSegment<f32>`.
pub struct LineSegments<Iter> {
    iter: Iter
}

impl<Iter> Iterator for LineSegments<Iter>
where Iter: Iterator<Item = FlattenedEvent> {
    type Item = LineSegment<f32>;
    fn next(&mut self) -> Option<LineSegment<f32>> {
        match self.iter.next() {
            Some(FlattenedEvent::Line(segment))
            | Some(FlattenedEvent::Close(segment))
            => Some(segment),
            Some(FlattenedEvent::MoveTo(..)) => self.next(),
            None => None,
        }
    }
}

/// Computes the length of a flattened path.
fn flattened_path_length<T>(iter: T) -> f32
where T: Iterator<Item = FlattenedEvent> {
    let mut length = 0.0;
    for evt in iter {
        match evt {
            FlattenedEvent::MoveTo(..) => {}
            FlattenedEvent::Line(segment) => { length += segment.length(); }
            FlattenedEvent::Close(segment) => { length += segment.length(); }
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

    assert_eq!(evts.next(), Some(FlattenedEvent::MoveTo(point(1.0, 1.0))));
    assert_eq!(evts.next(), Some(FlattenedEvent::Line(LineSegment { from: point(1.0, 1.0), to: point(3.0, 1.0) })));
    assert_eq!(evts.next(), Some(FlattenedEvent::Line(LineSegment { from: point(3.0, 1.0), to: point(4.0, 5.0) })));
    assert_eq!(evts.next(), Some(FlattenedEvent::Line(LineSegment { from: point(4.0, 5.0), to: point(5.0, 2.0) })));
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
    assert_eq!(evts.next(), Some(FlattenedEvent::Line(LineSegment { from: point(1.0, 1.0), to: point(3.0, 1.0) })));
    assert_eq!(evts.next(), Some(FlattenedEvent::Line(LineSegment { from: point(3.0, 1.0), to: point(4.0, 5.0) })));
    assert_eq!(evts.next(), Some(FlattenedEvent::Line(LineSegment { from: point(4.0, 5.0), to: point(5.0, 2.0) })));
    assert_eq!(evts.next(), Some(FlattenedEvent::Close(LineSegment { from: point(5.0, 2.0), to: point(1.0, 1.0) })));
}
