use path::PathEvent;
use path::builder::{FlatPathBuilder, PathBuilder};
use geom::LineSegment;
use geom::math::{Point, Vector, point, vector};
use geom::euclid::{Angle, Rotation2D};

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

    pub const DEFAULT: Self = HatchingOptions {
        tolerance: Self::DEFAULT_TOLERANCE,
        angle: Self::DEFAULT_ANGLE,
        compute_tangents: true,
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

type Edge = LineSegment<f32>;

/// The output of the hatcher.
pub trait HatchBuilder {
    fn add_segment(
        &mut self,
        a: &Point,
        a_tangent: &Vector,
        b: &Point,
        b_tangent: &Vector,
        row_index: u32
    );

    fn next_offset(&mut self, row_idx: u32) -> f32;
}

/// A context object that can fill a path with a hatching pattern.
pub struct Hatcher {
    events: HatchingEvents,
    active_edges: Vec<Edge>,
    transform: Rotation2D<f32>,
    row_idx: u32,
    compute_tangents: bool,
}

impl Hatcher {
    /// Constructor.
    pub fn new() -> Self {
        Hatcher {
            events: HatchingEvents::new(),
            active_edges: Vec::new(),
            transform: Rotation2D::identity(),
            row_idx: 0,
            compute_tangents: true,
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

    fn hatch(
        &mut self,
        events: &HatchingEvents,
        options: &HatchingOptions,
        output: &mut HatchBuilder
    ) {
        self.transform = Rotation2D::new(-options.angle);
        self.active_edges.clear();
        self.row_idx = 0;
        self.compute_tangents = options.compute_tangents;

        let mut y = events.edges.first().unwrap().from.y + output.next_offset(0);
        let mut y_max = y;

        for edge in &events.edges {
            let y2 = edge.from.y;
            while y < y2 {
                self.hatch_line(y, output);
                let offset = output.next_offset(self.row_idx);
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
            let offset = output.next_offset(self.row_idx);
            y += offset;
            if offset <= 0.0 {
                return;
            }
        }
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
                tangent = active_edge.to_vector().normalize();
            }

            if inside {
                let a = self.transform.transform_point(&point(prev_x, y));
                let b = self.transform.transform_point(&point(x, y));

                output.add_segment(&a, &tangent, &b, &prev_tangent, self.row_idx);
            }

            inside = !inside;
            prev_x = x;
            prev_tangent = tangent;
        }

        self.row_idx += 1;
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
