//! The default path data structure.

use crate::math::*;
pub use crate::{EndpointId, CtrlPointId};
use crate::events::{PathEvent, IdEvent};

use std::iter::IntoIterator;
use std::u32;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
enum Verb {
    LineTo = 0,
    QuadraticTo = 1,
    CubicTo = 2,
    MoveTo = 3,
    Close = 4,
    End = 5,
}

pub trait Vertex : Clone {
    fn position(&self) -> Point;
    fn set_position(&mut self, pos: Point);
    fn interpolate(a: &Self, b: &Self, t: f32) -> Self;
}

impl Vertex for Point {
    fn position(&self) -> Point { *self }
    fn set_position(&mut self, pos: Point) { *self = pos; }
    fn interpolate(a: &Self, b: &Self, t: f32) -> Self { a.lerp(*b, t) }
}

/// A simple path data structure.
///
/// It can be created using a [Builder](struct.Builder.html), and can be iterated over.
#[derive(Clone, Debug, Default)]
pub struct Path<Endpoint, CtrlPoint> {
    endpoints: Box<[Endpoint]>,
    ctrl_points: Box<[CtrlPoint]>,
    verbs: Box<[Verb]>,
}

/// A view on a `Path`.
#[derive(Copy, Clone, Debug)]
pub struct PathSlice<'l, Endpoint, CtrlPoint> {
    endpoints: &'l [Endpoint],
    ctrl_points: &'l [CtrlPoint],
    verbs: &'l [Verb],
}

impl<Endpoint: Vertex, CtrlPoint: Vertex> Path<Endpoint, CtrlPoint> {
    /// Creates a [Builder](struct.Builder.html) to create a path.
    pub fn builder() -> Builder<Endpoint, CtrlPoint> { Builder::new() }

    /// Creates an Empty `Path`.
    pub fn new() -> Self {
        Path {
            endpoints: Box::new([]),
            ctrl_points: Box::new([]),
            verbs: Box::new([]),
        }
    }

    /// Returns a view on this `Path`.
    pub fn as_slice(&self) -> PathSlice<Endpoint, CtrlPoint> {
        PathSlice {
            endpoints: &self.endpoints[..],
            ctrl_points: &self.ctrl_points[..],
            verbs: &self.verbs[..],
        }
    }

    // Iterates over the entire `Path`.
    pub fn iter(&self) -> RefIter<Endpoint, CtrlPoint> {
        RefIter {
            verbs: self.verbs.iter(),
            endpoints: self.endpoints.iter(),
            ctrl_points: self.ctrl_points.iter(),
            first_endpoint: &self.endpoints[0],
            current_endpoint: &self.endpoints[0],
        }
    }

    // Iterates over the entire `Path`.
    pub fn id_iter(&self) -> IdIter {
        IdIter {
            verbs: self.verbs.iter(),
            current_endpoint: 0,
            current_ctrl: 0,
            first_endpoint: 0,
        }
    }

    pub fn endpoints(&self) -> &[Endpoint] { &self.endpoints[..] }

    pub fn mut_endpoints(&mut self) -> &mut [Endpoint] { &mut self.endpoints[..] }

    pub fn ctrl_points(&self) -> &[CtrlPoint] { &self.ctrl_points[..] }

    pub fn mut_ctrl_points(&mut self) -> &mut [CtrlPoint] { &mut self.ctrl_points[..] }

    /// Concatenate two paths.
    pub fn merge(&self, other: &Self) -> Self {
        let mut verbs = Vec::with_capacity(self.verbs.len() + other.verbs.len());
        let mut endpoints = Vec::with_capacity(self.endpoints.len() + other.endpoints.len());
        let mut ctrl_points = Vec::with_capacity(self.ctrl_points.len() + other.ctrl_points.len());
        verbs.extend_from_slice(&self.verbs);
        verbs.extend_from_slice(&other.verbs);
        endpoints.extend_from_slice(&self.endpoints);
        endpoints.extend_from_slice(&other.endpoints);
        ctrl_points.extend_from_slice(&self.ctrl_points);
        ctrl_points.extend_from_slice(&other.ctrl_points);

        Path {
            verbs: verbs.into_boxed_slice(),
            endpoints: endpoints.into_boxed_slice(),
            ctrl_points: ctrl_points.into_boxed_slice(),
        }
    }
}

pub struct Builder<Endpoint, CtrlPoint> {
    endpoints: Vec<Endpoint>,
    ctrl_points: Vec<CtrlPoint>,
    verbs: Vec<Verb>,
    first_endpoint: EndpointId,
    first_verb: u32,
    prev_cmd: Verb,
}

impl<Endpoint: Vertex, CtrlPoint: Vertex> Builder<Endpoint, CtrlPoint> {
    pub fn new() -> Self { Builder::with_capacity(0, 0, 0) }

    pub fn with_capacity(endpoints: usize, ctrl_points: usize, edges: usize) -> Self {
        Builder {
            endpoints: Vec::with_capacity(endpoints),
            ctrl_points: Vec::with_capacity(ctrl_points),
            verbs: Vec::with_capacity(edges),
            first_endpoint: EndpointId(0),
            first_verb: 0,
            prev_cmd: Verb::End,
        }
    }

    pub fn move_to(&mut self, to: Endpoint) {
        self.first_endpoint = EndpointId(self.endpoints.len() as u32);
        self.first_verb = self.verbs.len() as u32;
        self.endpoints.push(to);
        self.verbs.push(Verb::MoveTo);
        self.prev_cmd = Verb::MoveTo;
    }

    pub fn line_to(&mut self, to: Endpoint) {
        if !self.edge_to(to) {
            return;
        }
        self.verbs.push(Verb::LineTo);
        self.prev_cmd = Verb::LineTo;
    }

    pub fn quadratic_bezier_to(&mut self, ctrl: CtrlPoint, to: Endpoint) {
        if !self.edge_to(to) {
            return;
        }
        self.ctrl_points.push(ctrl);
        self.verbs.push(Verb::QuadraticTo);
        self.prev_cmd = Verb::QuadraticTo;
    }

    pub fn cubic_bezier_to(&mut self, ctrl1: CtrlPoint, ctrl2: CtrlPoint, to: Endpoint) {
        if !self.edge_to(to) {
            return;
        }
        self.ctrl_points.push(ctrl1);
        self.ctrl_points.push(ctrl2);
        self.verbs.push(Verb::CubicTo);
        self.prev_cmd = Verb::CubicTo;
    }

    pub fn close(&mut self) {
        if self.endpoints.is_empty() {
            return;
        }

        let first_position = self.endpoints[self.first_endpoint.to_usize()].position();
        let p = self.endpoints.last_mut().unwrap();

        // Relative path ops tend to accumulate small floating point imprecisions
        // which results in the last segment ending almost but not quite at the
        // start of the sub-path, causing a new edge to be inserted which often
        // intersects with the first or last edge. This can affect algorithms that
        // Don't handle self-intersecting paths.
        // Deal with this by snapping the last point if it is very close to the
        // start of the sub path.
        let d = (p.position() - first_position).abs();
        if d.x + d.y < 0.0001 {
            p.set_position(first_position);
            if let Some(verb) = self.verbs.last_mut() {
                *verb = Verb::Close;
            }
        } else {
            self.verbs.push(Verb::Close);
        }

        self.prev_cmd = Verb::Close;
    }

    pub fn current_position(&self) -> Point {
        self.endpoints.last()
            .map(|p| p.position())
            .unwrap_or_else(|| point(0.0, 0.0))
    }

    pub fn build(self) -> Path<Endpoint, CtrlPoint> {
        Path {
            endpoints: self.endpoints.into_boxed_slice(),
            ctrl_points: self.ctrl_points.into_boxed_slice(),
            verbs: self.verbs.into_boxed_slice(),
        }
    }

    fn edge_to(&mut self, to: Endpoint) -> bool {
        if (self.prev_cmd as u8) < (Verb::MoveTo as u8) {
            self.endpoints.push(to);
            return true;
        }

        let first_offset = self.first_endpoint.to_usize();
        if first_offset < self.endpoints.len() {
            let first = self.endpoints[first_offset].clone();
            self.move_to(first);
            self.endpoints.push(to);
            return true;
        }

        self.verbs.push(Verb::End);

        self.move_to(to);
        return false;
    }
}

impl<'l, Endpoint: Vertex, CtrlPoint: Vertex> IntoIterator for &'l Path<Endpoint, CtrlPoint> {
    type Item = PathEvent<&'l Endpoint, &'l CtrlPoint>;
    type IntoIter = RefIter<'l, Endpoint, CtrlPoint>;

    fn into_iter(self) -> RefIter<'l, Endpoint, CtrlPoint> { self.iter() }
}

impl<'l, Endpoint: Vertex, CtrlPoint: Vertex> Into<PathSlice<'l, Endpoint, CtrlPoint>> for &'l Path<Endpoint, CtrlPoint> {
    fn into(self) -> PathSlice<'l, Endpoint, CtrlPoint> {
        self.as_slice()
    }
}

impl<'l, Endpoint, CtrlPoint> PathSlice<'l, Endpoint, CtrlPoint> {

    // Iterates over the entire `Path`.
    pub fn iter(&self) -> RefIter<Endpoint, CtrlPoint> {
        RefIter {
            verbs: self.verbs.iter(),
            endpoints: self.endpoints.iter(),
            ctrl_points: self.ctrl_points.iter(),
            first_endpoint: &self.endpoints[0],
            current_endpoint: &self.endpoints[0],
        }
    }

    // Iterates over the entire `Path`.
    pub fn id_iter(&self) -> IdIter {
        IdIter {
            verbs: self.verbs.iter(),
            current_endpoint: 0,
            current_ctrl: 0,
            first_endpoint: 0,
        }
    }

    pub fn endpoints(&self) -> &[Endpoint] { self.endpoints }
    pub fn ctrl_points(&self) -> &[CtrlPoint] { self.ctrl_points }
}

/// An iterator for `Path` and `PathSlice`.
#[derive(Clone, Debug)]
pub struct IdIter<'l> {
    verbs: std::slice::Iter<'l, Verb>,
    current_endpoint: u32,
    current_ctrl: u32,
    first_endpoint: u32,
}

impl<'l> Iterator for IdIter<'l> {
    type Item = IdEvent;
    fn next(&mut self) -> Option<PathEvent<EndpointId, CtrlPointId>> {
        match self.verbs.next() {
            Some(&Verb::LineTo) => {
                let from = EndpointId(self.current_endpoint);
                self.current_endpoint += 1;
                return Some(PathEvent::Line {
                    from,
                    to: EndpointId(self.current_endpoint),
                });
            }
            Some(&Verb::QuadraticTo) => {
                let from = EndpointId(self.current_endpoint);
                let ctrl = CtrlPointId(self.current_ctrl);
                self.current_endpoint += 1;
                self.current_ctrl += 1;
                return Some(PathEvent::Quadratic {
                    from,
                    to: EndpointId(self.current_endpoint),
                    ctrl,
                });
            }
            Some(&Verb::CubicTo) => {
                let from = EndpointId(self.current_endpoint);
                let ctrl = self.current_ctrl;
                self.current_endpoint += 1;
                self.current_ctrl += 2;
                return Some(PathEvent::Cubic {
                    from,
                    to: EndpointId(self.current_endpoint),
                    ctrl1: CtrlPointId(ctrl),
                    ctrl2: CtrlPointId(ctrl + 1),
                });
            }
            Some(&Verb::Close) => {
                return Some(PathEvent::End {
                    last: EndpointId(self.current_endpoint),
                    first: EndpointId(self.first_endpoint),
                    close: true,
                });
            }
            Some(&Verb::End) => {
                return Some(PathEvent::End {
                    last: EndpointId(self.current_endpoint),
                    first: EndpointId(self.first_endpoint),
                    close: false,
                });
            }
            Some(&Verb::MoveTo) => {
                self.current_endpoint += 1;
                self.first_endpoint = self.current_endpoint;
                return Some(PathEvent::Begin {
                    at: EndpointId(self.current_endpoint),
                });
            }
            None => { return None; }
        }
    }
}

/// An iterator for `Path` and `PathSlice`.
#[derive(Clone, Debug)]
pub struct RefIter<'l, Endpoint, CtrlPoint> {
    verbs: std::slice::Iter<'l, Verb>,
    endpoints: std::slice::Iter<'l, Endpoint>,
    ctrl_points: std::slice::Iter<'l, CtrlPoint>,
    current_endpoint: &'l Endpoint,
    first_endpoint: &'l Endpoint,
}


impl<'l, Endpoint, CtrlPoint> Iterator for RefIter<'l, Endpoint, CtrlPoint> {
    type Item = PathEvent<&'l Endpoint, &'l CtrlPoint>;
    fn next(&mut self) -> Option<PathEvent<&'l Endpoint, &'l CtrlPoint>> {
        match self.verbs.next() {
            Some(&Verb::MoveTo) => {
                self.current_endpoint = self.endpoints.next().unwrap();
                self.first_endpoint = self.current_endpoint;
                Some(PathEvent::Begin {
                    at: self.current_endpoint,
                })
            }
            Some(&Verb::LineTo) => {
                let from = self.current_endpoint;
                self.current_endpoint = self.endpoints.next().unwrap();
                Some(PathEvent::Line {
                    from, to: self.current_endpoint,
                })
            }
            Some(&Verb::QuadraticTo) => {
                let from = self.current_endpoint;
                let ctrl = self.ctrl_points.next().unwrap();
                self.current_endpoint = self.endpoints.next().unwrap();
                Some(PathEvent::Quadratic {
                    from, ctrl, to: self.current_endpoint,
                })
            }
            Some(&Verb::CubicTo) => {
                let from = self.current_endpoint;
                let ctrl1 = self.ctrl_points.next().unwrap();
                let ctrl2 = self.ctrl_points.next().unwrap();
                self.current_endpoint = self.endpoints.next().unwrap();
                Some(PathEvent::Cubic {
                    from, ctrl1, ctrl2, to: self.current_endpoint,
                })
            }
            Some(&Verb::Close) => {
                let last = self.current_endpoint;
                self.current_endpoint = self.first_endpoint;
                Some(PathEvent::End {
                    last,
                    first: self.first_endpoint,
                    close: true,
                })
            }
            Some(&Verb::End) => {
                let last = self.current_endpoint;
                self.current_endpoint = self.first_endpoint;
                Some(PathEvent::End {
                    last,
                    first: self.first_endpoint,
                    close: false,
                })
            }
            None => None,
        }
    }
}

#[test]
fn custom_vertices() {
    #[derive(Clone, Debug)]
    pub struct MyEndpoint {
        pos: Point,
        width: f32,
    };

    #[derive(Clone, Debug)]
    pub struct MyCtrlPoint {
        pos: Point,
    };

    impl Vertex for MyEndpoint {
        fn position(&self) -> Point { self.pos }
        fn set_position(&mut self, pos: Point) { self.pos = pos; }
        fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
            MyEndpoint { pos: a.pos.lerp(b.pos, t), width: a.width * (1.0 - t) + b.width * t }
        }
    }
    impl Vertex for MyCtrlPoint {
        fn position(&self) -> Point { self.pos }
        fn set_position(&mut self, pos: Point) { self.pos = pos; }
        fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
            MyCtrlPoint { pos: a.pos.lerp(b.pos, t) }
        }
    }

    let mut builder = Path::builder();
    builder.move_to(MyEndpoint { pos: point(0.0, 0.0), width: 1.0 });
    builder.line_to(MyEndpoint { pos: point(10.0, 0.0), width: 2.0 });
    builder.quadratic_bezier_to(
        MyCtrlPoint { pos: point(5.0, 5.0) },
        MyEndpoint { pos: point(0.0, 0.0), width: 1.0 },
    );
    builder.close();
    let _path = builder.build();
}

