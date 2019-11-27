use crate::math::{Point, Transform2D, Transform};
use crate::{EndpointId, CtrlPointId, EventId, Position};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum Event<Endpoint, CtrlPoint> {
    Begin { at: Endpoint, },
    Line { from: Endpoint, to: Endpoint },
    Quadratic { from: Endpoint, ctrl: CtrlPoint, to: Endpoint },
    Cubic { from: Endpoint, ctrl1: CtrlPoint, ctrl2: CtrlPoint, to: Endpoint },
    End { last: Endpoint, first: Endpoint, close: bool },
}

pub type PathEvent = Event<Point, Point>;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum IdEvent {
    Begin { at: EndpointId },
    Line { from: EndpointId, to: EndpointId, edge: EventId },
    Quadratic { from: EndpointId, ctrl: CtrlPointId, to: EndpointId, edge: EventId },
    Cubic { from: EndpointId, ctrl1: CtrlPointId, ctrl2: CtrlPointId, to: EndpointId, edge: EventId },
    End { last: EndpointId, first: EndpointId, close: bool, edge: EventId },
}

impl<Ep, Cp> Event<Ep, Cp> {
    pub fn is_edge(&self) -> bool {
        match self {
            &Event::Line { .. }
            | &Event::Quadratic { .. }
            | &Event::Cubic { .. }
            | &Event::End { close: true, .. }
            => true,
            _ => false,
        }
    }

    pub fn from(&self) -> Ep
    where Ep: Clone {
        match &self {
            &Event::Line { from, .. }
            | &Event::Quadratic { from, .. }
            | &Event::Cubic { from, .. }
            | &Event::Begin { at: from }
            | &Event::End { last: from, .. }
            => {
                from.clone()
            }
        }
    }

    pub fn to(&self) -> Ep
    where Ep: Clone {
        match &self {
            &Event::Line { to, .. }
            | &Event::Quadratic { to, .. }
            | &Event::Cubic { to, .. }
            | &Event::Begin { at: to }
            | &Event::End { first: to, .. }
            => {
                to.clone()
            }
        }
    }

    pub fn with_points(&self) -> PathEvent
    where
        Ep: Position,
        Cp: Position,
    {
        match self {
            Event::Line { from, to } => Event::Line {
                from: from.position(),
                to: to.position(),
            },
            Event::Quadratic { from, ctrl, to } => Event::Quadratic {
                from: from.position(),
                ctrl: ctrl.position(),
                to: to.position(),
            },
            Event::Cubic { from, ctrl1, ctrl2, to } => Event::Cubic {
                from: from.position(),
                ctrl1: ctrl1.position(),
                ctrl2: ctrl2.position(),
                to: to.position(),
            },
            Event::Begin { at } => Event::Begin {
                at: at.position(),
            },
            Event::End { last, first, close } => Event::End {
                last: last.position(),
                first: first.position(),
                close: *close
            },
        }
    }
}

impl Transform for PathEvent {
    fn transform(&self, mat: &Transform2D) -> Self {
        match self {
            Event::Line { from, to } => Event::Line {
                from: mat.transform_point(*from),
                to: mat.transform_point(*to),
            },
            Event::Quadratic { from, ctrl, to } => Event::Quadratic {
                from: mat.transform_point(*from),
                ctrl: mat.transform_point(*ctrl),
                to: mat.transform_point(*to),
            },
            Event::Cubic { from, ctrl1, ctrl2, to } => Event::Cubic {
                from: mat.transform_point(*from),
                ctrl1: mat.transform_point(*ctrl1),
                ctrl2: mat.transform_point(*ctrl2),
                to: mat.transform_point(*to),
            },
            Event::Begin { at } => Event::Begin {
                at: mat.transform_point(*at),
            },
            Event::End { first, last, close } => Event::End {
                last: mat.transform_point(*last),
                first: mat.transform_point(*first),
                close: *close,
            },
        }
    }
}
