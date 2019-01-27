//! The default path data structure.

use builder::{Build, FlatPathBuilder, PathBuilder, SvgPathBuilder, FlatteningBuilder};
use iterator::PathIter;

use PathEvent;
use VertexId;
use math::*;
use geom::Arc;

use std::iter::IntoIterator;
use std::ops;
use std::mem;

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

/// A cursor refers to an event within a Path.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Cursor {
    vertex: VertexId,
    verb: u32,
}

impl Cursor {
    pub fn edge_index(&self) -> usize {
        self.verb as usize
    }
}

/// A simple path data structure.
///
/// It can be created using a [Builder](struct.Builder.html), and can be iterated over.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Path {
    points: Box<[Point]>,
    verbs: Box<[Verb]>,
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
            points: Box::new([]),
            verbs: Box::new([]),
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

    /// Concatenate two paths.
    pub fn merge(&self, other: &Self) -> Self {
        let mut verbs = Vec::with_capacity(self.verbs.len() + other.verbs.len());
        let mut points = Vec::with_capacity(self.points.len() + other.points.len());
        verbs.extend_from_slice(&self.verbs);
        verbs.extend_from_slice(&other.verbs);
        points.extend_from_slice(&self.points);
        points.extend_from_slice(&other.points);

        Path {
            verbs: verbs.into_boxed_slice(),
            points: points.into_boxed_slice(),
        }
    }

    pub fn position_before(&self, cursor: Cursor) -> Point {
        self.points[u32::min(1, cursor.vertex.0) as usize - 1]
    }

    pub fn event_at_cursor(&self, cursor: Cursor) -> PathEvent {
        event_at_cursor(&cursor, &self.points, &self.verbs)
    }

    pub fn next_cursor(&self, cursor: Cursor) -> Option<Cursor> {
        next_cursor(&cursor, &self.verbs)
    }

    pub fn previous_cursor(&self, cursor: Cursor) -> Option<Cursor> {
        prev_cursor(&cursor, &self.verbs)
    }

    pub fn cursor(&self) -> Cursor {
        Cursor {
            vertex: VertexId(0),
            verb: 0,
        }
    }

    /// Returns whether this cursor points inside the storage of this path.
    pub fn cursor_is_valid(&self, cursor: Cursor) -> bool {
        (cursor.verb as usize) < self.verbs.len()
            && cursor.vertex.to_usize() + n_stored_points(self.verbs[cursor.verb as usize]) as usize <= self.points.len()
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

    pub fn iter(&self) -> Iter {
        Iter::new(self.points, self.verbs)
    }

    pub fn iter_from(&self, cursor: Cursor) -> Iter {
        Iter::new(
            &self.points[cursor.vertex.offset() as usize..],
            &self.verbs[cursor.verb as usize..],
        )
    }

    pub fn iter_until(&self, cursor: Cursor) -> Iter {
        Iter::new(
            &self.points[..cursor.vertex.offset() as usize],
            &self.verbs[..cursor.verb as usize],
        )
    }

    pub fn iter_range(&self, cursor: ops::Range<Cursor>) -> Iter {
        Iter::new(
            &self.points[cursor.start.vertex.offset() as usize .. cursor.end.vertex.offset() as usize],
            &self.verbs[cursor.start.verb as usize .. cursor.end.verb as usize],
        )
    }

    pub fn path_iter(&self) -> PathIter<Iter> {
        PathIter::new(self.iter())
    }

    pub fn path_iter_from(&self, cursor: Cursor) -> PathIter<Iter> {
        PathIter::new(self.iter_from(cursor))
    }

    pub fn path_iter_until(&self, cursor: Cursor) -> PathIter<Iter> {
        PathIter::new(self.iter_until(cursor))
    }

    pub fn path_iter_range(&self, cursor: ops::Range<Cursor>) -> PathIter<Iter> {
        PathIter::new(self.iter_range(cursor))
    }

    pub fn points(&self) -> &[Point] { self.points }

    pub fn verbs(&self) -> &[Verb] { self.verbs }

    /// Returns starting position of the edge that the provided cursor refers to.
    pub fn position_before(&self, cursor: Cursor) -> Point {
        self.points[u32::min(cursor.vertex.0, 1) as usize - 1]
    }

    pub fn event_at_cursor(&self, cursor: Cursor) -> PathEvent {
        event_at_cursor(&cursor, self.points, self.verbs)
    }

    pub fn next_cursor(&self, cursor: Cursor) -> Option<Cursor> {
        next_cursor(&cursor, self.verbs)
    }

    pub fn previous_cursor(&self, cursor: Cursor) -> Option<Cursor> {
        prev_cursor(&cursor, self.verbs)
    }
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
    points: Vec<Point>,
    verbs: Vec<Verb>,
    current_position: Point,
    first_position: Point,
    building: bool,
}

impl Builder {
    pub fn new() -> Self { Builder::with_capacity(128) }

    pub fn with_capacity(cap: usize) -> Self {
        Builder {
            points: Vec::with_capacity(cap),
            verbs: Vec::with_capacity(cap),
            current_position: Point::new(0.0, 0.0),
            first_position: Point::new(0.0, 0.0),
            building: false,
        }
    }

    pub fn with_svg(self) -> SvgPathBuilder<Self> { SvgPathBuilder::new(self) }

    pub fn flattened(self, tolerance: f32) -> FlatteningBuilder<Self> {
        FlatteningBuilder::new(self, tolerance)
    }

    pub fn move_to(&mut self, to: Point) {
        nan_check(to);
        self.first_position = to;
        self.current_position = to;
        self.building = true;
        self.points.push(to);
        self.verbs.push(Verb::MoveTo);
    }

    pub fn line_to(&mut self, to: Point) {
        nan_check(to);
        self.points.push(to);
        self.verbs.push(Verb::LineTo);
        self.current_position = to;
    }

    pub fn close(&mut self) {
        // Relative path ops tend to accumulate small floating point imprecisions
        // which results in the last segment ending almost but not quite at the
        // start of the sub-path, causing a new edge to be inserted which often
        // intersects with the first or last edge. This can affect algorithms that
        // Don't handle self-intersecting paths.
        // Deal with this by snapping the last point if it is very close to the
        // start of the sub path.
        if let Some(p) = self.points.last_mut() {
            let d = (*p - self.first_position).abs();
            if d.x + d.y < 0.0001 {
                *p = self.first_position;
            }
        }

        self.verbs.push(Verb::Close);
        self.current_position = self.first_position;
        self.building = false;
    }

    pub fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
        nan_check(ctrl);
        nan_check(to);
        self.points.push(ctrl);
        self.points.push(to);
        self.verbs.push(Verb::QuadraticTo);
        self.current_position = to;
    }

    pub fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
        nan_check(ctrl1);
        nan_check(ctrl2);
        nan_check(to);
        self.points.push(ctrl1);
        self.points.push(ctrl2);
        self.points.push(to);
        self.verbs.push(Verb::CubicTo);
        self.current_position = to;
    }

    pub fn arc(
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
        self.points.push(center);
        self.points.push(radii.to_point());
        self.points.push(point(
            sweep_angle.get(),
            x_rotation.get(),
        ));
        let start_angle = (self.current_position - center).angle_from_x_axis() - x_rotation;
        let arc = Arc { start_angle, center, radii, sweep_angle, x_rotation };
        self.current_position = arc.sample(1.0);
        self.points.push(self.current_position);
        self.verbs.push(Verb::Arc);
    }

    pub fn current_position(&self) -> Point { self.current_position }

    /// Returns a cursor to the next path event.
    pub fn cursor(&self) -> Cursor {
        Cursor {
            vertex: VertexId::from_usize(self.points.len()),
            verb: self.verbs.len() as u32,
        }
    }

    pub fn build(self) -> Path {
        Path {
            points: self.points.into_boxed_slice(),
            verbs: self.verbs.into_boxed_slice(),
        }
    }
}

#[inline]
fn nan_check(p: Point) {
    debug_assert!(p.x.is_finite());
    debug_assert!(p.y.is_finite());
}

impl Build for Builder {
    type PathType = Path;

    fn build(self) -> Path { self.build() }

    fn build_and_reset(&mut self) -> Path {
        self.current_position = Point::new(0.0, 0.0);
        self.first_position = Point::new(0.0, 0.0);
        self.building = false;

        Path {
            points: mem::replace(&mut self.points, Vec::new()).into_boxed_slice(),
            verbs: mem::replace(&mut self.verbs, Vec::new()).into_boxed_slice(),
        }
    }
}

impl FlatPathBuilder for Builder {
    fn move_to(&mut self, to: Point) {
        self.move_to(to);
    }

    fn line_to(&mut self, to: Point) {
        self.line_to(to);
    }

    fn close(&mut self) {
        self.close();
    }

    fn current_position(&self) -> Point { self.current_position }
}

impl<'l> ops::Index<VertexId> for PathSlice<'l> {
    type Output = Point;
    fn index(&self, id: VertexId) -> &Point {
        &self.points[id.offset() as usize]
    }
}

impl ops::Index<VertexId> for Path {
    type Output = Point;
    fn index(&self, id: VertexId) -> &Point {
        &self.points[id.offset() as usize]
    }
}

impl ops::IndexMut<VertexId> for Path {
    fn index_mut(&mut self, id: VertexId) -> &mut Point {
        &mut self.points[id.offset() as usize]
    }
}

impl PathBuilder for Builder {
    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
        self.quadratic_bezier_to(ctrl, to);
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
        self.cubic_bezier_to(ctrl1, ctrl2, to);
    }

    fn arc(
        &mut self,
        center: Point,
        radii: Vector,
        sweep_angle: Angle,
        x_rotation: Angle
    ) {
        self.arc(center, radii, sweep_angle, x_rotation);
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

fn n_stored_points(verb: Verb) -> u32 {
    match verb {
        Verb::MoveTo => 1,
        Verb::LineTo => 1,
        Verb::QuadraticTo => 2,
        Verb::CubicTo => 2,
        Verb::Arc => 4,
        Verb::Close => 0,
    }
}

fn next_cursor(cursor: &Cursor, verbs: &[Verb]) -> Option<Cursor> {
    if cursor.verb as usize >= verbs.len() {
        return None;
    }
    let verb = verbs[cursor.verb as usize];
    Some(Cursor {
        vertex: cursor.vertex + n_stored_points(verb),
        verb: cursor.verb + 1,
    })
}

fn prev_cursor(cursor: &Cursor, verbs: &[Verb]) -> Option<Cursor> {
    if cursor.verb == 0 {
        return None;
    }
    let verb = verbs[cursor.verb as usize - 1];
    Some(Cursor {
        vertex: cursor.vertex - n_stored_points(verb),
        verb: cursor.verb - 1,
    })
}

fn event_at_cursor(cursor: &Cursor, points: &[Point], verbs: &[Verb]) -> PathEvent {
    let p = &points[cursor.vertex.to_usize()..];
    match verbs[cursor.verb as usize] {
        Verb::MoveTo => PathEvent::MoveTo(p[0]),
        Verb::LineTo => PathEvent::LineTo(p[0]),
        Verb::QuadraticTo => PathEvent::QuadraticTo(p[0], p[1]),
        Verb::CubicTo => PathEvent::CubicTo(p[0], p[1], p[2]),
        Verb::Arc => {
            let center = p[0];
            let radii = p[1].to_vector();
            let sweep_angle_x_rot = p[2];
            PathEvent::Arc(
                center,
                radii,
                Angle::radians(sweep_angle_x_rot.x),
                Angle::radians(sweep_angle_x_rot.y),
            )
        }
        Verb::Close => PathEvent::Close,
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
