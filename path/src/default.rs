use builder::{FlatPathBuilder, PathBuilder, SvgPathBuilder, FlatteningBuilder};
use iterator::PathIter;

use PathEvent;
use math::*;

use std::iter::IntoIterator;

/// Enumeration corresponding to the [PathEvent](https://docs.rs/lyon_core/*/lyon_core/events/enum.PathEvent.html) enum
/// without the parameters.
///
/// This is used by the [Path](struct.Path.html) data structure to store path events a tad
/// more efficiently.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum Verb {
    MoveTo,
    LineTo,
    QuadraticTo,
    CubicTo,
    Arc,
    Close,
}

/// A simple path data structure.
///
/// It can be created using a [Builder](struct.Builder.html), and can be iterated over.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Path {
    points: Vec<Point>,
    verbs: Vec<Verb>,
}

#[derive(Copy, Clone, Debug)]
pub struct PathSlice<'l> {
    points: &'l [Point],
    verbs: &'l [Verb],
}

impl Path {
    /// Creates a [Builder](struct.Builder.html) to create a path.
    pub fn builder() -> Builder { Builder::new() }

    pub fn new() -> Path {
        Path {
            points: Vec::new(),
            verbs: Vec::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Path {
        Path {
            points: Vec::with_capacity(cap),
            verbs: Vec::with_capacity(cap),
        }
    }

    pub fn as_slice(&self) -> PathSlice {
        PathSlice {
            points: &self.points[..],
            verbs: &self.verbs[..],
        }
    }

    pub fn iter(&self) -> Iter { Iter::new(&self.points[..], &self.verbs[..]) }

    pub fn path_iter(&self) -> PathIter<Iter> { PathIter::new(self.iter()) }

    pub fn points(&self) -> &[Point] { &self.points[..] }

    pub fn mut_points(&mut self) -> &mut [Point] { &mut self.points[..] }

    pub fn verbs(&self) -> &[Verb] { &self.verbs[..] }

    /// Consumes two paths and builds one that contains them.
    pub fn merge(mut self, other: Self) -> Self {
        if other.verbs.is_empty() {
            return self;
        }

        if other.verbs[0] != Verb::MoveTo {
            self.verbs.push(Verb::MoveTo);
            self.points.push(point(0.0, 0.0));
        }

        self.verbs.extend(other.verbs);
        self.points.extend(other.points);

        self
    }
}

impl<'l> IntoIterator for &'l Path {
    type Item = PathEvent;
    type IntoIter = Iter<'l>;

    fn into_iter(self) -> Iter<'l> { self.iter() }
}

/// An immutable view over a Path.
impl<'l> PathSlice<'l> {
    pub fn new(points: &'l [Point], verbs: &'l [Verb]) -> PathSlice<'l> {
        PathSlice {
            points,
            verbs,
        }
    }

    pub fn iter(&self) -> Iter { Iter::new(self.points, self.verbs) }

    pub fn path_iter(&self) -> PathIter<Iter> { PathIter::new(self.iter()) }

    pub fn points(&self) -> &[Point] { self.points }

    pub fn verbs(&self) -> &[Verb] { self.verbs }
}

//impl<'l> IntoIterator for PathSlice<'l> {
//    type Item = PathEvent;
//    type IntoIter = Iter<'l>;
//
//    fn into_iter(self) -> Iter<'l> { self.iter() }
//}

/// Builds path object using the FlatPathBuilder interface.
///
/// See the [builder module](builder/index.html) documentation.
pub struct Builder {
    path: Path,
    current_position: Point,
    first_position: Point,
    building: bool,
}

impl Builder {
    pub fn new() -> Self { Builder::with_capacity(128) }

    pub fn with_capacity(cap: usize) -> Self {
        Builder {
            path: Path::with_capacity(cap),
            current_position: Point::new(0.0, 0.0),
            first_position: Point::new(0.0, 0.0),
            building: false,
        }
    }

    pub fn with_svg(self) -> SvgPathBuilder<Self> { SvgPathBuilder::new(self) }

    pub fn flattened(self, tolerance: f32) -> FlatteningBuilder<Self> {
        FlatteningBuilder::new(self, tolerance)
    }
}

#[inline]
fn nan_check(p: Point) {
    debug_assert!(!p.x.is_nan());
    debug_assert!(!p.y.is_nan());
}

impl FlatPathBuilder for Builder {
    type PathType = Path;

    fn move_to(&mut self, to: Point) {
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

    fn build(self) -> Path { self.path }

    fn build_and_reset(&mut self) -> Path {
        self.current_position = Point::new(0.0, 0.0);
        self.first_position = Point::new(0.0, 0.0);
        self.building = false;
        let mut tmp = Path::with_capacity(self.path.verbs.len());
        ::std::mem::swap(&mut self.path, &mut tmp);

        tmp
    }
}

impl PathBuilder for Builder {
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

    fn arc(
        &mut self,
        center: Point,
        radii: Vector,
        sweep_angle: Angle,
        x_rotation: Angle
    ) {
        nan_check(center);
        nan_check(radii.to_point());
        debug_assert!(!sweep_angle.get().is_nan());
        debug_assert!(!x_rotation.get().is_nan());
        self.path.points.push(center);
        self.path.points.push(radii.to_point());
        self.path.points.push(point(
            sweep_angle.get(),
            x_rotation.get(),
        ));
        self.path.verbs.push(Verb::Arc);
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'l> {
    points: ::std::slice::Iter<'l, Point>,
    verbs: ::std::slice::Iter<'l, Verb>,
}

impl<'l> Iter<'l> {
    pub fn new(points: &'l [Point], verbs: &'l [Verb]) -> Self {
        Iter {
            points: points.iter(),
            verbs: verbs.iter(),
        }
    }
}

impl<'l> Iterator for Iter<'l> {
    type Item = PathEvent;
    fn next(&mut self) -> Option<PathEvent> {
        match self.verbs.next() {
            Some(&Verb::MoveTo) => {
                let to = *self.points.next().unwrap();
                Some(PathEvent::MoveTo(to))
            }
            Some(&Verb::LineTo) => {
                let to = *self.points.next().unwrap();
                Some(PathEvent::LineTo(to))
            }
            Some(&Verb::QuadraticTo) => {
                let ctrl = *self.points.next().unwrap();
                let to = *self.points.next().unwrap();
                Some(PathEvent::QuadraticTo(ctrl, to))
            }
            Some(&Verb::CubicTo) => {
                let ctrl1 = *self.points.next().unwrap();
                let ctrl2 = *self.points.next().unwrap();
                let to = *self.points.next().unwrap();
                Some(PathEvent::CubicTo(ctrl1, ctrl2, to))
            }
            Some(&Verb::Arc) => {
                let center = *self.points.next().unwrap();
                let radii = self.points.next().unwrap().to_vector();
                let sweep_angle_x_rot = *self.points.next().unwrap();
                Some(PathEvent::Arc(
                    center,
                    radii,
                    Angle::radians(sweep_angle_x_rot.x),
                    Angle::radians(sweep_angle_x_rot.y),
                ))
            }
            Some(&Verb::Close) => Some(PathEvent::Close),
            None => None,
        }
    }
}

#[test]
fn test_path_builder_1() {

    let mut p = Builder::with_capacity(0);
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
    assert_eq!(it.next(), Some(PathEvent::LineTo(point(1.0, 0.0))));
    assert_eq!(it.next(), Some(PathEvent::LineTo(point(2.0, 0.0))));
    assert_eq!(it.next(), Some(PathEvent::LineTo(point(3.0, 0.0))));
    assert_eq!(it.next(), Some(PathEvent::QuadraticTo(point(4.0, 0.0), point(4.0, 1.0))));
    assert_eq!(
        it.next(),
        Some(PathEvent::CubicTo(point(5.0, 0.0), point(5.0, 1.0), point(5.0, 2.0)))
    );
    assert_eq!(it.next(), Some(PathEvent::Close));

    assert_eq!(it.next(), Some(PathEvent::MoveTo(point(10.0, 0.0))));
    assert_eq!(it.next(), Some(PathEvent::LineTo(point(11.0, 0.0))));
    assert_eq!(it.next(), Some(PathEvent::LineTo(point(12.0, 0.0))));
    assert_eq!(it.next(), Some(PathEvent::LineTo(point(13.0, 0.0))));
    assert_eq!(it.next(), Some(PathEvent::QuadraticTo(point(14.0, 0.0), point(14.0, 1.0))));
    assert_eq!(
        it.next(),
        Some(PathEvent::CubicTo(point(15.0, 0.0), point(15.0, 1.0), point(15.0, 2.0)))
    );
    assert_eq!(it.next(), Some(PathEvent::Close));

    assert_eq!(it.next(), Some(PathEvent::Close));
    assert_eq!(it.next(), Some(PathEvent::MoveTo(point(1.0, 1.0))));
    assert_eq!(it.next(), Some(PathEvent::MoveTo(point(2.0, 2.0))));
    assert_eq!(it.next(), Some(PathEvent::MoveTo(point(3.0, 3.0))));
    assert_eq!(it.next(), Some(PathEvent::LineTo(point(4.0, 4.0))));
    assert_eq!(it.next(), None);
    assert_eq!(it.next(), None);
    assert_eq!(it.next(), None);
}

#[test]
fn test_path_builder_empty() {
    let path = Path::builder().build();
    let mut it = path.iter();
    assert_eq!(it.next(), None);
    assert_eq!(it.next(), None);
}

#[test]
fn test_path_builder_empty_move_to() {
    let mut p = Path::builder();
    p.move_to(point(1.0, 2.0));
    p.move_to(point(3.0, 4.0));
    p.move_to(point(5.0, 6.0));

    let path = p.build();
    let mut it = path.iter();
    assert_eq!(it.next(), Some(PathEvent::MoveTo(point(1.0, 2.0))));
    assert_eq!(it.next(), Some(PathEvent::MoveTo(point(3.0, 4.0))));
    assert_eq!(it.next(), Some(PathEvent::MoveTo(point(5.0, 6.0))));
    assert_eq!(it.next(), None);
    assert_eq!(it.next(), None);
}

#[test]
fn test_path_builder_move_to_after_close() {
    let mut p = Path::builder();
    p.line_to(point(1.0, 0.0));
    p.close();
    p.line_to(point(2.0, 0.0));

    let path = p.build();
    let mut it = path.iter();
    assert_eq!(it.next(), Some(PathEvent::LineTo(point(1.0, 0.0))));
    assert_eq!(it.next(), Some(PathEvent::Close));
    assert_eq!(it.next(), Some(PathEvent::LineTo(point(2.0, 0.0))));
    assert_eq!(it.next(), None);
}

/// Builder for flattened paths
pub type FlattenedPathBuilder = SvgPathBuilder<FlatteningBuilder<Builder>>;
/// FlattenedPathBuilder constructor.
pub fn flattened_path_builder(tolerance: f32) -> FlattenedPathBuilder {
    SvgPathBuilder::new(FlatteningBuilder::new(Path::builder(), tolerance))
}

#[test]
fn test_merge_paths() {
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(5.0, 0.0));
    builder.line_to(point(5.0, 5.0));
    builder.close();

    let path1 = builder.build();

    let mut builder = Path::builder();
    builder.move_to(point(1.0, 1.0));
    builder.line_to(point(4.0, 0.0));
    builder.line_to(point(4.0, 4.0));
    builder.close();

    let path2 = builder.build();

    let path = path1.merge(path2);

    let mut it = path.iter();
    assert_eq!(it.next(), Some(PathEvent::MoveTo(point(0.0, 0.0))));
    assert_eq!(it.next(), Some(PathEvent::LineTo(point(5.0, 0.0))));
    assert_eq!(it.next(), Some(PathEvent::LineTo(point(5.0, 5.0))));
    assert_eq!(it.next(), Some(PathEvent::Close));
    assert_eq!(it.next(), Some(PathEvent::MoveTo(point(1.0, 1.0))));
    assert_eq!(it.next(), Some(PathEvent::LineTo(point(4.0, 0.0))));
    assert_eq!(it.next(), Some(PathEvent::LineTo(point(4.0, 4.0))));
    assert_eq!(it.next(), Some(PathEvent::Close));
    assert_eq!(it.next(), None);
}

#[test]
fn test_merge_missing_moveto() {
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(5.0, 0.0));
    builder.line_to(point(5.0, 5.0));

    let path1 = builder.build();

    let mut builder = Path::builder();
    builder.line_to(point(4.0, 0.0));
    builder.line_to(point(4.0, 4.0));

    let path2 = builder.build();

    let path = path1.merge(path2);

    let mut it = path.iter();
    assert_eq!(it.next(), Some(PathEvent::MoveTo(point(0.0, 0.0))));
    assert_eq!(it.next(), Some(PathEvent::LineTo(point(5.0, 0.0))));
    assert_eq!(it.next(), Some(PathEvent::LineTo(point(5.0, 5.0))));
    assert_eq!(it.next(), Some(PathEvent::MoveTo(point(0.0, 0.0))));
    assert_eq!(it.next(), Some(PathEvent::LineTo(point(4.0, 0.0))));
    assert_eq!(it.next(), Some(PathEvent::LineTo(point(4.0, 4.0))));
    assert_eq!(it.next(), None);
}

/*
#[test]
fn test_path_builder_simple() {

    // clockwise
    {
        let mut path = flattened_path_builder(0.05);
        path.move_to(vector(0.0, 0.0));
        path.line_to(vector(1.0, 0.0));
        path.line_to(vector(1.0, 1.0));
        let id = path.close();

        let path = path.build();
        let info = path.sub_path(id).info();
        assert_eq!(path.vertices().nth(0).position, vector(0.0, 0.0));
        assert_eq!(path.vertices().nth(1).position, vector(1.0, 0.0));
        assert_eq!(path.vertices().nth(2).position, vector(1.0, 1.0));
        assert_eq!(path.vertices().nth(0).point_type, PointType::Normal);
        assert_eq!(path.vertices().nth(1).point_type, PointType::Normal);
        assert_eq!(path.vertices().nth(2).point_type, PointType::Normal);
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(vector(0.0, 0.0), size(1.0, 1.0)));
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
        path.move_to(vector(0.0, 0.0));
        path.line_to(vector(1.0, 1.0));
        path.line_to(vector(1.0, 0.0));
        let id = path.close();

        let path = path.build();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(vector(0.0, 0.0), size(1.0, 1.0)));
    }

    // line_to back to the first vertex (should ignore the last vertex)
    {
        let mut path = flattened_path_builder(0.05);
        path.move_to(vector(0.0, 0.0));
        path.line_to(vector(1.0, 1.0));
        path.line_to(vector(1.0, 0.0));
        path.line_to(vector(0.0, 0.0));
        let id = path.close();

        let path = path.build();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(vector(0.0, 0.0), size(1.0, 1.0)));
    }
}

#[test]
fn test_path_builder_simple_bezier() {
    // clockwise
    {
        let mut path = bezier_path_builder();
        path.move_to(vector(0.0, 0.0));
        path.quadratic_bezier_to(vector(1.0, 0.0), vector(1.0, 1.0));
        let id = path.close();

        let path = path.build();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(vector(0.0, 0.0), size(1.0, 1.0)));
    }

    // counter-clockwise
    {
        let mut path = bezier_path_builder();
        path.move_to(vector(0.0, 0.0));
        path.quadratic_bezier_to(vector(1.0, 1.0), vector(1.0, 0.0));
        let id = path.close();

        let path = path.build();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(vector(0.0, 0.0), size(1.0, 1.0)));
    }

    // a slightly more elaborate path
    {
        let mut path = bezier_path_builder();
        path.move_to(vector(0.0, 0.0));
        path.line_to(vector(0.1, 0.0));
        path.line_to(vector(0.2, 0.1));
        path.line_to(vector(0.3, 0.1));
        path.line_to(vector(0.4, 0.0));
        path.line_to(vector(0.5, 0.0));
        path.quadratic_bezier_to(vector(0.5, 0.4), vector(0.3, 0.4));
        path.line_to(vector(0.1, 0.4));
        path.quadratic_bezier_to(vector(-0.2, 0.1), vector(-0.1, 0.0));
        let id = path.close();

        let path = path.build();
        let info = path.sub_path(id).info();
        assert_eq!(info.aabb, Rect::new(vector(-0.2, 0.0), size(0.7, 0.4)));
    }
}

#[test]
fn test_arc_simple() {
    use lyon_core::ArcFlags;
    use lyon_path_builder::SvgBuilder;

    let mut path = bezier_path_builder();

    // Two big elliptical arc
    path.move_to(vector(180.0, 180.0));
    path.arc_to(
        vector(160.0, 220.0), vector(20.0, 40.0) , 0.0,
        ArcFlags { large_arc: true, sweep: false }
    );
    path.move_to(vector(180.0, 180.0));
    path.arc_to(
        vector(160.0, 220.0), vector(20.0, 40.0) , 0.0,
        ArcFlags { large_arc: true, sweep: true }
    );

    // a small elliptical arc
    path.move_to(vector(260.0, 150.0));
    path.arc_to(
        vector(240.0, 190.0), vector(20.0, 40.0) , 0.0,
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
    builder.move_to(vector(0.0, 0.0));
    builder.move_to(vector(1.0, 0.0));
    builder.move_to(vector(2.0, 0.0));
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
