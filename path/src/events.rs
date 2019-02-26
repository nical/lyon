use crate::geom::{LineSegment, QuadraticBezierSegment, CubicBezierSegment};
use crate::math::{Point, Vector, Angle, Transform2D, Transform};
use crate::ArcFlags;

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

/// Path event enum that represents all operations in absolute coordinates.
///
/// Can express the same curves as `SvgEvent` with a simpler representation.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum PathEvent {
    MoveTo(Point),
    Line(LineSegment<f32>),
    Quadratic(QuadraticBezierSegment<f32>),
    Cubic(CubicBezierSegment<f32>),
    Close(LineSegment<f32>),
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

    pub fn to_path_event(self) -> PathEvent {
        match self {
            FlattenedEvent::MoveTo(to) => PathEvent::MoveTo(to),
            FlattenedEvent::Line(segment) => PathEvent::Line(segment),
            FlattenedEvent::Close(segment) => PathEvent::Close(segment),
        }
    }
}

impl Into<PathEvent> for FlattenedEvent {
    fn into(self) -> PathEvent { self.to_path_event() }
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

    pub fn to_path_event(self) -> PathEvent {
        match self {
            QuadraticEvent::MoveTo(to) => PathEvent::MoveTo(to),
            QuadraticEvent::Line(segment) => PathEvent::Line(segment),
            QuadraticEvent::Quadratic(segment) => PathEvent::Quadratic(segment),
            QuadraticEvent::Close(segment) => PathEvent::Close(segment),
        }
    }
}

impl Transform for FlattenedEvent {
    fn transform(&self, mat: &Transform2D) -> Self {
        match self {
            FlattenedEvent::MoveTo(ref to) => {
                FlattenedEvent::MoveTo(mat.transform_point(to))
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
            QuadraticEvent::MoveTo(ref to) => {
                QuadraticEvent::MoveTo(mat.transform_point(to))
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

impl Transform for PathEvent {
    fn transform(&self, mat: &Transform2D) -> Self {
        match self {
            PathEvent::MoveTo(ref to) => { PathEvent::MoveTo(mat.transform_point(to)) }
            PathEvent::Line(ref segment) => { PathEvent::Line(segment.transform(mat)) }
            PathEvent::Quadratic(ref segment) => { PathEvent::Quadratic(segment.transform(mat)) }
            PathEvent::Cubic(ref segment) => { PathEvent::Cubic(segment.transform(mat)) }
            PathEvent::Close(ref segment) => { PathEvent::Close(segment.transform(mat)) }
        }
    }
}
