use std::iter;

use core::math::*;
use core::{PathEvent, SvgEvent, FlattenedEvent, PathState};
use bezier::QuadraticBezierSegment;
use bezier::quadratic_bezier;
use bezier::CubicBezierSegment;
use bezier::cubic_bezier;

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

    /// Returns an iterator of SVG events.
    fn svg_events(self) -> iter::Map<Self, fn(PathEvent) -> SvgEvent> { self.map(path_to_svg_event) }
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
        return match self.it.next() {
                   Some(svg_evt) => Some(self.get_state().svg_to_path_event(svg_evt)),
                   None => None,
               };
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

impl<Iter: Iterator<Item = SvgEvent>> SvgPathIter<Iter> {
    pub fn new(it: Iter) -> Self {
        SvgPathIter {
            it: it,
            state: PathState::new(),
        }
    }
}

impl<Iter> SvgIterator for SvgPathIter<Iter>
where
    Iter: Iterator<Item = SvgEvent>,
{
    fn get_state(&self) -> &PathState { &self.state }
}

impl<Iter: Iterator<Item = SvgEvent>> Iterator for SvgPathIter<Iter> {
    type Item = SvgEvent;
    fn next(&mut self) -> Option<SvgEvent> {
        let next = self.it.next();
        if let Some(evt) = next {
            self.state.svg_event(evt);
        }
        return next;
    }
}

/// An adapater iterator that implements PathIterator on top of an Iterator<Item=PatheEvent>.
pub struct PathIter<Iter> {
    it: Iter,
    state: PathState,
}

impl<Iter: Iterator<Item = PathEvent>> PathIter<Iter> {
    pub fn new(it: Iter) -> Self {
        PathIter {
            it: it,
            state: PathState::new(),
        }
    }
}

impl<Iter> PathIterator for PathIter<Iter>
where
    Iter: Iterator<Item = PathEvent>,
{
    fn get_state(&self) -> &PathState { &self.state }
}

impl<Iter: Iterator<Item = PathEvent>> Iterator for PathIter<Iter> {
    type Item = PathEvent;
    fn next(&mut self) -> Option<PathEvent> {
        let next = self.it.next();
        if let Some(evt) = next {
            self.state.path_event(evt);
        }
        return next;
    }
}

fn flattened_to_path_event(evt: FlattenedEvent) -> PathEvent { evt.to_path_event() }
fn flattened_to_svg_event(evt: FlattenedEvent) -> SvgEvent { evt.to_svg_event() }
fn path_to_svg_event(evt: PathEvent) -> SvgEvent { evt.to_svg_event() }

/// An iterator that consumes an iterator of `Point`s and produces `FlattenedEvent`s.
///
/// # Example
///
/// ```
/// # extern crate lyon_path_iterator;
/// # use lyon_path_iterator::FromPolyline;
/// # use lyon_path_iterator::math::point;
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
