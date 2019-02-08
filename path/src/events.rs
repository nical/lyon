use math::{Point, Vector, Angle, Transform2D, Transform};
use ArcFlags;

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
    LineTo(Point),
    QuadraticTo(Point, Point),
    CubicTo(Point, Point, Point),
    Close,
}

/// Path event enum that can only present quadratic bézier curves and line segments.
///
/// Useful for algorithms that approximate all curves with quadratic béziers.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum QuadraticEvent {
    MoveTo(Point),
    LineTo(Point),
    QuadraticTo(Point, Point),
    Close,
}

/// Path event enum that can only present line segments.
///
/// Useful for algorithms that approximate all curves with line segments.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FlattenedEvent {
    MoveTo(Point),
    LineTo(Point),
    Close,
}

impl FlattenedEvent {
    pub fn to_svg_event(self) -> SvgEvent {
        match self {
            FlattenedEvent::MoveTo(to) => SvgEvent::MoveTo(to),
            FlattenedEvent::LineTo(to) => SvgEvent::LineTo(to),
            FlattenedEvent::Close => SvgEvent::Close,
        }
    }

    pub fn to_path_event(self) -> PathEvent {
        match self {
            FlattenedEvent::MoveTo(to) => PathEvent::MoveTo(to),
            FlattenedEvent::LineTo(to) => PathEvent::LineTo(to),
            FlattenedEvent::Close => PathEvent::Close,
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
            QuadraticEvent::LineTo(to) => SvgEvent::LineTo(to),
            QuadraticEvent::QuadraticTo(ctrl, to) => SvgEvent::QuadraticTo(ctrl, to),
            QuadraticEvent::Close => SvgEvent::Close,
        }
    }

    pub fn to_path_event(self) -> PathEvent {
        match self {
            QuadraticEvent::MoveTo(to) => PathEvent::MoveTo(to),
            QuadraticEvent::LineTo(to) => PathEvent::LineTo(to),
            QuadraticEvent::QuadraticTo(ctrl, to) => PathEvent::QuadraticTo(ctrl, to),
            QuadraticEvent::Close => PathEvent::Close,
        }
    }
}

impl Transform for FlattenedEvent {
    fn transform(&self, mat: &Transform2D) -> Self {
        match self {
            FlattenedEvent::MoveTo(ref to) => {
                FlattenedEvent::MoveTo(mat.transform_point(to))
            }
            FlattenedEvent::LineTo(ref to) => {
                FlattenedEvent::LineTo(mat.transform_point(to))
            }
            FlattenedEvent::Close => { FlattenedEvent::Close }
        }
    }
}

impl Transform for QuadraticEvent {
    fn transform(&self, mat: &Transform2D) -> Self {
        match self {
            QuadraticEvent::MoveTo(ref to) => {
                QuadraticEvent::MoveTo(mat.transform_point(to))
            }
            QuadraticEvent::LineTo(ref to) => {
                QuadraticEvent::LineTo(mat.transform_point(to))
            }
            QuadraticEvent::QuadraticTo(ref ctrl, ref to) => {
                QuadraticEvent::QuadraticTo(
                    mat.transform_point(ctrl),
                    mat.transform_point(to),
                )
            }
            QuadraticEvent::Close => { QuadraticEvent::Close }
        }
    }
}

impl Transform for PathEvent {
    fn transform(&self, mat: &Transform2D) -> Self {
        match self {
            PathEvent::MoveTo(ref to) => {
                PathEvent::MoveTo(mat.transform_point(to))
            }
            PathEvent::LineTo(ref to) => {
                PathEvent::LineTo(mat.transform_point(to))
            }
            PathEvent::QuadraticTo(ref ctrl, ref to) => {
                PathEvent::QuadraticTo(
                    mat.transform_point(ctrl),
                    mat.transform_point(to),
                )
            }
            PathEvent::CubicTo(ref ctrl1, ref ctrl2, ref to) => {
                PathEvent::CubicTo(
                    mat.transform_point(ctrl1),
                    mat.transform_point(ctrl2),
                    mat.transform_point(to),
                )
            }
            PathEvent::Close => { PathEvent::Close }
        }
    }
}
