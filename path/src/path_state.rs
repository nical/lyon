
use math::{Point, Vector, point, vector};
use geom::{Arc, SvgArc};
use events::{PathEvent, SvgEvent, FlattenedEvent};

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
            current: point(0.0, 0.0),
            first: point(0.0, 0.0),
            last_ctrl: point(0.0, 0.0),
        }
    }
}

impl PathState {
    pub fn svg_event(&mut self, event: SvgEvent) {
        match event {
            SvgEvent::MoveTo(to) => {
                self.move_to(to);
            }
            SvgEvent::RelativeMoveTo(to) => {
                let to = self.relative_to_absolute(to);
                self.move_to(to);
            }
            SvgEvent::LineTo(to) => self.line_to(to),
            SvgEvent::QuadraticTo(ctrl, to) => {
                self.curve_to(ctrl, to);
            }
            SvgEvent::CubicTo(_, ctrl2, to) => {
                self.curve_to(ctrl2, to);
            }
            SvgEvent::ArcTo(_, _, _, to) => {
                self.last_ctrl = self.current; // TODO
                self.current = to;
            }
            SvgEvent::RelativeLineTo(to) => {
                let to = self.relative_to_absolute(to);
                self.line_to(to);
            }
            SvgEvent::RelativeQuadraticTo(ctrl, to) => {
                let to = self.relative_to_absolute(to);
                let ctrl = self.relative_to_absolute(ctrl);
                self.curve_to(ctrl, to);
            }
            SvgEvent::RelativeCubicTo(_, ctrl2, to) => {
                let to = self.relative_to_absolute(to);
                let ctrl2 = self.relative_to_absolute(ctrl2);
                self.curve_to(ctrl2, to);
            }
            SvgEvent::RelativeArcTo(_, _, _, to) => {
                self.last_ctrl = self.current; // TODO
                self.relative_next(to);
            }
            SvgEvent::HorizontalLineTo(x) => {
                let to = point(x, self.current.y);
                self.line_to(to);
            }
            SvgEvent::VerticalLineTo(y) => {
                let to = point(self.current.x, y);
                self.line_to(to);
            }
            SvgEvent::RelativeHorizontalLineTo(x) => {
                let to = self.current + vector(x, 0.0);
                self.line_to(to);
            }
            SvgEvent::RelativeVerticalLineTo(y) => {
                let to = self.current + vector(0.0, y);
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
                let to = self.relative_to_absolute(to);
                self.curve_to(ctrl, to);
            }
            SvgEvent::SmoothRelativeCubicTo(ctrl2, to) => {
                let ctrl2 = self.relative_to_absolute(ctrl2);
                let to = self.relative_to_absolute(to);
                self.curve_to(ctrl2, to);
            }
            SvgEvent::Close => {
                self.close();
            }
        }
    }

    pub fn path_event(&mut self, event: PathEvent) {
        match event {
            PathEvent::MoveTo(to) => {
                self.move_to(to);
            }
            PathEvent::LineTo(to) => {
                self.line_to(to);
            }
            PathEvent::QuadraticTo(ctrl, to) => {
                self.curve_to(ctrl, to);
            }
            PathEvent::CubicTo(_, ctrl2, to) => {
                self.curve_to(ctrl2, to);
            }
            PathEvent::Arc(center, radii, sweep_angle, x_rotation) => {
                let start_angle = (self.current - center).angle_from_x_axis() - x_rotation;
                let arc = Arc {
                    center,
                    radii,
                    start_angle,
                    sweep_angle,
                    x_rotation,
                };
                let to = arc.to();
                let ctrl = to - arc.sample_tangent(1.0);
                self.curve_to(ctrl, to);
            }
            PathEvent::Close => {
                self.close();
            }
        }
    }

    pub fn flattened_event(&mut self, event: FlattenedEvent) {
        match event {
            FlattenedEvent::MoveTo(to) => {
                self.move_to(to);
            }
            FlattenedEvent::LineTo(to) => {
                self.line_to(to);
            }
            FlattenedEvent::Close => {
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

    pub fn next(&mut self, to: Point) { self.current = to; }

    pub fn relative_next(&mut self, to: Vector) { self.current = self.relative_to_absolute(to); }

    pub fn get_smooth_ctrl(&self) -> Point { self.current + (self.current - self.last_ctrl) }

    pub fn relative_to_absolute(&self, v: Vector) -> Point { self.current + v }

    pub fn svg_to_path_event(&self, event: SvgEvent) -> PathEvent {
        match event {
            SvgEvent::MoveTo(to) => PathEvent::MoveTo(to),
            SvgEvent::LineTo(to) => PathEvent::LineTo(to),
            SvgEvent::QuadraticTo(ctrl, to) => PathEvent::QuadraticTo(ctrl, to),
            SvgEvent::CubicTo(ctrl1, ctrl2, to) => PathEvent::CubicTo(ctrl1, ctrl2, to),
            SvgEvent::Close => PathEvent::Close,
            SvgEvent::RelativeMoveTo(to) => PathEvent::MoveTo(self.relative_to_absolute(to)),
            SvgEvent::RelativeLineTo(to) => PathEvent::LineTo(self.relative_to_absolute(to)),
            SvgEvent::RelativeQuadraticTo(ctrl, to) => {
                PathEvent::QuadraticTo(self.relative_to_absolute(ctrl), self.relative_to_absolute(to))
            }
            SvgEvent::RelativeCubicTo(ctrl1, ctrl2, to) => {
                PathEvent::CubicTo(
                    self.relative_to_absolute(ctrl1),
                    self.relative_to_absolute(ctrl2),
                    self.relative_to_absolute(to),
                )
            }
            SvgEvent::HorizontalLineTo(x) => {
                PathEvent::LineTo(point(x, self.current.y))
            }
            SvgEvent::VerticalLineTo(y) => PathEvent::LineTo(point(self.current.x, y)),
            SvgEvent::RelativeHorizontalLineTo(x) => {
                PathEvent::LineTo(point(self.current.x + x, self.current.y))
            }
            SvgEvent::RelativeVerticalLineTo(y) => {
                PathEvent::LineTo(point(self.current.x, self.current.y + y))
            }
            SvgEvent::SmoothQuadraticTo(to) => {
                PathEvent::QuadraticTo(self.get_smooth_ctrl(), to)
            }
            SvgEvent::SmoothCubicTo(ctrl2, to) => {
                PathEvent::CubicTo(self.get_smooth_ctrl(), ctrl2, to)
            }
            SvgEvent::SmoothRelativeQuadraticTo(to) => {
                PathEvent::QuadraticTo(self.get_smooth_ctrl(), self.relative_to_absolute(to))
            }
            SvgEvent::SmoothRelativeCubicTo(ctrl2, to) => {
                PathEvent::CubicTo(
                    self.get_smooth_ctrl(),
                    self.relative_to_absolute(ctrl2),
                    self.relative_to_absolute(to),
                )
            }
            SvgEvent::ArcTo(radii, x_rotation, flags, to) => {
                let arc = Arc::from_svg_arc(&SvgArc {
                    from: self.current,
                    to,
                    radii,
                    x_rotation,
                    flags,
                });
                PathEvent::Arc(
                    arc.center,
                    arc.radii,
                    arc.sweep_angle,
                    arc.x_rotation,
                )
            }
            SvgEvent::RelativeArcTo(radii, x_rotation, flags, to) => {
                let arc = Arc::from_svg_arc(&SvgArc {
                    from: self.current,
                    to: self.current + to,
                    radii,
                    x_rotation,
                    flags,
                });
                PathEvent::Arc(
                    arc.center,
                    arc.radii,
                    arc.sweep_angle,
                    arc.x_rotation,
                )
            }
        }
    }
}
