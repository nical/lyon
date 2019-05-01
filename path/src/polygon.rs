use crate::{EndpointId, FlattenedEvent};

pub struct IdPolygonSlice<'l> {
    pub points: &'l[EndpointId],
    pub closed: bool,
}

impl<'l> IdPolygonSlice<'l> {
    pub fn iter(&self) -> IdPolygonIter<'l> {
        IdPolygonIter {
            points: self.points.iter(),
            prev: None,
            first: EndpointId(0),
            closed: self.closed,
        }
    }
}

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

