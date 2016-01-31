use std::f32::consts::PI;
use tesselation::vectors::{ Vec2, vec2_sub, vec2_almost_eq, directed_angle, Position2D };
use tesselation::{ vertex_id, vertex_id_range, VertexId, VertexIdRange };
use tesselation::monotone::{ is_y_monotone, DecompositionContext, TriangulationContext, };
use tesselation::vertex_builder::{ VertexBufferBuilder };
use tesselation::bezier::{ separate_bezier_faces, triangulate_quadratic_bezier };

use tesselation::polygon::*;
use tesselation::polygon_partition::{ partition_polygon, Diagonals };

use vodk_id::id_vector::IdSlice;
use vodk_id::ReverseIdRange;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum WindingOrder {
    Clockwise,
    CounterClockwise,
    Unknown,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum PointType {
    Normal,
    Control,
}

pub struct PointData {
    pub position: Vec2,
    pub point_type: PointType,
}

impl Position2D for PointData { fn position(&self) -> Vec2 { self.position } }

pub struct PathBuilder<'l> {
    vertices: &'l mut Vec<PointData>,
    last_position: [f32; 2],
    accum_angle: f32,
    offset: u16,
}

pub struct PathInfo {
    pub vertices: VertexIdRange,
    pub winding: WindingOrder
}

impl<'l> PathBuilder<'l> {
    pub fn begin(storage: &'l mut Vec<PointData>, pos: Vec2) -> PathBuilder {
        let offset = storage.len() as u16;
        storage.push(PointData { position: pos, point_type: PointType::Normal });
        PathBuilder {
            vertices: storage,
            last_position: [::std::f32::NAN, ::std::f32::NAN],
            accum_angle: 0.0,
            offset: offset,
        }
    }

    pub fn line_to(mut self, to: Vec2) -> PathBuilder<'l> {
        self.push(to, PointType::Normal);
        return self;
    }

    pub fn quadratic_bezier_to(mut self, ctrl: Vec2, to: Vec2) -> PathBuilder<'l> {
        self.push(ctrl, PointType::Control);
        self.push(to, PointType::Normal);
        return self;
    }

    pub fn cubic_bezier_to(mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) -> PathBuilder<'l> {
        self.push(ctrl1, PointType::Control);
        self.push(ctrl2, PointType::Control);
        self.push(to, PointType::Normal);
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
        let winding = if vertex_count > 2 {
            self.accum_angle += directed_angle(
                vec2_sub(self.vertices[last - 1].position, self.vertices[last].position),
                vec2_sub(self.vertices[offset].position, self.vertices[last].position)
            );
            self.accum_angle += directed_angle(
                vec2_sub(self.vertices[last].position, self.vertices[offset].position),
                vec2_sub(self.vertices[offset + 1].position, self.vertices[offset].position)
            );

            if self.accum_angle > ((vertex_count-1) as f32) * PI {
                WindingOrder::Clockwise
            } else {
                WindingOrder::CounterClockwise
            }
        } else {
            WindingOrder::Unknown
        };

        return PathInfo {
            vertices: vertex_id_range(self.offset, self.offset + vertex_count as u16),
            winding: winding,
        };
    }

    fn push(&mut self, point: Vec2, ptype: PointType) {
        if point == self.last_position {
            return;
        }
        self.vertices.push(PointData{ position: point, point_type: ptype });
        self.update_angle();
        self.last_position = point;
    }

    fn update_angle(&mut self) {
        if self.vertices.len() - (self.offset as usize) > 2 {
            let last = self.vertices.len() - 1;
            self.accum_angle += directed_angle(
                vec2_sub(self.vertices[last - 2].position, self.vertices[last - 1].position),
                vec2_sub(self.vertices[last].position, self.vertices[last - 1].position)
            );
        }
    }
}

impl PathInfo {
    pub fn create_polygon(&self) -> Polygon {
        return match self.winding {
            WindingOrder::Clockwise => { Polygon::from_vertices(self.vertices) }
            WindingOrder::CounterClockwise => { Polygon::from_vertices(ReverseIdRange::new(self.vertices)) }
            _ => { panic!("Not implemented yet!"); }
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

    let mut polygon = ComplexPolygon{
        main: path.create_polygon(),
        holes: vec![],
    };

    for hole in holes {
        if hole.winding == WindingOrder::CounterClockwise {
            polygon.holes.push(Polygon::from_vertices(hole.vertices));
        } else {
            polygon.holes.push(Polygon::from_vertices(ReverseIdRange::new(hole.vertices)));
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

    println!(" ---- num points {}", polygon.num_vertices());

    let res = ctx.y_monotone_polygon_decomposition(&polygon, vertex_positions, &mut diagonals);
    assert_eq!(res, Ok(()));

    let mut monotone_polygons = Vec::new();
    partition_polygon(&polygon, vertex_positions, &mut diagonals, &mut monotone_polygons);
    println!(" -- there are {} monotone polygons", monotone_polygons.len());

    let mut triangulator = TriangulationContext::new();
    for monotone_poly in monotone_polygons {
        assert!(is_y_monotone(monotone_poly.view(), vertex_positions));
        let res = triangulator.y_monotone_triangulation(monotone_poly.view(), vertex_positions, output);
        assert_eq!(res, Ok(()));
    }

    for b in beziers {
        println!(" -- adding bezier loop");
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
    assert_eq!(path.winding, WindingOrder::Clockwise);
    assert_eq!(storage[0].position, [0.0, 0.0]);
    assert_eq!(storage[1].position, [1.0, 0.0]);
    assert_eq!(storage[2].position, [1.0, 1.0]);
    assert_eq!(storage[0].point_type, PointType::Normal);
    assert_eq!(storage[1].point_type, PointType::Normal);
    assert_eq!(storage[2].point_type, PointType::Normal);

    // counter-clockwise
    let path = PathBuilder::begin(&mut storage, [0.0, 0.0])
        .line_to([1.0, 1.0]).line_to([1.0, 0.0]).close();
    assert_eq!(path.vertices, vertex_id_range(3, 6));
    assert_eq!(path.winding, WindingOrder::CounterClockwise);

    // line_to back to the first vertex (should ignore the last vertex)
    let path = PathBuilder::begin(&mut storage, [0.0, 0.0])
        .line_to([1.0, 1.0]).line_to([1.0, 0.0]).line_to([0.0, 0.0]).close();
    assert_eq!(path.vertices, vertex_id_range(6, 9));
    assert_eq!(path.winding, WindingOrder::CounterClockwise);
}

#[test]
fn test_path_builder_simple_bezier() {
    let mut storage = vec![];

    // clockwise
    let path = PathBuilder::begin(&mut storage, [0.0, 0.0])
        .quadratic_bezier_to([1.0, 0.0], [1.0, 1.0]).close();
    assert_eq!(path.vertices, vertex_id_range(0, 3));
    assert_eq!(path.winding, WindingOrder::Clockwise);

    // counter-clockwise
    let path = PathBuilder::begin(&mut storage, [0.0, 0.0])
        .quadratic_bezier_to([1.0, 1.0], [1.0, 0.0]).close();
    assert_eq!(path.vertices, vertex_id_range(3, 6));
    assert_eq!(path.winding, WindingOrder::CounterClockwise);
}
