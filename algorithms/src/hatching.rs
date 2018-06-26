use path::PathEvent;
use path::builder::{FlatPathBuilder, PathBuilder};
use geom::LineSegment;
use geom::math::{Point, Vector, point, vector};
use geom::euclid::{Angle, Rotation2D};
use std::marker::PhantomData;

use std::cmp::Ordering;
use std::mem;
use std::f32;

/// Parameters for the hatcher.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct HatchingOptions {
    /// Maximum allowed distance to the path when building an approximation.
    ///
    /// See [Flattening and tolerance](index.html#flattening-and-tolerance).
    ///
    /// Default value: `HatchingOptions::DEFAULT_TOLERANCE`.
    pub tolerance: f32,
    /// Angle between the hatching pattern and the x axis.
    ///
    /// Default value: `HatchingOptions::ANGLE`.
    pub angle: Angle<f32>,
    /// Whether to compute the tangent of the outline where it meets the hatching pattern.
    ///
    /// Default value: `true, .
    pub compute_tangents: bool,

    /// The origin of the rotated uv coordinates.
    pub uv_origin: Point,

    // To be able to add fields without making it a breaking change, add an empty private field
    // which makes it impossible to create a StrokeOptions without calling the constructor.
    _private: (),
}

impl Default for HatchingOptions {
    fn default() -> Self { Self::DEFAULT }
}

impl HatchingOptions {
    /// Default flattening tolerance.
    pub const DEFAULT_TOLERANCE: f32 = 0.1;
    /// Default hatching angle.
    pub const DEFAULT_ANGLE: Angle<f32> = Angle { radians: 0.0 };
    pub const DEFAULT_UV_ORIGIN: Point = Point { x: 0.0, y: 0.0, _unit: PhantomData };

    pub const DEFAULT: Self = HatchingOptions {
        tolerance: Self::DEFAULT_TOLERANCE,
        angle: Self::DEFAULT_ANGLE,
        compute_tangents: true,
        uv_origin: Self::DEFAULT_UV_ORIGIN,
        _private: (),
    };

    #[inline]
    pub fn tolerance(tolerance: f32) -> Self {
        Self::DEFAULT.with_tolerance(tolerance)
    }

    #[inline]
    pub fn angle(angle: Angle<f32>) -> Self {
        Self::DEFAULT.with_angle(angle)
    }

    #[inline]
    pub fn with_tolerance(mut self, tolerance: f32) -> Self {
        self.tolerance = tolerance;
        self
    }

    #[inline]
    pub fn with_angle(mut self, angle: Angle<f32>) -> Self {
        self.angle = angle;
        self
    }

    #[inline]
    pub fn with_tangents(mut self, compute_tangents: bool) -> Self {
        self.compute_tangents = compute_tangents;
        self
    }
}

/// Parameters for generating dot patterns.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct DotOptions {
    /// Maximum allowed distance to the path when building an approximation.
    ///
    /// See [Flattening and tolerance](index.html#flattening-and-tolerance).
    ///
    /// Default value: `HatchingOptions::DEFAULT_TOLERANCE`.
    pub tolerance: f32,
    /// Angle between the hatching pattern and the x axis.
    ///
    /// Default value: `HatchingOptions::ANGLE`.
    pub angle: Angle<f32>,
    /// The origin of the rotated uv coordinates.
    pub uv_origin: Point,

    // To be able to add fields without making it a breaking change, add an empty private field
    // which makes it impossible to create a StrokeOptions without calling the constructor.
    _private: (),
}

impl Default for DotOptions {
    fn default() -> Self { Self::DEFAULT }
}

impl DotOptions {
    /// Default flattening tolerance.
    pub const DEFAULT_TOLERANCE: f32 = 0.1;
    /// Default inclination of the dot pattern.
    pub const DEFAULT_ANGLE: Angle<f32> = Angle { radians: 0.0 };
    pub const DEFAULT_UV_ORIGIN: Point = Point { x: 0.0, y: 0.0, _unit: PhantomData };

    pub const DEFAULT: Self = DotOptions {
        tolerance: Self::DEFAULT_TOLERANCE,
        angle: Self::DEFAULT_ANGLE,
        uv_origin: Self::DEFAULT_UV_ORIGIN,
        _private: (),
    };

    #[inline]
    pub fn tolerance(tolerance: f32) -> Self {
        Self::DEFAULT.with_tolerance(tolerance)
    }

    #[inline]
    pub fn angle(angle: Angle<f32>) -> Self {
        Self::DEFAULT.with_angle(angle)
    }

    #[inline]
    pub fn with_tolerance(mut self, tolerance: f32) -> Self {
        self.tolerance = tolerance;
        self
    }

    #[inline]
    pub fn with_angle(mut self, angle: Angle<f32>) -> Self {
        self.angle = angle;
        self
    }
}

type Edge = LineSegment<f32>;

pub struct HatchSegment {
    /// Left endpoint.
    pub a: HatchEndpoint,
    /// Right endpoint.
    pub b: HatchEndpoint,
    /// Index of the current row.
    pub row: u32,
    /// Rotated position along a direction perpendicular to the hatching pattern.
    ///
    /// This position is relative to `uv_origin` specified in the `HatchingOptions`.
    pub v: f32,
}

pub struct HatchEndpoint {
    /// Position in world space of the point.
    pub position: Point,
    /// Tangent of the path edge at this point (pointing downward).
    pub tangent: Vector,
    /// Rotated position along the direction of the hatching pattern.
    ///
    /// This position is relative to `uv_origin` specified in the `HatchingOptions`.
    pub u: f32,
}

/// The output of `Hatcher::hatch_path`.
///
/// Implement this trait to create custom hatching patterns.
pub trait HatchBuilder {
    /// Called for each hatch segment.
    fn add_segment(&mut self, segment: &HatchSegment);
    /// Specifies the distance between each row of the pattern.
    fn next_offset(&mut self, row_idx: u32) -> f32;
}

pub struct Dot {
    /// World-space position of the dot.
    pub position: Point,
    /// Rotated position along an axis parallel to the rows of the pattern.
    pub u: f32,
    /// Rotated position along an axis parallel to the columns of the pattern.
    pub v: f32,
    /// Index of the current column.
    pub column: u32,
    /// Index of the current row.
    pub row: u32,
}

/// The output of `Hatcher::dot_path`.
///
/// Implement this trait to create custom dot patterns.
pub trait DotBuilder {
    /// Offset of the first dot after a left edge.
    fn first_column_offset(&mut self, _row: u32) -> f32 { 0.0 }
    /// Whether and how much to align the dots for a given row.
    fn alignment(&mut self, _row: u32) -> Option<f32> { None }
    /// Called for each row of dots.
    fn next_row_offset(&mut self, column: u32, row: u32) -> f32;
    /// Distance between each dot in a given row.
    fn next_column_offset(&mut self, column: u32, row: u32) -> f32;

    /// Called for each dot.
    fn add_dot(&mut self, dot: &Dot);
}

/// A `DotBuilder` implementation for dot patterns with constant intervals.
pub struct RegularDotPattern<Cb: FnMut(&Dot)> {
    /// Minimum distance between dots in a given column.
    pub column_interval: f32,
    /// Minimum distance between dots in a given row.
    pub row_interval: f32,
    /// A callback invoked for each dot.
    pub callback: Cb,
}

/// A context object that can fill a path with a hatching or dot pattern.
pub struct Hatcher {
    events: HatchingEvents,
    active_edges: Vec<Edge>,
    transform: Rotation2D<f32>,
    compute_tangents: bool,
    segment: HatchSegment,
    uv_origin: Point,
}

impl Hatcher {
    /// Constructor.
    pub fn new() -> Self {
        Hatcher {
            events: HatchingEvents::new(),
            active_edges: Vec::new(),
            transform: Rotation2D::identity(),
            compute_tangents: true,
            segment: HatchSegment {
                a: HatchEndpoint {
                    position: point(0.0, 0.0),
                    tangent: vector(f32::NAN, f32::NAN),
                    u: 0.0,
                },
                b: HatchEndpoint {
                    position: point(0.0, 0.0),
                    tangent: vector(f32::NAN, f32::NAN),
                    u: 0.0,
                },
                row: 0,
                v: 0.0,
            },
            uv_origin: point(0.0, 0.0),
        }
    }

    /// Generate hatches for a path.
    pub fn hatch_path<Iter>(
        &mut self,
        it: Iter,
        options: &HatchingOptions,
        output: &mut HatchBuilder,
    )
    where
        Iter: Iterator<Item = PathEvent>,
    {
        let mut events = mem::replace(&mut self.events, HatchingEvents::new());
        events.set_path(options.tolerance, options.angle, it);

        self.hatch(&events, options, output);

        self.events = events;
    }

    /// Generate dots for a path.
    pub fn dot_path<Iter>(
        &mut self,
        it: Iter,
        options: &DotOptions,
        output: &mut DotBuilder,
    )
    where
        Iter: Iterator<Item = PathEvent>,
    {
        let mut events = mem::replace(&mut self.events, HatchingEvents::new());
        events.set_path(options.tolerance, options.angle, it);

        self.dot(&events, options, output);

        self.events = events;
    }

    fn hatch(
        &mut self,
        events: &HatchingEvents,
        options: &HatchingOptions,
        output: &mut HatchBuilder
    ) {
        self.transform = Rotation2D::new(-options.angle);
        self.uv_origin = Rotation2D::new(options.angle).transform_point(
            &options.uv_origin
        );
        self.active_edges.clear();
        self.segment.row = 0;
        self.segment.a.tangent = vector(f32::NAN, f32::NAN);
        self.segment.b.tangent = vector(f32::NAN, f32::NAN);
        self.compute_tangents = options.compute_tangents;

        let mut y = events.edges.first().unwrap().from.y + output.next_offset(0);
        let mut y_max = y;

        for edge in &events.edges {
            let y2 = edge.from.y;
            while y < y2 {
                self.hatch_line(y, output);
                let offset = output.next_offset(self.segment.row);
                y += offset;
                if offset <= 0.0 {
                    return;
                }
            }
            y_max = f32::max(y_max, edge.to.y);
            self.update_sweep_line(edge);
        }

        while y < y_max {
            self.hatch_line(y, output);
            let offset = output.next_offset(self.segment.row);
            y += offset;
            if offset <= 0.0 {
                return;
            }
        }
    }

    fn dot(
        &mut self,
        events: &HatchingEvents,
        options: &DotOptions,
        output: &mut DotBuilder
    ) {
        let mut dotted = HatchesToDots {
            builder: output,
            column: 0,
        };

        let options = HatchingOptions {
            tolerance: options.tolerance,
            angle: options.angle,
            uv_origin: options.uv_origin,
            compute_tangents: false,
            _private: (),
        };

        self.hatch(events, &options, &mut dotted);
    }

    fn update_sweep_line(&mut self, edge: &Edge) {
        self.active_edges.retain(|e| {
            compare_positions(e.to, edge.from) != Ordering::Less
        });
        self.active_edges.push(*edge);
    }

    fn hatch_line(&mut self, y: f32, output: &mut HatchBuilder) {
        self.active_edges.sort_by_key(|e| { Ordered(e.solve_x_for_y(y)) });

        let mut inside = false;
        let mut prev_x = f32::NAN;
        let mut prev_tangent = vector(f32::NAN, f32::NAN);
        let mut tangent = vector(f32::NAN, f32::NAN);
        self.segment.v = y - self.uv_origin.y;

        for active_edge in &self.active_edges {
            if active_edge.to.y < y {
                // TODO: we don't remove the edges during merge events so we can
                // end up with extra edges that end above the sweep line and have
                // to skip them. It would be better to properly manage the sweep
                // line instead!
                continue;
            }
            let x = active_edge.solve_x_for_y(y);
            if self.compute_tangents {
                tangent = self.transform.transform_vector(&active_edge.to_vector()).normalize();
            }

            if inside {
                self.segment.a.position = self.transform.transform_point(&point(prev_x, y));
                self.segment.b.position = self.transform.transform_point(&point(x, y));
                self.segment.a.u = prev_x - self.uv_origin.x;
                self.segment.b.u = x - self.uv_origin.x;
                if self.compute_tangents {
                    self.segment.a.tangent = prev_tangent;
                    self.segment.b.tangent = tangent;
                }

                output.add_segment(&self.segment);
            }

            inside = !inside;
            prev_x = x;
            prev_tangent = tangent;
        }

        self.segment.row += 1;
    }
}

struct HatchingEvents {
    edges: Vec<Edge>,
}

impl HatchingEvents {
    fn new() -> Self {
        HatchingEvents {
            edges: Vec::new()
        }
    }
}

struct EventsBuilder {
    edges: Vec<Edge>,
    angle: Angle<f32>,
    first: Point,
    current: Point,
    nth: u32,
}

impl EventsBuilder {
    fn new(angle: Angle<f32>) -> Self {
        EventsBuilder {
            edges: Vec::new(),
            angle,
            first: point(0.0, 0.0),
            current: point(0.0, 0.0),
            nth: 0,
        }
    }

    fn add_edge(&mut self, from: Point, to: Point) {
        let rotation = Rotation2D::new(self.angle);
        let mut from = rotation.transform_point(&from);
        let mut to = rotation.transform_point(&to);
        if compare_positions(from, to) == Ordering::Greater {
            mem::swap(&mut from, &mut to);
        }
        self.edges.push(Edge { from, to });
    }
}

impl FlatPathBuilder for EventsBuilder {
    type PathType = HatchingEvents;

    fn move_to(&mut self, to: Point) {
        self.close();
        let next = to;
        if self.nth > 1 {
            let current = self.current;
            let first = self.first;
            self.add_edge(current, first);
        }
        self.first = next;
        self.current = next;
        self.nth = 0;
    }

    fn line_to(&mut self, to: Point) {
        let next = to;
        if next == self.current {
            return;
        }
        let current = self.current;
        self.add_edge(current, next);
        self.current = next;
        self.nth += 1;
    }

    fn close(&mut self) {
        let current = self.current;
        let first = self.first;
        if self.current != self.first && self.nth > 0 {
            self.add_edge(current, first);
        }
        self.nth = 0;
        self.current = self.first;
    }

    fn build(mut self) -> HatchingEvents {
        self.build_and_reset()
    }

    fn build_and_reset(&mut self) -> HatchingEvents {
        self.close();

        self.first = point(0.0, 0.0);
        self.current = point(0.0, 0.0);
        self.nth = 0;

        self.edges.sort_by(|a, b| compare_positions(a.from, b.from));

        HatchingEvents {
            edges: mem::replace(&mut self.edges, Vec::new()),
        }
    }

    fn current_position(&self) -> Point {
        self.current
    }
}

impl HatchingEvents {

    pub fn set_path<Iter>(
        &mut self,
        tolerance: f32,
        angle: Angle<f32>,
        it: Iter
    )
        where Iter: Iterator<Item = PathEvent>
    {
        self.edges.clear();
        let mut builder = EventsBuilder::new(angle);
        builder.edges = mem::replace(&mut self.edges, Vec::new());

        let mut builder = builder.flattened(tolerance);
        for evt in it {
            builder.path_event(evt);
        }
        mem::swap(self, &mut builder.build());
    }
}

#[derive(PartialEq)]
struct Ordered(f32);
impl Eq for Ordered {}

impl PartialOrd for Ordered {
    fn partial_cmp(&self, other: &Ordered) -> Option<Ordering> {
        if self.0 > other.0 {
            return Some(Ordering::Greater);
        }

        if self.0 < other.0 {
            return Some(Ordering::Less);
        }

        Some(Ordering::Equal)
    }
}

impl Ord for Ordered {
    fn cmp(&self, other: &Ordered) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

fn compare_positions(a: Point, b: Point) -> Ordering {
    if a.y > b.y {
        return Ordering::Greater;
    }
    if a.y < b.y {
        return Ordering::Less;
    }
    if a.x > b.x {
        return Ordering::Greater;
    }
    if a.x < b.x {
        return Ordering::Less;
    }

    Ordering::Equal
}

impl<Cb: FnMut(&Dot)> DotBuilder for RegularDotPattern<Cb> {
    fn alignment(&mut self, _row: u32) -> Option<f32> { Some(self.column_interval) }
    fn next_row_offset(&mut self, _column: u32, _row: u32) -> f32 { self.row_interval }
    fn next_column_offset(&mut self, _column: u32, _row: u32) -> f32 { self.column_interval }
    fn add_dot(&mut self, dot: &Dot) { (self.callback)(dot) }
}

/// A `HatchBuilder` implementation for hatching patterns with constant intervals.
pub struct RegularHatchingPattern<Cb: FnMut(&HatchSegment)> {
    /// The distance between each row of hatches.
    pub interval: f32,
    /// A callback invoked for each segment.
    pub callback: Cb,
}

impl<Cb: FnMut(&HatchSegment)> HatchBuilder for RegularHatchingPattern<Cb> {
    fn next_offset(&mut self, _row: u32) -> f32 { self.interval }
    fn add_segment(&mut self, segment: &HatchSegment) { (self.callback)(segment) }
}

// Converts a hatching pattern into a dotted pattern.
struct HatchesToDots<'l> {
    builder: &'l mut DotBuilder,
    column: u32,
}

impl<'l> HatchBuilder for HatchesToDots<'l> {
    fn next_offset(&mut self, row: u32) -> f32 {
        let val = self.builder.next_row_offset(self.column, row);
        self.column = 0;

        val
    }

    fn add_segment(&mut self, segment: &HatchSegment) {
        let row = segment.row;
        let mut u = self.builder.first_column_offset(row);
        let u_start = segment.a.u;

        if let Some(d) = self.builder.alignment(row) {
            let m = modulo(u_start, d);
            if m != 0.0 {
                u += d - m;
            }
        }

        let a = segment.a.position;
        let ab = (segment.b.position - a).normalize();

        while u_start + u < segment.b.u {
            self.builder.add_dot(&Dot {
                position: a + ab * u,
                u: segment.a.u + u,
                v: segment.v,
                column: self.column,
                row,
            });

            self.column += 1;
            let du = self.builder.next_column_offset(self.column, row);
            if du <= 0.0 {
                return;
            }

            u += du;
        }
    }
}

fn modulo(a: f32, m: f32) -> f32 {
    if a >= 0.0 { a % m } else { m + (a % m) }
}
