use path_builder::*;
use core::ArcFlags;
use core::math::{Vec2, Point, Radians};
use std::f32::consts::PI;
use std::mem;

/// A `PathBuilder` that builds a `String` representation of the path
/// using the SVG syntax.
///
/// No effort is put into making the serializer performant or make the
/// output compact. Intended primarily for debugging purposes.
pub struct PathSerializer {
    path: String,
}

impl PathSerializer {
    pub fn new() -> Self {
        PathSerializer {
            path: String::new()
        }
    }
}

impl BaseBuilder for PathSerializer {
    type PathType = String;

    fn move_to(&mut self, to: Point) {
        self.path += &format!("M {} {} ", to.x, to.y);
    }

    fn line_to(&mut self, to: Point) {
        self.path += &format!("L {} {} ", to.x, to.y);
    }

    fn close(&mut self) {
        self.path.push_str("Z");
    }

    fn build(self) -> String { self.path }

    fn build_and_reset(&mut self) -> String {
        mem::replace(&mut self.path, String::new())
    }

    fn current_position(&self) -> Point {
        unimplemented!();
    }
}

impl PathBuilder for PathSerializer {
    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
        self.path += &format!("Q {} {} {} {}", ctrl.x, ctrl.y, to.x, to.y);
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
        self.path += &format!("C {} {} {} {} {} {}", ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y);
    }
}

impl SvgBuilder for PathSerializer {

    fn relative_move_to(&mut self, to: Vec2) {
        self.path += &format!("m {} {} ", to.x, to.y);
    }

    fn relative_line_to(&mut self, to: Vec2) {
        self.path += &format!("l {} {} ", to.x, to.y);
    }

    fn relative_quadratic_bezier_to(&mut self, ctrl: Vec2, to: Vec2) {
        self.path += &format!("q {} {} {} {}", ctrl.x, ctrl.y, to.x, to.y);
    }

    fn relative_cubic_bezier_to(&mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) {
        self.path += &format!("c {} {} {} {} {} {}", ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y);
    }

    fn smooth_cubic_bezier_to(&mut self, ctrl2: Point, to: Point) {
        self.path += &format!("S {} {} {} {}", ctrl2.x, ctrl2.y, to.x, to.y);
    }

    fn smooth_relative_cubic_bezier_to(&mut self, ctrl2: Vec2, to: Vec2) {
        self.path += &format!("s {} {} {} {}", ctrl2.x, ctrl2.y, to.x, to.y);
    }

    fn smooth_quadratic_bezier_to(&mut self, to: Point) {
        self.path += &format!("T {} {} ", to.x, to.y);
    }

    fn smooth_relative_quadratic_bezier_to(&mut self, to: Vec2) {
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
        radii: Vec2,
        x_rotation: Radians<f32>,
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
        radii: Vec2,
        x_rotation: Radians<f32>,
        flags: ArcFlags,
        to: Vec2,
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
