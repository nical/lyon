use tesselation::path::*;
use tesselation::bezier::*;
use tesselation::{
    vertex_id_range,
//    crash,
};

use vodk_math::{ Vec2, vec2, Rect };

pub trait CurveBuilder {
    fn push_vertex(&mut self, v: Vec2);
}

pub struct PathBuilder {
    vertices: Vec<PointData>,
    path_info: Vec<PathInfo>,
    last_position: Vec2,
    last_ctrl: Vec2,
    top_left: Vec2,
    bottom_right: Vec2,
    tolerance: f32,
    offset: u16,
    // flags
    has_beziers: bool,
    flatten: bool,
    building: bool,
}

impl PathBuilder {
    pub fn new() -> PathBuilder {
        PathBuilder {
            vertices: Vec::with_capacity(512),
            path_info: Vec::with_capacity(16),
            last_position: vec2(0.0, 0.0),
            last_ctrl: vec2(0.0, 0.0),
            top_left: vec2(0.0, 0.0),
            bottom_right: vec2(0.0, 0.0),
            offset: 0,
            tolerance: 0.05,
            has_beziers: false,
            flatten: false,
            building: false,
        }
    }

    pub fn finish(mut self) -> Path {
        if self.building {
            self.end();
        }
        return Path::from_vec(self.vertices, self.path_info);
    }

    pub fn set_flattening(&mut self, flattening: bool) { self.flatten = flattening }

    pub fn set_tolerance(&mut self, tolerance: f32) { self.tolerance = tolerance }

    pub fn move_to(&mut self, to: Vec2)
    {
        if self.building {
            self.end_sub_path(false);
        }
        self.last_ctrl = to;
        self.top_left = to;
        self.bottom_right = to;
        self.push(to, PointType::Normal);
    }

    pub fn line_to(&mut self, to: Vec2) {
        self.last_ctrl = to;
        self.push(to, PointType::Normal);
    }

    pub fn relative_line_to(&mut self, to: Vec2) {
        let offset = self.last_position;
        assert!(!offset.x.is_nan() && !offset.y.is_nan());
        self.push(offset + to, PointType::Normal);
    }

    pub fn quadratic_bezier_to(&mut self, ctrl: Vec2, to: Vec2) {
        self.last_ctrl = ctrl;
        if self.flatten {
            let from = self.last_position;
            let cubic = QuadraticBezierSegment { from: from, cp: ctrl, to: to }.to_cubic();
            flatten_cubic_bezier(cubic, self.tolerance, self);
        } else {
            self.push(ctrl, PointType::Control);
            self.push(to, PointType::Normal);
            self.has_beziers = true;
        }
    }

    pub fn relative_quadratic_bezier_to(&mut self, ctrl: Vec2, to: Vec2) {
        let offset = self.last_position;
        self.quadratic_bezier_to(ctrl + offset, to + offset);
    }

    pub fn cubic_bezier_to(&mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) {
        self.last_ctrl = ctrl2;
        if self.flatten {
            flatten_cubic_bezier(
                CubicBezierSegment{
                    from: self.last_position,
                    cp1: ctrl1,
                    cp2: ctrl2,
                    to: to,
                },
                self.tolerance,
                self
            );
        } else {
            self.push(ctrl1, PointType::Control);
            self.push(ctrl2, PointType::Control);
            self.push(to, PointType::Normal);
            self.has_beziers = true;
        }
    }

    pub fn relative_cubic_bezier_to(&mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) {
        let offset = self.last_position;
        self.cubic_bezier_to(ctrl1 + offset, ctrl2 + offset, to + offset);
    }

    pub fn cubic_bezier_symetry_to(&mut self, ctrl2: Vec2, to: Vec2) {
        let ctrl = self.last_position + (self.last_position - self.last_ctrl);
        self.cubic_bezier_to(ctrl, ctrl2, to);
    }

    pub fn relative_cubic_bezier_symetry_to(&mut self, ctrl2: Vec2, to: Vec2) {
        let ctrl = self.last_position - self.last_ctrl;
        self.relative_cubic_bezier_to(ctrl, ctrl2, to);
    }

    pub fn quadratic_bezier_symetry_to(&mut self, to: Vec2) {
        let ctrl = self.last_position + (self.last_position - self.last_ctrl);
        self.quadratic_bezier_to(ctrl, to);
    }

    pub fn relative_quadratic_bezier_symetry_to(&mut self, to: Vec2) {
        let ctrl = self.last_position - self.last_ctrl;
        self.relative_quadratic_bezier_to(ctrl, to);
    }

    pub fn horizontal_line_to(&mut self, x: f32) {
        let y = self.last_position.y;
        self.line_to(vec2(x, y));
    }

    pub fn relative_horizontal_line_to(&mut self, dx: f32) {
        let p = self.last_position;
        self.line_to(vec2(p.x + dx, p.y));
    }

    pub fn vertical_line_to(&mut self, y: f32) {
        let x = self.last_position.x;
        self.line_to(vec2(x, y));
    }

    pub fn relative_vertical_line_to(&mut self, dy: f32) {
        let p = self.last_position;
        self.line_to(vec2(p.x, p.y + dy));
    }

    pub fn end(&mut self) -> PathId { self.end_sub_path(false) }

    pub fn close(&mut self) -> PathId { self.end_sub_path(true) }

    fn begin_sub_path(&mut self) {
        self.offset = self.vertices.len() as u16;
        self.building = true;
    }

    fn end_sub_path(&mut self, mut closed: bool) -> PathId {
        self.building = false;
        let offset = self.offset as usize;
        let last = self.vertices.len() - 1;
        // If the first and last vertices are the same, remove the last vertex.
        let last = if last > 0 && self.vertices[last].position.fuzzy_eq(self.vertices[offset].position) {
            self.vertices.pop();
            closed = true;
            last - 1
        } else { last };

        let vertex_count = last - offset + 1;

        let vertex_range = vertex_id_range(self.offset, self.offset + vertex_count as u16);
        let aabb = Rect::new(
            self.top_left.x, self.top_left.y,
            self.bottom_right.x - self.top_left.x, self.bottom_right.y - self.top_left.y,
        );

        let shape_info = PathInfo {
            range: vertex_range,
            aabb: aabb,
            has_beziers: Some(self.has_beziers),
            is_closed: closed,
        };

        let index = path_id(self.path_info.len() as u16);
        self.path_info.push(shape_info);
        return index;
    }

    fn push(&mut self, point: Vec2, ptype: PointType) {
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

impl CurveBuilder for PathBuilder {
    fn push_vertex(&mut self, v: Vec2) { self.push(v, PointType::Normal); }
}

#[test]
fn test_path_builder_simple() {

    // clockwise
    {
        let mut path = PathBuilder::new();
        path.move_to(vec2(0.0, 0.0));
        path.line_to(vec2(1.0, 0.0));
        path.line_to(vec2(1.0, 1.0));
        let id = path.close();

        let path = path.finish();
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
        let mut path = PathBuilder::new();
        path.move_to(vec2(0.0, 0.0));
        path.line_to(vec2(1.0, 1.0));
        path.line_to(vec2(1.0, 0.0));
        let id = path.close();

        let path = path.finish();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
    }

    // line_to back to the first vertex (should ignore the last vertex)
    {
        let mut path = PathBuilder::new();
        path.move_to(vec2(0.0, 0.0));
        path.line_to(vec2(1.0, 1.0));
        path.line_to(vec2(1.0, 0.0));
        path.line_to(vec2(0.0, 0.0));
        let id = path.close();

        let path = path.finish();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
    }
}

#[test]
fn test_path_builder_simple_bezier() {
    // clockwise
    {
        let mut path = PathBuilder::new();
        path.move_to(vec2(0.0, 0.0));
        path.quadratic_bezier_to(vec2(1.0, 0.0), vec2(1.0, 1.0));
        let id = path.close();

        let path = path.finish();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
    }

    // counter-clockwise
    {
        let mut path = PathBuilder::new();
        path.move_to(vec2(0.0, 0.0));
        path.quadratic_bezier_to(vec2(1.0, 1.0), vec2(1.0, 0.0));
        let id = path.close();

        let path = path.finish();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
    }

    // a slightly more elaborate path
    {
        let mut path = PathBuilder::new();
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

        let path = path.finish();
        let info = path.sub_path(id).info();
        assert_eq!(info.aabb, Rect::new(-0.2, 0.0, 0.7, 0.4));
    }
}
