//! Path offsetting.

use core::ops::Range;

// In std builds, `tan` and `sqrt` come from the inherent f32 methods. They are
// only provided by the trait in no_std builds.
#[cfg(not(feature = "std"))]
use num_traits::Float;

use crate::geom::arrayvec::ArrayVec;
use crate::geom::euclid;
use crate::geom::{
    utils::{cubic_polynomial_roots, tangent},
    QuadraticBezierSegment, Scalar,
    CubicBezierSegment, Line, LineSegment,
};
use path::{builder::PathBuilder, Attributes, EndpointId, NO_ATTRIBUTES};
use crate::math::{Point, Vector};

/// Line join as defined by the SVG specification.
///
/// See: <https://svgwg.org/specs/strokes/#StrokeLinejoinProperty>
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum LineJoin {
    /// A sharp corner is to be used to join path segments.
    Miter,
    /// Same as a miter join, but if the miter limit is exceeded,
    /// the miter is clipped at a miter length equal to the miter limit value
    /// multiplied by the stroke width.
    MiterClip,
    /// A round corner is to be used to join path segments.
    Round,
    /// A beveled corner is to be used to join path segments.
    /// The bevel shape is a triangle that fills the area between the two stroked
    /// segments.
    Bevel,
}

impl From<path::LineJoin> for LineJoin {
    fn from(join: path::LineJoin) -> Self {
        match join {
            path::LineJoin::Miter => LineJoin::Miter,
            path::LineJoin::MiterClip => LineJoin::MiterClip,
            path::LineJoin::Round => LineJoin::Round,
            path::LineJoin::Bevel => LineJoin::Bevel,
        }
    }
}

/// Parameters for offsetting paths.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OffsetOptions {
    pub(crate) offset: f32,
    pub(crate) line_join: LineJoin,
    pub(crate) miter_limit: f32,
    pub(crate) tolerance: f32,
    pub(crate) simplify_inner_joins: bool,
}

impl OffsetOptions {
    /// Minimum miter limit as defined by the SVG specification.
    ///
    /// See [StrokeMiterLimitProperty](https://svgwg.org/specs/strokes/#StrokeMiterlimitProperty)
    pub const MINIMUM_MITER_LIMIT: f32 = 1.0;
    /// Default miter limit as defined by the SVG specification.
    ///
    /// See [StrokeMiterLimitProperty](https://svgwg.org/specs/strokes/#StrokeMiterlimitProperty)
    pub const DEFAULT_MITER_LIMIT: f32 = 4.0;
    pub const DEFAULT_LINE_JOIN: LineJoin = LineJoin::Round;
    pub const DEFAULT_TOLERANCE: f32 = 0.1;

    #[inline]
    pub fn new(offset: f32) -> Self {
        OffsetOptions {
            offset,
            line_join: Self::DEFAULT_LINE_JOIN,
            miter_limit: Self::DEFAULT_MITER_LIMIT,
            tolerance: Self::DEFAULT_TOLERANCE,
            simplify_inner_joins: true,
        }
    }

    #[inline]
    pub const fn with_tolerance(mut self, tolerance: f32) -> Self {
        self.tolerance = tolerance;
        self
    }

    #[inline]
    pub const fn with_line_join(mut self, join: LineJoin) -> Self {
        self.line_join = join;
        self
    }

    /// Sets the miter limit, clamped to at least
    /// [`OffsetOptions::MINIMUM_MITER_LIMIT`].
    #[inline]
    pub fn with_miter_limit(mut self, limit: f32) -> Self {
        self.miter_limit = limit.max(Self::MINIMUM_MITER_LIMIT);
        self
    }

    /// Whether to attempt to reduce the amount of self-overlap occurring when
    /// offsetting a join inward.
    #[inline]
    pub fn with_simplified_inner_joins(mut self, simplify: bool) -> Self {
        self.simplify_inner_joins = simplify;
        self
    }
}

impl Default for OffsetOptions {
    fn default() -> Self {
        OffsetOptions::new(0.0)
    }
}

/// A path builder which builds paths with an offset applied.
///
/// See [`OffsetOptions`].
///
/// This builder does not support custom attributes.
pub struct OffsetBuilder<'l> {
    output: &'l mut dyn PathBuilder,
    offsetter: Offsetter<1>,
}

impl<'l> OffsetBuilder<'l> {
    #[inline]
    pub fn new(output: &'l mut dyn PathBuilder, options: &OffsetOptions) -> Self {
        OffsetBuilder {
            output,
            offsetter: Offsetter::new(
                [options.offset],
                options.line_join,
                options.miter_limit,
                options.tolerance,
                options.simplify_inner_joins,
            ),
        }
    }

    #[inline]
    pub fn begin(&mut self, at: Point) {
        self.offsetter.begin(at);
    }

    #[inline]
    pub fn end(&mut self, close: bool) {
        if let Some(closed) = self.offsetter.end(close, &mut [&mut *self.output]) {
            self.output.end(closed);
        }
    }

    #[inline]
    pub fn line_to(&mut self, to: Point) {
        self.offsetter.line_to(to, &mut [&mut *self.output]);
    }

    #[inline]
    pub fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
        self.offsetter.quadratic_bezier_to(ctrl, to, &mut [&mut *self.output]);
    }

    #[inline]
    pub fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
        self.offsetter.cubic_bezier_to(ctrl1, ctrl2, to, &mut [&mut *self.output]);
    }
}

impl<'a> PathBuilder for OffsetBuilder<'a> {
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


/// Offsets a path by one or two distances, each one being a side of the
/// offset path.
///
/// The source path is processed once for all sides: short segments are
/// discarded, curves are split and approximated, and joins are classified as
/// inner or outer. The sides can't disagree about any of these decisions.
///
/// Each side is written to its own output. The methods take the outputs in the
/// order of the sides.
pub(crate) struct Offsetter<const N: usize> {
    sides: [Side; N],
    /// End of the last segment of the source path, including the segments that
    /// were discarded for being too short. The next segment starts here, and is
    /// displaced to start at `prev_endpoint` instead (see
    /// `displace_quadratic_bezier` and `displace_cubic_bezier`).
    position: Point,
    /// End of the last segment added to the outputs.
    pub(crate) prev_endpoint: Point,
    pub(crate) first_endpoint: Point,
    pub(crate) prev_tangent: Vector,
    pub(crate) first_tangent: Vector,
    pub(crate) tolerance: f32,
    miter_limit: f32,
    simplify_inner_joins: bool,
    /// Treat open sub-paths that end at their start point as closed.
    pub(crate) close_if_end_at_start: bool,
    is_first_segment: bool,
    /// Line segments shorter than this are discarded. See
    /// `default_min_segment_length`.
    min_segment_length: f32,
    /// Number of segments added to the outputs so far.
    num_steps: u32,
    join: JoinBuilder,
}

impl<const N: usize> Offsetter<N> {
    pub fn new(
        offsets: [f32; N],
        join: LineJoin,
        miter_limit: f32,
        tolerance: f32,
        simplify_inner_joins: bool,
    ) -> Self {
        let no_segment = Seg::Line(LineSegment {
            from: Point::zero(),
            to: Point::zero(),
        });
        Offsetter {
            sides: offsets.map(|offset| Side {
                offset,
                prev: no_segment,
            }),
            miter_limit: miter_limit.max(1.0),
            tolerance: tolerance.max(0.0001),
            prev_endpoint: Point::zero(),
            position: Point::zero(),
            prev_tangent: Vector::zero(),
            first_endpoint: Point::zero(),
            first_tangent: Vector::zero(),
            is_first_segment: true,
            min_segment_length: tolerance.max(0.0001) * 0.25,
            num_steps: 0,
            close_if_end_at_start: true,
            join: match join {
                LineJoin::Bevel => bevel_join,
                LineJoin::Miter => miter_join,
                LineJoin::MiterClip => miter_clip_join,
                LineJoin::Round => round_join,
            },
            simplify_inner_joins,
        }
    }

    #[inline]
    pub fn begin(&mut self, at: Point) {
        self.prev_endpoint = at;
        self.position = at;
        self.first_endpoint = at;
        self.is_first_segment = true;
    }

    /// Ends the current sub-path.
    ///
    /// Returns `None` if nothing was added to the outputs, otherwise whether
    /// the sub-path is closed. The sub-paths of the outputs are not ended. If
    /// the sub-path is closed, they stop after the closing join.
    pub fn end(&mut self, close: bool, outputs: &mut [&mut dyn PathBuilder; N]) -> Option<bool> {
        if self.is_first_segment {
            self.min_segment_length = self.default_min_segment_length();
            return None;
        }

        let end_at_start = (self.prev_endpoint - self.first_endpoint).square_length()
            <= self.tolerance * self.tolerance;
        let close = close || (self.close_if_end_at_start && end_at_start);

        if close {
            // The closing segment is kept even if it is too short.
            // Discarding it would merge the joins at both of its ends into a single
            // one, which turns the wrong way when their angles add up to more than
            // 180 degrees.
            let min_segment_length = self.min_segment_length;
            self.min_segment_length = min_segment_length.min(self.tolerance * 0.001);
            self.line_to(self.first_endpoint, outputs);
            self.min_segment_length = min_segment_length;

            // Add the join between the last and first segments by stepping into
            // a stand-in for the first segment. It bypasses line_to so that it is
            // never discarded for being too short.
            let n = normal(self.first_tangent);
            let mut first = self.sides.map(|side| {
                let d = n * side.offset;
                Seg::Line(LineSegment {
                    from: self.first_endpoint + d,
                    to: self.first_endpoint + self.first_tangent + d,
                })
            });
            self.step(&mut first, self.first_tangent, false, outputs);
        } else {
            for (side, output) in self.sides.iter().zip(outputs.iter_mut()) {
                add_segment(&side.prev, *output);
            }
        }

        self.min_segment_length = self.default_min_segment_length();

        Some(close)
    }

    pub fn line_to(&mut self, to: Point, outputs: &mut [&mut dyn PathBuilder; N]) {
        self.position = to;
        if (to - self.prev_endpoint).square_length()
            < self.min_segment_length * self.min_segment_length
        {
            return;
        }

        let unit_tangent = (to - self.prev_endpoint).normalize();
        let n = normal(unit_tangent);
        let mut segments = self.sides.map(|side| {
            let d = n * side.offset;
            Seg::Line(LineSegment {
                from: self.prev_endpoint + d,
                to: to + d,
            })
        });

        self.step(&mut segments, unit_tangent, self.simplify_inner_joins, outputs);
        self.prev_tangent = unit_tangent;
        self.prev_endpoint = to;
    }

    pub fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point, outputs: &mut [&mut dyn PathBuilder; N]) {
        let (displaced_ctrl, displaced_to) = self.displace_quadratic_bezier(ctrl, to);
        let curve = QuadraticBezierSegment {
            from: self.prev_endpoint,
            ctrl: displaced_ctrl,
            to: displaced_to,
        };
        let num_steps = self.num_steps;
        self.add_quadratic_bezier(&curve, outputs);
        if self.num_steps != num_steps {
            // See cubic_bezier_to.
            self.prev_endpoint = curve.to;
        }
        self.position = to;
    }

    pub fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point, outputs: &mut [&mut dyn PathBuilder; N]) {
        let (ctrl1, ctrl2, to) = self.displace_cubic_bezier(ctrl1, ctrl2, to);
        let curve = CubicBezierSegment {
            from: self.prev_endpoint,
            ctrl1,
            ctrl2,
            to,
        };
        // The quadratic bézier curves that approximate the cubic bézier curve
        // start on the displaced curve.
        self.position = curve.from;
        let num_steps = self.num_steps;
        self.add_cubic_bezier(&curve, outputs);

        // If the curve added segments to the outputs, it ends exactly at `to`,
        // even where its last part does not: split_range has rounding errors, the
        // last part can stop before a cusp, or be discarded for being too short.
        // This avoids tiny closing segments with arbitrary directions.
        if self.num_steps != num_steps {
            self.prev_endpoint = to;
        }
        self.position = to;
    }

    /// Length below which line segments are discarded, outside of the special
    /// cases that reduce it.
    ///
    /// The following curve is displaced by up to this distance to start where
    /// the output stops (see `displace_cubic_bezier`), on top of the error of the
    /// offset curves, which can reach the tolerance. Half of the tolerance keeps
    /// the sum of both reasonably close to the tolerance.
    fn default_min_segment_length(&self) -> f32 {
        self.tolerance * 0.25
    }

    /// Control point and endpoint of a quadratic bézier curve that starts at
    /// the current position, displaced so that the curve starts at
    /// `prev_endpoint`.
    ///
    /// The control point moves with the start of the curve, weighted by how
    /// close it is to the start along the largest axis of the curve's extent
    /// between its endpoints. This keeps the tangent at the end the control
    /// point is closest to. If the curve ends where it starts, the whole curve
    /// moves, and the next segment absorbs the displacement.
    fn displace_quadratic_bezier(&self, ctrl: Point, to: Point) -> (Point, Point) {
        let from = self.position;
        let delta = self.prev_endpoint - from;
        if delta == Vector::zero() {
            return (ctrl, to);
        }
        if to == from {
            return (ctrl + delta, to + delta);
        }

        let v = to - from;
        let weight = if v.x.abs() >= v.y.abs() {
            (to.x - ctrl.x) / v.x
        } else {
            (to.y - ctrl.y) / v.y
        };

        (ctrl + delta * weight.max(0.0).min(1.0), to)
    }

    /// Control points and endpoint of a cubic bézier curve that starts at the
    /// current position, displaced so that the curve starts at `prev_endpoint`.
    ///
    /// The first control point moves with the start of the curve, so the
    /// tangents at both ends are unchanged and the curve moves by at most the
    /// displacement.
    fn displace_cubic_bezier(&self, ctrl1: Point, ctrl2: Point, to: Point) -> (Point, Point, Point) {
        let delta = self.prev_endpoint - self.position;
        (ctrl1 + delta, ctrl2, to)
    }

    fn add_quadratic_bezier(&mut self, curve: &QuadraticBezierSegment<f32>, outputs: &mut [&mut dyn PathBuilder; N]) {
        match classify_quadratic_bezier(curve, self.tolerance) {
            QuadType::Linear => {
                self.line_to(curve.to, outputs);
                return;
            }
            QuadType::Degenerate(t) => {
                self.line_to(curve.sample(t), outputs);
                self.line_to(curve.to, outputs);
                return;
            }
            QuadType::Curve => {}
        }

        let offsets = self.sides.map(|side| side.offset);
        let mut first = true;
        for_each_offsets(curve, offsets, self.tolerance, &mut |sub_curves| {
            let mut segments = sub_curves.map(Seg::Quadratic);
            if first {
                first = false;
                let start_tangent = (curve.ctrl - curve.from).normalize();
                self.step(&mut segments, start_tangent, self.simplify_inner_joins, outputs);
            } else {
                self.step_without_join(&segments, outputs)
            }
        });
        self.prev_tangent = (curve.to - curve.ctrl).normalize();
        self.prev_endpoint = curve.to;
    }

    fn add_cubic_bezier(&mut self, curve: &CubicBezierSegment<f32>, outputs: &mut [&mut dyn PathBuilder; N]) {
        if cubic_bezier_is_linear(curve, self.tolerance) {
            for t in cubic_bezier_turnarounds(curve) {
                self.line_to(curve.sample(t), outputs);
            }
            self.line_to(curve.to, outputs);
            return;
        }

        // Leave a small gap around the cusps, where the tangent is undefined. The
        // sub-curves on each side are connected with a join.
        let cusps = find_cubic_bezier_cusps(curve, self.tolerance);
        let mut t0 = 0.0;
        let eps = 0.0001;
        for cusp in cusps {
            if cusp - eps > t0 {
                let sub_curve = curve.split_range(t0..(cusp - eps));
                sub_curve.for_each_quadratic_bezier(self.tolerance, &mut |quad| {
                    self.quadratic_bezier_to(quad.ctrl, quad.to, outputs);
                });
            }
            t0 = cusp + eps;
        }

        if t0 < 1.0 {
            let sub_curve = curve.split_range(t0..1.0);
            sub_curve.for_each_quadratic_bezier(self.tolerance, &mut |quad| {
                self.quadratic_bezier_to(quad.ctrl, quad.to, outputs)
            });
        }
    }

    fn step_without_join(&mut self, segments: &[Seg; N], outputs: &mut [&mut dyn PathBuilder; N]) {
        debug_assert!(!self.is_first_segment);
        self.num_steps += 1;
        for ((side, segment), output) in self.sides.iter_mut().zip(segments).zip(outputs.iter_mut()) {
            add_segment(&side.prev, *output);
            side.prev = *segment;
        }
    }

    fn step(
        &mut self,
        segments: &mut [Seg; N],
        unit_tangent: Vector,
        simplify_inner_joins: bool,
        outputs: &mut [&mut dyn PathBuilder; N],
    ) {
        self.num_steps += 1;

        if self.is_first_segment {
            self.first_tangent = unit_tangent;
            self.is_first_segment = false;
            for ((side, segment), output) in self.sides.iter_mut().zip(segments).zip(outputs.iter_mut()) {
                output.begin(segment.from(), NO_ATTRIBUTES);
                side.prev = *segment;
            }
            return;
        }

        // Whether a join is inner or outer only depends on the sign of the
        // offset of each side.
        let t01 = unit_tangent - self.prev_tangent;
        let n01 = normal(unit_tangent) + normal(self.prev_tangent);
        let turn = t01.dot(n01);
        // When the next segment goes back along the previous one, the offset
        // segments end on opposite sides of the stroke on both sides, so both
        // sides get an outer join to wrap around the tip.
        let reversal = self.prev_tangent.dot(unit_tangent) < 0.0
            && self.prev_tangent.cross(unit_tangent).abs() < 0.005;

        for i in 0..N {
            let output = &mut *outputs[i];
            let segment = &mut segments[i];
            let side = &mut self.sides[i];
            let no_join =
                (side.prev.to() - segment.from()).square_length() < self.tolerance * self.tolerance;
            // We are a bit conservative about inner joins when the dot product is close to zero.
            let inner_join = !reversal && turn * side.offset.signum() > -0.01;

            if no_join {
                add_segment(&side.prev, output);
            } else if inner_join {
                Self::inner_join(side, segment, simplify_inner_joins, output);
            } else {
                // If the previous segment is a line, the join adds its end.
                if let Seg::Quadratic(curve) = side.prev {
                    output.quadratic_bezier_to(curve.ctrl, curve.to, NO_ATTRIBUTES);
                }
                (self.join)(
                    output,
                    self.prev_endpoint,
                    self.prev_tangent,
                    unit_tangent,
                    side.offset,
                    self.miter_limit,
                    self.tolerance,
                    side.prev.is_line(),
                    segment.is_line(),
                );
            }

            side.prev = *segment;
        }
    }

    // Inner joins are expected to be inside the shape so we simplify them into a
    // single line segment and even try to prevent the folding that happens where
    // the two offset segments intersect in the simple cases.
    fn inner_join(side: &mut Side, next: &mut Seg, simplify: bool, output: &mut dyn PathBuilder) {
        // If the two segments are lines unfold the join.
        let mut simplified = false;
        if simplify {
            if let (Seg::Line(l0), Seg::Line(l1)) = (&mut side.prev, &mut *next) {
                if let Some(intersection) = l0.intersection(l1) {
                    l0.to = intersection;
                    l1.from = intersection;
                    simplified = true;
                }
            }
        }

        add_segment(&side.prev, output);

        if !simplified {
            output.line_to(next.from(), NO_ATTRIBUTES);
        }
    }
}

fn add_segment(segment: &Seg, output: &mut dyn PathBuilder) {
    match segment {
        Seg::Line(line) => {
            output.line_to(line.to, NO_ATTRIBUTES);
        }
        Seg::Quadratic(curve) => {
            output.quadratic_bezier_to(curve.ctrl, curve.to, NO_ATTRIBUTES);
        }
    }
}

pub(crate) type JoinBuilder = fn(
    output: &mut dyn PathBuilder,
    position: Point,
    start_tangent: Vector,
    end_tangent: Vector,
    width: f32,
    miter_limit: f32,
    tolerance: f32,
    prev_is_line: bool,
    next_is_line: bool,
);

fn bevel_join(
    output: &mut dyn PathBuilder,
    position: Point,
    start_tangent: Vector,
    end_tangent: Vector,
    width: f32,
    _miter_limit: f32,
    _tolerance: f32,
    prev_is_line: bool,
    _next_is_line: bool,
) {
    if prev_is_line {
        let start = position + normal(start_tangent) * width;
        output.line_to(start, NO_ATTRIBUTES);
    }
    let end = position + normal(end_tangent) * width;
    output.line_to(end, NO_ATTRIBUTES);
}

fn try_miter_join(
    output: &mut dyn PathBuilder,
    normal: Vector,
    position: Point,
    end_offset: Vector,
    width: f32,
    miter_limit: f32,
    next_is_line: bool,
) -> bool {
    if miter_limit_is_exceeded(normal, miter_limit) {
        return false;
    }

    output.line_to(position + normal * width, NO_ATTRIBUTES);
    if !next_is_line {
        output.line_to(position + end_offset, NO_ATTRIBUTES);
    }

    true
}

fn miter_join(
    output: &mut dyn PathBuilder,
    position: Point,
    start_tangent: Vector,
    end_tangent: Vector,
    width: f32,
    miter_limit: f32,
    _tolerance: f32,
    prev_is_line: bool,
    next_is_line: bool,
) {
    if prev_is_line {
        let start = position + normal(start_tangent) * width;
        output.line_to(start, NO_ATTRIBUTES);
    }

    let end_offset = normal(end_tangent) * width;
    let normal = compute_normal(start_tangent, end_tangent);

    if try_miter_join(
        output,
        normal,
        position,
        end_offset,
        width,
        miter_limit,
        next_is_line,
    ) {
        return;
    }

    output.line_to(position + end_offset, NO_ATTRIBUTES);
}

fn miter_clip_join(
    output: &mut dyn PathBuilder,
    position: Point,
    start_tangent: Vector,
    end_tangent: Vector,
    width: f32,
    miter_limit: f32,
    _tolerance: f32,
    prev_is_line: bool,
    next_is_line: bool,
) {
    let start_offset = normal(start_tangent) * width;
    let end_offset = normal(end_tangent) * width;
    let normal = compute_normal(start_tangent, end_tangent);

    if prev_is_line {
        output.line_to(position + start_offset, NO_ATTRIBUTES);
    }

    if try_miter_join(
        output,
        normal,
        position,
        end_offset,
        width,
        miter_limit,
        next_is_line,
    ) {
        return;
    }

    // A zero normal means that the segments go in (nearly) opposite directions,
    // in which case the miter points forward, along the bisector.
    let miter_direction = if normal == Vector::zero() {
        start_tangent - end_tangent
    } else {
        normal * width.signum()
    };

    let (v0, v1) = get_clip_intersections(
        start_offset,
        end_offset,
        miter_direction,
        // Half of the miter length limit (miter_limit * stroke_width).
        miter_limit * width.abs(),
    );

    output.line_to(position + v0, NO_ATTRIBUTES);
    output.line_to(position + v1, NO_ATTRIBUTES);
    if !next_is_line {
        output.line_to(position + end_offset, NO_ATTRIBUTES);
    }
}

fn cubic_join(
    output: &mut dyn PathBuilder,
    position: Point,
    start_tangent: Vector,
    end_tangent: Vector,
    width: f32,
    _miter_limit: f32,
    _tolerance: f32,
    prev_is_line: bool,
    _next_is_line: bool,
) {
    let sign = start_tangent.cross(end_tangent).signum();
    let start_normal = normal(start_tangent);
    let end_normal = normal(end_tangent);
    let start = position + start_normal * width;
    let end = position + end_normal * width;
    if prev_is_line {
        output.line_to(start, NO_ATTRIBUTES);
    }

    let angle = (start_normal * sign)
        .angle_to(end_normal * sign)
        .radians
        .abs();

    // From https://pomax.github.io/bezierinfo/#circles
    let k = width.abs() * 4.0 / 3.0 * (angle * 0.25).tan();

    let ctrl1 = start + start_tangent * k;
    let ctrl2 = end - end_tangent * k;

    output.cubic_bezier_to(ctrl1, ctrl2, end, NO_ATTRIBUTES);
}

fn round_join(
    output: &mut dyn PathBuilder,
    position: Point,
    start_tangent: Vector,
    end_tangent: Vector,
    width: f32,
    miter_limit: f32,
    tolerance: f32,
    prev_is_line: bool,
    next_is_line: bool,
) {
    cubic_join(
        output,
        position,
        start_tangent,
        end_tangent,
        width,
        miter_limit,
        tolerance,
        prev_is_line,
        next_is_line,
    )
}

/// Offsets a curve by up to two distances at once.
///
/// The offset curves are approximated over the same parameter ranges, so
/// that the analysis of the curve is shared: the curve is split at the
/// critical points of every distance, and a range is split further until
/// the offset sub-curves of every distance are within the tolerance. The
/// callback receives the offset sub-curves of each range, in the order of
/// `dists`.
///
/// The sub-curves are contiguous in the parameter of the curve but are not
/// necessarily connected in space. In particular, the curve is split at its
/// cusps, where the tangent reverses and the offset jumps from one side of
/// the cusp to the other; the resulting gap is not filled. Joining
/// consecutive sub-curves is left to the caller, which is what lets the
/// stroker choose a join style there (a round join adds the arc around the
/// cusp).
pub(crate) fn for_each_offsets<S: Scalar, F, const N: usize>(
    curve: &QuadraticBezierSegment<S>,
    dists: [S; N],
    tolerance: S,
    cb: &mut F,
) where
    F: FnMut(&[QuadraticBezierSegment<S>; N]),
{
    const { assert!(N >= 1 && N <= 2) };

    let tolerance = tolerance.max(S::value(0.001));

    let mut splits: ArrayVec<S, 7> = ArrayVec::new();
    for (i, dist) in dists.iter().enumerate() {
        // The critical points only depend on the magnitude of the distance.
        if dists[..i].iter().any(|d| d.abs() == dist.abs()) {
            continue;
        }
        let (t1, t2) = critical_points(curve, *dist);
        splits.extend(t1);
        splits.extend(t2);
    }
    // A tangent reversal (a cusp) turns by 180 degrees, so turn_splits places
    // a split at it (within rounding). The critical points above do it as
    // well for the cusps of collinear curves, but they may discard a point
    // close to an end of the curve.
    splits.extend(turn_splits(curve));
    splits.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mut t0 = S::ZERO;
    for t1 in splits.into_iter().chain(Some(S::ONE)) {
        if t1 > t0 {
            for_each_offset_inner(curve, t0..t1, dists, tolerance, cb);
            t0 = t1;
        }
    }
}

/// Parameters, in increasing order, at which a cubic bézier curve that lies on
/// a line changes direction.
pub(crate) fn cubic_bezier_turnarounds(curve: &CubicBezierSegment<f32>) -> ArrayVec<f32, 3> {
    let mut turnarounds = ArrayVec::new();

    let direction = cubic_bezier_direction(curve);

    // The derivative is a quadratic bézier curve with control points 3 times
    // the control vectors of the curve. If their projections on the line all
    // have the same sign, so does the projection of the derivative, and the
    // curve doesn't change direction. This is the common case, for example
    // with lines written as cubic bézier curves.
    let p0 = (curve.ctrl1 - curve.from).dot(direction);
    let p1 = (curve.ctrl2 - curve.ctrl1).dot(direction);
    let p2 = (curve.to - curve.ctrl2).dot(direction);
    if (p0 >= 0.0 && p1 >= 0.0 && p2 >= 0.0) || (p0 <= 0.0 && p1 <= 0.0 && p2 <= 0.0) {
        return turnarounds;
    }

    // The derivative is 3 (a + 2 b t + c t²). Find where its projection on the
    // line vanishes.
    let a = curve.ctrl1 - curve.from;
    let b = curve.ctrl2.to_vector() - curve.ctrl1.to_vector() * 2.0 + curve.from.to_vector();
    let c = curve.to.to_vector() + (curve.ctrl1 - curve.ctrl2) * 3.0 - curve.from.to_vector();
    let roots = cubic_polynomial_roots(0.0, c.dot(direction), 2.0 * b.dot(direction), a.dot(direction));
    for t in roots {
        if t > 0.0 && t < 1.0 {
            turnarounds.push(t);
        }
    }
    turnarounds.sort_by(|a, b| a.total_cmp(b));

    turnarounds
}

/// Parameters, in increasing order, at which a cubic bézier curve has a cusp,
/// or turns so tightly that it is indistinguishable from one at this tolerance.
pub(crate) fn find_cubic_bezier_cusps(curve: &CubicBezierSegment<f32>, tolerance: f32) -> ArrayVec<f32, 3> {
    let mut cusps = ArrayVec::new();

    // F' = 3 (a + 2 b t + c t²) and F'' = 6 (b + c t).
    let a = curve.ctrl1 - curve.from;
    let b = curve.ctrl2.to_vector() - curve.ctrl1.to_vector() * 2.0 + curve.from.to_vector();
    let c = curve.to.to_vector() + (curve.ctrl1 - curve.ctrl2) * 3.0 - curve.from.to_vector();

    // Rule out cusps without solving the cubic polynomial below, which is
    // comparatively expensive. F' is a quadratic bézier curve with control
    // points h0, h1 and h2, and F'' = 2 ((1 - t) (h1 - h0) + t (h2 - h1)). If
    // the control points of F' are all on the positive side of a direction u,
    // so is F', and |F'| >= min(h.u) / |u|. |F''| is at most twice the longest
    // of h1 - h0 and h2 - h1. Together, they bound the radius of curvature at
    // the extrema of the speed from below.
    let h0 = curve.ctrl1 - curve.from;
    let h1 = curve.ctrl2 - curve.ctrl1;
    let h2 = curve.to - curve.ctrl2;
    let u = curve.to - curve.from;
    let min_speed = h0.dot(u).min(h1.dot(u)).min(h2.dot(u));
    if min_speed > 0.0 {
        let max_d2 = (h1 - h0).square_length().max((h2 - h1).square_length()).sqrt() * 6.0;
        // Both sides are multiplied by |u|² and divided by 9.
        if min_speed * min_speed > tolerance * max_d2 * u.square_length() / 9.0 {
            return cusps;
        }
    }

    // The speed |F'| has its extrema where F'.F'' = 0, and cusps are among them.
    // F'.F'' / 18 = c.c t³ + 3 b.c t² + (2 b.b + a.c) t + a.b
    let p3 = c.dot(c);
    let p2 = 3.0 * b.dot(c);
    let p1 = 2.0 * b.dot(b) + a.dot(c);
    let p0 = a.dot(b);
    // When a control point is on an endpoint, the speed vanishes there and the
    // polynomial has a root at that end. Factoring it out leaves a quadratic
    // polynomial, which is cheaper to solve. It is a common case in SVG files.
    let roots = if h0 == Vector::zero() {
        cubic_polynomial_roots(0.0, p3, p2, p1)
    } else if h2 == Vector::zero() {
        cubic_polynomial_roots(0.0, p3, p3 + p2, p3 + p2 + p1)
    } else {
        cubic_polynomial_roots(p3, p2, p1, p0)
    };
    for t in roots {
        if !(t > 0.0 && t < 1.0) {
            continue;
        }

        // F' and F'' are orthogonal at the extrema of the speed, so the radius
        // of curvature there is |F'|² / |F''|.
        let d1 = (a + b * (2.0 * t) + c * (t * t)) * 3.0;
        let d2 = (b + c * t) * 6.0;
        if d1.square_length() <= tolerance * d2.length() {
            cusps.push(t);
        }
    }

    cusps.sort_by(|a, b| a.total_cmp(b));

    cusps
}

fn normal(v: Vector) -> Vector {
    Vector::new(-v.y, v.x)
}

fn normal_s<S: Scalar>(v: crate::geom::Vector<S>) -> crate::geom::Vector<S> {
    crate::geom::Vector::new(-v.y, v.x)
}

/// Direction of the line that a cubic bézier curve lies on, if it does. The
/// longest vector from the start to another point still gives it when the
/// endpoints are equal.
fn cubic_bezier_direction(curve: &CubicBezierSegment<f32>) -> Vector {
    let dirs = [
        curve.ctrl1 - curve.from,
        curve.ctrl2 - curve.from,
        curve.to - curve.from,
    ];

    *dirs.iter()
        .max_by(|a, b| a.square_length().total_cmp(&b.square_length()))
        .unwrap()
}

pub(crate) fn cubic_bezier_is_linear(curve: &CubicBezierSegment<f32>, tolerance: f32) -> bool {
    if curve.from != curve.to {
        return curve.is_linear(tolerance);
    }

    // CubicBezierSegment::is_linear measures the distance to the line that goes
    // through the endpoints, which is not defined when they are equal.
    let line = Line {
        point: curve.from,
        vector: cubic_bezier_direction(curve),
    };
    if line.vector == Vector::zero() {
        return true;
    }

    let tolerance2 = tolerance * tolerance;
    line.square_distance_to_point(curve.ctrl1) <= tolerance2
        && line.square_distance_to_point(curve.ctrl2) <= tolerance2
}

/// Maximum angle that the tangent of the curve turns by over each of the
/// ranges passed to for_each_offset_inner.
///
/// for_each_offset_inner places the control point of each offset sub-curve
/// at dist / cos(turn / 2) from the curve, which grows without bound as the
/// turn gets close to 180 degrees. Bounding the turn also bounds the error of
/// the sub-curves that are accepted without meeting the tolerance (see
/// min_step): a quadratic bézier curve approximates a 45 degree arc within
/// 0.3% of its radius.
pub(crate) const MAX_TURN: f32 = core::f32::consts::FRAC_PI_4;

/// Parameters that split the curve into parts that turn by at most MAX_TURN.
fn turn_splits<S: Scalar>(curve: &QuadraticBezierSegment<S>) -> ArrayVec<S, 3> {
    let mut splits = ArrayVec::new();

    // The derivative is 2 (p + t q).
    let p = curve.ctrl - curve.from;
    let q = curve.from.to_vector() - curve.ctrl.to_vector() * S::TWO + curve.to.to_vector();
    let p_end = p + q;
    // Less than 180 degrees.
    let turn = S::atan2(p.cross(p_end), p.dot(p_end));
    let max_turn = S::value(MAX_TURN);
    let count = (turn.abs() / max_turn).ceil();
    if !(count > S::ONE) {
        return splits;
    }

    let p_unit = p.normalize();
    let mut i = S::ONE;
    while i < count {
        // Solve cross(p + t q, direction) = 0.
        let angle = turn * i / count;
        let (sin, cos) = euclid::Angle::radians(angle).sin_cos();
        let direction = p_unit * cos + normal_s(p_unit) * sin;
        let t = -p.cross(direction) / q.cross(direction);
        if t > S::ZERO && t < S::ONE {
            splits.push(t);
        }
        i = i + S::ONE;
    }

    splits
}

pub(crate) fn for_each_offset_inner<S: Scalar, F, const N: usize>(
    curve: &QuadraticBezierSegment<S>,
    range: Range<S>,
    dists: [S; N],
    tolerance: S,
    cb: &mut F,
) where
    F: FnMut(&[QuadraticBezierSegment<S>; N]),
{
    debug_assert!(curve.from.is_finite());
    debug_assert!(curve.ctrl.is_finite());
    debug_assert!(curve.to.is_finite());

    // Sub-curves are accepted regardless of the error below this parameter
    // range, or when the range can't be halved within the precision of the
    // parameters, to guarantee termination. This is reached near the cusps
    // of the offset curve, or when the tolerance is too small for the
    // precision of the coordinates.
    let min_step = (range.end - range.start) * S::value(0.05);

    let end = range.end;
    let mut range = range;

    // Tangents are computed from the derivative of the curve rather than the
    // control points of the sub-curves, which lose precision as the
    // sub-curves get small.
    let tangent = |t: S| curve.derivative(t).normalize();

    // Distance between a point and the offset curve, estimated from the
    // closest point of the curve. It is found with Newton iterations on the
    // derivative of the squared distance, starting at parameter t and
    // staying within t_min..t_max.
    let second_derivative = (curve.from.to_vector() - curve.ctrl.to_vector() * S::TWO
        + curve.to.to_vector())
        * S::TWO;

    // Unit tangent at an end of the range. The derivative vanishes at the
    // turning point of degenerate curves, where the tangent is only defined
    // as a one-sided limit: just inside the range B'(t + h) is proportional
    // to h B''(t), so the direction is that of B'' towards increasing t and
    // its opposite towards decreasing t (`toward`).
    let boundary_tangent = |t: S, toward: S| {
        let d = curve.derivative(t);
        if d.square_length() > S::ZERO {
            d.normalize()
        } else {
            let d2 = second_derivative * toward;
            if d2.square_length() > S::ZERO {
                d2.normalize()
            } else {
                crate::geom::Vector::zero()
            }
        }
    };
    let offset_point = |t: S, dist: S, toward: S| {
        curve.sample(t) + normal_s(boundary_tangent(t, toward)) * dist
    };
    let offset_error = |p: crate::geom::Point<S>, dist: S, mut t: S, t_min: S, t_max: S| {
        for _ in 0..2 {
            let v = curve.sample(t) - p;
            let d1 = curve.derivative(t);
            let g = v.dot(d1);
            let dg = d1.square_length() + v.dot(second_derivative);
            // Not a minimum of the distance, which is the case on the
            // part of the offset curve that goes backwards between its
            // cusps.
            if !(dg > S::ZERO) {
                break;
            }
            t = (t - g / dg).max(t_min).min(t_max);
        }
        ((p - curve.sample(t)).dot(normal_s(tangent(t))) - dist).abs()
    };

    let mut v0 = boundary_tangent(range.start, S::ONE);
    let mut from = dists.map(|dist| offset_point(range.start, dist, S::ONE));

    while range.start < end {
        let sub_curve = curve.split_range(range.clone());

        // Offset the sub-curve
        let v1 = boundary_tangent(range.end, -S::ONE);
        let n1 = normal_s(v1);
        let n = normal_s((v0 + v1).normalize());
        // At least cos(MAX_TURN / 2).
        let inv_len = n.dot(n1);
        if !(inv_len > S::ZERO) || !from.iter().all(|p| p.is_finite()) {
            // The range spans a cusp of the curve, where the tangent turns
            // by 180 degrees (or the curve is degenerate). The offset is
            // discontinuous there, so no sub-curve is emitted for the range:
            // the caller joins the sub-curves on each side of the cusp.
            return;
        }
        let offset_curves: [QuadraticBezierSegment<S>; N] =
            core::array::from_fn(|i| QuadraticBezierSegment {
                from: from[i],
                ctrl: sub_curve.ctrl + n * (dists[i] / inv_len),
                to: sub_curve.to + n1 * dists[i],
            });

        // Evaluate the error. The sub-curve matches the offset curve's
        // endpoints and tangents, so the error vanishes at both ends and
        // peaks in between: near the middle, or up to about a quarter of the
        // way from one end when the curvature changes a lot over the range.
        // The peak is then up to about 1.5 times the error at the middle.
        // The error is almost always far below the tolerance, so the other
        // samples are only measured when the middle one is not.
        //
        // The error is first bounded without measuring it (see
        // offset_error_bound), which is enough for most ranges.
        let step = range.end - range.start;
        let mut error_bound = S::ZERO;
        for dist in dists {
            error_bound = error_bound.max(offset_error_bound(&sub_curve, inv_len, dist));
        }
        let mut err = S::ZERO;
        let half = range.start + step * S::HALF;
        let can_split = step > min_step && half > range.start && half < range.end;
        if can_split && !(error_bound <= tolerance) {
            for (offset_curve, dist) in offset_curves.iter().zip(dists) {
                let error_at = |s: S| {
                    let t = range.start + step * s;
                    offset_error(offset_curve.sample(s), dist, t, range.start, range.end)
                };
                let mut e = error_at(S::HALF);
                if e > tolerance * S::HALF {
                    e = e.max(error_at(S::value(0.25))).max(error_at(S::value(0.75)));
                }
                err = err.max(e);
            }
        }

        if err <= tolerance {
            cb(&offset_curves);
            range = range.end..end;
            from = offset_curves.map(|c| c.to);
            v0 = v1;
        } else {
            range.end = range.start + step * S::HALF;
        }
    }
}

/// Upper bound of the distance between the offset sub-curve that
/// for_each_offset_inner builds for `curve` and the exact offset curve.
/// `cos_half_turn` is the cosine of half of the angle θ between the
/// tangents at the ends of the curve, which must be less than 180 degrees.
///
/// With a and b the control vectors of the curve, its derivative is
/// proportional to w(t) = (1 - t) a + t b, and the exact offset curve is
/// O(t) = B(t) + dist N(t) with N(t) the unit normal along w(t). The offset
/// sub-curve is Q(t) = B(t) + dist L(t), with L the quadratic bézier curve
/// with control points N(0), n / cos(θ/2) and N(1), where n is the normal of
/// the bisector of the tangents. L(t) is between 1 and
/// r = (cos(θ/2) + 1 / cos(θ/2)) / 2 away from the origin, and at an angle φ
/// from N(t). So Q(t) is O(t) moved by at most dist (r - 1 + φ² / 2) along
/// N(t) and dist r φ along the tangent of O. Moving along the tangent of a
/// curve by δ moves away from the curve by at most k δ² / 2, with k the
/// curvature of the curve.
///
/// L(t) and N(t) are both positive combinations of the normals at the ends,
/// with ratios between the weights of N(1) and N(0) of u(t) and v(t). The
/// angle of such a combination changes by at most tan(θ/2) / 2 per unit of
/// ln(ratio), so φ is at most tan(θ/2) / 2 |ln(u / v)|. Since
/// u / v = (|a| / |b|) g(t) with g(t) between cos²(θ/2) and 1 / cos²(θ/2),
/// |ln(u / v)| is at most |ln(|b| / |a|)| - 2 ln(cos(θ/2)). Both logarithms
/// are bounded with |ln(x)| <= |x - 1 / x| / 2.
///
/// The curvature of the curve is |a x b| / (2 |w(t)|³), and the curvature of
/// the offset curve is k / (1 + dist k) on the convex side and
/// k / (1 - dist k) on the concave side. On the concave side, the bound is
/// infinite if dist k reaches 1 in the range, where the offset curve can
/// have a cusp.
pub(crate) fn offset_error_bound<S: Scalar>(
    curve: &QuadraticBezierSegment<S>,
    cos_half_turn: S,
    dist: S,
) -> S {
    let c = cos_half_turn;
    let a = curve.ctrl - curve.from;
    let b = curve.to - curve.ctrl;
    let a2 = a.square_length();
    let b2 = b.square_length();
    // N is on the left of the curve, which is the concave side if the curve
    // turns left.
    let cross = a.cross(b);
    let concave = (dist > S::ZERO) == (cross > S::ZERO);
    let dist = dist.abs();

    let max_radius = (c + S::ONE / c) * S::HALF;
    let tan_half_turn = (S::ONE - c * c).max(S::ZERO).sqrt() / c;
    let ln_ratio = (b2 - a2).abs() / (S::TWO * (a2 * b2).sqrt());
    let ln_cos = (S::ONE / c - c) * S::HALF;
    let angle = tan_half_turn * S::HALF * (ln_ratio + S::TWO * ln_cos);

    // The curvature is the largest where w(t) is the shortest, which is the
    // closest point to the origin on the segment from a to b.
    let ab = b - a;
    let t = (-a.dot(ab) / ab.square_length()).max(S::ZERO).min(S::ONE);
    let min_w = (a + ab * t).length();
    let k = cross.abs() / (S::TWO * min_w * min_w * min_w);
    let offset_k = if concave {
        if !(dist * k < S::ONE) {
            return S::infinity();
        }
        k / (S::ONE - dist * k)
    } else {
        k / (S::ONE + dist * k)
    };

    let tangential = dist * max_radius * angle;
    dist * (max_radius - S::ONE + angle * angle * S::HALF)
        + offset_k * tangential * tangential * S::HALF
}

pub(crate) fn critical_points<S: Scalar>(
    curve: &QuadraticBezierSegment<S>,
    offset: S,
) -> (Option<S>, Option<S>) {
    // See https://blend2d.com/research/precise_offset_curves.pdf
    //
    // The offset curve has a cusp where the radius of curvature of the curve
    // is equal to the offset distance d. With B'(t) = 2At + B and B'' = 2A,
    // the radius of curvature is |B'|³ / |B' x B''| = |B'|³ / (2 |A x B|),
    // so these points are the solutions of |B'(t)|² = cbrt(4 d² (A x B)²):
    //
    // 4 |A|² t² + 4 (A.B) t + |B|² - cbrt(4 d² (A x B)²) = 0
    //
    // Only the solutions on the concave side are cusps of the offset curve.
    // Splitting at the others is harmless.
    let from = curve.from.to_vector();
    let ctrl = curve.ctrl.to_vector();
    let to = curve.to.to_vector();
    let a = from - ctrl * S::TWO + to;
    let b = (ctrl - from) * S::TWO;

    let a_dot_b = a.dot(b);
    let a_len2 = a.square_length();
    let a_cross_b = a.cross(b);

    let r = (S::FOUR * offset * offset * a_cross_b * a_cross_b).cbrt();
    // NaN when there is no solution, in which case both comparisons below fail.
    let sqrt_delta = (a_dot_b * a_dot_b - a_len2 * (b.square_length() - r)).sqrt();

    let t1 = (-a_dot_b - sqrt_delta) / (S::TWO * a_len2);
    let t2 = (-a_dot_b + sqrt_delta) / (S::TWO * a_len2);

    // We don't want to split very close to the endpoints.
    let eps = S::value(0.05);

    let t1 = if t1 > S::ZERO + eps && t1 < S::ONE - eps {
        Some(t1)
    } else {
        None
    };
    let t2 = if t2 > S::ZERO + eps && t2 < S::ONE - eps {
        Some(t2)
    } else {
        None
    };

    (t1, t2)
}

/// Compute a normal vector at a point P such that ```x ---e1----> P ---e2---> x```
///
/// The resulting vector is not normalized. The length is such that extruding the shape
/// would yield parallel segments exactly 1 unit away from their original. (useful
/// for generating strokes and vertex-aa).
/// The normal points towards the positive side of e1.
/// Returns a zero vector if v1 and v2 point in (nearly) opposite directions.
///
/// v1 and v2 are expected to be normalized.
pub(crate) fn compute_normal(v1: Vector, v2: Vector) -> Vector {
    let epsilon = 1e-4;

    let n1 = Vector::new(-v1.y, v1.x);

    let v12 = v1 + v2;

    // !(a < b) instead of a <= b to catch NaN situations.
    if !(v12.square_length() > epsilon) {
        return Vector::new(0.0, 0.0);
    }

    let tangent = v12.normalize();
    let n = Vector::new(-tangent.y, tangent.x);

    let inv_len = n.dot(n1);

    if inv_len.abs() < epsilon {
        return n1;
    }

    n / inv_len
}

pub(crate) fn get_clip_intersections(
    start_offset: Vector,
    end_offset: Vector,
    normal: Vector,
    clip_distance: f32,
) -> (Vector, Vector) {
    // The clip line is the set of points p such that u.dot(p) == clip_distance.
    let u = normal.normalize();

    // Intersect the clip line with the offset edge o + t * tangent(o).
    let intersect = |o: Vector| {
        let d = tangent(o);
        let distance_to_clip = clip_distance - u.dot(o);
        let denom = u.dot(d);
        // |denom| is |o| * sin(angle between the edge and the clip line), so the
        // test is relative to the offset length and does not depend on the scale.
        // A (nearly) parallel edge has no intersection, fall back to the closest
        // point of the clip line.
        if denom.abs() <= 1e-4 * o.length() {
            o + u * distance_to_clip
        } else {
            o + d * (distance_to_clip / denom)
        }
    };

    (intersect(start_offset), intersect(end_offset))
}

// The miter length is the distance between the inner corner and the tip of the
// miter, which are both normal.length() * half_width away from the join point.
// miter_limit = miter_length / stroke_width
// miter_limit = (2.0 * normal.length() * half_width) / (2.0 * half_width)
//
// compute_normal returns a zero vector when the miter length is infinite.
fn miter_limit_is_exceeded(normal: Vector, miter_limit: f32) -> bool {
    normal == Vector::zero() || normal.square_length() > miter_limit * miter_limit
}

enum QuadType {
    Curve,
    Linear,
    /// The curve lies on a line and changes direction at the given parameter.
    Degenerate(f32),
}

fn classify_quadratic_bezier(curve: &QuadraticBezierSegment<f32>, tolerance: f32) -> QuadType {
    if !curve.is_linear(tolerance) {
        return QuadType::Curve;
    }

    // The derivative is 2 (p + t q). On a line, it vanishes where the curve
    // changes direction. This is where it is the shortest, which also gives the
    // turning point of curves that are only close to a line.
    let p = curve.ctrl - curve.from;
    let q = curve.from.to_vector() - curve.ctrl.to_vector() * 2.0 + curve.to.to_vector();
    // NaN if q is zero, which means that the curve goes straight from start to end.
    let t = -p.dot(q) / q.square_length();
    if t > 0.0 && t < 1.0 {
        return QuadType::Degenerate(t);
    }

    QuadType::Linear
}

#[derive(Copy, Clone, Debug)]
enum Seg {
    Line(LineSegment<f32>),
    Quadratic(QuadraticBezierSegment<f32>),
}

impl Seg {
    pub fn from(&self) -> Point {
        match self {
            Seg::Line(line) => line.from,
            Seg::Quadratic(quad) => quad.from,
        }
    }

    pub fn to(&self) -> Point {
        match self {
            Seg::Line(line) => line.to,
            Seg::Quadratic(quad) => quad.to,
        }
    }

    pub fn is_line(&self) -> bool {
        match self {
            Seg::Line { .. } => true,
            Seg::Quadratic { .. } => false,
        }
    }
}

/// One side of the offset path.
#[derive(Copy, Clone)]
struct Side {
    offset: f32,
    /// The last offset segment. It is added to the output once the join that
    /// follows it is known, since an inner join can shorten it.
    prev: Seg,
}
