
use math::{ Point, Vec2 };

use super::{ PathEvent, SvgEvent, FlattenedEvent };

/// Represents the current state of a path while it is being built.
pub struct PathState {
    /// The current point.
    pub current: Point,
    /// The first point of the current sub-path.
    pub first: Point,
    /// The last control point.
    pub last_ctrl: Point,
}

impl PathState {
    pub fn new() -> Self {
        PathState {
            current: Point::new(0.0, 0.0),
            first: Point::new(0.0, 0.0),
            last_ctrl: Point::new(0.0, 0.0),
        }
    }
}

impl PathState {
    pub fn svg_event(&mut self, event: SvgEvent) {
        match event {
            SvgEvent::MoveTo(to) => { self.move_to(to); }
            SvgEvent::RelativeMoveTo(to) => {
                let to = self.from_relative(to);
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
                let to = self.from_relative(to);
                self.line_to(to);
            }
            SvgEvent::RelativeQuadraticTo(ctrl, to) => {
                let to = self.from_relative(to);
                let ctrl = self.from_relative(ctrl);
                self.curve_to(ctrl, to);
            }
            SvgEvent::RelativeCubicTo(_, ctrl2, to) => {
                let to = self.from_relative(to);
                let ctrl2 = self.from_relative(ctrl2);
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
                let ctrl = self.get_smooth_ctrl();
                let to = self.from_relative(to);
                self.curve_to(ctrl, to);
            }
            SvgEvent::SmoothRelativeCubicTo(ctrl2, to) => {
                let ctrl2 = self.from_relative(ctrl2);
                let to = self.from_relative(to);
                self.curve_to(ctrl2, to);
            }
            SvgEvent::Close => {
                self.close();
            }
        }
    }

    pub fn path_event(&mut self, event: PathEvent) {
        match event {
            PathEvent::MoveTo(to) => { self.move_to(to); }
            PathEvent::LineTo(to) => { self.line_to(to); }
            PathEvent::QuadraticTo(ctrl, to) => { self.curve_to(ctrl, to); }
            PathEvent::CubicTo(_, ctrl2, to) => { self.curve_to(ctrl2, to); }
            PathEvent::Close => { self.close(); }
        }
    }

    pub fn flattened_event(&mut self, event: FlattenedEvent) {
        match event {
            FlattenedEvent::MoveTo(to) => { self.move_to(to); }
            FlattenedEvent::LineTo(to) => { self.line_to(to); }
            FlattenedEvent::Close => { self.close(); }
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

    pub fn next(&mut self, to: Point) { self.current = to; }

    pub fn relative_next(&mut self, to: Point) { self.current = self.from_relative(to); }

    pub fn get_smooth_ctrl(&self) -> Point { self.current + (self.current - self.last_ctrl) }

    pub fn from_relative(&self, v: Vec2) -> Point { self.current + v }

    pub fn svg_to_path_event(&self, event: SvgEvent) -> PathEvent {
        return match event {
            SvgEvent::MoveTo(to) => { PathEvent::MoveTo(to) }
            SvgEvent::LineTo(to) => { PathEvent::LineTo(to) }
            SvgEvent::QuadraticTo(ctrl, to) => { PathEvent::QuadraticTo(ctrl, to) }
            SvgEvent::CubicTo(ctrl1, ctrl2, to) => { PathEvent::CubicTo(ctrl1, ctrl2, to) }
            SvgEvent::Close => { PathEvent::Close }
            SvgEvent::RelativeMoveTo(to) => { PathEvent::MoveTo(self.from_relative(to)) }
            SvgEvent::RelativeLineTo(to) => { PathEvent::LineTo(self.from_relative(to)) }
            SvgEvent::RelativeQuadraticTo(ctrl, to) => {
                PathEvent::QuadraticTo(self.from_relative(ctrl), self.from_relative(to))
            }
            SvgEvent::RelativeCubicTo(ctrl1, ctrl2, to) => {
                PathEvent::CubicTo(
                    self.from_relative(ctrl1),
                    self.from_relative(ctrl2),
                    self.from_relative(to)
                )
            }
            SvgEvent::HorizontalLineTo(x) => { PathEvent::LineTo(Point::new(x, self.current.y)) }
            SvgEvent::VerticalLineTo(y) => { PathEvent::LineTo(Point::new(self.current.x, y)) }
            SvgEvent::RelativeHorizontalLineTo(x) => { PathEvent::LineTo(Point::new(self.current.x + x, self.current.y)) }
            SvgEvent::RelativeVerticalLineTo(y) => { PathEvent::LineTo(Point::new(self.current.x, self.current.y + y)) }
            SvgEvent::SmoothQuadraticTo(to) => {
                PathEvent::QuadraticTo(self.get_smooth_ctrl(), to)
            }
            SvgEvent::SmoothCubicTo(ctrl2, to) => {
                PathEvent::CubicTo(self.get_smooth_ctrl(), ctrl2, to)
            }
            SvgEvent::SmoothRelativeQuadraticTo(to) => {
                PathEvent::QuadraticTo(self.get_smooth_ctrl(), self.from_relative(to))
            }
            SvgEvent::SmoothRelativeCubicTo(ctrl2, to) => {
                PathEvent::CubicTo(
                    self.get_smooth_ctrl(),
                    self.from_relative(ctrl2),
                    self.from_relative(to)
                )
            }
            // TODO arcs
            _ => { unimplemented!() }
        };
    }
}

