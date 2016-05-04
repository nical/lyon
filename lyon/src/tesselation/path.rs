use std::f32::consts::PI;
use tesselation::{
    vertex_id, vertex_id_range,
    VertexId, VertexIdRange,
    VertexSlice, MutVertexSlice,
    WindingOrder
};

use tesselation::vectors::{ Position2D };
use tesselation::sweep_line::{ compute_event_type, EventType, };
use tesselation::bezier::*;

use vodk_math::{ Vec2, vec2, Rect };

use vodk_id::{ Id, IdRange, ToIndex };

#[derive(Debug)]
pub struct Path_;
pub type PathId = Id<Path_, u16>;
pub type PathIdRange = IdRange<Path_, u16>;
pub fn path_id(idx: u16) -> PathId { PathId::new(idx) }

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum PointType {
    Normal,
    Control,
}

#[derive(Copy, Clone, Debug)]
pub struct PointData {
    pub position: Vec2,
    pub point_type: PointType,
}

impl Position2D for PointData { fn position(&self) -> Vec2 { self.position } }

#[derive(Clone, Debug)]
pub struct ComplexPath {
    vertices: Vec<PointData>,
    sub_paths: Vec<PathInfo>,
}

impl ComplexPath {
    pub fn new() -> ComplexPath {
        ComplexPath { vertices: Vec::new(), sub_paths: Vec::new() }
    }

    pub fn vertices(&self) -> VertexSlice<PointData> { VertexSlice::new(&self.vertices[..]) }

    pub fn mut_vertices(&mut self) -> MutVertexSlice<PointData> { MutVertexSlice::new(&mut self.vertices[..]) }

    pub fn num_vertices(&self) -> usize { self.as_slice().num_vertices() }

    pub fn sub_path(&self, id: PathId) -> PathSlice {
        PathSlice {
            vertices: VertexSlice::new(&self.vertices[..]),
            info: &self.sub_paths[id.handle.to_index()]
        }
    }

    pub fn path_ids(&self) -> PathIdRange {
        IdRange::new(0, self.sub_paths.len() as u16)
    }

    pub fn as_slice(&self) -> ComplexPathSlice {
        ComplexPathSlice {
            vertices: VertexSlice::new(&self.vertices[..]),
            sub_paths: &self.sub_paths[..],
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ComplexVertexId {
    pub vertex_id: VertexId,
    pub path_id: PathId,
}

pub struct ComplexVertexIdRange {
    range: VertexIdRange,
    path_id: PathId,
}

impl Iterator for ComplexVertexIdRange {
    type Item = ComplexVertexId;
    fn next(&mut self) -> Option<ComplexVertexId> {
        return if let Some(next) = self.range.next() {
            Some(ComplexVertexId {
                vertex_id: next,
                path_id: self.path_id
            })
        } else {
            None
        };
    }
}

#[derive(Copy, Clone)]
pub struct ComplexPathSlice<'l> {
    vertices: VertexSlice<'l,PointData>,
    sub_paths: &'l[PathInfo],
}

impl<'l> ComplexPathSlice<'l> {

    pub fn vertices(&self) -> VertexSlice<PointData> { self.vertices }

    pub fn vertex_ids(&self, sub_path: PathId) -> ComplexVertexIdRange {
        ComplexVertexIdRange {
            range: self.sub_path(sub_path).vertex_ids(),
            path_id: sub_path,
        }
    }

    pub fn num_vertices(&self) -> usize { self.vertices.len() }

    pub fn num_sub_paths(&self) -> usize { self.sub_paths.len() }

    pub fn sub_path(&self, id: PathId) -> PathSlice {
        PathSlice {
            vertices: self.vertices,
            info: &self.sub_paths[id.handle.to_index()]
        }
    }

    pub fn path_ids(&self) -> PathIdRange {
        IdRange::new(0, self.sub_paths.len() as u16)
    }

    pub fn vertex(&self, id: ComplexVertexId) -> &PointData {
        &self.vertices[id.vertex_id]
    }

    pub fn next(&self, id: ComplexVertexId) -> ComplexVertexId {
        ComplexVertexId {
            path_id: id.path_id,
            vertex_id: self.sub_path(id.path_id).next(id.vertex_id),
        }
    }

    pub fn previous(&self, id: ComplexVertexId) -> ComplexVertexId {
        ComplexVertexId {
            path_id: id.path_id,
            vertex_id: self.sub_path(id.path_id).previous(id.vertex_id),
        }
    }
}

#[derive(Copy, Clone)]
pub struct PathSlice<'l> {
    vertices: VertexSlice<'l, PointData>,
    info: &'l PathInfo,
}

impl<'l> PathSlice<'l> {
    pub fn info(&self) -> &'l PathInfo { self.info }

    pub fn vertex(&self, id: VertexId) -> &PointData { &self.vertices[id] }

    pub fn first(&self) -> VertexId { self.info.range.first }

    pub fn next(&self, id: VertexId) -> VertexId {
        let first = self.info.range.first.handle;
        let last = first + self.info.range.count - 1;
        debug_assert!(id.handle >= first);
        debug_assert!(id.handle <= last);
        return Id::new(if id.handle == last { first } else { id.handle + 1 });
    }

    pub fn previous(&self, id: VertexId) -> VertexId {
        let first = self.info.range.first.handle;
        let last = first + self.info.range.count - 1;
        debug_assert!(id.handle >= first);
        debug_assert!(id.handle <= last);
        return Id::new(if id.handle == first { last } else { id.handle - 1 });
    }

    pub fn next_vertex(&self, id: VertexId) -> &PointData {
        self.vertex(self.next(id))
    }

    pub fn previous_vertex(&self, id: VertexId) -> &PointData {
        self.vertex(self.previous(id))
    }

    pub fn vertex_ids(&self) -> VertexIdRange { self.info().range }

    pub fn num_vertices(&self) -> usize { self.vertices.len() }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct PathInfo {
    pub aabb: Rect,
    pub range: VertexIdRange,
    pub winding_order: Option<WindingOrder>,
    pub is_convex: Option<bool>,
    pub is_y_monotone: Option<bool>,
    pub has_beziers: Option<bool>,
    pub is_closed: bool,
}

pub struct PathBuilder<'l> {
    path: &'l mut ComplexPath,
    last_position: Vec2,
    last_ctrl: Vec2,
    top_left: Vec2,
    bottom_right: Vec2,
    accum_angle: f32,
    offset: u16,
    // flags
    convex_if_cw: bool,
    convex_if_ccw: bool,
    y_monotone_if_cw: bool,
    y_monotone_if_ccw: bool,
    has_beziers: bool,
    flatten: bool,
}

impl<'l> PathBuilder<'l> {
    pub fn begin(path: &'l mut ComplexPath, pos: Vec2) -> PathBuilder {
        let offset = path.vertices.len() as u16;
        path.vertices.push(PointData { position: pos, point_type: PointType::Normal });
        PathBuilder {
            path: path,
            last_position: pos,
            last_ctrl: vec2(0.0, 0.0),
            accum_angle: 0.0,
            top_left: vec2(0.0, 0.0),
            bottom_right: vec2(0.0, 0.0),
            offset: offset,
            convex_if_cw: true,
            convex_if_ccw: true,
            y_monotone_if_cw: true,
            y_monotone_if_ccw: true,
            has_beziers: false,
            flatten: false,
        }
    }

    pub fn flattened(mut self) -> PathBuilder<'l> {
        self.flatten = true;
        return self;
    }

    pub fn line_to(mut self, to: Vec2) -> PathBuilder<'l> {
        self.push(to, PointType::Normal);
        return self;
    }

    pub fn relative_line_to(mut self, to: Vec2) -> PathBuilder<'l> {
        let offset = self.last_position;
        assert!(!offset.x.is_nan() && !offset.y.is_nan());
        self.push(offset + to, PointType::Normal);
        return self;
    }

    pub fn quadratic_bezier_to(mut self, ctrl: Vec2, to: Vec2) -> PathBuilder<'l> {
        if self.flatten {
            let num_points = 8;
            let from = self.last_position;
            for i in 0..num_points {
                let t = (i+1) as f32 / num_points as f32;
                self.push(sample_quadratic_bezier(from, ctrl, to, t), PointType::Normal);
            }
            self.push(to, PointType::Normal);
        } else {
            self.push(ctrl, PointType::Control);
            self.push(to, PointType::Normal);
            self.has_beziers = true;
        }
        self.last_ctrl = ctrl;
        return self;
    }

    pub fn relative_quadratic_bezier_to(self, ctrl: Vec2, to: Vec2) -> PathBuilder<'l> {
        let offset = self.last_position;
        return self.quadratic_bezier_to(ctrl+offset, to+offset);
    }

    pub fn cubic_bezier_to(mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) -> PathBuilder<'l> {
        if self.flatten {
            let num_points = 8;
            let from = self.last_position;
            for i in 0..num_points {
                let t = (i+1) as f32 / num_points as f32;
                self.push(sample_cubic_bezier(from, ctrl1, ctrl2, to, t), PointType::Normal);
            }
        } else {
            self.push(ctrl1, PointType::Control);
            self.push(ctrl2, PointType::Control);
            self.push(to, PointType::Normal);
            self.has_beziers = true;
        }
        self.last_ctrl = ctrl2;
        return self;
    }

    pub fn relative_cubic_bezier_to(self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) -> PathBuilder<'l> {
        let offset = self.last_position;
        return self.cubic_bezier_to(ctrl1+offset, ctrl2+offset, to+offset);
    }

    // TODO: This is the "S" operation from svg, not sure how it should be called
    pub fn cubic_bezier_to_s(self, ctrl2: Vec2, to: Vec2) -> PathBuilder<'l> {
        let ctrl = self.last_position + (self.last_position - self.last_ctrl);
        return self.cubic_bezier_to(ctrl, ctrl2, to);
    }

    pub fn relative_cubic_bezier_to_s(self, ctrl2: Vec2, to: Vec2) -> PathBuilder<'l> {
        let ctrl = self.last_position - self.last_ctrl;
        return self.relative_cubic_bezier_to(ctrl, ctrl2, to);
    }

    pub fn quadratic_bezier_to_s(self, to: Vec2) -> PathBuilder<'l> {
        let ctrl = self.last_position + (self.last_position - self.last_ctrl);
        return self.quadratic_bezier_to(ctrl, to);
    }

    pub fn relative_quadratic_bezier_to_s(self, to: Vec2) -> PathBuilder<'l> {
        let ctrl = self.last_position - self.last_ctrl;
        return self.relative_quadratic_bezier_to(ctrl, to);
    }

    pub fn horizontal_line_to(self, x: f32) -> PathBuilder<'l> {
        let y = self.last_position.y;
        return self.line_to(vec2(x, y));
    }

    pub fn relative_horizontal_line_to(self, dx: f32) -> PathBuilder<'l> {
        let p = self.last_position;
        return self.line_to(vec2(p.x + dx, p.y));
    }

    pub fn vertical_line_to(self, y: f32) -> PathBuilder<'l> {
        let x = self.last_position.x;
        return self.line_to(vec2(x, y));
    }

    pub fn relative_vertical_line_to(self, dy: f32) -> PathBuilder<'l> {
        let p = self.last_position;
        return self.line_to(vec2(p.x, p.y + dy));
    }

    pub fn end(self) -> PathId { self.finish(false) }

    pub fn close(self) -> PathId { self.finish(true) }

    fn finish(mut self, mut closed: bool) -> PathId {
        let offset = self.offset as usize;
        let last = self.path.vertices.len() - 1;
        // If the first and last vertices are the same, remove the last vertex.
        let last = if last > 0 && self.path.vertices[last].position.fuzzy_eq(self.path.vertices[offset].position) {
            self.path.vertices.pop();
            closed = true;
            last - 1
        } else { last };

        let vertex_count = last - offset + 1;

        let vertex_range = vertex_id_range(self.offset, self.offset + vertex_count as u16);
        let aabb = Rect::new(
            self.top_left.x, self.top_left.y,
            self.bottom_right.x - self.top_left.x, self.bottom_right.y - self.top_left.y,
        );

        let shape_info = if vertex_count > 2 {
            let a = self.path.vertices[last - 1].position;
            let b = self.path.vertices[last].position;
            let c = self.path.vertices[offset].position;
            let d = self.path.vertices[offset+1].position;

            self.update_angle(a, b, c);
            self.update_angle(b, c, d);

            if self.accum_angle > ((vertex_count-1) as f32) * PI {
                PathInfo {
                    range: vertex_range,
                    aabb: aabb,
                    winding_order: Some(WindingOrder::Clockwise),
                    is_convex: Some(self.convex_if_cw),
                    is_y_monotone: Some(self.y_monotone_if_cw),
                    has_beziers: Some(self.has_beziers),
                    is_closed: closed,
                }
            } else {
                PathInfo {
                    range: vertex_range,
                    aabb: aabb,
                    winding_order: Some(WindingOrder::CounterClockwise),
                    is_convex: Some(self.convex_if_ccw),
                    is_y_monotone: Some(self.y_monotone_if_ccw),
                    has_beziers: Some(self.has_beziers),
                    is_closed: closed,
                }
            }
        } else {
            PathInfo {
                range: vertex_range,
                aabb: aabb,
                winding_order: None,
                is_convex: None,
                is_y_monotone: None,
                has_beziers: Some(self.has_beziers),
                is_closed: false,
            }
        };

        let index = path_id(self.path.sub_paths.len() as u16);
        self.path.sub_paths.push(shape_info);
        return index;
    }

    fn push(&mut self, point: Vec2, ptype: PointType) {
        if point == self.last_position {
            println!(" point == last_position");
            return;
        }
        if self.path.vertices.len() == 0 {
            self.top_left = point;
            self.bottom_right = point;
        } else {
            if point.x < self.top_left.x { self.top_left.x = point.x; }
            if point.y < self.top_left.y { self.top_left.y = point.y; }
            if point.x > self.bottom_right.x { self.bottom_right.x = point.x; }
            if point.y > self.bottom_right.y { self.bottom_right.y = point.y; }
        }
        self.path.vertices.push(PointData{ position: point, point_type: ptype });
        let idx = self.path.vertices.len() - 1;
        if idx - self.offset as usize > 2 {
            let a = self.path.vertices[idx-2].position;
            let b = self.path.vertices[idx-1].position;
            let c = self.path.vertices[idx  ].position;
            self.update_angle(a, b, c);
        }
        self.last_position = point;
    }

    fn update_angle(&mut self, a: Vec2, b: Vec2, c: Vec2) {
        let angle = (a - b).directed_angle(c - b);
        self.accum_angle += angle;
        if angle < PI {
            self.convex_if_cw = false;
        } else {
            self.convex_if_ccw = false;
        }
        let vertex_type = compute_event_type(a, b, c);
        match vertex_type {
            EventType::Split|EventType::Merge => { self.y_monotone_if_cw = false; }
            EventType::Start|EventType::End => { self.y_monotone_if_ccw = false; }
            _ => {}
        }
    }
}

#[test]
fn test_path_builder_simple() {
    let mut path = ComplexPath::new();
    // clockwise
    {
        let id = PathBuilder::begin(&mut path, vec2(0.0, 0.0))
            .line_to(vec2(1.0, 0.0))
            .line_to(vec2(1.0, 1.0))
            .close();
        let info = path.sub_path(id).info();
        assert_eq!(path.vertices[0].position, vec2(0.0, 0.0));
        assert_eq!(path.vertices[1].position, vec2(1.0, 0.0));
        assert_eq!(path.vertices[2].position, vec2(1.0, 1.0));
        assert_eq!(path.vertices[0].point_type, PointType::Normal);
        assert_eq!(path.vertices[1].point_type, PointType::Normal);
        assert_eq!(path.vertices[2].point_type, PointType::Normal);
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
        assert_eq!(info.winding_order, Some(WindingOrder::Clockwise));
        assert_eq!(info.is_convex, Some(true));
        assert_eq!(info.is_y_monotone, Some(true));
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
        let id = PathBuilder::begin(&mut path, vec2(0.0, 0.0))
            .line_to(vec2(1.0, 1.0))
            .line_to(vec2(1.0, 0.0))
            .close();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(3, 6));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
        assert_eq!(info.winding_order, Some(WindingOrder::CounterClockwise));
        assert_eq!(info.is_convex, Some(true));
        assert_eq!(info.is_y_monotone, Some(true));
    }

    // line_to back to the first vertex (should ignore the last vertex)
    {
        let id = PathBuilder::begin(&mut path, vec2(0.0, 0.0))
            .line_to(vec2(1.0, 1.0))
            .line_to(vec2(1.0, 0.0))
            .line_to(vec2(0.0, 0.0))
            .close();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(6, 9));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
        assert_eq!(info.winding_order, Some(WindingOrder::CounterClockwise));
        assert_eq!(info.is_convex, Some(true));
        assert_eq!(info.is_y_monotone, Some(true));
    }
}

#[test]
fn test_path_builder_simple_bezier() {
    let mut path = ComplexPath::new();

    // clockwise
    {
        let id = PathBuilder::begin(&mut path, vec2(0.0, 0.0))
            .quadratic_bezier_to(vec2(1.0, 0.0), vec2(1.0, 1.0))
            .close();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
        assert_eq!(info.winding_order, Some(WindingOrder::Clockwise));
        assert_eq!(info.is_convex, Some(true));
        assert_eq!(info.is_y_monotone, Some(true));
    }

    // counter-clockwise
    {
        let id = PathBuilder::begin(&mut path, vec2(0.0, 0.0))
            .quadratic_bezier_to(vec2(1.0, 1.0), vec2(1.0, 0.0))
            .close();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(3, 6));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
        assert_eq!(info.winding_order, Some(WindingOrder::CounterClockwise));
        assert_eq!(info.is_convex, Some(true));
        assert_eq!(info.is_y_monotone, Some(true));
    }

    // a slightly more elaborate path
    {
        let id = PathBuilder::begin(&mut path, vec2(0.0, 0.0))
            .line_to(vec2(0.1, 0.0))
            .line_to(vec2(0.2, 0.1))
            .line_to(vec2(0.3, 0.1))
            .line_to(vec2(0.4, 0.0))
            .line_to(vec2(0.5, 0.0))
            .quadratic_bezier_to(vec2(0.5, 0.4), vec2(0.3, 0.4))
            .line_to(vec2(0.1, 0.4))
            .quadratic_bezier_to(vec2(-0.2, 0.1), vec2(-0.1, 0.0))
            .close();
        let info = path.sub_path(id).info();
        assert_eq!(info.aabb, Rect::new(-0.2, 0.0, 0.7, 0.4));
        assert_eq!(info.winding_order, Some(WindingOrder::Clockwise));
        assert_eq!(info.is_convex, Some(false));
        assert_eq!(info.is_y_monotone, Some(false));
    }

    // simple non-convex but y-monotone path
    {
        let id = PathBuilder::begin(&mut path, vec2(0.0, 0.0))
            .line_to(vec2(2.0, 1.0))
            .line_to(vec2(1.0, 2.0))
            .line_to(vec2(2.0, 3.0))
            .line_to(vec2(0.0, 4.0))
            .line_to(vec2(-2.0, 3.0))
            .line_to(vec2(-1.0, 2.0))
            .line_to(vec2(-2.0, 1.0))
            .close();
        let info = path.sub_path(id).info();
        assert_eq!(info.aabb, Rect::new(-2.0, 0.0, 4.0, 4.0));
        assert_eq!(info.winding_order, Some(WindingOrder::Clockwise));
        assert_eq!(info.is_convex, Some(false));
        assert_eq!(info.is_y_monotone, Some(true));
    }
}
