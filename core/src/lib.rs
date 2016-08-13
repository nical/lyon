extern crate euclid;

pub mod math;
pub mod math_utils;

use math::{ Point, Vec2 };

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
    ArcTo(Point, Vec2, f32, ArcFlags),
    HorizontalLineTo(f32),
    VerticalLineTo(f32),
    RelativeHorizontalLineTo(f32),
    RelativeVerticalLineTo(f32),
    Close,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PrimitiveEvent {
    MoveTo(Point),
    LineTo(Point),
    QuadraticTo(Point, Point),
    CubicTo(Point, Point, Point),
    Close,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FlattenedEvent {
    MoveTo(Point),
    LineTo(Point),
    Close,
}

impl SvgEvent {
    pub fn to_primitive(self, current: Point) -> PrimitiveEvent {
        return match self {
            SvgEvent::MoveTo(to) => { PrimitiveEvent::MoveTo(to) }
            SvgEvent::LineTo(to) => { PrimitiveEvent::LineTo(to) }
            SvgEvent::QuadraticTo(ctrl, to) => { PrimitiveEvent::QuadraticTo(ctrl, to) }
            SvgEvent::CubicTo(ctrl1, ctrl2, to) => { PrimitiveEvent::CubicTo(ctrl1, ctrl2, to) }
            SvgEvent::Close => { PrimitiveEvent::Close }
            SvgEvent::RelativeMoveTo(to) => { PrimitiveEvent::MoveTo(current + to) }
            SvgEvent::RelativeLineTo(to) => { PrimitiveEvent::LineTo(current + to) }
            SvgEvent::RelativeQuadraticTo(ctrl, to) => { PrimitiveEvent::QuadraticTo(current + ctrl, current + to) }
            SvgEvent::RelativeCubicTo(ctrl1, ctrl2, to) => { PrimitiveEvent::CubicTo(current + ctrl1, current + ctrl2, to) }
            SvgEvent::HorizontalLineTo(x) => { PrimitiveEvent::LineTo(Point::new(x, current.y)) }
            SvgEvent::VerticalLineTo(y) => { PrimitiveEvent::LineTo(Point::new(current.x, y)) }
            SvgEvent::RelativeHorizontalLineTo(x) => { PrimitiveEvent::LineTo(Point::new(current.x + x, current.y)) }
            SvgEvent::RelativeVerticalLineTo(y) => { PrimitiveEvent::LineTo(Point::new(current.x, current.y + y)) }
            // TODO arcs and smooth events
            _ => { unimplemented!() }
        };
    }
}

impl PrimitiveEvent {
    pub fn to_svg(self) -> SvgEvent {
        return match self {
            PrimitiveEvent::MoveTo(to) => { SvgEvent::MoveTo(to) }
            PrimitiveEvent::LineTo(to) => { SvgEvent::LineTo(to) }
            PrimitiveEvent::QuadraticTo(ctrl, to) => { SvgEvent::QuadraticTo(ctrl, to) }
            PrimitiveEvent::CubicTo(ctrl1, ctrl2, to) => { SvgEvent::CubicTo(ctrl1, ctrl2, to) }
            PrimitiveEvent::Close => { SvgEvent::Close }
        };
    }
}

/// Flag parameters for arcs as described by the SVG specification.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ArcFlags {
    pub large_arc: bool,
    pub sweep: bool,
}
