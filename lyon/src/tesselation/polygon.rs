
use vodk_id::{ Id, IdRange, IdSlice };

use tesselation::vectors::{ Position2D };
use tesselation::{ Direction, WindingOrder, VertexId };

use vodk_math::{ Rect };

use std::f32::consts::PI;
use std::iter::{ FromIterator };

#[derive(Debug)]
pub struct Point_;
pub type PointId = Id<Point_, u16>;
pub fn point_id(idx: u16) -> PointId { PointId::new(idx) }

#[derive(Debug)]
pub struct Polygon_;
pub type PolygonId = Id<Polygon_, u16>;
pub fn polygon_id(idx: u16) -> PolygonId { PolygonId::new(idx) }

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct ComplexPointId {
    pub point: PointId,
    pub polygon_id: PolygonId,
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

    // number of vertices total
    fn num_vertices(&self) -> usize;

    fn is_complex(&self) -> bool;

    fn as_simple_polygon(&self) -> Option<PolygonSlice>;
}

pub trait AbstractPolygonSlice : AbstractPolygon + Copy {}

pub trait PolygonMut : AbstractPolygon {
    fn push_vertex(&mut self, v: VertexId) -> Option<Self::PointId>;

    fn remove_vertex(&mut self, v: PointId);

    fn clear(&mut self);

    fn num_vertices(&self) -> usize;

    fn max_vertices(&self) -> Option<usize>;
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

    fn is_complex(&self) -> bool { false }

    fn as_simple_polygon(&self) -> Option<PolygonSlice> { Some(*self) }
}

impl<'l> AbstractPolygonSlice for PolygonSlice<'l> {}

pub struct PolygonSliceMut<'l> {
    vertices: &'l mut[VertexId],
    info: &'l mut PolygonInfo,
    num_vertices: u16,
}

#[derive(Clone)]
pub struct Polygon {
    pub vertices: Vec<VertexId>,
    pub info: PolygonInfo,
}

impl<'l> PolygonSliceMut<'l> {
    pub fn info(self) -> &'l PolygonInfo { self.info }

    pub fn point_ids(self) -> IdRange<Point_, u16> { self.as_slice().point_ids() }

    pub fn as_slice(&self) -> PolygonSlice {
        PolygonSlice {
            vertices: &self.vertices[0..self.num_vertices as usize],
            info: self.info,
        }
    }
}

impl<'l> AbstractPolygon for PolygonSliceMut<'l> {
    type PointId = PointId;

    fn first_point(&self) -> PointId { point_id(0) }

    fn vertex(&self, point: PointId) -> VertexId { self.as_slice().vertex(point) }

    fn next(&self, point: PointId) -> PointId { self.as_slice().next(point) }

    fn previous(&self, point: PointId) -> PointId { self.as_slice().previous(point) }

    fn num_vertices(&self) -> usize { self.num_vertices as usize }

    fn is_complex(&self) -> bool { false }

    fn as_simple_polygon(&self) -> Option<PolygonSlice> { Some(self.as_slice()) }
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
        let count = self.vertices.len() as u16;
        ComplexPolygon {
            vertices: self.vertices,
            sub_polygons: vec![SubPolygonInfo {
                info: self.info,
                first: 0, count: count,
            }],
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

    pub fn as_slice<'l>(&'l self) -> PolygonSlice<'l> {
        PolygonSlice { vertices: &self.vertices[..], info: &self.info }
    }
}

impl AbstractPolygon for Polygon {
    type PointId = PointId;

    fn first_point(&self) -> PointId { point_id(0) }

    fn vertex(&self, point: PointId) -> VertexId { self.vertices[point.handle as usize] }

    fn next(&self, point: PointId) -> PointId { self.as_slice().next(point) }

    fn previous(&self, point: PointId) -> PointId { self.as_slice().previous(point) }

    fn num_vertices(&self) -> usize { self.vertices.len() }

    fn is_complex(&self) -> bool { false }

    fn as_simple_polygon(&self) -> Option<PolygonSlice> { Some(self.as_slice()) }
}

pub struct SubPolygonInfo {
    info: PolygonInfo,
    first: u16,
    count: u16,
}

pub struct ComplexPolygon {
    vertices: Vec<VertexId>,
    sub_polygons: Vec<SubPolygonInfo>
}

impl ComplexPolygon {
    pub fn new() -> ComplexPolygon {
        ComplexPolygon {
            vertices: Vec::with_capacity(256),
            sub_polygons: Vec::new(),
        }
    }

    pub fn polygon(&self, id: PolygonId) -> PolygonSlice {
        let p = &self.sub_polygons[id.handle as usize];
        return PolygonSlice {
            vertices: &self.vertices[p.first as usize..p.first as usize + p.count as usize],
            info: &p.info
        }
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

    pub fn add_sub_polygon<IT: Iterator<Item=VertexId>>(&mut self, it: IT, info: PolygonInfo) -> PolygonId {
        let first = self.vertices.len();
        self.vertices.extend(it);
        let last = self.vertices.len();
        self.sub_polygons.push(SubPolygonInfo {
            first: first as u16,
            count: (last - first) as u16,
            info: info
        });
        return polygon_id(self.sub_polygons.len() as u16 -1 )
    }

    pub fn as_slice(&self) -> ComplexPolygonSlice {
        ComplexPolygonSlice {
            vertices: &self.vertices[..],
            sub_polygons: &self.sub_polygons[..]
        }
    }

    pub fn as_simple_polygon(&self) -> Option<PolygonSlice> {
        if self.sub_polygons.len() == 1 {
            return Some(self.polygon(polygon_id(0)));
        }
        return None;
    }
}

#[derive(Copy, Clone)]
pub struct ComplexPolygonSlice<'l> {
    vertices: &'l[VertexId],
    sub_polygons: &'l[SubPolygonInfo]
}

impl<'l> ComplexPolygonSlice<'l> {
    pub fn polygon(&self, id: PolygonId) -> PolygonSlice {
        let p = &self.sub_polygons[id.handle as usize];
        return PolygonSlice {
            vertices: &self.vertices[p.first as usize..p.first as usize + p.count as usize],
            info: &p.info
        }
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

impl<'l> AbstractPolygon for ComplexPolygonSlice<'l> {
    type PointId = ComplexPointId;

    fn first_point(&self) -> ComplexPointId {
        ComplexPointId { point: point_id(0), polygon_id: polygon_id(0) }
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
        for p in self.polygon_ids() {
            result += self.polygon(p).num_vertices();
        }
        return result;
    }

    fn is_complex(&self) -> bool { true }

    fn as_simple_polygon(&self) -> Option<PolygonSlice> {
        if self.sub_polygons.len() == 1 {
            return Some(self.polygon(polygon_id(0)));
        }
        return None;
    }
}

impl<'l> AbstractPolygonSlice for ComplexPolygonSlice<'l> {}

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
    fn default() -> PolygonInfo { PolygonInfo::new() }
}

impl PolygonInfo {
    pub fn new() -> PolygonInfo {
        PolygonInfo {
            aabb: None,
            is_convex: None,
            is_y_monotone: None,
            has_beziers: None,
            op: Operator::Add,
        }
    }

    pub fn with_aabb(mut self, aabb: Rect) -> PolygonInfo {
        self.aabb = Some(aabb);
        return self;
    }

    pub fn with_is_convex(mut self, convex: bool) -> PolygonInfo {
        self.is_convex = Some(convex);
        return self;
    }

    pub fn with_is_y_monotone(mut self, mnotone: bool) -> PolygonInfo {
        self.is_y_monotone = Some(mnotone);
        return self;
    }
}

#[cfg(test)]
fn compute_winding_order<'l, Pos: Position2D>(
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

        angle += (a - b).directed_angle(c - b);
    }

    return if angle > ((poly.num_vertices()-1) as f32) * PI {
        Some(WindingOrder::Clockwise)
    } else {
        Some(WindingOrder::CounterClockwise)
    };
}

#[cfg(test)]
use vodk_math::{ Vec2, vec2 };
#[cfg(test)]
use tesselation::{ vertex_id, vertex_id_range };

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
        vec2(0.0, 0.0),
        vec2(0.0,-1.0),
        vec2(0.0,-2.0),
        vec2(1.0,-2.0),
        vec2(2.0,-2.0),
        vec2(2.0,-1.0),
        vec2(2.0, 0.0),
        vec2(1.0, 0.0),
    ];
    let vertices = IdSlice::new(positions);
    let poly = Polygon::from_vertices(vertex_id_range(0, 8));
    assert_eq!(compute_winding_order(poly.as_slice(), vertices), Some(WindingOrder::Clockwise));

    let positions: &[Vec2] = &[
        vec2(1.0, 0.0),
        vec2(2.0, 0.0),
        vec2(2.0,-1.0),
        vec2(2.0,-2.0),
        vec2(1.0,-2.0),
        vec2(0.0,-2.0),
        vec2(0.0,-1.0),
        vec2(0.0, 0.0),
    ];
    let vertices = IdSlice::new(positions);
    let poly = Polygon::from_vertices(vertex_id_range(0, 8));
    assert_eq!(compute_winding_order(poly.as_slice(), vertices), Some(WindingOrder::CounterClockwise));

}
