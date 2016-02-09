use std::f32::consts::PI;
use tesselation::{ vertex_id_range, VertexIdRange, VertexId, WindingOrder };
use tesselation::vectors::{ Vec2, vec2_sub, vec2_add, vec2_almost_eq, directed_angle, Position2D, Rect };
use tesselation::monotone::{ get_vertex_type, VertexType, };

use vodk_id::{ Id, IdSlice, IdRange, ToIndex };

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

    pub fn vertices(&self) -> IdSlice<VertexId, PointData> { IdSlice::new(&self.vertices[..]) }

    pub fn sub_path(&self, id: PathId) -> PathSlice {
        PathSlice {
            vertices: &self.vertices[..],
            info: &self.sub_paths[id.handle.to_index()]
        }
    }

    pub fn path_ids(&self) -> PathIdRange {
        IdRange::new(0, self.sub_paths.len() as u16)
    }

    pub fn slice(&self) -> ComplexPathSlice {
        ComplexPathSlice {
            vertices: &self.vertices[..],
            sub_paths: &self.sub_paths[..],
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ComplexPathSlice<'l> {
    vertices: &'l[PointData],
    sub_paths: &'l[PathInfo],
}

impl<'l> ComplexPathSlice<'l> {

    pub fn vertices(&self) -> IdSlice<VertexId, PointData> { IdSlice::new(&self.vertices[..]) }

    pub fn sub_path(&self, id: PathId) -> PathSlice {
        PathSlice {
            vertices: self.vertices,
            info: &self.sub_paths[id.handle.to_index()]
        }
    }

    pub fn path_ids(&self) -> PathIdRange {
        IdRange::new(0, self.sub_paths.len() as u16)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PathSlice<'l> {
    vertices: &'l[PointData],
    info: &'l PathInfo,
}

impl<'l> PathSlice<'l> {
    pub fn vertices(self) -> &'l[PointData] {
        let range = self.info.range;
        let from = range.first.to_index();
        let count = range.count as usize;
        return &self.vertices[from..from+count];
    }

    pub fn info(self) -> &'l PathInfo { self.info }
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
}

impl<'l> PathBuilder<'l> {
    pub fn begin(path: &'l mut ComplexPath, pos: Vec2) -> PathBuilder {
        let offset = path.vertices.len() as u16;
        path.vertices.push(PointData { position: pos, point_type: PointType::Normal });
        PathBuilder {
            path: path,
            last_position: [::std::f32::NAN, ::std::f32::NAN],
            accum_angle: 0.0,
            top_left: [0.0, 0.0],
            bottom_right: [0.0, 0.0],
            offset: offset,
            convex_if_cw: true,
            convex_if_ccw: true,
            y_monotone_if_cw: true,
            y_monotone_if_ccw: true,
            has_beziers: false,
        }
    }

    pub fn line_to(mut self, to: Vec2) -> PathBuilder<'l> {
        self.push(to, PointType::Normal);
        return self;
    }

    pub fn quadratic_bezier_to(mut self, ctrl: Vec2, to: Vec2) -> PathBuilder<'l> {
        self.push(ctrl, PointType::Control);
        self.push(to, PointType::Normal);
        self.has_beziers = true;
        return self;
    }

    pub fn cubic_bezier_to(mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) -> PathBuilder<'l> {
        self.push(ctrl1, PointType::Control);
        self.push(ctrl2, PointType::Control);
        self.push(to, PointType::Normal);
        self.has_beziers = true;
        return self;
    }

    pub fn end(self) -> PathId { self.finish(false) }

    pub fn close(self) -> PathId { self.finish(true) }

    fn finish(mut self, mut closed: bool) -> PathId {
        let offset = self.offset as usize;
        let last = self.path.vertices.len() - 1;
        // If the first and last vertices are the same, remove the last vertex.
        let last = if vec2_almost_eq(self.path.vertices[last].position,
                                     self.path.vertices[offset].position) {
            self.path.vertices.pop();
            closed = true;
            last - 1
        } else { last };

        let vertex_count = last - offset + 1;

        let vertex_range = vertex_id_range(self.offset, self.offset + vertex_count as u16);
        let aabb = Rect {
            origin: self.top_left,
            size: vec2_sub(self.bottom_right, self.top_left)
        };

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
            return;
        }
        if self.path.vertices.len() == 0 {
            self.top_left = point;
            self.bottom_right = point;
        } else {
            if point.x() < self.top_left.x() { self.top_left[0] = point.x(); }
            if point.y() < self.top_left.y() { self.top_left[1] = point.y(); }
            if point.x() > self.bottom_right.x() { self.bottom_right[0] = point.x(); }
            if point.y() > self.bottom_right.y() { self.bottom_right[1] = point.y(); }
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
        let angle = directed_angle(vec2_sub(a, b), vec2_sub(c, b));
        self.accum_angle += angle;
        if angle < PI {
            self.convex_if_cw = false;
        } else {
            self.convex_if_ccw = false;
        }
        let vertex_type = get_vertex_type(a, b, c);
        match vertex_type {
            VertexType::Split|VertexType::Merge => { self.y_monotone_if_cw = false; }
            VertexType::Start|VertexType::End => { self.y_monotone_if_ccw = false; }
            _ => {}
        }
    }
}

#[test]
fn test_path_builder_simple() {
    let mut path = ComplexPath::new();
    // clockwise
    {
        let id = PathBuilder::begin(&mut path, [0.0, 0.0])
            .line_to([1.0, 0.0])
            .line_to([1.0, 1.0])
            .close();
        let info = path.sub_path(id).info();
        assert_eq!(path.vertices[0].position, [0.0, 0.0]);
        assert_eq!(path.vertices[1].position, [1.0, 0.0]);
        assert_eq!(path.vertices[2].position, [1.0, 1.0]);
        assert_eq!(path.vertices[0].point_type, PointType::Normal);
        assert_eq!(path.vertices[1].point_type, PointType::Normal);
        assert_eq!(path.vertices[2].point_type, PointType::Normal);
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect { origin: [0.0, 0.0], size: [1.0, 1.0] });
        assert_eq!(info.winding_order, Some(WindingOrder::Clockwise));
        assert_eq!(info.is_convex, Some(true));
        assert_eq!(info.is_y_monotone, Some(true));
    }

    // counter-clockwise
    {
        let id = PathBuilder::begin(&mut path, [0.0, 0.0])
            .line_to([1.0, 1.0])
            .line_to([1.0, 0.0])
            .close();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(3, 6));
        assert_eq!(info.aabb, Rect { origin: [0.0, 0.0], size: [1.0, 1.0] });
        assert_eq!(info.winding_order, Some(WindingOrder::CounterClockwise));
        assert_eq!(info.is_convex, Some(true));
        assert_eq!(info.is_y_monotone, Some(true));
    }

    // line_to back to the first vertex (should ignore the last vertex)
    {
        let id = PathBuilder::begin(&mut path, [0.0, 0.0])
            .line_to([1.0, 1.0])
            .line_to([1.0, 0.0])
            .line_to([0.0, 0.0])
            .close();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(6, 9));
        assert_eq!(info.aabb, Rect { origin: [0.0, 0.0], size: [1.0, 1.0] });
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
        let id = PathBuilder::begin(&mut path, [0.0, 0.0])
            .quadratic_bezier_to([1.0, 0.0], [1.0, 1.0])
            .close();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect { origin: [0.0, 0.0], size: [1.0, 1.0] });
        assert_eq!(info.winding_order, Some(WindingOrder::Clockwise));
        assert_eq!(info.is_convex, Some(true));
        assert_eq!(info.is_y_monotone, Some(true));
    }

    // counter-clockwise
    {
        let id = PathBuilder::begin(&mut path, [0.0, 0.0])
            .quadratic_bezier_to([1.0, 1.0], [1.0, 0.0])
            .close();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(3, 6));
        assert_eq!(info.aabb, Rect { origin: [0.0, 0.0], size: [1.0, 1.0] });
        assert_eq!(info.winding_order, Some(WindingOrder::CounterClockwise));
        assert_eq!(info.is_convex, Some(true));
        assert_eq!(info.is_y_monotone, Some(true));
    }

    // a slightly more elaborate path
    {
        let id = PathBuilder::begin(&mut path, [0.0, 0.0])
            .line_to([0.1, 0.0])
            .line_to([0.2, 0.1])
            .line_to([0.3, 0.1])
            .line_to([0.4, 0.0])
            .line_to([0.5, 0.0])
            .quadratic_bezier_to([0.5, 0.4], [0.3, 0.4])
            .line_to([0.1, 0.4])
            .quadratic_bezier_to([-0.2, 0.1], [-0.1, 0.0]) // TODO
            .close();
        let info = path.sub_path(id).info();
        assert_eq!(info.aabb, Rect { origin: [-0.2, 0.0], size: [0.7, 0.4] });
        assert_eq!(info.winding_order, Some(WindingOrder::Clockwise));
        assert_eq!(info.is_convex, Some(false));
        assert_eq!(info.is_y_monotone, Some(false));
    }

    // simple non-convex but y-monotone path
    {
        let id = PathBuilder::begin(&mut path, [0.0, 0.0])
            .line_to([2.0, 1.0])
            .line_to([1.0, 2.0])
            .line_to([2.0, 3.0])
            .line_to([0.0, 4.0])
            .line_to([-2.0, 3.0])
            .line_to([-1.0, 2.0])
            .line_to([-2.0, 1.0])
            .close();
        let info = path.sub_path(id).info();
        assert_eq!(info.aabb, Rect { origin: [-2.0, 0.0], size: [4.0, 4.0] });
        assert_eq!(info.winding_order, Some(WindingOrder::Clockwise));
        assert_eq!(info.is_convex, Some(false));
        assert_eq!(info.is_y_monotone, Some(true));
    }
}
