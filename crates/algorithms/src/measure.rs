//! Perform cached measurements and split operations on a path.
//!
//! # Path measuring
//!
//! ## Example
//!
//! ```
//! use lyon_algorithms::math::point;
//! use lyon_algorithms::path::Path;
//! use lyon_algorithms::{
//!     length::approximate_length,
//!     measure::{MeasureResult, PathMeasure},
//! };
//!
//! fn main() {
//!     let mut path = Path::builder();
//!     path.begin(point(0.0, 0.0));
//!     path.quadratic_bezier_to(point(1.0, 1.0), point(2.0, 0.0));
//!     path.end(false);
//!     let path = path.build();
//!     let mut measure = PathMeasure::from_path(&path, 1e-3);
//!
//!     let MeasureResult {
//!         position, tangent, ..
//!     } = measure.measure(0.5);
//!     println!("Mid-point position: {:?}, tangent: {:?}", position, tangent);
//!
//!     let last_half = measure.split(0.5..1.0);
//!     assert!((measure.length() / 2.0 - approximate_length(&last_half, 1e-3)).abs() < 1e-3);
//! }
//! ```
//!
use crate::geom::{CubicBezierSegment, LineSegment, QuadraticBezierSegment, Segment};
use crate::math::*;
use crate::path::{
    path::BuilderWithAttributes, AttributeStore, Attributes, EndpointId, IdEvent, Path, PathSlice,
    PositionStore,
};
use std::ops::Range;

#[doc(hidden)]
pub struct EmptyAttribtueStore;
impl AttributeStore for EmptyAttribtueStore {
    fn get(&self, _: EndpointId) -> Attributes {
        Attributes::NONE
    }

    fn num_attributes(&self) -> usize {
        0
    }
}

type EndpointPair = (EndpointId, EndpointId);

enum SegmentWrapper {
    Empty,
    Line(LineSegment<f32>, EndpointPair),
    Quadratic(QuadraticBezierSegment<f32>, EndpointPair),
    Cubic(CubicBezierSegment<f32>, EndpointPair),
}

impl SegmentWrapper {
    fn split(&self, range: Range<f32>) -> Self {
        match self {
            Self::Empty => Self::Empty,
            Self::Line(segment, pair) => Self::Line(segment.split_range(range), *pair),
            Self::Quadratic(segment, pair) => Self::Quadratic(segment.split_range(range), *pair),
            Self::Cubic(segment, pair) => Self::Cubic(segment.split_range(range), *pair),
        }
    }
}

#[derive(Debug)]
pub struct MeasureResult<'l> {
    pub position: Point,
    pub tangent: Vector,
    pub attributes_store: Attributes<'l>,
}

struct Edge {
    // distance from the beginning of the path
    distance: f32,
    // which segment this edge is on
    index: usize,
    // t-value of the endpoint on the segment
    t: f32,
}

/// A struct storing necessary information to perform cached measurements on a path.
///
/// Note that PathMeasure borrows the path instead of storing it.
pub struct PathMeasure<'l, PS: PositionStore, AS: AttributeStore> {
    events: Box<[IdEvent]>,
    attribute_buffer: Box<[f32]>,
    position_store: &'l PS,
    attributes_store: &'l AS,

    edges: Box<[Edge]>,

    // This points to the edge where last query takes place (default to 0).
    // With the help of this we can achieve better performance when queries are continuous.
    ptr: usize,
}

impl<'l, PS: PositionStore, AS: AttributeStore> PathMeasure<'l, PS, AS> {
    fn to_segment(&self, event: IdEvent) -> SegmentWrapper {
        macro_rules! endpoints {
            ($($t:ident),*) => (
                $(let $t = self.position_store.get_endpoint($t);)*
            )
        }
        macro_rules! ctrl_points {
            ($($t:ident),*) => (
                $(let $t = self.position_store.get_control_point($t);)*
            )
        }
        match event {
            IdEvent::Line { from, to } => {
                let pair = (from, to);
                endpoints!(from, to);
                SegmentWrapper::Line(LineSegment { from, to }, pair)
            }
            IdEvent::Quadratic { from, ctrl, to } => {
                let pair = (from, to);
                endpoints!(from, to);
                ctrl_points!(ctrl);
                SegmentWrapper::Quadratic(QuadraticBezierSegment { from, ctrl, to }, pair)
            }
            IdEvent::Cubic {
                from,
                ctrl1,
                ctrl2,
                to,
            } => {
                let pair = (from, to);
                endpoints!(from, to);
                ctrl_points!(ctrl1, ctrl2);
                SegmentWrapper::Cubic(
                    CubicBezierSegment {
                        from,
                        ctrl1,
                        ctrl2,
                        to,
                    },
                    pair,
                )
            }
            IdEvent::End {
                last,
                first,
                close: true,
            } => {
                let pair = (last, first);
                endpoints!(last, first);
                SegmentWrapper::Line(
                    LineSegment {
                        from: last,
                        to: first,
                    },
                    pair,
                )
            }
            _ => SegmentWrapper::Empty,
        }
    }

    pub fn with_attributes<Iter>(
        num_attributes: usize,
        path: Iter,
        tolerance: f32,
        position_store: &'l PS,
        attributes_store: &'l AS,
    ) -> Self
    where
        Iter: IntoIterator<Item = IdEvent>,
    {
        macro_rules! endpoints {
            ($($t:ident),*) => (
                $(let $t = position_store.get_endpoint($t);)*
            )
        }
        macro_rules! ctrl_points {
            ($($t:ident),*) => (
                $(let $t = position_store.get_control_point($t);)*
            )
        }
        let tolerance = tolerance.max(1e-4);
        let events = path.into_iter().collect::<Vec<_>>().into_boxed_slice();
        let mut edges = Vec::new();
        let mut distance = 0.0;
        for (index, event) in events.iter().cloned().enumerate() {
            match event {
                IdEvent::Begin { .. } => {
                    edges.push(Edge {
                        distance,
                        index,
                        t: 1.0,
                    });
                }
                IdEvent::Line { from, to } => {
                    endpoints!(from, to);
                    distance += (from - to).length();
                    edges.push(Edge {
                        distance,
                        index,
                        t: 1.0,
                    })
                }
                IdEvent::Quadratic { from, ctrl, to } => {
                    endpoints!(from, to);
                    ctrl_points!(ctrl);
                    let segment = QuadraticBezierSegment { from, ctrl, to };
                    segment.for_each_flattened_with_t(tolerance, &mut |line, t| {
                        distance += line.length();
                        edges.push(Edge {
                            distance,
                            index,
                            t: t.end,
                        });
                    });
                }
                IdEvent::Cubic { from, ctrl1, ctrl2, to } => {
                    endpoints!(from, to);
                    ctrl_points!(ctrl1, ctrl2);
                    let segment = CubicBezierSegment { from, ctrl1, ctrl2, to };
                    segment.for_each_flattened_with_t(tolerance, &mut |line, t| {
                        distance += line.length();
                        edges.push(Edge {
                            distance,
                            index,
                            t: t.end,
                        });
                    });
                }
                IdEvent::End { last, first, close: true } => {
                    endpoints!(last, first);
                    distance += (last - first).length();
                    edges.push(Edge {
                        distance,
                        index,
                        t: 1.0,
                    })
                }
                _ => {}
            }
        }
        if !edges.is_empty() {
            debug_assert_eq!(edges.first().unwrap().distance, 0.0);
        }
        PathMeasure {
            events,
            attribute_buffer: vec![0.0; num_attributes].into_boxed_slice(),
            position_store,
            attributes_store,
            edges: edges.into_boxed_slice(),
            ptr: 0,
        }
    }

    pub fn num_attributes(&self) -> usize {
        self.attribute_buffer.len()
    }

    /// Returns the (approximate) length of the path.
    pub fn length(&self) -> f32 {
        if self.edges.is_empty() {
            0.0
        } else {
            self.edges.last().unwrap().distance
        }
    }

    fn require_not_empty(&self) {
        if self.length() == 0.0 {
            panic!("Attempt to measure an empty path");
        }
    }

    fn in_bounds(&self, dist: f32) -> bool {
        self.ptr != 0 && self.edges[self.ptr - 1].distance <= dist && dist <= self.edges[self.ptr].distance
    }

    /// Move the pointer so the given point is on the current segment.
    fn move_ptr(&mut self, dist: f32) {
        self.require_not_empty();
        if dist < 0.0 || dist > self.length() {
            panic!("Illegal distance: {}", dist);
        }
        if dist == 0.0 {
            self.ptr = 1;
            return;
        }
        if self.in_bounds(dist) {
            // No need to move
            return;
        }

        // Performs on [first, last)
        // ...TTFFF...
        //      ^
        //      find this point
        fn partition_point(first: usize, last: usize, pred: impl Fn(usize) -> bool) -> usize {
            let mut l = first;
            let mut r = last;
            while l < r {
                let mid = (l + r) / 2;
                if pred(mid) {
                    l = mid + 1;
                } else {
                    r = mid;
                }
            }
            debug_assert_eq!(l, r);
            debug_assert_ne!(l, last);
            l
        }

        fn floor_log2(num: usize) -> u32 {
            std::mem::size_of::<usize>() as u32 * 8 - num.leading_zeros() - 1
        }

        // Here we use a heuristic method combining method 1 & 2
        // Method 1:        Move step by step until we found the corresponding segment, works well on short paths and near queries
        // Time complexity: (expected) (dist - start).abs() / len * num
        // Method 2.        Binary search on lengths, works well on long paths and random calls
        // Time complexity: (exact) floor(log2(num))
        // where `len` and `num` are given as follow
        //
        // According to the benchmark, this method works well in both cases and has low overhead in relative to Method 1 & 2.
        // Benchmark code: https://gist.github.com/Mivik/5f67ae5a72eae3884b2f386370554966
        let start = self.edges[self.ptr].distance;
        if start < dist {
            let (len, num) = (self.length() - start, self.edges.len() - self.ptr - 1);
            debug_assert_ne!(num, 0);
            if (dist - start) / len * (num as f32) < floor_log2(num) as f32 {
                loop {
                    self.ptr += 1;
                    if dist <= self.edges[self.ptr].distance {
                        break;
                    }
                }
            } else {
                self.ptr = partition_point(self.ptr + 1, self.edges.len(), |p| {
                    self.edges[p].distance < dist
                });
            }
        } else {
            let (len, num) = (start, self.ptr + 1);
            debug_assert_ne!(num, 0);
            if (start - dist) / len * (num as f32) < floor_log2(num) as f32 {
                loop {
                    self.ptr -= 1;
                    if self.ptr == 0 || self.edges[self.ptr - 1].distance < dist {
                        break;
                    }
                }
            } else {
                self.ptr = partition_point(0, self.ptr, |p| self.edges[p].distance < dist);
            }
        }
        debug_assert!(self.in_bounds(dist));
    }

    /// Linear interpolation between attributes_store.
    fn lerp<'a>(
        from: Attributes,
        to: Attributes,
        t: f32,
        buffer: &'a mut Box<[f32]>,
    ) -> Attributes<'a> {
        for i in 0..buffer.len() {
            buffer[i] = from[i] * (1.0 - t) + to[i] * t;
        }
        Attributes(&buffer[..])
    }

    /// Returns the relative position (0 ~ 1) of the given point on the current segment.
    fn t(&self, dist: f32) -> f32 {
        debug_assert!(self.in_bounds(dist));
        let prev = &self.edges[self.ptr - 1];
        let cur = &self.edges[self.ptr];
        let t_begin = if prev.index == cur.index { prev.t } else { 0.0 };
        let t_end = cur.t;
        t_begin + (t_end - t_begin) * ((dist - prev.distance) / (cur.distance - prev.distance))
    }

    /// Measures a given point by its relative distance on the path. See [`PathMeasure::measure_by_distance`].
    ///
    /// # Panics
    ///
    /// Panics if t is not in [0.0, 1.0] or the path is empty.
    pub fn measure(&mut self, t: f32) -> MeasureResult {
        self.measure_by_distance(t * self.length())
    }

    /// Measures a given point by its distance from the beginning on the path. See [`PathMeasure::measure`].
    ///
    /// # Panics
    ///
    /// Panics if t is not in [0.0, `self.length()`] or the path is empty.
    pub fn measure_by_distance(&mut self, dist: f32) -> MeasureResult {
        self.move_ptr(dist);
        let t = self.t(dist);
        macro_rules! dispatched_call {
            ([$v:expr] ($seg:ident, $pair:ident) => $code:block) => {
                #[allow(unused_parens)]
                match $v {
                    SegmentWrapper::Line($seg, $pair) => $code,
                    SegmentWrapper::Quadratic($seg, $pair) => $code,
                    SegmentWrapper::Cubic($seg, $pair) => $code,
                    _ => {}
                }
            };
        }
        dispatched_call!([self.to_segment(self.events[self.edges[self.ptr].index])] (segment, pair) => {
            return MeasureResult {
                position: segment.sample(t),
                tangent: segment.derivative(t).normalize(),
                attributes_store: Self::lerp(self.attributes_store.get(pair.0), self.attributes_store.get(pair.1), t, &mut self.attribute_buffer),
            }
        });
        unreachable!();
    }

    fn add_segment(
        &mut self,
        ptr: usize,
        range: Option<Range<f32>>,
        dest: &mut BuilderWithAttributes,
    ) {
        let segment = self.to_segment(self.events[ptr]);
        let segment = match range.clone() {
            Some(range) => segment.split(range),
            None => segment,
        };
        macro_rules! obtain_attrs {
            ($p:ident) => {
                match range {
                    Some(range) => {
                        if range.end == 1.0 {
                            self.attributes_store.get($p.1)
                        } else {
                            Self::lerp(
                                self.attributes_store.get($p.0),
                                self.attributes_store.get($p.1),
                                range.end,
                                &mut self.attribute_buffer,
                            )
                        }
                    }
                    None => self.attributes_store.get($p.1),
                }
            };
        }
        match segment {
            SegmentWrapper::Line(LineSegment { to, .. }, pair) => {
                dest.line_to(to, obtain_attrs!(pair));
            }
            SegmentWrapper::Quadratic(QuadraticBezierSegment { ctrl, to, .. }, pair) => {
                dest.quadratic_bezier_to(ctrl, to, obtain_attrs!(pair));
            }
            SegmentWrapper::Cubic(
                CubicBezierSegment {
                    ctrl1, ctrl2, to, ..
                },
                pair,
            ) => {
                dest.cubic_bezier_to(ctrl1, ctrl2, to, obtain_attrs!(pair));
            }
            _ => {}
        }
    }

    /// Return the curve inside a given range of relative distance. See [`PathMeasure::split_by_distance`].
    ///
    /// # Panics
    ///
    /// Panics if `range` is not in [0.0, 1] or the path is empty.
    pub fn split(&mut self, range: Range<f32>) -> Path {
        let len = self.length();
        self.split_by_distance((range.start * len)..(range.end * len))
    }

    /// Return the curve inside a given range of relative distance. See [`PathMeasure::split`].
    ///
    /// # Panics
    ///
    /// Panics if `range` is not in [0.0, 1] or the path is empty.
    pub fn split_by_distance(&mut self, range: Range<f32>) -> Path {
        self.require_not_empty();
        if range.is_empty() {
            return Path::new();
        }
        if range.start < 0.0 || range.end > self.length() {
            panic!("Illegal range: {:?}", range);
        }
        let mut path = Path::builder_with_attributes(self.num_attributes());
        let result = self.measure_by_distance(range.start);
        path.begin(result.position, result.attributes_store);
        let (ptr1, seg1) = (self.ptr, self.edges[self.ptr].index);
        self.move_ptr(range.end);
        let (ptr2, seg2) = (self.ptr, self.edges[self.ptr].index);

        if seg1 == seg2 {
            self.ptr = ptr1;
            let t_begin = self.t(range.start);
            self.ptr = ptr2;
            let t_end = self.t(range.end);
            self.add_segment(
                seg1,
                Some(t_begin..t_end),
                &mut path,
            );
        } else {
            self.ptr = ptr1;
            self.add_segment(seg1, Some(self.t(range.start)..1.0), &mut path);
            for seg in (seg1 + 1)..seg2 {
                self.add_segment(seg, None, &mut path);
            }
            self.ptr = ptr2;
            self.add_segment(seg2, Some(0.0..self.t(range.end)), &mut path);
        }
        path.end(false);
        path.build()
    }
}

impl<'l, PS: PositionStore> PathMeasure<'l, PS, EmptyAttribtueStore> {
    pub fn new<Iter>(path: Iter, tolerance: f32, points: &'l PS) -> Self
    where
        Iter: IntoIterator<Item = IdEvent>,
    {
        Self::with_attributes(0, path, tolerance, points, &EmptyAttribtueStore)
    }
}

impl<'l> PathMeasure<'l, Path, EmptyAttribtueStore> {
    #[must_use]
    pub fn from_path(path: &'l Path, tolerance: f32) -> Self {
        Self::new(path.id_iter(), tolerance, path)
    }
}

impl<'l> PathMeasure<'l, Path, Path> {
    pub fn from_path_with_attributes(path: &'l Path, tolerance: f32) -> Self {
        Self::with_attributes(path.num_attributes(), path.id_iter(), tolerance, path, path)
    }
}

impl<'l> PathMeasure<'l, PathSlice<'l>, EmptyAttribtueStore> {
    pub fn from_slice(path: &'l PathSlice, tolerance: f32) -> Self {
        Self::new(path.id_iter(), tolerance, path)
    }
}

impl<'l> PathMeasure<'l, PathSlice<'l>, PathSlice<'l>> {
    pub fn from_slice_with_attributes(path: &'l PathSlice, tolerance: f32) -> Self {
        Self::with_attributes(path.num_attributes(), path.id_iter(), tolerance, path, path)
    }
}

#[test]
fn measure_line() {
    let mut path = Path::builder();
    path.begin(point(1.0, 1.0));
    path.line_to(point(0.0, 0.0));
    path.end(false);
    let path = path.build();
    let mut measure = PathMeasure::from_path(&path, 0.01);
    for t in [0.0, 0.2, 0.3, 0.5, 1.0] {
        let result = measure.measure(t);
        assert!((result.position - point(1.0 - t, 1.0 - t)).length() < 1e-5);
        assert_eq!(result.tangent, vector(-1.0, -1.0).normalize());
    }
}

#[test]
fn measure_square() {
    let mut path = Path::builder();
    path.begin(point(0.0, 0.0));
    path.line_to(point(1.0, 0.0));
    path.line_to(point(1.0, 1.0));
    path.line_to(point(0.0, 1.0));
    path.close();
    let path = path.build();
    let mut measure = PathMeasure::from_path(&path, 0.01);
    for (t, position, tangent) in [
        (0.125, point(0.5, 0.0), vector(1.0, 0.0)),
        (0.375, point(1.0, 0.5), vector(0.0, 1.0)),
        (0.625, point(0.5, 1.0), vector(-1.0, 0.0)),
        (0.875, point(0.0, 0.5), vector(0.0, -1.0)),
    ] {
        let result = measure.measure(t);
        assert!((result.position - position).length() < 1e-5);
        assert_eq!(result.tangent, tangent);
    }
}

#[test]
fn measure_attributes() {
    let mut path = Path::builder_with_attributes(2);
    path.begin(point(0.0, 0.0), Attributes(&[1.0, 2.0]));
    path.line_to(point(1.0, 0.0), Attributes(&[2.0, 3.0]));
    path.line_to(point(1.0, 1.0), Attributes(&[0.0, 0.0]));
    path.end(false);
    let path = path.build();
    let mut measure = PathMeasure::from_path_with_attributes(&path, 0.01);
    for (t, position, attrs) in [
        (0.25, point(0.5, 0.0), Attributes(&[1.5, 2.5])),
        (0.5, point(1.0, 0.0), Attributes(&[2.0, 3.0])),
        (0.75, point(1.0, 0.5), Attributes(&[1.0, 1.5])),
    ] {
        let result = measure.measure(t);
        assert!((result.position - position).length() < 1e-5);
        for i in 0..2 {
            assert!((result.attributes_store[i] - attrs[i]).abs() < 1e-5);
        }
    }
}

#[test]
fn measure_bezier_curve() {
    let mut path = Path::builder();
    path.begin(point(0.0, 0.0));
    path.quadratic_bezier_to(point(0.5, 0.7), point(1.0, 0.0));
    path.quadratic_bezier_to(point(1.5, -0.7), point(2.0, 0.0));
    path.end(false);
    let path = path.build();
    let mut measure = PathMeasure::from_path(&path, 0.01);
    for t in [0.25, 0.75] {
        let result = measure.measure(t);
        assert_eq!(result.tangent, vector(1.0, 0.0));
    }
    for (t, position) in [
        (0.0, point(0.0, 0.0)),
        (0.5, point(1.0, 0.0)),
        (1.0, point(2.0, 0.0)),
    ] {
        let result = measure.measure(t);
        assert_eq!(result.position, position);
    }
}

#[test]
fn split_square() {
    use crate::path::Event;

    let mut path = Path::builder();
    path.begin(point(0.0, 0.0));
    path.line_to(point(1.0, 0.0));
    path.line_to(point(1.0, 1.0));
    path.line_to(point(0.0, 1.0));
    path.close();
    let path = path.build();
    let mut measure = PathMeasure::from_path(&path, 0.01);
    let result = measure.split(0.125..0.625);
    assert_eq!(
        result.iter().collect::<Vec<_>>(),
        vec![
            Event::Begin {
                at: point(0.5, 0.0)
            },
            Event::Line {
                from: point(0.5, 0.0),
                to: point(1.0, 0.0)
            },
            Event::Line {
                from: point(1.0, 0.0),
                to: point(1.0, 1.0)
            },
            Event::Line {
                from: point(1.0, 1.0),
                to: point(0.5, 1.0)
            },
            Event::End {
                last: point(0.5, 1.0),
                first: point(0.5, 0.0),
                close: false
            },
        ]
    );
}

#[test]
fn split_bezier_curve() {
    use crate::path::Event;

    let mut path = Path::builder();
    path.begin(point(0.0, 0.0));
    path.quadratic_bezier_to(point(1.0, 1.0), point(2.0, 0.0));
    path.end(false);
    let path = path.build();
    let mut measure = PathMeasure::from_path(&path, 0.01);
    let result = measure.split(0.5..1.0);
    assert_eq!(
        result.iter().collect::<Vec<_>>(),
        vec![
            Event::Begin {
                at: point(1.0, 0.5)
            },
            Event::Quadratic {
                from: point(1.0, 0.5),
                ctrl: point(1.5, 0.5),
                to: point(2.0, 0.0),
            },
            Event::End {
                last: point(2.0, 0.0),
                first: point(1.0, 0.5),
                close: false
            }
        ]
    );
}

#[test]
fn split_attributes() {
    use crate::path::Event;

    let mut path = Path::builder_with_attributes(2);
    path.begin(point(0.0, 0.0), Attributes(&[1.0, 2.0]));
    path.line_to(point(1.0, 0.0), Attributes(&[2.0, 3.0]));
    path.line_to(point(1.0, 1.0), Attributes(&[0.0, 0.0]));
    path.end(false);
    let path = path.build();
    let mut measure = PathMeasure::from_path_with_attributes(&path, 0.01);
    assert_eq!(
        measure
            .split(0.0..1.0)
            .iter_with_attributes()
            .collect::<Vec<_>>(),
        path.iter_with_attributes().collect::<Vec<_>>()
    );
    let result = measure.split(0.25..0.75);
    assert_eq!(
        result.iter_with_attributes().collect::<Vec<_>>(),
        vec![
            Event::Begin {
                at: (point(0.5, 0.0), Attributes(&[1.5, 2.5]))
            },
            Event::Line {
                from: (point(0.5, 0.0), Attributes(&[1.5, 2.5])),
                to: (point(1.0, 0.0), Attributes(&[2.0, 3.0])),
            },
            Event::Line {
                from: (point(1.0, 0.0), Attributes(&[2.0, 3.0])),
                to: (point(1.0, 0.5), Attributes(&[1.0, 1.5])),
            },
            Event::End {
                last: (point(1.0, 0.5), Attributes(&[1.0, 1.5])),
                first: (point(0.5, 0.0), Attributes(&[1.5, 2.5])),
                close: false
            }
        ]
    );
}
