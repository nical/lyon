use crate::parser::xmlparser::FromSpan;
use crate::parser::path::{Tokenizer, Token};

use crate::path::geom::Arc;
use crate::path::math::{Vector, vector, Point, point, Angle};
use crate::path::{ArcFlags};
use crate::path::builder::*;

use std::f32::consts::PI;
use std::mem;

#[derive(Clone, Debug, PartialEq)]
pub struct ParseError;

/// Builds path object using an SvgBuilder and a list of commands.
/// Once the path is built you can tessellate it.
///
/// The [SvgBuilder](trait.SvgBuilder.html) Adds to [PathBuilder](traits.PathBuilder.html)
/// the rest of the [SVG path](https://svgwg.org/specs/paths/) commands.
///
/// # Examples
///
/// ```
/// # extern crate lyon_svg as svg;
/// # extern crate lyon_path;
/// # use lyon_path::Path;
/// # use svg::path_utils::build_path;
/// # fn main() {
/// // Create a simple path.
/// let commands = &"M 0 0 L 10 0 L 10 10 L 0 10 z";
/// let svg_builder = Path::builder().with_svg();
/// let path = build_path(svg_builder, commands);
/// # }
/// ```
pub fn build_path<Builder>(mut builder: Builder, src: &str) -> Result<Builder::PathType, ParseError>
where
    Builder: SvgBuilder + Build
{
    for item in Tokenizer::from_str(src) {
        svg_event(&item, &mut builder);
    }

    Ok(builder.build())
}

fn svg_event<Builder>(token: &Token, builder: &mut Builder)
where Builder: SvgBuilder {
    fn vec2(x: f64, y: f64) -> Vector { vector(x as f32, y as f32) }
    fn point2(x: f64, y: f64) -> Point { point(x as f32, y as f32) }
    match *token {
        Token::MoveTo { abs: true, x, y } => {
            builder.move_to(point2(x, y));
        }
        Token::MoveTo { abs: false, x, y } => {
            builder.relative_move_to(vec2(x, y));
        }
        Token::LineTo { abs: true, x, y } => {
            builder.line_to(point2(x, y));
        }
        Token::LineTo { abs: false, x, y } => {
            builder.relative_line_to(vec2(x, y));
        }
        Token::HorizontalLineTo { abs: true, x } => {
            builder.horizontal_line_to(x as f32);
        }
        Token::HorizontalLineTo { abs: false, x } => {
            builder.relative_horizontal_line_to(x as f32);
        }
        Token::VerticalLineTo { abs: true, y } => {
            builder.vertical_line_to(y as f32);
        }
        Token::VerticalLineTo { abs: false, y } => {
            builder.relative_vertical_line_to(y as f32);
        }
        Token::CurveTo { abs: true, x1, y1, x2, y2, x, y } => {
            builder.cubic_bezier_to(point2(x1, y1), point2(x2, y2), point2(x, y));
        }
        Token::CurveTo { abs: false, x1, y1, x2, y2, x, y } => {
            builder.relative_cubic_bezier_to(vec2(x1, y1), vec2(x2, y2), vec2(x, y));
        }
        Token::SmoothCurveTo { abs: true, x2, y2, x, y } => {
            builder.smooth_cubic_bezier_to(point2(x2, y2), point2(x, y));
        }
        Token::SmoothCurveTo { abs: false, x2, y2, x, y } => {
            builder.smooth_relative_cubic_bezier_to(vec2(x2, y2), vec2(x, y));
        }
        Token::Quadratic { abs: true, x1, y1, x, y } => {
            builder.quadratic_bezier_to(point2(x1, y1), point2(x, y));
        }
        Token::Quadratic { abs: false, x1, y1, x, y } => {
            builder.relative_quadratic_bezier_to(vec2(x1, y1), vec2(x, y));
        }
        Token::SmoothQuadratic { abs: true, x, y } => {
            builder.smooth_quadratic_bezier_to(point2(x, y));
        }
        Token::SmoothQuadratic { abs: false, x, y } => {
            builder.smooth_relative_quadratic_bezier_to(vec2(x, y));
        }
        Token::EllipticalArc { abs: true, rx, ry, x_axis_rotation, large_arc, sweep, x, y } => {
            builder.arc_to(
                vec2(rx, ry),
                Angle::degrees(x_axis_rotation as f32),
                ArcFlags { large_arc: large_arc, sweep: sweep },
                point2(x, y),
            );
        }
        Token::EllipticalArc { abs: false, rx, ry, x_axis_rotation, large_arc, sweep, x, y } => {
            builder.relative_arc_to(
                vec2(rx, ry),
                Angle::degrees(x_axis_rotation as f32),
                ArcFlags { large_arc: large_arc, sweep: sweep },
                vec2(x, y),
            );
        }
        Token::ClosePath { .. } => { builder.close(); },
    }
}


/// A `PathBuilder` that builds a `String` representation of the path
/// using the SVG syntax.
///
/// No effort is put into making the serializer fast or make the
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

impl Build for PathSerializer {
    type PathType = String;

    fn build(self) -> String { self.path }

    fn build_and_reset(&mut self) -> String {
        self.current = point(0.0, 0.0);
        mem::replace(&mut self.path, String::new())
    }
}

impl FlatPathBuilder for PathSerializer {
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
        sweep_angle: Angle,
        x_rotation: Angle
    ) {
        let start_angle = (self.current - center).angle_from_x_axis() - x_rotation;
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
        x_rotation: Angle,
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
        x_rotation: Angle,
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

impl PolygonBuilder for PathSerializer {
    fn polygon(&mut self, points: &[Point]) {
        build_polygon(self, points);
    }
}
