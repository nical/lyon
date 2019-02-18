use crate::geom::{QuadraticBezierSegment, CubicBezierSegment};
use crate::math::Point;

use crate::{EndpointId, CtrlPointId};
use crate::events::{PathEvent, IdEvent};

use std::mem;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct PathEventId(pub(crate) u32);
impl PathEventId {
    //pub(crate) const INVALID: Self = PathEventId(!0u32);
    pub fn to_usize(&self) -> usize { self.0 as usize }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
enum Verb {
    LineTo = 0,
    QuadraticTo = 1,
    CubicTo = 2,
    Begin = 3,
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

type PathOp = u32;

#[repr(C)]
pub struct Cmd {
    op: Verb,
    data0: u8,
    data1: u8,
    data2: u8,
}

impl Cmd {
    const BEGIN: Self = Cmd {
        op: Verb::Begin,
        data0: 0,
        data1: 0,
        data2: 0,
    };

    const LINE: Self = Cmd {
        op: Verb::LineTo,
        data0: 0,
        data1: 0,
        data2: 0,
    };

    const QUAD: Self = Cmd {
        op: Verb::QuadraticTo,
        data0: 0,
        data1: 0,
        data2: 0,
    };

    const CUBIC: Self = Cmd {
        op: Verb::CubicTo,
        data0: 0,
        data1: 0,
        data2: 0,
    };

    const CLOSE: Self = Cmd {
        op: Verb::Close,
        data0: 0,
        data1: 0,
        data2: 0,
    };

    const END: Self = Cmd {
        op: Verb::End,
        data0: 0,
        data1: 0,
        data2: 0,
    };

    pub fn to_u32(self) -> u32 {
        unsafe { mem::transmute(self) }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct PathCommands {
    cmds: Vec<PathOp>,
}

impl PathCommands {
    pub fn new() -> Self {
        PathCommands {
            cmds: Vec::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        PathCommands {
            cmds: Vec::with_capacity(cap),
        }
    }

    pub fn builder() -> PathCommandsBuilder {
        PathCommandsBuilder::new()
    }

    pub fn write<'a, 'b : 'a>(&'b mut self) -> PathCommandsWriter<'a> {
        PathCommandsWriter::new(self)
    }

    pub fn iter_all(&self) -> IdIter {
        let id = self.path_id();
        self.iter(id)
    }

    pub fn iter(&self, path_id: PathId) -> IdIter {
        IdIter {
            cmds: (&self.cmds[path_id.range()]).iter(),
            first_endpoint: EndpointId(0),
            prev_endpoint: EndpointId(0),
        }
    }

    pub fn as_slice(&self) -> PathCommandsSlice {
        PathCommandsSlice {
            cmds: &self.cmds,
        }
    }

    pub fn slice(&self, path_id: PathId) -> PathCommandsSlice {
        PathCommandsSlice {
            cmds: &self.cmds[path_id.range()],
        }
    }

    pub fn event(&self, id: PathEventId) -> IdEvent {
        self.as_slice().event(id)
    }

    pub fn next_event_id_in_path(&self, id: PathEventId) -> Option<PathEventId> {
        self.as_slice().next_event_id_in_path(id)
    }

    pub fn next_event_id_in_sub_path(&self, id: PathEventId) -> PathEventId {
        self.as_slice().next_event_id_in_sub_path(id)
    }

    fn path_id(&self) -> PathId {
        PathId {
            start: 0,
            end: self.cmds.len() as u32,
        }
    }
}


impl<'l> IntoIterator for &'l PathCommands {
    type Item = PathEvent<EndpointId, CtrlPointId>;
    type IntoIter = IdIter<'l>;

    fn into_iter(self) -> IdIter<'l> { self.iter_all() }
}

impl<'l> Into<PathCommandsSlice<'l>> for &'l PathCommands {
    fn into(self) -> PathCommandsSlice<'l> {
        self.as_slice()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PathCommandsSlice<'l> {
    cmds: &'l [PathOp],
}

impl<'l> PathCommandsSlice<'l> {
    pub fn iter(&self) -> IdIter {
        IdIter {
            cmds: self.cmds.iter(),
            first_endpoint: EndpointId(0),
            prev_endpoint: EndpointId(0),
        }
    }

    pub fn event(&self, id: PathEventId) -> IdEvent {
        let idx = id.to_usize();
        let cmd: Cmd = unsafe { mem::transmute(self.cmds[idx]) };
        match cmd.op {
            Verb::LineTo => PathEvent::Line {
                from: EndpointId(self.cmds[idx - 1]),
                to: EndpointId(self.cmds[idx + 1]),
            },
            Verb::QuadraticTo => PathEvent::Quadratic {
                from: EndpointId(self.cmds[idx - 1]),
                ctrl: CtrlPointId(self.cmds[idx + 1]),
                to: EndpointId(self.cmds[idx + 2]),
            },
            Verb::CubicTo => PathEvent::Cubic {
                from: EndpointId(self.cmds[idx - 1]),
                ctrl1: CtrlPointId(self.cmds[idx + 1]),
                ctrl2: CtrlPointId(self.cmds[idx + 2]),
                to: EndpointId(self.cmds[idx + 3]),
            },
            Verb::Begin => PathEvent::Begin {
                at: EndpointId(self.cmds[idx + 1])
            },
            Verb::End => {
                let first_event = self.cmds[idx + 1] as usize;
                PathEvent::End {
                    last: EndpointId(self.cmds[idx - 1]),
                    first: EndpointId(self.cmds[first_event + 1]),
                    close: false,
                }
            }
            Verb::Close => {
                let first_event = self.cmds[idx + 1] as usize;
                PathEvent::End {
                    last: EndpointId(self.cmds[idx - 1]),
                    first: EndpointId(self.cmds[first_event + 1]),
                    close: true,
                }
            }
        }
    }

    pub fn next_event_id_in_sub_path(&self, id: PathEventId) -> PathEventId {
        let idx = id.to_usize();
        let cmd: Cmd = unsafe { mem::transmute(self.cmds[idx]) };
        match cmd.op {
            Verb::LineTo | Verb::Begin => PathEventId(id.0 + 2),
            Verb::QuadraticTo => PathEventId(id.0 + 3),
            Verb::CubicTo => PathEventId(id.0 + 4),
            Verb::End | Verb::Close => PathEventId(self.cmds[idx + 1]),
        }
    }

    pub fn next_event_id_in_path(&self, id: PathEventId) -> Option<PathEventId> {
        let idx = id.to_usize();
        let cmd: Cmd = unsafe { mem::transmute(self.cmds[idx]) };
        let next = match cmd.op {
            Verb::LineTo
            | Verb::Begin
            | Verb::End
            | Verb::Close => PathEventId(id.0 + 2),
            Verb::QuadraticTo => PathEventId(id.0 + 3),
            Verb::CubicTo => PathEventId(id.0 + 4),
        };

        if next.0 < self.cmds.len() as u32 {
            return Some(next);
        }

        None
    }
}

#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Path<Endpoint, CtrlPoint> {
    endpoints: Box<[Endpoint]>,
    ctrl_points: Box<[CtrlPoint]>,
    cmds: PathCommands,
}

impl<Endpoint, CtrlPoint> Path<Endpoint, CtrlPoint> {
    pub fn builder() -> PathBuilder<Endpoint, CtrlPoint> {
        PathBuilder::new()
    }

    pub fn id_iter(&self) -> IdIter {
        self.cmds.iter_all()
    }

    pub fn iter(&self) -> RefIter<Endpoint, CtrlPoint> {
        RefIter {
            cmds: self.cmds.cmds.iter(),
            first_endpoint: 0,
            prev_endpoint: 0,
            endpoints: &self.endpoints[..],
            ctrl_points: &self.ctrl_points[..],
        }
    }

    pub fn as_slice(&self) -> PathSlice<Endpoint, CtrlPoint> {
        PathSlice {
            endpoints: &self.endpoints,
            ctrl_points: &self.ctrl_points,
            cmds: self.cmds.as_slice(),
        }
    }
}

impl<'l, Endpoint, CtrlPoint> IntoIterator for &'l Path<Endpoint, CtrlPoint> {
    type Item = PathEvent<&'l Endpoint, &'l CtrlPoint>;
    type IntoIter = RefIter<'l, Endpoint, CtrlPoint>;

    fn into_iter(self) -> RefIter<'l, Endpoint, CtrlPoint> { self.iter() }
}

impl<'l, Endpoint, CtrlPoint> Into<PathSlice<'l, Endpoint, CtrlPoint>> for &'l Path<Endpoint, CtrlPoint> {
    fn into(self) -> PathSlice<'l, Endpoint, CtrlPoint> {
        self.as_slice()
    }
}

/// A view on a `Path`.
#[derive(Copy, Clone, Debug)]
pub struct PathSlice<'l, Endpoint, CtrlPoint> {
    endpoints: &'l [Endpoint],
    ctrl_points: &'l [CtrlPoint],
    cmds: PathCommandsSlice<'l>,
}

impl<'l, Endpoint, CtrlPoint> PathSlice<'l, Endpoint, CtrlPoint> {
    pub fn id_iter(&self) -> IdIter {
        self.cmds.iter()
    }

    pub fn iter(&self) -> RefIter<Endpoint, CtrlPoint> {
        RefIter {
            cmds: self.cmds.cmds.iter(),
            first_endpoint: 0,
            prev_endpoint: 0,
            endpoints: &self.endpoints[..],
            ctrl_points: &self.ctrl_points[..],
        }
    }

    pub fn sample_event(&self, evt: PathEventId, t: f32) -> Endpoint
    where
        Endpoint: Vertex,
        CtrlPoint: Vertex,
    {
        match self.cmds.event(evt) {
            PathEvent::Line { from, to } => Endpoint::interpolate(
                &self.endpoints[from.to_usize()],
                &self.endpoints[to.to_usize()],
                t
            ),
            PathEvent::Quadratic { from, to, ctrl } => {
                let from = &self.endpoints[from.to_usize()];
                let to = &self.endpoints[to.to_usize()];
                let mut result = Endpoint::interpolate(from, to, t);
                result.set_position(
                    QuadraticBezierSegment {
                        from: to.position(),
                        to: to.position(),
                        ctrl: self.ctrl_points[ctrl.to_usize()].position(),
                    }.sample(t)
                );

                result
            }
            PathEvent::Cubic { from, to, ctrl1, ctrl2 } => {
                let from = &self.endpoints[from.to_usize()];
                let to = &self.endpoints[to.to_usize()];
                let mut result = Endpoint::interpolate(from, to, t);
                result.set_position(
                    CubicBezierSegment {
                        from: to.position(),
                        to: to.position(),
                        ctrl1: self.ctrl_points[ctrl1.to_usize()].position(),
                        ctrl2: self.ctrl_points[ctrl2.to_usize()].position(),
                    }.sample(t)
                );

                result
            }
            PathEvent::Begin { at } => { self.endpoints[at.to_usize()].clone() }
            PathEvent::End { last, first, .. } => Endpoint::interpolate(
                &self.endpoints[last.to_usize()],
                &self.endpoints[first.to_usize()],
                t
            ),
        }
    }
}

impl<'l, Endpoint, CtrlPoint> std::ops::Index<EndpointId> for PathSlice<'l, Endpoint, CtrlPoint> {
    type Output = Endpoint;
    fn index(&self, id: EndpointId) -> &Endpoint {
        &self.endpoints[id.to_usize()]
    }
}

impl<'l, Endpoint, CtrlPoint> std::ops::Index<CtrlPointId> for PathSlice<'l, Endpoint, CtrlPoint> {
    type Output = CtrlPoint;
    fn index(&self, id: CtrlPointId) -> &CtrlPoint {
        &self.ctrl_points[id.to_usize()]
    }
}

#[derive(Clone)]
pub struct PathCommandsBuilder {
    path: PathCommands,
    last_cmd: Verb,
    start: u32,
    first_event_index: u32,
}

impl PathCommandsBuilder {
    pub fn new() -> Self {
        Self {
            start: 0,
            path: PathCommands::new(),
            last_cmd: Verb::End,
            first_event_index: 0,
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            start: 0,
            path: PathCommands::with_capacity(cap),
            last_cmd: Verb::End,
            first_event_index: 0,
        }
    }

    pub fn from_commands(path: PathCommands) -> Self {
        Self {
            first_event_index: path.cmds.len() as u32,
            start: path.cmds.len() as u32,
            path,
            last_cmd: Verb::End,
        }
    }

    pub fn move_to(&mut self, to: EndpointId) {
        self.end_if_needed();
        self.first_event_index = self.path.cmds.len() as u32;
        self.path.cmds.push(Cmd::BEGIN.to_u32());
        self.path.cmds.push(to.0 as PathOp);
        self.last_cmd = Verb::Begin;
    }

    pub fn line_to(&mut self, to: EndpointId) {
        self.path.cmds.push(Cmd::LINE.to_u32());
        self.path.cmds.push(to.0 as PathOp);
        self.last_cmd = Verb::LineTo;
    }

    pub fn quadratic_bezier_to(&mut self, ctrl: CtrlPointId, to: EndpointId) {
        self.path.cmds.push(Cmd::QUAD.to_u32());
        self.path.cmds.push(ctrl.0 as PathOp);
        self.path.cmds.push(to.0 as PathOp);
        self.last_cmd = Verb::QuadraticTo;
    }

    pub fn cubic_bezier_to(&mut self, ctrl1: CtrlPointId, ctrl2: CtrlPointId, to: EndpointId) {
        self.path.cmds.push(Cmd::CUBIC.to_u32());
        self.path.cmds.push(ctrl1.0 as PathOp);
        self.path.cmds.push(ctrl2.0 as PathOp);
        self.path.cmds.push(to.0 as PathOp);
        self.last_cmd = Verb::CubicTo;
    }

    pub fn close(&mut self) {
        if (self.last_cmd as u8) >= (Verb::Close as u8) {
            return;
        }
        self.path.cmds.push(Cmd::CLOSE.to_u32());
        self.path.cmds.push(self.first_event_index);
        self.last_cmd = Verb::Close;
    }

    pub fn end_if_needed(&mut self) {
        if (self.last_cmd as u8) <= (Verb::Begin as u8) {
            self.path.cmds.push(Cmd::END.to_u32());
            self.path.cmds.push(self.first_event_index);
        }
    }

    pub fn build(mut self) -> PathCommands {
        self.end_if_needed();
        self.path
    }

    fn finish(&mut self) -> PathId {
        self.end_if_needed();

        PathId {
            start: self.start,
            end: self.path.cmds.len() as u32,
        }
    }
}

pub struct PathCommandsWriter<'l> {
    builder: PathCommandsBuilder,
    storage: &'l mut PathCommands,
}

impl<'l> PathCommandsWriter<'l> {
    pub fn new<'b>(storage: &'b mut PathCommands) -> Self  where 'b : 'l {
        PathCommandsWriter {
            builder: PathCommandsBuilder::from_commands(
                mem::replace(storage, PathCommands::new()),
            ),
            storage,
        }
    }
}

impl<'l> Drop for PathCommandsWriter<'l> {
    fn drop(&mut self) {
        let mut cmds = mem::replace(&mut self.builder.path, PathCommands::new());
        mem::swap(&mut cmds, &mut self.storage);
    }
}

impl<'l> std::ops::Deref for PathCommandsWriter<'l> {
    type Target = PathCommandsBuilder;
    fn deref(&self) -> &PathCommandsBuilder {
        &self.builder
    }
}

#[derive(Clone)]
pub struct IdIter<'l> {
    cmds: std::slice::Iter<'l, PathOp>,
    prev_endpoint: EndpointId,
    first_endpoint: EndpointId,
}

impl<'l> Iterator for IdIter<'l> {
    type Item = IdEvent;

    fn next(&mut self) -> Option<IdEvent> {
        let cmd: Option<&Cmd> = unsafe { mem::transmute(self.cmds.next()) };
        return match cmd {
            Some(&Cmd { op: Verb::Begin, .. }) => {
                let to = EndpointId(*self.cmds.next().unwrap() as u32);
                self.prev_endpoint = to;
                self.first_endpoint = to;
                Some(PathEvent::Begin { at: to })
            }
            Some(&Cmd { op: Verb::LineTo, .. }) => {
                let to = EndpointId(*self.cmds.next().unwrap() as u32);
                let from = self.prev_endpoint;
                self.prev_endpoint = to;
                Some(PathEvent::Line { from, to })
            }
            Some(&Cmd { op: Verb::QuadraticTo, .. }) => {
                let ctrl = CtrlPointId(*self.cmds.next().unwrap() as u32);
                let to = EndpointId(*self.cmds.next().unwrap() as u32);
                let from = self.prev_endpoint;
                self.prev_endpoint = to;
                Some(PathEvent::Quadratic { from, ctrl, to })
            }
            Some(&Cmd { op: Verb::CubicTo, .. }) => {
                let ctrl1 = CtrlPointId(*self.cmds.next().unwrap() as u32);
                let ctrl2 = CtrlPointId(*self.cmds.next().unwrap() as u32);
                let to = EndpointId(*self.cmds.next().unwrap() as u32);
                let from = self.prev_endpoint;
                self.prev_endpoint = to;
                Some(PathEvent::Cubic { from, ctrl1, ctrl2, to })
            }
            Some(&Cmd { op: Verb::End, .. }) => {
                let _first_index = self.cmds.next();
                let last = self.prev_endpoint;
                let first = self.first_endpoint;
                self.prev_endpoint = first;
                Some(PathEvent::End { last, first, close: false })
            }
            Some(&Cmd { op: Verb::Close, .. }) => {
                let _first_index = self.cmds.next();
                let last = self.prev_endpoint;
                let first = self.first_endpoint;
                self.prev_endpoint = first;
                Some(PathEvent::End { last, first, close: true })
            }
            _ => None,
        };
    }
}

#[derive(Clone)]
pub struct RefIter<'l, Endpoint, CtrlPoint> {
    cmds: std::slice::Iter<'l, PathOp>,
    prev_endpoint: usize,
    first_endpoint: usize,
    endpoints: &'l[Endpoint],
    ctrl_points: &'l[CtrlPoint],
}

impl<'l, Endpoint, CtrlPoint> Iterator for RefIter<'l, Endpoint, CtrlPoint> {
    type Item = PathEvent<&'l Endpoint, &'l CtrlPoint>;

    fn next(&mut self) -> Option<PathEvent<&'l Endpoint, &'l CtrlPoint>> {
        let cmd: Option<&Cmd> = unsafe { mem::transmute(self.cmds.next()) };
        return match cmd {
            Some(&Cmd { op: Verb::Begin, .. }) => {
                let to = *self.cmds.next().unwrap() as usize;
                self.prev_endpoint = to;
                self.first_endpoint = to;
                Some(PathEvent::Begin {
                    at: &self.endpoints[to],
                })
            }
            Some(&Cmd { op: Verb::LineTo, .. }) => {
                let to = *self.cmds.next().unwrap() as usize;
                let from = self.prev_endpoint;
                self.prev_endpoint = to;
                Some(PathEvent::Line {
                    from: &self.endpoints[from],
                    to: &self.endpoints[to],
                })
            }
            Some(&Cmd { op: Verb::QuadraticTo, .. }) => {
                let ctrl = *self.cmds.next().unwrap() as usize;
                let to = *self.cmds.next().unwrap() as usize;
                let from = self.prev_endpoint;
                self.prev_endpoint = to;
                Some(PathEvent::Quadratic {
                    from: &self.endpoints[from],
                    ctrl: &self.ctrl_points[ctrl],
                    to: &self.endpoints[to],
                })
            }
            Some(&Cmd { op: Verb::CubicTo, .. }) => {
                let ctrl1 = *self.cmds.next().unwrap() as usize;
                let ctrl2 = *self.cmds.next().unwrap() as usize;
                let to = *self.cmds.next().unwrap() as usize;
                let from = self.prev_endpoint;
                self.prev_endpoint = to;
                Some(PathEvent::Cubic {
                    from: &self.endpoints[from],
                    ctrl1: &self.ctrl_points[ctrl1],
                    ctrl2: &self.ctrl_points[ctrl2],
                    to: &self.endpoints[to],
                })
            }
            Some(&Cmd { op: Verb::End, .. }) => {
                let _first_index = self.cmds.next();
                let last = self.prev_endpoint;
                let first = self.first_endpoint;
                self.prev_endpoint = first;
                Some(PathEvent::End {
                    last: &self.endpoints[last],
                    first: &self.endpoints[first],
                    close: false,
                })
            }
            Some(&Cmd { op: Verb::Close, .. }) => {
                let _first_index = self.cmds.next();
                let last = self.prev_endpoint;
                let first = self.first_endpoint;
                self.prev_endpoint = first;
                Some(PathEvent::End {
                    last: &self.endpoints[last],
                    first: &self.endpoints[first],
                    close: true,
                })
            }
            _ => None,
        };
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct PathId {
    start: u32,
    end: u32,
}

impl PathId {
    fn range(&self) -> std::ops::Range<usize> {
        (self.start as usize) .. (self.end as usize)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct PathBuffer<Endpoint, CtrlPoint> {
    endpoints: Vec<Endpoint>,
    ctrl_points: Vec<CtrlPoint>,
    cmds: PathCommands,
}

impl<Endpoint, CtrlPoint> PathBuffer<Endpoint, CtrlPoint> {
    pub fn new() -> Self {
        PathBuffer {
            endpoints: Vec::new(),
            ctrl_points: Vec::new(),
            cmds: PathCommands::new(),
        }
    }

    pub fn with_capacity(endpoints: usize, ctrl_points: usize, edges: usize) -> Self {
        PathBuffer {
            endpoints: Vec::with_capacity(endpoints),
            ctrl_points: Vec::with_capacity(ctrl_points),
            cmds: PathCommands::with_capacity(edges + endpoints + ctrl_points),
        }
    }

    pub fn iter(&self, path_id: PathId) -> RefIter<Endpoint, CtrlPoint> {
        RefIter {
            cmds: (&self.cmds.cmds[path_id.range()]).iter(),
            first_endpoint: 0,
            prev_endpoint: 0,
            endpoints: &self.endpoints[..],
            ctrl_points: &self.ctrl_points[..],
        }
    }

    pub fn id_iter(&self, path_id: PathId) -> IdIter {
        IdIter {
            cmds: (&self.cmds.cmds[path_id.range()]).iter(),
            first_endpoint: EndpointId(0),
            prev_endpoint: EndpointId(0),
        }
    }

    pub fn slice(&self, path_id: PathId) -> PathSlice<Endpoint, CtrlPoint> {
        PathSlice {
            endpoints: &self.endpoints[..],
            ctrl_points: &self.ctrl_points[..],
            cmds: self.cmds.slice(path_id),
        }
    }

    pub fn id_slice(&self, path_id: PathId) -> PathCommandsSlice {
        self.cmds.slice(path_id)
    }

    pub fn into_path(self) -> Path<Endpoint, CtrlPoint> {
        Path {
            endpoints: self.endpoints.into_boxed_slice(),
            ctrl_points: self.ctrl_points.into_boxed_slice(),
            cmds: self.cmds,
        }
    }

    pub fn write<'a, 'b : 'a>(&'b mut self) -> PathWriter<'a, Endpoint, CtrlPoint> {
        PathWriter::new(self)
    }
}

#[derive(Clone)]
pub struct PathBuilder<Endpoint, CtrlPoint> {
    endpoints: Vec<Endpoint>,
    ctrl_points: Vec<CtrlPoint>,
    cmds: PathCommandsBuilder,
}

impl<'l, Endpoint, CtrlPoint> PathBuilder<Endpoint, CtrlPoint> {
    pub fn new() -> Self {
        Self {
            endpoints: Vec::new(),
            ctrl_points: Vec::new(),
            cmds: PathCommandsBuilder::new(),
        }
    }

    pub fn with_capacity(endpoints: usize, ctrl_points: usize, edges: usize) -> Self {
        Self {
            endpoints: Vec::with_capacity(endpoints),
            ctrl_points: Vec::with_capacity(ctrl_points),
            cmds: PathCommandsBuilder::with_capacity(edges + endpoints + ctrl_points),
        }
    }

    pub fn move_to(&mut self, to: Endpoint) {
        let id = self.add_endpoint(to);
        self.cmds.move_to(id);
    }

    pub fn line_to(&mut self, to: Endpoint) {
        let id = self.add_endpoint(to);
        self.cmds.line_to(id);
    }

    pub fn quadratic_bezier_to(&mut self, ctrl: CtrlPoint, to: Endpoint) {
        let ctrl = self.add_ctrl_point(ctrl);
        let to = self.add_endpoint(to);
        self.cmds.quadratic_bezier_to(ctrl, to);
    }

    pub fn cubic_bezier_to(&mut self, ctrl1: CtrlPoint, ctrl2: CtrlPoint, to: Endpoint) {
        let ctrl1 = self.add_ctrl_point(ctrl1);
        let ctrl2 = self.add_ctrl_point(ctrl2);
        let to = self.add_endpoint(to);
        self.cmds.cubic_bezier_to(ctrl1, ctrl2, to);
    }

    pub fn close(&mut self) {
        self.cmds.close();
    }

    pub fn build(self) -> Path<Endpoint, CtrlPoint> {
        Path {
            endpoints: self.endpoints.into_boxed_slice(),
            ctrl_points: self.ctrl_points.into_boxed_slice(),
            cmds: self.cmds.build(),
        }
    }

    fn add_endpoint(&mut self, ep: Endpoint) -> EndpointId {
        let id = EndpointId(self.endpoints.len() as u32);
        self.endpoints.push(ep);
        id
    }

    fn add_ctrl_point(&mut self, cp: CtrlPoint) -> CtrlPointId {
        let id = CtrlPointId(self.ctrl_points.len() as u32);
        self.ctrl_points.push(cp);
        id
    }
}

pub struct PathWriter<'l, Endpoint, CtrlPoint> {
    builder: PathBuilder<Endpoint, CtrlPoint>,
    buffer: &'l mut PathBuffer<Endpoint, CtrlPoint>,
}

impl<'l, Endpoint, CtrlPoint> PathWriter<'l, Endpoint, CtrlPoint> {
    pub fn new(buffer: &'l mut PathBuffer<Endpoint, CtrlPoint>) -> Self {
        let builder = PathBuilder {
            endpoints: mem::replace(&mut buffer.endpoints, Vec::new()),
            ctrl_points: mem::replace(&mut buffer.ctrl_points, Vec::new()),
            cmds: PathCommandsBuilder::from_commands(
                mem::replace(&mut buffer.cmds, PathCommands::new()),
            ),
        };

        PathWriter {
            builder,
            buffer,
        }
    }

    pub fn build(mut self) -> PathId {
        self.builder.cmds.finish()
    }
}

impl<'l, Endpoint, CtrlPoint> Drop for PathWriter<'l, Endpoint, CtrlPoint> {
    fn drop(&mut self) {
        let mut buffer = PathBuffer {
            endpoints: mem::replace(&mut self.builder.endpoints, Vec::new()),
            ctrl_points: mem::replace(&mut self.builder.ctrl_points, Vec::new()),
            cmds: mem::replace(&mut self.builder.cmds.path, PathCommands::new()),
        };
        mem::swap(&mut buffer, &mut self.buffer);
    }
}

impl<'l, Endpoint, CtrlPoint> std::ops::Deref for PathWriter<'l, Endpoint, CtrlPoint> {
    type Target = PathBuilder<Endpoint, CtrlPoint>;
    fn deref(&self) -> &PathBuilder<Endpoint, CtrlPoint> {
        &self.builder
    }
}

#[test]
fn simple_path() {
    use crate::math::{point, Point};

    let mut builder = Path::<Point, Point>::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.quadratic_bezier_to(point(2.0, 0.0), point(2.0, 1.0));
    builder.cubic_bezier_to(point(2.0, 2.0), point(1.0, 2.0), point(0.0, 2.0));

    builder.move_to(point(0.0, 10.0));
    builder.line_to(point(1.0, 10.0));
    builder.quadratic_bezier_to(point(2.0, 10.0), point(2.0, 11.0));
    builder.cubic_bezier_to(point(2.0, 12.0), point(1.0, 12.0), point(0.0, 12.0));
    builder.close();

    builder.move_to(point(0.0, 20.0));
    builder.line_to(point(1.0, 20.0));
    builder.quadratic_bezier_to(point(2.0, 20.0), point(2.0, 21.0));
    builder.cubic_bezier_to(point(2.0, 22.0), point(1.0, 22.0), point(0.0, 22.0));

    let path = builder.build();
    let mut iter = path.iter();
    assert_eq!(iter.next(), Some(PathEvent::Begin { at: &point(0.0, 0.0) }));
    assert_eq!(iter.next(), Some(PathEvent::Line { from: &point(0.0, 0.0), to: &point(1.0, 0.0) }));
    assert_eq!(iter.next(), Some(PathEvent::Quadratic { from: &point(1.0, 0.0), ctrl: &point(2.0, 0.0), to: &point(2.0, 1.0) }));
    assert_eq!(iter.next(), Some(PathEvent::Cubic { from: &point(2.0, 1.0), ctrl1: &point(2.0, 2.0), ctrl2: &point(1.0, 2.0), to: &point(0.0, 2.0) }));
    assert_eq!(iter.next(), Some(PathEvent::End { last: &point(0.0, 2.0), first: &point(0.0, 0.0), close: false }));

    assert_eq!(iter.next(), Some(PathEvent::Begin { at: &point(0.0, 10.0) }));
    assert_eq!(iter.next(), Some(PathEvent::Line { from: &point(0.0, 10.0), to: &point(1.0, 10.0) }));
    assert_eq!(iter.next(), Some(PathEvent::Quadratic { from: &point(1.0, 10.0), ctrl: &point(2.0, 10.0), to: &point(2.0, 11.0) }));
    assert_eq!(iter.next(), Some(PathEvent::Cubic { from: &point(2.0, 11.0), ctrl1: &point(2.0, 12.0), ctrl2: &point(1.0, 12.0), to: &point(0.0, 12.0) }));
    assert_eq!(iter.next(), Some(PathEvent::End { last: &point(0.0, 12.0), first: &point(0.0, 10.0), close: true }));

    assert_eq!(iter.next(), Some(PathEvent::Begin { at: &point(0.0, 20.0) }));
    assert_eq!(iter.next(), Some(PathEvent::Line { from: &point(0.0, 20.0), to: &point(1.0, 20.0) }));
    assert_eq!(iter.next(), Some(PathEvent::Quadratic { from: &point(1.0, 20.0), ctrl: &point(2.0, 20.0), to: &point(2.0, 21.0) }));
    assert_eq!(iter.next(), Some(PathEvent::Cubic { from: &point(2.0, 21.0), ctrl1: &point(2.0, 22.0), ctrl2: &point(1.0, 22.0), to: &point(0.0, 22.0) }));
    assert_eq!(iter.next(), Some(PathEvent::End { last: &point(0.0, 22.0), first: &point(0.0, 20.0), close: false }));

    assert_eq!(iter.next(), None);
}

#[test]
fn next_event() {
    let mut builder = PathCommands::builder();
    builder.move_to(EndpointId(0));
    builder.line_to(EndpointId(1));
    builder.quadratic_bezier_to(CtrlPointId(2), EndpointId(3));
    builder.cubic_bezier_to(CtrlPointId(4), CtrlPointId(5), EndpointId(6));

    builder.move_to(EndpointId(10));
    builder.line_to(EndpointId(11));
    builder.quadratic_bezier_to(CtrlPointId(12), EndpointId(13));
    builder.cubic_bezier_to(CtrlPointId(14), CtrlPointId(15), EndpointId(16));
    builder.close();

    builder.move_to(EndpointId(20));
    builder.line_to(EndpointId(21));
    builder.quadratic_bezier_to(CtrlPointId(22), EndpointId(23));
    builder.cubic_bezier_to(CtrlPointId(24), CtrlPointId(25), EndpointId(26));

    let path = builder.build();

    let mut id = PathEventId(0);
    let first = id;
    assert_eq!(path.event(id), PathEvent::Begin { at: EndpointId(0) });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), PathEvent::Line { from: EndpointId(0), to: EndpointId(1) });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), PathEvent::Quadratic { from: EndpointId(1), ctrl: CtrlPointId(2), to: EndpointId(3) });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), PathEvent::Cubic { from: EndpointId(3), ctrl1: CtrlPointId(4), ctrl2: CtrlPointId(5), to: EndpointId(6) });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), PathEvent::End { last: EndpointId(6), first: EndpointId(0), close: false });

    assert_eq!(path.next_event_id_in_sub_path(id), first);

    id = path.next_event_id_in_path(id).unwrap();
    let first = id;
    assert_eq!(path.event(id), PathEvent::Begin { at: EndpointId(10) });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), PathEvent::Line { from: EndpointId(10), to: EndpointId(11) });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), PathEvent::Quadratic { from: EndpointId(11), ctrl: CtrlPointId(12), to: EndpointId(13) });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), PathEvent::Cubic { from: EndpointId(13), ctrl1: CtrlPointId(14), ctrl2: CtrlPointId(15), to: EndpointId(16) });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), PathEvent::End { last: EndpointId(16), first: EndpointId(10), close: true });

    assert_eq!(path.next_event_id_in_sub_path(id), first);

    id = path.next_event_id_in_path(id).unwrap();
    let first = id;
    assert_eq!(path.event(id), PathEvent::Begin { at: EndpointId(20) });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), PathEvent::Line { from: EndpointId(20), to: EndpointId(21) });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), PathEvent::Quadratic { from: EndpointId(21), ctrl: CtrlPointId(22), to: EndpointId(23) });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), PathEvent::Cubic { from: EndpointId(23), ctrl1: CtrlPointId(24), ctrl2: CtrlPointId(25), to: EndpointId(26) });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), PathEvent::End { last: EndpointId(26), first: EndpointId(20), close: false });

    assert_eq!(path.next_event_id_in_path(id), None);
    assert_eq!(path.next_event_id_in_sub_path(id), first);
}
