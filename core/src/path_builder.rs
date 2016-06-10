use path::*;
use bezier::*;
use std::f32::*;
use vodk_math::{ Vector2D };
use math_utils::*;

use super::{
    vertex_id_range,
//    crash,
};

use vodk_math::{ Vec2, vec2, Rect };

/// Builder for paths that can contain lines, and quadratic/cubic bezier segments.
pub type BezierPathBuilder = SvgPathBuilder<PrimitiveImpl>;

/// Builder for flattened paths
pub type FlattenedPathBuilder = SvgPathBuilder<FlattenedBuilder<PrimitiveImpl>>;

/// FlattenedPathBuilder constructor.
pub fn flattened_path_builder() -> FlattenedPathBuilder {
    SvgPathBuilder::from_builder(FlattenedBuilder::new(PrimitiveImpl::new(),0.05))
}

/// BezierPathBuilder constructor.
pub fn bezier_path_builder() -> BezierPathBuilder {
    SvgPathBuilder::from_builder(PrimitiveImpl::new())
}

/// The base path building interface. More elaborate interfaces are built on top
/// of the provided primitives.
pub trait PrimitiveBuilder {
    type PathType;

    fn move_to(&mut self, to: Vec2);
    fn line_to(&mut self, to: Vec2);
    fn quadratic_bezier_to(&mut self, ctrl: Vec2, to: Vec2);
    fn cubic_bezier_to(&mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2);
    fn end(&mut self) -> PathId;
    fn close(&mut self) -> PathId;
    fn current_position(&self) -> Vec2;

    fn build(self) -> Self::PathType;
}

/// A path building interface that tries to stay close to SVG's path specification.
/// https://svgwg.org/specs/paths/
pub trait SvgBuilder : PrimitiveBuilder {
    fn relative_move_to(&mut self, to: Vec2);
    fn relative_line_to(&mut self, to: Vec2);
    fn relative_quadratic_bezier_to(&mut self, ctrl: Vec2, to: Vec2);
    fn relative_cubic_bezier_to(&mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2);
    fn cubic_bezier_smooth_to(&mut self, ctrl2: Vec2, to: Vec2);
    fn relative_cubic_bezier_smooth_to(&mut self, ctrl2: Vec2, to: Vec2);
    fn quadratic_bezier_smooth_to(&mut self, to: Vec2);
    fn relative_quadratic_bezier_smooth_to(&mut self, to: Vec2);
    fn horizontal_line_to(&mut self, x: f32);
    fn relative_horizontal_line_to(&mut self, dx: f32);
    fn vertical_line_to(&mut self, y: f32);
    fn relative_vertical_line_to(&mut self, dy: f32);
    // TODO: Would it be better to use an api closer to cairo/skia for arcs?
    fn arc_to(&mut self, to: Vec2, radii: Vec2, x_rotation: f32, flags: ArcFlags);
}

pub trait PolygonBuilder {
    fn polygon(&mut self, points: &[Vec2]) -> PathId;
}

#[derive(Copy, Clone, Debug)]
pub struct ArcFlags {
    pub large_arc: bool,
    pub sweep: bool,
}

/// Implements the Svg building interface on top of the a primitive builder.
pub struct SvgPathBuilder<Builder: PrimitiveBuilder> {
    builder: Builder,
    last_ctrl: Vec2,
}

impl<Builder: PrimitiveBuilder> SvgPathBuilder<Builder> {
    pub fn from_builder(builder: Builder) -> SvgPathBuilder<Builder> {
        SvgPathBuilder {
            builder: builder,
            last_ctrl: vec2(0.0, 0.0),
        }
    }

    /// Draw an elliptical arc between two points with a sweep_angle <= Pi/2
    /// current_position and to are points of the ellipse when aligned with the origin axis !!
    /// which means that they are the rotation of the original starting and ending point
    /// x_rotation_radian is in radian
    fn sub_arc(&mut self, to: Vec2, start_angle: f32, sweep_angle: f32, radii: Vec2, x_rotation_radian: f32) {

        let alpha = sweep_angle.sin()* ( ((4.0 + 3.0*(sweep_angle/2.0).tan().powi(2)).sqrt() - 1.0) /3.0);
        let end_angle = start_angle + sweep_angle;

        let ctrl_point_1 : Vec2 = Vec2::new(
            (self.current_position().x + alpha * (- radii.x *  x_rotation_radian.cos() * start_angle.sin() - radii.y * x_rotation_radian.sin() * start_angle.cos())).round(),
            (self.current_position().y + alpha * (- radii.x *  x_rotation_radian.sin() * start_angle.sin() + radii.y * x_rotation_radian.cos() * start_angle.cos())).round()
        );

        let ctrl_point_2 : Vec2 = Vec2::new(
            (to.x - alpha * (- radii.x *  x_rotation_radian.cos() * end_angle.sin() - radii.y * x_rotation_radian.sin() * end_angle.cos())).round(),
            (to.y - alpha * (- radii.x *  x_rotation_radian.sin() * end_angle.sin() + radii.y * x_rotation_radian.cos() * end_angle.cos())).round()
        );

        self.cubic_bezier_to(ctrl_point_1, ctrl_point_2, to);
    }

    fn find_center(&mut self, radii: Vec2, point: Vec2, flags: ArcFlags) -> Vec2{
        let center_num = radii.x.powi(2) * radii.y.powi(2)
                       - radii.x.powi(2) * point.y.powi(2)
                       - radii.y.powi(2) * point.x.powi(2);

        let center_denom = radii.x.powi(2) * point.y.powi(2)
                         + radii.y.powi(2) * point.x.powi(2);

        let mut center_coef = center_num / center_denom;
        if center_coef < 0.0 {
            center_coef = 0.0;
        }

        if flags.large_arc == flags.sweep {
            center_coef = - center_coef.sqrt();
        } else {
            center_coef = center_coef.sqrt();
        }

        return vec2(
            center_coef * radii.x * point.y / radii.y,
            center_coef * -radii.y * point.x / radii.x
        )
    }
}


fn radii_to_scale(radii: Vec2, point: Vec2) -> Vec2 {
    let mut radii_to_scale = point.x.powi(2)/radii.x.powi(2) + point.y.powi(2)/radii.y.powi(2);
    if radii_to_scale > 1.0 {
        radii_to_scale = radii_to_scale.sqrt();
    } else {
        radii_to_scale = 1.0;
    }
    vec2(radii_to_scale*radii.x.abs(), radii_to_scale*radii.y.abs())
}

impl<Builder: PrimitiveBuilder> PrimitiveBuilder for SvgPathBuilder<Builder> {
    type PathType = Builder::PathType;

    fn move_to(&mut self, to: Vec2) {
        self.last_ctrl = to;
        self.builder.move_to(to);
    }

    fn line_to(&mut self, to: Vec2) {
        self.last_ctrl = self.current_position();
        self.builder.line_to(to);
    }

    fn quadratic_bezier_to(&mut self, ctrl: Vec2, to: Vec2) {
        self.last_ctrl = ctrl;
        self.builder.quadratic_bezier_to(ctrl, to);
    }

    fn cubic_bezier_to(&mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) {
        self.last_ctrl = ctrl2;
        self.builder.cubic_bezier_to(ctrl1, ctrl2, to);
    }

    fn end(&mut self) -> PathId {
        self.last_ctrl = vec2(0.0, 0.0);
        self.builder.end()
    }

    fn close(&mut self) -> PathId {
        self.last_ctrl = vec2(0.0, 0.0);
        self.builder.close()
    }

    fn current_position(&self) -> Vec2 {
        self.builder.current_position()
    }

    fn build(self) -> Builder::PathType { self.builder.build() }
}

impl<Builder: PrimitiveBuilder> SvgBuilder for SvgPathBuilder<Builder> {
    fn relative_move_to(&mut self, to: Vec2) {
        let offset = self.builder.current_position();
        self.move_to(offset + to);
    }

    fn relative_line_to(&mut self, to: Vec2) {
        let offset = self.builder.current_position();
        self.line_to(offset + to);
    }

    fn relative_quadratic_bezier_to(&mut self, ctrl: Vec2, to: Vec2) {
        let offset = self.builder.current_position();
        self.quadratic_bezier_to(ctrl + offset, to + offset);
    }

    fn relative_cubic_bezier_to(&mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) {
        let offset = self.builder.current_position();
        self.cubic_bezier_to(ctrl1 + offset, ctrl2 + offset, to + offset);
    }

    fn cubic_bezier_smooth_to(&mut self, ctrl2: Vec2, to: Vec2) {
        let ctrl = self.builder.current_position() + (self.builder.current_position() - self.last_ctrl);
        self.cubic_bezier_to(ctrl, ctrl2, to);
    }

    fn relative_cubic_bezier_smooth_to(&mut self, ctrl2: Vec2, to: Vec2) {
        let ctrl = self.builder.current_position() - self.last_ctrl;
        self.relative_cubic_bezier_to(ctrl, ctrl2, to);
    }

    fn quadratic_bezier_smooth_to(&mut self, to: Vec2) {
        let ctrl = self.builder.current_position() + (self.builder.current_position() - self.last_ctrl);
        self.quadratic_bezier_to(ctrl, to);
    }

    fn relative_quadratic_bezier_smooth_to(&mut self, to: Vec2) {
        let ctrl = self.builder.current_position() - self.last_ctrl;
        self.relative_quadratic_bezier_to(ctrl, to);
    }

    fn horizontal_line_to(&mut self, x: f32) {
        let y = self.builder.current_position().y;
        self.line_to(vec2(x, y));
    }

    fn relative_horizontal_line_to(&mut self, dx: f32) {
        let p = self.builder.current_position();
        self.line_to(vec2(p.x + dx, p.y));
    }

    fn vertical_line_to(&mut self, y: f32) {
        let x = self.builder.current_position().x;
        self.line_to(vec2(x, y));
    }

    fn relative_vertical_line_to(&mut self, dy: f32) {
        let p = self.builder.current_position();
        self.line_to(vec2(p.x, p.y + dy));
    }

    // x_rotation in radian
    fn arc_to(&mut self, to: Vec2, radii: Vec2, x_rotation: f32, flags: ArcFlags) {

        // If end and starting point are identical, then there is not ellipse to be drawn
        if self.current_position().x == to.x && self.current_position().y == to.y {
            return;
        }

        if radii.x == 0.0 && radii.y == 0.0 {
            self.line_to(to) ;
        }

        let x_axis_rotation = x_rotation % (2.0*consts::PI);
        let from : Vec2 = self.current_position();

        // Middle point between start and end point
        let dx = (from.x - to.x) / 2.0;
        let dy = (from.y - to.y) / 2.0;
        let transformed_point : Vec2 =  Vec2::new(
            (x_axis_rotation.cos() *  dx + x_axis_rotation.sin() * dy ).round(),
            (- x_axis_rotation.sin() * dx + x_axis_rotation.cos() * dy).round()
        );

        let scaled_radii = radii_to_scale(radii, transformed_point);
        let transformed_center : Vec2 =  self.find_center(scaled_radii, transformed_point, flags);

        // Start, end and sweep angles
        let start_vector : Vec2 = ellipse_center_to_point(transformed_center,
                                                          transformed_point,
                                                          scaled_radii
                                                          );
        let mut start_angle = angle_between(Vector2D::new(1.0, 0.0), start_vector);

        let end_vector : Vec2 = ellipse_center_to_point(transformed_center,
                                                        vec2(-transformed_point.x, -transformed_point.y),
                                                        scaled_radii
                                                        );
        let mut end_angle = angle_between(Vector2D::new(1.0, 0.0), end_vector);

        let mut sweep_angle = end_angle - start_angle;

        // Affect the flags value to get the right arc among the 4 possible
        if !flags.sweep && sweep_angle > 0.0 {
            sweep_angle =  sweep_angle  - 2.0*consts::PI;
        } else if flags.sweep && sweep_angle < 0.0 {
            sweep_angle = sweep_angle + 2.0*consts::PI;
        }
        sweep_angle %= 2.0*consts::PI;

        // Break down the arc into smaller ones of maximum PI/2 angle from point to point
        while sweep_angle.abs() > consts::FRAC_PI_2 {
            // compute crossing-points
            end_angle = start_angle + sweep_angle.signum() * consts::FRAC_PI_2;

            let mut crossing_point = ellipse_point_from_angle(transformed_center, scaled_radii, end_angle);

            crossing_point = Vec2::new(
                x_axis_rotation.cos()*crossing_point.x - x_axis_rotation.sin() * crossing_point.y + (from.x + to.x) /2.0,
                x_axis_rotation.sin()*crossing_point.x + x_axis_rotation.cos() * crossing_point.y + (from.y + to.y) /2.0
            );

            self.sub_arc(
                crossing_point,
                start_angle,
                sweep_angle.signum() * consts::FRAC_PI_2,
                scaled_radii,
                x_axis_rotation
            );

            if sweep_angle > 0.0 {
                sweep_angle -= consts::FRAC_PI_2;
                start_angle += consts::FRAC_PI_2;
            } else {
                sweep_angle += consts::FRAC_PI_2;
                start_angle -= consts::FRAC_PI_2;
            }
        }
        self.sub_arc(to,start_angle, sweep_angle, scaled_radii, x_axis_rotation);
    }
}

/// Generates flattened paths
pub struct FlattenedBuilder<Builder> {
    builder: Builder,
    tolerance: f32,
}

/// Generates path objects with bezier segments
pub struct PrimitiveImpl {
    vertices: Vec<PointData>,
    path_info: Vec<PathInfo>,
    last_position: Vec2,
    top_left: Vec2,
    bottom_right: Vec2,
    offset: u16,
    // flags
    building: bool,
}

impl<Builder: PrimitiveBuilder> PrimitiveBuilder for FlattenedBuilder<Builder> {
    type PathType = Builder::PathType;

    fn move_to(&mut self, to: Vec2) { self.builder.move_to(to); }

    fn line_to(&mut self, to: Vec2) { self.builder.line_to(to); }

    fn quadratic_bezier_to(&mut self, ctrl: Vec2, to: Vec2) {
        let from = self.current_position();
        let cubic = QuadraticBezierSegment { from: from, cp: ctrl, to: to }.to_cubic();
        flatten_cubic_bezier(cubic, self.tolerance, self);
    }

    fn cubic_bezier_to(&mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) {
        flatten_cubic_bezier(
            CubicBezierSegment{
                from: self.current_position(),
                cp1: ctrl1,
                cp2: ctrl2,
                to: to,
            },
            self.tolerance,
            self
        );
    }

    fn end(&mut self) -> PathId { self.builder.end() }

    fn close(&mut self) -> PathId { self.builder.close() }

    fn current_position(&self) -> Vec2 { self.builder.current_position() }

    fn build(self) -> Builder::PathType { self.builder.build() }
}

impl PrimitiveBuilder for PrimitiveImpl {
    type PathType = Path;

    fn move_to(&mut self, to: Vec2)
    {
        if self.building {
            self.end_sub_path(false);
        }
        self.top_left = to;
        self.bottom_right = to;
        self.push(to, PointType::Normal);
    }

    fn line_to(&mut self, to: Vec2) {
        self.push(to, PointType::Normal);
    }

    fn quadratic_bezier_to(&mut self, ctrl: Vec2, to: Vec2) {
        self.push(ctrl, PointType::Control);
        self.push(to, PointType::Normal);
    }

    fn cubic_bezier_to(&mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) {
        self.push(ctrl1, PointType::Control);
        self.push(ctrl2, PointType::Control);
        self.push(to, PointType::Normal);
    }

    fn end(&mut self) -> PathId { self.end_sub_path(false) }

    fn close(&mut self) -> PathId { self.end_sub_path(true) }

    fn current_position(&self) -> Vec2 { self.last_position }

    fn build(mut self) -> Path {
        if self.building {
            self.end();
        }
        return Path::from_vec(self.vertices, self.path_info);
    }
}

impl<Builder: PrimitiveBuilder> FlattenedBuilder<Builder> {
    pub fn new(builder: Builder, tolerance: f32) -> FlattenedBuilder<Builder> {
        FlattenedBuilder {
            builder: builder,
            tolerance: tolerance,
        }
    }

    pub fn set_tolerance(&mut self, tolerance: f32) { self.tolerance = tolerance }
}

impl PrimitiveImpl {
    pub fn new() -> PrimitiveImpl {
        PrimitiveImpl {
            vertices: Vec::with_capacity(512),
            path_info: Vec::with_capacity(16),
            last_position: vec2(0.0, 0.0),
            top_left: vec2(0.0, 0.0),
            bottom_right: vec2(0.0, 0.0),
            offset: 0,
            building: false,
        }
    }

    pub fn begin_sub_path(&mut self) {
        self.offset = self.vertices.len() as u16;
        self.building = true;
    }

    pub fn end_sub_path(&mut self, mut closed: bool) -> PathId {
        self.building = false;
        let vertices_len = self.vertices.len();
        if vertices_len == 0 {
            return path_id(self.path_info.len() as u16);
        }
        let offset = self.offset as usize;
        let last = vertices_len - 1;
        // If the first and last vertices are the same, remove the last vertex.
        let last = if last > 0 && self.vertices[last].position.fuzzy_eq(self.vertices[offset].position) {
            self.vertices.pop();
            closed = true;
            last - 1
        } else { last };

        let vertex_count = last + 1 - offset;

        if vertex_count == 0 {
            return path_id(self.path_info.len() as u16);
        }

        let vertex_range = vertex_id_range(self.offset, self.offset + vertex_count as u16);
        let aabb = Rect::new(
            self.top_left.x, self.top_left.y,
            self.bottom_right.x - self.top_left.x, self.bottom_right.y - self.top_left.y,
        );

        let shape_info = PathInfo {
            range: vertex_range,
            aabb: aabb,
            has_beziers: Some(false),
            is_closed: closed,
        };

        let index = path_id(self.path_info.len() as u16);
        self.path_info.push(shape_info);
        return index;
    }

    pub fn push(&mut self, point: Vec2, ptype: PointType) {
        debug_assert!(!point.x.is_nan());
        debug_assert!(!point.y.is_nan());
        if self.building && point == self.last_position {
            return;
        }

        if !self.building {
            self.begin_sub_path();
        }

        if self.vertices.len() == 0 {
            self.top_left = point;
            self.bottom_right = point;
        } else {
            if point.x < self.top_left.x { self.top_left.x = point.x; }
            if point.y < self.top_left.y { self.top_left.y = point.y; }
            if point.x > self.bottom_right.x { self.bottom_right.x = point.x; }
            if point.y > self.bottom_right.y { self.bottom_right.y = point.y; }
        }
        self.vertices.push(PointData{ position: point, point_type: ptype });
        self.last_position = point;
    }
}

impl<Builder: PrimitiveBuilder> PolygonBuilder for Builder {
    fn polygon(&mut self, points: &[Vec2]) -> PathId {
        assert!(!points.is_empty());

        self.move_to(points[0]);
        for p in points[1..].iter() {
            self.line_to(vec2(p.x,p.y));
        }
        return self.close();
    }
}

#[test]
fn test_path_builder_empty_path() {
    let _ = flattened_path_builder().build();
}

#[test]
fn test_path_builder_empty_sub_path() {
    let mut builder = flattened_path_builder();
    builder.move_to(vec2(0.0, 0.0));
    builder.move_to(vec2(1.0, 0.0));
    builder.move_to(vec2(2.0, 0.0));
    let _ = builder.build();
}

#[test]
fn test_path_builder_close_empty() {
    let mut builder = flattened_path_builder();
    builder.end();
    builder.close();
    builder.end();
    builder.end();
    builder.close();
    builder.close();
    let _ = builder.build();
}


#[test]
fn test_path_builder_simple() {

    // clockwise
    {
        let mut path = flattened_path_builder();
        path.move_to(vec2(0.0, 0.0));
        path.line_to(vec2(1.0, 0.0));
        path.line_to(vec2(1.0, 1.0));
        let id = path.close();

        let path = path.build();
        let info = path.sub_path(id).info();
        assert_eq!(path.vertices().nth(0).position, vec2(0.0, 0.0));
        assert_eq!(path.vertices().nth(1).position, vec2(1.0, 0.0));
        assert_eq!(path.vertices().nth(2).position, vec2(1.0, 1.0));
        assert_eq!(path.vertices().nth(0).point_type, PointType::Normal);
        assert_eq!(path.vertices().nth(1).point_type, PointType::Normal);
        assert_eq!(path.vertices().nth(2).point_type, PointType::Normal);
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
        let sub_path = path.sub_path(id);
        let first = sub_path.first();
        let next = sub_path.next(first);
        let prev = sub_path.previous(first);
        assert!(first != next);
        assert!(first != prev);
        assert!(next != prev);
        assert_eq!(first, sub_path.previous(next));
        assert_eq!(first, sub_path.next(prev));
    }

    // counter-clockwise
    {
        let mut path = flattened_path_builder();
        path.move_to(vec2(0.0, 0.0));
        path.line_to(vec2(1.0, 1.0));
        path.line_to(vec2(1.0, 0.0));
        let id = path.close();

        let path = path.build();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
    }

    // line_to back to the first vertex (should ignore the last vertex)
    {
        let mut path = flattened_path_builder();
        path.move_to(vec2(0.0, 0.0));
        path.line_to(vec2(1.0, 1.0));
        path.line_to(vec2(1.0, 0.0));
        path.line_to(vec2(0.0, 0.0));
        let id = path.close();

        let path = path.build();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
    }
}

#[test]
fn test_path_builder_simple_bezier() {
    // clockwise
    {
        let mut path = bezier_path_builder();
        path.move_to(vec2(0.0, 0.0));
        path.quadratic_bezier_to(vec2(1.0, 0.0), vec2(1.0, 1.0));
        let id = path.close();

        let path = path.build();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
    }

    // counter-clockwise
    {
        let mut path = bezier_path_builder();
        path.move_to(vec2(0.0, 0.0));
        path.quadratic_bezier_to(vec2(1.0, 1.0), vec2(1.0, 0.0));
        let id = path.close();

        let path = path.build();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
    }

    // a slightly more elaborate path
    {
        let mut path = bezier_path_builder();
        path.move_to(vec2(0.0, 0.0));
        path.line_to(vec2(0.1, 0.0));
        path.line_to(vec2(0.2, 0.1));
        path.line_to(vec2(0.3, 0.1));
        path.line_to(vec2(0.4, 0.0));
        path.line_to(vec2(0.5, 0.0));
        path.quadratic_bezier_to(vec2(0.5, 0.4), vec2(0.3, 0.4));
        path.line_to(vec2(0.1, 0.4));
        path.quadratic_bezier_to(vec2(-0.2, 0.1), vec2(-0.1, 0.0));
        let id = path.close();

        let path = path.build();
        let info = path.sub_path(id).info();
        assert_eq!(info.aabb, Rect::new(-0.2, 0.0, 0.7, 0.4));
    }
}

#[test]
fn test_arc_simple() {
    let mut path = bezier_path_builder();

    // Two big elliptical arc
    path.move_to(vec2(180.0, 180.0));
    path.arc_to(
        vec2(160.0, 220.0), vec2(20.0, 40.0) , 0.0,
        ArcFlags { large_arc: true, sweep: false }
    );
    path.move_to(vec2(180.0, 180.0));
    path.arc_to(
        vec2(160.0, 220.0), vec2(20.0, 40.0) , 0.0,
        ArcFlags { large_arc: true, sweep: true }
    );

    // a small elliptical arc
    path.move_to(vec2(260.0, 150.0));
    path.arc_to(
        vec2(240.0, 190.0), vec2(20.0, 40.0) , 0.0,
        ArcFlags {large_arc: false, sweep: true}
    );

    path.build();
}