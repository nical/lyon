use crate::{EndpointId, CtrlPointId, EventId, Position, PositionStore};
use crate::events::{Event, PathEvent, IdEvent};
use crate::math::Point;

use std::fmt;

// Note: Tried making the path generic over the integer type used to store
// the commands to allow u16 and u32, but the performance difference is very
// small and not worth the added complexity.

mod verb {
    pub const LINE: u32 = 0;
    pub const QUADRATIC: u32 = 1;
    pub const CUBIC: u32 = 2;
    pub const BEGIN: u32 = 3;
    pub const CLOSE: u32 = 4;
    pub const END: u32 = 5;
}

/// Sadly this is very close to std::slice::Iter but reimplementing
/// it manually to iterate over u32 makes a difference.
/// It would seem that having next return u32 with a special value
/// for the end of the iteration instead of Option<u32> should
/// improve performance (simpler code and a bunch of unwraps removed),
/// however a naive initial attempt led to worse performance.
#[derive(Copy, Clone)]
struct CmdIter<'l> {
    ptr: *const u32,
    end: *const u32,
    _marker: std::marker::PhantomData<&'l u32>,
}

impl<'l> CmdIter<'l> {
    fn new(slice: &[u32]) -> Self {
        let ptr = slice.as_ptr();
        let end = unsafe { ptr.offset(slice.len() as isize) };
        CmdIter {
            ptr,
            end,
            _marker: std::marker::PhantomData,
        }
    }

    #[inline]
    fn next(&mut self) -> Option<u32> {
        unsafe {
            if self.ptr == self.end {
                return None;
            }

            let val = *self.ptr;
            self.ptr = self.ptr.offset(1);

            Some(val)
        }
    }
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
    cmds: Box<[u32]>,
}

impl PathCommands {
    /// Creates a [PathCommandsBuilder](struct.PathCommandsBuilder.html) to create path commands.
    pub fn builder() -> PathCommandsBuilder {
        PathCommandsBuilder::new()
    }

    /// Returns an iterator over the path commands.
    pub fn id_events(&self) -> IdEvents {
        IdEvents::new(&self.cmds)
    }

    /// Returns a view on the path commands.
    pub fn as_slice(&self) -> PathCommandsSlice {
        PathCommandsSlice {
            cmds: &self.cmds,
        }
    }

    /// Returns a view on a path made of these commands with endpoint and
    /// control point slices.
    pub fn path_slice<'l, Endpoint, CtrlPoint>(
        &'l self,
        endpoints: &'l [Endpoint],
        ctrl_points: &'l [CtrlPoint],
    ) -> GenericPathSlice<Endpoint, CtrlPoint> {
        GenericPathSlice {
            endpoints,
            ctrl_points,
            cmds: self.as_slice(),
        }
    }

    /// Returns an iterator over the path, with endpoints and control points.
    pub fn events<'l, Endpoint, CtrlPoint>(
        &'l self,
        endpoints: &'l [Endpoint],
        ctrl_points: &'l [CtrlPoint],
    ) -> Events<Endpoint, CtrlPoint> {
        Events {
            cmds: CmdIter::new(&self.cmds),
            first_endpoint: 0,
            prev_endpoint: 0,
            endpoints,
            ctrl_points,
        }
    }

    /// Returns the event for a given event ID.
    pub fn event(&self, id: EventId) -> IdEvent {
        self.as_slice().event(id)
    }

    /// Returns the next event id within the path.
    pub fn next_event_id_in_path(&self, id: EventId) -> Option<EventId> {
        self.as_slice().next_event_id_in_path(id)
    }

    /// Returns the next event id within the sub-path.
    ///
    /// Loops back to the first event after the end of the sub-path.
    pub fn next_event_id_in_sub_path(&self, id: EventId) -> EventId {
        self.as_slice().next_event_id_in_sub_path(id)
    }
}

impl fmt::Debug for PathCommands {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl<'l> IntoIterator for &'l PathCommands {
    type Item = IdEvent;
    type IntoIter = IdEvents<'l>;

    fn into_iter(self) -> IdEvents<'l> { self.id_events() }
}

impl<'l> Into<PathCommandsSlice<'l>> for &'l PathCommands {
    fn into(self) -> PathCommandsSlice<'l> {
        self.as_slice()
    }
}

#[derive(Copy, Clone)]
pub struct PathCommandsSlice<'l> {
    cmds: &'l [u32],
}

impl<'l> PathCommandsSlice<'l> {
    /// Returns an iterator over the path commands.
    pub fn id_events(&self) -> IdEvents {
        IdEvents::new(self.cmds)
    }

    /// Returns the event for a given event ID.
    pub fn event(&self, id: EventId) -> IdEvent {
        let idx = id.to_usize();
        match self.cmds[idx] {
            verb::LINE => IdEvent::Line {
                from: EndpointId(self.cmds[idx - 1]),
                to: EndpointId(self.cmds[idx + 1]),
                edge: id,
            },
            verb::QUADRATIC => IdEvent::Quadratic {
                from: EndpointId(self.cmds[idx - 1]),
                ctrl: CtrlPointId(self.cmds[idx + 1]),
                to: EndpointId(self.cmds[idx + 2]),
                edge: id,
            },
            verb::CUBIC => IdEvent::Cubic {
                from: EndpointId(self.cmds[idx - 1]),
                ctrl1: CtrlPointId(self.cmds[idx + 1]),
                ctrl2: CtrlPointId(self.cmds[idx + 2]),
                to: EndpointId(self.cmds[idx + 3]),
                edge: id,
            },
            verb::BEGIN => IdEvent::Begin {
                at: EndpointId(self.cmds[idx + 1])
            },
            verb::END => {
                let first_event = self.cmds[idx + 1] as usize;
                IdEvent::End {
                    last: EndpointId(self.cmds[idx - 1]),
                    first: EndpointId(self.cmds[first_event + 1]),
                    close: false,
                    edge: id,
                }
            }
            _ => {
                // CLOSE
                let first_event = self.cmds[idx + 1] as usize;
                IdEvent::End {
                    last: EndpointId(self.cmds[idx - 1]),
                    first: EndpointId(self.cmds[first_event + 1]),
                    close: true,
                    edge: id,
                }
            }
        }
    }

    /// Returns the next event id within the path.
    pub fn next_event_id_in_sub_path(&self, id: EventId) -> EventId {
        let idx = id.to_usize();
        match self.cmds[idx] {
            verb::LINE | verb::BEGIN => EventId(id.0 + 2),
            verb::QUADRATIC => EventId(id.0 + 3),
            verb::CUBIC => EventId(id.0 + 4),
            //verb::END | verb::CLOSE
            _ => EventId(self.cmds[idx + 1]),
        }
    }

    /// Returns the next event id within the path.
    pub fn next_event_id_in_path(&self, id: EventId) -> Option<EventId> {
        let idx = id.to_usize();
        let next = match self.cmds[idx] {
            verb::QUADRATIC => EventId(id.0 + 3),
            verb::CUBIC => EventId(id.0 + 4),
            // verb::LINE | verb::BEGIN | verb::END | verb::CLOSE
            _ => EventId(id.0 + 2),
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
        for evt in self.id_events() {
            match evt {
                IdEvent::Line { to, .. } => { write!(f, "L {:?}", to) }
                IdEvent::Quadratic { ctrl,  to, .. } => { write!(f, "Q {:?} {:?} ", ctrl, to) }
                IdEvent::Cubic { ctrl1, ctrl2, to, .. } => { write!(f, "C {:?} {:?} {:?} ", ctrl1, ctrl2, to) }
                IdEvent::Begin { at, .. } => { write!(f, "M {:?} ", at) }
                IdEvent::End { close: true, .. } => { write!(f, "Z ") }
                IdEvent::End { close: false, .. } => { Ok(()) }
            }?;
        }
        write!(f, "}}")
    }
}

pub struct GenericPath<Endpoint, CtrlPoint> {
    cmds: PathCommands,
    endpoints: Box<[Endpoint]>,
    ctrl_points: Box<[CtrlPoint]>,
}

impl<Endpoint, CtrlPoint> GenericPath<Endpoint, CtrlPoint> {
    /// Creates a [GenericPathBuilder](struct.GenericPathBuilder.html).
    pub fn builder() -> GenericPathBuilder<Endpoint, CtrlPoint> {
        GenericPathBuilder::new()
    }

    /// Returns a view on a path made of these commands with endpoint and
    /// control point slices.
    pub fn as_slice(&self) -> GenericPathSlice<Endpoint, CtrlPoint> {
        GenericPathSlice {
            cmds: self.cmds.as_slice(),
            endpoints: &self.endpoints,
            ctrl_points: &self.ctrl_points,
        }
    }

    /// Returns an iterator over the path commands, using endpoint
    /// and control point ids instead of positions.
    pub fn id_events(&self) -> IdEvents {
        self.cmds.id_events()
    }

    /// Returns an iterator over the path, with endpoints and control points.
    pub fn events(&self) -> Events<Endpoint, CtrlPoint> {
        Events {
            cmds: CmdIter::new(&self.cmds.cmds),
            first_endpoint: 0,
            prev_endpoint: 0,
            endpoints: &self.endpoints,
            ctrl_points: &self.ctrl_points,
        }
    }

    /// Returns the event for a given event ID.
    pub fn id_event(&self, id: EventId) -> IdEvent {
        self.cmds.as_slice().event(id)
    }

    /// Returns the event for a given event ID.
    pub fn event(&self, id: EventId) -> Event<&Endpoint, &CtrlPoint> {
        match self.id_event(id) {
            IdEvent::Begin { at } => Event::Begin {
                at: &self[at]
            },
            IdEvent::Line { from, to, .. } => Event::Line {
                from: &self[from],
                to: &self[to]
            },
            IdEvent::Quadratic { from, ctrl, to, .. } => Event::Quadratic {
                from: &self[from],
                ctrl: &self[ctrl],
                to: &self[to]
            },
            IdEvent::Cubic { from, ctrl1, ctrl2, to, .. } => Event::Cubic {
                from: &self[from],
                ctrl1: &self[ctrl1],
                ctrl2: &self[ctrl2],
                to: &self[to]
            },
            IdEvent::End { last, first, close, .. } => Event::End {
                last: &self[last],  first: &self[first], close
            },
        }
    }

    /// Returns the next event id within the path.
    pub fn next_event_id_in_path(&self, id: EventId) -> Option<EventId> {
        self.cmds.as_slice().next_event_id_in_path(id)
    }

    /// Returns the next event id within the sub-path.
    ///
    /// Loops back to the first event after the end of the sub-path.
    pub fn next_event_id_in_sub_path(&self, id: EventId) -> EventId {
        self.cmds.as_slice().next_event_id_in_sub_path(id)
    }

    pub fn endpoints(&self) -> &[Endpoint] { &self.endpoints }

    pub fn ctrl_points(&self) -> &[CtrlPoint] { &self.ctrl_points }
}

impl<Endpoint, CtrlPoint> std::ops::Index<EndpointId> for GenericPath<Endpoint, CtrlPoint> {
    type Output = Endpoint;
    fn index(&self, id: EndpointId) -> &Endpoint {
        &self.endpoints[id.to_usize()]
    }
}

impl<Endpoint, CtrlPoint> std::ops::Index<CtrlPointId> for GenericPath<Endpoint, CtrlPoint> {
    type Output = CtrlPoint;
    fn index(&self, id: CtrlPointId) -> &CtrlPoint {
        &self.ctrl_points[id.to_usize()]
    }
}

/// A view on a `Path`.
#[derive(Copy, Clone)]
pub struct GenericPathSlice<'l, Endpoint, CtrlPoint> {
    endpoints: &'l [Endpoint],
    ctrl_points: &'l [CtrlPoint],
    cmds: PathCommandsSlice<'l>,
}

impl<'l, Endpoint, CtrlPoint> GenericPathSlice<'l, Endpoint, CtrlPoint> {
    /// Returns an iterator over the events of the path using IDs.
    pub fn id_events(&self) -> IdEvents {
        self.cmds.id_events()
    }

    /// Returns an iterator over the events of the path using endpoint
    /// and control point references.
    pub fn events(&self) -> Events<Endpoint, CtrlPoint> {
        Events {
            cmds: CmdIter::new(&self.cmds.cmds),
            first_endpoint: 0,
            prev_endpoint: 0,
            endpoints: &self.endpoints[..],
            ctrl_points: &self.ctrl_points[..],
        }
    }
}

impl<'l, Endpoint, CtrlPoint> std::ops::Index<EndpointId> for GenericPathSlice<'l, Endpoint, CtrlPoint> {
    type Output = Endpoint;
    fn index(&self, id: EndpointId) -> &Endpoint {
        &self.endpoints[id.to_usize()]
    }
}

impl<'l, Endpoint, CtrlPoint> std::ops::Index<CtrlPointId> for GenericPathSlice<'l, Endpoint, CtrlPoint> {
    type Output = CtrlPoint;
    fn index(&self, id: CtrlPointId) -> &CtrlPoint {
        &self.ctrl_points[id.to_usize()]
    }
}

impl<'l, Endpoint, CtrlPoint> fmt::Debug for GenericPathSlice<'l, Endpoint, CtrlPoint>
where
    Endpoint: fmt::Debug,
    CtrlPoint: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{ ")?;
        for evt in self.events() {
            match evt {
                Event::Line { to, .. } => { write!(f, "L {:?}", to) }
                Event::Quadratic { ctrl,  to, .. } => { write!(f, "Q {:?} {:?} ", ctrl, to) }
                Event::Cubic { ctrl1, ctrl2, to, .. } => { write!(f, "C {:?} {:?} {:?} ", ctrl1, ctrl2, to) }
                Event::Begin { at, .. } => { write!(f, "M {:?} ", at) }
                Event::End { close: true, .. } => { write!(f, "Z ") }
                Event::End { close: false, .. } => { Ok(()) }
            }?;
        }
        write!(f, "}}")
    }
}

/// Builds path commands.
#[derive(Clone)]
pub struct PathCommandsBuilder {
    cmds: Vec<u32>,
    last_cmd: u32,
    start: u32,
    first_event_index: u32,
}

impl PathCommandsBuilder {
    /// Creates a builder without allocating memory.
    pub fn new() -> Self {
        Self {
            start: 0,
            cmds: Vec::new(),
            last_cmd: verb::END,
            first_event_index: 0,
        }
    }

    /// Creates a pre-allocated builder.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            start: 0,
            cmds: Vec::with_capacity(cap),
            last_cmd: verb::END,
            first_event_index: 0,
        }
    }

    pub fn move_to(&mut self, to: EndpointId) -> EventId {
        self.end_if_needed();
        self.first_event_index = self.cmds.len() as u32;
        let id = EventId(self.cmds.len() as u32);
        self.cmds.push(verb::BEGIN);
        self.cmds.push(to.0);
        self.last_cmd = verb::BEGIN;

        id
    }

    pub fn line_to(&mut self, to: EndpointId) -> EventId {
        self.begin_if_needed();
        let id = EventId(self.cmds.len() as u32);
        self.cmds.push(verb::LINE);
        self.cmds.push(to.0);
        self.last_cmd = verb::LINE;

        id
    }

    pub fn quadratic_bezier_to(&mut self, ctrl: CtrlPointId, to: EndpointId) -> EventId {
        self.begin_if_needed();
        let id = EventId(self.cmds.len() as u32);
        self.cmds.push(verb::QUADRATIC);
        self.cmds.push(ctrl.0);
        self.cmds.push(to.0);
        self.last_cmd = verb::QUADRATIC;

        id
    }

    pub fn cubic_bezier_to(&mut self, ctrl1: CtrlPointId, ctrl2: CtrlPointId, to: EndpointId) -> EventId {
        self.begin_if_needed();
        let id = EventId(self.cmds.len() as u32);
        self.cmds.push(verb::CUBIC);
        self.cmds.push(ctrl1.0);
        self.cmds.push(ctrl2.0);
        self.cmds.push(to.0);
        self.last_cmd = verb::CUBIC;

        id
    }

    pub fn close(&mut self) -> EventId {
        let id = EventId(self.cmds.len() as u32);
        match self.last_cmd {
            verb::CLOSE | verb::END => {
                return id;
            }
            _ => {}
        }
        self.cmds.push(verb::CLOSE);
        self.cmds.push(self.first_event_index);
        self.last_cmd = verb::CLOSE;

        id
    }

    fn begin_if_needed(&mut self) {
        match self.last_cmd {
            verb::CLOSE | verb::END => {
                let first = self.cmds.last().cloned().unwrap_or(0);
                self.move_to(EndpointId(first));
            }
            _ => {}
        }
    }

    fn end_if_needed(&mut self) {
        match self.last_cmd {
            verb::LINE | verb::QUADRATIC | verb::CUBIC => {
                self.cmds.push(verb::END);
                self.cmds.push(self.first_event_index);
            }
            _ => {}
        }
    }

    /// Consumes the builder and returns path commands.
    pub fn build(mut self) -> PathCommands {
        self.end_if_needed();

        PathCommands {
            cmds: self.cmds.into_boxed_slice(),
        }
    }
}

/// Builds path commands as well as endpoint and control point vectors.
pub struct GenericPathBuilder<Endpoint, CtrlPoint> {
    endpoints: Vec<Endpoint>,
    ctrl_points: Vec<CtrlPoint>,
    cmds: PathCommandsBuilder,
}

impl<Endpoint, CtrlPoint> GenericPathBuilder<Endpoint, CtrlPoint> {
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

    pub fn move_to(&mut self, to: Endpoint) -> EventId {
        let id = self.add_endpoint(to);
        self.cmds.move_to(id)
    }

    pub fn line_to(&mut self, to: Endpoint) -> EventId {
        let id = self.add_endpoint(to);
        self.cmds.line_to(id)
    }

    pub fn quadratic_bezier_to(&mut self, ctrl: CtrlPoint, to: Endpoint) -> EventId {
        let ctrl = self.add_ctrl_point(ctrl);
        let to = self.add_endpoint(to);
        self.cmds.quadratic_bezier_to(ctrl, to)
    }

    pub fn cubic_bezier_to(&mut self, ctrl1: CtrlPoint, ctrl2: CtrlPoint, to: Endpoint) -> EventId {
        let ctrl1 = self.add_ctrl_point(ctrl1);
        let ctrl2 = self.add_ctrl_point(ctrl2);
        let to = self.add_endpoint(to);
        self.cmds.cubic_bezier_to(ctrl1, ctrl2, to)
    }

    pub fn close(&mut self) -> EventId {
        self.cmds.close()
    }

    /// Consumes the builder and returns the generated path commands.
    pub fn build(self) -> GenericPath<Endpoint, CtrlPoint> {
        GenericPath {
            cmds: self.cmds.build(),
            endpoints: self.endpoints.into_boxed_slice(),
            ctrl_points: self.ctrl_points.into_boxed_slice(),
        }
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

/// An iterator of `Event<&Endpoint, &CtrlPoint>`.
#[derive(Clone)]
pub struct Events<'l, Endpoint, CtrlPoint> {
    cmds: CmdIter<'l>,
    prev_endpoint: usize,
    first_endpoint: usize,
    endpoints: &'l[Endpoint],
    ctrl_points: &'l[CtrlPoint],
}

impl<'l, Endpoint, CtrlPoint> Iterator for Events<'l, Endpoint, CtrlPoint> {
    type Item = Event<&'l Endpoint, &'l CtrlPoint>;


    fn next(&mut self) -> Option<Event<&'l Endpoint, &'l CtrlPoint>> {
        match self.cmds.next() {
            Some(verb::BEGIN) => {
                let to = self.cmds.next().unwrap() as usize;
                self.prev_endpoint = to;
                self.first_endpoint = to;
                Some(Event::Begin {
                    at: &self.endpoints[to]
                })
            }
            Some(verb::LINE) => {
                let to = self.cmds.next().unwrap() as usize;
                let from = self.prev_endpoint;
                self.prev_endpoint = to;
                Some(Event::Line {
                    from: &self.endpoints[from],
                    to: &self.endpoints[to],
                })
            }
            Some(verb::QUADRATIC) => {
                let ctrl = self.cmds.next().unwrap() as usize;
                let to = self.cmds.next().unwrap() as usize;
                let from = self.prev_endpoint;
                self.prev_endpoint = to;
                Some(Event::Quadratic {
                    from: &self.endpoints[from],
                    ctrl: &self.ctrl_points[ctrl],
                    to: &self.endpoints[to],
                })
            }
            Some(verb::CUBIC) => {
                let ctrl1 = self.cmds.next().unwrap() as usize;
                let ctrl2 = self.cmds.next().unwrap() as usize;
                let to = self.cmds.next().unwrap() as usize;
                let from = self.prev_endpoint;
                self.prev_endpoint = to;
                Some(Event::Cubic {
                    from: &self.endpoints[from],
                    ctrl1: &self.ctrl_points[ctrl1],
                    ctrl2: &self.ctrl_points[ctrl2],
                    to: &self.endpoints[to],
                })
            }
            Some(verb::END) => {
                let _first_index = self.cmds.next();
                let last = self.prev_endpoint;
                let first = self.first_endpoint;
                self.prev_endpoint = first;
                Some(Event::End {
                    last: &self.endpoints[last],
                    first: &self.endpoints[first],
                    close: false,
                })
            }
            Some(_) => {
                // CLOSE
                let _first_index = self.cmds.next();
                let last = self.prev_endpoint;
                let first = self.first_endpoint;
                self.prev_endpoint = first;
                Some(Event::End {
                    last: &self.endpoints[last],
                    first: &self.endpoints[first],
                    close: true,
                })
            }
            None => None
        }
    }
}

impl<'l, Ep, Cp> Events<'l, Ep, Cp>
where
    Ep: Position,
    Cp: Position,
{
    pub fn points(self) -> PointEvents<'l, Ep, Cp> {
        PointEvents {
            cmds: self.cmds,
            prev_endpoint: self.prev_endpoint,
            first_endpoint: self.first_endpoint,
            endpoints: self.endpoints,
            ctrl_points: self.ctrl_points,
        }
    }
}
/// An iterator of `Event<&Endpoint, &CtrlPoint>`.
#[derive(Clone)]
pub struct IdEvents<'l> {
    cmds: CmdIter<'l>,
    idx: u32,
    prev_endpoint: EndpointId,
    first_endpoint: EndpointId,
}

impl<'l> IdEvents<'l> {
    fn new(cmds: &[u32]) -> Self {
        IdEvents {
            cmds: CmdIter::new(cmds),
            idx:0,
            prev_endpoint: EndpointId(0),
            first_endpoint: EndpointId(0),
        }
    }
}

impl<'l> Iterator for IdEvents<'l> {
    type Item = IdEvent;

    fn next(&mut self) -> Option<IdEvent> {
        let evt_idx = EventId(self.idx);
        match self.cmds.next() {
            Some(verb::BEGIN) => {
                let to = EndpointId(self.cmds.next().unwrap());
                self.prev_endpoint = to;
                self.first_endpoint = to;
                self.idx += 2;
                Some(IdEvent::Begin { at: to })
            }
            Some(verb::LINE) => {
                let to = EndpointId(self.cmds.next().unwrap());
                let from = self.prev_endpoint;
                self.prev_endpoint = to;
                self.idx += 2;
                Some(IdEvent::Line {
                    from: from,
                    to: to,
                    edge: evt_idx,
                })
            }
            Some(verb::QUADRATIC) => {
                let ctrl = CtrlPointId(self.cmds.next().unwrap());
                let to = EndpointId(self.cmds.next().unwrap());
                let from = self.prev_endpoint;
                self.prev_endpoint = to;
                self.idx += 3;
                Some(IdEvent::Quadratic {
                    from: from,
                    ctrl: ctrl,
                    to: to,
                    edge: evt_idx,
                })
            }
            Some(verb::CUBIC) => {
                let ctrl1 = CtrlPointId(self.cmds.next().unwrap());
                let ctrl2 = CtrlPointId(self.cmds.next().unwrap());
                let to = EndpointId(self.cmds.next().unwrap());
                let from = self.prev_endpoint;
                self.prev_endpoint = to;
                self.idx += 4;
                Some(IdEvent::Cubic {
                    from: from,
                    ctrl1: ctrl1,
                    ctrl2: ctrl2,
                    to: to,
                    edge: evt_idx,
                })
            }
            Some(verb::END) => {
                let _first_index = self.cmds.next();
                let last = self.prev_endpoint;
                let first = self.first_endpoint;
                self.prev_endpoint = first;
                self.idx += 2;
                Some(IdEvent::End {
                    last: last,
                    first: first,
                    close: false,
                    edge: evt_idx,
                })
            }
            Some(_) => {
                let _first_index = self.cmds.next();
                let last = self.prev_endpoint;
                let first = self.first_endpoint;
                self.prev_endpoint = first;
                self.idx += 2;
                Some(IdEvent::End {
                    last: last,
                    first: first,
                    close: true,
                    edge: evt_idx,
                })
            }
            None => None,
        }
    }
}

/// An iterator of `PathEvent`.
#[derive(Clone)]
pub struct PointEvents<'l, Endpoint, CtrlPoint> {
    cmds: CmdIter<'l>,
    prev_endpoint: usize,
    first_endpoint: usize,
    endpoints: &'l[Endpoint],
    ctrl_points: &'l[CtrlPoint],
}

impl<'l, Endpoint, CtrlPoint> Iterator for PointEvents<'l, Endpoint, CtrlPoint>
where
    Endpoint: Position,
    CtrlPoint: Position,
{
    type Item = PathEvent;

    fn next(&mut self) -> Option<PathEvent> {
        match self.cmds.next() {
            Some(verb::BEGIN) => {
                let to = self.cmds.next().unwrap() as usize;
                self.prev_endpoint = to;
                self.first_endpoint = to;
                Some(Event::Begin {
                    at: self.endpoints[to].position()
                })
            }
            Some(verb::LINE) => {
                let to = self.cmds.next().unwrap() as usize;
                let from = self.prev_endpoint;
                self.prev_endpoint = to;
                Some(Event::Line {
                    from: self.endpoints[from].position(),
                    to: self.endpoints[to].position(),
                })
            }
            Some(verb::QUADRATIC) => {
                let ctrl = self.cmds.next().unwrap() as usize;
                let to = self.cmds.next().unwrap() as usize;
                let from = self.prev_endpoint;
                self.prev_endpoint = to;
                Some(Event::Quadratic {
                    from: self.endpoints[from].position(),
                    ctrl: self.ctrl_points[ctrl].position(),
                    to: self.endpoints[to].position(),
                })
            }
            Some(verb::CUBIC) => {
                let ctrl1 = self.cmds.next().unwrap() as usize;
                let ctrl2 = self.cmds.next().unwrap() as usize;
                let to = self.cmds.next().unwrap() as usize;
                let from = self.prev_endpoint;
                self.prev_endpoint = to;
                Some(Event::Cubic {
                    from: self.endpoints[from].position(),
                    ctrl1: self.ctrl_points[ctrl1].position(),
                    ctrl2: self.ctrl_points[ctrl2].position(),
                    to: self.endpoints[to].position(),
                })
            }
            Some(verb::END) => {
                let _first_index = self.cmds.next();
                let last = self.prev_endpoint;
                let first = self.first_endpoint;
                self.prev_endpoint = first;
                Some(Event::End {
                    last: self.endpoints[last].position(),
                    first: self.endpoints[first].position(),
                    close: false,
                })
            }
            Some(_) => {
                let _first_index = self.cmds.next();
                let last = self.prev_endpoint;
                let first = self.first_endpoint;
                self.prev_endpoint = first;
                Some(Event::End {
                    last: self.endpoints[last].position(),
                    first: self.endpoints[first].position(),
                    close: true,
                })
            }
            None => None,
        }
    }
}

impl<'l, Endpoint, CtrlPoint> PositionStore for GenericPathSlice<'l, Endpoint, CtrlPoint>
where
    Endpoint: Position,
    CtrlPoint: Position,
{
    fn endpoint_position(&self, id: EndpointId) -> Point {
        self[id].position()
    }

    fn ctrl_point_position(&self, id: CtrlPointId) -> Point {
        self[id].position()
    }
}

impl<Endpoint, CtrlPoint> PositionStore for GenericPath<Endpoint, CtrlPoint>
where
    Endpoint: Position,
    CtrlPoint: Position,
{
    fn endpoint_position(&self, id: EndpointId) -> Point {
        self[id].position()
    }

    fn ctrl_point_position(&self, id: CtrlPointId) -> Point {
        self[id].position()
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
    let mut iter = path.id_events();
    assert_eq!(iter.next(), Some(IdEvent::Begin { at: EndpointId(0) }));
    assert_eq!(iter.next(), Some(IdEvent::Line { from: EndpointId(0), to: EndpointId(1), edge: EventId(2) }));
    assert_eq!(iter.next(), Some(IdEvent::Quadratic { from: EndpointId(1), ctrl: CtrlPointId(2), to: EndpointId(3), edge: EventId(4) }));
    assert_eq!(iter.next(), Some(IdEvent::Cubic { from: EndpointId(3), ctrl1: CtrlPointId(4), ctrl2: CtrlPointId(5), to: EndpointId(6), edge: EventId(7) }));
    assert_eq!(iter.next(), Some(IdEvent::End { last: EndpointId(6), first: EndpointId(0), close: false, edge: EventId(11) }));

    assert_eq!(iter.next(), Some(IdEvent::Begin { at: EndpointId(10) }));
    assert_eq!(iter.next(), Some(IdEvent::Line { from: EndpointId(10), to: EndpointId(11), edge: EventId(15) }));
    assert_eq!(iter.next(), Some(IdEvent::Quadratic { from: EndpointId(11), ctrl: CtrlPointId(12), to: EndpointId(13), edge: EventId(17) }));
    assert_eq!(iter.next(), Some(IdEvent::Cubic { from: EndpointId(13), ctrl1: CtrlPointId(14), ctrl2: CtrlPointId(15), to: EndpointId(16), edge: EventId(20) }));
    assert_eq!(iter.next(), Some(IdEvent::End { last: EndpointId(16), first: EndpointId(10), close: true, edge: EventId(24) }));

    assert_eq!(iter.next(), Some(IdEvent::Begin { at: EndpointId(20) }));
    assert_eq!(iter.next(), Some(IdEvent::Line { from: EndpointId(20), to: EndpointId(21), edge: EventId(28) }));
    assert_eq!(iter.next(), Some(IdEvent::Quadratic { from: EndpointId(21), ctrl: CtrlPointId(22), to: EndpointId(23), edge: EventId(30) }));
    assert_eq!(iter.next(), Some(IdEvent::Cubic { from: EndpointId(23), ctrl1: CtrlPointId(24), ctrl2: CtrlPointId(25), to: EndpointId(26), edge: EventId(33) }));
    assert_eq!(iter.next(), Some(IdEvent::End { last: EndpointId(26), first: EndpointId(20), close: false, edge: EventId(37) }));

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

    let mut id = EventId(0);
    let first = id;
    assert_eq!(path.event(id), IdEvent::Begin { at: EndpointId(0) });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), IdEvent::Line { from: EndpointId(0), to: EndpointId(1), edge: id });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), IdEvent::Quadratic { from: EndpointId(1), ctrl: CtrlPointId(2), to: EndpointId(3), edge: id });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), IdEvent::Cubic { from: EndpointId(3), ctrl1: CtrlPointId(4), ctrl2: CtrlPointId(5), to: EndpointId(6), edge: id });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), IdEvent::End { last: EndpointId(6), first: EndpointId(0), close: false, edge: id });

    assert_eq!(path.next_event_id_in_sub_path(id), first);

    id = path.next_event_id_in_path(id).unwrap();
    let first = id;
    assert_eq!(path.event(id), IdEvent::Begin { at: EndpointId(10) });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), IdEvent::Line { from: EndpointId(10), to: EndpointId(11), edge: id });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), IdEvent::Quadratic { from: EndpointId(11), ctrl: CtrlPointId(12), to: EndpointId(13), edge: id });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), IdEvent::Cubic { from: EndpointId(13), ctrl1: CtrlPointId(14), ctrl2: CtrlPointId(15), to: EndpointId(16), edge: id });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), IdEvent::End { last: EndpointId(16), first: EndpointId(10), close: true, edge: id });

    assert_eq!(path.next_event_id_in_sub_path(id), first);

    id = path.next_event_id_in_path(id).unwrap();
    let first = id;
    assert_eq!(path.event(id), IdEvent::Begin { at: EndpointId(20) });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), IdEvent::Line { from: EndpointId(20), to: EndpointId(21), edge: id });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), IdEvent::Quadratic { from: EndpointId(21), ctrl: CtrlPointId(22), to: EndpointId(23), edge: id });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), IdEvent::Cubic { from: EndpointId(23), ctrl1: CtrlPointId(24), ctrl2: CtrlPointId(25), to: EndpointId(26), edge: id });
    id = path.next_event_id_in_path(id).unwrap();
    assert_eq!(path.event(id), IdEvent::End { last: EndpointId(26), first: EndpointId(20), close: false, edge: id });

    assert_eq!(path.next_event_id_in_path(id), None);
    assert_eq!(path.next_event_id_in_sub_path(id), first);
}
