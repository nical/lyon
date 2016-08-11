//! Data structures to represent complex paths.
//!
//! This whole module will change at some point in order to implement a more
//! flexible and efficient Path data structure.

use super::{
    vertex_id, VertexId, VertexIdRange,
    VertexSlice, MutVertexSlice,
};

use path_builder::{ PrimitiveBuilder };
use path_iterator::{ PathIter };

use math::*;

use sid::{ Id, IdRange, ToIndex };

#[derive(Debug)]
/// Phatom type marker for PathId.
pub struct Path_;
/// An Id that represents a sub-path in a certain path object.
pub type PathId = Id<Path_, u16>;
/// A contiguous range of PathIds.
pub type PathIdRange = IdRange<Path_, u16>;
pub fn path_id(idx: u16) -> PathId { PathId::new(idx) }

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Verb {
    MoveTo,
    LineTo,
    QuadraticTo,
    CubicTo,
    Close,
}

#[derive(Clone, Debug)]
pub struct Path2 {
    vertices: Vec<Point>,
    verbs: Vec<Verb>,
}

#[derive(Copy, Clone, Debug)]
pub struct PathSlice2<'l> {
    vertices: &'l[Point],
    verbs: &'l[Verb],
}

impl Path2 {
    pub fn new() -> Path2 { Path2::with_capacity(128) }

    pub fn with_capacity(cap: usize) -> Path2 {
        Path2 {
            vertices: Vec::with_capacity(cap),
            verbs: Vec::with_capacity(cap),
        }
    }

    pub fn as_slice(&self) -> PathSlice2 {
        PathSlice2 { vertices: &self.vertices[..], verbs: &self.verbs[..] }
    }

    pub fn iter(&self) -> PathIter {
        PathIter::new(&self.vertices[..], &self.verbs[..])
    }
}

impl<'l> PathSlice2<'l> {
    pub fn new(vertices: &'l[Point], verbs: &'l[Verb]) -> PathSlice2<'l> {
        PathSlice2 { vertices: vertices, verbs: verbs }
    }

    pub fn iter(&self) -> PathIter { PathIter::new(self.vertices, self.verbs) }
}

pub struct PathBuilder {
    path: Path2,
    current_position: Point,
    first_position: Point,
    building: bool,
}

impl PathBuilder {
    pub fn new() -> PathBuilder { PathBuilder::with_capacity(128) }

    pub fn with_capacity(cap: usize) -> PathBuilder {
        PathBuilder {
            path: Path2::with_capacity(cap),
            current_position: Point::new(0.0, 0.0),
            first_position: Point::new(0.0, 0.0),
            building: false,
        }
    }
}

#[inline]
fn nan_check(p: Point) {
    debug_assert!(!p.x.is_nan());
    debug_assert!(!p.y.is_nan());
}

impl PrimitiveBuilder for PathBuilder {
    type PathType = Path2;

    fn move_to(&mut self, to: Point)
    {
        nan_check(to);
        if self.path.verbs.last() == Some(&Verb::MoveTo) {
            // previous op was also MoveTo, just overrwrite it.
            self.path.vertices.pop();
            self.path.verbs.pop();
        }
        self.first_position = to;
        self.current_position = to;
        self.building = true;
        self.path.vertices.push(to);
        self.path.verbs.push(Verb::MoveTo);
    }

    fn line_to(&mut self, to: Point) {
        nan_check(to);
        self.path.vertices.push(to);
        self.path.verbs.push(Verb::LineTo);
        self.current_position = to;
    }

    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
        nan_check(ctrl);
        nan_check(to);
        self.path.vertices.push(ctrl);
        self.path.vertices.push(to);
        self.path.verbs.push(Verb::QuadraticTo);
        self.current_position = to;
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
        nan_check(ctrl1);
        nan_check(ctrl2);
        nan_check(to);
        self.path.vertices.push(ctrl1);
        self.path.vertices.push(ctrl2);
        self.path.vertices.push(to);
        self.path.verbs.push(Verb::CubicTo);
        self.current_position = to;
    }

    fn close(&mut self) -> PathId {
        if self.path.verbs.last() == Some(&Verb::MoveTo) {
            // previous op was MoveTo we don't have a path to close, drop it.
            self.path.vertices.pop();
            self.path.verbs.pop();
        } else if self.path.verbs.last() == Some(&Verb::Close) {
            return path_id(0); // TODO
        }

        self.path.verbs.push(Verb::Close);
        self.current_position = self.first_position;
        self.building = false;
        path_id(0) // TODO
    }

    fn current_position(&self) -> Point { self.current_position }

    fn build(self) -> Path2 {
        //self.end();
        self.path
    }
}

#[cfg(test)]
use path_iterator::PrimitiveEvent;

#[test]
fn test_path_builder_1() {

    let mut p = PathBuilder::with_capacity(0);
    p.line_to(point(1.0, 0.0));
    p.line_to(point(2.0, 0.0));
    p.line_to(point(3.0, 0.0));
    p.quadratic_bezier_to(point(4.0, 0.0), point(4.0, 1.0));
    p.cubic_bezier_to(point(5.0, 0.0), point(5.0, 1.0), point(5.0, 2.0));
    p.close();

    p.move_to(point(10.0, 0.0));
    p.line_to(point(11.0, 0.0));
    p.line_to(point(12.0, 0.0));
    p.line_to(point(13.0, 0.0));
    p.quadratic_bezier_to(point(14.0, 0.0), point(14.0, 1.0));
    p.cubic_bezier_to(point(15.0, 0.0), point(15.0, 1.0), point(15.0, 2.0));
    p.close();

    p.close();
    p.move_to(point(1.0, 1.0));
    p.move_to(point(2.0, 2.0));
    p.move_to(point(3.0, 3.0));
    p.line_to(point(4.0, 4.0));

    let path = p.build();

    let mut it = path.iter();
    assert_eq!(it.next(), Some(PrimitiveEvent::LineTo(point(1.0, 0.0))));
    assert_eq!(it.next(), Some(PrimitiveEvent::LineTo(point(2.0, 0.0))));
    assert_eq!(it.next(), Some(PrimitiveEvent::LineTo(point(3.0, 0.0))));
    assert_eq!(it.next(), Some(PrimitiveEvent::QuadraticTo(point(4.0, 0.0), point(4.0, 1.0))));
    assert_eq!(it.next(), Some(PrimitiveEvent::CubicTo(point(5.0, 0.0), point(5.0, 1.0), point(5.0, 2.0))));
    assert_eq!(it.next(), Some(PrimitiveEvent::Close));

    assert_eq!(it.next(), Some(PrimitiveEvent::MoveTo(point(10.0, 0.0))));
    assert_eq!(it.next(), Some(PrimitiveEvent::LineTo(point(11.0, 0.0))));
    assert_eq!(it.next(), Some(PrimitiveEvent::LineTo(point(12.0, 0.0))));
    assert_eq!(it.next(), Some(PrimitiveEvent::LineTo(point(13.0, 0.0))));
    assert_eq!(it.next(), Some(PrimitiveEvent::QuadraticTo(point(14.0, 0.0), point(14.0, 1.0))));
    assert_eq!(it.next(), Some(PrimitiveEvent::CubicTo(point(15.0, 0.0), point(15.0, 1.0), point(15.0, 2.0))));
    assert_eq!(it.next(), Some(PrimitiveEvent::Close));

    assert_eq!(it.next(), Some(PrimitiveEvent::MoveTo(point(3.0, 3.0))));
    assert_eq!(it.next(), Some(PrimitiveEvent::LineTo(point(4.0, 4.0))));
    assert_eq!(it.next(), None);
    assert_eq!(it.next(), None);
    assert_eq!(it.next(), None);
}

#[test]
fn test_path_builder_empty() {
    let path = PathBuilder::new().build();
    let mut it = path.iter();
    assert_eq!(it.next(), None);
    assert_eq!(it.next(), None);
}

#[test]
fn test_path_builder_empty_move_to() {
    let mut p = PathBuilder::new();
    p.move_to(point(1.0, 2.0));
    p.move_to(point(3.0, 4.0));
    p.move_to(point(5.0, 6.0));

    let path = p.build();
    let mut it = path.iter();
    assert_eq!(it.next(), Some(PrimitiveEvent::MoveTo(point(5.0, 6.0))));
    assert_eq!(it.next(), None);
    assert_eq!(it.next(), None);
}

#[test]
fn test_path_builder_move_to_after_close() {
    let mut p = PathBuilder::new();
    p.line_to(point(1.0, 0.0));
    p.close();
    p.line_to(point(2.0, 0.0));

    let path = p.build();
    let mut it = path.iter();
    assert_eq!(it.next(), Some(PrimitiveEvent::LineTo(point(1.0, 0.0))));
    assert_eq!(it.next(), Some(PrimitiveEvent::Close));
    assert_eq!(it.next(), Some(PrimitiveEvent::LineTo(point(2.0, 0.0))));
    assert_eq!(it.next(), None);
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum PointType {
    Normal,
    Control,
}

// TODO: Need a better representation for paths. It needs to:
//  * be compact
//  * be stored in a contiguous buffer
//  * be extensible (add extra parameters to vertices)
//  * not store pointers
//  * be iterable but not necessarily random-accessible
#[derive(Copy, Clone, Debug)]
pub struct PointData {
    pub position: Vec2,
    pub point_type: PointType,
}

/// The data structure that represent a complex path.
///
/// This API is not stable yet. Both the data structure and the methods it exposes will
/// change, hopefully soon.
#[derive(Clone, Debug)]
pub struct Path {
    vertices: Vec<PointData>,
    sub_paths: Vec<PathInfo>,
}

impl Path {
    pub fn new() -> Path {
        Path { vertices: Vec::new(), sub_paths: Vec::new() }
    }

    pub fn from_vec(vertices: Vec<PointData>, sub_paths: Vec<PathInfo>) -> Path {
        Path {
            vertices: vertices,
            sub_paths: sub_paths,
        }
    }

    pub fn vertices(&self) -> VertexSlice<PointData> { VertexSlice::new(&self.vertices[..]) }

    pub fn mut_vertices(&mut self) -> MutVertexSlice<PointData> { MutVertexSlice::new(&mut self.vertices[..]) }

    pub fn num_vertices(&self) -> usize { self.as_slice().num_vertices() }

    pub fn sub_path(&self, id: PathId) -> SubPathSlice {
        SubPathSlice {
            vertices: VertexSlice::new(&self.vertices[..]),
            info: &self.sub_paths[id.handle.to_index()]
        }
    }

    pub fn path_ids(&self) -> PathIdRange {
        IdRange::new(0, self.sub_paths.len() as u16)
    }

    pub fn as_slice(&self) -> PathSlice {
        PathSlice {
            vertices: VertexSlice::new(&self.vertices[..]),
            sub_paths: &self.sub_paths[..],
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PathVertexId {
    pub vertex_id: VertexId,
    pub path_id: PathId,
}

pub struct PathVertexIdRange {
    pub range: VertexIdRange,
    pub path_id: PathId,
}

impl Iterator for PathVertexIdRange {
    type Item = PathVertexId;
    fn next(&mut self) -> Option<PathVertexId> {
        return if let Some(next) = self.range.next() {
            Some(PathVertexId {
                vertex_id: next,
                path_id: self.path_id
            })
        } else {
            None
        };
    }
}

#[derive(Copy, Clone)]
pub struct PathSlice<'l> {
    vertices: VertexSlice<'l,PointData>,
    sub_paths: &'l[PathInfo],
}

impl<'l> PathSlice<'l> {

    pub fn vertices(&self) -> VertexSlice<PointData> { self.vertices }

    pub fn vertex_ids(&self, sub_path: PathId) -> PathVertexIdRange {
        PathVertexIdRange {
            range: self.sub_path(sub_path).vertex_ids(),
            path_id: sub_path,
        }
    }

    pub fn num_vertices(&self) -> usize { self.vertices.len() }

    pub fn num_sub_paths(&self) -> usize { self.sub_paths.len() }

    pub fn sub_path(&self, id: PathId) -> SubPathSlice {
        SubPathSlice {
            vertices: self.vertices,
            info: &self.sub_paths[id.handle.to_index()]
        }
    }

    pub fn path_ids(&self) -> PathIdRange {
        IdRange::new(0, self.sub_paths.len() as u16)
    }

    pub fn vertex(&self, id: PathVertexId) -> &PointData {
        &self.vertices[id.vertex_id]
    }

    pub fn next(&self, id: PathVertexId) -> PathVertexId {
        PathVertexId {
            path_id: id.path_id,
            vertex_id: self.sub_path(id.path_id).next(id.vertex_id),
        }
    }

    pub fn previous(&self, id: PathVertexId) -> PathVertexId {
        PathVertexId {
            path_id: id.path_id,
            vertex_id: self.sub_path(id.path_id).previous(id.vertex_id),
        }
    }
}

#[derive(Copy, Clone)]
pub struct SubPathSlice<'l> {
    vertices: VertexSlice<'l, PointData>,
    info: &'l PathInfo,
}

impl<'l> SubPathSlice<'l> {
    pub fn info(&self) -> &'l PathInfo { self.info }

    pub fn vertex(&self, id: VertexId) -> &PointData { &self.vertices[id] }

    pub fn first(&self) -> VertexId { self.info.range.first }

    pub fn last(&self) -> VertexId {
        vertex_id(self.info.range.first.handle + self.info.range.count - 1)
    }

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

/// Some metadata for sub paths
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct PathInfo {
    pub aabb: Rect,
    pub range: VertexIdRange,
    pub has_beziers: Option<bool>,
    pub is_closed: bool,
}
