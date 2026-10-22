use alloc::vec::Vec;

// In std builds, `tan` comes from the inherent f32 method. It is only provided
// by the trait in no_std builds.
#[cfg(not(feature = "std"))]
use num_traits::Float;
use path::{builder::PathBuilder, Attributes, EndpointId, PathEvent, NO_ATTRIBUTES};

use crate::*;

/// Parameters for converting a stroked path into a fill.
///
/// The produced path must be filled with the non-zero fill rule.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct StrokeToFillOptions {
    pub(crate) offsets: (f32, f32),
    pub(crate) miter_limit: f32,
    pub(crate) tolerance: f32,
    pub(crate) line_join: LineJoin,
    pub(crate) start_cap: LineCap,
    pub(crate) end_cap: LineCap,
    pub(crate) add_empty_caps: bool,
}

impl StrokeToFillOptions {
    /// Minimum miter limit as defined by the SVG specification.
    ///
    /// See [StrokeMiterLimitProperty](https://svgwg.org/specs/strokes/#StrokeMiterlimitProperty)
    pub const MINIMUM_MITER_LIMIT: f32 = 1.0;
    /// Default miter limit as defined by the SVG specification.
    ///
    /// See [StrokeMiterLimitProperty](https://svgwg.org/specs/strokes/#StrokeMiterlimitProperty)
    pub const DEFAULT_MITER_LIMIT: f32 = 4.0;
    pub const DEFAULT_LINE_CAP: LineCap = LineCap::Butt;
    pub const DEFAULT_LINE_JOIN: LineJoin = LineJoin::Miter;
    pub const DEFAULT_LINE_WIDTH: f32 = 1.0;
    pub const DEFAULT_TOLERANCE: f32 = 0.1;

    #[inline]
    pub fn new(width: f32) -> Self {
        StrokeToFillOptions {
            offsets: (-width * 0.5, width * 0.5),
            miter_limit: Self::DEFAULT_MITER_LIMIT,
            tolerance: Self::DEFAULT_TOLERANCE,
            line_join: Self::DEFAULT_LINE_JOIN,
            start_cap: Self::DEFAULT_LINE_CAP,
            end_cap: Self::DEFAULT_LINE_CAP,
            add_empty_caps: true,
        }
    }

    #[inline]
    pub fn with_line_width(mut self, width: f32) -> Self {
        self.offsets = (-width * 0.5, width * 0.5);
        self
    }

    /// Instead of using a symmetric line width along the path, specify
    /// two offsets.
    ///
    /// `StrokeToFillOptions::with_line_width(w)` is equivalent to
    /// `StrokeToFillOptions::with_offsets(-w * 0.5, w * 0.5)`.
    #[inline]
    pub fn with_offsets(mut self, first: f32, second: f32) -> Self {
        self.offsets = (first, second);
        self
    }

    #[inline]
    pub fn with_line_join(mut self, join: LineJoin) -> Self {
        self.line_join = join;
        self
    }

    #[inline]
    pub fn with_line_cap(mut self, cap: LineCap) -> Self {
        self.start_cap = cap;
        self.end_cap = cap;
        self
    }

    #[inline]
    pub fn with_start_cap(mut self, cap: LineCap) -> Self {
        self.start_cap = cap;
        self
    }

    #[inline]
    pub fn with_end_cap(mut self, cap: LineCap) -> Self {
        self.end_cap = cap;
        self
    }

    /// Sets the miter limit, clamped to at least
    /// [`StrokeToFillOptions::MINIMUM_MITER_LIMIT`].
    #[inline]
    pub fn with_miter_limit(mut self, limit: f32) -> Self {
        self.miter_limit = limit.max(Self::MINIMUM_MITER_LIMIT);
        self
    }

    #[inline]
    pub fn with_tolerance(mut self, tolerance: f32) -> Self {
        self.tolerance = tolerance;
        self
    }

    #[inline]
    pub fn with_empty_caps(mut self, add_empty_caps: bool) -> Self {
        self.add_empty_caps = add_empty_caps;
        self
    }
}

impl Default for StrokeToFillOptions {
    fn default() -> Self {
        StrokeToFillOptions::new(StrokeToFillOptions::DEFAULT_LINE_WIDTH)
    }
}

/// A context object that can build the contour of a stroked path.
///
/// The produced paths self-overlap and must be filled with the non-zero
/// fill rule to properly represent the stroke.
#[derive(Default)]
pub struct Stroker {
    /// Storage for StrokeToFillBuilder::opposite, kept between calls to avoid
    /// reallocating it.
    opposite: SegmentBuffer,
}

impl Stroker {
    pub fn new() -> Stroker {
        Stroker {
            opposite: SegmentBuffer::default(),
        }
    }

    /// Compute the contour of a stroked path.
    ///
    /// The produced paths self-overlap and must be filled with the non-zero
    /// fill rule to properly represent the stroke.
    pub fn stroke_to_fill(
        &mut self,
        input: impl Iterator<Item = PathEvent>,
        options: &StrokeToFillOptions,
        output: &mut dyn PathBuilder,
    ) {
        let mut stroker = StrokeToFillBuilder::new(output, options);
        stroker.opposite = core::mem::take(&mut self.opposite);
        for event in input {
            match event {
                PathEvent::Begin { at } => {
                    stroker.begin(at);
                }
                PathEvent::End { close, .. } => {
                    stroker.end(close);
                }
                PathEvent::Line { to, .. } => {
                    stroker.line_to(to);
                }
                PathEvent::Quadratic { ctrl, to, .. } => {
                    stroker.quadratic_bezier_to(ctrl, to);
                }
                PathEvent::Cubic {
                    ctrl1, ctrl2, to, ..
                } => {
                    stroker.cubic_bezier_to(ctrl1, ctrl2, to);
                }
            }
        }

        self.opposite = core::mem::take(&mut stroker.opposite);
    }
}

pub struct StrokeToFillBuilder<'l> {
    output: &'l mut dyn PathBuilder,
    /// The second side of the stroke, in the direction of the source path. It
    /// is added to the output in reverse order at the end of each sub-path.
    opposite: SegmentBuffer,
    offsetter: Offsetter<2>,
    offsets: (f32, f32),
    start_cap: LineCap,
    end_cap: LineCap,
    add_empty_caps: bool,
}

impl<'l> StrokeToFillBuilder<'l> {
    pub fn new(output: &'l mut dyn PathBuilder, options: &StrokeToFillOptions) -> Self {
        let mut offsetter = Offsetter::new(
            [options.offsets.0, options.offsets.1],
            options.line_join,
            options.miter_limit,
            options.tolerance,
            true,
        );
        // Open sub-paths get caps even if they end at their start point.
        offsetter.close_if_end_at_start = false;

        StrokeToFillBuilder {
            output,
            opposite: SegmentBuffer::default(),
            offsetter,
            offsets: options.offsets,
            start_cap: options.start_cap,
            end_cap: options.end_cap,
            add_empty_caps: options.add_empty_caps,
        }
    }

    #[inline]
    pub fn begin(&mut self, at: Point) {
        self.offsetter.begin(at);
    }

    #[inline]
    pub fn line_to(&mut self, to: Point) {
        let mut outputs: [&mut dyn PathBuilder; 2] = [&mut *self.output, &mut self.opposite];
        self.offsetter.line_to(to, &mut outputs);
    }

    #[inline]
    pub fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
        let mut outputs: [&mut dyn PathBuilder; 2] = [&mut *self.output, &mut self.opposite];
        self.offsetter.quadratic_bezier_to(ctrl, to, &mut outputs);
    }

    #[inline]
    pub fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
        let mut outputs: [&mut dyn PathBuilder; 2] = [&mut *self.output, &mut self.opposite];
        self.offsetter.cubic_bezier_to(ctrl1, ctrl2, to, &mut outputs);
    }

    pub fn end(&mut self, close: bool) {
        let mut outputs: [&mut dyn PathBuilder; 2] = [&mut *self.output, &mut self.opposite];
        let Some(closed) = self.offsetter.end(close, &mut outputs) else {
            if self.add_empty_caps {
                self.add_empty_cap();
            }
            return;
        };

        if closed {
            // Each side is a separate contour.
            self.output.end(true);
            self.output.begin(self.opposite.last_point(), NO_ATTRIBUTES);
            self.opposite.add_reversed_to(&mut *self.output);
            self.output.end(true);
        } else {
            // A single contour that goes along one side, around the end cap, back
            // along the other side and around the start cap.
            self.add_end_cap();
            self.opposite.add_reversed_to(&mut *self.output);
            self.add_start_cap();
            self.output.end(true);
        }
    }

    fn add_start_cap(&mut self) {
        let endpoint = self.offsetter.first_endpoint;
        let tangent = -self.offsetter.first_tangent;
        let n = normal(tangent);
        let width = (self.offsets.0 - self.offsets.1).abs() * 0.5;
        let pivot_offset = (self.offsets.0 + self.offsets.1) * 0.5;
        let pivot = endpoint - n * pivot_offset;
        let end = endpoint - n * self.offsets.0;
        let sign = (self.offsets.0 - self.offsets.1).signum();
        get_cap_fn(self.start_cap)(
            &mut *self.output,
            pivot,
            tangent,
            end,
            width,
            sign,
            self.offsetter.tolerance,
        );
    }

    fn add_end_cap(&mut self) {
        let tangent = self.offsetter.prev_tangent;
        let n = normal(tangent);
        let width = (self.offsets.0 - self.offsets.1).abs() * 0.5;
        let pivot_offset = (self.offsets.0 + self.offsets.1) * 0.5;
        let pivot = self.offsetter.prev_endpoint + n * pivot_offset;
        let end = self.offsetter.prev_endpoint + n * self.offsets.1;
        let sign = (self.offsets.0 - self.offsets.1).signum();
        get_cap_fn(self.end_cap)(
            &mut *self.output,
            pivot,
            tangent,
            end,
            width,
            sign,
            self.offsetter.tolerance,
        );
    }

    fn add_empty_cap(&mut self) {
        // Inverted caps carve into the stroke, and a zero-length sub-path has
        // no stroke to carve into, so they act like butt caps here.
        let empty_cap = |cap| match cap {
            LineCap::RoundInverted | LineCap::TriangleInverted => LineCap::Butt,
            cap => cap,
        };
        let start_cap = empty_cap(self.start_cap);
        let end_cap = empty_cap(self.end_cap);

        // Butt caps have no area, so a zero-length sub-path with them draws
        // nothing.
        if start_cap == LineCap::Butt && end_cap == LineCap::Butt {
            return;
        }

        let tangent = Vector::new(1.0, 0.0);
        let normal = normal(tangent);

        // We don't have a real tangent so we don't offset the pivot.
        let pivot = self.offsetter.first_endpoint;

        let width = (self.offsets.0 - self.offsets.1).abs() * 0.5;
        let p0 = pivot + normal * width;
        let p1 = pivot - normal * width;

        let tolerance = self.offsetter.tolerance;
        let output = &mut *self.output;
        output.begin(p0, NO_ATTRIBUTES);
        get_cap_fn(end_cap)(output, pivot, tangent, p1, width, 1.0, tolerance);
        get_cap_fn(start_cap)(output, pivot, -tangent, p0, width, 1.0, tolerance);
        output.end(true);
    }
}

impl<'a> PathBuilder for StrokeToFillBuilder<'a> {
    fn num_attributes(&self) -> usize {
        0
    }

    fn begin(&mut self, at: Point, _: Attributes) -> EndpointId {
        self.begin(at);
        EndpointId::INVALID
    }

    fn end(&mut self, close: bool) {
        self.end(close);
    }

    fn line_to(&mut self, to: Point, _: Attributes) -> EndpointId {
        self.line_to(to);
        EndpointId::INVALID
    }

    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point, _: Attributes) -> EndpointId {
        self.quadratic_bezier_to(ctrl, to);
        EndpointId::INVALID
    }

    fn cubic_bezier_to(
        &mut self,
        ctrl1: Point,
        ctrl2: Point,
        to: Point,
        _: Attributes,
    ) -> EndpointId {
        self.cubic_bezier_to(ctrl1, ctrl2, to);
        EndpointId::INVALID
    }
}

fn get_cap_fn(cap: LineCap) -> CapBuilder {
    match cap {
        LineCap::Butt => butt_cap,
        LineCap::Square => square_cap,
        LineCap::Round => round_cap_cubic,
        LineCap::RoundInverted => round_inverted_cap,
        LineCap::Triangle => triangle_cap,
        LineCap::TriangleInverted => triangle_inverted_cap,
    }
}

#[derive(Copy, Clone)]
enum RecordedSegment {
    Line { to: Point },
    Quadratic { ctrl: Point, to: Point },
    Cubic { ctrl1: Point, ctrl2: Point, to: Point },
}

impl RecordedSegment {
    fn to(&self) -> Point {
        match *self {
            RecordedSegment::Line { to }
            | RecordedSegment::Quadratic { to, .. }
            | RecordedSegment::Cubic { to, .. } => to,
        }
    }
}

/// Records a sub-path, to add it to another path in reverse order.
#[derive(Default)]
struct SegmentBuffer {
    start: Point,
    segments: Vec<RecordedSegment>,
}

impl SegmentBuffer {
    fn last_point(&self) -> Point {
        self.segments.last().map_or(self.start, RecordedSegment::to)
    }

    /// Adds the segments to `output` in reverse order, starting from the
    /// current position of `output`.
    fn add_reversed_to(&self, output: &mut dyn PathBuilder) {
        // Each segment, reversed, ends where the previous one starts.
        for (i, segment) in self.segments.iter().enumerate().rev() {
            let to = match i {
                0 => self.start,
                _ => self.segments[i - 1].to(),
            };
            match *segment {
                RecordedSegment::Line { .. } => {
                    output.line_to(to, NO_ATTRIBUTES);
                }
                RecordedSegment::Quadratic { ctrl, .. } => {
                    output.quadratic_bezier_to(ctrl, to, NO_ATTRIBUTES);
                }
                RecordedSegment::Cubic { ctrl1, ctrl2, .. } => {
                    output.cubic_bezier_to(ctrl2, ctrl1, to, NO_ATTRIBUTES);
                }
            }
        }
    }
}

impl PathBuilder for SegmentBuffer {
    fn num_attributes(&self) -> usize {
        0
    }

    fn begin(&mut self, at: Point, _: Attributes) -> EndpointId {
        self.start = at;
        self.segments.clear();
        EndpointId::INVALID
    }

    fn end(&mut self, _close: bool) {}

    fn line_to(&mut self, to: Point, _: Attributes) -> EndpointId {
        self.segments.push(RecordedSegment::Line { to });
        EndpointId::INVALID
    }

    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point, _: Attributes) -> EndpointId {
        self.segments.push(RecordedSegment::Quadratic { ctrl, to });
        EndpointId::INVALID
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point, _: Attributes) -> EndpointId {
        self.segments.push(RecordedSegment::Cubic { ctrl1, ctrl2, to });
        EndpointId::INVALID
    }
}

pub(crate) type CapBuilder = fn(
    output: &mut dyn PathBuilder,
    position: Point,
    tangent: Vector,
    end: Point,
    width: f32,
    sign: f32,
    tolerance: f32,
);

fn butt_cap(
    output: &mut dyn PathBuilder,
    _position: Point,
    _tangent: Vector,
    end: Point,
    _width: f32,
    _sign: f32,
    _tolerance: f32,
) {
    output.line_to(end, NO_ATTRIBUTES);
}

fn square_cap(
    output: &mut dyn PathBuilder,
    pivot: Point,
    tangent: Vector,
    end: Point,
    width: f32,
    sign: f32,
    _tolerance: f32,
) {
    let tw = tangent * width;
    let n = normal(tw) * sign;
    let mid = pivot + tw;
    let a = mid + n;
    let b = mid - n;

    output.line_to(a, NO_ATTRIBUTES);
    output.line_to(b, NO_ATTRIBUTES);
    output.line_to(end, NO_ATTRIBUTES);
}

fn round_cap_cubic(
    output: &mut dyn PathBuilder,
    pivot: Point,
    tangent: Vector,
    end: Point,
    width: f32,
    sign: f32,
    _tolerance: f32,
) {
    let normal = normal(tangent) * sign;
    let start = pivot + normal * width;

    // From https://pomax.github.io/bezierinfo/#circles
    let angle = core::f32::consts::PI * 0.5;
    let k = width.abs() * 4.0 / 3.0 * (angle * 0.25).tan();

    let mid = pivot + tangent * width;
    let ctrl1 = start + tangent * k;
    let ctrl2 = mid + normal * k;
    output.cubic_bezier_to(ctrl1, ctrl2, mid, NO_ATTRIBUTES);
    let ctrl1 = mid - normal * k;
    let ctrl2 = end + tangent * k;
    output.cubic_bezier_to(ctrl1, ctrl2, end, NO_ATTRIBUTES);
}

fn round_inverted_cap(
    output: &mut dyn PathBuilder,
    pivot: Point,
    tangent: Vector,
    end: Point,
    width: f32,
    sign: f32,
    _tolerance: f32,
) {
    round_cap_cubic(output, pivot, -tangent, end, width, -sign, _tolerance);
}

fn triangle_cap(
    output: &mut dyn PathBuilder,
    pivot: Point,
    tangent: Vector,
    end: Point,
    width: f32,
    _sign: f32,
    _tolerance: f32,
) {
    let tw = tangent * width;
    let mid = pivot + tw;

    output.line_to(mid, NO_ATTRIBUTES);
    output.line_to(end, NO_ATTRIBUTES);
}

fn triangle_inverted_cap(
    output: &mut dyn PathBuilder,
    pivot: Point,
    tangent: Vector,
    end: Point,
    width: f32,
    sign: f32,
    tolerance: f32,
) {
    triangle_cap(output, pivot, -tangent, end, width, -sign, tolerance)
}

fn normal(v: Vector) -> Vector {
    Vector::new(-v.y, v.x)
}
