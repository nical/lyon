use math::{ Point, Vec2, Radians };
use super::ArcFlags;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SvgEvent {
    MoveTo(Point),
    RelativeMoveTo(Vec2),
    LineTo(Point),
    RelativeLineTo(Vec2),
    QuadraticTo(Point, Point),
    RelativeQuadraticTo(Vec2, Vec2),
    CubicTo(Point, Point, Point),
    RelativeCubicTo(Vec2, Vec2, Vec2),
    ArcTo(Point, Vec2, Radians<f32>, ArcFlags),
    RelativeArcTo(Vec2, Vec2, Radians<f32>, ArcFlags),
    HorizontalLineTo(f32),
    VerticalLineTo(f32),
    RelativeHorizontalLineTo(f32),
    RelativeVerticalLineTo(f32),
    SmoothQuadraticTo(Point),
    SmoothRelativeQuadraticTo(Vec2),
    SmoothCubicTo(Point, Point),
    SmoothRelativeCubicTo(Vec2, Vec2),
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
            PathEvent::MoveTo(to) => { SvgEvent::MoveTo(to) }
            PathEvent::LineTo(to) => { SvgEvent::LineTo(to) }
            PathEvent::QuadraticTo(ctrl, to) => { SvgEvent::QuadraticTo(ctrl, to) }
            PathEvent::CubicTo(ctrl1, ctrl2, to) => { SvgEvent::CubicTo(ctrl1, ctrl2, to) }
            PathEvent::Close => { SvgEvent::Close }
        };
    }

    pub fn destination(self) -> Option<Point> {
        return match self {
            PathEvent::MoveTo(to) => Some(to),
            PathEvent::LineTo(to) => Some(to),
            PathEvent::QuadraticTo(_, to) => Some(to),
            PathEvent::CubicTo(_, _, to) => Some(to),
            PathEvent::Close => None,
        }
    }
}

impl FlattenedEvent {
    pub fn to_svg_event(self) -> SvgEvent {
        return match self {
            FlattenedEvent::MoveTo(to) => { SvgEvent::MoveTo(to) }
            FlattenedEvent::LineTo(to) => { SvgEvent::LineTo(to) }
            FlattenedEvent::Close => { SvgEvent::Close }
        }
    }

    pub fn to_path_event(self) -> PathEvent {
        return match self {
            FlattenedEvent::MoveTo(to) => { PathEvent::MoveTo(to) }
            FlattenedEvent::LineTo(to) => { PathEvent::LineTo(to) }
            FlattenedEvent::Close => { PathEvent::Close }
        }
    }
}
