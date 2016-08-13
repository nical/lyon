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
    RelativeArcTo(Point, Vec2, f32, ArcFlags),
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

impl FlattenedEvent {
    pub fn to_svg(self) -> SvgEvent {
        return match self {
            FlattenedEvent::MoveTo(to) => { SvgEvent::MoveTo(to) }
            FlattenedEvent::LineTo(to) => { SvgEvent::LineTo(to) }
            FlattenedEvent::Close => { SvgEvent::Close }
        }
    }

    pub fn to_primitive(self) -> PrimitiveEvent {
        return match self {
            FlattenedEvent::MoveTo(to) => { PrimitiveEvent::MoveTo(to) }
            FlattenedEvent::LineTo(to) => { PrimitiveEvent::LineTo(to) }
            FlattenedEvent::Close => { PrimitiveEvent::Close }
        }
    }
}

pub struct PositionState {
    pub current: Point,
    pub first: Point,
}

impl PositionState {
    pub fn new() -> Self {
        PositionState { current: Point::new(0.0, 0.0), first: Point::new(0.0, 0.0) }
    }
}

impl PositionState {
    pub fn svg_event(&mut self, event: SvgEvent) {
        match event {
            SvgEvent::MoveTo(to) => {
              self.next(to);
              self.first = to;
            }
            SvgEvent::RelativeMoveTo(to) => {
                self.relative_next(to);
                self.first = self.current;
            }
            SvgEvent::LineTo(to) => { self.next(to); }
            SvgEvent::QuadraticTo(_, to) => { self.next(to); }
            SvgEvent::CubicTo(_, _, to) => { self.next(to); }
            SvgEvent::ArcTo(to, _, _, _) => { self.next(to); }

            SvgEvent::RelativeLineTo(to) => { self.relative_next(to); }
            SvgEvent::RelativeQuadraticTo(_, to) => { self.relative_next(to); }
            SvgEvent::RelativeCubicTo(_, _, to) => { self.relative_next(to); }
            SvgEvent::RelativeArcTo(to, _, _, _) => { self.relative_next(to); }

            SvgEvent::HorizontalLineTo(x) => { self.current.x = x }
            SvgEvent::VerticalLineTo(y) => { self.current.y = y }
            SvgEvent::RelativeHorizontalLineTo(x) => { self.current.x += x }
            SvgEvent::RelativeVerticalLineTo(y) => { self.current.y += y }
            SvgEvent::Close => { self.current = self.first; }
        }
    }

    pub fn primitive_event(&mut self, event: PrimitiveEvent) {
        match event {
            PrimitiveEvent::MoveTo(to) => {
              self.next(to);
              self.first = to;
            }
            PrimitiveEvent::LineTo(to) => { self.next(to); }
            PrimitiveEvent::QuadraticTo(_, to) => { self.next(to); }
            PrimitiveEvent::CubicTo(_, _, to) => { self.next(to); }
            PrimitiveEvent::Close => {}
        }
    }

    pub fn flattened_event(&mut self, event: FlattenedEvent) {
        match event {
            FlattenedEvent::MoveTo(to) => {
              self.next(to);
              self.first = to;
            }
            FlattenedEvent::LineTo(to) => { self.next(to); }
            FlattenedEvent::Close => {}
        }
    }

    pub fn next(&mut self, to: Point) { self.current = to; }

    pub fn relative_next(&mut self, to: Point) { self.current = self.current + to; }
}

/// Flag parameters for arcs as described by the SVG specification.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ArcFlags {
    pub large_arc: bool,
    pub sweep: bool,
}
