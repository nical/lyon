//! The default path data structure.
//!

use crate::builder::*;
use crate::geom::traits::Transformation;
use crate::geom::{QuadraticBezierSegment, CubicBezierSegment};
use crate::math::*;
use crate::private::DebugValidator;
use crate::{AttributeStore, ControlPointId, EndpointId, Event, IdEvent, PathEvent, PositionStore};

use std::fmt;
use std::iter::{FromIterator, IntoIterator};
use std::u32;

/// Enumeration corresponding to the [Event](https://docs.rs/lyon_core/*/lyon_core/events/enum.Event.html) enum
/// without the parameters.
///
/// This is used by the [Path](struct.Path.html) data structure to store path events a tad
/// more efficiently.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(crate) enum Verb {
    LineTo,
    QuadraticTo,
    CubicTo,
    Begin,
    Close,
    End,
}

/// A simple path data structure.
///
/// # Custom attributes
///
/// Paths can store a fixed number of extra `f32` values per endpoint, called
/// "custom attributes" or "interpolated attributes" through the documentation.
/// These can be handy to represent arbitrary attributes such as variable colors,
/// line width, etc.
///
/// See also:
/// - [`BuilderWithAttributes`](struct.BuilderWithAttributes.html).
/// - [`Path::builder_with_attributes`](struct.Path.html#method.builder_with_attributes).
/// - [`Path::attributes`](struct.Path.html#method.attributes).
///
/// # Representation
///
/// Paths contain two buffers:
/// - a buffer of commands (Begin, Line, Quadratic, Cubic, Close or End),
/// - and a buffer of pairs of floats that can be endpoints control points or custom attributes.
///
/// The order of storage for points is determined by the sequence of commands.
/// Custom attributes (if any) always directly follow endpoints. If there is an odd number
/// of attributes, the last float of the each attribute sequence is set to zero and is not used.
///
/// ```ascii
///  __________________________
/// |       |      |         |
/// | Begin | Line |Quadratic| ...
/// |_______|______|_________|_
///  __________________________________________________________________________
/// |         |          |         |          |         |         |          |
/// |start x,y|attributes| to x, y |attributes|ctrl x,y | to x, y |attributes| ...
/// |_________|__________|_________|__________|_________|_________|__________|_
/// ```
///
#[derive(Clone, Default)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Path {
    points: Box<[Point]>,
    verbs: Box<[Verb]>,
    num_attributes: usize,
}

/// A view on a `Path`.
#[derive(Copy, Clone)]
pub struct PathSlice<'l> {
    pub(crate) points: &'l [Point],
    pub(crate) verbs: &'l [Verb],
    pub(crate) num_attributes: usize,
}

impl Path {
    /// Creates a [Builder](struct.Builder.html) to build a path.
    pub fn builder() -> Builder {
        Builder::new()
    }

    /// Creates a [BuilderWithAttributes](struct.BuilderWithAttributes.html) to build a path
    /// with custom attributes.
    pub fn builder_with_attributes(num_attributes: usize) -> BuilderWithAttributes {
        BuilderWithAttributes::new(num_attributes)
    }

    /// Creates an [WithSvg](../builder/struct.WithSvg.html) to build a path
    /// with a rich set of commands.
    pub fn svg_builder() -> WithSvg<Builder> {
        WithSvg::new(Self::builder())
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

    /// Returns a slice over an endpoint's custom attributes.
    #[inline]
    pub fn attributes(&self, endpoint: EndpointId) -> &[f32] {
        interpolated_attributes(self.num_attributes, &self.points, endpoint)
    }

    /// Iterates over the entire `Path`.
    pub fn iter(&self) -> Iter {
        Iter::new(self.num_attributes, &self.points[..], &self.verbs[..])
    }

    /// Iterates over the endpoint and control point ids of the `Path`.
    pub fn id_iter(&self) -> IdIter {
        IdIter::new(self.num_attributes, &self.verbs[..])
    }

    pub fn iter_with_attributes(&self) -> IterWithAttributes {
        IterWithAttributes::new(self.num_attributes(), &self.points[..], &self.verbs[..])
    }

    /// Applies a transform to all endpoints and control points of this path and
    /// Returns the result.
    pub fn transformed<T: Transformation<f32>>(mut self, transform: &T) -> Self {
        self.apply_transform(transform);

        self
    }

    /// Returns a reversed version of this path with edge loops specified in the opposite
    /// order.
    pub fn reversed(&self) -> Self {
        reverse_path(self.as_slice())
    }

    fn apply_transform<T: Transformation<f32>>(&mut self, transform: &T) {
        let iter = IdIter::new(self.num_attributes, &self.verbs[..]);

        for evt in iter {
            match evt {
                IdEvent::Begin { at } => {
                    self.points[at.to_usize()] =
                        transform.transform_point(self.points[at.to_usize()]);
                }
                IdEvent::Line { to, .. } => {
                    self.points[to.to_usize()] =
                        transform.transform_point(self.points[to.to_usize()]);
                }
                IdEvent::Quadratic { ctrl, to, .. } => {
                    self.points[ctrl.to_usize()] =
                        transform.transform_point(self.points[ctrl.to_usize()]);
                    self.points[to.to_usize()] =
                        transform.transform_point(self.points[to.to_usize()]);
                }
                IdEvent::Cubic {
                    ctrl1, ctrl2, to, ..
                } => {
                    self.points[ctrl1.to_usize()] =
                        transform.transform_point(self.points[ctrl1.to_usize()]);
                    self.points[ctrl2.to_usize()] =
                        transform.transform_point(self.points[ctrl2.to_usize()]);
                    self.points[to.to_usize()] =
                        transform.transform_point(self.points[to.to_usize()]);
                }
                IdEvent::End { .. } => {}
            }
        }
    }
}

impl FromIterator<PathEvent> for Path {
    fn from_iter<T: IntoIterator<Item = PathEvent>>(iter: T) -> Path {
        iter.into_iter().fold(Path::builder(), |mut builder, event| {
            builder.path_event(event);
            builder
        }).build()
    }
}

impl std::ops::Index<EndpointId> for Path {
    type Output = Point;
    fn index(&self, id: EndpointId) -> &Point {
        &self.points[id.to_usize()]
    }
}

impl std::ops::Index<ControlPointId> for Path {
    type Output = Point;
    fn index(&self, id: ControlPointId) -> &Point {
        &self.points[id.to_usize()]
    }
}

impl<'l> IntoIterator for &'l Path {
    type Item = PathEvent;
    type IntoIter = Iter<'l>;

    fn into_iter(self) -> Iter<'l> {
        self.iter()
    }
}

impl<'l> From<&'l Path> for PathSlice<'l> {
    fn from(path: &'l Path) -> Self {
        path.as_slice()
    }
}

impl PositionStore for Path {
    fn get_endpoint(&self, id: EndpointId) -> Point {
        self.points[id.to_usize()]
    }

    fn get_control_point(&self, id: ControlPointId) -> Point {
        self.points[id.to_usize()]
    }
}

impl AttributeStore for Path {
    fn get(&self, id: EndpointId) -> &[f32] {
        interpolated_attributes(self.num_attributes, &self.points, id)
    }

    fn num_attributes(&self) -> usize {
        self.num_attributes
    }
}

impl fmt::Debug for Path {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        self.as_slice().fmt(formatter)
    }
}

/// An immutable view over a Path.
impl<'l> PathSlice<'l> {
    /// Iterates over the path.
    pub fn iter<'a>(&'a self) -> Iter<'l> {
        Iter::new(self.num_attributes, self.points, self.verbs)
    }

    /// Iterates over the endpoint and control point ids of the `Path`.
    pub fn id_iter(&self) -> IdIter {
        IdIter::new(self.num_attributes, self.verbs)
    }

    pub fn is_empty(&self) -> bool {
        self.verbs.is_empty()
    }

    /// Returns a slice over an endpoint's custom attributes.
    #[inline]
    pub fn attributes(&self, endpoint: EndpointId) -> &[f32] {
        interpolated_attributes(self.num_attributes, &self.points, endpoint)
    }
}

impl<'l> fmt::Debug for PathSlice<'l> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        fn write_point(formatter: &mut fmt::Formatter, point: Point) -> fmt::Result {
            write!(formatter, " ")?;
            fmt::Debug::fmt(&point.x, formatter)?;
            write!(formatter, " ")?;
            fmt::Debug::fmt(&point.y, formatter)
        }

        write!(formatter, "\"")?;
        for evt in self {
            match evt {
                PathEvent::Begin { at } => {
                    write!(formatter, " M")?;
                    write_point(formatter, at)?;
                }
                PathEvent::End { close, .. } => {
                    if close {
                        write!(formatter, " Z")?;
                    }
                }
                PathEvent::Line { to, .. } => {
                    write!(formatter, " L")?;
                    write_point(formatter, to)?;
                }
                PathEvent::Quadratic { ctrl, to, .. } => {
                    write!(formatter, " Q")?;
                    write_point(formatter, ctrl)?;
                    write_point(formatter, to)?;
                }
                PathEvent::Cubic {
                    ctrl1, ctrl2, to, ..
                } => {
                    write!(formatter, " C")?;
                    write_point(formatter, ctrl1)?;
                    write_point(formatter, ctrl2)?;
                    write_point(formatter, to)?;
                }
            }
        }

        write!(formatter, "\"")
    }
}

impl<'l> std::ops::Index<EndpointId> for PathSlice<'l> {
    type Output = Point;
    fn index(&self, id: EndpointId) -> &Point {
        &self.points[id.to_usize()]
    }
}

impl<'l> std::ops::Index<ControlPointId> for PathSlice<'l> {
    type Output = Point;
    fn index(&self, id: ControlPointId) -> &Point {
        &self.points[id.to_usize()]
    }
}

impl<'l> IntoIterator for PathSlice<'l> {
    type Item = PathEvent;
    type IntoIter = Iter<'l>;

    fn into_iter(self) -> Iter<'l> {
        self.iter()
    }
}

impl<'l, 'a> IntoIterator for &'a PathSlice<'l> {
    type Item = PathEvent;
    type IntoIter = Iter<'l>;

    fn into_iter(self) -> Iter<'l> {
        self.iter()
    }
}

impl<'l> PositionStore for PathSlice<'l> {
    fn get_endpoint(&self, id: EndpointId) -> Point {
        self.points[id.to_usize()]
    }

    fn get_control_point(&self, id: ControlPointId) -> Point {
        self.points[id.to_usize()]
    }
}

impl<'l> AttributeStore for PathSlice<'l> {
    fn get(&self, id: EndpointId) -> &[f32] {
        interpolated_attributes(self.num_attributes, self.points, id)
    }

    fn num_attributes(&self) -> usize {
        self.num_attributes
    }
}

/// The default builder for `Path`.
#[derive(Clone)]
pub struct Builder {
    pub(crate) points: Vec<Point>,
    pub(crate) verbs: Vec<Verb>,
    validator: DebugValidator,
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    pub fn new() -> Self {
        Builder {
            points: Vec::new(),
            verbs: Vec::new(),
            validator: DebugValidator::new(),
        }
    }

    pub fn with_capacity(points: usize, edges: usize) -> Self {
        Builder {
            points: Vec::with_capacity(points),
            verbs: Vec::with_capacity(edges),
            validator: DebugValidator::new(),
        }
    }

    pub fn with_svg(self) -> WithSvg<Self> {
        assert!(self.verbs.is_empty());
        WithSvg::new(self)
    }

    #[inline]
    pub fn begin(&mut self, at: Point) -> EndpointId {
        self.validator.begin();
        nan_check(at);

        let id = EndpointId(self.points.len() as u32);
        self.points.push(at);
        self.verbs.push(Verb::Begin);

        id
    }

    #[inline]
    pub fn end(&mut self, close: bool) {
        self.validator.end();

        self.verbs.push(if close { Verb::Close } else { Verb::End });
    }

    #[inline]
    pub fn close(&mut self) {
        self.end(true);
    }

    #[inline]
    pub fn line_to(&mut self, to: Point) -> EndpointId {
        self.validator.edge();
        nan_check(to);

        let id = EndpointId(self.points.len() as u32);
        self.points.push(to);
        self.verbs.push(Verb::LineTo);

        id
    }

    #[inline]
    pub fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) -> EndpointId {
        self.validator.edge();
        nan_check(ctrl);
        nan_check(to);

        self.points.push(ctrl);
        let id = EndpointId(self.points.len() as u32);
        self.points.push(to);
        self.verbs.push(Verb::QuadraticTo);

        id
    }

    #[inline]
    pub fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) -> EndpointId {
        self.validator.edge();
        nan_check(ctrl1);
        nan_check(ctrl2);
        nan_check(to);

        self.points.push(ctrl1);
        self.points.push(ctrl2);
        let id = EndpointId(self.points.len() as u32);
        self.points.push(to);
        self.verbs.push(Verb::CubicTo);

        id
    }

    #[inline]
    pub fn build(self) -> Path {
        self.validator.build();
        Path {
            points: self.points.into_boxed_slice(),
            verbs: self.verbs.into_boxed_slice(),
            num_attributes: 0,
        }
    }

    pub fn path_event(&mut self, event: PathEvent) {
        match event {
            PathEvent::Begin { at } => {
                self.begin(at);
            }
            PathEvent::Line { to, .. } => {
                self.line_to(to);
            }
            PathEvent::Quadratic { ctrl, to, .. } => {
                self.quadratic_bezier_to(ctrl, to);
            }
            PathEvent::Cubic {
                ctrl1, ctrl2, to, ..
            } => {
                self.cubic_bezier_to(ctrl1, ctrl2, to);
            }
            PathEvent::End { close: true, .. } => {
                self.end(true);
            }
            PathEvent::End { close: false, .. } => {
                self.end(false);
            }
        }
    }

    pub fn reserve(&mut self, endpoints: usize, ctrl_points: usize) {
        self.points.reserve(endpoints + ctrl_points);
        self.verbs.reserve(endpoints);
    }

    #[inline]
    pub fn concatenate(&mut self, paths: &[PathSlice]) {
        concatenate_paths(&mut self.points, &mut self.verbs, paths, 0);
    }
}

impl PathBuilder for Builder {
    fn begin(&mut self, at: Point) -> EndpointId {
        self.begin(at)
    }

    fn end(&mut self, close: bool) {
        self.end(close);
    }

    fn line_to(&mut self, to: Point) -> EndpointId {
        self.line_to(to)
    }

    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) -> EndpointId {
        self.quadratic_bezier_to(ctrl, to)
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) -> EndpointId {
        self.cubic_bezier_to(ctrl1, ctrl2, to)
    }

    fn reserve(&mut self, endpoints: usize, ctrl_points: usize) {
        self.reserve(endpoints, ctrl_points);
    }
}

impl Build for Builder {
    type PathType = Path;

    fn build(self) -> Path {
        self.build()
    }
}

/// A builder for `Path` with custom attributes.
///
/// Custom attributes are a fixed number of `f32` values associated with each endpoint.
/// All endpoints must have the same number of custom attributes,
#[derive(Clone)]
pub struct BuilderWithAttributes {
    pub(crate) builder: Builder,
    pub(crate) num_attributes: usize,
}

impl BuilderWithAttributes {
    pub fn new(num_attributes: usize) -> Self {
        BuilderWithAttributes {
            builder: Builder::new(),
            num_attributes,
        }
    }

    pub fn reserve(&mut self, endpoints: usize, ctrl_points: usize) {
        let attr = self.num_attributes / 2 + self.num_attributes % 2;
        let n_points = endpoints * (1 + attr) + ctrl_points;
        self.builder.points.reserve(n_points);
        self.builder.verbs.reserve(endpoints);
    }

    #[inline]
    pub fn begin(&mut self, at: Point, attributes: &[f32]) -> EndpointId {
        let id = self.builder.begin(at);
        self.push_attributes(attributes);

        id
    }

    #[inline]
    pub fn end(&mut self, close: bool) {
        self.builder.end(close);
    }

    #[inline]
    pub fn close(&mut self) {
        self.builder.end(true);
    }

    #[inline]
    pub fn line_to(&mut self, to: Point, attributes: &[f32]) -> EndpointId {
        let id = self.builder.line_to(to);
        self.push_attributes(attributes);

        id
    }

    #[inline]
    pub fn quadratic_bezier_to(
        &mut self,
        ctrl: Point,
        to: Point,
        attributes: &[f32],
    ) -> EndpointId {
        let id = self.builder.quadratic_bezier_to(ctrl, to);
        self.push_attributes(attributes);

        id
    }

    #[inline]
    pub fn cubic_bezier_to(
        &mut self,
        ctrl1: Point,
        ctrl2: Point,
        to: Point,
        attributes: &[f32],
    ) -> EndpointId {
        let id = self.builder.cubic_bezier_to(ctrl1, ctrl2, to);
        self.push_attributes(attributes);

        id
    }

    #[inline]
    pub fn build(self) -> Path {
        self.builder.validator.build();
        Path {
            points: self.builder.points.into_boxed_slice(),
            verbs: self.builder.verbs.into_boxed_slice(),
            num_attributes: self.num_attributes,
        }
    }

    #[inline]
    pub fn concatenate(&mut self, paths: &[PathSlice]) {
        concatenate_paths(
            &mut self.builder.points,
            &mut self.builder.verbs,
            paths,
            self.num_attributes,
        );
    }

    fn push_attributes(&mut self, attributes: &[f32]) {
        assert_eq!(attributes.len(), self.num_attributes);
        for i in 0..(self.num_attributes / 2) {
            let x = attributes[i * 2];
            let y = attributes[i * 2 + 1];
            self.builder.points.push(point(x, y));
        }
        if self.num_attributes % 2 == 1 {
            let x = attributes[self.num_attributes - 1];
            self.builder.points.push(point(x, 0.0));
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
                Some(PathEvent::Line {
                    from,
                    to: self.current,
                })
            }
            Some(&Verb::QuadraticTo) => {
                let from = self.current;
                let ctrl = self.points.next();
                self.current = self.points.next();
                self.skip_attributes();
                Some(PathEvent::Quadratic {
                    from,
                    ctrl,
                    to: self.current,
                })
            }
            Some(&Verb::CubicTo) => {
                let from = self.current;
                let ctrl1 = self.points.next();
                let ctrl2 = self.points.next();
                self.current = self.points.next();
                self.skip_attributes();
                Some(PathEvent::Cubic {
                    from,
                    ctrl1,
                    ctrl2,
                    to: self.current,
                })
            }
            Some(&Verb::Close) => {
                let last = self.current;
                self.current = self.first;
                Some(PathEvent::End {
                    last,
                    first: self.first,
                    close: true,
                })
            }
            Some(&Verb::End) => {
                let last = self.current;
                self.current = self.first;
                Some(PathEvent::End {
                    last,
                    first: self.first,
                    close: false,
                })
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
    fn new(slice: &'l [Point]) -> Self {
        let ptr = slice.as_ptr();
        let end = unsafe { ptr.add(slice.len()) };
        PointIter {
            ptr,
            end,
            _marker: std::marker::PhantomData,
        }
    }

    #[inline]
    fn remaining_len(&self) -> usize {
        (self.end as usize - self.ptr as usize) / std::mem::size_of::<Point>()
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

            output
        }
    }

    #[inline]
    fn advance_n(&mut self, n: usize) {
        unsafe {
            assert!(self.remaining_len() >= n);
            self.ptr = self.ptr.add(n);
        }
    }
}

/// An iterator for `Path` and `PathSlice`.
#[derive(Clone)]
pub struct IterWithAttributes<'l> {
    points: PointIter<'l>,
    verbs: ::std::slice::Iter<'l, Verb>,
    current: (Point, &'l [f32]),
    first: (Point, &'l [f32]),
    num_attributes: usize,
    attrib_stride: usize,
}

impl<'l> IterWithAttributes<'l> {
    fn new(num_attributes: usize, points: &'l [Point], verbs: &'l [Verb]) -> Self {
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
            attrib_stride: self.attrib_stride,
        }
    }

    /// Iterate on a flattened approximation of the path with interpolated custom attributes
    /// using callbacks.
    ///
    /// At the time of writing, it is impossible to implement this efficiently
    /// with the `Iterator` trait, because of the need to express some lifetime
    /// constraints in an associated type, see #701.
    pub fn for_each_flattened<F>(self, tolerance: f32, callback: &mut F)
    where
        F: FnMut(&Event<(Point, &[f32]), Point>)
    {
        let num_attributes = self.num_attributes;
        // Some scratch space for writing the interpolated custom attributes.
        let mut stack_buffer = [0.0; 16];
        let mut vec_buffer;
        // No need to allocate memory if the number of custom attributes is small,
        // which is likely the common case.
        let buffer = if num_attributes <= 8 {
            &mut stack_buffer[..]
        } else {
            vec_buffer = vec![0.0; num_attributes * 2];
            &mut vec_buffer[..]
        };

        for evt in self {
            match evt {
                Event::Begin { at } => {
                    callback(&Event::Begin { at });
                }
                Event::End { last, first, close } => {
                    callback(&Event::End { last, first, close });
                }
                Event::Line { from, to } => {
                    callback(&Event::Line { from, to });
                }
                Event::Quadratic { from, ctrl, to } => {
                    let from_attr = from.1;
                    let to_attr = to.1;
                    let curve = QuadraticBezierSegment { from: from.0, ctrl, to: to.0 };
                    let mut prev_pos = from.0;
                    let mut offset = num_attributes;
                    buffer[0..num_attributes].copy_from_slice(from_attr);
                    curve.for_each_flattened_with_t(tolerance, &mut|pos, t| {
                        for i in 0..num_attributes {
                            buffer[offset + i] = (1.0 - t) * from_attr[i] + t * to_attr[i];
                        }

                        let next_offset =  if offset == 0 { num_attributes } else { 0 };

                        callback(&Event::Line {
                            from: (prev_pos, &buffer[next_offset..(next_offset + num_attributes)]),
                            to: (pos, &buffer[offset..(offset + num_attributes)]),
                        });

                        offset = next_offset;
                        prev_pos = pos;
                    });
                }
                Event::Cubic { from, ctrl1, ctrl2, to } => {
                    let from_attr = from.1;
                    let to_attr = to.1;
                    let curve = CubicBezierSegment { from: from.0, ctrl1, ctrl2, to: to.0 };
                    let mut prev_pos = from.0;
                    let mut offset = num_attributes;
                    buffer[0..num_attributes].copy_from_slice(from_attr);
                    curve.for_each_flattened_with_t(tolerance, &mut|pos, t| {
                        for i in 0..num_attributes {
                            buffer[offset + i] = (1.0 - t) * from_attr[i] + t * to_attr[i];
                        }

                        let next_offset =  if offset == 0 { num_attributes } else { 0 };

                        callback(&Event::Line {
                            from: (prev_pos, &buffer[next_offset..(next_offset + num_attributes)]),
                            to: (pos, &buffer[offset..(offset + num_attributes)]),
                        });

                        offset = next_offset;
                        prev_pos = pos;
                    });
                }
            }
        }
    }

    #[inline]
    fn pop_endpoint(&mut self) -> (Point, &'l [f32]) {
        let position = self.points.next();
        let attributes_ptr = self.points.ptr as *const f32;
        self.points.advance_n(self.attrib_stride);
        let attributes = unsafe {
            // SAFETY: advance_n would have panicked if the slice is out of bounds
            std::slice::from_raw_parts(attributes_ptr, self.num_attributes)
        };

        (position, attributes)
    }
}

impl<'l> Iterator for IterWithAttributes<'l> {
    type Item = Event<(Point, &'l [f32]), Point>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.verbs.next() {
            Some(&Verb::Begin) => {
                self.current = self.pop_endpoint();
                self.first = self.current;
                Some(Event::Begin { at: self.current })
            }
            Some(&Verb::LineTo) => {
                let from = self.current;
                self.current = self.pop_endpoint();
                Some(Event::Line {
                    from,
                    to: self.current,
                })
            }
            Some(&Verb::QuadraticTo) => {
                let from = self.current;
                let ctrl = self.points.next();
                self.current = self.pop_endpoint();
                Some(Event::Quadratic {
                    from,
                    ctrl,
                    to: self.current,
                })
            }
            Some(&Verb::CubicTo) => {
                let from = self.current;
                let ctrl1 = self.points.next();
                let ctrl2 = self.points.next();
                self.current = self.pop_endpoint();
                Some(Event::Cubic {
                    from,
                    ctrl1,
                    ctrl2,
                    to: self.current,
                })
            }
            Some(&Verb::Close) => {
                let last = self.current;
                self.current = self.first;
                Some(Event::End {
                    last,
                    first: self.first,
                    close: true,
                })
            }
            Some(&Verb::End) => {
                let last = self.current;
                self.current = self.first;
                Some(Event::End {
                    last,
                    first: self.first,
                    close: false,
                })
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
                Some(IdEvent::Line { from, to })
            }
            Some(&Verb::QuadraticTo) => {
                let from = EndpointId(self.current);
                let base = self.current + self.endpoint_stride;
                let ctrl = ControlPointId(base);
                let to = EndpointId(base + 1);
                self.current = base + 1;
                self.evt += 1;
                Some(IdEvent::Quadratic { from, ctrl, to })
            }
            Some(&Verb::CubicTo) => {
                let from = EndpointId(self.current);
                let base = self.current + self.endpoint_stride;
                let ctrl1 = ControlPointId(base);
                let ctrl2 = ControlPointId(base + 1);
                let to = EndpointId(base + 2);
                self.current = base + 2;
                self.evt += 1;
                Some(IdEvent::Cubic {
                    from,
                    ctrl1,
                    ctrl2,
                    to,
                })
            }
            Some(&Verb::Close) => {
                let last = EndpointId(self.current);
                let first = EndpointId(self.first);
                self.current += self.endpoint_stride;
                self.evt += 1;
                Some(IdEvent::End {
                    last,
                    first,
                    close: true,
                })
            }
            Some(&Verb::End) => {
                let last = EndpointId(self.current);
                let first = EndpointId(self.first);
                self.current += self.endpoint_stride;
                self.evt += 1;
                Some(IdEvent::End {
                    last,
                    first,
                    close: false,
                })
            }
            None => None,
        }
    }
}

#[inline]
fn interpolated_attributes(
    num_attributes: usize,
    points: &[Point],
    endpoint: EndpointId,
) -> &[f32] {
    if num_attributes == 0 {
        return &[];
    }

    let idx = endpoint.0 as usize + 1;
    assert!(idx + (num_attributes + 1) / 2 <= points.len());

    unsafe {
        let ptr = &points[idx].x as *const f32;
        std::slice::from_raw_parts(ptr, num_attributes)
    }
}

fn concatenate_paths(
    points: &mut Vec<Point>,
    verbs: &mut Vec<Verb>,
    paths: &[PathSlice],
    num_attributes: usize,
) {
    let mut np = 0;
    let mut nv = 0;

    for path in paths {
        assert_eq!(path.num_attributes(), num_attributes);
        np += path.points.len();
        nv += path.verbs.len();
    }

    verbs.reserve(nv);
    points.reserve(np);

    for path in paths {
        verbs.extend_from_slice(&path.verbs);
        points.extend_from_slice(&path.points);
    }
}

fn reverse_path(path: PathSlice) -> Path {
    let mut builder = Path::builder_with_attributes(path.num_attributes());

    let attrib_stride = (path.num_attributes() + 1) / 2;
    let points = path.points;
    // At each iteration, p points to the first point after the current verb.
    let mut p = points.len();
    let mut need_close = false;

    for v in path.verbs.iter().rev().cloned() {
        match v {
            Verb::Close => {
                let idx = p - 1 - attrib_stride;
                need_close = true;
                builder.begin(points[idx], path.attributes(EndpointId(idx as u32)));
            }
            Verb::End => {
                let idx = p - 1 - attrib_stride;
                need_close = false;
                builder.begin(points[idx], path.attributes(EndpointId(idx as u32)));
            }
            Verb::Begin => {
                builder.end(need_close);
                need_close = false;
            }
            Verb::LineTo => {
                let idx = p - 2 - attrib_stride * 2;
                builder.line_to(points[idx], path.attributes(EndpointId(idx as u32)));
            }
            Verb::QuadraticTo => {
                let ctrl_idx = p - attrib_stride - 2;
                let to_idx = ctrl_idx - attrib_stride - 1;
                builder.quadratic_bezier_to(
                    points[ctrl_idx],
                    points[to_idx],
                    path.attributes(EndpointId(to_idx as u32)),
                );
            }
            Verb::CubicTo => {
                let ctrl1_idx = p - attrib_stride - 2;
                let ctrl2_idx = ctrl1_idx - 1;
                let to_idx = ctrl2_idx - attrib_stride - 1;
                builder.cubic_bezier_to(
                    points[ctrl1_idx],
                    points[ctrl2_idx],
                    points[to_idx],
                    path.attributes(EndpointId(to_idx as u32)),
                );
            }
        }
        p -= n_stored_points(v, attrib_stride);
    }

    builder.build()
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
fn test_reverse_path_simple() {
    let mut builder = Path::builder_with_attributes(1);
    builder.begin(point(0.0, 0.0), &[1.0]);
    builder.line_to(point(1.0, 0.0), &[2.0]);
    builder.line_to(point(1.0, 1.0), &[3.0]);
    builder.line_to(point(0.0, 1.0), &[4.0]);
    builder.end(false);

    let p1 = builder.build();
    let p2 = p1.reversed();

    let mut it = p2.iter_with_attributes();

    // Using a function that explicits the argument types works around type inference issue.
    fn check<'l>(
        a: Option<Event<(Point, &'l [f32]), Point>>,
        b: Option<Event<(Point, &'l [f32]), Point>>,
    ) -> bool {
        if a != b {
            println!("left: {:?}", a);
            println!("right: {:?}", b);
        }

        a == b
    }

    assert!(check(
        it.next(),
        Some(Event::Begin {
            at: (point(0.0, 1.0), &[4.0])
        })
    ));
    assert!(check(
        it.next(),
        Some(Event::Line {
            from: (point(0.0, 1.0), &[4.0]),
            to: (point(1.0, 1.0), &[3.0])
        })
    ));
    assert!(check(
        it.next(),
        Some(Event::Line {
            from: (point(1.0, 1.0), &[3.0]),
            to: (point(1.0, 0.0), &[2.0])
        })
    ));
    assert!(check(
        it.next(),
        Some(Event::Line {
            from: (point(1.0, 0.0), &[2.0]),
            to: (point(0.0, 0.0), &[1.0])
        })
    ));
    assert!(check(
        it.next(),
        Some(Event::End {
            last: (point(0.0, 0.0), &[1.0]),
            first: (point(0.0, 1.0), &[4.0]),
            close: false
        })
    ));

    assert!(check(it.next(), None));
}

#[test]
fn test_reverse_path() {
    let mut builder = Path::builder_with_attributes(1);
    builder.begin(point(0.0, 0.0), &[1.0]);
    builder.line_to(point(1.0, 0.0), &[2.0]);
    builder.line_to(point(1.0, 1.0), &[3.0]);
    builder.line_to(point(0.0, 1.0), &[4.0]);
    builder.end(false);

    builder.begin(point(10.0, 0.0), &[5.0]);
    builder.line_to(point(11.0, 0.0), &[6.0]);
    builder.line_to(point(11.0, 1.0), &[7.0]);
    builder.line_to(point(10.0, 1.0), &[8.0]);
    builder.end(true);

    builder.begin(point(20.0, 0.0), &[9.0]);
    builder.quadratic_bezier_to(point(21.0, 0.0), point(21.0, 1.0), &[10.0]);
    builder.end(false);

    let p1 = builder.build();
    let p2 = p1.reversed();

    let mut it = p2.iter_with_attributes();

    // Using a function that explicits the argument types works around type inference issue.
    fn check<'l>(
        a: Option<Event<(Point, &'l [f32]), Point>>,
        b: Option<Event<(Point, &'l [f32]), Point>>,
    ) -> bool {
        if a != b {
            println!("left: {:?}", a);
            println!("right: {:?}", b);
        }

        a == b
    }

    assert!(check(
        it.next(),
        Some(Event::Begin {
            at: (point(21.0, 1.0), &[10.0])
        })
    ));
    assert!(check(
        it.next(),
        Some(Event::Quadratic {
            from: (point(21.0, 1.0), &[10.0]),
            ctrl: point(21.0, 0.0),
            to: (point(20.0, 0.0), &[9.0]),
        })
    ));
    assert!(check(
        it.next(),
        Some(Event::End {
            last: (point(20.0, 0.0), &[9.0]),
            first: (point(21.0, 1.0), &[10.0]),
            close: false
        })
    ));

    assert!(check(
        it.next(),
        Some(Event::Begin {
            at: (point(10.0, 1.0), &[8.0])
        })
    ));
    assert!(check(
        it.next(),
        Some(Event::Line {
            from: (point(10.0, 1.0), &[8.0]),
            to: (point(11.0, 1.0), &[7.0])
        })
    ));
    assert!(check(
        it.next(),
        Some(Event::Line {
            from: (point(11.0, 1.0), &[7.0]),
            to: (point(11.0, 0.0), &[6.0])
        })
    ));
    assert!(check(
        it.next(),
        Some(Event::Line {
            from: (point(11.0, 0.0), &[6.0]),
            to: (point(10.0, 0.0), &[5.0])
        })
    ));
    assert!(check(
        it.next(),
        Some(Event::End {
            last: (point(10.0, 0.0), &[5.0]),
            first: (point(10.0, 1.0), &[8.0]),
            close: true
        })
    ));

    assert!(check(
        it.next(),
        Some(Event::Begin {
            at: (point(0.0, 1.0), &[4.0])
        })
    ));
    assert!(check(
        it.next(),
        Some(Event::Line {
            from: (point(0.0, 1.0), &[4.0]),
            to: (point(1.0, 1.0), &[3.0])
        })
    ));
    assert!(check(
        it.next(),
        Some(Event::Line {
            from: (point(1.0, 1.0), &[3.0]),
            to: (point(1.0, 0.0), &[2.0])
        })
    ));
    assert!(check(
        it.next(),
        Some(Event::Line {
            from: (point(1.0, 0.0), &[2.0]),
            to: (point(0.0, 0.0), &[1.0])
        })
    ));
    assert!(check(
        it.next(),
        Some(Event::End {
            last: (point(0.0, 0.0), &[1.0]),
            first: (point(0.0, 1.0), &[4.0]),
            close: false
        })
    ));

    assert!(check(it.next(), None));
}

#[test]
fn test_reverse_path_no_close() {
    let mut builder = Path::builder();
    builder.begin(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(1.0, 1.0));
    builder.end(false);

    let p1 = builder.build();

    let p2 = p1.reversed();

    let mut it = p2.iter();

    assert_eq!(
        it.next(),
        Some(PathEvent::Begin {
            at: point(1.0, 1.0)
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::Line {
            from: point(1.0, 1.0),
            to: point(1.0, 0.0)
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::Line {
            from: point(1.0, 0.0),
            to: point(0.0, 0.0)
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::End {
            last: point(0.0, 0.0),
            first: point(1.0, 1.0),
            close: false
        })
    );
    assert_eq!(it.next(), None);
}

#[test]
fn test_reverse_empty_path() {
    let p1 = Path::builder().build();
    let p2 = p1.reversed();
    assert_eq!(p2.iter().next(), None);
}

#[test]
fn test_reverse_single_point() {
    let mut builder = Path::builder();
    builder.begin(point(0.0, 0.0));
    builder.end(false);

    let p1 = builder.build();
    let p2 = p1.reversed();
    let mut it = p2.iter();
    assert_eq!(
        it.next(),
        Some(PathEvent::Begin {
            at: point(0.0, 0.0)
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::End {
            last: point(0.0, 0.0),
            first: point(0.0, 0.0),
            close: false
        })
    );
    assert_eq!(it.next(), None);
}

#[test]
fn test_path_builder_1() {
    let mut p = BuilderWithAttributes::new(1);
    p.begin(point(0.0, 0.0), &[0.0]);
    p.line_to(point(1.0, 0.0), &[1.0]);
    p.line_to(point(2.0, 0.0), &[2.0]);
    p.line_to(point(3.0, 0.0), &[3.0]);
    p.quadratic_bezier_to(point(4.0, 0.0), point(4.0, 1.0), &[4.0]);
    p.cubic_bezier_to(point(5.0, 0.0), point(5.0, 1.0), point(5.0, 2.0), &[5.0]);
    p.end(true);

    p.begin(point(10.0, 0.0), &[6.0]);
    p.line_to(point(11.0, 0.0), &[7.0]);
    p.line_to(point(12.0, 0.0), &[8.0]);
    p.line_to(point(13.0, 0.0), &[9.0]);
    p.quadratic_bezier_to(point(14.0, 0.0), point(14.0, 1.0), &[10.0]);
    p.cubic_bezier_to(
        point(15.0, 0.0),
        point(15.0, 1.0),
        point(15.0, 2.0),
        &[11.0],
    );
    p.end(true);

    p.begin(point(1.0, 1.0), &[12.0]);
    p.end(false);
    p.begin(point(2.0, 2.0), &[13.0]);
    p.end(false);
    p.begin(point(3.0, 3.0), &[14.0]);
    p.line_to(point(4.0, 4.0), &[15.0]);
    p.end(false);

    let path = p.build();

    let mut it = path.iter_with_attributes();
    assert_eq!(
        it.next(),
        Some(Event::Begin {
            at: (point(0.0, 0.0), &[0.0][..])
        })
    );
    assert_eq!(
        it.next(),
        Some(Event::Line {
            from: (point(0.0, 0.0), &[0.0][..]),
            to: (point(1.0, 0.0), &[1.0][..])
        })
    );
    assert_eq!(
        it.next(),
        Some(Event::Line {
            from: (point(1.0, 0.0), &[1.0][..]),
            to: (point(2.0, 0.0), &[2.0][..])
        })
    );
    assert_eq!(
        it.next(),
        Some(Event::Line {
            from: (point(2.0, 0.0), &[2.0][..]),
            to: (point(3.0, 0.0), &[3.0][..])
        })
    );
    assert_eq!(
        it.next(),
        Some(Event::Quadratic {
            from: (point(3.0, 0.0), &[3.0][..]),
            ctrl: point(4.0, 0.0),
            to: (point(4.0, 1.0), &[4.0][..])
        })
    );
    assert_eq!(
        it.next(),
        Some(Event::Cubic {
            from: (point(4.0, 1.0), &[4.0][..]),
            ctrl1: point(5.0, 0.0),
            ctrl2: point(5.0, 1.0),
            to: (point(5.0, 2.0), &[5.0][..]),
        })
    );
    assert_eq!(
        it.next(),
        Some(Event::End {
            last: (point(5.0, 2.0), &[5.0][..]),
            first: (point(0.0, 0.0), &[0.0][..]),
            close: true
        })
    );

    assert_eq!(
        it.next(),
        Some(Event::Begin {
            at: (point(10.0, 0.0), &[6.0][..])
        })
    );
    assert_eq!(
        it.next(),
        Some(Event::Line {
            from: (point(10.0, 0.0), &[6.0][..]),
            to: (point(11.0, 0.0), &[7.0][..])
        })
    );
    assert_eq!(
        it.next(),
        Some(Event::Line {
            from: (point(11.0, 0.0), &[7.0][..]),
            to: (point(12.0, 0.0), &[8.0][..])
        })
    );
    assert_eq!(
        it.next(),
        Some(Event::Line {
            from: (point(12.0, 0.0), &[8.0][..]),
            to: (point(13.0, 0.0), &[9.0][..])
        })
    );
    assert_eq!(
        it.next(),
        Some(Event::Quadratic {
            from: (point(13.0, 0.0), &[9.0][..]),
            ctrl: point(14.0, 0.0),
            to: (point(14.0, 1.0), &[10.0][..]),
        })
    );
    assert_eq!(
        it.next(),
        Some(Event::Cubic {
            from: (point(14.0, 1.0), &[10.0][..]),
            ctrl1: point(15.0, 0.0),
            ctrl2: point(15.0, 1.0),
            to: (point(15.0, 2.0), &[11.0][..]),
        })
    );
    assert_eq!(
        it.next(),
        Some(Event::End {
            last: (point(15.0, 2.0), &[11.0][..]),
            first: (point(10.0, 0.0), &[6.0][..]),
            close: true
        })
    );

    assert_eq!(
        it.next(),
        Some(Event::Begin {
            at: (point(1.0, 1.0), &[12.0][..])
        })
    );
    assert_eq!(
        it.next(),
        Some(Event::End {
            last: (point(1.0, 1.0), &[12.0][..]),
            first: (point(1.0, 1.0), &[12.0][..]),
            close: false
        })
    );
    assert_eq!(
        it.next(),
        Some(Event::Begin {
            at: (point(2.0, 2.0), &[13.0][..])
        })
    );
    assert_eq!(
        it.next(),
        Some(Event::End {
            last: (point(2.0, 2.0), &[13.0][..]),
            first: (point(2.0, 2.0), &[13.0][..]),
            close: false
        })
    );
    assert_eq!(
        it.next(),
        Some(Event::Begin {
            at: (point(3.0, 3.0), &[14.0][..])
        })
    );
    assert_eq!(
        it.next(),
        Some(Event::Line {
            from: (point(3.0, 3.0), &[14.0][..]),
            to: (point(4.0, 4.0), &[15.0][..])
        })
    );
    assert_eq!(
        it.next(),
        Some(Event::End {
            last: (point(4.0, 4.0), &[15.0][..]),
            first: (point(3.0, 3.0), &[14.0][..]),
            close: false
        })
    );
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
fn test_path_builder_empty_begin() {
    let mut p = Path::builder_with_attributes(1);
    p.begin(point(1.0, 2.0), &[0.0]);
    p.end(false);
    p.begin(point(3.0, 4.0), &[1.0]);
    p.end(false);
    p.begin(point(5.0, 6.0), &[2.0]);
    p.end(false);

    let path = p.build();
    let mut it = path.iter();
    assert_eq!(
        it.next(),
        Some(PathEvent::Begin {
            at: point(1.0, 2.0)
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::End {
            last: point(1.0, 2.0),
            first: point(1.0, 2.0),
            close: false,
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::Begin {
            at: point(3.0, 4.0)
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::End {
            last: point(3.0, 4.0),
            first: point(3.0, 4.0),
            close: false,
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::Begin {
            at: point(5.0, 6.0)
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::End {
            last: point(5.0, 6.0),
            first: point(5.0, 6.0),
            close: false,
        })
    );
    assert_eq!(it.next(), None);
    assert_eq!(it.next(), None);
}

#[test]
fn test_concatenate() {
    let mut builder = Path::builder();
    builder.begin(point(0.0, 0.0));
    builder.line_to(point(5.0, 0.0));
    builder.line_to(point(5.0, 5.0));
    builder.end(true);

    let path1 = builder.build();

    let mut builder = Path::builder();
    builder.begin(point(1.0, 1.0));
    builder.line_to(point(4.0, 0.0));
    builder.line_to(point(4.0, 4.0));
    builder.end(true);

    let path2 = builder.build();

    let mut builder = Path::builder();
    builder.concatenate(&[path1.as_slice(), path2.as_slice()]);
    let path = builder.build();

    let mut it = path.iter();
    assert_eq!(
        it.next(),
        Some(PathEvent::Begin {
            at: point(0.0, 0.0)
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::Line {
            from: point(0.0, 0.0),
            to: point(5.0, 0.0)
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::Line {
            from: point(5.0, 0.0),
            to: point(5.0, 5.0)
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::End {
            last: point(5.0, 5.0),
            first: point(0.0, 0.0),
            close: true
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::Begin {
            at: point(1.0, 1.0)
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::Line {
            from: point(1.0, 1.0),
            to: point(4.0, 0.0)
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::Line {
            from: point(4.0, 0.0),
            to: point(4.0, 4.0)
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::End {
            last: point(4.0, 4.0),
            first: point(1.0, 1.0),
            close: true
        })
    );
    assert_eq!(it.next(), None);
}

#[test]
fn flattened_custom_attributes() {
    let mut path = Path::builder_with_attributes(1);
    path.begin(point(0.0, 0.0), &[0.0]);
    path.quadratic_bezier_to(point(1.0, 0.0), point(1.0, 1.0), &[1.0]);
    path.cubic_bezier_to(point(1.0, 2.0), point(0.0, 2.0), point(0.0, 1.0), &[2.0]);
    path.end(false);

    let path = path.build();

    let mut prev = -1.0;
    path.iter_with_attributes().for_each_flattened(0.01, &mut|evt| {
        let attribute = match evt {
            Event::Begin { at: (_, attr) } => attr[0],
            Event::Line { from: (_, from_attr), to: (_, to_attr) } => {
                assert_eq!(from_attr[0], prev);
                to_attr[0]
            }
            Event::End { last: (_, last_attr), .. } => {
                assert_eq!(last_attr[0], prev);
                return;
            }
            Event::Quadratic { .. }
            | Event::Cubic { .. }
            => {
                panic!("Should not get a curve in for_each_flattened");
            }
        };

        assert!(attribute > prev);
        prev = attribute;
    });
}

