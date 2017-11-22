use path::builder::*;
use path::ArcFlags;
use core::math::{Vector, Point, Radians, point};
use std::f32::consts::PI;
use std::mem;
use path::bezier::Arc;
use path::bezier::utils::vector_angle;

/// A `PathBuilder` that builds a `String` representation of the path
/// using the SVG syntax.
///
/// No effort is put into making the serializer performant or make the
/// output compact. Intended primarily for debugging purposes.
pub struct PathSerializer {
    path: String,
    current: Point,
}

impl PathSerializer {
    pub fn new() -> Self {
        PathSerializer {
            path: String::new(),
            current: point(0.0, 0.0),
        }
    }
}

impl FlatPathBuilder for PathSerializer {
    type PathType = String;

    fn move_to(&mut self, to: Point) {
        self.path += &format!("M {} {} ", to.x, to.y);
        self.current = to;
    }

    fn line_to(&mut self, to: Point) {
        self.path += &format!("L {} {} ", to.x, to.y);
        self.current = to;
    }

    fn close(&mut self) {
        self.path.push_str("Z");
    }

    fn build(self) -> String { self.path }

    fn build_and_reset(&mut self) -> String {
        self.current = point(0.0, 0.0);
        mem::replace(&mut self.path, String::new())
    }

    fn current_position(&self) -> Point {
        self.current
    }
}

impl PathBuilder for PathSerializer {
    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
        self.path += &format!("Q {} {} {} {}", ctrl.x, ctrl.y, to.x, to.y);
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
        self.path += &format!("C {} {} {} {} {} {}", ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y);
    }

    fn arc(
        &mut self,
        center: Point,
        radii: Vector,
        sweep_angle: Radians,
        x_rotation: Radians
    ) {
        let start_angle = vector_angle(self.current - center);
        let svg = Arc {
            center, radii, start_angle, sweep_angle, x_rotation
        }.to_svg_arc();
        self.path += &format!(
            "A {} {} {} {} {} {} {}",
            radii.x, radii.y, svg.x_rotation.get(),
            svg.flags.large_arc, svg.flags.sweep,
            svg.to.x, svg.to.y
        );
    }
}

impl SvgBuilder for PathSerializer {

    fn relative_move_to(&mut self, to: Vector) {
        self.path += &format!("m {} {} ", to.x, to.y);
    }

    fn relative_line_to(&mut self, to: Vector) {
        self.path += &format!("l {} {} ", to.x, to.y);
    }

    fn relative_quadratic_bezier_to(&mut self, ctrl: Vector, to: Vector) {
        self.path += &format!("q {} {} {} {}", ctrl.x, ctrl.y, to.x, to.y);
    }

    fn relative_cubic_bezier_to(&mut self, ctrl1: Vector, ctrl2: Vector, to: Vector) {
        self.path += &format!("c {} {} {} {} {} {}", ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y);
    }

    fn smooth_cubic_bezier_to(&mut self, ctrl2: Point, to: Point) {
        self.path += &format!("S {} {} {} {}", ctrl2.x, ctrl2.y, to.x, to.y);
    }

    fn smooth_relative_cubic_bezier_to(&mut self, ctrl2: Vector, to: Vector) {
        self.path += &format!("s {} {} {} {}", ctrl2.x, ctrl2.y, to.x, to.y);
    }

    fn smooth_quadratic_bezier_to(&mut self, to: Point) {
        self.path += &format!("T {} {} ", to.x, to.y);
    }

    fn smooth_relative_quadratic_bezier_to(&mut self, to: Vector) {
        self.path += &format!("t {} {} ", to.x, to.y);
    }

    fn horizontal_line_to(&mut self, x: f32) {
        self.path += &format!("H {} ", x);
    }

    fn relative_horizontal_line_to(&mut self, dx: f32) {
        self.path += &format!("h {} ", dx);
    }

    fn vertical_line_to(&mut self, y: f32) {
        self.path += &format!("V {} ", y);
    }

    fn relative_vertical_line_to(&mut self, dy: f32) {
        self.path += &format!("v {} ", dy);
    }

    fn arc_to(
        &mut self,
        radii: Vector,
        x_rotation: Radians,
        flags: ArcFlags,
        to: Point
    ) {
        self.path += &format!(
            "A {} {} {} {} {} {} {} ",
            radii.x, radii.y, x_rotation.get() * 180.0 / PI,
            if flags.large_arc { 1u32 } else { 0 },
            if flags.sweep { 1u32 } else { 0 },
            to.x, to.y
        );
    }

    fn relative_arc_to(
        &mut self,
        radii: Vector,
        x_rotation: Radians,
        flags: ArcFlags,
        to: Vector,
    ) {
        self.path += &format!(
            "a {} {} {} {} {} {} {} ",
            radii.x, radii.y, x_rotation.get() * 180.0 / PI,
            if flags.large_arc { 1u32 } else { 0 },
            if flags.sweep { 1u32 } else { 0 },
            to.x, to.y
        );
    }
}
