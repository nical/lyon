//! Data structures to represent complex paths.
//!
//! This whole module will change at some point in order to implement a more
//! flexible and efficient Path data structure.

extern crate lyon_core;
extern crate lyon_path_builder;
extern crate lyon_path_iterator;
extern crate sid;
extern crate sid_vec;


use lyon_path_builder::{ PrimitiveBuilder, SvgPathBuilder, FlattenedBuilder, Path_, PathId, path_id };

use lyon_core::PrimitiveEvent;
use lyon_core::math::*;

use sid::{ Id, IdRange, ToIndex };
use sid_vec::{ IdSlice, MutIdSlice, };

/// A contiguous range of PathIds.
pub type PathIdRange = IdRange<Path_, u16>;

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
    points: Vec<Point>,
    verbs: Vec<Verb>,
}

#[derive(Copy, Clone, Debug)]
pub struct PathSlice2<'l> {
    points: &'l[Point],
    verbs: &'l[Verb],
}

impl Path2 {
    pub fn new() -> Path2 { Path2::with_capacity(128) }

    pub fn with_capacity(cap: usize) -> Path2 {
        Path2 {
            points: Vec::with_capacity(cap),
            verbs: Vec::with_capacity(cap),
        }
    }

    pub fn as_slice(&self) -> PathSlice2 {
        PathSlice2 { points: &self.points[..], verbs: &self.verbs[..] }
    }

    pub fn iter(&self) -> PathIter {
        PathIter::new(&self.points[..], &self.verbs[..])
    }

    pub fn points(&self) -> &[Point] { &self.points[..] }

    pub fn mut_points(&mut self) -> &mut[Point] { &mut self.points[..] }

    pub fn verbs(&self) -> &[Verb] { &self.verbs[..] }
}

impl<'l> PathSlice2<'l> {
    pub fn new(points: &'l[Point], verbs: &'l[Verb]) -> PathSlice2<'l> {
        PathSlice2 { points: points, verbs: verbs }
    }

    pub fn iter(&self) -> PathIter { PathIter::new(self.points, self.verbs) }

    pub fn points(&self) -> &[Point] { self.points }

    pub fn verbs(&self) -> &[Verb] { self.verbs }
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
        //if self.path.verbs.last() == Some(&Verb::MoveTo) {
        //    // previous op was also MoveTo, just overrwrite it.
        //    self.path.vertices.pop();
        //    self.path.verbs.pop();
        //}
        self.first_position = to;
        self.current_position = to;
        self.building = true;
        self.path.points.push(to);
        self.path.verbs.push(Verb::MoveTo);
    }

    fn line_to(&mut self, to: Point) {
        nan_check(to);
        self.path.points.push(to);
        self.path.verbs.push(Verb::LineTo);
        self.current_position = to;
    }

    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
        nan_check(ctrl);
        nan_check(to);
        self.path.points.push(ctrl);
        self.path.points.push(to);
        self.path.verbs.push(Verb::QuadraticTo);
        self.current_position = to;
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
        nan_check(ctrl1);
        nan_check(ctrl2);
        nan_check(to);
        self.path.points.push(ctrl1);
        self.path.points.push(ctrl2);
        self.path.points.push(to);
        self.path.verbs.push(Verb::CubicTo);
        self.current_position = to;
    }

    fn close(&mut self) -> PathId {
        //if self.path.verbs.last() == Some(&Verb::MoveTo) {
        //    // previous op was MoveTo we don't have a path to close, drop it.
        //    self.path.points.pop();
        //    self.path.verbs.pop();
        //} else if self.path.verbs.last() == Some(&Verb::Close) {
        //    return path_id(0); // TODO
        //}

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

#[derive(Clone, Debug)]
pub struct PathIter<'l> {
    points: ::std::slice::Iter<'l, Point>,
    verbs: ::std::slice::Iter<'l, Verb>,
}

impl<'l> PathIter<'l> {
    pub fn new(points: &'l[Point], verbs: &'l[Verb]) -> Self {
        PathIter {
            points: points.iter(),
            verbs: verbs.iter(),
        }
    }
}

impl<'l> Iterator for PathIter<'l> {
    type Item = PrimitiveEvent;
    fn next(&mut self) -> Option<PrimitiveEvent> {
        return match self.verbs.next() {
            Some(&Verb::MoveTo) => {
                let to = *self.points.next().unwrap();
                Some(PrimitiveEvent::MoveTo(to))
            }
            Some(&Verb::LineTo) => {
                let to = *self.points.next().unwrap();
                Some(PrimitiveEvent::LineTo(to))
            }
            Some(&Verb::QuadraticTo) => {
                let ctrl = *self.points.next().unwrap();
                let to = *self.points.next().unwrap();
                Some(PrimitiveEvent::QuadraticTo(ctrl, to))
            }
            Some(&Verb::CubicTo) => {
                let ctrl1 = *self.points.next().unwrap();
                let ctrl2 = *self.points.next().unwrap();
                let to = *self.points.next().unwrap();
                Some(PrimitiveEvent::CubicTo(ctrl1, ctrl2, to))
            }
            Some(&Verb::Close) => {
                Some(PrimitiveEvent::Close)
            }
            None => { None }
        };
    }
}

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

// TODO: remove this
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

use lyon_core::math_utils::fuzzy_eq;

/// The integer type to index a vertex in a vertex buffer or path.
pub type Index = u16;

/// Phantom type marker for VertexId.
#[derive(Debug)]
pub struct Vertex_;

/// An Id that represents a vertex in a contiguous vertex buffer.
pub type VertexId = Id<Vertex_, Index>;

/// Create a VertexId from an u16 index.
#[inline]
pub fn vertex_id(index: Index) -> VertexId { VertexId::new(index) }

/// A range of VertexIds pointing to contiguous vertices.
pub type VertexIdRange = IdRange<Vertex_, Index>;

/// Create a VertexIdRange.
pub fn vertex_id_range(from: u16, to: u16) -> VertexIdRange {
    IdRange {
        first: Id::new(from),
        count: to - from,
    }
}

/// A slice of vertices indexed with VertexIds rather than usize offset.
pub type VertexSlice<'l, V> = IdSlice<'l, VertexId, V>;

/// A slice of mutable vertices indexed with VertexIds rather than usize offset.
pub type MutVertexSlice<'l, V> = MutIdSlice<'l, VertexId, V>;

/// Some metadata for sub paths
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct PathInfo {
    pub aabb: Rect,
    pub range: VertexIdRange,
    pub has_beziers: Option<bool>,
    pub is_closed: bool,
}

/// Builder for paths that can contain lines, and quadratic/cubic bezier segments.
pub type BezierPathBuilder = SvgPathBuilder<PrimitiveImpl>;

/// Builder for flattened paths
pub type FlattenedPathBuilder = SvgPathBuilder<FlattenedBuilder<PrimitiveImpl>>;
/// FlattenedPathBuilder constructor.
pub fn flattened_path_builder(tolerance: f32) -> FlattenedPathBuilder {
    SvgPathBuilder::new(FlattenedBuilder::new(PrimitiveImpl::new(), tolerance))
}

/// Builder for flattened paths
pub type FlattenedPathBuilder2 = SvgPathBuilder<FlattenedBuilder<PathBuilder>>;
/// FlattenedPathBuilder constructor.
pub fn flattened_path_builder2(tolerance: f32) -> FlattenedPathBuilder2 {
    SvgPathBuilder::new(FlattenedBuilder::new(PathBuilder::new(), tolerance))
}

/// BezierPathBuilder constructor.
pub fn bezier_path_builder() -> BezierPathBuilder {
    SvgPathBuilder::new(PrimitiveImpl::new())
}

/// Generates path objects with bezier segments
pub struct PrimitiveImpl {
    vertices: Vec<PointData>,
    path_info: Vec<PathInfo>,
    last_position: Point,
    top_left: Point,
    bottom_right: Point,
    offset: u16,
    // flags
    building: bool,
}

impl PrimitiveBuilder for PrimitiveImpl {
    type PathType = Path;

    fn move_to(&mut self, to: Point)
    {
        if self.building {
            self.end_sub_path(false);
        }
        self.top_left = to;
        self.bottom_right = to;
        self.push(to, PointType::Normal);
    }

    fn line_to(&mut self, to: Point) {
        self.push(to, PointType::Normal);
    }

    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
        self.push(ctrl, PointType::Control);
        self.push(to, PointType::Normal);
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
        self.push(ctrl1, PointType::Control);
        self.push(ctrl2, PointType::Control);
        self.push(to, PointType::Normal);
    }

    fn close(&mut self) -> PathId { self.end_sub_path(true) }

    fn current_position(&self) -> Point { self.last_position }

    fn build(mut self) -> Path {
        if self.building {
            self.end_sub_path(false);
        }
        return Path::from_vec(self.vertices, self.path_info);
    }
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
        let last = if last > 0 && fuzzy_eq(self.vertices[last].position, self.vertices[offset].position) {
            self.vertices.pop();
            closed = true;
            last - 1
        } else { last };

        let vertex_count = last + 1 - offset;

        if vertex_count == 0 {
            return path_id(self.path_info.len() as u16);
        }

        let vertex_range = vertex_id_range(self.offset, self.offset + vertex_count as u16);
        let aabb = Rect::new(self.top_left,
            size(self.bottom_right.x - self.top_left.x, self.bottom_right.y - self.top_left.y),
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



#[test]
fn test_path_builder_simple() {

    // clockwise
    {
        let mut path = flattened_path_builder(0.05);
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
        assert_eq!(info.aabb, Rect::new(vec2(0.0, 0.0), size(1.0, 1.0)));
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
        let mut path = flattened_path_builder(0.05);
        path.move_to(vec2(0.0, 0.0));
        path.line_to(vec2(1.0, 1.0));
        path.line_to(vec2(1.0, 0.0));
        let id = path.close();

        let path = path.build();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(vec2(0.0, 0.0), size(1.0, 1.0)));
    }

    // line_to back to the first vertex (should ignore the last vertex)
    {
        let mut path = flattened_path_builder(0.05);
        path.move_to(vec2(0.0, 0.0));
        path.line_to(vec2(1.0, 1.0));
        path.line_to(vec2(1.0, 0.0));
        path.line_to(vec2(0.0, 0.0));
        let id = path.close();

        let path = path.build();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(vec2(0.0, 0.0), size(1.0, 1.0)));
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
        assert_eq!(info.aabb, Rect::new(vec2(0.0, 0.0), size(1.0, 1.0)));
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
        assert_eq!(info.aabb, Rect::new(vec2(0.0, 0.0), size(1.0, 1.0)));
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
        assert_eq!(info.aabb, Rect::new(vec2(-0.2, 0.0), size(0.7, 0.4)));
    }
}

#[test]
fn test_arc_simple() {
    use lyon_core::ArcFlags;
    use lyon_path_builder::SvgBuilder;

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

#[test]
fn test_path_builder_empty_path() {
    let _ = flattened_path_builder(0.05).build();
}

#[test]
fn test_path_builder_empty_sub_path() {
    let mut builder = flattened_path_builder(0.05);
    builder.move_to(vec2(0.0, 0.0));
    builder.move_to(vec2(1.0, 0.0));
    builder.move_to(vec2(2.0, 0.0));
    let _ = builder.build();
}

#[test]
fn test_path_builder_close_empty() {
    let mut builder = flattened_path_builder(0.05);
    builder.close();
    builder.close();
    builder.close();
    let _ = builder.build();
}
