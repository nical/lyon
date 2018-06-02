use math::{Point, Vector, Angle, Transform2D, Transform};
use ArcFlags;
use geom::{LineSegment, QuadraticBezierSegment, CubicBezierSegment, Arc};

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
    ArcTo(Vector, Angle, ArcFlags, Point),
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

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum PathEvent {
    MoveTo(Point),
    LineTo(Point),
    QuadraticTo(Point, Point),
    CubicTo(Point, Point, Point),
    Arc(Point, Vector, Angle, Angle),
    Close,
}

#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum QuadraticEvent {
    MoveTo(Point),
    LineTo(Point),
    QuadraticTo(Point, Point),
    Close,
}

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

// TODO: serialization
#[derive(Copy, Clone, Debug)]
pub enum Segment {
    Line(LineSegment<f32>),
    Quadratic(QuadraticBezierSegment<f32>),
    Cubic(CubicBezierSegment<f32>),
    Arc(Arc<f32>),
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
            PathEvent::Arc(..) => {
                unimplemented!(); // TODO!
            }
            PathEvent::Close => { PathEvent::Close }
        }
    }
}
