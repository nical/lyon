//! # Path iterators
//!
//! Composable tools to iterate over paths.
//!
//! ## Overview
//!
//! TODO
//!
//! ## Example
//!
//! TODO
//!

use std::iter;

use core::math::*;
use core::{PathEvent, SvgEvent, FlattenedEvent, PathState};
use bezier::{QuadraticBezierSegment, QuadraticFlatteningIte};
use bezier::{CubicBezierSegment, CubicFlatteningIter};

/// Convenience for algorithms which prefer to iterate over segments directly rather than
/// path events.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Segment {
    Line(Point, Point),
    QuadraticBezier(Point, Point, Point),
    CubicBezier(Point, Point, Point, Point),
}

/// An extension to the common Iterator interface, that adds information which is useful when
/// chaining path-specific iterators.
pub trait PathIterator: Iterator<Item = PathEvent> + Sized {
    /// The returned structure exposes the current position, the first position in the current
    /// sub-path, and the position of the last control point.
    fn get_state(&self) -> &PathState;

    /// Returns an iterator that turns curves into line segments.
    fn flattened(self, tolerance: f32) -> FlatteningIter<Self> {
        FlatteningIter::new(tolerance, self)
    }

    /// Returns an iterator of SVG events.
    fn svg_iter(self) -> iter::Map<Self, fn(PathEvent) -> SvgEvent> { self.map(path_to_svg_event) }
}

/// An extension to the common Iterator interface, that adds information which is useful when
/// chaining path-specific iterators.
pub trait SvgIterator: Iterator<Item = SvgEvent> + Sized {
    /// The returned structure exposes the current position, the first position in the current
    /// sub-path, and the position of the last control point.
    fn get_state(&self) -> &PathState;

    /// Returns an iterator of FlattenedEvents, turning curves into sequences of line segments.
    fn flattened(self, tolerance: f32) -> FlatteningIter<SvgToPathIter<Self>> {
        self.path_iter().flattened(tolerance)
    }

    /// Returns an iterator of path events.
    fn path_iter(self) -> SvgToPathIter<Self> { SvgToPathIter::new(self) }
}

/// An extension to the common Iterator interface, that adds information which is useful when
/// chaining path-specific iterators.
pub trait FlattenedIterator: Iterator<Item = FlattenedEvent> + Sized {
    /// The returned structure exposes the current position, the first position in the current
    /// sub-path, and the position of the last control point.
    fn get_state(&self) -> &PathState;

    /// Returns an iterator of path events.
    fn path_iter(self) -> iter::Map<Self, fn(FlattenedEvent) -> PathEvent> {
        self.map(flattened_to_path_event)
    }

    /// Returns an iterator of svg events.
    fn svg_iter(self) -> iter::Map<Self, fn(FlattenedEvent) -> SvgEvent> {
        self.map(flattened_to_svg_event)
    }
}

/// Consumes an iterator of path events and yields segments.
pub struct SegmentIterator<PathIt> {
    it: PathIt,
    state: PathState,
    in_sub_path: bool,
}

impl<'l, PathIt: 'l + Iterator<Item = PathEvent>> SegmentIterator<PathIt> {
    /// Constructor.
    pub fn new(it: PathIt) -> Self {
        SegmentIterator {
            it: it,
            state: PathState::new(),
            in_sub_path: false,
        }
    }

    fn close(&mut self) -> Option<Segment> {
        let first = self.state.first;
        self.state.close();
        self.in_sub_path = false;
        if first != self.state.current {
            Some(Segment::Line(first, self.state.current))
        } else {
            self.next()
        }
    }
}

impl<'l, PathIt: 'l + Iterator<Item = PathEvent>> Iterator for SegmentIterator<PathIt> {
    type Item = Segment;
    fn next(&mut self) -> Option<Segment> {
        return match self.it.next() {
                   Some(PathEvent::MoveTo(to)) => {
                       let first = self.state.first;
                       self.state.move_to(to);
                       if self.in_sub_path && first != self.state.current {
                           Some(Segment::Line(first, self.state.current))
                       } else {
                           self.in_sub_path = true;
                           self.next()
                       }
                   }
                   Some(PathEvent::LineTo(to)) => {
                       self.in_sub_path = true;
                       let from = self.state.current;
                       self.state.line_to(to);
                       Some(Segment::Line(from, to))
                   }
                   Some(PathEvent::QuadraticTo(ctrl, to)) => {
                       self.in_sub_path = true;
                       let from = self.state.current;
                       self.state.curve_to(ctrl, to);
                       Some(Segment::QuadraticBezier(from, ctrl, to))
                   }
                   Some(PathEvent::CubicTo(ctrl1, ctrl2, to)) => {
                       self.in_sub_path = true;
                       let from = self.state.current;
                       self.state.curve_to(ctrl2, to);
                       Some(Segment::CubicBezier(from, ctrl1, ctrl2, to))
                   }
                   Some(PathEvent::Close) => self.close(),
                   None => None,
               };
    }
}

pub struct SvgToPathIter<SvgIter> {
    it: SvgIter,
}

impl<SvgIter> SvgToPathIter<SvgIter> {
    pub fn new(it: SvgIter) -> Self { SvgToPathIter { it: it } }
}

impl<SvgIter> PathIterator for SvgToPathIter<SvgIter>
where
    SvgIter: SvgIterator,
{
    fn get_state(&self) -> &PathState { self.it.get_state() }
}

impl<SvgIter> Iterator for SvgToPathIter<SvgIter>
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
pub struct FlatteningIter<Iter> {
    it: Iter,
    current_curve: TmpFlatteningIter,
    tolerance: f32,
}

enum TmpFlatteningIter {
    Quadratic(QuadraticFlatteningIter),
    Cubic(CubicFlatteningIter),
    None,
}

impl<Iter: PathIterator> FlatteningIter<Iter> {
    /// Create the iterator.
    pub fn new(tolerance: f32, it: Iter) -> Self {
        FlatteningIter {
            it: it,
            current_curve: TmpFlatteningIter::None,
            tolerance: tolerance,
        }
    }
}

impl<Iter> FlattenedIterator for FlatteningIter<Iter>
where
    Iter: PathIterator,
{
    fn get_state(&self) -> &PathState { self.it.get_state() }
}

impl<Iter> Iterator for FlatteningIter<Iter>
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
                    }.flattening_iter(self.tolerance)
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
                    }.flattening_iter(self.tolerance)
                );
                return self.next();
            }
            None => None,
        };
    }
}

/// An adapater iterator that implements SvgIterator on top of an Iterator<Item=SvgEvent>.
pub struct PathStateSvgIter<Iter> {
    it: Iter,
    state: PathState,
}

impl<Iter: Iterator<Item = SvgEvent>> PathStateSvgIter<Iter> {
    pub fn new(it: Iter) -> Self {
        PathStateSvgIter {
            it: it,
            state: PathState::new(),
        }
    }
}

impl<Iter> SvgIterator for PathStateSvgIter<Iter>
where
    Iter: Iterator<Item = SvgEvent>,
{
    fn get_state(&self) -> &PathState { &self.state }
}

impl<Iter: Iterator<Item = SvgEvent>> Iterator for PathStateSvgIter<Iter> {
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
pub struct PathStateIter<Iter> {
    it: Iter,
    state: PathState,
}

impl<Iter: Iterator<Item = PathEvent>> PathStateIter<Iter> {
    pub fn new(it: Iter) -> Self {
        PathStateIter {
            it: it,
            state: PathState::new(),
        }
    }
}

impl<Iter> PathIterator for PathStateIter<Iter>
where
    Iter: Iterator<Item = PathEvent>,
{
    fn get_state(&self) -> &PathState { &self.state }
}

impl<Iter: Iterator<Item = PathEvent>> Iterator for PathStateIter<Iter> {
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

/*
#[test]
fn test_svg_to_flattened_iter() {
    let mut it = PathStateSvgIter::new(
        [
            SvgEvent::MoveTo(point(1.0, 1.0)),
            SvgEvent::LineTo(point(2.0, 2.0)),
            SvgEvent::LineTo(point(3.0, 3.0)),
            SvgEvent::MoveTo(point(0.0, 0.0)),
            SvgEvent::QuadraticTo(point(1.0, 0.0), point(1.0, 1.0)),
            SvgEvent::MoveTo(point(10.0, 10.0)),
            SvgEvent::CubicTo(point(11.0, 10.0), point(11.0, 11.0), point(11.0, 11.0)),
        ].iter()
    ).flattened(0.05);

    assert_eq!(it.next(), FlattenedEvent::MoveTo(point(1.0, 1.0)));
    assert_eq!(it.next(), FlattenedEvent::LineTo(point(2.0, 2.0)));
    assert_eq!(it.next(), FlattenedEvent::LineTo(point(3.0, 3.0)));
    assert_eq!(it.next(), FlattenedEvent::MoveTo(point(0.0, 0.0)));

    // Flattened quadratic curve.
    loop {
        let evt = it.next();
        if evt == Some(FlattenedEvent::MoveTo(point(10.0, 10.0))) {
            break;
        }
        if let Some(FlattenedEvent::MoveTo(to)) = evt {
            // ok
        } else {
            panic!("Expected a MoveTo event, got {:?}", evt);
        }
    }

    // Flattened cubic curve.
    loop {
        let evt = it.next();
        if evt.is_none() {
            break;
        }
        if let Some(FlattenedEvent::MoveTo(to)) = evt {
            // ok
        } else {
            panic!("Expected a MoveTo event, got {:?}", evt);
        }
    }
}
*/
