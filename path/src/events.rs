use math::{Point, Vector, Radians};
use ArcFlags;

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
    ArcTo(Vector, Radians, ArcFlags, Point),
    RelativeArcTo(Vector, Radians, ArcFlags, Vector),
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
    Arc(Point, Vector, Radians, Radians),
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
