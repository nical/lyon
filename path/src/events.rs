use crate::math::{Point, Transform2D, Transform};
use crate::{EndpointId, CtrlPointId, Position};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum PathEvent<Endpoint, CtrlPoint> {
    Line { from: Endpoint, to: Endpoint },
    Quadratic { from: Endpoint, ctrl: CtrlPoint, to: Endpoint },
    Cubic { from: Endpoint, ctrl1: CtrlPoint, ctrl2: CtrlPoint, to: Endpoint },
    Begin { at: Endpoint, },
    End { last: Endpoint, first: Endpoint, close: bool },
}

impl<Ep, Cp> PathEvent<Ep, Cp> {
    pub fn is_edge(&self) -> bool {
        match self {
            &PathEvent::Line { .. }
            | &PathEvent::Quadratic { .. }
            | &PathEvent::Cubic { .. }
            | &PathEvent::End { close: true, .. }
            => true,
            _ => false,
        }
    }

    pub fn from(&self) -> Ep
    where Ep: Clone {
        match &self {
            &PathEvent::Line { from, .. }
            | &PathEvent::Quadratic { from, .. }
            | &PathEvent::Cubic { from, .. }
            | &PathEvent::Begin { at: from }
            | &PathEvent::End { last: from, .. }
            => {
                from.clone()
            }
        }
    }

    pub fn to(&self) -> Ep
    where Ep: Clone {
        match &self {
            &PathEvent::Line { to, .. }
            | &PathEvent::Quadratic { to, .. }
            | &PathEvent::Cubic { to, .. }
            | &PathEvent::Begin { at: to }
            | &PathEvent::End { first: to, .. }
            => {
                to.clone()
            }
        }
    }

    pub fn with_points(&self) -> PathEvent<Point, Point>
    where
        Ep: Position,
        Cp: Position,
    {
        match self {
            PathEvent::Line { from, to } => PathEvent::Line {
                from: from.position(),
                to: to.position(),
            },
            PathEvent::Quadratic { from, ctrl, to } => PathEvent::Quadratic {
                from: from.position(),
                ctrl: ctrl.position(),
                to: to.position(),
            },
            PathEvent::Cubic { from, ctrl1, ctrl2, to } => PathEvent::Cubic {
                from: from.position(),
                ctrl1: ctrl1.position(),
                ctrl2: ctrl2.position(),
                to: to.position(),
            },
            PathEvent::Begin { at } => PathEvent::Begin {
                at: at.position(),
            },
            PathEvent::End { last, first, close } => PathEvent::End {
                last: last.position(),
                first: first.position(),
                close: *close
            },
        }
    }
}

pub type IdEvent = PathEvent<EndpointId, CtrlPointId>;

/// Path event enum that can only present line segments.
///
/// Useful for algorithms that approximate all curves with line segments.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FlattenedEvent<Endpoint> {
    Begin { at: Endpoint },
    Line { from: Endpoint, to: Endpoint },
    End { last: Endpoint, first: Endpoint, close: bool },
}

impl<T> FlattenedEvent<T> {
    pub fn to_path_event<Cp>(self) -> PathEvent<T, Cp> {
        match self {
            FlattenedEvent::Begin { at } => PathEvent::Begin { at },
            FlattenedEvent::Line { from, to } => PathEvent::Line { from, to },
            FlattenedEvent::End { last, first, close } => PathEvent::End { last, first, close, },
        }
    }
}

impl<Ep, Cp> Into<PathEvent<Ep, Cp>> for FlattenedEvent<Ep> {
    fn into(self) -> PathEvent<Ep, Cp> { self.to_path_event() }
}

impl Transform for FlattenedEvent<Point> {
    fn transform(&self, mat: &Transform2D) -> Self {
        match self {
            FlattenedEvent::Line { from, to } => FlattenedEvent::Line {
                from: mat.transform_point(*from),
                to: mat.transform_point(*to),
            },
            FlattenedEvent::Begin { at } => FlattenedEvent::Begin {
                at: mat.transform_point(*at),
            },
            FlattenedEvent::End { first, last, close }  => FlattenedEvent::End {
                last: mat.transform_point(*last),
                first: mat.transform_point(*first),
                close: *close,
            },
        }
    }
}

impl Transform for PathEvent<Point, Point> {
    fn transform(&self, mat: &Transform2D) -> Self {
        match self {
            PathEvent::Line { from, to } => PathEvent::Line {
                from: mat.transform_point(*from),
                to: mat.transform_point(*to),
            },
            PathEvent::Quadratic { from, ctrl, to } => PathEvent::Quadratic {
                from: mat.transform_point(*from),
                ctrl: mat.transform_point(*ctrl),
                to: mat.transform_point(*to),
            },
            PathEvent::Cubic { from, ctrl1, ctrl2, to } => PathEvent::Cubic {
                from: mat.transform_point(*from),
                ctrl1: mat.transform_point(*ctrl1),
                ctrl2: mat.transform_point(*ctrl2),
                to: mat.transform_point(*to),
            },
            PathEvent::Begin { at } => PathEvent::Begin {
                at: mat.transform_point(*at),
            },
            PathEvent::End { first, last, close } => PathEvent::End {
                last: mat.transform_point(*last),
                first: mat.transform_point(*first),
                close: *close,
            },
        }
    }
}
