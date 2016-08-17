//! Data structures to represent complex paths.
//!
//! This whole module will change at some point in order to implement a more
//! flexible and efficient Path data structure.

extern crate lyon_core;
extern crate lyon_path_builder;
extern crate lyon_path_iterator;

use lyon_path_builder::{ PrimitiveBuilder, SvgPathBuilder, FlattenedBuilder };

use lyon_core::PrimitiveEvent;
use lyon_core::math::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Verb {
    MoveTo,
    LineTo,
    QuadraticTo,
    CubicTo,
    Close,
}

#[derive(Clone, Debug)]
pub struct Path {
    points: Vec<Point>,
    verbs: Vec<Verb>,
}

#[derive(Copy, Clone, Debug)]
pub struct PathSlice<'l> {
    points: &'l[Point],
    verbs: &'l[Verb],
}

impl Path {
    pub fn new() -> Path { Path::with_capacity(128) }

    pub fn with_capacity(cap: usize) -> Path {
        Path {
            points: Vec::with_capacity(cap),
            verbs: Vec::with_capacity(cap),
        }
    }

    pub fn as_slice(&self) -> PathSlice {
        PathSlice { points: &self.points[..], verbs: &self.verbs[..] }
    }

    pub fn iter(&self) -> PathIter {
        PathIter::new(&self.points[..], &self.verbs[..])
    }

    pub fn points(&self) -> &[Point] { &self.points[..] }

    pub fn mut_points(&mut self) -> &mut[Point] { &mut self.points[..] }

    pub fn verbs(&self) -> &[Verb] { &self.verbs[..] }
}

impl<'l> PathSlice<'l> {
    pub fn new(points: &'l[Point], verbs: &'l[Verb]) -> PathSlice<'l> {
        PathSlice { points: points, verbs: verbs }
    }

    pub fn iter(&self) -> PathIter { PathIter::new(self.points, self.verbs) }

    pub fn points(&self) -> &[Point] { self.points }

    pub fn verbs(&self) -> &[Verb] { self.verbs }
}

pub struct PathBuilder {
    path: Path,
    current_position: Point,
    first_position: Point,
    building: bool,
}

impl PathBuilder {
    pub fn new() -> PathBuilder { PathBuilder::with_capacity(128) }

    pub fn with_capacity(cap: usize) -> PathBuilder {
        PathBuilder {
            path: Path::with_capacity(cap),
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
    type PathType = Path;

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

    fn close(&mut self) {
        //if self.path.verbs.last() == Some(&Verb::MoveTo) {
        //    // previous op was MoveTo we don't have a path to close, drop it.
        //    self.path.points.pop();
        //    self.path.verbs.pop();
        //} else if self.path.verbs.last() == Some(&Verb::Close) {
        //    return;
        //}

        self.path.verbs.push(Verb::Close);
        self.current_position = self.first_position;
        self.building = false;
    }

    fn current_position(&self) -> Point { self.current_position }

    fn build(self) -> Path {
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

/// Builder for flattened paths
pub type FlattenedPathBuilder2 = SvgPathBuilder<FlattenedBuilder<PathBuilder>>;
/// FlattenedPathBuilder constructor.
pub fn flattened_path_builder(tolerance: f32) -> FlattenedPathBuilder2 {
    SvgPathBuilder::new(FlattenedBuilder::new(PathBuilder::new(), tolerance))
}

/*
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
*/