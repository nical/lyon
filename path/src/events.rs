use math::{Point, Vector, Radians};
use super::ArcFlags;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SvgEvent {
    MoveTo(Point),
    RelativeMoveTo(Vector),
    LineTo(Point),
    RelativeLineTo(Vector),
    QuadraticTo(Point, Point),
    RelativeQuadraticTo(Vector, Vector),
    CubicTo(Point, Point, Point),
    RelativeCubicTo(Vector, Vector, Vector),
    ArcTo(Vector, Radians<f32>, ArcFlags, Point),
    RelativeArcTo(Vector, Radians<f32>, ArcFlags, Vector),
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
pub enum PathEvent {
    MoveTo(Point),
    LineTo(Point),
    QuadraticTo(Point, Point),
    CubicTo(Point, Point, Point),
    Close,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum QuadraticPathEvent {
    MoveTo,
    LineTo,
    QuadraticTo(Point, Point),
    Close,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FlattenedEvent {
    MoveTo(Point),
    LineTo(Point),
    Close,
}

impl PathEvent {
    pub fn to_svg_event(self) -> SvgEvent {
        return match self {
            PathEvent::MoveTo(to) => SvgEvent::MoveTo(to),
            PathEvent::LineTo(to) => SvgEvent::LineTo(to),
            PathEvent::QuadraticTo(ctrl, to) => SvgEvent::QuadraticTo(ctrl, to),
            PathEvent::CubicTo(ctrl1, ctrl2, to) => SvgEvent::CubicTo(ctrl1, ctrl2, to),
            PathEvent::Close => SvgEvent::Close,
        };
    }

    pub fn destination(self) -> Option<Point> {
        return match self {
            PathEvent::MoveTo(to) => Some(to),
            PathEvent::LineTo(to) => Some(to),
            PathEvent::QuadraticTo(_, to) => Some(to),
            PathEvent::CubicTo(_, _, to) => Some(to),
            PathEvent::Close => None,
        };
    }
}

impl FlattenedEvent {
    pub fn to_svg_event(self) -> SvgEvent {
        return match self {
            FlattenedEvent::MoveTo(to) => SvgEvent::MoveTo(to),
            FlattenedEvent::LineTo(to) => SvgEvent::LineTo(to),
            FlattenedEvent::Close => SvgEvent::Close,
        };
    }

    pub fn to_path_event(self) -> PathEvent {
        return match self {
            FlattenedEvent::MoveTo(to) => PathEvent::MoveTo(to),
            FlattenedEvent::LineTo(to) => PathEvent::LineTo(to),
            FlattenedEvent::Close => PathEvent::Close,
        };
    }
}

impl Into<PathEvent> for FlattenedEvent {
    fn into(self) -> PathEvent { self.to_path_event() }
}

impl Into<SvgEvent> for FlattenedEvent {
    fn into(self) -> SvgEvent { self.to_svg_event() }
}

impl Into<SvgEvent> for PathEvent {
    fn into(self) -> SvgEvent { self.to_svg_event() }
}
