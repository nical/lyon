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
    SmoothQuadraticTo(Point),
    SmoothRelativeQuadraticTo(Vec2),
    SmoothCubicTo(Point, Point),
    SmoothRelativeCubicTo(Vec2, Vec2),
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
    pub last_ctrl: Point,
}

impl PositionState {
    pub fn new() -> Self {
        PositionState {
            current: Point::new(0.0, 0.0),
            first: Point::new(0.0, 0.0),
            last_ctrl: Point::new(0.0, 0.0),
        }
    }
}

impl PositionState {
    pub fn svg_event(&mut self, event: SvgEvent) {
        match event {
            SvgEvent::MoveTo(to) => { self.move_to(to); }
            SvgEvent::RelativeMoveTo(to) => {
                let to = self.get_relative(to);
                self.move_to(to);
            }
            SvgEvent::LineTo(to) => { self.line_to(to) }
            SvgEvent::QuadraticTo(ctrl, to) => { self.curve_to(ctrl, to); }
            SvgEvent::CubicTo(_, ctrl2, to) => { self.curve_to(ctrl2, to); }
            SvgEvent::ArcTo(to, _, _, _) => {
                self.last_ctrl = self.current; // TODO
                self.current = to;
            }
            SvgEvent::RelativeLineTo(to) => {
                let to = self.get_relative(to);
                self.line_to(to);
            }
            SvgEvent::RelativeQuadraticTo(ctrl, to) => {
                let to = self.get_relative(to);
                let ctrl = self.get_relative(ctrl);
                self.curve_to(ctrl, to);
            }
            SvgEvent::RelativeCubicTo(_, ctrl2, to) => {
                let to = self.get_relative(to);
                let ctrl2 = self.get_relative(ctrl2);
                self.curve_to(ctrl2, to);
            }
            SvgEvent::RelativeArcTo(to, _, _, _) => {
                self.last_ctrl = self.current; // TODO
                self.relative_next(to);
            }
            SvgEvent::HorizontalLineTo(x) => {
                let to = Point::new(x, self.current.y);
                self.line_to(to);
            }
            SvgEvent::VerticalLineTo(y) => {
                let to = Point::new(self.current.x, y);
                self.line_to(to);
            }
            SvgEvent::RelativeHorizontalLineTo(x) => {
                let to = self.current + Point::new(x, 0.0);
                self.line_to(to);
            }
            SvgEvent::RelativeVerticalLineTo(y) => {
                let to = self.current + Point::new(0.0, y);
                self.line_to(to);
            }
            SvgEvent::SmoothQuadraticTo(to) => {
                let ctrl = self.get_smooth_ctrl();
                self.curve_to(ctrl, to);
            }
            SvgEvent::SmoothCubicTo(ctrl2, to) => {
                self.curve_to(ctrl2, to);
            }
            SvgEvent::SmoothRelativeQuadraticTo(to) => {
                let to = self.get_relative(to);
                let ctrl = self.get_smooth_ctrl();
                self.curve_to(ctrl, to);
            }
            SvgEvent::SmoothRelativeCubicTo(ctrl2, to) => {
                let to = self.get_relative(to);
                self.curve_to(ctrl2, to);
            }
            SvgEvent::Close => {
                self.close();
            }
        }
    }

    pub fn move_to(&mut self, to: Point) {
        self.last_ctrl = self.current;
        self.current = to;
        self.first = to;
    }

    pub fn line_to(&mut self, to: Point) {
        self.last_ctrl = self.current;
        self.current = to;
    }

    pub fn curve_to(&mut self, ctrl: Point, to: Point) {
        self.last_ctrl = ctrl;
        self.current = to;
    }

    pub fn close(&mut self) {
        self.last_ctrl = self.first;
        self.current = self.first;
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

    pub fn relative_next(&mut self, to: Point) { self.current = self.get_relative(to); }

    pub fn get_smooth_ctrl(&self) -> Point { self.current + (self.current - self.last_ctrl) }

    pub fn get_relative(&self, v: Vec2) -> Point { self.current + v }

    pub fn svg_to_primitive(&self, event: SvgEvent) -> PrimitiveEvent {
        return match event {
            SvgEvent::MoveTo(to) => { PrimitiveEvent::MoveTo(to) }
            SvgEvent::LineTo(to) => { PrimitiveEvent::LineTo(to) }
            SvgEvent::QuadraticTo(ctrl, to) => { PrimitiveEvent::QuadraticTo(ctrl, to) }
            SvgEvent::CubicTo(ctrl1, ctrl2, to) => { PrimitiveEvent::CubicTo(ctrl1, ctrl2, to) }
            SvgEvent::Close => { PrimitiveEvent::Close }
            SvgEvent::RelativeMoveTo(to) => { PrimitiveEvent::MoveTo(self.get_relative(to)) }
            SvgEvent::RelativeLineTo(to) => { PrimitiveEvent::LineTo(self.get_relative(to)) }
            SvgEvent::RelativeQuadraticTo(ctrl, to) => { PrimitiveEvent::QuadraticTo(self.current + ctrl, self.get_relative(to)) }
            SvgEvent::RelativeCubicTo(ctrl1, ctrl2, to) => { PrimitiveEvent::CubicTo(self.get_relative(ctrl1), self.get_relative(ctrl2), self.get_relative(to)) }
            SvgEvent::HorizontalLineTo(x) => { PrimitiveEvent::LineTo(Point::new(x, self.current.y)) }
            SvgEvent::VerticalLineTo(y) => { PrimitiveEvent::LineTo(Point::new(self.current.x, y)) }
            SvgEvent::RelativeHorizontalLineTo(x) => { PrimitiveEvent::LineTo(Point::new(self.current.x + x, self.current.y)) }
            SvgEvent::RelativeVerticalLineTo(y) => { PrimitiveEvent::LineTo(Point::new(self.current.x, self.current.y + y)) }
            SvgEvent::SmoothQuadraticTo(to) => { PrimitiveEvent::QuadraticTo(self.get_smooth_ctrl(), to) }
            SvgEvent::SmoothRelativeQuadraticTo(to) => { PrimitiveEvent::QuadraticTo(self.get_smooth_ctrl(), self.get_relative(to)) }
            SvgEvent::SmoothCubicTo(ctrl2, to) => { PrimitiveEvent::CubicTo(self.get_smooth_ctrl(), ctrl2, to) }
            SvgEvent::SmoothRelativeCubicTo(ctrl2, to) => { PrimitiveEvent::CubicTo(self.get_smooth_ctrl(), ctrl2, self.get_relative(to)) }
            // TODO arcs
            _ => { unimplemented!() }
        };
    }
}

/// Flag parameters for arcs as described by the SVG specification.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ArcFlags {
    pub large_arc: bool,
    pub sweep: bool,
}
