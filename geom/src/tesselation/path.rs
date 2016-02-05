use std::f32::consts::PI;
use tesselation::{ vertex_id, vertex_id_range, VertexId, VertexIdRange };
use tesselation::polygon::*;
use tesselation::polygon_partition::{ partition_polygon, Diagonals };
use tesselation::vectors::{ Vec2, vec2_sub, vec2_add, vec2_almost_eq, directed_angle, Position2D };
use tesselation::vertex_builder::{ VertexBufferBuilder };
use tesselation::bezier::{ separate_bezier_faces, triangulate_quadratic_bezier };
use tesselation::monotone::{
    is_y_monotone, get_vertex_type, VertexType, DecompositionContext, TriangulationContext,
};

use vodk_id::id_vector::IdSlice;
use vodk_id::ReverseIdRange;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum WindingOrder {
    Clockwise,
    CounterClockwise,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum PointType {
    Normal,
    Control,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Rect {
    pub origin: Vec2,
    pub size: Vec2,
}

impl Rect {
    pub fn top_left(&self) -> Vec2 { self.origin }
    pub fn bottom_right(&self) -> Vec2 { vec2_add(self.origin, self.size) }
    pub fn x_most(&self) -> f32 { self.bottom_right().x() }
    pub fn y_most(&self) -> f32 { self.bottom_right().y() }
    pub fn contains(&self, p: Vec2) -> bool {
        let bottom_right = self.bottom_right();
        let top_left = self.top_left();
        return top_left.x() <= p.x() && top_left.y() <= p.y() &&
               bottom_right.x() >= p.x() && bottom_right.y() >= p.y();
    }
}

pub struct PointData {
    pub position: Vec2,
    pub point_type: PointType,
}

impl Position2D for PointData { fn position(&self) -> Vec2 { self.position } }

pub struct PathBuilder<'l> {
    vertices: &'l mut Vec<PointData>,
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

#[derive(PartialEq, Debug)]
pub struct ShapeInfo {
    pub winding_order: WindingOrder,
    pub is_convex: bool,
    pub is_y_monotone: bool,
    pub has_beziers: bool,
}

#[derive(PartialEq, Debug)]
pub struct PathInfo {
    pub vertices: VertexIdRange,
    pub shape: Option<ShapeInfo>,
    pub aabb: Rect,
}

impl<'l> PathBuilder<'l> {
    pub fn begin(storage: &'l mut Vec<PointData>, pos: Vec2) -> PathBuilder {
        let offset = storage.len() as u16;
        storage.push(PointData { position: pos, point_type: PointType::Normal });
        PathBuilder {
            vertices: storage,
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

    pub fn close(mut self) -> PathInfo {
        let offset = self.offset as usize;
        let last = self.vertices.len() - 1;
        // If the first and last vertices are the same, remove the last vertex.
        let last = if vec2_almost_eq(self.vertices[last].position,self.vertices[offset].position) {
            self.vertices.pop();
            last - 1
        } else { last };

        let vertex_count = last - offset + 1;
        let shape_info = if vertex_count > 2 {
            let a = self.vertices[last - 1].position;
            let b = self.vertices[last].position;
            let c = self.vertices[offset].position;
            let d = self.vertices[offset+1].position;

            self.update_angle(a, b, c);
            self.update_angle(b, c, d);

            if self.accum_angle > ((vertex_count-1) as f32) * PI {
                Some(ShapeInfo {
                    winding_order: WindingOrder::Clockwise,
                    is_convex: self.convex_if_cw,
                    is_y_monotone: self.y_monotone_if_cw,
                    has_beziers: self.has_beziers,
                })
            } else {
                Some(ShapeInfo {
                    winding_order: WindingOrder::CounterClockwise,
                    is_convex: self.convex_if_ccw,
                    is_y_monotone: self.y_monotone_if_ccw,
                    has_beziers: self.has_beziers,
                })
            }
        } else {
            None
        };

        return PathInfo {
            vertices: vertex_id_range(self.offset, self.offset + vertex_count as u16),
            shape: shape_info,
            aabb: Rect {
                origin: self.top_left,
                size: vec2_sub(self.bottom_right, self.top_left)
            }
        };
    }

    fn push(&mut self, point: Vec2, ptype: PointType) {
        if point == self.last_position {
            return;
        }
        if self.vertices.len() == 0 {
            self.top_left = point;
            self.bottom_right = point;
        } else {
            if point.x() < self.top_left.x() { self.top_left[0] = point.x(); }
            if point.y() < self.top_left.y() { self.top_left[1] = point.y(); }
            if point.x() > self.bottom_right.x() { self.bottom_right[0] = point.x(); }
            if point.y() > self.bottom_right.y() { self.bottom_right[1] = point.y(); }
        }
        self.vertices.push(PointData{ position: point, point_type: ptype });
        let idx = self.vertices.len() - 1;
        if idx - self.offset as usize > 2 {
            let a = self.vertices[idx-2].position;
            let b = self.vertices[idx-1].position;
            let c = self.vertices[idx  ].position;
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

pub fn triangulate_path_fill<'l, Output: VertexBufferBuilder<Vec2>>(
    path: PathInfo,
    holes: &[PathInfo],
    points: &'l Vec<PointData>,
    output: &mut Output
) {
    output.begin_geometry();

    let main_poly = match path.shape.unwrap().winding_order {
        WindingOrder::Clockwise => { Polygon::from_vertices(path.vertices) }
        WindingOrder::CounterClockwise => { Polygon::from_vertices(ReverseIdRange::new(path.vertices)) }
    };

    let mut polygon = ComplexPolygon{
        main: main_poly,
        holes: vec![],
    };

    for ref hole in holes {
        assert!(hole.shape.is_some());
        if let Some(ref shape) = hole.shape {
            if shape.winding_order == WindingOrder::CounterClockwise {
                polygon.holes.push(Polygon::from_vertices(hole.vertices));
            } else {
                polygon.holes.push(Polygon::from_vertices(ReverseIdRange::new(hole.vertices)));
            }
        }
    }

    for v in points {
        output.push_vertex(v.position());
    }

    let vertex_positions = IdSlice::new(points);
    let mut beziers: Vec<[Vec2; 3]> = vec![];

    separate_bezier_faces(&mut polygon.main, vertex_positions, &mut beziers);

    let mut diagonals = Diagonals::new();
    let mut ctx = DecompositionContext::new();

    let res = ctx.y_monotone_polygon_decomposition(&polygon, vertex_positions, &mut diagonals);
    assert_eq!(res, Ok(()));

    let mut monotone_polygons = Vec::new();
    partition_polygon(&polygon, vertex_positions, &mut diagonals, &mut monotone_polygons);

    let mut triangulator = TriangulationContext::new();
    for monotone_poly in monotone_polygons {
        assert!(is_y_monotone(monotone_poly.view(), vertex_positions));
        let res = triangulator.y_monotone_triangulation(monotone_poly.view(), vertex_positions, output);
        assert_eq!(res, Ok(()));
    }

    for b in beziers {
        let from = b[0];
        let ctrl = b[1];
        let to = b[2];
        triangulate_quadratic_bezier(from, ctrl, to, 16, output);
    }
}


#[test]
fn test_path_builder_simple() {
    let mut storage = vec![];
    // clockwise
    let path = PathBuilder::begin(&mut storage, [0.0, 0.0])
        .line_to([1.0, 0.0]).line_to([1.0, 1.0]).close();
    assert_eq!(path.vertices, vertex_id_range(0, 3));
    assert_eq!(storage[0].position, [0.0, 0.0]);
    assert_eq!(storage[1].position, [1.0, 0.0]);
    assert_eq!(storage[2].position, [1.0, 1.0]);
    assert_eq!(storage[0].point_type, PointType::Normal);
    assert_eq!(storage[1].point_type, PointType::Normal);
    assert_eq!(storage[2].point_type, PointType::Normal);
    assert_eq!(path.shape, Some(ShapeInfo {
        winding_order: WindingOrder::Clockwise,
        is_convex: true,
        is_y_monotone: true,
        has_beziers: false
    }));

    // counter-clockwise
    let path = PathBuilder::begin(&mut storage, [0.0, 0.0])
        .line_to([1.0, 1.0]).line_to([1.0, 0.0]).close();
    assert_eq!(path.vertices, vertex_id_range(3, 6));
    assert_eq!(path.shape, Some(ShapeInfo {
        winding_order: WindingOrder::CounterClockwise,
        is_convex: true,
        is_y_monotone: true,
        has_beziers: false
    }));

    // line_to back to the first vertex (should ignore the last vertex)
    let path = PathBuilder::begin(&mut storage, [0.0, 0.0])
        .line_to([1.0, 1.0]).line_to([1.0, 0.0]).line_to([0.0, 0.0]).close();
    assert_eq!(path.vertices, vertex_id_range(6, 9));
    assert_eq!(path.shape, Some(ShapeInfo {
        winding_order: WindingOrder::CounterClockwise,
        is_convex: true,
        is_y_monotone: true,
        has_beziers: false
    }));
}

#[test]
fn test_path_builder_simple_bezier() {
    let mut storage = vec![];

    // clockwise
    let path = PathBuilder::begin(&mut storage, [0.0, 0.0])
        .quadratic_bezier_to([1.0, 0.0], [1.0, 1.0]).close();
    assert_eq!(path.vertices, vertex_id_range(0, 3));
    assert_eq!(path.shape, Some(ShapeInfo {
        winding_order: WindingOrder::Clockwise,
        is_convex: true,
        is_y_monotone: true,
        has_beziers: true
    }));
    assert!(path.aabb.contains([0.0, 0.0]));
    assert!(path.aabb.contains([1.0, 0.0]));
    assert!(path.aabb.contains([1.0, 1.0]));
    assert!(!path.aabb.contains([2.0, 1.0]));
    assert!(!path.aabb.contains([0.0, -1.0]));

    // counter-clockwise
    let path = PathBuilder::begin(&mut storage, [0.0, 0.0])
        .quadratic_bezier_to([1.0, 1.0], [1.0, 0.0]).close();
    assert_eq!(path.vertices, vertex_id_range(3, 6));
    assert_eq!(path.shape, Some(ShapeInfo {
        winding_order: WindingOrder::CounterClockwise,
        is_convex: true,
        is_y_monotone: true,
        has_beziers: true
    }));
    assert!(path.aabb.contains([0.0, 0.0]));
    assert!(path.aabb.contains([1.0, 0.0]));
    assert!(path.aabb.contains([1.0, 1.0]));
    assert!(!path.aabb.contains([2.0, 1.0]));
    assert!(!path.aabb.contains([0.0, -1.0]));

    // a slightly more elaborate path
    let path = PathBuilder::begin(&mut storage, [0.0, 0.0])
        .line_to([0.1, 0.0])
        .line_to([0.2, 0.1])
        .line_to([0.3, 0.1])
        .line_to([0.4, 0.0])
        .line_to([0.5, 0.0])
        .quadratic_bezier_to([0.5, 0.4], [0.3, 0.4])
        .line_to([0.1, 0.4])
        .quadratic_bezier_to([-0.2, 0.1], [-0.1, 0.0]) // TODO
        .close();
    assert_eq!(path.shape, Some(ShapeInfo {
        winding_order: WindingOrder::Clockwise,
        is_convex: false,
        is_y_monotone: false,
        has_beziers: true
    }));

    // simple non-convex but y-monotone path
    let path = PathBuilder::begin(&mut storage, [0.0, 0.0])
        .line_to([2.0, 1.0])
        .line_to([1.0, 2.0])
        .line_to([2.0, 3.0])
        .line_to([0.0, 4.0])
        .line_to([-2.0, 3.0])
        .line_to([-1.0, 2.0])
        .line_to([-2.0, 1.0])
        .close();
    assert_eq!(path.shape, Some(ShapeInfo {
        winding_order: WindingOrder::Clockwise,
        is_convex: false,
        is_y_monotone: true,
        has_beziers: false
    }));
    assert!(path.aabb.contains([2.0, 1.0]));
    assert!(path.aabb.contains([1.0, 2.0]));
    assert!(path.aabb.contains([2.0, 3.0]));
    assert!(path.aabb.contains([0.0, 4.0]));
    assert!(path.aabb.contains([-2.0, 3.0]));
    assert!(path.aabb.contains([-1.0, 2.0]));
    assert!(path.aabb.contains([-2.0, 1.0]));
    assert!(!path.aabb.contains([0.0, -0.1]));
}
