use crate::{EndpointId, CtrlPointId, EventId, Event, IdEvent, Position, PositionStore};
use crate::math::Point;

/// A view over a sequence of endpoint IDs forming a polygon.
pub struct IdPolygonSlice<'l> {
    pub points: &'l[EndpointId],
    pub closed: bool,
}

impl<'l> IdPolygonSlice<'l> {
    // Returns an iterator over the endpoint IDs of the polygon.
    pub fn iter(&self) -> IdPolygonIter<'l> {
        IdPolygonIter {
            points: self.points.iter(),
            idx: 0,
            prev: None,
            first: EndpointId(0),
            closed: self.closed,
        }
    }

    /// Returns the event for a given event ID.
    pub fn event(&self, id: EventId) -> IdEvent {
        let idx = id.0 as usize;
        if idx == 0 {
            IdEvent::Begin { at: self.points[0] }
        } else if idx as usize == self.points.len() {
            IdEvent::End {
                last: self.points[self.points.len() - 1],
                first: self.points[0],
                close: self.closed,
                edge: id,
            }
        } else {
            IdEvent::Line {
                from: self.points[idx - 1],
                to: self.points[idx],
                edge: id,
            }
        }
    }
}

// An iterator of `Event<EndpointId, ()>`.
pub struct IdPolygonIter<'l> {
    points: std::slice::Iter<'l, EndpointId>,
    idx: u32,
    prev: Option<EndpointId>,
    first: EndpointId,
    closed: bool,
}

impl<'l> Iterator for IdPolygonIter<'l> {
    type Item = IdEvent;
    fn next(&mut self) -> Option<IdEvent> {
        let edge = EventId(self.idx);

        match (self.prev, self.points.next()) {
            (Some(from), Some(to)) => {
                self.prev = Some(*to);
                self.idx += 1;
                Some(IdEvent::Line { from, to: *to, edge })
            }
            (None, Some(at)) => {
                self.prev = Some(*at);
                self.first = *at;
                self.idx += 1;
                Some(IdEvent::Begin { at: *at })
            }
            (Some(last), None) => {
                self.prev = None;
                Some(IdEvent::End {
                    last,
                    first: self.first,
                    close: self.closed,
                    edge,
                })
            }
            (None, None) => None,
        }
    }
}

/// A view over a sequence of endpoints forming a polygon.
pub struct PolygonSlice<'l, T> {
    pub points: &'l[T],
    pub closed: bool,
}

impl<'l, T> PolygonSlice<'l, T> {
    pub fn iter(&self) -> PolygonIter<'l, T> {
        PolygonIter {
            points: self.points.iter(),
            prev: None,
            first: None,
            closed: self.closed,
        }
    }

    /// Returns the event for a given event ID.
    pub fn event(&self, id: EventId) -> Event<&T, ()> {
        let idx = id.0 as usize;
        if idx == 0 {
            Event::Begin { at: &self.points[0] }
        } else if idx as usize == self.points.len() - 1 {
            Event::End {
                last: &self.points[self.points.len() - 1],
                first: &self.points[0],
                close: self.closed,
            }
        } else {
            Event::Line {
                from: &self.points[idx - 1],
                to: &self.points[idx],
            }
        }
    }
}

// An iterator of `Event<&Endpoint, ()>`.
pub struct PolygonIter<'l, T> {
    points: std::slice::Iter<'l, T>,
    prev: Option<&'l T>,
    first: Option<&'l T>,
    closed: bool,
}

impl<'l, T> Iterator for PolygonIter<'l, T> {
    type Item = Event<&'l T, ()>;
    fn next(&mut self) -> Option<Event<&'l T, ()>> {
        match (self.prev, self.points.next()) {
            (Some(from), Some(to)) => {
                self.prev = Some(to);
                Some(Event::Line { from, to })
            }
            (None, Some(at)) => {
                self.prev = Some(at);
                self.first = Some(at);
                Some(Event::Begin { at })
            }
            (Some(last), None) => {
                self.prev = None;
                Some(Event::End {
                    last,
                    first: self.first.unwrap(),
                    close: self.closed,
                })
            }
            (None, None) => None,
        }
    }
}

impl<'l, Endpoint> PositionStore for PolygonSlice<'l, Endpoint>
where
    Endpoint: Position,
{
    fn endpoint_position(&self, id: EndpointId) -> Point {
        self.points[id.to_usize()].position()
    }

    fn ctrl_point_position(&self, _: CtrlPointId) -> Point {
        panic!("Polygons do not have control points.");
    }
}

#[test]
fn event_ids() {
    let poly = IdPolygonSlice {
        points: &[EndpointId(0), EndpointId(1), EndpointId(2), EndpointId(3)],
        closed: true,
    };

    assert_eq!(poly.event(EventId(0)), IdEvent::Begin { at: EndpointId(0) });
    assert_eq!(poly.event(EventId(1)), IdEvent::Line { from: EndpointId(0), to: EndpointId(1), edge: EventId(1) });
    assert_eq!(poly.event(EventId(2)), IdEvent::Line { from: EndpointId(1), to: EndpointId(2), edge: EventId(2) });
    assert_eq!(poly.event(EventId(3)), IdEvent::Line { from: EndpointId(2), to: EndpointId(3), edge: EventId(3) });
    assert_eq!(poly.event(EventId(4)), IdEvent::End { last: EndpointId(3), first: EndpointId(0), close: true, edge: EventId(4) });

    let mut iter = poly.iter();
    assert_eq!(iter.next(), Some(IdEvent::Begin { at: EndpointId(0) }));
    assert_eq!(iter.next(), Some(IdEvent::Line { from: EndpointId(0), to: EndpointId(1), edge: EventId(1) }));
    assert_eq!(iter.next(), Some(IdEvent::Line { from: EndpointId(1), to: EndpointId(2), edge: EventId(2) }));
    assert_eq!(iter.next(), Some(IdEvent::Line { from: EndpointId(2), to: EndpointId(3), edge: EventId(3) }));
    assert_eq!(iter.next(), Some(IdEvent::End { last: EndpointId(3), first: EndpointId(0), close: true, edge: EventId(4) }));
    assert_eq!(iter.next(), None);
    assert_eq!(iter.next(), None);
}

