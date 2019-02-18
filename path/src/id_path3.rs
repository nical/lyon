use crate::{EndpointId, CtrlPointId};
use crate::events::{PathEvent, IdEvent};
use crate::geom::math::Point;

use std::mem;
use std::fmt;

/// Refers to an event in an `IdPathSlice` or `PathCommands`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct PathEventId(pub(crate) u32);
impl PathEventId {
    //pub(crate) const INVALID: Self = PathEventId(!0u32);
    pub fn to_usize(&self) -> usize { self.0 as usize }
}

#[repr(u32)]
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

impl PathOp {
    #[inline]
    fn get_verb(self) -> Verb {
        assert_is_verb(self);
        unsafe {
            self.verb
        }
    }
}

#[inline]
fn assert_is_verb(op: PathOp) {
    unsafe { assert!(op.offset < 6) }
}

/// The commands of a path encoded in a single array using IDs to refer
/// to endpoints and control points.
///
/// `PathCommands` is a good fit when the a custom endpoint and control point
/// types are needed or when their the user needs to control their position in
/// the buffers.
#[derive(Clone)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct PathCommands {
    cmds: Box<[PathOp]>,
}

impl PathCommands {
    /// Creates a [PathCommandsBuilder](struct.PathCommandsBuilder.html) to create path commands.
    pub fn builder() -> PathCommandsBuilder {
        PathCommandsBuilder::new()
    }

    /// Returns an iterator over the path commands.
    pub fn iter(&self) -> IdIter {
        IdIter {
            cmds: self.cmds.iter(),
            first_endpoint: EndpointId(0),
            prev_endpoint: EndpointId(0),
        }
    }

    /// Returns a view on the path commands.
    pub fn as_slice(&self) -> PathCommandsSlice {
        PathCommandsSlice {
            cmds: &self.cmds,
        }
    }

    /// Returns a view on a path made of these commands with endpoint and
    /// control point slices.
    pub fn path_slice<'l>(
        &'l self,
        endpoints: &'l [Point],
        ctrl_points: &'l [Point],
    ) -> IdPathSlice {
        IdPathSlice {
            endpoints,
            ctrl_points,
            cmds: self.as_slice(),
        }
    }

    /// Returns an iterator over the path, with endpoints and control points.
    pub fn path_iter<'l>(
        &'l self,
        endpoints: &'l [Point],
        ctrl_points: &'l [Point],
    ) -> RefIter {
        RefIter {
            cmds: self.cmds.iter(),
            first_endpoint: 0,
            prev_endpoint: 0,
            endpoints,
            ctrl_points,
        }
    }

    /// Returns the event for a given event ID.
    pub fn event(&self, id: PathEventId) -> IdEvent {
        self.as_slice().event(id)
    }

    /// Returns the next event id within the path.
    pub fn next_event_id_in_path(&self, id: PathEventId) -> Option<PathEventId> {
        self.as_slice().next_event_id_in_path(id)
    }

    /// Returns the next event id within the sub-path.
    ///
    /// Loops back to the first event after the end of the sub-path.
    pub fn next_event_id_in_sub_path(&self, id: PathEventId) -> PathEventId {
        self.as_slice().next_event_id_in_sub_path(id)
    }
}

impl fmt::Debug for PathCommands {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_slice().fmt(f)
    }
}

/// A growable buffer of path commands.
#[derive(Clone)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct PathCommandsBuffer {
    cmds: Vec<PathOp>,
}

impl PathCommandsBuffer {
    /// Creates a buffer without allocating memory.
    pub fn new() -> Self {
        PathCommandsBuffer {
            cmds: Vec::new(),
        }
    }

    /// Creates a preallocated buffer.
    pub fn with_capacity(cap: usize) -> Self {
        PathCommandsBuffer {
            cmds: Vec::with_capacity(cap),
        }
    }

    /// Returns an object that can push commands into this buffer.
    pub fn write<'a, 'b : 'a>(&'b mut self) -> PathCommandsWriter<'a> {
        PathCommandsWriter::new(self)
    }

    /// Returns an iterator over all path events in the buffer.
    pub fn iter_all(&self) -> IdIter {
        IdIter {
            cmds: self.cmds.iter(),
            first_endpoint: EndpointId(0),
            prev_endpoint: EndpointId(0),
        }
    }

    /// Returns an iterator over the path events associated to a given path ID.
    pub fn iter(&self, path_id: PathId) -> IdIter {
        IdIter {
            cmds: (&self.cmds[path_id.range()]).iter(),
            first_endpoint: EndpointId(0),
            prev_endpoint: EndpointId(0),
        }
    }

    /// Returns a view over the events associated to a given path ID.
    pub fn slice(&self, path_id: PathId) -> PathCommandsSlice {
        PathCommandsSlice {
            cmds: &self.cmds[path_id.range()],
        }
    }

    /// Returns the event for a given event ID.
    pub fn event(&self, id: PathEventId) -> IdEvent {
        self.as_slice().event(id)
    }

    /// Returns the next event id within the path.
    pub fn next_event_id_in_path(&self, id: PathEventId) -> Option<PathEventId> {
        self.as_slice().next_event_id_in_path(id)
    }

    /// Returns the next event id within the sub-path.
    ///
    /// Loops back to the first event after the end of the sub-path.
    pub fn next_event_id_in_sub_path(&self, id: PathEventId) -> PathEventId {
        self.as_slice().next_event_id_in_sub_path(id)
    }

    /// Consumes this buffer and turn it into a `PathCommands` object.
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
    /// Returns an iterator over the path commands.
    pub fn iter(&self) -> IdIter {
        IdIter {
            cmds: self.cmds.iter(),
            first_endpoint: EndpointId(0),
            prev_endpoint: EndpointId(0),
        }
    }

    /// Returns the event for a given event ID.
    pub fn event(&self, id: PathEventId) -> IdEvent {
        let idx = id.to_usize();
        unsafe {
            match self.cmds[idx].get_verb() {
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

    /// Returns the next event id within the path.
    pub fn next_event_id_in_sub_path(&self, id: PathEventId) -> PathEventId {
        let idx = id.to_usize();
        let cmd = self.cmds[idx].get_verb();
        match cmd {
            Verb::Line | Verb::Begin => PathEventId(id.0 + 2),
            Verb::Quadratic => PathEventId(id.0 + 3),
            Verb::Cubic => PathEventId(id.0 + 4),
            Verb::End | Verb::Close => PathEventId(unsafe { self.cmds[idx + 1].offset }),
        }
    }

    /// Returns the next event id within the path.
    pub fn next_event_id_in_path(&self, id: PathEventId) -> Option<PathEventId> {
        let idx = id.to_usize();
        let next = match self.cmds[idx].get_verb() {
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

pub struct IdPath {
    cmds: PathCommands,
    endpoints: Box<[Point]>,
    ctrl_points: Box<[Point]>,
}

impl IdPath {
    /// Creates a [IdPathBuilder](struct.IdPathBuilder.html).
    pub fn builder() -> IdPathBuilder {
        IdPathBuilder::new()
    }

    /// Returns a view on a path made of these commands with endpoint and
    /// control point slices.
    pub fn as_slice(&self) -> IdPathSlice {
        IdPathSlice {
            cmds: self.cmds.as_slice(),
            endpoints: &self.endpoints,
            ctrl_points: &self.ctrl_points,
        }
    }

    /// Returns an iterator over the path commands.
    pub fn id_iter(&self) -> IdIter {
        self.cmds.iter()
    }

    /// Returns an iterator over the path, with endpoints and control points.
    pub fn iter(&self) -> RefIter {
        RefIter {
            cmds: self.cmds.cmds.iter(),
            first_endpoint: 0,
            prev_endpoint: 0,
            endpoints: &self.endpoints,
            ctrl_points: &self.ctrl_points,
        }
    }

    /// Returns the event for a given event ID.
    pub fn id_event(&self, id: PathEventId) -> IdEvent {
        self.cmds.as_slice().event(id)
    }

    /// Returns the event for a given event ID.
    pub fn event(&self, id: PathEventId) -> PathEvent<&Point, &Point> {
        match self.id_event(id) {
            PathEvent::Begin { at } => PathEvent::Begin {
                at: &self[at]
            },
            PathEvent::Line { from, to, } => PathEvent::Line {
                from: &self[from],
                to: &self[to]
            },
            PathEvent::Quadratic { from, ctrl, to } => PathEvent::Quadratic {
                from: &self[from],
                ctrl: &self[ctrl],
                to: &self[to]
            },
            PathEvent::Cubic { from, ctrl1, ctrl2, to } => PathEvent::Cubic {
                from: &self[from],
                ctrl1: &self[ctrl1],
                ctrl2: &self[ctrl2],
                to: &self[to]
            },
            PathEvent::End { last, first, close } => PathEvent::End {
                last: &self[last],  first: &self[first], close
            },
        }
    }

    /// Returns the next event id within the path.
    pub fn next_event_id_in_path(&self, id: PathEventId) -> Option<PathEventId> {
        self.cmds.as_slice().next_event_id_in_path(id)
    }

    /// Returns the next event id within the sub-path.
    ///
    /// Loops back to the first event after the end of the sub-path.
    pub fn next_event_id_in_sub_path(&self, id: PathEventId) -> PathEventId {
        self.cmds.as_slice().next_event_id_in_sub_path(id)
    }

    pub fn endpoints(&self) -> &[Point] { &self.endpoints }

    pub fn ctrl_points(&self) -> &[Point] { &self.ctrl_points }
}

impl std::ops::Index<EndpointId> for IdPath {
    type Output = Point;
    fn index(&self, id: EndpointId) -> &Point {
        &self.endpoints[id.to_usize()]
    }
}

impl std::ops::Index<CtrlPointId> for IdPath {
    type Output = Point;
    fn index(&self, id: CtrlPointId) -> &Point {
        &self.ctrl_points[id.to_usize()]
    }
}

/// A view on a `Path`.
#[derive(Copy, Clone)]
pub struct IdPathSlice<'l> {
    endpoints: &'l [Point],
    ctrl_points: &'l [Point],
    cmds: PathCommandsSlice<'l>,
}

impl<'l> IdPathSlice<'l> {
    /// Returns an iterator over the events of the path using IDs.
    pub fn id_iter(&self) -> IdIter {
        self.cmds.iter()
    }

    /// Returns an iterator over the events of the path using endpoint
    /// and control point references.
    pub fn iter(&self) -> RefIter {
        RefIter {
            cmds: self.cmds.cmds.iter(),
            first_endpoint: 0,
            prev_endpoint: 0,
            endpoints: &self.endpoints[..],
            ctrl_points: &self.ctrl_points[..],
        }
    }
}

impl<'l> std::ops::Index<EndpointId> for IdPathSlice<'l> {
    type Output = Point;
    fn index(&self, id: EndpointId) -> &Point {
        &self.endpoints[id.to_usize()]
    }
}

impl<'l> std::ops::Index<CtrlPointId> for IdPathSlice<'l> {
    type Output = Point;
    fn index(&self, id: CtrlPointId) -> &Point {
        &self.ctrl_points[id.to_usize()]
    }
}

impl<'l> fmt::Debug for IdPathSlice<'l> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{ ")?;
        for evt in self.iter() {
            match evt {
                // TODO: format the points properly
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

/// An iterator of `PathEvent<EndpointId, CtrlPointId>`.
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

/// An iterator of `PathEvent<&Endpoint, &CtrlPoint>`.
#[derive(Clone)]
pub struct RefIter<'l> {
    cmds: std::slice::Iter<'l, PathOp>,
    prev_endpoint: usize,
    first_endpoint: usize,
    endpoints: &'l[Point],
    ctrl_points: &'l[Point],
}

impl<'l> Iterator for RefIter<'l> {
    type Item = PathEvent<&'l Point, &'l Point>;

    fn next(&mut self) -> Option<PathEvent<&'l Point, &'l Point>> {
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

/// Refers to a path within a PathBuffer.
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

/// Builds path commands.
#[derive(Clone)]
pub struct PathCommandsBuilder {
    path: PathCommandsBuffer,
    last_cmd: Verb,
    start: u32,
    first_event_index: u32,
}

impl PathCommandsBuilder {
    /// Creates a builder without allocating memory.
    pub fn new() -> Self {
        Self {
            start: 0,
            path: PathCommandsBuffer::new(),
            last_cmd: Verb::End,
            first_event_index: 0,
        }
    }

    /// Creates a pre-allocated builder.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            start: 0,
            path: PathCommandsBuffer::with_capacity(cap),
            last_cmd: Verb::End,
            first_event_index: 0,
        }
    }

    /// Consumes a `PathCommandsBuffer` and continues building from
    /// the consumed buffer's commands.
    pub fn from_commands(path: PathCommandsBuffer) -> Self {
        Self {
            first_event_index: path.cmds.len() as u32,
            start: path.cmds.len() as u32,
            path,
            last_cmd: Verb::End,
        }
    }

    pub fn move_to(&mut self, to: EndpointId) -> PathEventId {
        self.end_if_needed();
        self.first_event_index = self.path.cmds.len() as u32;
        let id = PathEventId(self.path.cmds.len() as u32);
        self.path.cmds.push(PathOp { verb: Verb::Begin });
        self.path.cmds.push(PathOp { endpoint: to });
        self.last_cmd = Verb::Begin;

        id
    }

    pub fn line_to(&mut self, to: EndpointId) -> PathEventId {
        self.begin_if_needed();
        let id = PathEventId(self.path.cmds.len() as u32);
        self.path.cmds.push(PathOp { verb: Verb::Line });
        self.path.cmds.push(PathOp { endpoint: to });
        self.last_cmd = Verb::Line;

        id
    }

    pub fn quadratic_bezier_to(&mut self, ctrl: CtrlPointId, to: EndpointId) -> PathEventId {
        self.begin_if_needed();
        let id = PathEventId(self.path.cmds.len() as u32);
        self.path.cmds.push(PathOp { verb: Verb::Quadratic });
        self.path.cmds.push(PathOp { ctrl_point: ctrl });
        self.path.cmds.push(PathOp { endpoint: to });
        self.last_cmd = Verb::Quadratic;

        id
    }

    pub fn cubic_bezier_to(&mut self, ctrl1: CtrlPointId, ctrl2: CtrlPointId, to: EndpointId)  -> PathEventId {
        self.begin_if_needed();
        let id = PathEventId(self.path.cmds.len() as u32);
        self.path.cmds.push(PathOp { verb: Verb::Cubic });
        self.path.cmds.push(PathOp { ctrl_point: ctrl1 });
        self.path.cmds.push(PathOp { ctrl_point: ctrl2 });
        self.path.cmds.push(PathOp { endpoint: to });
        self.last_cmd = Verb::Cubic;

        id
    }

    pub fn close(&mut self) -> PathEventId {
        let id = PathEventId(self.path.cmds.len() as u32);
        match self.last_cmd {
            Verb::Close | Verb::End => {
                return id;
            }
            _ => {}
        }
        self.path.cmds.push(PathOp { verb: Verb::Close });
        self.path.cmds.push(PathOp { offset: self.first_event_index });
        self.last_cmd = Verb::Close;

        id
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

    /// Consumes the builder and returns path commands.
    pub fn build(mut self) -> PathCommands {
        self.end_if_needed();
        self.path.into_path_commands()
    }
}

/// Can write into a `PathCommandsBuffer`.
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

/// Builds path commands as well as endpoint and control point vectors.
pub struct IdPathBuilder {
    endpoints: Vec<Point>,
    ctrl_points: Vec<Point>,
    cmds: PathCommandsBuilder,
}

impl IdPathBuilder {
    /// Creates a builder without allocating memory.
    pub fn new() -> Self {
        Self {
            endpoints: Vec::new(),
            ctrl_points: Vec::new(),
            cmds: PathCommandsBuilder::new(),
        }
    }

    /// Creates a pre-allocated builder.
    pub fn with_capacity(
        n_endpoints: usize,
        n_ctrl_points: usize,
        n_edges: usize,
    ) -> Self {
        Self {
            endpoints: Vec::with_capacity(n_endpoints),
            ctrl_points: Vec::with_capacity(n_ctrl_points),
            cmds: PathCommandsBuilder::with_capacity(n_edges + n_endpoints + n_ctrl_points),
        }
    }

    pub fn move_to(&mut self, to: Point) -> PathEventId {
        let id = self.add_endpoint(to);
        self.cmds.move_to(id)
    }

    pub fn line_to(&mut self, to: Point) -> PathEventId {
        let id = self.add_endpoint(to);
        self.cmds.line_to(id)
    }

    pub fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) -> PathEventId {
        let ctrl = self.add_ctrl_point(ctrl);
        let to = self.add_endpoint(to);
        self.cmds.quadratic_bezier_to(ctrl, to)
    }

    pub fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) -> PathEventId {
        let ctrl1 = self.add_ctrl_point(ctrl1);
        let ctrl2 = self.add_ctrl_point(ctrl2);
        let to = self.add_endpoint(to);
        self.cmds.cubic_bezier_to(ctrl1, ctrl2, to)
    }

    pub fn close(&mut self) -> PathEventId {
        self.cmds.close()
    }

    /// Consumes the builder and returns the generated path commands.
    pub fn build(self) -> IdPath {
        IdPath {
            cmds: self.cmds.build(),
            endpoints: self.endpoints.into_boxed_slice(),
            ctrl_points: self.ctrl_points.into_boxed_slice(),
        }
    }

    #[inline]
    fn add_endpoint(&mut self, ep: Point) -> EndpointId {
        let id = EndpointId(self.endpoints.len() as u32);
        self.endpoints.push(ep);
        id
    }

    #[inline]
    fn add_ctrl_point(&mut self, cp: Point) -> CtrlPointId {
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
