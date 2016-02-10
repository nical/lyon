
use vodk_id::id_vector::IdSlice;
use vodk_id::{ Id, IdRange };

use tesselation::vectors::{ Vec2, vec2_sub, directed_angle, Position2D, Rect };
use tesselation::{ Direction, WindingOrder, vertex_id, vertex_id_range, VertexId };

use std::f32::consts::PI;
use std::iter::{ FromIterator };

#[derive(Debug)]
pub struct Point_;
pub type PointId = Id<Point_, u16>;
pub fn point_id(idx: u16) -> PointId { PointId::new(idx) }

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Operator {
    Add,
    Substract,
    EvenOdd,
}

#[derive(Clone, Debug)]
pub struct PolygonInfo {
    pub aabb: Option<Rect>,
    pub is_convex: Option<bool>,
    pub is_y_monotone: Option<bool>,
    pub has_beziers: Option<bool>,
    pub op: Operator,
}

impl ::std::default::Default for PolygonInfo {
    fn default() -> PolygonInfo {
        PolygonInfo {
            aabb: None,
            is_convex: None,
            is_y_monotone: None,
            has_beziers: None,
            op: Operator::Add,
        }
    }
}

pub trait AbstractPolygon {
    type PointId: Copy + Eq + ::std::fmt::Debug;

    fn first_point(&self) -> Self::PointId;

    fn vertex(&self, point: Self::PointId) -> VertexId;

    fn next(&self, point: Self::PointId) -> Self::PointId;

    fn previous(&self, point: Self::PointId) -> Self::PointId;

    fn advance(&self, point: Self::PointId, dir: Direction) -> Self::PointId {
        return match dir {
            Direction::Forward => { self.next(point) }
            Direction::Backward => { self.previous(point) }
        };
    }

    fn next_vertex(&self, point: Self::PointId) -> VertexId {
        self.vertex(self.next(point))
    }

    fn previous_vertex(&self, point: Self::PointId) -> VertexId {
        self.vertex(self.previous(point))
    }

    // number of vertices on the loop containing point
    fn num_vertices_on_loop(&self, point: Self::PointId) -> usize;

    // number of vertices total
    fn num_vertices(&self) -> usize;

    fn get_sub_polygon<'l>(&'l self, id: PolygonId) -> Option<PolygonSlice<'l>>;

    fn is_complex(&self) -> bool;

    fn as_slice(&self) -> Option<PolygonSlice>;
}

#[derive(Copy, Clone)]
pub struct PolygonSlice<'l> {
    vertices: &'l [VertexId],
    info: &'l PolygonInfo,
}

impl<'l> PolygonSlice<'l> {
    pub fn info(self) -> &'l PolygonInfo { self.info }

    pub fn point_ids(self) -> IdRange<Point_, u16> {
        IdRange {
            first: point_id(0),
            count: self.vertices.len() as u16
        }
    }
}

impl<'l> AbstractPolygon for PolygonSlice<'l> {
    type PointId = PointId;

    fn first_point(&self) -> PointId { point_id(0) }

    fn vertex(&self, point: PointId) -> VertexId {
        self.vertices[point.handle as usize]
    }

    fn next(&self, point: PointId) -> PointId {
        point_id((point.handle + 1) % (self.vertices.len() as u16))
    }

    fn previous(&self, point: PointId) -> PointId {
        point_id(if point.handle == 0 { self.vertices.len() as u16 - 1 }  else { point.handle - 1 })
    }

    fn num_vertices(&self) -> usize { self.vertices.len() }

    fn num_vertices_on_loop(&self, _point: PointId) -> usize { self.num_vertices() }

    fn get_sub_polygon<'m>(&'m self, _: PolygonId) -> Option<PolygonSlice<'m>> { None }

    fn is_complex(&self) -> bool { false }

    fn as_slice(&self) -> Option<PolygonSlice> { Some(*self) }
}

#[derive(Clone)]
pub struct Polygon {
    pub vertices: Vec<VertexId>,
    pub info: PolygonInfo,
}

impl Polygon {
    pub fn new() -> Polygon {
        Polygon {
            vertices: Vec::new(),
            info: PolygonInfo::default(),
        }
    }

    pub fn from_vertices<It: Iterator<Item=VertexId>>(it: It) -> Polygon {
        let (lower_bound, _) = it.size_hint();
        let mut v = Vec::with_capacity(lower_bound);
        v.extend(it);
        Polygon {
            vertices: v,
            info: PolygonInfo::default(),
        }
    }

    pub fn from_slice(slice: PolygonSlice) -> Polygon {
        Polygon {
            // TODO: there's a more efficient way to copy a slice into a vec.
            vertices: Vec::from_iter(slice.vertices.iter().cloned()),
            info: slice.info().clone(),
        }
    }

    pub fn info(&self) -> &PolygonInfo { &self.info }

    pub fn is_empty(&self) -> bool { self.vertices.is_empty() }

    pub fn into_complex_polygon(self) -> ComplexPolygon {
        ComplexPolygon {
            sub_polygons: Vec::new()
        }
    }

    /// Add vertex to the end
    pub fn push_vertex(&mut self, v: VertexId) -> PointId {
        self.vertices.push(v);
        return Id::new(self.vertices.len() as u16 - 1);
    }

    /// Add vertex to the end
    pub fn remove_vertex(&mut self, v: PointId) -> VertexId {
        self.vertices.remove(v.handle as usize)
    }

    /// Retains only the elements specified by the predicate (seimilar to std::vec::Vec::retain).
    pub fn retain_vertices<F>(&mut self, f: F) where F: FnMut(&VertexId) -> bool {
        self.vertices.retain(f)
    }

    /// Insert a vertex for a given point_id shifting all elements after that position to the right.
    pub fn insert_vertex(&mut self, point: PointId, new_vertex: VertexId) {
        self.vertices.insert(point.handle as usize, new_vertex);
    }

    pub fn slice<'l>(&'l self) -> PolygonSlice<'l> {
        PolygonSlice { vertices: &self.vertices[..], info: &self.info }
    }
}

impl AbstractPolygon for Polygon {
    type PointId = PointId;

    fn first_point(&self) -> PointId { point_id(0) }

    fn vertex(&self, point: PointId) -> VertexId { self.vertices[point.handle as usize] }

    fn next(&self, point: PointId) -> PointId { self.slice().next(point) }

    fn previous(&self, point: PointId) -> PointId { self.slice().previous(point) }

    fn num_vertices(&self) -> usize { self.vertices.len() }

    fn num_vertices_on_loop(&self, _point: PointId) -> usize { self.num_vertices() }

    fn get_sub_polygon<'l>(&'l self, _: PolygonId) -> Option<PolygonSlice<'l>> { None }

    fn is_complex(&self) -> bool { false }

    fn as_slice(&self) -> Option<PolygonSlice> { Some(self.slice()) }
}

pub struct ComplexPolygon {
    pub sub_polygons: Vec<Polygon>
}

impl ComplexPolygon {
    pub fn new() -> ComplexPolygon {
        ComplexPolygon {
            sub_polygons: Vec::new(),
        }
    }

    pub fn add_hole(&mut self, mut hole: Polygon) {
        hole.info.op = Operator::Substract;
        self.sub_polygons.push(hole);
    }

    pub fn polygon(&self, id: PolygonId) -> &Polygon {
        return &self.sub_polygons[id.handle as usize];
    }

    pub fn point_ids(&self, p: PolygonId) -> ComplexPointIdRange {
        ComplexPointIdRange {
            range: IdRange {
                first: point_id(0),
                count: self.polygon(p).num_vertices() as u16
            },
            polygon_id: p,
        }
    }

    pub fn polygon_ids(&self) -> IdRange<Polygon_, u16> {
        IdRange {
            first: polygon_id(0),
            count: self.sub_polygons.len() as u16,
        }
    }
}

pub struct ComplexPointIdRange {
    range: IdRange<Point_, u16>,
    polygon_id: PolygonId,
}

impl Iterator for ComplexPointIdRange {
    type Item = ComplexPointId;
    fn next(&mut self) -> Option<ComplexPointId> {
        return if let Some(next) = self.range.next() {
            Some(ComplexPointId {
                point: next,
                polygon_id: self.polygon_id
            })
        } else {
            None
        };
    }
}

impl AbstractPolygon for ComplexPolygon {
    type PointId = ComplexPointId;

    fn first_point(&self) -> ComplexPointId {
        ComplexPointId { point: self.sub_polygons[0].first_point(), polygon_id: polygon_id(0) }
    }

    fn vertex(&self, id: ComplexPointId) -> VertexId {
        self.polygon(id.polygon_id).vertex(id.point)
    }

    fn next(&self, id: ComplexPointId) -> ComplexPointId {
        ComplexPointId {
            point: self.polygon(id.polygon_id).next(id.point),
            polygon_id: id.polygon_id
        }
    }

    fn previous(&self, id: ComplexPointId) -> ComplexPointId {
        ComplexPointId {
            point: self.polygon(id.polygon_id).previous(id.point),
            polygon_id: id.polygon_id
        }
    }

    fn num_vertices(&self) -> usize {
        let mut result = 0;
        for hole in &self.sub_polygons {
            result += hole.num_vertices();
        }
        return result;
    }

    fn num_vertices_on_loop(&self, point: ComplexPointId) -> usize {
        self.polygon(point.polygon_id).num_vertices()
    }

    fn get_sub_polygon<'l>(&'l self, id: PolygonId) -> Option<PolygonSlice<'l>> {
        if id.handle <= self.sub_polygons.len() as u16 {
            return Some(self.sub_polygons[id.handle as usize].slice());
        }

        return None;
    }

    fn is_complex(&self) -> bool { true }

    fn as_slice(&self) -> Option<PolygonSlice> {
        if self.sub_polygons.len() == 1 {
            return Some(self.sub_polygons[0].slice());
        }
        return None;
    }
}

#[derive(Copy, Clone)]
pub struct PolygonCirculator<'l> {
    polygon: PolygonSlice<'l>,
    point: PointId,
}

impl<'l> PolygonCirculator<'l> {
    pub fn next_vertex(self) -> VertexId {
        self.polygon.next_vertex(self.point)
    }

    pub fn previous_vertex(self) -> VertexId {
        self.polygon.previous_vertex(self.point)
    }

    pub fn advance(&mut self, dir: Direction) {
        self.point = self.polygon.advance(self.point, dir);
    }

    pub fn iter(self) -> PolygonIterator<'l> {
        PolygonIterator {
            polygon: self.polygon,
            first: self.point.handle,
            count: 0
        }
    }
}

#[derive(Copy, Clone)]
pub struct PolygonIterator<'l> {
    polygon: PolygonSlice<'l>,
    first: u16,
    count: u16,
}

impl<'l> Iterator for PolygonIterator<'l> {
    type Item = VertexId;

    fn next(&mut self) -> Option<VertexId> {
        let num_vertices = self.polygon.num_vertices();
        if self.count as usize >= num_vertices {
            return None;
        }

        let idx = (self.count + self.first) as usize % num_vertices;
        self.count += 1;

        return Some(self.polygon.vertices[idx]);
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let num_vertices = self.polygon.num_vertices();
        (num_vertices, Some(num_vertices))
    }
}

#[derive(Copy, Clone)]
pub struct ComplexPolygonIterator<'l> {
    iter: PolygonIterator<'l>,
    polygon_id: PolygonId,
}

impl<'l> Iterator for ComplexPolygonIterator<'l> {
    type Item = VertexId;

    fn next(&mut self) -> Option<VertexId> { self.iter.next() }

    fn size_hint(&self) -> (usize, Option<usize>) { self.iter.size_hint() }
}


#[derive(Debug)]
pub struct Polygon_;
pub type PolygonId = Id<Polygon_, u16>;
pub fn polygon_id(idx: u16) -> PolygonId { PolygonId::new(idx) }

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct ComplexPointId {
    pub point: PointId,
    pub polygon_id: PolygonId,
}

pub fn compute_winding_order<'l, Pos: Position2D>(
    poly: PolygonSlice<'l>,
    vertices: IdSlice<VertexId, Pos>
) -> Option<WindingOrder> {
    if poly.num_vertices() < 3 {
        return None;
    }

    let mut angle = 0.0;
    for it in poly.point_ids() {
        let a = vertices[poly.previous_vertex(it)].position();
        let b = vertices[poly.vertex(it)].position();
        let c = vertices[poly.next_vertex(it)].position();

        angle += directed_angle(vec2_sub(a, b), vec2_sub(c, b));
    }

    return if angle > ((poly.num_vertices()-1) as f32) * PI {
        Some(WindingOrder::Clockwise)
    } else {
        Some(WindingOrder::CounterClockwise)
    };
}

#[test]
fn test_simple_polygon() {
  let poly = Polygon {
    vertices: vec![
        vertex_id(0),
        vertex_id(1),
        vertex_id(2),
        vertex_id(3),
        vertex_id(4),
    ],
    info: PolygonInfo::default(),
  };

  let _ = poly.into_complex_polygon();
}

#[test]
fn test_winding_order()
{
    let positions: &[Vec2] = &[
        [0.0, 0.0],
        [0.0,-1.0],
        [0.0,-2.0],
        [1.0,-2.0],
        [2.0,-2.0],
        [2.0,-1.0],
        [2.0, 0.0],
        [1.0, 0.0],
    ];
    let vertices = IdSlice::new(positions);
    let poly = Polygon::from_vertices(vertex_id_range(0, 8));
    assert_eq!(compute_winding_order(poly.slice(), vertices), Some(WindingOrder::Clockwise));

    let positions: &[Vec2] = &[
        [1.0, 0.0],
        [2.0, 0.0],
        [2.0,-1.0],
        [2.0,-2.0],
        [1.0,-2.0],
        [0.0,-2.0],
        [0.0,-1.0],
        [0.0, 0.0],
    ];
    let vertices = IdSlice::new(positions);
    let poly = Polygon::from_vertices(vertex_id_range(0, 8));
    assert_eq!(compute_winding_order(poly.slice(), vertices), Some(WindingOrder::CounterClockwise));

}
