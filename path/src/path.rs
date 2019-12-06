//! The default path data structure.

use crate::math::*;
use crate::builder::*;
use crate::{Event, PathEvent, IdEvent, EndpointId, CtrlPointId, EventId, AttributeStore, PositionStore};
use crate::geom::Arc;

use std::iter::IntoIterator;
use std::u32;

/// Enumeration corresponding to the [Event](https://docs.rs/lyon_core/*/lyon_core/events/enum.Event.html) enum
/// without the parameters.
///
/// This is used by the [Path](struct.Path.html) data structure to store path events a tad
/// more efficiently.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
enum Verb {
    LineTo,
    QuadraticTo,
    CubicTo,
    Begin,
    Close,
    End,
}

/// A simple path data structure.
///
/// It can be created using a [BuilderWithAttributes](struct.BuilderWithAttributes.html), and can be iterated over.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Path {
    points: Box<[Point]>,
    verbs: Box<[Verb]>,
    num_attributes: usize,
}

/// A view on a `Path`.
#[derive(Copy, Clone, Debug)]
pub struct PathSlice<'l> {
    points: &'l [Point],
    verbs: &'l [Verb],
    num_attributes: usize,
}

pub struct WithAttributes<'l>(PathSlice<'l>);

impl<'l> std::ops::Deref for WithAttributes<'l> {
    type Target = PathSlice<'l>;
    fn deref(&self) -> &PathSlice<'l> { &self.0 }
}

impl<'l> WithAttributes<'l> {
    pub fn iter(&self) -> IterWithAttributes<'l> {
        IterWithAttributes::new(
            self.num_attributes,
            &self.points,
            &self.verbs,
        )
    }
}

impl<'l> IntoIterator for WithAttributes<'l> {
    type Item = Event<(Point, &'l[f32]), Point>;
    type IntoIter = IterWithAttributes<'l>;

    fn into_iter(self) -> IterWithAttributes<'l> { self.iter() }
}

impl Path {
    /// Creates a [Builder](struct.Builder.html) to build a path.
    pub fn builder() -> Builder { Builder::new() }

    /// Creates a [BuilderWithAttributes](struct.BuilderWithAttributes.html) to build a path
    /// with custom attributes.
    pub fn builder_with_attributes(num_attributes: usize) -> BuilderWithAttributes {
        BuilderWithAttributes::new(num_attributes)
    }

    /// Creates an Empty `Path`.
    #[inline]
    pub fn new() -> Path {
        Path {
            points: Box::new([]),
            verbs: Box::new([]),
            num_attributes: 0,
        }
    }

    /// Returns a view on this `Path`.
    #[inline]
    pub fn as_slice(&self) -> PathSlice {
        PathSlice {
            points: &self.points[..],
            verbs: &self.verbs[..],
            num_attributes: self.num_attributes,
        }
    }

    #[inline]
    pub fn with_attributes(&self) -> WithAttributes {
        WithAttributes(self.as_slice())
    }

    #[inline]
    pub fn get_attributes(&self, endpoint: EndpointId) -> &[f32] {
        get_attributes(self.num_attributes, &self.points, endpoint)
    }

    /// Iterates over the entire `Path`.
    pub fn iter(&self) -> Iter { Iter::new(self.num_attributes, &self.points[..], &self.verbs[..]) }

    /// Iterates over the endpoint and control point ids of the `Path`.
    pub fn id_iter(&self) -> IdIter {
        IdIter::new(self.num_attributes, &self.verbs[..])
    }

    pub fn transformed(&self, transform: &Transform2D) -> Self {
        let mut result = self.clone();
        result.apply_transform(transform);

        result
    }

    fn apply_transform(&mut self, transform: &Transform2D) {
        let iter = IdIter::new(self.num_attributes, &self.verbs[..]);

        for evt in iter {
            match evt {
                IdEvent::Begin { at } => {
                    self.points[at.to_usize()] = transform.transform_point(self.points[at.to_usize()]);
                }
                IdEvent::Line { to, .. } => {
                    self.points[to.to_usize()] = transform.transform_point(self.points[to.to_usize()]);
                }
                IdEvent::Quadratic { ctrl, to, .. } => {
                    self.points[ctrl.to_usize()] = transform.transform_point(self.points[ctrl.to_usize()]);
                    self.points[to.to_usize()] = transform.transform_point(self.points[to.to_usize()]);
                }
                IdEvent::Cubic { ctrl1, ctrl2, to, .. } => {
                    self.points[ctrl1.to_usize()] = transform.transform_point(self.points[ctrl1.to_usize()]);
                    self.points[ctrl2.to_usize()] = transform.transform_point(self.points[ctrl2.to_usize()]);
                    self.points[to.to_usize()] = transform.transform_point(self.points[to.to_usize()]);
                }
                IdEvent::End { .. } => {}
            }
        }
    }

    /// Concatenate two paths.
    ///
    /// They must have the same number of custom attributes.
    pub fn merge(&self, other: &Self) -> Self {
        assert_eq!(self.num_attributes, other.num_attributes);

        let mut verbs = Vec::with_capacity(self.verbs.len() + other.verbs.len());
        let mut points = Vec::with_capacity(self.points.len() + other.points.len());
        verbs.extend_from_slice(&self.verbs);
        verbs.extend_from_slice(&other.verbs);
        points.extend_from_slice(&self.points);
        points.extend_from_slice(&other.points);

        Path {
            verbs: verbs.into_boxed_slice(),
            points: points.into_boxed_slice(),
            num_attributes: self.num_attributes,
        }
    }
}

impl std::ops::Index<EndpointId> for Path {
    type Output = Point;
    fn index(&self, id: EndpointId) -> &Point {
        &self.points[id.to_usize()]
    }
}

impl std::ops::Index<CtrlPointId> for Path {
    type Output = Point;
    fn index(&self, id: CtrlPointId) -> &Point {
        &self.points[id.to_usize()]
    }
}

impl<'l> IntoIterator for &'l Path {
    type Item = PathEvent;
    type IntoIter = Iter<'l>;

    fn into_iter(self) -> Iter<'l> { self.iter() }
}

impl<'l> Into<PathSlice<'l>> for &'l Path {
    fn into(self) -> PathSlice<'l> {
        self.as_slice()
    }
}


impl PositionStore for Path {
    fn endpoint_position(&self, id: EndpointId) -> Point {
        self.points[id.to_usize()]
    }

    fn ctrl_point_position(&self, id: CtrlPointId) -> Point {
        self.points[id.to_usize()]
    }
}

impl AttributeStore for Path {
    fn get_attributes(&self, id: EndpointId) -> &[f32] {
        get_attributes(self.num_attributes, &self.points, id)
    }

    fn num_attributes(&self) -> usize {
        self.num_attributes
    }
}

/// An immutable view over a Path.
impl<'l> PathSlice<'l> {

    pub fn iter<'a>(&'a self) -> Iter<'l> {
        Iter::new(self.num_attributes, self.points, self.verbs)
    }

    pub fn points(&self) -> &[Point] { self.points }

    pub fn is_empty(&self) -> bool {
        self.verbs.is_empty()
    }
}

impl<'l> IntoIterator for PathSlice<'l> {
    type Item = PathEvent;
    type IntoIter = Iter<'l>;

    fn into_iter(self) -> Iter<'l> { self.iter() }
}

impl<'l, 'a> IntoIterator for &'a PathSlice<'l> {
    type Item = PathEvent;
    type IntoIter = Iter<'l>;

    fn into_iter(self) -> Iter<'l> { self.iter() }
}

impl<'l> PositionStore for PathSlice<'l> {
    fn endpoint_position(&self, id: EndpointId) -> Point {
        self.points[id.to_usize()]
    }

    fn ctrl_point_position(&self, id: CtrlPointId) -> Point {
        self.points[id.to_usize()]
    }
}

impl<'l> AttributeStore for PathSlice<'l> {
    fn get_attributes(&self, id: EndpointId) -> &[f32] {
        get_attributes(self.num_attributes, self.points, id)
    }

    fn num_attributes(&self) -> usize {
        self.num_attributes
    }
}

/// Builds path object using the FlatPathBuilder interface.
///
/// See the [builder module](builder/index.html) documentation.
pub struct Builder {
    points: Vec<Point>,
    verbs: Vec<Verb>,
    current_position: Point,
    first_position: Point,
    first_vertex: EndpointId,
    first_verb: u32,
    need_moveto: bool,
    last_cmd: Verb,
}

impl Builder {
    pub fn new() -> Self { Builder::with_capacity(0, 0) }

    pub fn with_capacity(points: usize, edges: usize) -> Self {
        Builder {
            points: Vec::with_capacity(points),
            verbs: Vec::with_capacity(edges),
            current_position: Point::new(0.0, 0.0),
            first_position: Point::new(0.0, 0.0),
            first_vertex: EndpointId(0),
            first_verb: 0,
            need_moveto: true,
            last_cmd: Verb::End,
        }
    }

    pub fn with_svg(self) -> SvgPathBuilder<Self> { SvgPathBuilder::new(self) }

    pub fn flattened(self, tolerance: f32) -> FlatteningBuilder<Self> {
        FlatteningBuilder::new(self, tolerance)
    }

    pub fn move_to(&mut self, to: Point) {
        nan_check(to);
        self.end_if_needed();
        self.need_moveto = false;
        self.first_position = to;
        self.first_vertex = EndpointId(self.points.len() as u32);
        self.first_verb = self.verbs.len() as u32;
        self.current_position = to;
        self.points.push(to);
        self.verbs.push(Verb::Begin);
        self.last_cmd = Verb::Begin;
    }

    pub fn line_to(&mut self, to: Point) {
        nan_check(to);
        self.move_to_if_needed();
        self.points.push(to);
        self.verbs.push(Verb::LineTo);
        self.current_position = to;
        self.last_cmd = Verb::LineTo;
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
        self.need_moveto = true;
        self.last_cmd = Verb::Close;
    }

    pub fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
        nan_check(ctrl);
        nan_check(to);
        self.move_to_if_needed();
        self.points.push(ctrl);
        self.points.push(to);
        self.verbs.push(Verb::QuadraticTo);
        self.current_position = to;
        self.last_cmd = Verb::QuadraticTo;
    }

    pub fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
        nan_check(ctrl1);
        nan_check(ctrl2);
        nan_check(to);
        self.move_to_if_needed();
        self.points.push(ctrl1);
        self.points.push(ctrl2);
        self.points.push(to);
        self.verbs.push(Verb::CubicTo);
        self.current_position = to;
        self.last_cmd = Verb::CubicTo;
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

        let start_angle = (self.current_position - center).angle_from_x_axis() - x_rotation;
        let arc = Arc { start_angle, center, radii, sweep_angle, x_rotation };
        arc.for_each_quadratic_bezier(&mut|curve| {
            self.quadratic_bezier_to(curve.ctrl, curve.to);
        });
    }

    /// Add a closed polygon.
    pub fn polygon(&mut self, points: &[Point]) {
        self.points.reserve(points.len());
        self.verbs.reserve(points.len() + 1);
        build_polygon(self, points);
    }

    fn move_to_if_needed(&mut self) {
        if self.need_moveto {
            let first = self.first_position;
            self.move_to(first);
        }
    }

    fn end_if_needed(&mut self) {
        if (self.last_cmd as u8) <= (Verb::Begin as u8) {
            self.verbs.push(Verb::End);
        }
    }

    pub fn current_position(&self) -> Point { self.current_position }

    pub fn build(mut self) -> Path {
        self.end_if_needed();
        Path {
            points: self.points.into_boxed_slice(),
            verbs: self.verbs.into_boxed_slice(),
            num_attributes: 0,
        }
    }
}

impl Build for Builder {
    type PathType = Path;

    fn build(self) -> Path { self.build() }

    fn build_and_reset(&mut self) -> Path {
        self.current_position = Point::new(0.0, 0.0);
        self.first_position = Point::new(0.0, 0.0);

        Path {
            points: std::mem::replace(&mut self.points, Vec::new()).into_boxed_slice(),
            verbs: std::mem::replace(&mut self.verbs, Vec::new()).into_boxed_slice(),
            num_attributes: 0,
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

impl PolygonBuilder for Builder {
    fn polygon(&mut self, points: &[Point]) {
        self.polygon(points);
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

pub struct BuilderWithAttributes {
    points: Vec<Point>,
    verbs: Vec<Verb>,
    current_position: Point,
    first_position: Point,
    first_vertex: EndpointId,
    first_verb: u32,
    need_moveto: bool,
    last_cmd: Verb,
    num_attributes: usize,
}

impl BuilderWithAttributes {
    pub fn new(num_attributes: usize) -> Self { BuilderWithAttributes::with_capacity(num_attributes, 0, 0) }

    pub fn with_capacity(num_attributes: usize, points: usize, edges: usize) -> Self {
        BuilderWithAttributes {
            points: Vec::with_capacity(points),
            verbs: Vec::with_capacity(edges),
            current_position: Point::new(0.0, 0.0),
            first_position: Point::new(0.0, 0.0),
            first_vertex: EndpointId(0),
            first_verb: 0,
            need_moveto: true,
            last_cmd: Verb::End,
            num_attributes,
        }
    }

    fn push_attributes(&mut self, attributes: &[f32]) {
        assert_eq!(attributes.len(), self.num_attributes);
        for i in 0..(self.num_attributes/2) {
            let x = attributes[i*2];
            let y = attributes[i*2 + 1];
            self.points.push(point(x, y));
        }
        if self.num_attributes % 2 == 1 {
            let x = attributes[self.num_attributes - 1];
            self.points.push(point(x, 0.0));
        }
    }

    pub fn move_to(&mut self, to: Point, attributes: &[f32]) {
        nan_check(to);
        self.end_if_needed();
        self.need_moveto = false;
        self.first_position = to;
        self.first_vertex = EndpointId(self.points.len() as u32);
        self.first_verb = self.verbs.len() as u32;
        self.current_position = to;
        self.points.push(to);
        self.push_attributes(attributes);
        self.verbs.push(Verb::Begin);
        self.last_cmd = Verb::Begin;
    }

    pub fn line_to(&mut self, to: Point, attributes: &[f32]) {
        nan_check(to);
        self.move_to_if_needed();
        self.points.push(to);
        self.verbs.push(Verb::LineTo);
        self.push_attributes(attributes);
        self.current_position = to;
        self.last_cmd = Verb::LineTo;
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
        self.need_moveto = true;
        self.last_cmd = Verb::Close;
    }

    pub fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point, attributes: &[f32]) {
        nan_check(ctrl);
        nan_check(to);
        self.move_to_if_needed();
        self.points.push(ctrl);
        self.points.push(to);
        self.push_attributes(attributes);
        self.verbs.push(Verb::QuadraticTo);
        self.current_position = to;
        self.last_cmd = Verb::QuadraticTo;
    }

    pub fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point, attributes: &[f32]) {
        nan_check(ctrl1);
        nan_check(ctrl2);
        nan_check(to);
        self.move_to_if_needed();
        self.points.push(ctrl1);
        self.points.push(ctrl2);
        self.points.push(to);
        self.push_attributes(attributes);
        self.verbs.push(Verb::CubicTo);
        self.current_position = to;
        self.last_cmd = Verb::CubicTo;
    }

    fn move_to_if_needed(&mut self) {
        if !self.need_moveto {
            return;
        }

        assert!(!self.points.is_empty());

        let first = self.first_position;
        self.need_moveto = false;
        self.current_position = first;
        self.points.push(first);
        let first_idx = self.first_vertex.to_usize();
        for i in 0..(self.num_attributes + 1) / 2 {
            let val = self.points[first_idx + i];
            self.points.push(val);
        }
        self.verbs.push(Verb::Begin);
        self.last_cmd = Verb::Begin;
    }

    fn end_if_needed(&mut self) {
        if (self.last_cmd as u8) <= (Verb::Begin as u8) {
            self.verbs.push(Verb::End);
        }
    }

    pub fn current_position(&self) -> Point { self.current_position }

    pub fn build(mut self) -> Path {
        self.end_if_needed();
        Path {
            points: self.points.into_boxed_slice(),
            verbs: self.verbs.into_boxed_slice(),
            num_attributes: self.num_attributes,
        }
    }
}

#[inline]
fn nan_check(p: Point) {
    debug_assert!(p.x.is_finite());
    debug_assert!(p.y.is_finite());
}

/// An iterator for `Path` and `PathSlice`.
#[derive(Clone)]
pub struct Iter<'l> {
    points: PointIter<'l>,
    verbs: ::std::slice::Iter<'l, Verb>,
    current: Point,
    first: Point,
    num_attributes: usize,
    // Number of slots in the points array occupied by the custom attributes.
    attrib_stride: usize,
}

impl<'l> Iter<'l> {
    fn new(num_attributes: usize, points: &'l [Point], verbs: &'l [Verb]) -> Self {
        Iter {
            points: PointIter::new(points),
            verbs: verbs.iter(),
            current: point(0.0, 0.0),
            first: point(0.0, 0.0),
            num_attributes,
            attrib_stride: (num_attributes + 1) / 2,
        }
    }

    #[inline]
    fn skip_attributes(&mut self) {
        self.points.advance_n(self.attrib_stride);
    }
}

impl<'l> Iterator for Iter<'l> {
    type Item = PathEvent;
    #[inline]
    fn next(&mut self) -> Option<PathEvent> {
        match self.verbs.next() {
            Some(&Verb::Begin) => {
                self.current = self.points.next();
                self.skip_attributes();
                self.first = self.current;
                Some(PathEvent::Begin { at: self.current })
            }
            Some(&Verb::LineTo) => {
                let from = self.current;
                self.current = self.points.next();
                self.skip_attributes();
                Some(PathEvent::Line { from, to: self.current })
            }
            Some(&Verb::QuadraticTo) => {
                let from = self.current;
                let ctrl = self.points.next();
                self.current = self.points.next();
                self.skip_attributes();
                Some(PathEvent::Quadratic { from, ctrl, to: self.current })
            }
            Some(&Verb::CubicTo) => {
                let from = self.current;
                let ctrl1 = self.points.next();
                let ctrl2 = self.points.next();
                self.current = self.points.next();
                self.skip_attributes();
                Some(PathEvent::Cubic { from, ctrl1, ctrl2, to: self.current })
            }
            Some(&Verb::Close) => {
                let last = self.current;
                self.current = self.first;
                Some(PathEvent::End { last, first: self.first, close: true, })
            }
            Some(&Verb::End) => {
                let last = self.current;
                self.current = self.first;
                Some(PathEvent::End { last, first: self.first, close: false, })
            }
            None => None,
        }
    }
}

/// Manually implemented to avoid iterator overhead when skipping over
/// several points where the custom attributes are stored.
///
/// It makes an unfortunately large difference (the simple iterator
/// benchmarks are 2 to 3 times faster).
#[derive(Copy, Clone)]
struct PointIter<'l> {
    ptr: *const Point,
    end: *const Point,
    _marker: std::marker::PhantomData<&'l Point>,
}

impl<'l> PointIter<'l> {
    fn new(slice: &[Point]) -> Self {
        let ptr = slice.as_ptr();
        let end = unsafe { ptr.offset(slice.len() as isize) };
        PointIter {
            ptr,
            end,
            _marker: std::marker::PhantomData,
        }
    }

    #[inline]
    fn next(&mut self) -> Point {
        // Don't bother panicking here. calls to next
        // are always followed by advance_n which will
        // catch the issue and panic.
        if self.ptr >= self.end {
            return point(std::f32::NAN, std::f32::NAN);
        }

        unsafe {
            let output = *self.ptr;
            self.ptr = self.ptr.offset(1);

            return output;
        }
    }

    #[inline]
    fn advance_n(&mut self, n: usize) {
        unsafe {
            self.ptr = self.ptr.offset(n as isize);
            assert!(self.ptr <= self.end)
        }
    }
}

/// An iterator for `Path` and `PathSlice`.
#[derive(Clone)]
pub struct IterWithAttributes<'l> {
    points: PointIter<'l>,
    verbs: ::std::slice::Iter<'l, Verb>,
    current: (Point, &'l[f32]),
    first: (Point, &'l[f32]),
    num_attributes: usize,
    attrib_stride: usize,
}

impl<'l> IterWithAttributes<'l> {
    fn new(num_attributes: usize, points: &'l[Point], verbs: &'l[Verb]) -> Self {
        IterWithAttributes {
            points: PointIter::new(points),
            verbs: verbs.iter(),
            current: (point(0.0, 0.0), &[]),
            first: (point(0.0, 0.0), &[]),
            num_attributes,
            attrib_stride: (num_attributes + 1) / 2,
        }
    }

    pub fn points(self) -> Iter<'l> {
        Iter {
            points: self.points,
            verbs: self.verbs,
            current: self.current.0,
            first: self.first.0,
            num_attributes: self.num_attributes,
            attrib_stride: self.attrib_stride,
        }
    }

    #[inline]
    fn pop_endpoint(&mut self) -> (Point, &'l[f32]) {
        let position = self.points.next();
        let attributes = unsafe {
            let ptr = &(*self.points.ptr).x as *const f32;
            std::slice::from_raw_parts(ptr, self.num_attributes)
        };

        self.points.advance_n(self.attrib_stride);

        (position, attributes)
    }
}

impl<'l> Iterator for IterWithAttributes<'l> {
    type Item = Event<(Point, &'l[f32]), Point>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.verbs.next() {
            Some(&Verb::Begin) => {
                self.current = self.pop_endpoint();
                self.first = self.current;
                Some(Event::Begin { at: self.current, })
            }
            Some(&Verb::LineTo) => {
                let from = self.current;
                self.current = self.pop_endpoint();
                Some(Event::Line { from, to: self.current })
            }
            Some(&Verb::QuadraticTo) => {
                let from = self.current;
                let ctrl = self.points.next();
                self.current = self.pop_endpoint();
                Some(Event::Quadratic { from, ctrl, to: self.current })
            }
            Some(&Verb::CubicTo) => {
                let from = self.current;
                let ctrl1 = self.points.next();
                let ctrl2 = self.points.next();
                self.current = self.pop_endpoint();
                Some(Event::Cubic { from, ctrl1, ctrl2, to: self.current })
            }
            Some(&Verb::Close) => {
                let last = self.current;
                self.current = self.first;
                Some(Event::End { last, first: self.first, close: true, })
            }
            Some(&Verb::End) => {
                let last = self.current;
                self.current = self.first;
                Some(Event::End { last, first: self.first, close: false, })
            }
            None => None,
        }
    }
}

/// An iterator of endpoint and control point ids for `Path` and `PathSlice`.
#[derive(Clone, Debug)]
pub struct IdIter<'l> {
    verbs: ::std::slice::Iter<'l, Verb>,
    current: u32,
    first: u32,
    evt: u32,
    endpoint_stride: u32,
}

impl<'l> IdIter<'l> {
    fn new(num_attributes: usize, verbs: &'l [Verb]) -> Self {
        IdIter {
            verbs: verbs.iter(),
            current: 0,
            first: 0,
            evt: 0,
            endpoint_stride: (num_attributes as u32 + 1) / 2 + 1,
        }
    }
}

impl<'l> Iterator for IdIter<'l> {
    type Item = IdEvent;
    #[inline]
    fn next(&mut self) -> Option<IdEvent> {
        let edge = EventId(self.evt);
        match self.verbs.next() {
            Some(&Verb::Begin) => {
                let at = self.current;
                self.first = at;
                Some(IdEvent::Begin { at: EndpointId(at) })
            }
            Some(&Verb::LineTo) => {
                let from = EndpointId(self.current);
                self.current += self.endpoint_stride;
                let to = EndpointId(self.current);
                self.evt += 1;
                Some(IdEvent::Line { from, to, edge })
            }
            Some(&Verb::QuadraticTo) => {
                let from = EndpointId(self.current);
                let base = self.current + self.endpoint_stride;
                let ctrl = CtrlPointId(base);
                let to = EndpointId(base + 1);
                self.current = base + 1;
                self.evt += 1;
                Some(IdEvent::Quadratic { from, ctrl, to, edge })
            }
            Some(&Verb::CubicTo) => {
                let from = EndpointId(self.current);
                let base = self.current + self.endpoint_stride;
                let ctrl1 = CtrlPointId(base);
                let ctrl2 = CtrlPointId(base + 1);
                let to = EndpointId(base + 2);
                self.current = base + 2;
                self.evt += 1;
                Some(IdEvent::Cubic { from, ctrl1, ctrl2, to, edge })
            }
            Some(&Verb::Close) => {
                let last = EndpointId(self.current);
                let first = EndpointId(self.first);
                self.current += self.endpoint_stride;
                self.evt += 1;
                Some(IdEvent::End { last, first, close: true, edge })
            }
            Some(&Verb::End) => {
                let last = EndpointId(self.current);
                let first = EndpointId(self.first);
                self.current += self.endpoint_stride;
                self.evt += 1;
                Some(IdEvent::End { last, first, close: false, edge })
            }
            None => None,
        }
    }
}

#[inline]
fn get_attributes(num_attributes: usize, points: &[Point], endpoint: EndpointId) -> &[f32] {
    let idx = endpoint.0 as usize + 1;
    assert!(idx + (num_attributes + 1) / 2 < points.len());

    unsafe {
        let ptr = &points[idx].x as *const f32;
        std::slice::from_raw_parts(ptr, num_attributes)
    }
}

pub fn reverse_path(path: PathSlice, builder: &mut dyn PathBuilder) {
    let attrib_stride = (path.num_attributes() + 1) / 2;
    let points = &path.points[..];
    // At each iteration, p points to the first point after the current verb.
    let mut p = points.len();
    let mut need_close = false;

    for v in path.verbs.iter().rev().cloned() {
        match v {
            Verb::Close => {
                need_close = true;
                builder.move_to(points[p - 1]);
            }
            Verb::End => {
                need_close = false;
                builder.move_to(points[p - 1]);
            }
            Verb::Begin => {
                if need_close {
                    need_close = false;
                    builder.close();
                }
            }
            Verb::LineTo => {
                builder.line_to(points[p - 2]);
            }
            Verb::QuadraticTo => {
                builder.quadratic_bezier_to(points[p - 2], points[p - 3]);
            }
            Verb::CubicTo => {
                builder.cubic_bezier_to(points[p - 2], points[p - 3], points[p - 4]);
            }
        }
        p -= n_stored_points(v, attrib_stride);
    }
}

fn n_stored_points(verb: Verb, attrib_stride: usize) -> usize {
    match verb {
        Verb::Begin => attrib_stride + 1,
        Verb::LineTo => attrib_stride + 1,
        Verb::QuadraticTo => attrib_stride + 2,
        Verb::CubicTo => attrib_stride + 3,
        Verb::Close => 0,
        Verb::End => 0,
    }
}

#[test]
fn test_reverse_path() {
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(0.0, 1.0));

    builder.move_to(point(10.0, 0.0));
    builder.line_to(point(11.0, 0.0));
    builder.line_to(point(11.0, 1.0));
    builder.line_to(point(10.0, 1.0));
    builder.close();

    builder.move_to(point(20.0, 0.0));
    builder.quadratic_bezier_to(point(21.0, 0.0), point(21.0, 1.0));

    let p1 = builder.build();
    let mut builder = Path::builder();
    reverse_path(p1.as_slice(), &mut builder);
    let p2 = builder.build();

    let mut it = p2.iter();

    assert_eq!(it.next(), Some(PathEvent::Begin { at: point(21.0, 1.0) }));
    assert_eq!(it.next(), Some(PathEvent::Quadratic {
        from: point(21.0, 1.0),
        ctrl: point(21.0, 0.0),
        to: point(20.0, 0.0),
    }));
    assert_eq!(it.next(), Some(PathEvent::End { last: point(20.0, 0.0), first: point(21.0, 1.0), close: false }));

    assert_eq!(it.next(), Some(PathEvent::Begin { at: point(10.0, 1.0) }));
    assert_eq!(it.next(), Some(PathEvent::Line { from: point(10.0, 1.0), to: point(11.0, 1.0) }));
    assert_eq!(it.next(), Some(PathEvent::Line { from: point(11.0, 1.0), to: point(11.0, 0.0) }));
    assert_eq!(it.next(), Some(PathEvent::Line { from: point(11.0, 0.0), to: point(10.0, 0.0) }));
    assert_eq!(it.next(), Some(PathEvent::End { last: point(10.0, 0.0), first: point(10.0, 1.0), close: true }));

    assert_eq!(it.next(), Some(PathEvent::Begin { at: point(0.0, 1.0) }));
    assert_eq!(it.next(), Some(PathEvent::Line { from: point(0.0, 1.0), to: point(1.0, 1.0) }));
    assert_eq!(it.next(), Some(PathEvent::Line { from: point(1.0, 1.0), to: point(1.0, 0.0) }));
    assert_eq!(it.next(), Some(PathEvent::Line { from: point(1.0, 0.0), to: point(0.0, 0.0) }));
    assert_eq!(it.next(), Some(PathEvent::End { last: point(0.0, 0.0), first: point(0.0, 1.0), close: false }));

    assert_eq!(it.next(), None);
}

#[test]
fn test_reverse_path_no_close() {
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(1.0, 1.0));

    let p1 = builder.build();

    let mut builder = Path::builder();
    reverse_path(p1.as_slice(), &mut builder);
    let p2 = builder.build();

    let mut it = p2.iter();

    assert_eq!(it.next(), Some(PathEvent::Begin { at: point(1.0, 1.0) }));
    assert_eq!(it.next(), Some(PathEvent::Line { from: point(1.0, 1.0), to: point(1.0, 0.0) }));
    assert_eq!(it.next(), Some(PathEvent::Line { from: point(1.0, 0.0), to: point(0.0, 0.0) }));
    assert_eq!(it.next(), Some(PathEvent::End { last: point(0.0, 0.0), first: point(1.0, 1.0), close: false }));
    assert_eq!(it.next(), None);
}

#[test]
fn test_reverse_empty_path() {
    let p1 = Path::builder().build();
    let mut builder = Path::builder();
    reverse_path(p1.as_slice(), &mut builder);
    let p2 = builder.build();
    assert_eq!(p2.iter().next(), None);
}

#[test]
fn test_reverse_single_moveto() {
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    let p1 = builder.build();
    let mut builder = Path::builder();
    reverse_path(p1.as_slice(), &mut builder);
    let p2 = builder.build();
    let mut it = p2.iter();
    assert_eq!(it.next(), Some(PathEvent::Begin { at: point(0.0, 0.0) }));
    assert_eq!(it.next(), Some(PathEvent::End { last: point(0.0, 0.0), first: point(0.0, 0.0), close: false }));
    assert_eq!(it.next(), None);
}

#[test]
fn test_path_builder_1() {

    let mut p = BuilderWithAttributes::new(1);
    p.move_to(point(0.0, 0.0), &[0.0]);
    p.line_to(point(1.0, 0.0), &[1.0]);
    p.line_to(point(2.0, 0.0), &[2.0]);
    p.line_to(point(3.0, 0.0), &[3.0]);
    p.quadratic_bezier_to(point(4.0, 0.0), point(4.0, 1.0), &[4.0]);
    p.cubic_bezier_to(point(5.0, 0.0), point(5.0, 1.0), point(5.0, 2.0), &[5.0]);
    p.close();

    p.move_to(point(10.0, 0.0), &[6.0]);
    p.line_to(point(11.0, 0.0), &[7.0]);
    p.line_to(point(12.0, 0.0), &[8.0]);
    p.line_to(point(13.0, 0.0), &[9.0]);
    p.quadratic_bezier_to(point(14.0, 0.0), point(14.0, 1.0), &[10.0]);
    p.cubic_bezier_to(point(15.0, 0.0), point(15.0, 1.0), point(15.0, 2.0), &[11.0]);
    p.close();

    p.close();
    p.move_to(point(1.0, 1.0), &[12.0]);
    p.move_to(point(2.0, 2.0), &[13.0]);
    p.move_to(point(3.0, 3.0), &[14.0]);
    p.line_to(point(4.0, 4.0), &[15.0]);

    let path = p.build();

    let mut it = path.with_attributes().iter();
    assert_eq!(it.next(), Some(Event::Begin { at: (point(0.0, 0.0), &[0.0][..]) }));
    assert_eq!(it.next(), Some(Event::Line { from: (point(0.0, 0.0), &[0.0][..]), to: (point(1.0, 0.0), &[1.0][..]) }));
    assert_eq!(it.next(), Some(Event::Line { from: (point(1.0, 0.0), &[1.0][..]), to: (point(2.0, 0.0), &[2.0][..]) }));
    assert_eq!(it.next(), Some(Event::Line { from: (point(2.0, 0.0), &[2.0][..]), to: (point(3.0, 0.0), &[3.0][..]) }));
    assert_eq!(it.next(), Some(Event::Quadratic {
        from: (point(3.0, 0.0), &[3.0][..]),
        ctrl: point(4.0, 0.0),
        to: (point(4.0, 1.0), &[4.0][..])
    }));
    assert_eq!(
        it.next(),
        Some(Event::Cubic {
            from: (point(4.0, 1.0), &[4.0][..]),
            ctrl1: point(5.0, 0.0),
            ctrl2: point(5.0, 1.0),
            to: (point(5.0, 2.0), &[5.0][..]),
        })
    );
    assert_eq!(it.next(), Some(Event::End { last: (point(5.0, 2.0), &[5.0][..]), first: (point(0.0, 0.0), &[0.0][..]), close: true }));

    assert_eq!(it.next(), Some(Event::Begin { at: (point(10.0, 0.0), &[6.0][..]) }));
    assert_eq!(it.next(), Some(Event::Line { from: (point(10.0, 0.0), &[6.0][..]), to: (point(11.0, 0.0), &[7.0][..]) }));
    assert_eq!(it.next(), Some(Event::Line { from: (point(11.0, 0.0), &[7.0][..]), to: (point(12.0, 0.0), &[8.0][..]) }));
    assert_eq!(it.next(), Some(Event::Line { from: (point(12.0, 0.0), &[8.0][..]), to: (point(13.0, 0.0), &[9.0][..]) }));
    assert_eq!(it.next(), Some(Event::Quadratic {
        from: (point(13.0, 0.0), &[9.0][..]),
        ctrl: point(14.0, 0.0),
        to: (point(14.0, 1.0), &[10.0][..]),
    }));
    assert_eq!(
        it.next(),
        Some(Event::Cubic {
            from: (point(14.0, 1.0), &[10.0][..]),
            ctrl1: point(15.0, 0.0),
            ctrl2: point(15.0, 1.0),
            to: (point(15.0, 2.0), &[11.0][..]),
        })
    );
    assert_eq!(it.next(), Some(Event::End {
        last: (point(15.0, 2.0), &[11.0][..]),
        first: (point(10.0, 0.0), &[6.0][..]),
        close: true
    }));

    // Not clear that this is the most useful behavior.
    // Closing when there is no path should probably be dropped.
    assert_eq!(it.next(), Some(Event::End {
        last: (point(10.0, 0.0), &[6.0][..]),
        first: (point(10.0, 0.0), &[6.0][..]),
        close: true,
    }));

    assert_eq!(it.next(), Some(Event::Begin { at: (point(1.0, 1.0), &[12.0][..]) }));
    assert_eq!(it.next(), Some(Event::End { last: (point(1.0, 1.0), &[12.0][..]), first: (point(1.0, 1.0), &[12.0][..]), close: false }));
    assert_eq!(it.next(), Some(Event::Begin { at: (point(2.0, 2.0), &[13.0][..]) }));
    assert_eq!(it.next(), Some(Event::End { last: (point(2.0, 2.0), &[13.0][..]), first: (point(2.0, 2.0), &[13.0][..]), close: false }));
    assert_eq!(it.next(), Some(Event::Begin { at: (point(3.0, 3.0), &[14.0][..]) }));
    assert_eq!(it.next(), Some(Event::Line { from: (point(3.0, 3.0), &[14.0][..]), to: (point(4.0, 4.0), &[15.0][..]) }));
    assert_eq!(it.next(), Some(Event::End { last: (point(4.0, 4.0), &[15.0][..]), first: (point(3.0, 3.0), &[14.0][..]), close: false }));
    assert_eq!(it.next(), None);
    assert_eq!(it.next(), None);
    assert_eq!(it.next(), None);
}

#[test]
fn test_path_builder_empty() {
    let path = Path::builder_with_attributes(5).build();
    let mut it = path.iter();
    assert_eq!(it.next(), None);
    assert_eq!(it.next(), None);
}

#[test]
fn test_path_builder_empty_move_to() {
    let mut p = Path::builder_with_attributes(1);
    p.move_to(point(1.0, 2.0), &[0.0]);
    p.move_to(point(3.0, 4.0), &[1.0]);
    p.move_to(point(5.0, 6.0), &[2.0]);

    let path = p.build();
    let mut it = path.iter();
    assert_eq!(it.next(), Some(PathEvent::Begin { at: point(1.0, 2.0) }));
    assert_eq!(it.next(), Some(PathEvent::End { last: point(1.0, 2.0), first: point(1.0, 2.0), close: false, }));
    assert_eq!(it.next(), Some(PathEvent::Begin { at: point(3.0, 4.0) }));
    assert_eq!(it.next(), Some(PathEvent::End { last: point(3.0, 4.0), first: point(3.0, 4.0), close: false, }));
    assert_eq!(it.next(), Some(PathEvent::Begin { at: point(5.0, 6.0) }));
    assert_eq!(it.next(), Some(PathEvent::End { last: point(5.0, 6.0), first: point(5.0, 6.0), close: false, }));
    assert_eq!(it.next(), None);
    assert_eq!(it.next(), None);
}

#[test]
fn test_path_builder_line_to_after_close() {
    let mut p = Path::builder();
    p.line_to(point(1.0, 0.0));
    p.close();
    p.line_to(point(2.0, 0.0));

    let path = p.build();
    let mut it = path.iter();
    assert_eq!(it.next(), Some(PathEvent::Begin { at: point(0.0, 0.0) }));
    assert_eq!(it.next(), Some(PathEvent::Line { from: point(0.0, 0.0), to: point(1.0, 0.0) }));
    assert_eq!(it.next(), Some(PathEvent::End { last: point(1.0, 0.0), first: point(0.0, 0.0), close: true }));
    assert_eq!(it.next(), Some(PathEvent::Begin { at: point(0.0, 0.0) }));
    assert_eq!(it.next(), Some(PathEvent::Line { from: point(0.0, 0.0), to: point(2.0, 0.0) }));
    assert_eq!(it.next(), Some(PathEvent::End { last: point(2.0, 0.0), first: point(0.0, 0.0), close: false }));
    assert_eq!(it.next(), None);
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

    let path = path1.merge(&path2);

    let mut it = path.iter();
    assert_eq!(it.next(), Some(PathEvent::Begin { at: point(0.0, 0.0) }));
    assert_eq!(it.next(), Some(PathEvent::Line { from: point(0.0, 0.0), to: point(5.0, 0.0) }));
    assert_eq!(it.next(), Some(PathEvent::Line { from: point(5.0, 0.0), to: point(5.0, 5.0) }));
    assert_eq!(it.next(), Some(PathEvent::End { last: point(5.0, 5.0), first: point(0.0, 0.0), close: true }));
    assert_eq!(it.next(), Some(PathEvent::Begin { at: point(1.0, 1.0) }));
    assert_eq!(it.next(), Some(PathEvent::Line { from: point(1.0, 1.0), to: point(4.0, 0.0) }));
    assert_eq!(it.next(), Some(PathEvent::Line { from: point(4.0, 0.0), to: point(4.0, 4.0) }));
    assert_eq!(it.next(), Some(PathEvent::End { last: point(4.0, 4.0), first: point(1.0, 1.0), close: true }));
    assert_eq!(it.next(), None);
}
