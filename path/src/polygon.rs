use crate::{EndpointId, FlattenedEvent};

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
            prev: None,
            first: EndpointId(0),
            closed: self.closed,
        }
    }
}

// An iterator of `FlattenedEvent<EndpointId>`.
pub struct IdPolygonIter<'l> {
    points: std::slice::Iter<'l, EndpointId>,
    prev: Option<EndpointId>,
    first: EndpointId,
    closed: bool,
}

impl<'l> Iterator for IdPolygonIter<'l> {
    type Item = FlattenedEvent<EndpointId>;
    fn next(&mut self) -> Option<FlattenedEvent<EndpointId>> {
        match (self.prev, self.points.next()) {
            (Some(from), Some(to)) => {
                self.prev = Some(*to);
                Some(FlattenedEvent::Line { from, to: *to })
            }
            (None, Some(at)) => {
                self.prev = Some(*at);
                self.first = *at;
                Some(FlattenedEvent::Begin { at: *at })
            }
            (Some(last), None) => {
                self.prev = None;
                Some(FlattenedEvent::End {
                    last,
                    first: self.first,
                    close: self.closed,
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
}

// An iterator of `FlattenedEvent<&Endpoint>`.
pub struct PolygonIter<'l, T> {
    points: std::slice::Iter<'l, T>,
    prev: Option<&'l T>,
    first: Option<&'l T>,
    closed: bool,
}

impl<'l, T> Iterator for PolygonIter<'l, T> {
    type Item = FlattenedEvent<&'l T>;
    fn next(&mut self) -> Option<FlattenedEvent<&'l T>> {
        match (self.prev, self.points.next()) {
            (Some(from), Some(to)) => {
                self.prev = Some(to);
                Some(FlattenedEvent::Line { from, to })
            }
            (None, Some(at)) => {
                self.prev = Some(at);
                self.first = Some(at);
                Some(FlattenedEvent::Begin { at })
            }
            (Some(last), None) => {
                self.prev = None;
                Some(FlattenedEvent::End {
                    last,
                    first: self.first.unwrap(),
                    close: self.closed,
                })
            }
            (None, None) => None,
        }
    }
}

