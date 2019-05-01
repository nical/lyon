use crate::{EndpointId, CtrlPointId};
use crate::events::{PathEvent, IdEvent};

use std::mem;
use std::fmt;

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
    Line = 0,
    Quadratic = 1,
    Cubic = 2,
    Begin = 3,
    Close = 4,
    End = 5,
}

#[derive(Copy, Clone)]
union PathOp {
    verb: Verb,
    endpoint: EndpointId,
    ctrl_point: CtrlPointId,
    offset: u32,
}

#[derive(Clone)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct PathCommands {
    cmds: Box<[PathOp]>,
}

impl PathCommands {
    pub fn builder() -> PathCommandsBuilder {
        PathCommandsBuilder::new()
    }

    pub fn iter(&self) -> IdIter {
        IdIter {
            cmds: self.cmds.iter(),
            first_endpoint: EndpointId(0),
            prev_endpoint: EndpointId(0),
        }
    }

    pub fn as_slice(&self) -> PathCommandsSlice {
        PathCommandsSlice {
            cmds: &self.cmds,
        }
    }

    pub fn path_slice<'l, Endpoint, CtrlPoint>(
        &'l self,
        endpoints: &'l [Endpoint],
        ctrl_points: &'l [CtrlPoint],
    ) -> IdPathSlice<Endpoint, CtrlPoint> {
        IdPathSlice {
            endpoints,
            ctrl_points,
            cmds: self.as_slice(),
        }
    }

    pub fn path_iter<'l, Endpoint, CtrlPoint>(
        &'l self,
        endpoints: &'l [Endpoint],
        ctrl_points: &'l [CtrlPoint],
    ) -> RefIter<Endpoint, CtrlPoint> {
        RefIter {
            cmds: self.cmds.iter(),
            first_endpoint: 0,
            prev_endpoint: 0,
            endpoints,
            ctrl_points,
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
}

impl fmt::Debug for PathCommands {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_slice().fmt(f)
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct PathCommandsBuffer {
    cmds: Vec<PathOp>,
}

impl PathCommandsBuffer {
    pub fn new() -> Self {
        PathCommandsBuffer {
            cmds: Vec::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        PathCommandsBuffer {
            cmds: Vec::with_capacity(cap),
        }
    }

    pub fn write<'a, 'b : 'a>(&'b mut self) -> PathCommandsWriter<'a> {
        PathCommandsWriter::new(self)
    }

    pub fn iter_all(&self) -> IdIter {
        IdIter {
            cmds: self.cmds.iter(),
            first_endpoint: EndpointId(0),
            prev_endpoint: EndpointId(0),
        }
    }

    pub fn iter(&self, path_id: PathId) -> IdIter {
        IdIter {
            cmds: (&self.cmds[path_id.range()]).iter(),
            first_endpoint: EndpointId(0),
            prev_endpoint: EndpointId(0),
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

    pub fn into_path_commands(self) -> PathCommands {
        PathCommands {
            cmds: self.cmds.into_boxed_slice()
        }
    }

    fn as_slice(&self) -> PathCommandsSlice {
        PathCommandsSlice {
            cmds: &self.cmds,
        }
    }
}

impl fmt::Debug for PathCommandsBuffer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl<'l> IntoIterator for &'l PathCommands {
    type Item = PathEvent<EndpointId, CtrlPointId>;
    type IntoIter = IdIter<'l>;

    fn into_iter(self) -> IdIter<'l> { self.iter() }
}

impl<'l> Into<PathCommandsSlice<'l>> for &'l PathCommands {
    fn into(self) -> PathCommandsSlice<'l> {
        self.as_slice()
    }
}

#[derive(Copy, Clone)]
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
        unsafe {
            match self.cmds[idx].verb {
                Verb::Line => PathEvent::Line {
                    from: self.cmds[idx - 1].endpoint,
                    to: self.cmds[idx + 1].endpoint,
                },
                Verb::Quadratic => PathEvent::Quadratic {
                    from: self.cmds[idx - 1].endpoint,
                    ctrl: self.cmds[idx + 1].ctrl_point,
                    to: self.cmds[idx + 2].endpoint,
                },
                Verb::Cubic => PathEvent::Cubic {
                    from: self.cmds[idx - 1].endpoint,
                    ctrl1: self.cmds[idx + 1].ctrl_point,
                    ctrl2: self.cmds[idx + 2].ctrl_point,
                    to: self.cmds[idx + 3].endpoint,
                },
                Verb::Begin => PathEvent::Begin {
                    at: self.cmds[idx + 1].endpoint
                },
                Verb::End => {
                    let first_event = self.cmds[idx + 1].offset as usize;
                    PathEvent::End {
                        last: self.cmds[idx - 1].endpoint,
                        first: self.cmds[first_event + 1].endpoint,
                        close: false,
                    }
                }
                Verb::Close => {
                    let first_event = self.cmds[idx + 1].offset as usize;
                    PathEvent::End {
                        last: self.cmds[idx - 1].endpoint,
                        first: self.cmds[first_event + 1].endpoint,
                        close: true,
                    }
                }
            }
        }
    }

    pub fn next_event_id_in_sub_path(&self, id: PathEventId) -> PathEventId {
        let idx = id.to_usize();
        let cmd = unsafe { self.cmds[idx].verb };
        match cmd {
            Verb::Line | Verb::Begin => PathEventId(id.0 + 2),
            Verb::Quadratic => PathEventId(id.0 + 3),
            Verb::Cubic => PathEventId(id.0 + 4),
            Verb::End | Verb::Close => PathEventId(unsafe { self.cmds[idx + 1].offset }),
        }
    }

    pub fn next_event_id_in_path(&self, id: PathEventId) -> Option<PathEventId> {
        let idx = id.to_usize();
        let next = match unsafe { self.cmds[idx].verb } {
            Verb::Line
            | Verb::Begin
            | Verb::End
            | Verb::Close => PathEventId(id.0 + 2),
            Verb::Quadratic => PathEventId(id.0 + 3),
            Verb::Cubic => PathEventId(id.0 + 4),
        };

        if next.0 < self.cmds.len() as u32 {
            return Some(next);
        }

        None
    }
}

impl<'l> fmt::Debug for PathCommandsSlice<'l> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{ ")?;
        for evt in self.iter() {
            match evt {
                PathEvent::Line { to, .. } => { write!(f, "L {:?}", to) }
                PathEvent::Quadratic { ctrl,  to, .. } => { write!(f, "Q {:?} {:?} ", ctrl, to) }
                PathEvent::Cubic { ctrl1, ctrl2, to, .. } => { write!(f, "C {:?} {:?} {:?} ", ctrl1, ctrl2, to) }
                PathEvent::Begin { at, .. } => { write!(f, "M {:?} ", at) }
                PathEvent::End { close: true, .. } => { write!(f, "Z ") }
                PathEvent::End { close: false, .. } => { Ok(()) }
            }?;
        }
        write!(f, "}}")
    }
}

/// A view on a `Path`.
#[derive(Copy, Clone)]
pub struct IdPathSlice<'l, Endpoint, CtrlPoint> {
    endpoints: &'l [Endpoint],
    ctrl_points: &'l [CtrlPoint],
    cmds: PathCommandsSlice<'l>,
}

impl<'l, Endpoint, CtrlPoint> IdPathSlice<'l, Endpoint, CtrlPoint> {
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
}

impl<'l, Endpoint, CtrlPoint> std::ops::Index<EndpointId> for IdPathSlice<'l, Endpoint, CtrlPoint> {
    type Output = Endpoint;
    fn index(&self, id: EndpointId) -> &Endpoint {
        &self.endpoints[id.to_usize()]
    }
}

impl<'l, Endpoint, CtrlPoint> std::ops::Index<CtrlPointId> for IdPathSlice<'l, Endpoint, CtrlPoint> {
    type Output = CtrlPoint;
    fn index(&self, id: CtrlPointId) -> &CtrlPoint {
        &self.ctrl_points[id.to_usize()]
    }
}

impl<'l, Endpoint, CtrlPoint> fmt::Debug for IdPathSlice<'l, Endpoint, CtrlPoint>
where
    Endpoint: fmt::Debug,
    CtrlPoint: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{ ")?;
        for evt in self.iter() {
            match evt {
                PathEvent::Line { to, .. } => { write!(f, "L {:?}", to) }
                PathEvent::Quadratic { ctrl,  to, .. } => { write!(f, "Q {:?} {:?} ", ctrl, to) }
                PathEvent::Cubic { ctrl1, ctrl2, to, .. } => { write!(f, "C {:?} {:?} {:?} ", ctrl1, ctrl2, to) }
                PathEvent::Begin { at, .. } => { write!(f, "M {:?} ", at) }
                PathEvent::End { close: true, .. } => { write!(f, "Z ") }
                PathEvent::End { close: false, .. } => { Ok(()) }
            }?;
        }
        write!(f, "}}")
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
        unsafe {
            match self.cmds.next() {
                Some(&PathOp { verb: Verb::Begin }) => {
                    let to = self.cmds.next().unwrap().endpoint;
                    self.prev_endpoint = to;
                    self.first_endpoint = to;
                    Some(PathEvent::Begin { at: to })
                }
                Some(&PathOp { verb: Verb::Line }) => {
                    let to = self.cmds.next().unwrap().endpoint;
                    let from = self.prev_endpoint;
                    self.prev_endpoint = to;
                    Some(PathEvent::Line { from, to })
                }
                Some(&PathOp { verb: Verb::Quadratic }) => {
                    let ctrl = self.cmds.next().unwrap().ctrl_point;
                    let to = self.cmds.next().unwrap().endpoint;
                    let from = self.prev_endpoint;
                    self.prev_endpoint = to;
                    Some(PathEvent::Quadratic { from, ctrl, to })
                }
                Some(&PathOp { verb: Verb::Cubic }) => {
                    let ctrl1 = self.cmds.next().unwrap().ctrl_point;
                    let ctrl2 = self.cmds.next().unwrap().ctrl_point;
                    let to = self.cmds.next().unwrap().endpoint;
                    let from = self.prev_endpoint;
                    self.prev_endpoint = to;
                    Some(PathEvent::Cubic { from, ctrl1, ctrl2, to })
                }
                Some(&PathOp { verb: Verb::End }) => {
                    let _first_index = self.cmds.next();
                    let last = self.prev_endpoint;
                    let first = self.first_endpoint;
                    self.prev_endpoint = first;
                    Some(PathEvent::End { last, first, close: false })
                }
                Some(&PathOp { verb: Verb::Close }) => {
                    let _first_index = self.cmds.next();
                    let last = self.prev_endpoint;
                    let first = self.first_endpoint;
                    self.prev_endpoint = first;
                    Some(PathEvent::End { last, first, close: true })
                }
                _ => None,
            }
        }
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
        unsafe {
            match self.cmds.next() {
                Some(&PathOp { verb: Verb::Begin }) => {
                    let to = self.cmds.next().unwrap().offset as usize;
                    self.prev_endpoint = to;
                    self.first_endpoint = to;
                    Some(PathEvent::Begin {
                        at: &self.endpoints[to],
                    })
                }
                Some(&PathOp { verb: Verb::Line }) => {
                    let to = self.cmds.next().unwrap().offset as usize;
                    let from = self.prev_endpoint;
                    self.prev_endpoint = to;
                    Some(PathEvent::Line {
                        from: &self.endpoints[from],
                        to: &self.endpoints[to],
                    })
                }
                Some(&PathOp { verb: Verb::Quadratic }) => {
                    let ctrl = self.cmds.next().unwrap().offset as usize;
                    let to = self.cmds.next().unwrap().offset as usize;
                    let from = self.prev_endpoint;
                    self.prev_endpoint = to;
                    Some(PathEvent::Quadratic {
                        from: &self.endpoints[from],
                        ctrl: &self.ctrl_points[ctrl],
                        to: &self.endpoints[to],
                    })
                }
                Some(&PathOp { verb: Verb::Cubic }) => {
                    let ctrl1 = self.cmds.next().unwrap().offset as usize;
                    let ctrl2 = self.cmds.next().unwrap().offset as usize;
                    let to = self.cmds.next().unwrap().offset as usize;
                    let from = self.prev_endpoint;
                    self.prev_endpoint = to;
                    Some(PathEvent::Cubic {
                        from: &self.endpoints[from],
                        ctrl1: &self.ctrl_points[ctrl1],
                        ctrl2: &self.ctrl_points[ctrl2],
                        to: &self.endpoints[to],
                    })
                }
                Some(&PathOp { verb: Verb::End }) => {
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
                Some(&PathOp { verb: Verb::Close }) => {
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
            }
        }
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

#[derive(Clone)]
pub struct PathCommandsBuilder {
    path: PathCommandsBuffer,
    last_cmd: Verb,
    start: u32,
    first_event_index: u32,
}

impl PathCommandsBuilder {
    pub fn new() -> Self {
        Self {
            start: 0,
            path: PathCommandsBuffer::new(),
            last_cmd: Verb::End,
            first_event_index: 0,
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            start: 0,
            path: PathCommandsBuffer::with_capacity(cap),
            last_cmd: Verb::End,
            first_event_index: 0,
        }
    }

    pub fn from_commands(path: PathCommandsBuffer) -> Self {
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
        self.path.cmds.push(PathOp { verb: Verb::Begin });
        self.path.cmds.push(PathOp { endpoint: to });
        self.last_cmd = Verb::Begin;
    }

    pub fn line_to(&mut self, to: EndpointId) {
        self.begin_if_needed();
        self.path.cmds.push(PathOp { verb: Verb::Line });
        self.path.cmds.push(PathOp { endpoint: to });
        self.last_cmd = Verb::Line;
    }

    pub fn quadratic_bezier_to(&mut self, ctrl: CtrlPointId, to: EndpointId) {
        self.begin_if_needed();
        self.path.cmds.push(PathOp { verb: Verb::Quadratic });
        self.path.cmds.push(PathOp { ctrl_point: ctrl });
        self.path.cmds.push(PathOp { endpoint: to });
        self.last_cmd = Verb::Quadratic;
    }

    pub fn cubic_bezier_to(&mut self, ctrl1: CtrlPointId, ctrl2: CtrlPointId, to: EndpointId) {
        self.begin_if_needed();
        self.path.cmds.push(PathOp { verb: Verb::Cubic });
        self.path.cmds.push(PathOp { ctrl_point: ctrl1 });
        self.path.cmds.push(PathOp { ctrl_point: ctrl2 });
        self.path.cmds.push(PathOp { endpoint: to });
        self.last_cmd = Verb::Cubic;
    }

    pub fn close(&mut self) {
        match self.last_cmd {
            Verb::Close | Verb::End => {
                return;
            }
            _ => {}
        }
        self.path.cmds.push(PathOp { verb: Verb::Close });
        self.path.cmds.push(PathOp { offset: self.first_event_index });
        self.last_cmd = Verb::Close;
    }

    fn begin_if_needed(&mut self) {
        match self.last_cmd {
            Verb::Close | Verb::End => {
                let first = self.path.cmds.last().cloned().unwrap_or(PathOp { offset: 0 });
                self.move_to(unsafe { first.endpoint });
            }
            _ => {}
        }
    }

    fn end_if_needed(&mut self) {
        match self.last_cmd {
            Verb::Line | Verb::Quadratic | Verb::Cubic => {
                self.path.cmds.push(PathOp { verb: Verb::End });
                self.path.cmds.push(PathOp { offset: self.first_event_index });
            }
            _ => {}
        }
    }

    pub fn build(mut self) -> PathCommands {
        self.end_if_needed();
        self.path.into_path_commands()
    }
}

pub struct PathCommandsWriter<'l> {
    builder: PathCommandsBuilder,
    storage: &'l mut PathCommandsBuffer,
}

impl<'l> PathCommandsWriter<'l> {
    pub fn new<'b>(storage: &'b mut PathCommandsBuffer) -> Self  where 'b : 'l {
        PathCommandsWriter {
            builder: PathCommandsBuilder::from_commands(
                mem::replace(storage, PathCommandsBuffer::new()),
            ),
            storage,
        }
    }
}

impl<'l> Drop for PathCommandsWriter<'l> {
    fn drop(&mut self) {
        let mut cmds = mem::replace(&mut self.builder.path, PathCommandsBuffer::new());
        mem::swap(&mut cmds, &mut self.storage);
    }
}

impl<'l> std::ops::Deref for PathCommandsWriter<'l> {
    type Target = PathCommandsBuilder;
    fn deref(&self) -> &PathCommandsBuilder {
        &self.builder
    }
}

pub struct IdPathBuilder<'l, Endpoint, CtrlPoint> {
    endpoints: &'l mut Vec<Endpoint>,
    ctrl_points: &'l mut Vec<CtrlPoint>,
    cmds: PathCommandsBuilder,
}

impl<'l, Endpoint, CtrlPoint> IdPathBuilder<'l, Endpoint, CtrlPoint> {
    pub fn new(endpoints: &'l mut Vec<Endpoint>, ctrl_points: &'l mut Vec<CtrlPoint>) -> Self {
        Self {
            endpoints,
            ctrl_points,
            cmds: PathCommandsBuilder::new(),
        }
    }

    pub fn with_capacity(
        n_endpoints: usize,
        n_ctrl_points: usize,
        n_edges: usize,
        endpoints: &'l mut Vec<Endpoint>,
        ctrl_points: &'l mut Vec<CtrlPoint>,
        ) -> Self {
        endpoints.reserve(n_endpoints);
        ctrl_points.reserve(n_ctrl_points);
        Self {
            endpoints,
            ctrl_points,
            cmds: PathCommandsBuilder::with_capacity(n_edges + n_endpoints + n_ctrl_points),
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

    pub fn build(self) -> PathCommands {
        self.cmds.build()
    }

    #[inline]
    fn add_endpoint(&mut self, ep: Endpoint) -> EndpointId {
        let id = EndpointId(self.endpoints.len() as u32);
        self.endpoints.push(ep);
        id
    }

    #[inline]
    fn add_ctrl_point(&mut self, cp: CtrlPoint) -> CtrlPointId {
        let id = CtrlPointId(self.ctrl_points.len() as u32);
        self.ctrl_points.push(cp);
        id
    }
}

#[test]
fn simple_path() {
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
    let mut iter = path.iter();
    assert_eq!(iter.next(), Some(PathEvent::Begin { at: EndpointId(0) }));
    assert_eq!(iter.next(), Some(PathEvent::Line { from: EndpointId(0), to: EndpointId(1) }));
    assert_eq!(iter.next(), Some(PathEvent::Quadratic { from: EndpointId(1), ctrl: CtrlPointId(2), to: EndpointId(3) }));
    assert_eq!(iter.next(), Some(PathEvent::Cubic { from: EndpointId(3), ctrl1: CtrlPointId(4), ctrl2: CtrlPointId(5), to: EndpointId(6) }));
    assert_eq!(iter.next(), Some(PathEvent::End { last: EndpointId(6), first: EndpointId(0), close: false }));

    assert_eq!(iter.next(), Some(PathEvent::Begin { at: EndpointId(10) }));
    assert_eq!(iter.next(), Some(PathEvent::Line { from: EndpointId(10), to: EndpointId(11) }));
    assert_eq!(iter.next(), Some(PathEvent::Quadratic { from: EndpointId(11), ctrl: CtrlPointId(12), to: EndpointId(13) }));
    assert_eq!(iter.next(), Some(PathEvent::Cubic { from: EndpointId(13), ctrl1: CtrlPointId(14), ctrl2: CtrlPointId(15), to: EndpointId(16) }));
    assert_eq!(iter.next(), Some(PathEvent::End { last: EndpointId(16), first: EndpointId(10), close: true }));

    assert_eq!(iter.next(), Some(PathEvent::Begin { at: EndpointId(20) }));
    assert_eq!(iter.next(), Some(PathEvent::Line { from: EndpointId(20), to: EndpointId(21) }));
    assert_eq!(iter.next(), Some(PathEvent::Quadratic { from: EndpointId(21), ctrl: CtrlPointId(22), to: EndpointId(23) }));
    assert_eq!(iter.next(), Some(PathEvent::Cubic { from: EndpointId(23), ctrl1: CtrlPointId(24), ctrl2: CtrlPointId(25), to: EndpointId(26) }));
    assert_eq!(iter.next(), Some(PathEvent::End { last: EndpointId(26), first: EndpointId(20), close: false }));

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
