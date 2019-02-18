use crate::geom::{LineSegment, QuadraticBezierSegment};
use crate::math::{Point, Vector, Angle, Transform2D, Transform};
use crate::ArcFlags;
use crate::{EndpointId, CtrlPointId};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum PathEvent<Endpoint, CtrlPoint> {
    Line { from: Endpoint, to: Endpoint },
    Quadratic { from: Endpoint, ctrl: CtrlPoint, to: Endpoint },
    Cubic { from: Endpoint, ctrl1: CtrlPoint, ctrl2: CtrlPoint, to: Endpoint },
    Begin { at: Endpoint, },
    End { last: Endpoint, first: Endpoint, close: bool },
}

pub type IdEvent = PathEvent<EndpointId, CtrlPointId>;

/// Path event enum that can represent all of SVG's path description syntax.
///
/// See the SVG specification: https://www.w3.org/TR/SVG/paths.html
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum SvgEvent {
    MoveTo(Point),
    RelativeMoveTo(Vector),
    LineTo(Point),
    RelativeLineTo(Vector),
    QuadraticTo(Point, Point),
    RelativeQuadraticTo(Vector, Vector),
    CubicTo(Point, Point, Point),
    RelativeCubicTo(Vector, Vector, Vector),
    /// Elliptic arc represented with the radii, the x axis rotation, arc flags
    /// and the destination point.
    ArcTo(Vector, Angle, ArcFlags, Point),
    /// Elliptic arc represented with the radii, the x axis rotation, arc flags
    /// and the vector from the current position to the destination point.
    RelativeArcTo(Vector, Angle, ArcFlags, Vector),
    HorizontalLineTo(f32),
    VerticalLineTo(f32),
    RelativeHorizontalLineTo(f32),
    RelativeVerticalLineTo(f32),
    SmoothQuadraticTo(Point),
    SmoothRelativeQuadraticTo(Vector),
    SmoothCubicTo(Point, Point),
    SmoothRelativeCubicTo(Vector, Vector),
    Close,
}

/// Path event enum that can only present quadratic bézier curves and line segments.
///
/// Useful for algorithms that approximate all curves with quadratic béziers.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum QuadraticEvent {
    MoveTo(Point),
    Line(LineSegment<f32>),
    Quadratic(QuadraticBezierSegment<f32>),
    Close(LineSegment<f32>),
}

/// Path event enum that can only present line segments.
///
/// Useful for algorithms that approximate all curves with line segments.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FlattenedEvent {
    MoveTo(Point),
    Line(LineSegment<f32>),
    Close(LineSegment<f32>),
}

impl FlattenedEvent {
    pub fn to_svg_event(self) -> SvgEvent {
        match self {
            FlattenedEvent::MoveTo(to) => SvgEvent::MoveTo(to),
            FlattenedEvent::Line(segment) => SvgEvent::LineTo(segment.to),
            FlattenedEvent::Close(..) => SvgEvent::Close,
        }
    }

    pub fn to_path_event(self) -> PathEvent<Point, Point> {
        match self {
            FlattenedEvent::MoveTo(to) => PathEvent::Begin { at: to },
            FlattenedEvent::Line(segment) => PathEvent::Line { from: segment.from, to: segment.to },
            FlattenedEvent::Close(segment) => PathEvent::End { last: segment.from, first: segment.to, close: true, },
        }
    }
}

impl Into<PathEvent<Point, Point>> for FlattenedEvent {
    fn into(self) -> PathEvent<Point, Point> { self.to_path_event() }
}

impl Into<SvgEvent> for FlattenedEvent {
    fn into(self) -> SvgEvent { self.to_svg_event() }
}

impl QuadraticEvent {
    pub fn to_svg_event(self) -> SvgEvent {
        match self {
            QuadraticEvent::MoveTo(to) => SvgEvent::MoveTo(to),
            QuadraticEvent::Line(segment) => SvgEvent::LineTo(segment.to),
            QuadraticEvent::Quadratic(segment) => SvgEvent::QuadraticTo(segment.ctrl, segment.to),
            QuadraticEvent::Close(..) => SvgEvent::Close,
        }
    }

    pub fn to_path_event(self) -> PathEvent<Point, Point> {
        match self {
            QuadraticEvent::MoveTo(to) => PathEvent::Begin { at: to },
            QuadraticEvent::Line(segment) => PathEvent::Line { from: segment.from, to: segment.to },
            QuadraticEvent::Quadratic(segment) => PathEvent::Quadratic { from: segment.from, ctrl: segment.ctrl, to: segment.to },
            QuadraticEvent::Close(segment) => PathEvent::End { last: segment.from, first: segment.to, close: true },
        }
    }
}

impl Transform for FlattenedEvent {
    fn transform(&self, mat: &Transform2D) -> Self {
        match self {
            FlattenedEvent::MoveTo(to) => {
                FlattenedEvent::MoveTo(mat.transform_point(*to))
            }
            FlattenedEvent::Line(ref segment) => {
                FlattenedEvent::Line(segment.transform(mat))
            }
            FlattenedEvent::Close(ref segment) => {
                FlattenedEvent::Close(segment.transform(mat))
            }
        }
    }
}

impl Transform for QuadraticEvent {
    fn transform(&self, mat: &Transform2D) -> Self {
        match self {
            QuadraticEvent::MoveTo(to) => {
                QuadraticEvent::MoveTo(mat.transform_point(*to))
            }
            QuadraticEvent::Line(ref segment) => {
                QuadraticEvent::Line(segment.transform(mat))
            }
            QuadraticEvent::Quadratic(ref segment) => {
                QuadraticEvent::Quadratic(segment.transform(mat))
            }
            QuadraticEvent::Close(ref segment) => {
                QuadraticEvent::Close(segment.transform(mat))
            }
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
