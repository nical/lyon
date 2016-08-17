extern crate lyon_core;
extern crate lyon_bezier;

use std::iter;

use lyon_core::{ PrimitiveEvent, SvgEvent, FlattenedEvent, PositionState };
use lyon_core::math::*;
use lyon_bezier::{
    QuadraticBezierSegment, CubicBezierSegment,
    QuadraticFlattenIter, CubicFlattenIter
};

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
pub trait PrimitiveIterator : Iterator<Item=PrimitiveEvent> + Sized {
    /// The returned structure exposes the current position, the first position in the current
    /// sub-path, and the position of the last control point.
    fn get_state(&self) -> &PositionState;

    /// Returns an iterator that turns curves into line segments.
    fn flattened(self, tolerance: f32) -> FlattenIter<Self> { FlattenIter::new(tolerance, self) }

    /// Returns an iterator of SVG events.
    fn to_svg(self) -> iter::Map<Self, fn(PrimitiveEvent)->SvgEvent> {
        self.map(primitive_to_svg_event)
    }
}

/// An extension to the common Iterator interface, that adds information which is useful when
/// chaining path-specific iterators.
pub trait SvgIterator : Iterator<Item=SvgEvent> + Sized {
    /// The returned structure exposes the current position, the first position in the current
    /// sub-path, and the position of the last control point.
    fn get_state(&self) -> &PositionState;

    /// Returns an iterator of FlattenedEvents, turning curves into sequences of line segments.
    fn flattened(self, tolerance: f32) -> FlattenIter<SvgToPrimitiveIter<Self>> { FlattenIter::new(tolerance, self.to_primitive()) }

    /// Returns an iterator of primitive events.
    fn to_primitive(self) -> SvgToPrimitiveIter<Self> { SvgToPrimitiveIter::new(self) }
}

/// An extension to the common Iterator interface, that adds information which is useful when
/// chaining path-specific iterators.
pub trait FlattenedIterator : Iterator<Item=FlattenedEvent> + Sized {
    /// The returned structure exposes the current position, the first position in the current
    /// sub-path, and the position of the last control point.
    fn get_state(&self) -> &PositionState;

    /// Returns an iterator of primitive events.
    fn to_primitive(self) -> iter::Map<Self, fn(FlattenedEvent)->PrimitiveEvent> {
        self.map(flattened_to_primitive_event)
    }

    /// Returns an iterator of svg events.
    fn to_svg(self) -> iter::Map<Self, fn(FlattenedEvent)->SvgEvent> {
        self.map(flattened_to_svg_event)
    }
}

/// Consumes an iterator of path events and yields segments.
pub struct SegmentIterator<PathIt> {
    it: PathIt,
    state: PositionState,
    in_sub_path: bool,
}

impl<'l, PathIt:'l+Iterator<Item=PrimitiveEvent>> SegmentIterator<PathIt> {
    /// Constructor.
    pub fn new(it: PathIt) -> Self {
        SegmentIterator {
            it: it,
            state: PositionState::new(),
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

impl<'l, PathIt:'l+Iterator<Item=PrimitiveEvent>> Iterator
for SegmentIterator<PathIt> {
    type Item = Segment;
    fn next(&mut self) -> Option<Segment> {
        return match self.it.next() {
            Some(PrimitiveEvent::MoveTo(to)) => {
                let first = self.state.first;
                self.state.move_to(to);
                if self.in_sub_path && first != self.state.current {
                    Some(Segment::Line(first, self.state.current))
                } else {
                    self.in_sub_path = true;
                    self.next()
                }
            }
            Some(PrimitiveEvent::LineTo(to)) => {
                self.in_sub_path = true;
                let from = self.state.current;
                self.state.line_to(to);
                Some(Segment::Line(from, to))
            }
            Some(PrimitiveEvent::QuadraticTo(ctrl, to)) => {
                self.in_sub_path = true;
                let from = self.state.current;
                self.state.curve_to(ctrl, to);
                Some(Segment::QuadraticBezier(from, ctrl, to))
            }
            Some(PrimitiveEvent::CubicTo(ctrl1, ctrl2, to)) => {
                self.in_sub_path = true;
                let from = self.state.current;
                self.state.curve_to(ctrl2, to);
                Some(Segment::CubicBezier(from, ctrl1, ctrl2, to))
            }
            Some(PrimitiveEvent::Close) => { self.close() }
            None => { None }
        };
    }
}

pub struct SvgToPrimitiveIter<SvgIter> {
    it: SvgIter,
}

impl<SvgIter> SvgToPrimitiveIter<SvgIter> {
    pub fn new(it: SvgIter) -> Self { SvgToPrimitiveIter { it: it } }
}

impl<SvgIter> PrimitiveIterator for SvgToPrimitiveIter<SvgIter>
where SvgIter : SvgIterator {
    fn get_state(&self) -> &PositionState { self.it.get_state() }
}

impl<SvgIter> Iterator for SvgToPrimitiveIter<SvgIter>
where SvgIter: SvgIterator {
    type Item = PrimitiveEvent;
    fn next(&mut self) -> Option<PrimitiveEvent> {
        return match self.it.next() {
            Some(svg_evt) => { Some(self.get_state().svg_to_primitive(svg_evt)) }
            None => { None }
        }
    }
}

/// An iterator that consumes an PrimitiveIterator and yields FlattenedEvents.
pub struct FlattenIter<Iter> {
    it: Iter,
    current_curve: TmpFlattenIter,
    tolerance: f32,
}

enum TmpFlattenIter {
    Quadratic(QuadraticFlattenIter),
    Cubic(CubicFlattenIter),
    None,
}

impl<Iter: PrimitiveIterator> FlattenIter<Iter> {
    /// Create the iterator.
    pub fn new(tolerance: f32, it: Iter) -> Self {
        FlattenIter {
            it: it,
            current_curve: TmpFlattenIter::None,
            tolerance: tolerance,
        }
    }
}

impl<Iter> FlattenedIterator for FlattenIter<Iter>
where Iter : PrimitiveIterator {
    fn get_state(&self) -> &PositionState { self.it.get_state() }
}

impl<Iter> Iterator for FlattenIter<Iter>
where Iter: PrimitiveIterator {
    type Item = FlattenedEvent;
    fn next(&mut self) -> Option<FlattenedEvent> {
        match self.current_curve {
            TmpFlattenIter::Quadratic(ref mut it) => {
                if let Some(point) = it.next() {
                    return Some(FlattenedEvent::LineTo(point));
                }
            }
            TmpFlattenIter::Cubic(ref mut it) => {
                if let Some(point) = it.next() {
                    return Some(FlattenedEvent::LineTo(point));
                }
            }
            _ => {}
        }
        self.current_curve = TmpFlattenIter::None;
        let current = self.get_state().current;
        return match self.it.next() {
            Some(PrimitiveEvent::MoveTo(to)) => { Some(FlattenedEvent::MoveTo(to)) }
            Some(PrimitiveEvent::LineTo(to)) => { Some(FlattenedEvent::LineTo(to)) }
            Some(PrimitiveEvent::Close) => { Some(FlattenedEvent::Close) }
            Some(PrimitiveEvent::QuadraticTo(ctrl, to)) => {
                self.current_curve = TmpFlattenIter::Quadratic(
                    QuadraticBezierSegment {
                        from: current, cp: ctrl, to: to
                    }.flatten_iter(self.tolerance)
                );
                return self.next();
            }
            Some(PrimitiveEvent::CubicTo(ctrl1, ctrl2, to)) => {
                self.current_curve = TmpFlattenIter::Cubic(
                    CubicBezierSegment {
                        from: current, cp1: ctrl1, cp2: ctrl2, to: to
                    }.flatten_iter(self.tolerance)
                );
                return self.next();
            }
            None => { None }
        }
    }
}

/// An adapater iterator that implements SvgIterator on top of an Iterator<Item=SvgEvent>.
pub struct PositionedSvgIter<Iter> {
    it: Iter,
    state: PositionState,
}

impl<Iter: Iterator<Item=SvgEvent>> PositionedSvgIter<Iter> {
    pub fn new(it: Iter) -> Self { PositionedSvgIter { it: it, state: PositionState::new() } }
}

impl<Iter> SvgIterator for PositionedSvgIter<Iter>
where Iter: Iterator<Item=SvgEvent> {
    fn get_state(&self) -> &PositionState { &self.state }
}

impl<Iter: Iterator<Item=SvgEvent>> Iterator for PositionedSvgIter<Iter> {
    type Item = SvgEvent;
    fn next(&mut self) -> Option<SvgEvent> {
        let next = self.it.next();
        if let Some(evt) = next {
            self.state.svg_event(evt);
        }
        return next;
    }
}

/// An adapater iterator that implements PrimitiveIterator on top of an Iterator<Item=PrimitveEvent>.
pub struct PositionedPrimitiveIter<Iter> {
    it: Iter,
    state: PositionState,
}

impl<Iter: Iterator<Item=PrimitiveEvent>> PositionedPrimitiveIter<Iter> {
  pub fn new(it: Iter) -> Self { PositionedPrimitiveIter { it: it, state: PositionState::new() } }
}

impl<Iter> PrimitiveIterator for PositionedPrimitiveIter<Iter>
where Iter : Iterator<Item=PrimitiveEvent> {
    fn get_state(&self) -> &PositionState { &self.state }
}

impl<Iter: Iterator<Item=PrimitiveEvent>> Iterator for PositionedPrimitiveIter<Iter> {
    type Item = PrimitiveEvent;
    fn next(&mut self) -> Option<PrimitiveEvent> {
        let next = self.it.next();
        if let Some(evt) = next {
            self.state.primitive_event(evt);
        }
        return next;
    }
}

fn flattened_to_primitive_event(evt: FlattenedEvent) -> PrimitiveEvent { evt.to_primitive() }
fn flattened_to_svg_event(evt: FlattenedEvent) -> SvgEvent { evt.to_svg() }
fn primitive_to_svg_event(evt: PrimitiveEvent) -> SvgEvent { evt.to_svg() }

#[test]
fn test_svg_to_flattened_iter() {
    let mut it = PositionedSvgIter::new(&[
        SvgEvent::MoveTo(point(1.0, 1.0)),
        SvgEvent::LineTo(point(2.0, 2.0)),
        SvgEvent::LineTo(point(3.0, 3.0)),
        SvgEvent::MoveTo(point(0.0, 0.0)),
        SvgEvent::QuadraticTo(point(1.0, 0.0), point(1.0, 1.0)),
        SvgEvent::MoveTo(point(10.0, 10.0)),
        SvgEvent::CubicTo(point(11.0, 10.0), point(11.0, 11.0), point(11.0, 11.0)),
    ].iter()).to_flattened();

    assert_eq!(it.next(), FlattenedEvent::MoveTo(1.0, 1.0));
    assert_eq!(it.next(), FlattenedEvent::LineTo(2.0, 2.0));
    assert_eq!(it.next(), FlattenedEvent::LineTo(3.0, 3.0));
    assert_eq!(it.next(), FlattenedEvent::MoveTo(0.0, 0.0));

    // Flattened quadratic curve.
    loop {
        let evt = it.next();
        if evt == Some(FlattenedEvent::MoveTo(10.0, 10.0)) = evt {
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
