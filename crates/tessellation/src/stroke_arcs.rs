//! Internal support geometry for constructing an SVG 2 `arcs` line join.
//!
//! This module deliberately stops before triangulation. `stroke` adapts Lyon's
//! endpoint data into this module and owns the eventual mesh emission.

use core::fmt;
use core::ops::{Add, Mul, Neg, Sub};

mod svg2;
pub(crate) use svg2::construct_svg2;

#[cfg(not(feature = "std"))]
use num_traits::Float;

const PARALLEL_EPSILON: f64 = 1.0e-12;
const GEOMETRY_EPSILON: f64 = 1.0e-12;

/// A point represented with 64-bit coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point64 {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate.
    pub y: f64,
}

impl Point64 {
    /// Creates a point from two coordinates.
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// A two-dimensional vector represented with 64-bit components.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector64 {
    /// Horizontal component.
    pub x: f64,
    /// Vertical component.
    pub y: f64,
}

impl Vector64 {
    /// Creates a vector from two components.
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    pub(crate) fn cross(self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }

    fn left_normal(self) -> Self {
        Self::new(-self.y, self.x)
    }

    fn square_length(self) -> f64 {
        self.dot(self)
    }

    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl Add<Vector64> for Point64 {
    type Output = Self;

    fn add(self, vector: Vector64) -> Self::Output {
        Self::new(self.x + vector.x, self.y + vector.y)
    }
}

impl Sub<Vector64> for Point64 {
    type Output = Self;

    fn sub(self, vector: Vector64) -> Self::Output {
        Self::new(self.x - vector.x, self.y - vector.y)
    }
}

impl Sub<Point64> for Point64 {
    type Output = Vector64;

    fn sub(self, other: Point64) -> Self::Output {
        Vector64::new(self.x - other.x, self.y - other.y)
    }
}

impl Add for Vector64 {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Vector64 {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self::new(self.x - other.x, self.y - other.y)
    }
}

impl Mul<f64> for Vector64 {
    type Output = Self;

    fn mul(self, scale: f64) -> Self::Output {
        Self::new(self.x * scale, self.y * scale)
    }
}

impl Neg for Vector64 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

/// Tangent and signed curvature of a path segment at the join point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SegmentEnd {
    /// Tangent following the path direction. It does not have to be normalized.
    pub tangent: Vector64,
    /// Signed curvature in inverse user units. A line has curvature zero.
    pub curvature: f64,
}

/// Input required to analyze one outer `arcs` join.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JoinInput {
    /// Point shared by the incoming and outgoing path segments.
    pub at: Point64,
    /// Geometry at the end of the incoming segment.
    pub incoming: SegmentEnd,
    /// Geometry at the start of the outgoing segment.
    pub outgoing: SegmentEnd,
    /// Half of the stroke width in user units.
    pub half_width: f64,
    /// Maximum miter length as a multiple of the stroke width.
    pub miter_limit: f64,
}

/// Direction in which the centerline turns at the join.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnSide {
    /// The outgoing tangent lies to the left of the incoming tangent.
    Left,
    /// The outgoing tangent lies to the right of the incoming tangent.
    Right,
}

/// Infinite line or circle extending one outer stroke edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SupportCurve {
    /// A line used when the segment curvature is zero.
    Line {
        /// Point where the offset edge meets the join.
        point: Point64,
        /// Unit direction following the path.
        direction: Vector64,
    },
    /// An osculating circle adjusted by half the stroke width.
    Circle {
        /// Center shared with the centerline osculating circle.
        center: Point64,
        /// Positive radius of the outer stroke edge.
        radius: f64,
    },
}

/// Result of intersecting two support curves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Intersections {
    /// The supports have no common point.
    None,
    /// The supports touch at one point.
    One(Point64),
    /// The supports cross at two points.
    Two([Point64; 2]),
    /// The supports describe the same infinite line or circle.
    Coincident,
}

/// Compact representation of the symmetric roots produced by circle solvers.
///
/// Keeping the midpoint and offset separate lets join construction select the
/// nearer root from one dot product, while diagnostics can still materialize
/// both points through [`Self::all`].
#[derive(Clone, Copy)]
enum SymmetricIntersections {
    None,
    One(Point64),
    Two { midpoint: Point64, offset: Vector64 },
    Coincident,
}

impl SymmetricIntersections {
    #[inline]
    fn all(self) -> Intersections {
        match self {
            Self::None => Intersections::None,
            Self::One(point) => Intersections::One(point),
            Self::Two { midpoint, offset } => {
                Intersections::Two([midpoint - offset, midpoint + offset])
            }
            Self::Coincident => Intersections::Coincident,
        }
    }

    #[inline]
    fn closest_to(self, at: Point64) -> Result<Point64, IntersectionProblem> {
        match self {
            Self::None => Err(IntersectionProblem::Disjoint),
            Self::One(point) => Ok(point),
            Self::Two { midpoint, offset } => {
                if (at - midpoint).dot(offset) > 0.0 {
                    Ok(midpoint + offset)
                } else {
                    Ok(midpoint - offset)
                }
            }
            Self::Coincident => Err(IntersectionProblem::Coincident),
        }
    }
}

/// Diagnostic result for the first stage of `arcs` join construction.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg(test)]
pub struct JoinAnalysis {
    /// Direction of the centerline turn.
    pub turn: TurnSide,
    /// Join point on the outer incoming stroke edge.
    pub incoming_offset_point: Point64,
    /// Join point on the outer outgoing stroke edge.
    pub outgoing_offset_point: Point64,
    /// Extension of the incoming outer stroke edge.
    pub incoming_support: SupportCurve,
    /// Extension of the outgoing outer stroke edge.
    pub outgoing_support: SupportCurve,
    /// All intersections of the two infinite supports.
    pub intersections: Intersections,
}

/// One finite edge of the outer `arcs` join boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoundaryPiece {
    /// Straight extension of a zero-curvature offset edge.
    Line {
        /// Offset point where the extension begins.
        start: Point64,
        /// Resolved join vertex where the extension ends.
        end: Point64,
    },
    /// Circular extension of a curved offset edge.
    Arc {
        /// Center of the final support circle.
        center: Point64,
        /// Positive radius of the final support circle.
        radius: f64,
        /// Offset point where the arc begins.
        start: Point64,
        /// Resolved join vertex where the arc ends.
        end: Point64,
        /// Lazily or eagerly resolved signed sweep from `start` to `end`.
        sweep: ArcSweep,
    },
}

/// A signed arc sweep whose zero representation defers the expensive angle.
///
/// Positive zero means counter-clockwise and negative zero means clockwise.
/// A non-zero value contains the already resolved signed sweep in radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArcSweep(f64);

impl ArcSweep {
    pub(crate) fn lazy(counter_clockwise: bool) -> Self {
        Self(if counter_clockwise { 0.0 } else { -0.0 })
    }

    pub(crate) fn resolved(sweep_radians: f64) -> Self {
        Self(sweep_radians)
    }

    pub(crate) fn counter_clockwise(self) -> bool {
        self.0 > 0.0 || (self.0 == 0.0 && self.0.is_sign_positive())
    }

    pub(crate) fn resolve(self, start: Vector64, end: Vector64) -> f64 {
        if self.0 == 0.0 {
            directed_sweep_vectors(start, end, self.counter_clockwise())
        } else {
            self.0
        }
    }

    pub(crate) fn resolved_radians(self) -> Option<f64> {
        (self.0 != 0.0).then_some(self.0)
    }

    pub(crate) fn is_finite(self) -> bool {
        self.0.is_finite()
    }
}

#[cfg(not(test))]
const _: () = assert!(core::mem::size_of::<BoundaryPiece>() <= 72);

/// Edge introduced when the SVG miter limit clips an `arcs` join.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipEdge {
    /// End of the clipped incoming boundary.
    pub incoming: Point64,
    /// End of the clipped outgoing boundary.
    pub outgoing: Point64,
    /// Point at the requested distance along the SVG miter-limit arc.
    pub limit_point: Point64,
    /// Unit direction of the clipping line, perpendicular to that arc.
    pub line_direction: Vector64,
}

/// Fully resolved and miter-clipped support geometry for an `arcs` join.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedArcsJoin {
    /// Direction of the centerline turn.
    turn: TurnSide,
    /// Finite continuation of the incoming offset edge toward the intersection.
    incoming_boundary: BoundaryPiece,
    /// Finite continuation of the outgoing offset edge toward the intersection.
    outgoing_boundary: BoundaryPiece,
    /// Whether a miter-limit edge connects the boundary endpoints.
    clipped: bool,
    /// Join point on the incoming outer stroke edge, retained for diagnostics.
    #[cfg(test)]
    pub incoming_offset_point: Point64,
    /// Join point on the outgoing outer stroke edge, retained for diagnostics.
    #[cfg(test)]
    pub outgoing_offset_point: Point64,
    /// Final incoming support after any radius correction, retained for diagnostics.
    #[cfg(test)]
    pub incoming_support: SupportCurve,
    /// Final outgoing support after any radius correction, retained for diagnostics.
    #[cfg(test)]
    pub outgoing_support: SupportCurve,
    /// Closest intersection of the final supports, retained for diagnostics.
    #[cfg(test)]
    pub intersection: Point64,
    /// Common clipping edge, retained for diagnostics.
    #[cfg(test)]
    pub clip: Option<ClipEdge>,
}

impl ResolvedArcsJoin {
    pub(crate) fn clip_endpoints(&self) -> Option<[Point64; 2]> {
        self.clipped.then(|| {
            [
                boundary_end(self.incoming_boundary),
                boundary_end(self.outgoing_boundary),
            ]
        })
    }

    /// Both directions point from the retained boundary towards the cut.
    pub(crate) fn clip_tangents(&self) -> [Vector64; 2] {
        [
            boundary_end_tangent(self.incoming_boundary),
            boundary_end_tangent(self.outgoing_boundary),
        ]
    }

    pub(crate) fn turn(&self) -> TurnSide {
        self.turn
    }

    pub(crate) fn incoming_boundary(&self) -> BoundaryPiece {
        self.incoming_boundary
    }

    pub(crate) fn outgoing_boundary(&self) -> BoundaryPiece {
        self.outgoing_boundary
    }

    pub(crate) fn incoming_offset_point(&self) -> Point64 {
        boundary_start(self.incoming_boundary)
    }

    pub(crate) fn outgoing_offset_point(&self) -> Point64 {
        boundary_start(self.outgoing_boundary)
    }

    pub(crate) fn is_clipped(&self) -> bool {
        self.clipped
    }
}

#[cfg(not(test))]
const _: () = assert!(core::mem::size_of::<ResolvedArcsJoin>() <= 152);

/// Rectangle required by SVG 2 when opposite parallel tangents cannot be
/// connected by adjusted support circles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParallelRectangleJoin {
    /// Corner on the left of the incoming tangent at the join point.
    pub near_left: Point64,
    /// Corner on the right of the incoming tangent at the join point.
    pub near_right: Point64,
    /// Outer corner reached from `near_left` along the incoming tangent.
    pub far_left: Point64,
    /// Outer corner reached from `near_right` along the incoming tangent.
    pub far_right: Point64,
}

/// Join whose miter-limit line crosses the radial bevel edges before the
/// curved support extensions begin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadialClipJoin {
    /// Direction of the centerline turn.
    pub turn: TurnSide,
    /// Outer stroke point at the end of the incoming segment.
    pub incoming_offset_point: Point64,
    /// Outer stroke point at the start of the outgoing segment.
    pub outgoing_offset_point: Point64,
    /// Intersection with the edge from the join point to the incoming offset.
    pub incoming: Point64,
    /// Intersection with the edge from the join point to the outgoing offset.
    pub outgoing: Point64,
    /// Point at the requested distance along the SVG miter-limit arc.
    pub limit_point: Point64,
    /// Unit direction of the clipping line, perpendicular to that arc.
    pub line_direction: Vector64,
}

/// Result selected by the SVG 2 `arcs` fallback rules.
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)] // A join in the tessellator's hot path must not allocate.
pub enum JoinConstruction {
    /// Equal tangents need no join region.
    Empty,
    /// Two straight supports use the `miter-clip` algorithm.
    MiterClip,
    /// Opposite parallel tangents use the SVG 2 rectangular fallback.
    ParallelRectangle(ParallelRectangleJoin),
    /// The miter-limit line crosses both radial bevel edges.
    RadialClip(RadialClipJoin),
    /// Curved supports were resolved to a single intersection.
    Arcs(ResolvedArcsJoin),
}

#[cfg(not(test))]
const _: () = assert!(core::mem::size_of::<JoinConstruction>() <= 152);

/// An invalid or numerically degenerate join input.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct JoinError(JoinErrorKind);

impl From<JoinErrorKind> for JoinError {
    fn from(kind: JoinErrorKind) -> Self {
        Self(kind)
    }
}

impl fmt::Display for JoinError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            JoinErrorKind::ExcessiveOuterCurvature => {
                formatter.write_str("SVG 2 outer curvature exceeds 2 / stroke-width")
            }
            JoinErrorKind::RadiusAdjustmentFailed => {
                formatter.write_str("SVG 2 radius adjustment has no finite tangent solution")
            }
            JoinErrorKind::NonFinitePoint => {
                formatter.write_str("join point contains a non-finite coordinate")
            }
            JoinErrorKind::NonFiniteTangent { segment } => {
                write!(
                    formatter,
                    "{segment} tangent contains a non-finite component"
                )
            }
            JoinErrorKind::ZeroTangent { segment } => {
                write!(formatter, "{segment} tangent has zero length")
            }
            JoinErrorKind::NonFiniteCurvature { segment } => {
                write!(formatter, "{segment} curvature is not finite")
            }
            JoinErrorKind::InvalidHalfWidth => {
                formatter.write_str("half width must be finite and greater than zero")
            }
            JoinErrorKind::InvalidMiterLimit => {
                formatter.write_str("miter limit must be finite and non-negative")
            }
            JoinErrorKind::CollinearTangents => {
                formatter.write_str("collinear tangents do not define an outer side")
            }
            JoinErrorKind::NoUniqueIntersection => {
                formatter.write_str("the original supports have no unique intersection")
            }
            JoinErrorKind::CollapsedOffsetCircle { segment } => {
                write!(formatter, "{segment} offset circle collapses to a point")
            }
            JoinErrorKind::MiterClipMissesBoundary { segment } => write!(
                formatter,
                "the miter-limit line does not cross the {segment} join boundary"
            ),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum JoinErrorKind {
    ExcessiveOuterCurvature,
    RadiusAdjustmentFailed,
    NonFinitePoint,
    NonFiniteTangent { segment: &'static str },
    ZeroTangent { segment: &'static str },
    NonFiniteCurvature { segment: &'static str },
    InvalidHalfWidth,
    InvalidMiterLimit,
    CollinearTangents,
    NoUniqueIntersection,
    CollapsedOffsetCircle { segment: &'static str },
    MiterClipMissesBoundary { segment: &'static str },
}

#[derive(Clone, Copy)]
struct PreparedJoin {
    turn: TurnSide,
    incoming_tangent: Vector64,
    outgoing_tangent: Vector64,
    incoming_offset_point: Point64,
    outgoing_offset_point: Point64,
    incoming_support: SupportCurve,
    outgoing_support: SupportCurve,
}

#[cfg(test)]
pub(crate) fn construct(input: JoinInput) -> Result<JoinConstruction, JoinError> {
    construct_svg2(input)
}

#[inline]
pub(crate) fn outer_curvature_values(input: JoinInput, tangent_cross: f64) -> (f64, f64, f64) {
    let curvature_limit = input.half_width.recip();
    if tangent_cross.abs() <= PARALLEL_EPSILON {
        return (
            input.incoming.curvature.abs(),
            input.outgoing.curvature.abs(),
            curvature_limit,
        );
    }

    let outer_side_sign = if tangent_cross > 0.0 { -1.0 } else { 1.0 };

    (
        input.incoming.curvature * outer_side_sign,
        input.outgoing.curvature * outer_side_sign,
        curvature_limit,
    )
}

fn parallel_rectangle(input: JoinInput, incoming_tangent: Vector64) -> ParallelRectangleJoin {
    let normal = incoming_tangent.left_normal() * input.half_width;
    let extension = incoming_tangent * (input.half_width * input.miter_limit);
    let near_left = input.at + normal;
    let near_right = input.at - normal;
    ParallelRectangleJoin {
        near_left,
        near_right,
        far_left: near_left + extension,
        far_right: near_right + extension,
    }
}

/// Builds the two infinite supports required by an `arcs` join and intersects them.
///
/// The function uses a mathematical coordinate system where positive curvature
/// turns left. It does not choose the closest intersection, adjust disjoint
/// circles, apply the miter limit, or triangulate the join.
///
/// # Errors
///
/// Returns [`JoinError`] for non-finite values, zero tangents, collinear
/// tangents, non-positive stroke width, or a collapsed offset circle.
///
#[must_use = "the support analysis must be handled"]
#[cfg(test)]
pub fn analyze(input: JoinInput) -> Result<JoinAnalysis, JoinError> {
    validate_input(input)?;

    let incoming_tangent = normalize(input.incoming.tangent, "incoming")?;
    let outgoing_tangent = normalize(input.outgoing.tangent, "outgoing")?;
    let prepared = prepare_join(input, incoming_tangent, outgoing_tangent)?;
    let intersections = intersect_supports(prepared.incoming_support, prepared.outgoing_support);

    Ok(JoinAnalysis {
        turn: prepared.turn,
        incoming_offset_point: prepared.incoming_offset_point,
        outgoing_offset_point: prepared.outgoing_offset_point,
        incoming_support: prepared.incoming_support,
        outgoing_support: prepared.outgoing_support,
        intersections,
    })
}

fn prepare_join(
    input: JoinInput,
    incoming_tangent: Vector64,
    outgoing_tangent: Vector64,
) -> Result<PreparedJoin, JoinError> {
    let turn_cross = incoming_tangent.cross(outgoing_tangent);
    let (turn, offset_distance) = if turn_cross > PARALLEL_EPSILON {
        (TurnSide::Left, -input.half_width)
    } else if turn_cross < -PARALLEL_EPSILON {
        (TurnSide::Right, input.half_width)
    } else {
        return Err(JoinErrorKind::CollinearTangents.into());
    };

    let incoming_offset_point = input.at + incoming_tangent.left_normal() * offset_distance;
    let outgoing_offset_point = input.at + outgoing_tangent.left_normal() * offset_distance;
    let incoming_support = build_support(
        incoming_offset_point,
        incoming_tangent,
        input.incoming.curvature,
        offset_distance,
        "incoming",
    )?;
    let outgoing_support = build_support(
        outgoing_offset_point,
        outgoing_tangent,
        input.outgoing.curvature,
        offset_distance,
        "outgoing",
    )?;
    Ok(PreparedJoin {
        turn,
        incoming_tangent,
        outgoing_tangent,
        incoming_offset_point,
        outgoing_offset_point,
        incoming_support,
        outgoing_support,
    })
}

fn validate_input(input: JoinInput) -> Result<(), JoinError> {
    if !input.at.is_finite() {
        return Err(JoinErrorKind::NonFinitePoint.into());
    }
    if !input.incoming.tangent.is_finite() {
        return Err(JoinErrorKind::NonFiniteTangent {
            segment: "incoming",
        }
        .into());
    }
    if !input.outgoing.tangent.is_finite() {
        return Err(JoinErrorKind::NonFiniteTangent {
            segment: "outgoing",
        }
        .into());
    }
    if !input.incoming.curvature.is_finite() {
        return Err(JoinErrorKind::NonFiniteCurvature {
            segment: "incoming",
        }
        .into());
    }
    if !input.outgoing.curvature.is_finite() {
        return Err(JoinErrorKind::NonFiniteCurvature {
            segment: "outgoing",
        }
        .into());
    }
    if !input.half_width.is_finite() || input.half_width <= 0.0 {
        return Err(JoinErrorKind::InvalidHalfWidth.into());
    }
    if !input.miter_limit.is_finite() || input.miter_limit < 0.0 {
        return Err(JoinErrorKind::InvalidMiterLimit.into());
    }

    Ok(())
}

fn normalize(vector: Vector64, segment: &'static str) -> Result<Vector64, JoinError> {
    let length = vector.x.hypot(vector.y);
    if length <= 0.0 {
        return Err(JoinErrorKind::ZeroTangent { segment }.into());
    }

    Ok(vector * length.recip())
}

fn build_support(
    offset_point: Point64,
    tangent: Vector64,
    curvature: f64,
    offset_distance: f64,
    segment: &'static str,
) -> Result<SupportCurve, JoinError> {
    let signed_center_radius = curvature.recip();
    if !signed_center_radius.is_finite() {
        return Ok(SupportCurve::Line {
            point: offset_point,
            direction: tangent,
        });
    }

    let signed_offset_radius = signed_center_radius - offset_distance;
    let radius = signed_offset_radius.abs();
    let scale = signed_center_radius
        .abs()
        .max(offset_distance.abs())
        .max(1.0);
    if radius <= GEOMETRY_EPSILON * scale {
        return Err(JoinErrorKind::CollapsedOffsetCircle { segment }.into());
    }

    let center_direction = if signed_offset_radius > 0.0 {
        tangent.left_normal()
    } else {
        -tangent.left_normal()
    };
    Ok(SupportCurve::Circle {
        center: offset_point + center_direction * radius,
        radius,
    })
}

fn resolve_boundaries<const EARLY_MITER_ACCEPT: bool, const EAGER_SWEEP: bool>(
    input: JoinInput,
    prepared: PreparedJoin,
    intersection: Point64,
) -> Result<JoinConstruction, JoinError> {
    let incoming_support = prepared.incoming_support;
    let outgoing_support = prepared.outgoing_support;
    let incoming_boundary = build_boundary_piece::<EAGER_SWEEP>(
        incoming_support,
        prepared.incoming_offset_point,
        intersection,
        prepared.incoming_tangent,
    );
    let outgoing_boundary = build_boundary_piece::<EAGER_SWEEP>(
        outgoing_support,
        prepared.outgoing_offset_point,
        intersection,
        -prepared.outgoing_tangent,
    );

    // Equal curvatures already use the cheaper zero-denominator chord fallback.
    let asymmetric_curvature =
        input.incoming.curvature.to_bits() != input.outgoing.curvature.to_bits();
    let clipped = if EARLY_MITER_ACCEPT && asymmetric_curvature {
        apply_miter_limit::<true>(
            input,
            prepared,
            intersection,
            incoming_boundary,
            outgoing_boundary,
        )?
    } else {
        apply_miter_limit::<false>(
            input,
            prepared,
            intersection,
            incoming_boundary,
            outgoing_boundary,
        )?
    };
    let (incoming_boundary, outgoing_boundary, clip) = match clipped {
        MiterLimitResult::SupportBoundaries {
            incoming_boundary,
            outgoing_boundary,
            clip,
        } => (incoming_boundary, outgoing_boundary, clip),
        MiterLimitResult::RadialEdges(join) => {
            return Ok(JoinConstruction::RadialClip(join));
        }
    };
    Ok(JoinConstruction::Arcs(ResolvedArcsJoin {
        turn: prepared.turn,
        incoming_boundary,
        outgoing_boundary,
        clipped: clip.is_some(),
        #[cfg(test)]
        incoming_offset_point: prepared.incoming_offset_point,
        #[cfg(test)]
        outgoing_offset_point: prepared.outgoing_offset_point,
        #[cfg(test)]
        incoming_support,
        #[cfg(test)]
        outgoing_support,
        #[cfg(test)]
        intersection,
        #[cfg(test)]
        clip,
    }))
}

#[derive(Clone, Copy)]
struct ClipLine {
    point: Point64,
    direction: Vector64,
}

enum MiterLimitResult {
    SupportBoundaries {
        incoming_boundary: BoundaryPiece,
        outgoing_boundary: BoundaryPiece,
        clip: Option<ClipEdge>,
    },
    RadialEdges(RadialClipJoin),
}

fn apply_miter_limit<const EARLY_ACCEPT: bool>(
    input: JoinInput,
    prepared: PreparedJoin,
    intersection: Point64,
    incoming_boundary: BoundaryPiece,
    outgoing_boundary: BoundaryPiece,
) -> Result<MiterLimitResult, JoinError> {
    let limit = input.half_width * input.miter_limit;
    let Some(clip_line) = miter_clip_line::<EARLY_ACCEPT>(
        input.at,
        prepared.incoming_offset_point,
        prepared.outgoing_offset_point,
        intersection,
        limit,
    ) else {
        return Ok(MiterLimitResult::SupportBoundaries {
            incoming_boundary,
            outgoing_boundary,
            clip: None,
        });
    };
    match (
        clip_boundary(incoming_boundary, clip_line, "incoming"),
        clip_boundary(outgoing_boundary, clip_line, "outgoing"),
    ) {
        (Ok(incoming_boundary), Ok(outgoing_boundary)) => {
            let clip = ClipEdge {
                incoming: boundary_end(incoming_boundary),
                outgoing: boundary_end(outgoing_boundary),
                limit_point: clip_line.point,
                line_direction: clip_line.direction,
            };
            Ok(MiterLimitResult::SupportBoundaries {
                incoming_boundary,
                outgoing_boundary,
                clip: Some(clip),
            })
        }
        _ => Ok(MiterLimitResult::RadialEdges(RadialClipJoin {
            turn: prepared.turn,
            incoming_offset_point: prepared.incoming_offset_point,
            outgoing_offset_point: prepared.outgoing_offset_point,
            incoming: clip_radial_edge(
                input.at,
                prepared.incoming_offset_point,
                clip_line,
                "incoming",
            )?,
            outgoing: clip_radial_edge(
                input.at,
                prepared.outgoing_offset_point,
                clip_line,
                "outgoing",
            )?,
            limit_point: clip_line.point,
            line_direction: clip_line.direction,
        })),
    }
}

fn clip_radial_edge(
    at: Point64,
    offset_point: Point64,
    clip_line: ClipLine,
    segment: &'static str,
) -> Result<Point64, JoinError> {
    let radial = offset_point - at;
    let denominator = radial.cross(clip_line.direction);
    if denominator.abs() <= PARALLEL_EPSILON {
        return Err(JoinErrorKind::MiterClipMissesBoundary { segment }.into());
    }

    let parameter = (clip_line.point - at).cross(clip_line.direction) / denominator;
    let epsilon = GEOMETRY_EPSILON * parameter.abs().max(1.0);
    if parameter < -epsilon || parameter > 1.0 + epsilon {
        return Err(JoinErrorKind::MiterClipMissesBoundary { segment }.into());
    }

    Ok(at + radial * parameter.clamp(0.0, 1.0))
}

fn miter_clip_line<const EARLY_ACCEPT: bool>(
    at: Point64,
    incoming_offset_point: Point64,
    outgoing_offset_point: Point64,
    intersection: Point64,
    limit: f64,
) -> Option<ClipLine> {
    let bisector_vector = (incoming_offset_point - at) + (outgoing_offset_point - at);
    let to_intersection = intersection - at;

    if EARLY_ACCEPT && miter_arc_fits_limit(bisector_vector, to_intersection, limit) {
        return None;
    }
    let bisector = normalized(bisector_vector)?;
    let normal = bisector.left_normal();
    let denominator = 2.0 * to_intersection.dot(normal);
    let scale = to_intersection.x.hypot(to_intersection.y).max(1.0);

    if denominator.abs() <= GEOMETRY_EPSILON * scale {
        let total_length = to_intersection.x.hypot(to_intersection.y);
        if total_length <= limit + GEOMETRY_EPSILON * total_length.max(1.0) {
            return None;
        }

        return Some(ClipLine {
            point: at + bisector * limit,
            direction: normal,
        });
    }

    let signed_center_distance = to_intersection.square_length() / denominator;
    let center = at + normal * signed_center_distance;
    let radius = signed_center_distance.abs();
    let start_radius = at - center;
    let end_radius = intersection - center;
    let sweep = directed_sweep_vectors(start_radius, end_radius, signed_center_distance > 0.0);
    let total_length = radius * sweep.abs();
    if total_length <= limit + GEOMETRY_EPSILON * total_length.max(1.0) {
        return None;
    }

    let limit_angle = sweep.signum() * limit / radius;
    let (limit_sine, limit_cosine) = limit_angle.sin_cos();
    let start_unit_radius = start_radius * radius.recip();
    let limit_radius = Vector64::new(
        start_unit_radius.x * limit_cosine - start_unit_radius.y * limit_sine,
        start_unit_radius.x * limit_sine + start_unit_radius.y * limit_cosine,
    );

    Some(ClipLine {
        point: center + limit_radius * radius,
        direction: limit_radius,
    })
}

/// Proves that the auxiliary miter arc fits without constructing its circle.
///
/// A minor circular arc is at most `π / 2` times its chord. Ambiguous
/// orientations and zero denominators stay on the exact path so this check can
/// only accept, never introduce clipping differences near a half turn.
#[allow(clippy::float_cmp)] // Exact zero is routed to the existing tolerant fallback.
#[inline(always)]
fn miter_arc_fits_limit(bisector: Vector64, to_intersection: Vector64, limit: f64) -> bool {
    const MAX_LENGTH_TO_CHORD_SQUARED: f64 =
        core::f64::consts::FRAC_PI_2 * core::f64::consts::FRAC_PI_2;
    const ROUNDING_MARGIN: f64 = 1.0 + 32.0 * f64::EPSILON;

    let chord_squared = to_intersection.square_length();
    let limit_squared = limit * limit;
    let chord_upper_squared = chord_squared * ROUNDING_MARGIN;
    let chord_fits = chord_upper_squared <= limit_squared;
    if !chord_fits {
        return false;
    }

    let normal = bisector.left_normal();
    if normal.dot(to_intersection) == 0.0 {
        return false;
    }

    let upper_length_squared = chord_upper_squared * MAX_LENGTH_TO_CHORD_SQUARED;
    let minor_arc_fits = upper_length_squared <= limit_squared;
    if !minor_arc_fits {
        return false;
    }

    let orientation = normal.cross(to_intersection);
    let orientation_scale =
        (normal.x * to_intersection.y).abs() + (normal.y * to_intersection.x).abs();

    orientation < -GEOMETRY_EPSILON * orientation_scale
}

fn clip_boundary(
    boundary: BoundaryPiece,
    clip_line: ClipLine,
    segment: &'static str,
) -> Result<BoundaryPiece, JoinError> {
    match boundary {
        BoundaryPiece::Line { start, end } => {
            let boundary_direction = end - start;
            let denominator = boundary_direction.cross(clip_line.direction);
            if denominator.abs() <= PARALLEL_EPSILON {
                return Err(JoinErrorKind::MiterClipMissesBoundary { segment }.into());
            }
            let parameter = (clip_line.point - start).cross(clip_line.direction) / denominator;
            let epsilon = GEOMETRY_EPSILON * parameter.abs().max(1.0);
            if parameter < -epsilon || parameter > 1.0 + epsilon {
                return Err(JoinErrorKind::MiterClipMissesBoundary { segment }.into());
            }

            Ok(BoundaryPiece::Line {
                start,
                end: start + boundary_direction * parameter.clamp(0.0, 1.0),
            })
        }
        BoundaryPiece::Arc {
            center,
            radius,
            start,
            end,
            sweep,
        } => {
            let start_radius = start - center;
            let end_radius = end - center;
            let sweep_radians = sweep.resolve(start_radius, end_radius);
            let intersections =
                line_circle_intersections(clip_line.point, clip_line.direction, center, radius);
            let clipped = intersection_points(intersections)
                .iter()
                .copied()
                .flatten()
                .filter_map(|point| {
                    let candidate_radius = point - center;
                    let candidate_sweep = directed_sweep_vectors(
                        start_radius,
                        candidate_radius,
                        sweep.counter_clockwise(),
                    );
                    let epsilon = GEOMETRY_EPSILON * sweep_radians.abs().max(1.0);
                    (candidate_sweep.abs() <= sweep_radians.abs() + epsilon).then_some((
                        candidate_sweep.abs(),
                        point,
                        candidate_sweep,
                    ))
                })
                .min_by(|first, second| first.0.total_cmp(&second.0));
            let Some((_, clipped_end, clipped_sweep)) = clipped else {
                return Err(JoinErrorKind::MiterClipMissesBoundary { segment }.into());
            };

            Ok(BoundaryPiece::Arc {
                center,
                radius,
                start,
                end: clipped_end,
                sweep: ArcSweep::resolved(clipped_sweep),
            })
        }
    }
}

fn normalized(vector: Vector64) -> Option<Vector64> {
    let length = vector.x.hypot(vector.y);
    (length > GEOMETRY_EPSILON).then_some(vector * length.recip())
}

fn intersection_points(intersections: Intersections) -> [Option<Point64>; 2] {
    match intersections {
        Intersections::None | Intersections::Coincident => [None, None],
        Intersections::One(point) => [Some(point), None],
        Intersections::Two([first, second]) => [Some(first), Some(second)],
    }
}

fn boundary_end(boundary: BoundaryPiece) -> Point64 {
    match boundary {
        BoundaryPiece::Line { end, .. } | BoundaryPiece::Arc { end, .. } => end,
    }
}

fn boundary_end_tangent(boundary: BoundaryPiece) -> Vector64 {
    let tangent = match boundary {
        BoundaryPiece::Line { start, end } => end - start,
        BoundaryPiece::Arc {
            end, center, sweep, ..
        } => (end - center).left_normal() * if sweep.counter_clockwise() { 1.0 } else { -1.0 },
    };
    tangent * tangent.square_length().sqrt().recip()
}

fn boundary_start(boundary: BoundaryPiece) -> Point64 {
    match boundary {
        BoundaryPiece::Line { start, .. } | BoundaryPiece::Arc { start, .. } => start,
    }
}

fn build_boundary_piece<const EAGER_SWEEP: bool>(
    support: SupportCurve,
    start: Point64,
    end: Point64,
    desired_start_tangent: Vector64,
) -> BoundaryPiece {
    let SupportCurve::Circle { center, radius } = support else {
        return BoundaryPiece::Line { start, end };
    };

    let start_radius = start - center;
    let end_radius = end - center;
    let counter_clockwise = start_radius.cross(desired_start_tangent) > 0.0;
    let sweep = if EAGER_SWEEP {
        ArcSweep::resolved(directed_sweep_vectors(
            start_radius,
            end_radius,
            counter_clockwise,
        ))
    } else {
        ArcSweep::lazy(counter_clockwise)
    };

    BoundaryPiece::Arc {
        center,
        radius,
        start,
        end,
        sweep,
    }
}

#[cfg(test)]
fn directed_sweep(start: f64, end: f64, counter_clockwise: bool) -> f64 {
    if counter_clockwise {
        positive_modulo(end - start, core::f64::consts::TAU)
    } else {
        -positive_modulo(start - end, core::f64::consts::TAU)
    }
}

pub(crate) fn directed_sweep_vectors(
    start: Vector64,
    end: Vector64,
    counter_clockwise: bool,
) -> f64 {
    let signed_sweep = start.cross(end).atan2(start.dot(end));
    if counter_clockwise {
        if signed_sweep < 0.0 {
            signed_sweep + core::f64::consts::TAU
        } else {
            signed_sweep
        }
    } else if signed_sweep > 0.0 {
        signed_sweep - core::f64::consts::TAU
    } else {
        signed_sweep
    }
}

#[cfg(test)]
pub(crate) fn directed_sweep_vectors_with_modulo(
    start: Vector64,
    end: Vector64,
    counter_clockwise: bool,
) -> f64 {
    let signed_sweep = start.cross(end).atan2(start.dot(end));
    if counter_clockwise {
        positive_modulo(signed_sweep, core::f64::consts::TAU)
    } else {
        -positive_modulo(-signed_sweep, core::f64::consts::TAU)
    }
}

#[cfg(test)]
pub(crate) fn directed_sweep_with_endpoint_angles(
    start: Vector64,
    end: Vector64,
    counter_clockwise: bool,
) -> f64 {
    directed_sweep(
        start.y.atan2(start.x),
        end.y.atan2(end.x),
        counter_clockwise,
    )
}

fn positive_modulo(value: f64, modulus: f64) -> f64 {
    let remainder = value % modulus;
    if remainder < 0.0 {
        remainder + modulus
    } else {
        remainder
    }
}

#[derive(Clone, Copy)]
enum IntersectionProblem {
    Disjoint,
    Coincident,
}

fn closest_intersection(
    intersections: Intersections,
    at: Point64,
) -> Result<Point64, IntersectionProblem> {
    match intersections {
        Intersections::None => Err(IntersectionProblem::Disjoint),
        Intersections::One(point) => Ok(point),
        Intersections::Two([first, second]) => {
            let first_distance = (first - at).square_length();
            let second_distance = (second - at).square_length();
            if first_distance <= second_distance {
                Ok(first)
            } else {
                Ok(second)
            }
        }
        Intersections::Coincident => Err(IntersectionProblem::Coincident),
    }
}

fn intersect_supports(first: SupportCurve, second: SupportCurve) -> Intersections {
    match (first, second) {
        (
            SupportCurve::Line {
                point: first_point,
                direction: first_direction,
            },
            SupportCurve::Line {
                point: second_point,
                direction: second_direction,
            },
        ) => line_line_intersections(first_point, first_direction, second_point, second_direction),
        (SupportCurve::Line { point, direction }, SupportCurve::Circle { center, radius })
        | (SupportCurve::Circle { center, radius }, SupportCurve::Line { point, direction }) => {
            line_circle_intersections(point, direction, center, radius)
        }
        (
            SupportCurve::Circle {
                center: first_center,
                radius: first_radius,
            },
            SupportCurve::Circle {
                center: second_center,
                radius: second_radius,
            },
        ) => circle_circle_intersections(first_center, first_radius, second_center, second_radius),
    }
}

fn closest_support_intersection(
    first: SupportCurve,
    second: SupportCurve,
    at: Point64,
) -> Result<Point64, IntersectionProblem> {
    match (first, second) {
        (
            SupportCurve::Line {
                point: first_point,
                direction: first_direction,
            },
            SupportCurve::Line {
                point: second_point,
                direction: second_direction,
            },
        ) => closest_intersection(
            line_line_intersections(first_point, first_direction, second_point, second_direction),
            at,
        ),
        (SupportCurve::Line { point, direction }, SupportCurve::Circle { center, radius })
        | (SupportCurve::Circle { center, radius }, SupportCurve::Line { point, direction }) => {
            solve_line_circle(point, direction, center, radius).closest_to(at)
        }
        (
            SupportCurve::Circle {
                center: first_center,
                radius: first_radius,
            },
            SupportCurve::Circle {
                center: second_center,
                radius: second_radius,
            },
        ) => solve_circle_circle(first_center, first_radius, second_center, second_radius)
            .closest_to(at),
    }
}

fn line_line_intersections(
    first_point: Point64,
    first_direction: Vector64,
    second_point: Point64,
    second_direction: Vector64,
) -> Intersections {
    let denominator = first_direction.cross(second_direction);
    let between = second_point - first_point;
    if denominator.abs() <= PARALLEL_EPSILON {
        let scale = between.x.abs().max(between.y.abs()).max(1.0);
        if between.cross(first_direction).abs() <= GEOMETRY_EPSILON * scale {
            return Intersections::Coincident;
        }
        return Intersections::None;
    }

    let first_parameter = between.cross(second_direction) / denominator;
    Intersections::One(first_point + first_direction * first_parameter)
}

fn line_circle_intersections(
    line_point: Point64,
    line_direction: Vector64,
    circle_center: Point64,
    circle_radius: f64,
) -> Intersections {
    solve_line_circle(line_point, line_direction, circle_center, circle_radius).all()
}

fn solve_line_circle(
    line_point: Point64,
    line_direction: Vector64,
    circle_center: Point64,
    circle_radius: f64,
) -> SymmetricIntersections {
    let to_center = circle_center - line_point;
    let closest = line_point + line_direction * to_center.dot(line_direction);
    let closest_delta = closest - circle_center;
    let radius_squared = circle_radius * circle_radius;
    let distance_squared = closest_delta.square_length();
    let remaining_squared = radius_squared - distance_squared;
    let epsilon = GEOMETRY_EPSILON * radius_squared.max(distance_squared).max(1.0);

    if remaining_squared < -epsilon {
        return SymmetricIntersections::None;
    }
    if remaining_squared.abs() <= epsilon {
        return SymmetricIntersections::One(closest);
    }

    let distance_along_line = remaining_squared.sqrt();
    SymmetricIntersections::Two {
        midpoint: closest,
        offset: line_direction * distance_along_line,
    }
}

fn circle_circle_intersections(
    first_center: Point64,
    first_radius: f64,
    second_center: Point64,
    second_radius: f64,
) -> Intersections {
    solve_circle_circle(first_center, first_radius, second_center, second_radius).all()
}

fn solve_circle_circle(
    first_center: Point64,
    first_radius: f64,
    second_center: Point64,
    second_radius: f64,
) -> SymmetricIntersections {
    let between = second_center - first_center;
    let center_distance = between.x.hypot(between.y);
    let scale = first_radius
        .max(second_radius)
        .max(center_distance)
        .max(1.0);
    let epsilon = GEOMETRY_EPSILON * scale;

    if center_distance <= epsilon {
        return if (first_radius - second_radius).abs() <= epsilon {
            SymmetricIntersections::Coincident
        } else {
            SymmetricIntersections::None
        };
    }

    let radius_sum = first_radius + second_radius;
    let radius_difference = (first_radius - second_radius).abs();
    if center_distance > radius_sum + epsilon || center_distance < radius_difference - epsilon {
        return SymmetricIntersections::None;
    }

    let along_centers = (center_distance * center_distance + first_radius * first_radius
        - second_radius * second_radius)
        / (2.0 * center_distance);
    let height_squared = first_radius * first_radius - along_centers * along_centers;
    let squared_epsilon = GEOMETRY_EPSILON * first_radius.max(second_radius).max(1.0).powi(2);
    let center_direction = between * center_distance.recip();
    let midpoint = first_center + center_direction * along_centers;

    if height_squared.abs() <= squared_epsilon {
        return SymmetricIntersections::One(midpoint);
    }
    if height_squared < 0.0 {
        return SymmetricIntersections::None;
    }

    let perpendicular = center_direction.left_normal() * height_squared.sqrt();
    SymmetricIntersections::Two {
        midpoint,
        offset: perpendicular,
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::*;

    const TEST_EPSILON: f64 = 1.0e-9;

    #[test]
    fn vector_sweep_keeps_a_small_counter_clockwise_turn_small() {
        let epsilon = 1.0e-6_f64;
        let start = Vector64::new(1.0, 0.0);
        let end = Vector64::new(epsilon.cos(), epsilon.sin());

        let sweep = directed_sweep_vectors(start, end, true);

        assert!(near(sweep, epsilon));
    }

    #[test]
    fn vector_sweep_keeps_a_small_clockwise_turn_small() {
        let epsilon = 1.0e-6_f64;
        let start = Vector64::new(1.0, 0.0);
        let end = Vector64::new(epsilon.cos(), -epsilon.sin());

        let sweep = directed_sweep_vectors(start, end, false);

        assert!(near(sweep, -epsilon));
    }

    #[test]
    fn vector_sweep_selects_the_long_turn_in_each_direction() {
        let epsilon = 1.0e-6_f64;
        let start = Vector64::new(1.0, 0.0);
        let above = Vector64::new(epsilon.cos(), epsilon.sin());
        let below = Vector64::new(epsilon.cos(), -epsilon.sin());

        let clockwise = directed_sweep_vectors(start, above, false);
        let counter_clockwise = directed_sweep_vectors(start, below, true);

        assert!(
            near(clockwise, -(core::f64::consts::TAU - epsilon))
                && near(counter_clockwise, core::f64::consts::TAU - epsilon)
        );
    }

    #[test]
    fn vector_sweep_preserves_the_direction_of_a_half_turn() {
        let start = Vector64::new(1.0, 0.0);
        let opposite = Vector64::new(-1.0, 0.0);

        let counter_clockwise = directed_sweep_vectors(start, opposite, true);
        let clockwise = directed_sweep_vectors(start, opposite, false);

        assert!(
            near(counter_clockwise, core::f64::consts::PI)
                && near(clockwise, -core::f64::consts::PI)
        );
    }

    #[test]
    fn vector_sweep_maps_equal_radii_to_zero_in_both_directions() {
        let radius = Vector64::new(3.0, -4.0);

        let counter_clockwise = directed_sweep_vectors(radius, radius, true);
        let clockwise = directed_sweep_vectors(radius, radius, false);

        assert!(near(counter_clockwise, 0.0) && near(clockwise, 0.0));
    }

    #[test]
    fn analyze_places_a_left_turn_outer_edge_on_the_right() {
        let analysis =
            analyze(right_angle_input(0.0, 0.0)).expect("a right-angle line join must be valid");

        assert!(
            point_is_near(analysis.incoming_offset_point, Point64::new(0.0, -2.0))
                && point_is_near(analysis.outgoing_offset_point, Point64::new(2.0, 0.0))
        );
    }

    #[test]
    fn analyze_builds_the_expected_osculating_circle() {
        let analysis =
            analyze(right_angle_input(0.1, 0.0)).expect("the curved input must be valid");

        let SupportCurve::Circle { center, radius } = analysis.incoming_support else {
            panic!("incoming support should be a circle");
        };
        assert!(point_is_near(center, Point64::new(0.0, 10.0)) && near(radius, 12.0));
    }

    #[test]
    fn analyze_rejects_a_zero_tangent() {
        let mut input = right_angle_input(0.0, 0.0);
        input.incoming.tangent = Vector64::new(0.0, 0.0);

        let error = analyze(input).expect_err("a zero tangent must fail");

        assert_eq!(error.to_string(), "incoming tangent has zero length");
    }

    #[test]
    fn analyze_rejects_collinear_tangents() {
        let mut input = right_angle_input(0.0, 0.0);
        input.outgoing.tangent = Vector64::new(2.0, 0.0);

        let error = analyze(input).expect_err("collinear tangents must fail for now");

        assert_eq!(
            error.to_string(),
            "collinear tangents do not define an outer side"
        );
    }

    #[test]
    fn analyze_rejects_a_negative_miter_limit() {
        let mut input = right_angle_input(0.0, 0.0);
        input.miter_limit = -1.0;

        let error = analyze(input).expect_err("a negative miter limit must fail");

        assert_eq!(
            error.to_string(),
            "miter limit must be finite and non-negative"
        );
    }

    #[test]
    fn construct_returns_empty_for_equal_tangents() {
        let mut input = right_angle_input(0.1, 0.1);
        input.outgoing.tangent = Vector64::new(2.0, 0.0);

        let result = construct(input).expect("equal tangents must be valid");

        assert_eq!(result, JoinConstruction::Empty);
    }

    #[test]
    fn construct_uses_miter_clip_for_two_lines() {
        let result =
            construct(right_angle_input(0.0, 0.0)).expect("a right-angle line join must be valid");

        assert_eq!(result, JoinConstruction::MiterClip);
    }

    #[test]
    fn construct_uses_a_rectangle_for_opposite_parallel_tangents() {
        let mut input = right_angle_input(0.0, 0.1);
        input.outgoing.tangent = Vector64::new(-1.0, 0.0);
        input.miter_limit = 3.0;

        let result = construct(input).expect("the parallel fallback must be valid");

        assert_eq!(
            result,
            JoinConstruction::ParallelRectangle(ParallelRectangleJoin {
                near_left: Point64::new(0.0, 2.0),
                near_right: Point64::new(0.0, -2.0),
                far_left: Point64::new(6.0, 2.0),
                far_right: Point64::new(6.0, -2.0),
            })
        );
    }

    #[test]
    fn opposite_parallel_tangents_apply_the_curvature_guard_first() {
        let mut input = right_angle_input(0.0, 0.6);
        input.outgoing.tangent = Vector64::new(-1.0, 0.0);
        assert_eq!(
            construct(input).unwrap_err().0,
            JoinErrorKind::ExcessiveOuterCurvature
        );
    }

    #[test]
    fn construct_preserves_tight_curvature_toward_the_inner_side() {
        let result = construct(right_angle_input(0.6, 0.0))
            .expect("inner curvature has a valid outer offset");

        assert!(
            matches!(result, JoinConstruction::Arcs(_)),
            "inner curvature should produce arcs, got {:?}",
            result
        );
    }

    #[test]
    fn excessive_outer_curvature_requests_round_fallback() {
        assert_eq!(
            construct(right_angle_input(-0.6, 0.0)).unwrap_err().0,
            JoinErrorKind::ExcessiveOuterCurvature
        );
    }

    #[test]
    fn collapsed_offset_circle_is_reported_as_degenerate() {
        assert_eq!(
            construct(right_angle_input(-0.5, 0.0)).unwrap_err().0,
            JoinErrorKind::CollapsedOffsetCircle {
                segment: "incoming"
            }
        );
    }

    #[test]
    fn construct_preserves_tight_inner_curvature_for_a_right_turn() {
        let mut input = right_angle_input(-0.6, 0.0);
        input.outgoing.tangent = Vector64::new(0.0, -1.0);
        let result = construct(input).expect("inner curvature has a valid outer offset");

        assert!(
            matches!(result, JoinConstruction::Arcs(_)),
            "inner curvature should produce arcs, got {:?}",
            result
        );
    }

    #[test]
    fn construct_selects_the_intersection_closest_to_the_join() {
        let result =
            construct(right_angle_input(0.0, 0.1)).expect("line and circle supports must resolve");
        let JoinConstruction::Arcs(join) = result else {
            panic!("line and circle supports should produce an arcs join");
        };

        assert!(point_is_near(
            join.intersection,
            Point64::new(-10.0 + 140.0_f64.sqrt(), -2.0)
        ));
    }

    #[test]
    fn construct_selects_the_near_circle_circle_intersection() {
        let result =
            construct(right_angle_input(0.1, 0.1)).expect("two crossing circles must resolve");
        let JoinConstruction::Arcs(join) = result else {
            panic!("the two curved supports should produce an arcs join");
        };
        let root = 47.0_f64.sqrt();

        assert!(point_is_near(
            join.intersection,
            Point64::new(-5.0 + root, 5.0 - root)
        ));
    }

    #[test]
    fn disjoint_line_circle_supports_are_adjusted_to_contact() {
        let JoinConstruction::Arcs(join) = construct(right_angle_input(0.0, -1.0 / 3.0)).unwrap()
        else {
            panic!("disjoint supports should resolve after radius adjustment");
        };
        assert!(point_is_near(join.intersection, Point64::new(4.0, -2.0)));
        assert_eq!(
            join.outgoing_support,
            SupportCurve::Circle {
                center: Point64::new(4.0, 0.0),
                radius: 2.0,
            }
        );
    }

    #[test]
    fn construct_builds_a_line_from_the_incoming_offset_to_the_vertex() {
        let result =
            construct(right_angle_input(0.0, 0.1)).expect("line and circle supports must resolve");
        let JoinConstruction::Arcs(join) = result else {
            panic!("line and circle supports should produce an arcs join");
        };

        assert!(matches!(
            join.incoming_boundary,
            BoundaryPiece::Line { start, end }
                if point_is_near(start, join.incoming_offset_point)
                    && point_is_near(end, join.intersection)
        ));
    }

    #[test]
    fn construct_orients_the_outgoing_arc_back_toward_the_join() {
        let result =
            construct(right_angle_input(0.0, 0.1)).expect("line and circle supports must resolve");
        let JoinConstruction::Arcs(join) = result else {
            panic!("line and circle supports should produce an arcs join");
        };
        let BoundaryPiece::Arc {
            center,
            start,
            end,
            sweep,
            ..
        } = join.outgoing_boundary
        else {
            panic!("the outgoing curved support should produce an arc");
        };
        let sweep_radians = sweep.resolve(start - center, end - center);

        assert!(sweep_radians < 0.0 && sweep_radians.abs() < core::f64::consts::PI);
    }

    #[test]
    fn a_large_miter_limit_keeps_the_resolved_boundaries() {
        let result =
            construct(right_angle_input(0.0, 0.1)).expect("line and circle supports must resolve");
        let JoinConstruction::Arcs(join) = result else {
            panic!("line and circle supports should produce an arcs join");
        };

        assert!(
            join.clip.is_none()
                && point_is_near(boundary_end(join.incoming_boundary), join.intersection)
                && point_is_near(boundary_end(join.outgoing_boundary), join.intersection)
        );
    }

    #[test]
    fn miter_arc_bound_accepts_only_proven_no_clip_cases() {
        let bisector = Vector64::new(1.0, 0.0);
        let minor_chord = Vector64::new(1.0, 1.0);
        let major_chord = Vector64::new(-1.0, 1.0);
        let threshold = core::f64::consts::FRAC_PI_2 * 2.0_f64.sqrt();

        assert!(miter_arc_fits_limit(
            bisector,
            minor_chord,
            threshold * 1.000_001,
        ));
        assert!(!miter_arc_fits_limit(
            bisector,
            minor_chord,
            threshold * 0.999_999,
        ));
        assert!(!miter_arc_fits_limit(
            bisector,
            major_chord,
            threshold * 2.0,
        ));
        assert!(!miter_arc_fits_limit(
            bisector,
            Vector64::new(1.0, 0.0),
            1.000_001,
        ));
    }

    #[test]
    fn miter_limit_clips_both_boundaries_with_one_line() {
        let mut input = right_angle_input(0.0, 0.1);
        input.miter_limit = 1.2;

        let result = construct(input).expect("the miter limit must clip the arcs join");
        let JoinConstruction::Arcs(join) = result else {
            panic!("line and circle supports should produce an arcs join");
        };
        let clip = join
            .clip
            .expect("the resolved join should have a clip edge");

        assert!(
            (clip.incoming - clip.limit_point)
                .cross(clip.line_direction)
                .abs()
                <= TEST_EPSILON
                && (clip.outgoing - clip.limit_point)
                    .cross(clip.line_direction)
                    .abs()
                    <= TEST_EPSILON
                && point_is_near(boundary_end(join.incoming_boundary), clip.incoming)
                && point_is_near(boundary_end(join.outgoing_boundary), clip.outgoing)
                && !point_is_near(clip.incoming, join.intersection)
                && !point_is_near(clip.outgoing, join.intersection)
        );
    }

    #[test]
    fn miter_limit_shortens_a_curved_boundary_without_moving_its_start() {
        let unbounded =
            construct(right_angle_input(0.0, 0.1)).expect("the reference arcs join must resolve");
        let JoinConstruction::Arcs(unbounded) = unbounded else {
            panic!("line and circle supports should produce an arcs join");
        };
        let mut clipped_input = right_angle_input(0.0, 0.1);
        clipped_input.miter_limit = 1.2;
        let clipped = construct(clipped_input).expect("the arcs join must be clipped");
        let JoinConstruction::Arcs(clipped) = clipped else {
            panic!("line and circle supports should produce an arcs join");
        };
        let BoundaryPiece::Arc {
            center: unbounded_center,
            start: unbounded_start,
            end: unbounded_end,
            sweep: unbounded_sweep,
            ..
        } = unbounded.outgoing_boundary
        else {
            panic!("the outgoing boundary should be curved");
        };
        let BoundaryPiece::Arc {
            center: clipped_center,
            start: clipped_start,
            end: clipped_end,
            sweep: clipped_sweep,
            ..
        } = clipped.outgoing_boundary
        else {
            panic!("the clipped outgoing boundary should remain curved");
        };

        let unbounded_sweep = unbounded_sweep.resolve(
            unbounded_start - unbounded_center,
            unbounded_end - unbounded_center,
        );
        let clipped_sweep =
            clipped_sweep.resolve(clipped_start - clipped_center, clipped_end - clipped_center);
        assert!(
            point_is_near(clipped_start, unbounded_start)
                && clipped_sweep.signum() == unbounded_sweep.signum()
                && clipped_sweep.abs() < unbounded_sweep.abs()
        );
    }

    #[test]
    fn sub_unit_miter_limit_clips_the_radial_join_edges() {
        let mut input = right_angle_input(0.0, 0.1);
        input.miter_limit = 0.5;

        let result = construct(input).expect("the inner clip must be representable");
        let JoinConstruction::RadialClip(join) = result else {
            panic!("a sub-unit miter limit should clip the radial join edges");
        };

        assert!(
            near(join.incoming.x, 0.0)
                && join.incoming.y > -input.half_width
                && join.incoming.y < 0.0
                && near(join.outgoing.y, 0.0)
                && join.outgoing.x > 0.0
                && join.outgoing.x < input.half_width
                && (join.incoming - join.limit_point)
                    .cross(join.line_direction)
                    .abs()
                    <= TEST_EPSILON
                && (join.outgoing - join.limit_point)
                    .cross(join.line_direction)
                    .abs()
                    <= TEST_EPSILON
        );
    }

    #[test]
    fn incoming_arc_starts_with_the_incoming_path_tangent() {
        let result =
            construct(right_angle_input(0.1, 0.1)).expect("two crossing circles must resolve");
        let JoinConstruction::Arcs(join) = result else {
            panic!("two curved supports should produce an arcs join");
        };
        let start_tangent = boundary_start_tangent(join.incoming_boundary);

        assert!(start_tangent.dot(Vector64::new(1.0, 0.0)) > 1.0 - TEST_EPSILON);
    }

    #[test]
    fn outgoing_arc_starts_opposite_to_the_outgoing_path_tangent() {
        let result =
            construct(right_angle_input(0.1, 0.1)).expect("two crossing circles must resolve");
        let JoinConstruction::Arcs(join) = result else {
            panic!("two curved supports should produce an arcs join");
        };
        let start_tangent = boundary_start_tangent(join.outgoing_boundary);

        assert!(start_tangent.dot(Vector64::new(0.0, -1.0)) > 1.0 - TEST_EPSILON);
    }

    #[test]
    fn line_line_returns_the_crossing_point() {
        let result = line_line_intersections(
            Point64::new(0.0, 2.0),
            Vector64::new(1.0, 0.0),
            Point64::new(1.0, 0.0),
            Vector64::new(0.0, 1.0),
        );

        assert_eq!(result, Intersections::One(Point64::new(1.0, 2.0)));
    }

    #[test]
    fn line_line_reports_distinct_parallel_lines() {
        let result = line_line_intersections(
            Point64::new(0.0, 0.0),
            Vector64::new(1.0, 0.0),
            Point64::new(0.0, 1.0),
            Vector64::new(1.0, 0.0),
        );

        assert_eq!(result, Intersections::None);
    }

    #[test]
    fn line_line_reports_the_same_line() {
        let result = line_line_intersections(
            Point64::new(0.0, 0.0),
            Vector64::new(1.0, 0.0),
            Point64::new(4.0, 0.0),
            Vector64::new(-1.0, 0.0),
        );

        assert_eq!(result, Intersections::Coincident);
    }

    #[test]
    fn line_circle_returns_two_crossing_points() {
        let result = line_circle_intersections(
            Point64::new(-10.0, 0.0),
            Vector64::new(1.0, 0.0),
            Point64::new(0.0, 0.0),
            5.0,
        );

        assert!(two_points_are_near(
            result,
            Point64::new(-5.0, 0.0),
            Point64::new(5.0, 0.0)
        ));
    }

    #[test]
    fn line_circle_returns_one_tangent_point() {
        let result = line_circle_intersections(
            Point64::new(-10.0, 5.0),
            Vector64::new(1.0, 0.0),
            Point64::new(0.0, 0.0),
            5.0,
        );

        assert!(matches!(
            result,
            Intersections::One(point) if point_is_near(point, Point64::new(0.0, 5.0))
        ));
    }

    #[test]
    fn line_circle_reports_a_miss() {
        let result = line_circle_intersections(
            Point64::new(-10.0, 6.0),
            Vector64::new(1.0, 0.0),
            Point64::new(0.0, 0.0),
            5.0,
        );

        assert_eq!(result, Intersections::None);
    }

    #[test]
    fn circle_circle_returns_two_crossing_points() {
        let result =
            circle_circle_intersections(Point64::new(0.0, 0.0), 5.0, Point64::new(8.0, 0.0), 5.0);

        assert!(two_points_are_near(
            result,
            Point64::new(4.0, -3.0),
            Point64::new(4.0, 3.0)
        ));
    }

    #[test]
    fn circle_circle_returns_one_external_tangent_point() {
        let result =
            circle_circle_intersections(Point64::new(0.0, 0.0), 5.0, Point64::new(8.0, 0.0), 3.0);

        assert!(matches!(
            result,
            Intersections::One(point) if point_is_near(point, Point64::new(5.0, 0.0))
        ));
    }

    #[test]
    fn circle_circle_reports_nested_circles_without_contact() {
        let result =
            circle_circle_intersections(Point64::new(0.0, 0.0), 5.0, Point64::new(1.0, 0.0), 1.0);

        assert_eq!(result, Intersections::None);
    }

    #[test]
    fn circle_circle_reports_the_same_circle() {
        let result =
            circle_circle_intersections(Point64::new(2.0, 3.0), 5.0, Point64::new(2.0, 3.0), 5.0);

        assert_eq!(result, Intersections::Coincident);
    }

    fn right_angle_input(incoming_curvature: f64, outgoing_curvature: f64) -> JoinInput {
        JoinInput {
            at: Point64::new(0.0, 0.0),
            incoming: SegmentEnd {
                tangent: Vector64::new(1.0, 0.0),
                curvature: incoming_curvature,
            },
            outgoing: SegmentEnd {
                tangent: Vector64::new(0.0, 1.0),
                curvature: outgoing_curvature,
            },
            half_width: 2.0,
            miter_limit: 4.0,
        }
    }

    fn near(actual: f64, expected: f64) -> bool {
        (actual - expected).abs() <= TEST_EPSILON
    }

    fn point_is_near(actual: Point64, expected: Point64) -> bool {
        near(actual.x, expected.x) && near(actual.y, expected.y)
    }

    fn two_points_are_near(
        result: Intersections,
        expected_a: Point64,
        expected_b: Point64,
    ) -> bool {
        let Intersections::Two([actual_a, actual_b]) = result else {
            return false;
        };

        (point_is_near(actual_a, expected_a) && point_is_near(actual_b, expected_b))
            || (point_is_near(actual_a, expected_b) && point_is_near(actual_b, expected_a))
    }

    fn boundary_start_tangent(boundary: BoundaryPiece) -> Vector64 {
        match boundary {
            BoundaryPiece::Line { start, end } => {
                let direction = end - start;
                direction * direction.square_length().sqrt().recip()
            }
            BoundaryPiece::Arc {
                center,
                start,
                sweep,
                ..
            } => {
                let radius = start - center;
                let tangent = if sweep.counter_clockwise() {
                    radius.left_normal()
                } else {
                    -radius.left_normal()
                };
                tangent * tangent.square_length().sqrt().recip()
            }
        }
    }
}
