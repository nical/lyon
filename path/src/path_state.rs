
use math::{Point, Vector, point, vector, Angle};
use geom::{Arc, SvgArc, ArcFlags};
use events::{PathEvent, SvgEvent};
use builder::{FlatPathBuilder, PathBuilder, SvgBuilder};

#[derive(Copy, Clone, Debug, PartialEq)]
enum LastCtrl {
    Cubic(Point),
    Quad(Point),
    None,
}

/// Represents the current state of a path while it is being built.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PathState {
    /// The current point.
    current: Point,
    /// The first point of the current sub-path.
    first: Point,
    /// The last control point.
    last_ctrl: LastCtrl,
}

impl PathState {
    pub fn new() -> Self {
        PathState {
            current: point(0.0, 0.0),
            first: point(0.0, 0.0),
            last_ctrl: LastCtrl::None,
        }
    }
}

impl FlatPathBuilder for PathState {
    type PathType = PathState;

    fn move_to(&mut self, to: Point) { self.move_to(to); }

    fn line_to(&mut self, to: Point) { self.line_to(to); }

    fn close(&mut self) { self.close(); }

    fn build(self) -> PathState { self }

    fn build_and_reset(&mut self) -> PathState {
        let result = self.clone();
        *self = PathState::new();
        result
    }

    fn current_position(&self) -> Point {
        self.current
    }
}

impl PathBuilder for PathState {
    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
        self.last_ctrl = LastCtrl::Quad(ctrl);
        self.current = to;
    }

    fn cubic_bezier_to(&mut self, _ctrl1: Point, ctrl2: Point, to: Point) {
        self.last_ctrl = LastCtrl::Cubic(ctrl2);
        self.current = to;
    }

    fn arc(&mut self, center: Point, radii: Vector, sweep_angle: Angle, x_rotation: Angle) {
        let start_angle = (self.current - center).angle_from_x_axis() - x_rotation;
        let arc = Arc {
            center,
            radii,
            start_angle,
            sweep_angle,
            x_rotation,
        };
        let to = arc.to();
        self.last_ctrl = LastCtrl::None;
        self.current = to;
    }
}

impl SvgBuilder for PathState {
    fn relative_move_to(&mut self, to: Vector) {
        let to = self.relative_to_absolute(to);
        self.move_to(to);
    }
    fn relative_line_to(&mut self, to: Vector) {
        let to = self.relative_to_absolute(to);
        self.line_to(to);
    }
    fn relative_quadratic_bezier_to(&mut self, ctrl: Vector, to: Vector) {
        let to = self.relative_to_absolute(to);
        let ctrl = self.relative_to_absolute(ctrl);
        self.last_ctrl = LastCtrl::Quad(ctrl);
        self.current = to;
    }
    fn relative_cubic_bezier_to(&mut self, _ctrl1: Vector, ctrl2: Vector, to: Vector) {
        let to = self.relative_to_absolute(to);
        let ctrl2 = self.relative_to_absolute(ctrl2);
        self.last_ctrl = LastCtrl::Cubic(ctrl2);
        self.current = to;
    }
    fn smooth_cubic_bezier_to(&mut self, ctrl2: Point, to: Point) {
        self.last_ctrl = LastCtrl::Cubic(ctrl2);
        self.current = to;
    }
    fn smooth_relative_cubic_bezier_to(&mut self, ctrl2: Vector, to: Vector) {
        self.last_ctrl = LastCtrl::Cubic(self.relative_to_absolute(ctrl2));
        self.current = self.relative_to_absolute(to);
    }
    fn smooth_quadratic_bezier_to(&mut self, to: Point) {
        let last_ctrl = match self.last_ctrl {
            LastCtrl::Quad(ctrl) => ctrl,
            _ => self.current,
        };
        self.last_ctrl = LastCtrl::Quad(to + (to - last_ctrl));
        self.current = to;
    }
    fn smooth_relative_quadratic_bezier_to(&mut self, to: Vector) {
        let to = self.relative_to_absolute(to);
        let last_ctrl = match self.last_ctrl {
            LastCtrl::Quad(ctrl) => ctrl,
            _ => self.current,
        };
        self.last_ctrl = LastCtrl::Quad(to + (to - last_ctrl));
        self.current = to;
    }
    fn horizontal_line_to(&mut self, x: f32) {
        let to = point(x, self.current.y);
        self.line_to(to);
    }
    fn relative_horizontal_line_to(&mut self, dx: f32) {
        let to = self.current + vector(dx, 0.0);
        self.line_to(to);
    }
    fn vertical_line_to(&mut self, y: f32) {
        let to = point(self.current.x, y);
        self.line_to(to);
    }
    fn relative_vertical_line_to(&mut self, dy: f32) {
        let to = self.current + vector(0.0, dy);
        self.line_to(to);
    }
    fn arc_to(&mut self, _radii: Vector, _x_rotation: Angle, _flags: ArcFlags, to: Point) {
        self.last_ctrl = LastCtrl::None;
        self.current = to;
    }
    fn relative_arc_to(
        &mut self,
        _radii: Vector,
        _x_rotation: Angle,
        _flags: ArcFlags,
        to: Vector,
    ) {
        let to = self.relative_to_absolute(to);
        self.last_ctrl = LastCtrl::None;
        self.current = to;
    }
}

impl PathState {
    pub fn move_to(&mut self, to: Point) {
        self.last_ctrl = LastCtrl::None;
        self.current = to;
        self.first = to;
    }

    pub fn line_to(&mut self, to: Point) {
        self.last_ctrl = LastCtrl::None;
        self.current = to;
    }

    pub fn close(&mut self) {
        self.last_ctrl = LastCtrl::None;
        self.current = self.first;
    }

    pub fn next(&mut self, to: Point) { self.current = to; }

    pub fn relative_next(&mut self, to: Vector) { self.current = self.relative_to_absolute(to); }

    pub fn get_smooth_cubic_ctrl(&self) -> Point {
        match self.last_ctrl {
            LastCtrl::Cubic(ctrl) => self.current + (self.current - ctrl),
            _ => self.current,
        }
    }

    pub fn get_smooth_quadratic_ctrl(&self) -> Point {
        match self.last_ctrl {
            LastCtrl::Quad(ctrl) => self.current + (self.current - ctrl),
            _ => self.current,
        }
    }

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
                PathEvent::QuadraticTo(self.get_smooth_quadratic_ctrl(), to)
            }
            SvgEvent::SmoothCubicTo(ctrl2, to) => {
                PathEvent::CubicTo(self.get_smooth_cubic_ctrl(), ctrl2, to)
            }
            SvgEvent::SmoothRelativeQuadraticTo(to) => {
                PathEvent::QuadraticTo(self.get_smooth_quadratic_ctrl(), self.relative_to_absolute(to))
            }
            SvgEvent::SmoothRelativeCubicTo(ctrl2, to) => {
                PathEvent::CubicTo(
                    self.get_smooth_cubic_ctrl(),
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
