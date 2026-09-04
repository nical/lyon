// There's a number of cases in this file where this lint just complicates the code.
#![allow(clippy::needless_range_loop)]

use crate::geom::arrayvec::ArrayVec;
use crate::geom::utils::tangent;
use crate::geom::{CubicBezierSegment, Line, LineSegment, QuadraticBezierSegment};
use crate::math::*;
use crate::math_utils::compute_normal;
use crate::path::builder::{Build, NoAttributes, PathBuilder};
use crate::path::polygon::Polygon;
use crate::path::private::DebugValidator;
use crate::path::{
    AttributeStore, Attributes, EndpointId, IdEvent, PathEvent, PathSlice, PositionStore, Winding,
};
use crate::stroke_arcs::{
    self, JoinConstruction, JoinInput, ParallelRectangleJoin, Point64, RadialClipJoin,
    ResolvedArcsJoin, SegmentEnd, TurnSide, Vector64,
};
use crate::stroke_arcs_mesh::{ArcsMesh, ArcsOutput, ValidatedFan};
use crate::{
    LineCap, LineJoin, Side, SimpleAttributeStore, StrokeGeometryBuilder, StrokeOptions,
    TessellationError, TessellationResult, VertexId, VertexSource,
};

use core::f32::consts::PI;
use alloc::vec::Vec;

#[cfg(not(feature = "std"))]
use num_traits::Float;

const SIDE_POSITIVE: usize = 0;
const SIDE_NEGATIVE: usize = 1;

macro_rules! nan_check {
    ($($v:expr),+) => { $(debug_assert!(!$v.is_nan());)+ };
}

// TODO: the stroke tessellator's code is has a lot of duplication and a bunch of error prone parts
// such as having to know at which stage each member of SidePoints is set.
// It would be good to spend some time simplifying it.

/// A Context object that can tessellate stroke operations for complex paths.
///
/// ## Arcs joins
///
/// [`LineJoin::Arcs`] uses endpoint tangents and curvature from the original
/// quadratic or cubic segments, before flattening. It adjusts disjoint support
/// circles and clips the resulting join using the SVG 2 miter limit.
///
/// Pass the original path to preserve curvature. A pre-flattened path contains
/// only lines, so its arcs joins behave like miter-clip joins. Elliptical arcs
/// converted to Beziers use the curvature of those Beziers.
///
/// The SVG 2 excessive-curvature rule selects round joins. Numerically
/// degenerate inputs or joins that cannot be triangulated also use round joins.
/// Variable-width strokes use the width at the join; they do not account for
/// derivatives of the width profile when constructing offset supports.
///
/// ```
/// use lyon_tessellation::{LineJoin, StrokeOptions};
/// let options = StrokeOptions::default()
///     .with_line_join(LineJoin::Arcs)
///     .with_line_width(8.0)
///     .with_miter_limit(4.0);
/// ```
///
/// ## Overview
///
/// The stroke tessellation algorithm simply generates a strip of triangles along
/// the path. This method is fast and simple to implement, however it means that
/// if the path overlap with itself (for example in the case of a self-intersecting
/// path), some triangles will overlap in the intersecting region, which may not
/// be the desired behavior. This needs to be kept in mind when rendering transparent
/// SVG strokes since the spec mandates that each point along a semi-transparent path
/// is shaded once no matter how many times the path overlaps with itself at this
/// location.
///
/// `StrokeTessellator` exposes a similar interface to its
/// [fill equivalent](struct.FillTessellator.html).
///
/// This stroke tessellator takes an iterator of path events as inputs as well as
/// a [`StrokeOption`](struct.StrokeOptions.html), and produces its outputs using
/// a [`StrokeGeometryBuilder`](geometry_builder/trait.StrokeGeometryBuilder.html).
///
///
/// See the [`geometry_builder` module documentation](geometry_builder/index.html)
/// for more details about how to output custom vertex layouts.
///
/// See <https://github.com/nical/lyon/wiki/Stroke-tessellation> for some notes
/// about how the path stroke tessellator is implemented.
///
/// # Examples
///
/// ```
/// # extern crate lyon_tessellation as tess;
/// # use tess::path::Path;
/// # use tess::path::builder::*;
/// # use tess::path::iterator::*;
/// # use tess::math::*;
/// # use tess::geometry_builder::{VertexBuffers, simple_builder};
/// # use tess::*;
/// # fn main() {
/// // Create a simple path.
/// let mut path_builder = Path::builder();
/// path_builder.begin(point(0.0, 0.0));
/// path_builder.line_to(point(1.0, 2.0));
/// path_builder.line_to(point(2.0, 0.0));
/// path_builder.line_to(point(1.0, 1.0));
/// path_builder.end(true);
/// let path = path_builder.build();
///
/// // Create the destination vertex and index buffers.
/// let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
///
/// {
///     // Create the destination vertex and index buffers.
///     let mut vertex_builder = simple_builder(&mut buffers);
///
///     // Create the tessellator.
///     let mut tessellator = StrokeTessellator::new();
///
///     // Compute the tessellation.
///     tessellator.tessellate(
///         &path,
///         &StrokeOptions::default(),
///         &mut vertex_builder
///     );
/// }
///
/// println!("The generated vertices are: {:?}.", &buffers.vertices[..]);
/// println!("The generated indices are: {:?}.", &buffers.indices[..]);
///
/// # }
/// ```
#[derive(Default)]
pub struct StrokeTessellator {
    attrib_buffer: Vec<f32>,
    builder_attrib_store: SimpleAttributeStore,
    // Keep opt-in join data out of EndpointData and reuse its mesh allocations.
    arcs: ArcsState,
}

impl StrokeTessellator {
    pub fn new() -> Self {
        StrokeTessellator {
            attrib_buffer: Vec::new(),
            builder_attrib_store: SimpleAttributeStore::new(0),
            arcs: ArcsState::default(),
        }
    }

    /// Compute the tessellation from a path iterator.
    pub fn tessellate(
        &mut self,
        input: impl IntoIterator<Item = PathEvent>,
        options: &StrokeOptions,
        builder: &mut dyn StrokeGeometryBuilder,
    ) -> TessellationResult {
        self.tessellate_impl(input, options, builder)
    }

    fn tessellate_impl<Output: StrokeGeometryBuilder + ?Sized>(
        &mut self,
        input: impl IntoIterator<Item = PathEvent>,
        options: &StrokeOptions,
        builder: &mut Output,
    ) -> TessellationResult {
        debug_assert!(
            options.variable_line_width.is_none(),
            "Variable line width requires custom attributes. Try tessellate_with_ids or tessellate_path",
        );

        let mut buffer = Vec::new();
        let builder = StrokeBuilderImpl::new(options, &mut buffer, &mut self.arcs, builder);

        builder.tessellate_fw(input)
    }

    /// Compute the tessellation from a path iterator.
    pub fn tessellate_with_ids(
        &mut self,
        path: impl IntoIterator<Item = IdEvent>,
        positions: &impl PositionStore,
        custom_attributes: Option<&dyn AttributeStore>,
        options: &StrokeOptions,
        output: &mut dyn StrokeGeometryBuilder,
    ) -> TessellationResult {
        self.tessellate_with_ids_impl(path, positions, custom_attributes, options, output)
    }

    fn tessellate_with_ids_impl<Output: StrokeGeometryBuilder + ?Sized>(
        &mut self,
        path: impl IntoIterator<Item = IdEvent>,
        positions: &impl PositionStore,
        custom_attributes: Option<&dyn AttributeStore>,
        options: &StrokeOptions,
        output: &mut Output,
    ) -> TessellationResult {
        let custom_attributes = custom_attributes.unwrap_or(&());

        self.attrib_buffer.clear();
        for _ in 0..custom_attributes.num_attributes() {
            self.attrib_buffer.push(0.0);
        }

        let builder =
            StrokeBuilderImpl::new(options, &mut self.attrib_buffer, &mut self.arcs, output);

        builder.tessellate_with_ids(path, positions, custom_attributes)
    }

    /// Compute the tessellation from a path slice.
    ///
    /// The tessellator will internally only track vertex sources and interpolated
    /// attributes if the path has interpolated attributes.
    pub fn tessellate_path<'l>(
        &'l mut self,
        path: impl Into<PathSlice<'l>>,
        options: &'l StrokeOptions,
        builder: &'l mut dyn StrokeGeometryBuilder,
    ) -> TessellationResult {
        self.tessellate_path_impl(path.into(), options, builder)
    }

    /// Compute the tessellation from a path slice with static output dispatch.
    ///
    /// This produces the same geometry as [`StrokeTessellator::tessellate_path`]
    /// while allowing the compiler to specialize the hot stroke-output calls for
    /// the concrete builder type.
    pub fn tessellate_path_with_builder<'l, Output: StrokeGeometryBuilder>(
        &'l mut self,
        path: impl Into<PathSlice<'l>>,
        options: &'l StrokeOptions,
        builder: &'l mut Output,
    ) -> TessellationResult {
        self.tessellate_path_impl(path.into(), options, builder)
    }

    fn tessellate_path_impl<'l, Output: StrokeGeometryBuilder + ?Sized>(
        &'l mut self,
        path: PathSlice<'l>,
        options: &'l StrokeOptions,
        builder: &'l mut Output,
    ) -> TessellationResult {
        if path.num_attributes() > 0 {
            self.tessellate_with_ids_impl(path.id_iter(), &path, Some(&path), options, builder)
        } else {
            self.tessellate_impl(path.iter(), options, builder)
        }
    }

    /// Tessellate directly from a sequence of `PathBuilder` commands, without
    /// creating an intermediate path data structure.
    ///
    /// The returned builder implements the [`lyon_path::traits::PathBuilder`] trait,
    /// is compatible with the all `PathBuilder` adapters.
    /// It also has all requirements documented in `PathBuilder` (All sub-paths must be
    /// wrapped in a `begin`/`end` pair).
    ///
    /// # Example
    ///
    /// ```rust
    /// use lyon_tessellation::{StrokeTessellator, StrokeOptions};
    /// use lyon_tessellation::geometry_builder::{simple_builder, VertexBuffers};
    /// use lyon_tessellation::math::{Point, point};
    ///
    /// let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
    /// let mut vertex_builder = simple_builder(&mut buffers);
    /// let mut tessellator = StrokeTessellator::new();
    /// let options = StrokeOptions::default();
    ///
    /// // Create a temporary builder (borrows the tessellator).
    /// let mut builder = tessellator.builder(&options, &mut vertex_builder);
    ///
    /// // Build the path directly in the tessellator, skipping an intermediate data
    /// // structure.
    /// builder.begin(point(0.0, 0.0));
    /// builder.line_to(point(10.0, 0.0));
    /// builder.line_to(point(10.0, 10.0));
    /// builder.line_to(point(0.0, 10.0));
    /// builder.end(true);
    ///
    /// // Finish the tessellation and get the result.
    /// let result = builder.build();
    /// ```
    ///
    /// [`lyon_path::traits::PathBuilder`]: https://docs.rs/lyon_path/*/lyon_path/traits/trait.PathBuilder.html
    pub fn builder<'l>(
        &'l mut self,
        options: &'l StrokeOptions,
        output: &'l mut dyn StrokeGeometryBuilder,
    ) -> NoAttributes<StrokeBuilder<'l>> {
        self.builder_attrib_store.reset(0);
        self.attrib_buffer.clear();
        NoAttributes::wrap(StrokeBuilder::new(
            options,
            &mut self.attrib_buffer,
            &mut self.builder_attrib_store,
            &mut self.arcs,
            output,
        ))
    }

    /// Tessellate directly from a sequence of `PathBuilder` commands, without
    /// creating an intermediate path data structure.
    ///
    /// Similar to `StrokeTessellator::builder` with custom attributes.
    pub fn builder_with_attributes<'l>(
        &'l mut self,
        num_attributes: usize,
        options: &'l StrokeOptions,
        output: &'l mut dyn StrokeGeometryBuilder,
    ) -> StrokeBuilder<'l> {
        self.builder_attrib_store.reset(num_attributes);
        self.attrib_buffer.clear();
        for _ in 0..num_attributes {
            self.attrib_buffer.push(0.0);
        }

        StrokeBuilder::new(
            options,
            &mut self.attrib_buffer,
            &mut self.builder_attrib_store,
            &mut self.arcs,
            output,
        )
    }

    /// Tessellate the stroke for a `Polygon`.
    pub fn tessellate_polygon(
        &mut self,
        polygon: Polygon<Point>,
        options: &StrokeOptions,
        output: &mut dyn StrokeGeometryBuilder,
    ) -> TessellationResult {
        self.tessellate(polygon.path_events(), options, output)
    }

    /// Tessellate the stroke for an axis-aligned rectangle.
    pub fn tessellate_rectangle(
        &mut self,
        rect: &Box2D,
        options: &StrokeOptions,
        output: &mut dyn StrokeGeometryBuilder,
    ) -> TessellationResult {
        assert!(options.variable_line_width.is_none());

        let mut builder = self.builder(options, output);
        builder.add_rectangle(rect, Winding::Positive);

        builder.build()
    }

    /// Tessellate the stroke for a circle.
    pub fn tessellate_circle(
        &mut self,
        center: Point,
        radius: f32,
        options: &StrokeOptions,
        output: &mut dyn StrokeGeometryBuilder,
    ) -> TessellationResult {
        let mut builder = self.builder(options, output);
        builder.add_circle(center, radius, Winding::Positive);

        builder.build()
    }

    /// Tessellate the stroke for an ellipse.
    pub fn tessellate_ellipse(
        &mut self,
        center: Point,
        radii: Vector,
        x_rotation: Angle,
        winding: Winding,
        options: &StrokeOptions,
        output: &mut dyn StrokeGeometryBuilder,
    ) -> TessellationResult {
        let mut builder = self.builder(options, output);
        builder.add_ellipse(center, radii, x_rotation, winding);

        builder.build()
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct SidePoints {
    prev: Point,
    next: Point,
    single_vertex: Option<Point>,
    prev_vertex: VertexId,
    next_vertex: VertexId,
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct EndpointData {
    pub position: Point,
    pub half_width: f32,
    pub advancement: f32,
    pub line_join: LineJoin,
    pub src: VertexSource,
    pub side_points: [SidePoints; 2],
    pub fold: [bool; 2],
    pub is_flattening_step: bool,
}

impl Default for EndpointData {
    fn default() -> Self {
        EndpointData {
            position: Point::zero(),
            half_width: f32::NAN,
            advancement: f32::NAN,
            line_join: LineJoin::Miter,
            src: VertexSource::Endpoint {
                id: EndpointId::INVALID,
            },
            side_points: [SidePoints {
                prev: point(f32::NAN, f32::NAN),
                prev_vertex: VertexId(u32::MAX),
                next: point(f32::NAN, f32::NAN),
                next_vertex: VertexId(u32::MAX),
                single_vertex: None,
            }; 2],
            fold: [false, false],
            is_flattening_step: false,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum EndpointDifferential {
    None,
    Regular {
        unit_tangent: Vector,
        curvature: f64,
    },
    Degenerate,
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
struct EndpointDifferentials {
    incoming: EndpointDifferential,
    outgoing: EndpointDifferential,
}

impl Default for EndpointDifferential {
    fn default() -> Self {
        Self::None
    }
}

struct ArcsState {
    point_buffer: PointBuffer<EndpointDifferentials>,
    firsts: ArrayVec<EndpointDifferentials, 2>,
    next: EndpointDifferentials,
    mesh: ArcsMesh,
    vertex_ids: Vec<VertexId>,
}

impl Default for ArcsState {
    fn default() -> Self {
        Self {
            point_buffer: PointBuffer::new(),
            firsts: ArrayVec::new(),
            next: EndpointDifferentials::default(),
            mesh: ArcsMesh::default(),
            vertex_ids: Vec::new(),
        }
    }
}

impl ArcsState {
    fn reset_differentials(&mut self, point_count: usize) {
        self.point_buffer.clear();
        self.firsts.clear();
        self.next = EndpointDifferentials::default();
        for _ in 0..point_count {
            self.point_buffer.push(EndpointDifferentials::default());
        }
    }

    fn record_segment(&mut self, differentials: SegmentDifferentials) {
        self.point_buffer.last_mut().outgoing = differentials.start;
        self.next.incoming = differentials.end;
    }

    fn push_next(&mut self, is_endpoint: bool) {
        let next = self.take_next(is_endpoint);
        self.point_buffer.push(next);
    }

    fn replace_last_with_next(&mut self, is_endpoint: bool) {
        let next = self.take_next(is_endpoint);
        self.point_buffer.replace_last(next);
    }

    fn take_next(&mut self, is_endpoint: bool) -> EndpointDifferentials {
        if !is_endpoint {
            return EndpointDifferentials::default();
        }

        let next = self.next;
        self.next = EndpointDifferentials::default();
        next
    }

    fn capture_firsts(&mut self) {
        let (prev, join) = self.point_buffer.last_two();
        self.firsts.push(*prev);
        self.firsts.push(*join);
    }

    fn prepare_first_for_close(&mut self, last_position: Point, first_position: Point) {
        let mut first = self.firsts[0];
        if last_position == first_position {
            self.point_buffer.last_mut().outgoing = first.outgoing;
        } else {
            let closing = line_differentials(last_position, first_position);
            self.point_buffer.last_mut().outgoing = closing.start;
            first.incoming = closing.end;
        }
        self.next = first;
    }

    fn prepare_second_for_close(&mut self) {
        self.next = self.firsts[1];
    }

    fn clear(&mut self) {
        self.reset_differentials(0);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ArcsJoinFallback {
    Round,
}

#[allow(clippy::large_enum_variant)] // Join preparation must not allocate in the stroke hot path.
enum PreparedArcsJoin {
    Curved(ResolvedArcsJoin),
    ParallelRectangle(PreparedParallelRectangle),
    RadialClip(PreparedRadialClip),
}

#[cfg(not(test))]
const _: () = assert!(core::mem::size_of::<PreparedArcsJoin>() <= 152);

#[derive(Copy, Clone)]
struct PreparedParallelRectangle {
    near_left: Point,
    near_right: Point,
    far_left: Point,
    far_right: Point,
}

#[derive(Copy, Clone)]
struct PreparedRadialClip {
    turn: TurnSide,
    incoming_offset_point: Point,
    outgoing_offset_point: Point,
    incoming: Point,
    outgoing: Point,
}

#[derive(Copy, Clone)]
struct SegmentDifferentials {
    start: EndpointDifferential,
    end: EndpointDifferential,
}

/// A builder object that tessellates a stroked path via the `PathBuilder`
/// interface.
///
/// Can be created using `StrokeTessellator::builder_with_attributes`.
pub struct StrokeBuilder<'l> {
    builder: StrokeBuilderImpl<'l, dyn StrokeGeometryBuilder + 'l>,
    attrib_store: &'l mut SimpleAttributeStore,
    validator: DebugValidator,
    prev: (Point, EndpointId, f32),
}

impl<'l> StrokeBuilder<'l> {
    fn new(
        options: &StrokeOptions,
        attrib_buffer: &'l mut Vec<f32>,
        attrib_store: &'l mut SimpleAttributeStore,
        arcs: &'l mut ArcsState,
        output: &'l mut dyn StrokeGeometryBuilder,
    ) -> Self {
        StrokeBuilder {
            builder: StrokeBuilderImpl::new(options, attrib_buffer, arcs, output),
            attrib_store,
            validator: DebugValidator::new(),
            prev: (Point::zero(), EndpointId::INVALID, 0.0),
        }
    }

    #[inline]
    pub fn set_line_join(&mut self, join: LineJoin) {
        self.builder.set_line_join(join);
    }

    #[inline]
    pub fn set_start_cap(&mut self, cap: LineCap) {
        self.builder.options.start_cap = cap;
    }

    #[inline]
    pub fn set_end_cap(&mut self, cap: LineCap) {
        self.builder.options.end_cap = cap;
    }

    #[inline]
    pub fn set_miter_limit(&mut self, limit: f32) {
        self.builder.options.miter_limit = limit;
    }

    fn get_width(&self, attributes: Attributes) -> f32 {
        if let Some(idx) = self.builder.options.variable_line_width {
            self.builder.options.line_width * attributes[idx]
        } else {
            self.builder.options.line_width
        }
    }
}

impl<'l> PathBuilder for StrokeBuilder<'l> {
    fn num_attributes(&self) -> usize {
        self.attrib_store.num_attributes()
    }

    fn begin(&mut self, to: Point, attributes: Attributes) -> EndpointId {
        self.validator.begin();
        let id = self.attrib_store.add(attributes);
        if let Some(attrib_index) = self.builder.options.variable_line_width {
            let width = self.builder.options.line_width * attributes[attrib_index];
            self.builder.begin(to, id, width, self.attrib_store);
            self.prev = (to, id, width);
        } else {
            self.builder.begin_fw(to, id, self.attrib_store);
            self.prev = (to, id, self.builder.options.line_width);
        }

        id
    }

    fn end(&mut self, close: bool) {
        self.validator.end();
        self.builder.end(close, self.attrib_store);
    }

    fn line_to(&mut self, to: Point, attributes: Attributes) -> EndpointId {
        let id = self.attrib_store.add(attributes);
        self.validator.edge();
        if let Some(attrib_index) = self.builder.options.variable_line_width {
            let width = self.builder.options.line_width * attributes[attrib_index];
            self.builder.line_to(to, id, width, self.attrib_store);
            self.prev = (to, id, width);
        } else {
            self.builder.line_to_fw(to, id, self.attrib_store);
            self.prev = (to, id, self.builder.options.line_width);
        }

        id
    }

    fn quadratic_bezier_to(
        &mut self,
        ctrl: Point,
        to: Point,
        attributes: Attributes,
    ) -> EndpointId {
        self.validator.edge();
        let (from, from_id, start_width) = self.prev;
        let to_id = self.attrib_store.add(attributes);

        let curve = QuadraticBezierSegment { from, ctrl, to };

        if let Some(attrib_index) = self.builder.options.variable_line_width {
            let end_width = self.builder.options.line_width * attributes[attrib_index];
            self.builder.quadratic_bezier_to(
                &curve,
                from_id,
                to_id,
                start_width,
                end_width,
                self.attrib_store,
            );

            self.prev = (to, to_id, end_width);
        } else {
            self.builder
                .quadratic_bezier_to_fw(&curve, from_id, to_id, self.attrib_store);

            self.prev = (to, to_id, self.builder.options.line_width);
        }

        to_id
    }

    fn cubic_bezier_to(
        &mut self,
        ctrl1: Point,
        ctrl2: Point,
        to: Point,
        attributes: Attributes,
    ) -> EndpointId {
        self.validator.edge();
        let (from, from_id, start_width) = self.prev;
        let to_id = self.attrib_store.add(attributes);

        let curve = CubicBezierSegment {
            from,
            ctrl1,
            ctrl2,
            to,
        };

        if let Some(attrib_index) = self.builder.options.variable_line_width {
            let end_width = self.builder.options.line_width * attributes[attrib_index];
            self.builder.cubic_bezier_to(
                &curve,
                from_id,
                to_id,
                start_width,
                end_width,
                self.attrib_store,
            );

            self.prev = (to, to_id, end_width);
        } else {
            self.builder
                .cubic_bezier_to_fw(&curve, from_id, to_id, self.attrib_store);

            self.prev = (to, to_id, self.builder.options.line_width);
        }

        to_id
    }

    fn add_rectangle(&mut self, rect: &Box2D, winding: Winding, attributes: Attributes) {
        // The thin rectangle approximation for works best with miter joins. We
        // only use it with other joins if the rectangle is much smaller than the
        // line width.
        let threshold = match self.builder.options.line_join {
            LineJoin::Miter => 1.0,
            _ => 0.05,
        } * self.builder.options.line_width;

        if self.builder.options.variable_line_width.is_none()
            && (rect.width().abs() < threshold || rect.height().abs() < threshold)
        {
            approximate_thin_rectangle(self, rect, attributes);
            return;
        }

        match winding {
            Winding::Positive => self.add_polygon(
                Polygon {
                    points: &[
                        rect.min,
                        point(rect.max.x, rect.min.y),
                        rect.max,
                        point(rect.min.x, rect.max.y),
                    ],
                    closed: true,
                },
                attributes,
            ),
            Winding::Negative => self.add_polygon(
                Polygon {
                    points: &[
                        rect.min,
                        point(rect.min.x, rect.max.y),
                        rect.max,
                        point(rect.max.x, rect.min.y),
                    ],
                    closed: true,
                },
                attributes,
            ),
        };
    }
}

impl<'l> Build for StrokeBuilder<'l> {
    type PathType = TessellationResult;

    fn build(self) -> TessellationResult {
        self.builder.build()
    }
}

/// A builder that tessellates a stroke directly without allocating any intermediate data structure.
pub(crate) struct StrokeBuilderImpl<'l, Output: StrokeGeometryBuilder + ?Sized> {
    options: StrokeOptions,
    pub(crate) error: Option<TessellationError>,
    pub(crate) output: &'l mut Output,
    vertex: StrokeVertexData<'l>,
    point_buffer: PointBuffer,
    firsts: ArrayVec<EndpointData, 2>,
    previous: Option<EndpointData>,
    sub_path_start_advancement: f32,
    square_merge_threshold: f32,
    may_need_empty_cap: bool,
    arcs: &'l mut ArcsState,
    arcs_differentials_enabled: bool,
}

impl<'l, Output: StrokeGeometryBuilder + ?Sized> StrokeBuilderImpl<'l, Output> {
    fn new(
        options: &StrokeOptions,
        attrib_buffer: &'l mut Vec<f32>,
        arcs: &'l mut ArcsState,
        output: &'l mut Output,
    ) -> Self {
        output.begin_geometry();

        let arcs_differentials_enabled = options.line_join == LineJoin::Arcs;
        if arcs_differentials_enabled {
            arcs.reset_differentials(0);
        }

        // Ideally we'd use the bounding rect of the path as an indication
        // of what is considered a very small distance between two points,
        // but we don't have this information so we use a combination of the
        // tolerance threshold and, in case the latter is high to get "low-poly"
        // curves, the line width.
        let square_merge_threshold = (options.tolerance * options.tolerance * 0.5)
            .min(options.line_width * options.line_width * 0.05)
            .max(1e-8);

        let zero = Point::new(0.0, 0.0);
        StrokeBuilderImpl {
            options: *options,
            error: None,
            output,
            vertex: StrokeVertexData {
                position_on_path: zero,
                normal: vector(0.0, 0.0),
                half_width: options.line_width * 0.5,
                advancement: 0.0,
                buffer: attrib_buffer,
                side: Side::Negative,
                src: VertexSource::Endpoint {
                    id: EndpointId::INVALID,
                },
                buffer_is_valid: false,
            },
            point_buffer: PointBuffer::new(),
            firsts: ArrayVec::new(),
            previous: None,
            sub_path_start_advancement: 0.0,
            square_merge_threshold,
            may_need_empty_cap: false,
            arcs,
            arcs_differentials_enabled,
        }
    }

    #[cold]
    pub(crate) fn error<E: Into<TessellationError>>(&mut self, e: E) {
        if self.error.is_none() {
            self.error = Some(e.into());
        }
    }

    fn set_line_join(&mut self, join: LineJoin) {
        if join == LineJoin::Arcs && !self.arcs_differentials_enabled {
            self.arcs.reset_differentials(self.point_buffer.count());
            self.arcs_differentials_enabled = true;
        }
        self.options.line_join = join;
    }

    fn record_segment_differentials(&mut self, compute: impl FnOnce() -> SegmentDifferentials) {
        if self.arcs_differentials_enabled {
            self.arcs.record_segment(compute());
        }
    }

    fn record_closing_differentials(&mut self, first_position: Point) {
        if self.arcs_differentials_enabled {
            self.arcs
                .prepare_first_for_close(self.point_buffer.last().position, first_position);
        }
    }

    pub(crate) fn tessellate_with_ids(
        self,
        path: impl IntoIterator<Item = IdEvent>,
        positions: &impl PositionStore,
        attributes: &dyn AttributeStore,
    ) -> TessellationResult {
        if self.options.variable_line_width.is_some() {
            self.tessellate_with_ids_vw(path, positions, attributes)
        } else {
            self.tessellate_with_ids_fw(path, positions, attributes)
        }
    }

    fn tessellate_with_ids_vw(
        mut self,
        path: impl IntoIterator<Item = IdEvent>,
        positions: &impl PositionStore,
        attributes: &dyn AttributeStore,
    ) -> TessellationResult {
        let base_width = self.options.line_width;
        let attrib_index = self.options.variable_line_width.unwrap();

        let mut validator = DebugValidator::new();

        let mut current_endpoint = EndpointId(u32::MAX);
        let mut current_position = point(f32::NAN, f32::NAN);

        for evt in path.into_iter() {
            match evt {
                IdEvent::Begin { at } => {
                    validator.begin();
                    let half_width = base_width * attributes.get(at)[attrib_index] * 0.5;
                    current_endpoint = at;
                    current_position = positions.get_endpoint(at);
                    self.may_need_empty_cap = false;
                    self.step(
                        EndpointData {
                            position: current_position,
                            half_width,
                            advancement: self.sub_path_start_advancement,
                            line_join: self.options.line_join,
                            src: VertexSource::Endpoint { id: at },
                            ..Default::default()
                        },
                        attributes,
                    );
                }
                IdEvent::Line { to, .. } => {
                    validator.edge();
                    let half_width = base_width * attributes.get(to)[attrib_index] * 0.5;
                    let from_position = current_position;
                    current_endpoint = to;
                    current_position = positions.get_endpoint(to);
                    self.record_segment_differentials(|| {
                        line_differentials(from_position, current_position)
                    });
                    self.step(
                        EndpointData {
                            position: current_position,
                            half_width,
                            line_join: self.options.line_join,
                            src: VertexSource::Endpoint { id: to },
                            ..Default::default()
                        },
                        attributes,
                    );
                }
                IdEvent::Quadratic { ctrl, to, .. } => {
                    validator.edge();
                    let start_width = base_width * attributes.get(current_endpoint)[attrib_index];
                    let end_width = base_width * attributes.get(to)[attrib_index];

                    let from = current_endpoint;
                    let from_pos = current_position;
                    current_endpoint = to;
                    current_position = positions.get_endpoint(to);

                    self.quadratic_bezier_to(
                        &QuadraticBezierSegment {
                            from: from_pos,
                            ctrl: positions.get_control_point(ctrl),
                            to: current_position,
                        },
                        from,
                        to,
                        start_width,
                        end_width,
                        attributes,
                    );
                }
                IdEvent::Cubic {
                    ctrl1, ctrl2, to, ..
                } => {
                    validator.edge();

                    let start_width = base_width * attributes.get(current_endpoint)[attrib_index];
                    let end_width = base_width * attributes.get(to)[attrib_index];

                    let from = current_endpoint;
                    let from_pos = current_position;
                    current_endpoint = to;
                    current_position = positions.get_endpoint(to);

                    self.cubic_bezier_to(
                        &CubicBezierSegment {
                            from: from_pos,
                            ctrl1: positions.get_control_point(ctrl1),
                            ctrl2: positions.get_control_point(ctrl2),
                            to: current_position,
                        },
                        from,
                        to,
                        start_width,
                        end_width,
                        attributes,
                    );
                }
                IdEvent::End { close, .. } => {
                    validator.end();
                    self.end(close, attributes);
                }
            }

            if let Some(err) = self.error {
                self.output.abort_geometry();
                return Err(err);
            }
        }

        validator.build();
        self.build()
    }

    fn tessellate_with_ids_fw(
        mut self,
        path: impl IntoIterator<Item = IdEvent>,
        positions: &impl PositionStore,
        attributes: &dyn AttributeStore,
    ) -> TessellationResult {
        let mut validator = DebugValidator::new();

        let mut current_endpoint = EndpointId(u32::MAX);
        let mut current_position = point(f32::NAN, f32::NAN);

        let half_width = self.options.line_width * 0.5;

        for evt in path.into_iter() {
            match evt {
                IdEvent::Begin { at } => {
                    validator.begin();
                    current_endpoint = at;
                    current_position = positions.get_endpoint(at);
                    self.may_need_empty_cap = false;
                    self.fixed_width_step(
                        EndpointData {
                            position: current_position,
                            half_width,
                            advancement: self.sub_path_start_advancement,
                            line_join: self.options.line_join,
                            src: VertexSource::Endpoint { id: at },
                            ..Default::default()
                        },
                        attributes,
                    );
                }
                IdEvent::Line { to, .. } => {
                    validator.edge();
                    let from_position = current_position;
                    current_endpoint = to;
                    current_position = positions.get_endpoint(to);
                    self.record_segment_differentials(|| {
                        line_differentials(from_position, current_position)
                    });
                    self.fixed_width_step(
                        EndpointData {
                            position: current_position,
                            half_width,
                            line_join: self.options.line_join,
                            src: VertexSource::Endpoint { id: to },
                            ..Default::default()
                        },
                        attributes,
                    );
                }
                IdEvent::Quadratic { ctrl, to, .. } => {
                    validator.edge();
                    let from = current_endpoint;
                    let from_pos = current_position;
                    current_endpoint = to;
                    current_position = positions.get_endpoint(to);

                    self.quadratic_bezier_to_fw(
                        &QuadraticBezierSegment {
                            from: from_pos,
                            ctrl: positions.get_control_point(ctrl),
                            to: current_position,
                        },
                        from,
                        to,
                        attributes,
                    );
                }
                IdEvent::Cubic {
                    ctrl1, ctrl2, to, ..
                } => {
                    validator.edge();
                    let from = current_endpoint;
                    let from_pos = current_position;
                    current_endpoint = to;
                    current_position = positions.get_endpoint(to);

                    self.cubic_bezier_to_fw(
                        &CubicBezierSegment {
                            from: from_pos,
                            ctrl1: positions.get_control_point(ctrl1),
                            ctrl2: positions.get_control_point(ctrl2),
                            to: current_position,
                        },
                        from,
                        to,
                        attributes,
                    );
                }
                IdEvent::End { close, .. } => {
                    validator.end();
                    self.end(close, attributes);
                }
            }

            if let Some(err) = self.error {
                self.output.abort_geometry();
                return Err(err);
            }
        }

        validator.build();
        self.build()
    }

    /// Compute the tessellation from a path iterator.
    pub(crate) fn tessellate_fw(
        mut self,
        input: impl IntoIterator<Item = PathEvent>,
    ) -> TessellationResult {
        // Ensure we use the fixed line width code paths since we don't have
        // custom attributes to get the line width from;
        self.options.variable_line_width = None;

        let mut validator = DebugValidator::new();

        let mut id = EndpointId(0);
        let mut current_position = point(f32::NAN, f32::NAN);

        for evt in input {
            match evt {
                PathEvent::Begin { at } => {
                    validator.begin();
                    current_position = at;
                    self.begin_fw(at, id, &());
                    id.0 += 1;
                }
                PathEvent::Line { to, .. } => {
                    validator.edge();
                    current_position = to;
                    self.line_to_fw(to, id, &());
                    id.0 += 1;
                }
                PathEvent::Quadratic { ctrl, to, .. } => {
                    validator.edge();

                    let from = current_position;
                    current_position = to;
                    let prev_id = EndpointId(id.0 - 1);

                    self.quadratic_bezier_to_fw(
                        &QuadraticBezierSegment { from, ctrl, to },
                        prev_id,
                        id,
                        &(),
                    );

                    id.0 += 1;
                }
                PathEvent::Cubic {
                    ctrl1, ctrl2, to, ..
                } => {
                    validator.edge();
                    let prev_id = EndpointId(id.0 - 1);

                    let from = current_position;
                    current_position = to;

                    self.cubic_bezier_to_fw(
                        &CubicBezierSegment {
                            from,
                            ctrl1,
                            ctrl2,
                            to,
                        },
                        prev_id,
                        id,
                        &(),
                    );

                    id.0 += 1;
                }
                PathEvent::End { close, .. } => {
                    validator.end();
                    self.end(close, &());
                }
            }

            if let Some(err) = self.error {
                self.output.abort_geometry();
                return Err(err);
            }
        }

        validator.build();
        self.build()
    }

    pub(crate) fn begin(
        &mut self,
        position: Point,
        endpoint: EndpointId,
        width: f32,
        attributes: &dyn AttributeStore,
    ) {
        self.may_need_empty_cap = false;
        let half_width = width * 0.5;
        self.step(
            EndpointData {
                position,
                half_width,
                advancement: self.sub_path_start_advancement,
                line_join: self.options.line_join,
                src: VertexSource::Endpoint { id: endpoint },
                ..Default::default()
            },
            attributes,
        );
    }

    pub(crate) fn line_to(
        &mut self,
        to: Point,
        endpoint: EndpointId,
        width: f32,
        attributes: &dyn AttributeStore,
    ) {
        let half_width = width * 0.5;
        let from = self.point_buffer.last().position;
        self.record_segment_differentials(|| line_differentials(from, to));
        self.step(
            EndpointData {
                position: to,
                half_width,
                line_join: self.options.line_join,
                src: VertexSource::Endpoint { id: endpoint },
                ..Default::default()
            },
            attributes,
        );
    }

    pub(crate) fn quadratic_bezier_to(
        &mut self,
        curve: &QuadraticBezierSegment<f32>,
        from_id: EndpointId,
        to_id: EndpointId,
        start_width: f32,
        end_width: f32,
        attributes: &dyn AttributeStore,
    ) {
        self.record_segment_differentials(|| quadratic_differentials(curve));
        flatten_quad(
            curve,
            self.options.tolerance,
            &mut |position, t, is_flattening_step| {
                let src = if t == 1.0 {
                    VertexSource::Endpoint { id: to_id }
                } else {
                    VertexSource::Edge {
                        from: from_id,
                        to: to_id,
                        t,
                    }
                };

                self.step(
                    EndpointData {
                        position,
                        half_width: (start_width * (1.0 - t) + end_width * t) * 0.5,
                        line_join: self.options.line_join,
                        src,
                        is_flattening_step,
                        ..Default::default()
                    },
                    attributes,
                );
            },
        );
    }

    pub(crate) fn cubic_bezier_to(
        &mut self,
        curve: &CubicBezierSegment<f32>,
        from_id: EndpointId,
        to_id: EndpointId,
        start_width: f32,
        end_width: f32,
        attributes: &dyn AttributeStore,
    ) {
        self.record_segment_differentials(|| cubic_differentials(curve));
        curve.for_each_flattened_with_t(self.options.tolerance, &mut |line, t| {
            let is_flattening_step = t.end != 1.0;
            let src = if is_flattening_step {
                VertexSource::Edge {
                    from: from_id,
                    to: to_id,
                    t: t.end,
                }
            } else {
                VertexSource::Endpoint { id: to_id }
            };

            self.step(
                EndpointData {
                    position: line.to,
                    half_width: (start_width * (1.0 - t.end) + end_width * t.end) * 0.5,
                    line_join: self.options.line_join,
                    src,
                    is_flattening_step,
                    ..Default::default()
                },
                attributes,
            );
        });
    }

    pub(crate) fn begin_fw(
        &mut self,
        position: Point,
        endpoint: EndpointId,
        attributes: &dyn AttributeStore,
    ) {
        self.may_need_empty_cap = false;
        self.fixed_width_step(
            EndpointData {
                position,
                half_width: self.options.line_width * 0.5,
                advancement: self.sub_path_start_advancement,
                line_join: self.options.line_join,
                src: VertexSource::Endpoint { id: endpoint },
                ..Default::default()
            },
            attributes,
        );
    }

    pub(crate) fn line_to_fw(
        &mut self,
        to: Point,
        endpoint: EndpointId,
        attributes: &dyn AttributeStore,
    ) {
        let half_width = self.options.line_width * 0.5;
        let from = self.point_buffer.last().position;
        self.record_segment_differentials(|| line_differentials(from, to));
        self.fixed_width_step(
            EndpointData {
                position: to,
                half_width,
                line_join: self.options.line_join,
                src: VertexSource::Endpoint { id: endpoint },
                ..Default::default()
            },
            attributes,
        );
    }

    pub(crate) fn quadratic_bezier_to_fw(
        &mut self,
        curve: &QuadraticBezierSegment<f32>,
        from_id: EndpointId,
        to_id: EndpointId,
        attributes: &dyn AttributeStore,
    ) {
        let half_width = self.options.line_width * 0.5;
        self.record_segment_differentials(|| quadratic_differentials(curve));
        flatten_quad(
            curve,
            self.options.tolerance,
            &mut |position, t, is_flattening_step| {
                let src = if t == 1.0 {
                    VertexSource::Endpoint { id: to_id }
                } else {
                    VertexSource::Edge {
                        from: from_id,
                        to: to_id,
                        t,
                    }
                };

                self.fixed_width_step(
                    EndpointData {
                        position,
                        half_width,
                        line_join: self.options.line_join,
                        src,
                        is_flattening_step,
                        ..Default::default()
                    },
                    attributes,
                );
            },
        );
    }

    pub(crate) fn cubic_bezier_to_fw(
        &mut self,
        curve: &CubicBezierSegment<f32>,
        from_id: EndpointId,
        to_id: EndpointId,
        attributes: &dyn AttributeStore,
    ) {
        let half_width = self.options.line_width * 0.5;
        self.record_segment_differentials(|| cubic_differentials(curve));
        curve.for_each_flattened_with_t(self.options.tolerance, &mut |line, t| {
            let is_flattening_step = t.end != 1.0;
            let src = if is_flattening_step {
                VertexSource::Edge {
                    from: from_id,
                    to: to_id,
                    t: t.end,
                }
            } else {
                VertexSource::Endpoint { id: to_id }
            };

            self.fixed_width_step(
                EndpointData {
                    position: line.to,
                    half_width,
                    line_join: self.options.line_join,
                    src,
                    is_flattening_step,
                    ..Default::default()
                },
                attributes,
            );
        });
    }

    pub(crate) fn end(&mut self, close: bool, attributes: &dyn AttributeStore) {
        self.may_need_empty_cap |= close && self.point_buffer.count() == 1;
        let e = if close && self.point_buffer.count() > 2 {
            self.close(attributes)
        } else {
            self.end_with_caps(attributes)
        };

        if let Err(e) = e {
            self.error(e);
        }

        self.point_buffer.clear();
        self.firsts.clear();
        if self.arcs_differentials_enabled {
            self.arcs.clear();
        }
    }

    pub(crate) fn build(self) -> TessellationResult {
        if let Some(err) = self.error {
            self.output.abort_geometry();
            return Err(err);
        }

        self.output.end_geometry();

        Ok(())
    }

    fn close(&mut self, attributes: &dyn AttributeStore) -> Result<(), TessellationError> {
        if self.point_buffer.count() == 1 {
            self.tessellate_empty_cap(attributes)?;
        }

        if self.point_buffer.count() <= 2 {
            return Ok(());
        }

        assert!(!self.firsts.is_empty());

        let mut p = self.firsts[0];
        self.record_closing_differentials(p.position);
        // Make sure we re-compute the advancement instead of using the one found at the
        // beginning of the sub-path.
        let advancement = p.advancement;
        p.advancement = f32::NAN;
        let segment_added = if self.options.variable_line_width.is_some() {
            self.step_impl(p, attributes)?
        } else {
            self.fixed_width_step_impl(p, attributes)?
        };

        if !segment_added {
            // The closing code relies on not skipping the edge from the first to
            // the second point. In most case this is ensured by points not being
            // added to self.firsts if they are not far enough apart. However there
            // could still be a situation where the last point is placed in such
            // a way that it is within merge range of both the first and second
            // points.
            // Fixing the position up ensures that even though we skip the edge to
            // the first point, we don't skip the edge to the second one.
            self.point_buffer.last_mut().position = p.position;
        }

        if self.firsts.len() >= 2 {
            let p2 = self.firsts[1];
            if self.arcs_differentials_enabled {
                self.arcs.prepare_second_for_close();
            }
            if self.options.variable_line_width.is_some() {
                self.step_impl(p2, attributes)?;
            } else {
                self.fixed_width_step_impl(p2, attributes)?;
            }

            let (p0, p1) = self.point_buffer.last_two_mut();
            // TODO: This is hacky: We re-create the first two vertices on the edge towards the second endpoint
            // so that they use the advancement value of the start of the sub path instead of the end of the
            // sub path as computed in the step_impl above.
            self.vertex.src = p0.src;
            self.vertex.position_on_path = p0.position;
            self.vertex.half_width = p0.half_width;
            self.vertex.advancement = advancement;
            self.vertex.buffer_is_valid = false;
            for side in 0..2 {
                self.vertex.side = if side == SIDE_POSITIVE {
                    Side::Positive
                } else {
                    Side::Negative
                };
                self.vertex.normal = if let Some(pos) = p0.side_points[side].single_vertex {
                    (pos - p0.position) / p0.half_width
                } else {
                    (p0.side_points[side].next - p0.position) / p0.half_width
                };

                let vertex = self
                    .output
                    .add_stroke_vertex(StrokeVertex(&mut self.vertex, attributes))?;
                p0.side_points[side].next_vertex = vertex;
            }

            add_edge_triangles(p0, p1, self.output);
        }

        Ok(())
    }

    #[cold]
    fn tessellate_empty_cap(
        &mut self,
        attributes: &dyn AttributeStore,
    ) -> Result<(), TessellationError> {
        let point = self.point_buffer.get(0);
        self.vertex.advancement = point.advancement;
        self.vertex.src = point.src;
        self.vertex.half_width = point.half_width;

        match self.options.start_cap {
            LineCap::Square => {
                // Even if there is no edge, if we are using square caps we have to place a square
                // at the current position.
                crate::stroke::tessellate_empty_square_cap(
                    point.position,
                    &mut self.vertex,
                    attributes,
                    self.output,
                )?;
            }
            LineCap::Round => {
                // Same thing for round caps.
                crate::stroke::tessellate_empty_round_cap(
                    point.position,
                    &self.options,
                    &mut self.vertex,
                    attributes,
                    self.output,
                )?;
            }
            _ => {}
        }

        Ok(())
    }

    fn points_are_too_close(&self, p0: Point, p1: Point) -> bool {
        (p0 - p1).square_length() < self.square_merge_threshold
    }

    fn end_with_caps(&mut self, attributes: &dyn AttributeStore) -> Result<(), TessellationError> {
        let count = self.point_buffer.count();

        if self.may_need_empty_cap && count == 1 {
            return self.tessellate_empty_cap(attributes);
        }

        if count >= 2 {
            // Last edge.

            // Add a fake fake point p2 aligned with p0 and p1 so that we can tessellate
            // the join for p1.
            let (p0, p1) = self.point_buffer.last_two_mut();
            let mut p0 = *p0;
            let mut p1 = *p1;

            if self.options.variable_line_width.is_none() {
                // TODO: this is a bit hacky: with the fixed width fast path we only compute the
                // side point positions for joins, so we haven't gotten to that in the case of
                // the last edge.
                let tangent = (p1.position - p0.position).normalize();
                let n = vector(-tangent.y, tangent.x) * p1.half_width;
                p1.side_points[SIDE_POSITIVE].prev = p1.position + n;
                p1.side_points[SIDE_NEGATIVE].prev = p1.position - n;
            }

            let is_first = count == 2;
            tessellate_last_edge(
                &p0,
                &mut p1,
                is_first,
                &self.options,
                &mut self.vertex,
                attributes,
                self.output,
            )?;

            self.sub_path_start_advancement = p1.advancement;

            if count > 2 {
                p0 = self.firsts[0];
                p1 = self.firsts[1];
            }

            // First edge.
            tessellate_first_edge(
                &mut p0,
                &p1,
                &self.options,
                &mut self.vertex,
                attributes,
                self.output,
            )?;
        }

        Ok(())
    }

    pub(crate) fn step_impl(
        &mut self,
        mut next: EndpointData,
        attributes: &dyn AttributeStore,
    ) -> Result<bool, TessellationError> {
        let count = self.point_buffer.count();

        debug_assert!(self.options.variable_line_width.is_some());

        if count > 0 && self.points_are_too_close(self.point_buffer.last().position, next.position)
        {
            if count == 1 {
                // move-to followed by empty segment and end of paths means we have to generate
                // an empty cap.
                self.may_need_empty_cap = true;
            }
            // TODO: should do something like:
            // - add the endpoint
            // - only allow two consecutive endpoints at the same position
            // - if the join type is round, maybe tessellate a round cap for the largest one
            // TODO: we should make sure that if the next point is an endpoint and the previous
            // is on an edge, we discard the previous instead of the next (to get the correct join)
            return Ok(false);
        }

        if count > 0 {
            let join = self.point_buffer.last_mut();
            // Compute the position of the vertices that act as reference the edge between
            // p0 and next
            if !join.is_flattening_step || !next.is_flattening_step {
                compute_edge_attachment_positions(join, &mut next);
            }
        }

        let mut skip = false;
        if count > 1 {
            let (prev, join) = self.point_buffer.last_two_mut();
            nan_check!(join.advancement);
            nan_check!(prev.advancement);

            self.vertex.src = join.src;
            self.vertex.position_on_path = join.position;
            self.vertex.half_width = join.half_width;
            self.vertex.advancement = join.advancement;
            self.vertex.buffer_is_valid = false;
            // We can take the fast path if the join is a flattening step and
            // not at a sharp turn.
            let fast_path = if join.is_flattening_step {
                let v0 = join.position - prev.position;
                let v1 = next.position - join.position;
                v0.dot(v1) > 0.0
            } else {
                false
            };
            let mut arcs_join = None;
            if fast_path {
                join.line_join = LineJoin::Miter;
                // can fast-path.
                skip = flattened_step(
                    prev,
                    join,
                    &mut next,
                    &mut self.vertex,
                    attributes,
                    self.output,
                )?;
            } else {
                if join.line_join == LineJoin::Arcs {
                    debug_assert!(self.arcs_differentials_enabled);
                    let differentials = *self.arcs.point_buffer.last();
                    arcs_join = prepare_arcs_join(join, differentials, &self.options);
                }
                compute_join_side_positions(
                    prev,
                    join,
                    &next,
                    self.options.miter_limit,
                    SIDE_POSITIVE,
                );
                compute_join_side_positions(
                    prev,
                    join,
                    &next,
                    self.options.miter_limit,
                    SIDE_NEGATIVE,
                );
                if let Some(resolved) = arcs_join.as_ref() {
                    if align_arcs_side_points(join, resolved).is_err() {
                        join.line_join = LineJoin::Round;
                        arcs_join = None;
                    }
                }

                // Prevent folding when the other side is concave.
                if join.side_points[SIDE_POSITIVE].single_vertex.is_some() {
                    join.fold[SIDE_NEGATIVE] = false;
                }
                if join.side_points[SIDE_NEGATIVE].single_vertex.is_some() {
                    join.fold[SIDE_POSITIVE] = false;
                }

                add_join_base_vertices(
                    join,
                    &mut self.vertex,
                    attributes,
                    self.output,
                    Side::Negative,
                )?;
                add_join_base_vertices(
                    join,
                    &mut self.vertex,
                    attributes,
                    self.output,
                    Side::Positive,
                )?;
            }

            if !skip {
                if count > 2 {
                    add_edge_triangles(prev, join, self.output);
                }

                tessellate_join(
                    join,
                    arcs_join.as_ref(),
                    &self.options,
                    &mut self.arcs.mesh,
                    &mut self.arcs.vertex_ids,
                    &mut self.vertex,
                    attributes,
                    self.output,
                )?;

                if count == 2 {
                    self.firsts.push(*prev);
                    self.firsts.push(*join);
                    if self.arcs_differentials_enabled {
                        self.arcs.capture_firsts();
                    }
                }
            }
        }

        if skip {
            if self.arcs_differentials_enabled {
                self.arcs.replace_last_with_next(next.src.is_endpoint());
            }
            self.point_buffer.replace_last(next);
        } else {
            if self.arcs_differentials_enabled {
                self.arcs.push_next(next.src.is_endpoint());
            }
            self.point_buffer.push(next);
        }

        Ok(true)
    }

    pub(crate) fn step(&mut self, next: EndpointData, attributes: &dyn AttributeStore) {
        if let Err(e) = self.step_impl(next, attributes) {
            self.error(e);
        }
    }

    pub(crate) fn fixed_width_step_impl(
        &mut self,
        mut next: EndpointData,
        attributes: &dyn AttributeStore,
    ) -> Result<bool, TessellationError> {
        let count = self.point_buffer.count();

        debug_assert!(self.options.variable_line_width.is_none());

        if count > 0 {
            if self.points_are_too_close(self.point_buffer.last().position, next.position) {
                if count == 1 {
                    self.may_need_empty_cap = true;
                }
                return Ok(false);
            }

            if count == 1 {
                // TODO: this is a bit hacky: with the fixed width fast path we only compute the
                // side point positions for joins but we'll need it for the first point when we get
                // back to tessellating the first edge.
                let first = self.point_buffer.last_mut();
                let edge = next.position - first.position;
                let length = edge.length();

                if next.advancement.is_nan() {
                    nan_check!(first.advancement);
                    nan_check!(length);
                    next.advancement = first.advancement + length;
                }

                let tangent = edge / length;
                let n = vector(-tangent.y, tangent.x) * next.half_width;
                first.side_points[SIDE_POSITIVE].next = first.position + n;
                first.side_points[SIDE_NEGATIVE].next = first.position - n;
            }
        }

        if count > 1 {
            let (prev, join) = self.point_buffer.last_two_mut();

            self.vertex.src = join.src;
            self.vertex.position_on_path = join.position;
            self.vertex.half_width = join.half_width;
            self.vertex.buffer_is_valid = false;
            // We can take the fast path if the join is a flattening step and
            // not at a sharp turn.
            let fast_path = if join.is_flattening_step {
                let v0 = join.position - prev.position;
                let v1 = next.position - join.position;
                v0.dot(v1) > 0.0
            } else {
                false
            };
            let mut arcs_join = None;
            if fast_path {
                join.line_join = LineJoin::Miter;
                // can fast-path.
                flattened_step(
                    prev,
                    join,
                    &mut next,
                    &mut self.vertex,
                    attributes,
                    self.output,
                )?;
            } else {
                if join.line_join == LineJoin::Arcs {
                    debug_assert!(self.arcs_differentials_enabled);
                    let differentials = *self.arcs.point_buffer.last();
                    arcs_join = prepare_arcs_join(join, differentials, &self.options);
                }
                compute_join_side_positions_fixed_width(
                    prev,
                    join,
                    &next,
                    self.options.miter_limit,
                    &mut self.vertex,
                )?;
                if let Some(resolved) = arcs_join.as_ref() {
                    if align_arcs_side_points(join, resolved).is_err() {
                        join.line_join = LineJoin::Round;
                        arcs_join = None;
                    }
                }

                add_join_base_vertices(
                    join,
                    &mut self.vertex,
                    attributes,
                    self.output,
                    Side::Negative,
                )?;
                add_join_base_vertices(
                    join,
                    &mut self.vertex,
                    attributes,
                    self.output,
                    Side::Positive,
                )?;
            }

            if count > 2 {
                add_edge_triangles(prev, join, self.output);
            }

            tessellate_join(
                join,
                arcs_join.as_ref(),
                &self.options,
                &mut self.arcs.mesh,
                &mut self.arcs.vertex_ids,
                &mut self.vertex,
                attributes,
                self.output,
            )?;

            if count == 2 {
                self.firsts.push(*prev);
                self.firsts.push(*join);
                if self.arcs_differentials_enabled {
                    self.arcs.capture_firsts();
                }
            }
        }

        if self.arcs_differentials_enabled {
            self.arcs.push_next(next.src.is_endpoint());
        }
        self.point_buffer.push(next);

        Ok(true)
    }

    pub(crate) fn fixed_width_step(&mut self, next: EndpointData, attributes: &dyn AttributeStore) {
        if let Err(e) = self.fixed_width_step_impl(next, attributes) {
            self.error(e);
        }
    }
}

fn line_differentials(from: Point, to: Point) -> SegmentDifferentials {
    let tangent = to - from;
    let differential = regular_differential(tangent, Vector::zero());
    SegmentDifferentials {
        start: differential,
        end: differential,
    }
}

fn quadratic_differentials(curve: &QuadraticBezierSegment<f32>) -> SegmentDifferentials {
    let second_derivative =
        (curve.to.to_vector() - curve.ctrl.to_vector() * 2.0 + curve.from.to_vector()) * 2.0;
    SegmentDifferentials {
        start: regular_differential((curve.ctrl - curve.from) * 2.0, second_derivative),
        end: regular_differential((curve.to - curve.ctrl) * 2.0, second_derivative),
    }
}

fn cubic_differentials(curve: &CubicBezierSegment<f32>) -> SegmentDifferentials {
    let start_second_derivative =
        (curve.from.to_vector() - curve.ctrl1.to_vector() * 2.0 + curve.ctrl2.to_vector()) * 6.0;
    let end_second_derivative =
        (curve.to.to_vector() - curve.ctrl2.to_vector() * 2.0 + curve.ctrl1.to_vector()) * 6.0;
    SegmentDifferentials {
        start: regular_differential((curve.ctrl1 - curve.from) * 3.0, start_second_derivative),
        end: regular_differential((curve.to - curve.ctrl2) * 3.0, end_second_derivative),
    }
}

pub(crate) fn regular_differential(
    tangent: Vector,
    second_derivative: Vector,
) -> EndpointDifferential {
    let tangent_x = f64::from(tangent.x);
    let tangent_y = f64::from(tangent.y);
    let speed_squared = tangent_x * tangent_x + tangent_y * tangent_y;
    if !speed_squared.is_finite() || speed_squared == 0.0 {
        return EndpointDifferential::Degenerate;
    }

    let cross =
        tangent_x * f64::from(second_derivative.y) - tangent_y * f64::from(second_derivative.x);
    let inverse_speed = speed_squared.sqrt().recip();
    let curvature = cross * inverse_speed * inverse_speed * inverse_speed;
    if !curvature.is_finite() {
        return EndpointDifferential::Degenerate;
    }

    EndpointDifferential::Regular {
        unit_tangent: Vector::new(
            (tangent_x * inverse_speed) as f32,
            (tangent_y * inverse_speed) as f32,
        ),
        curvature,
    }
}

fn compute_join_side_positions_fixed_width(
    prev: &EndpointData,
    join: &mut EndpointData,
    next: &EndpointData,
    miter_limit: f32,
    vertex: &mut StrokeVertexData,
) -> Result<(), TessellationError> {
    let prev_tangent = join.position - prev.position;
    let next_tangent = next.position - join.position;
    let prev_length = prev_tangent.length();
    let next_length = next_tangent.length();
    let prev_tangent = prev_tangent / prev_length;
    let next_tangent = next_tangent / next_length;

    if join.advancement.is_nan() {
        nan_check!(prev.advancement);
        nan_check!(prev_length);
        join.advancement = prev.advancement + prev_length;
    }
    vertex.advancement = join.advancement;

    let normal = compute_normal(prev_tangent, next_tangent);
    let (front_side, front_normal) = if prev_tangent.cross(next_tangent) >= 0.0 {
        (SIDE_NEGATIVE, -normal)
    } else {
        (SIDE_POSITIVE, normal)
    };

    let back_side = 1 - front_side;

    // The folding code path's threshold is a tad too eager when dealing with flattened curves due to
    // how flattening can create segments that are much smaller than the line width. In practice the
    // curve tends to cover the spike of the back vertex in a lot of cases. Unfortunately at the moment
    // the stroke tessellators force miter joins to be clipped in the folding case. Work around it by
    // disabling folding when a miter join is not clipped. That's often an indication that the join is not
    // sharp enough to generate a huge spike, although the spiking artifact will show up in some cases.
    // Better solutions could involve:
    //  - Using the distance between the curve control point and endpoint instead of the previous flattened
    //    segment in the spike detection heuristic.
    //  - Implement proper miter joins when the folding code is active.
    //  - Better integrating curves with the tessellator instead of only considering the previous and next
    //    flattened segment.
    let extruded_normal = front_normal * vertex.half_width;
    let unclipped_miter = (join.line_join == LineJoin::Miter
        || join.line_join == LineJoin::MiterClip)
        && !miter_limit_is_exceeded(front_normal, miter_limit);

    let mut fold = false;
    let angle_is_sharp = next_tangent.dot(prev_tangent) < 0.0;
    if !unclipped_miter && angle_is_sharp {
        // Project the back vertex on the previous and next edges and subtract the edge length
        // to see if the back vertex ends up further than the opposite endpoint of the edge.
        let d_next = extruded_normal.dot(-next_tangent) - next_length;
        let d_prev = extruded_normal.dot(prev_tangent) - prev_length;
        if d_next.min(d_prev) > 0.0 || normal.square_length() < 1e-5 {
            // Case of an overlapping stroke. In order to prevent the back vertex from creating a
            // spike outside of the stroke, we simply don't create it and we'll "fold" the join
            // instead.
            join.fold[front_side] = true;
            fold = true;
        }
    }

    let n0 = vector(-prev_tangent.y, prev_tangent.x) * vertex.half_width;
    let n1 = vector(-next_tangent.y, next_tangent.x) * vertex.half_width;
    join.side_points[SIDE_POSITIVE].prev = join.position + n0;
    join.side_points[SIDE_POSITIVE].next = join.position + n1;
    join.side_points[SIDE_NEGATIVE].prev = join.position - n0;
    join.side_points[SIDE_NEGATIVE].next = join.position - n1;

    if !fold {
        let miter_pos = [
            join.position + normal * vertex.half_width,
            join.position - normal * vertex.half_width,
        ];

        join.side_points[back_side].single_vertex = Some(miter_pos[back_side]);
        if unclipped_miter {
            join.side_points[front_side].single_vertex = Some(miter_pos[front_side]);
        } else if join.line_join == LineJoin::MiterClip {
            let n0 = join.side_points[front_side].prev - join.position;
            let n1 = join.side_points[front_side].next - join.position;
            let (prev_normal, next_normal) =
                get_clip_intersections(n0, n1, front_normal, miter_limit * vertex.half_width);
            join.side_points[front_side].prev = join.position + prev_normal;
            join.side_points[front_side].next = join.position + next_normal;
        }
    }

    Ok(())
}

// A fast path for when we know we are in a flattened curve, taking
// advantage of knowing that we don't have to handle special joins.
//
// Returning Ok(true) means we are in a weird looking case with small edges
// and varying line width causing the join to fold back. When this is the
// case we are better off skipping this join.
// "M 170 150 60 Q 215 120 240 140 2" is an example of this.
fn flattened_step(
    prev: &mut EndpointData,
    join: &mut EndpointData,
    next: &mut EndpointData,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut (impl StrokeGeometryBuilder + ?Sized),
) -> Result<bool, TessellationError> {
    let prev_edge = join.position - prev.position;
    let prev_length = prev_edge.length();
    let prev_tangent = prev_edge / prev_length;
    let next_edge = next.position - join.position;
    let next_length = next_edge.length();
    let next_tangent = next_edge / next_length;
    let normal = compute_normal(prev_tangent, next_tangent);

    if join.advancement.is_nan() {
        nan_check!(prev.advancement);
        nan_check!(prev_length);
        join.advancement = prev.advancement + prev_length;
    }

    if next.advancement.is_nan() {
        nan_check!(join.advancement);
        nan_check!(next_length);
        next.advancement = join.advancement + next_length;
    }

    vertex.advancement = join.advancement;

    let p0 = join.position + normal * vertex.half_width;
    nan_check!(p0);
    join.side_points[SIDE_POSITIVE].prev = p0;
    join.side_points[SIDE_POSITIVE].next = p0;
    join.side_points[SIDE_POSITIVE].single_vertex = Some(p0);

    let p1 = join.position - normal * vertex.half_width;
    nan_check!(p1);
    join.side_points[SIDE_NEGATIVE].prev = p1;
    join.side_points[SIDE_NEGATIVE].next = p1;
    join.side_points[SIDE_NEGATIVE].single_vertex = Some(p1);

    let v0 = p0 - prev.side_points[SIDE_POSITIVE].next;
    let v1 = p1 - prev.side_points[SIDE_NEGATIVE].next;
    if prev_edge.dot(v0) < 0.0 && prev_edge.dot(v1) < 0.0 {
        return Ok(true);
    }

    vertex.normal = normal;
    vertex.side = Side::Positive;
    let pos_vertex = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;

    vertex.normal = -normal;
    vertex.side = Side::Negative;
    let neg_vertex = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;

    join.side_points[SIDE_POSITIVE].prev_vertex = pos_vertex;
    join.side_points[SIDE_POSITIVE].next_vertex = pos_vertex;

    join.side_points[SIDE_NEGATIVE].prev_vertex = neg_vertex;
    join.side_points[SIDE_NEGATIVE].next_vertex = neg_vertex;

    Ok(false)
}

fn compute_edge_attachment_positions(p0: &mut EndpointData, p1: &mut EndpointData) {
    let edge = p1.position - p0.position;
    let d = edge.length();
    let edge_angle = edge.angle_from_x_axis().radians;

    // Extra angle produced by the varying stroke width.
    // sin(vwidth_angle) = (hw1 - hw0) / d
    let sin_vwidth_angle = (p1.half_width - p0.half_width) / d;
    let mut vwidth_angle = sin_vwidth_angle.asin();
    // If the distance between the joins (d) is smaller than either of the half
    // widths, we end up in a situation where sin_vwidth_angle is not in [-1, 1],
    // which causes vwidth_angle to be NaN. Prevent that here for safety's sake
    // but it would be better to handle that earlier to do something that looks
    // more plausible with round joins.
    if vwidth_angle.is_nan() {
        vwidth_angle = 0.0;
    }

    nan_check!(d, p0.half_width, p1.half_width, vwidth_angle);

    compute_side_attachment_positions(p0, p1, edge_angle, vwidth_angle, SIDE_POSITIVE);
    compute_side_attachment_positions(p0, p1, edge_angle, vwidth_angle, SIDE_NEGATIVE);

    if p1.advancement.is_nan() {
        nan_check!(p0.advancement, d);
        p1.advancement = p0.advancement + d;
    }
}

fn compute_side_attachment_positions(
    p0: &mut EndpointData,
    p1: &mut EndpointData,
    edge_angle: f32,
    vwidth_angle: f32,
    side: usize,
) {
    nan_check!(
        edge_angle,
        vwidth_angle,
        p0.position,
        p1.position,
        p0.half_width,
        p1.half_width
    );

    let nl = side_sign(side);

    let normal_angle = edge_angle + nl * (PI * 0.5 + vwidth_angle);
    let normal = vector(normal_angle.cos(), normal_angle.sin());

    nan_check!(normal);

    p0.side_points[side].next = p0.position + normal * p0.half_width;
    p1.side_points[side].prev = p1.position + normal * p1.half_width;

    nan_check!(p0.side_points[side].next);
    nan_check!(p1.side_points[side].prev);
}

fn add_edge_triangles(
    p0: &EndpointData,
    p1: &EndpointData,
    output: &mut (impl StrokeGeometryBuilder + ?Sized),
) {
    let mut p0_neg = p0.side_points[SIDE_NEGATIVE].next_vertex;
    let mut p0_pos = p0.side_points[SIDE_POSITIVE].next_vertex;
    let mut p1_neg = p1.side_points[SIDE_NEGATIVE].prev_vertex;
    let mut p1_pos = p1.side_points[SIDE_POSITIVE].prev_vertex;

    if p0.fold[SIDE_POSITIVE] {
        p0_neg = p0.side_points[SIDE_POSITIVE].prev_vertex;
    }
    if p0.fold[SIDE_NEGATIVE] {
        p0_pos = p0.side_points[SIDE_NEGATIVE].prev_vertex;
    }
    if p1.fold[SIDE_POSITIVE] {
        p1_neg = p1.side_points[SIDE_POSITIVE].next_vertex;
    }
    if p1.fold[SIDE_NEGATIVE] {
        p1_pos = p1.side_points[SIDE_NEGATIVE].next_vertex;
    }

    // TODO: These checks are a temporary workaround, see the issue_894 test below.
    if p0_neg == p1_pos {
        return;
    }

    if p0_neg != p0_pos && p0_pos != p1_pos {
        output.add_triangle(p0_neg, p0_pos, p1_pos);
    }

    if p0_neg != p1_neg && p1_pos != p1_neg {
        output.add_triangle(p0_neg, p1_pos, p1_neg);
    }
}

fn resolve_arcs_join(
    join: &EndpointData,
    differentials: EndpointDifferentials,
    options: &StrokeOptions,
) -> Result<JoinConstruction, ArcsJoinFallback> {
    let (
        EndpointDifferential::Regular {
            unit_tangent: incoming_tangent,
            curvature: incoming_curvature,
        },
        EndpointDifferential::Regular {
            unit_tangent: outgoing_tangent,
            curvature: outgoing_curvature,
        },
    ) = (differentials.incoming, differentials.outgoing)
    else {
        return Err(ArcsJoinFallback::Round);
    };

    let input = JoinInput {
        at: Point64::new(f64::from(join.position.x), f64::from(join.position.y)),
        incoming: SegmentEnd {
            tangent: Vector64::new(f64::from(incoming_tangent.x), f64::from(incoming_tangent.y)),
            curvature: incoming_curvature,
        },
        outgoing: SegmentEnd {
            tangent: Vector64::new(f64::from(outgoing_tangent.x), f64::from(outgoing_tangent.y)),
            curvature: outgoing_curvature,
        },
        half_width: f64::from(join.half_width),
        miter_limit: f64::from(options.miter_limit),
    };

    stroke_arcs::construct_svg2(input).map_err(|_| ArcsJoinFallback::Round)
}

fn prepare_arcs_join(
    join: &mut EndpointData,
    differentials: EndpointDifferentials,
    options: &StrokeOptions,
) -> Option<PreparedArcsJoin> {
    debug_assert_eq!(join.line_join, LineJoin::Arcs);

    let resolved = resolve_arcs_join(join, differentials, options);
    match resolved {
        Ok(JoinConstruction::Empty) => join.line_join = LineJoin::Miter,
        Ok(JoinConstruction::MiterClip) => join.line_join = LineJoin::MiterClip,
        Err(ArcsJoinFallback::Round) => {
            join.line_join = LineJoin::Round;
        }
        Ok(JoinConstruction::ParallelRectangle(rectangle)) => {
            if let Ok(rectangle) = prepare_parallel_rectangle(rectangle) {
                return Some(PreparedArcsJoin::ParallelRectangle(rectangle));
            }
            join.line_join = LineJoin::Round;
        }
        Ok(JoinConstruction::RadialClip(radial_clip)) => {
            if let Ok(radial_clip) = prepare_radial_clip(radial_clip) {
                return Some(PreparedArcsJoin::RadialClip(radial_clip));
            }
            join.line_join = LineJoin::Round;
        }
        Ok(JoinConstruction::Arcs(resolved)) => return Some(PreparedArcsJoin::Curved(resolved)),
    }

    None
}

fn prepare_radial_clip(
    radial_clip: RadialClipJoin,
) -> Result<PreparedRadialClip, ArcsJoinFallback> {
    Ok(PreparedRadialClip {
        turn: radial_clip.turn,
        incoming_offset_point: point64_to_point(radial_clip.incoming_offset_point)?,
        outgoing_offset_point: point64_to_point(radial_clip.outgoing_offset_point)?,
        incoming: point64_to_point(radial_clip.incoming)?,
        outgoing: point64_to_point(radial_clip.outgoing)?,
    })
}

fn prepare_parallel_rectangle(
    rectangle: ParallelRectangleJoin,
) -> Result<PreparedParallelRectangle, ArcsJoinFallback> {
    Ok(PreparedParallelRectangle {
        near_left: point64_to_point(rectangle.near_left)?,
        near_right: point64_to_point(rectangle.near_right)?,
        far_left: point64_to_point(rectangle.far_left)?,
        far_right: point64_to_point(rectangle.far_right)?,
    })
}

fn align_arcs_side_points(
    join: &mut EndpointData,
    prepared: &PreparedArcsJoin,
) -> Result<(), ArcsJoinFallback> {
    match prepared {
        PreparedArcsJoin::Curved(resolved) => {
            let incoming = point64_to_point(resolved.incoming_offset_point())?;
            let outgoing = point64_to_point(resolved.outgoing_offset_point())?;
            let side = arcs_outer_side(resolved.turn());
            join.side_points[side].prev = incoming;
            join.side_points[side].next = outgoing;
            join.side_points[side].single_vertex = None;
        }
        PreparedArcsJoin::ParallelRectangle(rectangle) => {
            join.side_points[SIDE_POSITIVE].prev = rectangle.near_left;
            join.side_points[SIDE_POSITIVE].next = rectangle.near_right;
            join.side_points[SIDE_POSITIVE].single_vertex = None;
            join.side_points[SIDE_NEGATIVE].prev = rectangle.near_right;
            join.side_points[SIDE_NEGATIVE].next = rectangle.near_left;
            join.side_points[SIDE_NEGATIVE].single_vertex = None;
            join.fold = [false, false];
        }
        PreparedArcsJoin::RadialClip(radial_clip) => {
            let side = arcs_outer_side(radial_clip.turn);
            join.side_points[side].prev = radial_clip.incoming_offset_point;
            join.side_points[side].next = radial_clip.outgoing_offset_point;
            join.side_points[side].single_vertex = None;
        }
    }
    Ok(())
}

fn arcs_outer_side(turn: TurnSide) -> usize {
    match turn {
        TurnSide::Left => SIDE_NEGATIVE,
        TurnSide::Right => SIDE_POSITIVE,
    }
}

fn point64_to_point(value: Point64) -> Result<Point, ArcsJoinFallback> {
    let minimum = f64::from(f32::MIN);
    let maximum = f64::from(f32::MAX);
    if !value.x.is_finite()
        || !value.y.is_finite()
        || value.x < minimum
        || value.x > maximum
        || value.y < minimum
        || value.y > maximum
    {
        return Err(ArcsJoinFallback::Round);
    }

    Ok(point(value.x as f32, value.y as f32))
}

fn tessellate_join(
    join: &mut EndpointData,
    arcs_join: Option<&PreparedArcsJoin>,
    options: &StrokeOptions,
    arcs_mesh: &mut ArcsMesh,
    arcs_vertex_ids: &mut Vec<VertexId>,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut (impl StrokeGeometryBuilder + ?Sized),
) -> Result<(), TessellationError> {
    debug_assert!(join.line_join != LineJoin::Arcs || arcs_join.is_some());

    if let Some(PreparedArcsJoin::ParallelRectangle(rectangle)) = arcs_join {
        emit_parallel_arcs_rectangle(join, rectangle, vertex, attributes, output)?;
        return Ok(());
    }

    if let Some(PreparedArcsJoin::RadialClip(radial_clip)) = arcs_join {
        if emit_radial_arcs_clip(join, radial_clip, vertex, attributes, output)? {
            return Ok(());
        }
    }

    let side_needs_join = [
        join.side_points[SIDE_POSITIVE].single_vertex.is_none() && !join.fold[SIDE_NEGATIVE],
        join.side_points[SIDE_NEGATIVE].single_vertex.is_none() && !join.fold[SIDE_POSITIVE],
    ];

    if !join.fold[SIDE_POSITIVE] && !join.fold[SIDE_NEGATIVE] {
        // Tessellate the interior of the join.
        match side_needs_join {
            [true, true] => {
                output.add_triangle(
                    join.side_points[SIDE_POSITIVE].prev_vertex,
                    join.side_points[SIDE_POSITIVE].next_vertex,
                    join.side_points[SIDE_NEGATIVE].next_vertex,
                );

                output.add_triangle(
                    join.side_points[SIDE_POSITIVE].prev_vertex,
                    join.side_points[SIDE_NEGATIVE].next_vertex,
                    join.side_points[SIDE_NEGATIVE].prev_vertex,
                );
            }
            [false, true] => {
                output.add_triangle(
                    join.side_points[SIDE_NEGATIVE].prev_vertex,
                    join.side_points[SIDE_POSITIVE].prev_vertex,
                    join.side_points[SIDE_NEGATIVE].next_vertex,
                );
            }
            [true, false] => {
                output.add_triangle(
                    join.side_points[SIDE_NEGATIVE].prev_vertex,
                    join.side_points[SIDE_POSITIVE].prev_vertex,
                    join.side_points[SIDE_POSITIVE].next_vertex,
                );
            }
            [false, false] => {}
        }
    }

    // Tessellate the remaining specific shape for convex joins
    for side in 0..2 {
        if !side_needs_join[side] {
            continue;
        }

        match join.line_join {
            LineJoin::Round => {
                tessellate_round_join(join, side, options, vertex, attributes, output)?;
            }
            LineJoin::Arcs => {
                let emitted = if let Some(PreparedArcsJoin::Curved(resolved)) = arcs_join {
                    side == arcs_outer_side(resolved.turn())
                        && emit_arcs_join(
                            join,
                            resolved,
                            side,
                            options,
                            arcs_mesh,
                            arcs_vertex_ids,
                            vertex,
                            attributes,
                            output,
                        )?
                } else {
                    false
                };
                if !emitted {
                    tessellate_round_join(join, side, options, vertex, attributes, output)?;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn emit_parallel_arcs_rectangle(
    join: &EndpointData,
    rectangle: &PreparedParallelRectangle,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut (impl StrokeGeometryBuilder + ?Sized),
) -> Result<(), TessellationError> {
    if rectangle.far_left == join.side_points[SIDE_POSITIVE].prev
        && rectangle.far_right == join.side_points[SIDE_NEGATIVE].prev
    {
        return Ok(());
    }

    vertex.position_on_path = join.position;
    vertex.half_width = join.half_width;
    vertex.side = Side::Positive;
    vertex.normal = (rectangle.far_left - join.position) / join.half_width;
    let far_left_vertex = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;
    vertex.side = Side::Negative;
    vertex.normal = (rectangle.far_right - join.position) / join.half_width;
    let far_right_vertex = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;

    let near_left_vertex = join.side_points[SIDE_POSITIVE].prev_vertex;
    let near_right_vertex = join.side_points[SIDE_NEGATIVE].prev_vertex;
    output.add_triangle(near_left_vertex, far_left_vertex, far_right_vertex);
    output.add_triangle(near_left_vertex, far_right_vertex, near_right_vertex);
    Ok(())
}

fn emit_radial_arcs_clip(
    join: &EndpointData,
    radial_clip: &PreparedRadialClip,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut (impl StrokeGeometryBuilder + ?Sized),
) -> Result<bool, TessellationError> {
    let outer_side = arcs_outer_side(radial_clip.turn);
    let inner_side = 1 - outer_side;
    if join.side_points[inner_side].single_vertex.is_none()
        || join.fold[outer_side]
        || join.fold[inner_side]
    {
        return Ok(false);
    }

    vertex.position_on_path = join.position;
    vertex.half_width = join.half_width;
    vertex.side = if outer_side == SIDE_POSITIVE {
        Side::Positive
    } else {
        Side::Negative
    };

    vertex.normal = (radial_clip.incoming - join.position) / join.half_width;
    let incoming_clip_vertex = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;
    let outgoing_clip_vertex = if radial_clip.outgoing == radial_clip.incoming {
        incoming_clip_vertex
    } else {
        vertex.normal = (radial_clip.outgoing - join.position) / join.half_width;
        output.add_stroke_vertex(StrokeVertex(vertex, attributes))?
    };

    let inner_vertex = join.side_points[inner_side].prev_vertex;
    let incoming_outer_vertex = join.side_points[outer_side].prev_vertex;
    let outgoing_outer_vertex = join.side_points[outer_side].next_vertex;
    match radial_clip.turn {
        TurnSide::Left => {
            output.add_triangle(inner_vertex, outgoing_outer_vertex, outgoing_clip_vertex);
            if outgoing_clip_vertex != incoming_clip_vertex {
                output.add_triangle(inner_vertex, outgoing_clip_vertex, incoming_clip_vertex);
            }
            output.add_triangle(inner_vertex, incoming_clip_vertex, incoming_outer_vertex);
        }
        TurnSide::Right => {
            output.add_triangle(inner_vertex, incoming_outer_vertex, incoming_clip_vertex);
            if incoming_clip_vertex != outgoing_clip_vertex {
                output.add_triangle(inner_vertex, incoming_clip_vertex, outgoing_clip_vertex);
            }
            output.add_triangle(inner_vertex, outgoing_clip_vertex, outgoing_outer_vertex);
        }
    }

    Ok(true)
}

#[allow(clippy::too_many_arguments)]
fn emit_arcs_join(
    join: &EndpointData,
    resolved: &ResolvedArcsJoin,
    side: usize,
    options: &StrokeOptions,
    mesh: &mut ArcsMesh,
    vertex_ids: &mut Vec<VertexId>,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut (impl StrokeGeometryBuilder + ?Sized),
) -> Result<bool, TessellationError> {
    let tessellation = match mesh.tessellate_for_output(resolved, f64::from(options.tolerance)) {
        Ok(tessellation) => tessellation,
        Err(_) => return Ok(false),
    };

    match tessellation {
        ArcsOutput::Triangle(triangle) => {
            let Ok(position) = point64_to_point(triangle.middle) else {
                return Ok(false);
            };
            if position == join.position {
                return Ok(false);
            }

            vertex.position_on_path = join.position;
            vertex.half_width = join.half_width;
            vertex.side = if side == SIDE_POSITIVE {
                Side::Positive
            } else {
                Side::Negative
            };
            vertex.normal = (position - join.position) / join.half_width;
            let middle_vertex = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;
            let vertex_ids = [
                join.side_points[side].prev_vertex,
                middle_vertex,
                join.side_points[side].next_vertex,
            ];
            output.add_triangle(
                vertex_ids[triangle.indices[0]],
                vertex_ids[triangle.indices[2]],
                vertex_ids[triangle.indices[1]],
            );
            Ok(true)
        }
        ArcsOutput::Quad(quad) => {
            let Ok(first_position) = point64_to_point(quad.middle[0]) else {
                return Ok(false);
            };
            let Ok(second_position) = point64_to_point(quad.middle[1]) else {
                return Ok(false);
            };
            if first_position == join.position || second_position == join.position {
                return Ok(false);
            }

            vertex.position_on_path = join.position;
            vertex.half_width = join.half_width;
            vertex.side = if side == SIDE_POSITIVE {
                Side::Positive
            } else {
                Side::Negative
            };
            vertex.normal = (first_position - join.position) / join.half_width;
            let first_middle = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;
            vertex.normal = (second_position - join.position) / join.half_width;
            let second_middle = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;
            let vertex_ids = [
                join.side_points[side].prev_vertex,
                first_middle,
                second_middle,
                join.side_points[side].next_vertex,
            ];
            for triangle in quad.indices.chunks_exact(3) {
                output.add_triangle(
                    vertex_ids[usize::from(triangle[0])],
                    vertex_ids[usize::from(triangle[2])],
                    vertex_ids[usize::from(triangle[1])],
                );
            }
            Ok(true)
        }
        ArcsOutput::Fan { vertices, fan } => emit_arcs_mesh_output(
            join,
            side,
            vertices,
            Some(fan),
            &[],
            vertex_ids,
            vertex,
            attributes,
            output,
        ),
        ArcsOutput::Mesh { vertices, indices } => emit_arcs_mesh_output(
            join, side, vertices, None, indices, vertex_ids, vertex, attributes, output,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_arcs_mesh_output(
    join: &EndpointData,
    side: usize,
    vertices: &[Point64],
    fan: Option<ValidatedFan>,
    indices: &[usize],
    vertex_ids: &mut Vec<VertexId>,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut (impl StrokeGeometryBuilder + ?Sized),
) -> Result<bool, TessellationError> {
    if vertices.len() < 3 {
        return Ok(false);
    }

    for point64 in &vertices[1..vertices.len() - 1] {
        let Ok(position) = point64_to_point(*point64) else {
            return Ok(false);
        };
        if position == join.position {
            return Ok(false);
        }
    }

    vertex_ids.clear();
    vertex_ids.reserve(vertices.len());
    vertex_ids.push(join.side_points[side].prev_vertex);

    vertex.position_on_path = join.position;
    vertex.half_width = join.half_width;
    vertex.side = if side == SIDE_POSITIVE {
        Side::Positive
    } else {
        Side::Negative
    };
    for point64 in &vertices[1..vertices.len() - 1] {
        // The preflight above checked both components against the f32 range.
        let position = point(point64.x as f32, point64.y as f32);
        vertex.normal = (position - join.position) / join.half_width;
        vertex_ids.push(output.add_stroke_vertex(StrokeVertex(vertex, attributes))?);
    }
    vertex_ids.push(join.side_points[side].next_vertex);

    if let Some(fan) = fan {
        for [first, second, third] in fan.triangles() {
            output.add_triangle(vertex_ids[first], vertex_ids[third], vertex_ids[second]);
        }
    } else {
        for triangle in indices.chunks_exact(3) {
            output.add_triangle(
                vertex_ids[triangle[0]],
                vertex_ids[triangle[2]],
                vertex_ids[triangle[1]],
            );
        }
    }

    Ok(true)
}

fn tessellate_round_join(
    join: &mut EndpointData,
    side: usize,
    options: &StrokeOptions,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut (impl StrokeGeometryBuilder + ?Sized),
) -> Result<(), TessellationError> {
    let center = join.position;
    let radius = join.half_width;
    let start_normal = join.side_points[side].prev - center;
    let end_normal = join.side_points[side].next - center;

    let mut start_vertex = join.side_points[side].prev_vertex;
    let mut end_vertex = join.side_points[side].next_vertex;

    let angle_sign = if side == SIDE_NEGATIVE { 1.0 } else { -1.0 };

    let mut start_angle = start_normal.angle_from_x_axis();
    let mut diff = start_angle.angle_to(end_normal.angle_from_x_axis());

    // if the angle is doesn't have the desired sign, adjust it.
    if diff.radians * angle_sign < 0.0 {
        diff.radians = angle_sign * (2.0 * PI - diff.radians.abs());
    }
    let mut end_angle = start_angle + diff;

    if side == SIDE_NEGATIVE {
        // Flip to keep consistent winding order.
        core::mem::swap(&mut start_angle, &mut end_angle);
        core::mem::swap(&mut start_vertex, &mut end_vertex);
    }

    // Compute the required number of subdivisions,
    let num_subdivisions = num_arc_subdivisions(radius, options.tolerance, diff.radians);

    vertex.side = if side == SIDE_POSITIVE {
        Side::Positive
    } else {
        Side::Negative
    };

    crate::stroke::tessellate_arc(
        (start_angle.radians, end_angle.radians),
        start_vertex,
        end_vertex,
        num_subdivisions,
        vertex,
        attributes,
        output,
    )
}

fn add_join_base_vertices(
    join: &mut EndpointData,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut (impl StrokeGeometryBuilder + ?Sized),
    side: Side,
) -> Result<(), TessellationError> {
    vertex.side = side;

    let side = match side {
        Side::Positive => SIDE_POSITIVE,
        Side::Negative => SIDE_NEGATIVE,
    };

    if let Some(pos) = join.side_points[side].single_vertex {
        vertex.normal = (pos - join.position) / join.half_width;
        let vertex = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;
        join.side_points[side].prev_vertex = vertex;
        join.side_points[side].next_vertex = vertex;
    } else {
        vertex.normal = (join.side_points[side].prev - join.position) / join.half_width;
        let prev_vertex = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;

        vertex.normal = (join.side_points[side].next - join.position) / join.half_width;
        let next_vertex = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;

        join.side_points[side].prev_vertex = prev_vertex;
        join.side_points[side].next_vertex = next_vertex;
    }

    Ok(())
}

// TODO: the naming is a bit confusing. We do half of the work to compute the join's side positions
// in compute_side_attachment_positions.
fn compute_join_side_positions(
    prev: &EndpointData,
    join: &mut EndpointData,
    next: &EndpointData,
    miter_limit: f32,
    side: usize,
) {
    nan_check!(join.position);
    nan_check!(prev.side_points[side].next);
    nan_check!(join.side_points[side].next);

    let sign = side_sign(side);
    let v0 = (join.side_points[side].prev - prev.side_points[side].next).normalize();
    let v1 = (next.side_points[side].prev - join.side_points[side].next).normalize();
    let inward = v0.cross(v1) * sign > 0.0;
    let forward = v0.dot(v1) > 0.0;

    let normal = compute_normal(v0, v1) * sign;
    let path_v0 = (join.position - prev.position).normalize();
    let path_v1 = (next.position - join.position).normalize();

    nan_check!(v0, v1);

    let normal_same_side = (v0 + v1).dot(path_v0 + path_v1) >= 0.0;

    // We must watch out for special cases where the previous or next edge is small relative
    // to the line width. Our workaround only applies to "sharp" angles (more than 90 degrees).
    let angle_is_sharp = inward && !forward && normal_same_side;
    if angle_is_sharp {
        // Project the back vertex on the previous and next edges and subtract the edge length
        // to see if the back vertex ends up further than the opposite endpoint of the edge.
        let extruded_normal = normal * join.half_width;
        let prev_length = join.advancement - prev.advancement;
        let next_length = next.advancement - join.advancement;
        let d_next = extruded_normal.dot(v1) - next_length;
        let d_prev = extruded_normal.dot(-v0) - prev_length;

        if d_next.min(d_prev) > 0.0 || normal.square_length() < 1e-5 {
            // Case of an overlapping stroke. In order to prevent the back vertex to create a
            // spike outside of the stroke, we simply don't create it and we'll "fold" the join
            // instead.
            join.fold[side] = true;
        }
    }

    // For concave sides we'll simply connect at the intersection of the two side edges.
    let concave = inward && normal_same_side && !join.fold[side];

    if concave
        || ((join.line_join == LineJoin::Miter || join.line_join == LineJoin::MiterClip)
            && !miter_limit_is_exceeded(normal, miter_limit))
    {
        let p = join.position + normal * join.half_width;
        join.side_points[side].single_vertex = Some(p);
    } else if join.line_join == LineJoin::MiterClip {
        // It is convenient to handle the miter-clip case here by simply moving
        // tow points on this side to the clip line.
        // This way the rest of the code doesn't differentiate between miter and miter-clip.
        let n0 = join.side_points[side].prev - join.position;
        let n1 = join.side_points[side].next - join.position;
        let (prev_normal, next_normal) =
            get_clip_intersections(n0, n1, normal, miter_limit * join.half_width);
        join.side_points[side].prev = join.position + prev_normal;
        join.side_points[side].next = join.position + next_normal;
        nan_check!(n0, n1, prev_normal, next_normal);
        nan_check!(join.side_points[side].prev);
        nan_check!(join.side_points[side].next);
    }
}

fn tessellate_last_edge(
    p0: &EndpointData,
    p1: &mut EndpointData,
    is_first_edge: bool,
    options: &StrokeOptions,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut (impl StrokeGeometryBuilder + ?Sized),
) -> Result<(), TessellationError> {
    let v = p1.position - p0.position;
    p1.advancement = p0.advancement + v.length();

    vertex.src = p1.src;
    vertex.position_on_path = p1.position;
    vertex.advancement = p1.advancement;
    vertex.half_width = p1.half_width;
    vertex.buffer_is_valid = false;

    let sides = [Side::Positive, Side::Negative];

    for side in 0..2 {
        let side_position = p1.side_points[side].prev;
        let clip = match options.end_cap {
            LineCap::Square => Some(p1.half_width),
            LineCap::Butt => Some(0.0),
            _ => None,
        };

        if let Some(clip) = clip {
            let normal = (p1.position - p0.position).normalize();
            let clip_line = Line {
                point: p1.position + normal * clip,
                vector: tangent(normal),
            };
            let side_line = Line {
                point: side_position,
                vector: side_position - p0.side_points[side].next,
            };

            let intersection = clip_line
                .to_f64()
                .intersection(&side_line.to_f64())
                .map(|p| p.to_f32())
                .unwrap_or(p1.side_points[side].prev);

            p1.side_points[side].prev = intersection;
        }

        vertex.side = sides[side];
        vertex.normal = (p1.side_points[side].prev - p1.position) / p1.half_width;
        let prev_vertex = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;
        p1.side_points[side].prev_vertex = prev_vertex;
    }

    // Skip the edge triangles if it is also the first edge (tessellate_first_edge will do it).
    if !is_first_edge {
        add_edge_triangles(p0, p1, output);
    }

    if options.end_cap == LineCap::Round {
        crate::stroke::tessellate_round_cap(
            p1.position,
            p1.half_width,
            p1.side_points[SIDE_POSITIVE].prev - p1.position,
            p1.side_points[SIDE_POSITIVE].prev_vertex,
            p1.side_points[SIDE_NEGATIVE].prev_vertex,
            v,
            options,
            false,
            vertex,
            attributes,
            output,
        )?;
    }

    Ok(())
}

fn tessellate_first_edge(
    first: &mut EndpointData,
    second: &EndpointData,
    options: &StrokeOptions,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut (impl StrokeGeometryBuilder + ?Sized),
) -> Result<(), TessellationError> {
    vertex.src = first.src;
    vertex.position_on_path = first.position;
    vertex.advancement = first.advancement;
    vertex.half_width = first.half_width;
    vertex.buffer_is_valid = false;

    let sides = [Side::Positive, Side::Negative];

    for side in 0..2 {
        let mut side_position = first.side_points[side].next;
        let clip = match options.start_cap {
            LineCap::Square => Some(first.half_width),
            LineCap::Butt => Some(0.0),
            _ => None,
        };

        if let Some(clip) = clip {
            let normal = (first.position - second.position).normalize();
            let clip_line = Line {
                point: first.position + normal * clip,
                vector: tangent(normal),
            };
            let side_line = Line {
                point: side_position,
                vector: side_position - second.side_points[side].prev,
            };

            let intersection = clip_line
                .to_f64()
                .intersection(&side_line.to_f64())
                .map(|p| p.to_f32())
                .unwrap_or(first.side_points[side].next);
            side_position = intersection;
        }

        vertex.side = sides[side];
        vertex.normal = (side_position - first.position) / first.half_width;
        first.side_points[side].next_vertex =
            output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;
    }

    // Tessellate the edge between prev and join.
    add_edge_triangles(first, second, output);

    match options.start_cap {
        LineCap::Round => crate::stroke::tessellate_round_cap(
            first.position,
            first.half_width,
            first.side_points[SIDE_NEGATIVE].next - first.position,
            first.side_points[SIDE_NEGATIVE].next_vertex,
            first.side_points[SIDE_POSITIVE].next_vertex,
            first.position - second.position,
            options,
            true,
            vertex,
            attributes,
            output,
        ),
        _ => Ok(()),
    }
}

fn get_clip_intersections(
    previous_normal: Vector,
    next_normal: Vector,
    normal: Vector,
    clip_distance: f32,
) -> (Vector, Vector) {
    let clip_line = Line {
        point: normal.normalize().to_point() * clip_distance,
        vector: tangent(normal),
    }
    .to_f64();

    let prev_line = Line {
        point: previous_normal.to_point(),
        vector: tangent(previous_normal),
    }
    .to_f64();

    let next_line = Line {
        point: next_normal.to_point(),
        vector: tangent(next_normal),
    }
    .to_f64();

    let i1 = clip_line
        .intersection(&prev_line)
        .map(|p| p.to_f32())
        .unwrap_or_else(|| normal.to_point())
        .to_vector();
    let i2 = clip_line
        .intersection(&next_line)
        .map(|p| p.to_f32())
        .unwrap_or_else(|| normal.to_point())
        .to_vector();

    (i1, i2)
}

// Derived from:
// miter_limit = miter_length / stroke_width
// miter_limit = (normal.length() * half_width) / (2.0 * half_width)
fn miter_limit_is_exceeded(normal: Vector, miter_limit: f32) -> bool {
    normal.square_length() > miter_limit * miter_limit * 4.0
}

fn side_sign(side: usize) -> f32 {
    if side == SIDE_NEGATIVE {
        -1.0
    } else {
        1.0
    }
}

// A fall-back that avoids off artifacts with zero-area rectangles as
// well as overlapping triangles if the rectangle is much smaller than the
// line width in any dimension.
#[inline(never)]
fn approximate_thin_rectangle(builder: &mut StrokeBuilder, rect: &Box2D, attributes: Attributes) {
    let (from, to, d) = if rect.width() > rect.height() {
        let d = rect.height() * 0.5;
        let min_x = rect.min.x + d;
        let max_x = rect.max.x - d;
        let y = (rect.min.y + rect.max.y) * 0.5;

        (point(min_x, y), point(max_x, y), d)
    } else {
        let d = rect.width() * 0.5;
        let min_y = rect.min.y + d;
        let max_y = rect.max.y - d;
        let x = (rect.min.x + rect.max.x) * 0.5;

        (point(x, min_y), point(x, max_y), d)
    };

    // Save the builder options.
    let options = builder.builder.options;

    let cap = match options.line_join {
        LineJoin::Round | LineJoin::Arcs => LineCap::Round,
        _ => LineCap::Square,
    };

    builder.builder.options.line_width += d;
    builder.builder.options.start_cap = cap;
    builder.builder.options.end_cap = cap;

    builder.add_line_segment(&LineSegment { from, to }, attributes);

    // Restore the builder options.
    builder.builder.options = options;
}

struct PointBuffer<T = EndpointData> {
    points: [T; 3],
    start: usize,
    count: usize,
}

impl<T: Copy + Default> PointBuffer<T> {
    fn new() -> Self {
        PointBuffer {
            points: [T::default(); 3],
            start: 0,
            count: 0,
        }
    }

    fn push(&mut self, point: T) {
        if self.count < 3 {
            self.points[self.count] = point;
            self.count += 1;
            return;
        }

        self.points[self.start] = point;
        self.start += 1;
        if self.start == 3 {
            self.start = 0;
        }
    }

    fn replace_last(&mut self, point: T) {
        let mut idx = self.start;
        if idx == 0 {
            idx = self.count;
        }
        self.points[idx - 1] = point;
    }

    fn clear(&mut self) {
        self.count = 0;
        self.start = 0;
    }

    fn count(&self) -> usize {
        self.count
    }

    fn get(&self, idx: usize) -> &T {
        assert!(idx < self.count);
        let idx = (idx + self.start) % 3;

        &self.points[idx]
    }

    fn get_reverse(&self, idx: usize) -> &T {
        assert!(idx < self.count);
        self.get(self.count - 1 - idx)
    }

    fn get_mut(&mut self, idx: usize) -> &mut T {
        assert!(idx < self.count);
        let idx = (idx + self.start) % 3;

        &mut self.points[idx]
    }

    fn last(&self) -> &T {
        assert!(self.count > 0);
        self.get(self.count - 1)
    }

    fn last_mut(&mut self) -> &mut T {
        assert!(self.count > 0);
        self.get_mut(self.count - 1)
    }

    fn last_two(&self) -> (&T, &T) {
        assert!(self.count >= 2);
        (self.get_reverse(1), self.get_reverse(0))
    }

    fn last_two_mut(&mut self) -> (&mut T, &mut T) {
        assert!(self.count >= 2);
        let i0 = (self.start + self.count - 1) % 3;
        let i1 = (self.start + self.count - 2) % 3;
        unsafe {
            (
                &mut *(self.points.get_unchecked_mut(i1) as *mut _),
                &mut *(self.points.get_unchecked_mut(i0) as *mut _),
            )
        }
    }
}

pub(crate) fn tessellate_round_cap(
    center: Point,
    radius: f32,
    start_normal: Vector,
    start_vertex: VertexId,
    end_vertex: VertexId,
    edge_normal: Vector,
    options: &StrokeOptions,
    is_start: bool,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut (impl StrokeGeometryBuilder + ?Sized),
) -> Result<(), TessellationError> {
    if radius < options.tolerance {
        return Ok(());
    }

    let first_side = if is_start ^ (edge_normal.cross(start_normal) >= 0.0) {
        Side::Positive
    } else {
        Side::Negative
    };

    let start_angle = start_normal.angle_from_x_axis();
    let diff = start_angle.angle_to(edge_normal.angle_from_x_axis());
    let mid_angle = start_angle + diff;
    let end_angle = mid_angle + diff;

    // Compute the required number of subdivisions on each side,
    let num_subdivisions = num_arc_subdivisions(radius, options.tolerance, diff.radians);

    vertex.position_on_path = center;
    vertex.half_width = radius;
    vertex.side = first_side;

    vertex.normal = edge_normal.normalize();
    let mid_vertex = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;

    output.add_triangle(start_vertex, mid_vertex, end_vertex);

    tessellate_arc(
        (start_angle.radians, mid_angle.radians),
        start_vertex,
        mid_vertex,
        num_subdivisions,
        vertex,
        attributes,
        output,
    )?;

    vertex.side = first_side.opposite();

    tessellate_arc(
        (mid_angle.radians, end_angle.radians),
        mid_vertex,
        end_vertex,
        num_subdivisions,
        vertex,
        attributes,
        output,
    )?;

    Ok(())
}

pub(crate) fn tessellate_empty_square_cap(
    position: Point,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut (impl StrokeGeometryBuilder + ?Sized),
) -> Result<(), TessellationError> {
    vertex.position_on_path = position;

    vertex.normal = vector(1.0, 1.0);
    vertex.side = Side::Negative;

    let a = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;

    vertex.normal = vector(1.0, -1.0);
    vertex.side = Side::Positive;

    let b = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;

    vertex.normal = vector(-1.0, -1.0);
    vertex.side = Side::Positive;

    let c = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;

    vertex.normal = vector(-1.0, 1.0);
    vertex.side = Side::Negative;

    let d = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;

    output.add_triangle(a, b, c);
    output.add_triangle(a, c, d);

    Ok(())
}

pub(crate) fn tessellate_empty_round_cap(
    center: Point,
    options: &StrokeOptions,
    vertex: &mut StrokeVertexData,
    attribute_store: &dyn AttributeStore,
    output: &mut (impl StrokeGeometryBuilder + ?Sized),
) -> Result<(), TessellationError> {
    let radius = vertex.half_width;

    vertex.position_on_path = center;
    vertex.normal = vector(-1.0, 0.0);
    vertex.side = Side::Positive;

    let left_id = output.add_stroke_vertex(StrokeVertex(vertex, attribute_store))?;

    vertex.normal = vector(1.0, 0.0);
    vertex.side = Side::Negative;

    let right_id = output.add_stroke_vertex(StrokeVertex(vertex, attribute_store))?;

    tessellate_round_cap(
        center,
        radius,
        vector(-1.0, 0.0),
        left_id,
        right_id,
        vector(0.0, 1.0),
        options,
        true,
        vertex,
        attribute_store,
        output,
    )?;

    tessellate_round_cap(
        center,
        radius,
        vector(1.0, 0.0),
        right_id,
        left_id,
        vector(0.0, -1.0),
        options,
        false,
        vertex,
        attribute_store,
        output,
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn tessellate_arc(
    angle: (f32, f32),
    va: VertexId,
    vb: VertexId,
    num_recursions: u32,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut (impl StrokeGeometryBuilder + ?Sized),
) -> Result<(), TessellationError> {
    if num_recursions == 0 {
        return Ok(());
    }

    let mid_angle = (angle.0 + angle.1) * 0.5;

    let normal = vector(mid_angle.cos(), mid_angle.sin());

    vertex.normal = normal;

    let vertex_id = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;

    output.add_triangle(va, vertex_id, vb);

    tessellate_arc(
        (angle.0, mid_angle),
        va,
        vertex_id,
        num_recursions - 1,
        vertex,
        attributes,
        output,
    )?;
    tessellate_arc(
        (mid_angle, angle.1),
        vertex_id,
        vb,
        num_recursions - 1,
        vertex,
        attributes,
        output,
    )
}

/// Extra vertex information from the `StrokeTessellator`.
pub(crate) struct StrokeVertexData<'l> {
    pub(crate) position_on_path: Point,
    pub(crate) half_width: f32,
    pub(crate) normal: Vector,
    pub(crate) advancement: f32,
    pub(crate) side: Side,
    pub(crate) src: VertexSource,
    pub(crate) buffer: &'l mut [f32],
    pub(crate) buffer_is_valid: bool,
}

/// Extra vertex information from the `StrokeTessellator` accessible when building vertices.
pub struct StrokeVertex<'a, 'b>(
    pub(crate) &'b mut StrokeVertexData<'a>,
    pub(crate) &'b dyn AttributeStore,
);

impl<'a, 'b> StrokeVertex<'a, 'b> {
    /// The vertex position.
    #[inline]
    pub fn position(&self) -> Point {
        self.0.position_on_path + self.0.normal * self.0.half_width
    }

    /// Normal at this vertex.
    ///
    /// The length of the provided normal is such that displacing the vertex along it
    /// inflates the stroke by 2.0 (1.0 on each side).
    /// It can be zero for a clipped join vertex located on the path itself.
    #[inline]
    pub fn normal(&self) -> Vector {
        self.0.normal
    }

    /// Position of this vertex on the path, unaffected by the line width.
    #[inline]
    pub fn position_on_path(&self) -> Point {
        self.0.position_on_path
    }

    /// The line width at this vertex.
    ///
    /// If a line width modifier is set via `StrokeOptions::variable_line_width`, the
    /// returned line width is equal to `StrokeOptions::line_width` multiplied by the
    /// line width modifier at this vertex.
    #[inline]
    pub fn line_width(&self) -> f32 {
        self.0.half_width * 2.0
    }

    /// How far along the path this vertex is.
    #[inline]
    pub fn advancement(&self) -> f32 {
        self.0.advancement
    }

    /// Whether the vertex is on the positive or negative side of the path.
    #[inline]
    pub fn side(&self) -> Side {
        self.0.side
    }

    /// Returns the source of this vertex.
    #[inline]
    pub fn source(&self) -> VertexSource {
        self.0.src
    }

    /// Computes and returns the custom attributes for this vertex.
    ///
    /// The attributes are interpolated along the edges on which this vertex is.
    /// This can include multiple edges if the vertex is at an intersection.
    #[inline]
    pub fn interpolated_attributes(&mut self) -> Attributes {
        if self.0.buffer_is_valid {
            return self.0.buffer;
        }

        match self.0.src {
            VertexSource::Endpoint { id } => self.1.get(id),
            VertexSource::Edge { from, to, t } => {
                let a = self.1.get(from);
                let b = self.1.get(to);
                for i in 0..self.0.buffer.len() {
                    self.0.buffer[i] = a[i] * (1.0 - t) + b[i] * t;
                }
                self.0.buffer_is_valid = true;

                self.0.buffer
            }
        }
    }
}

pub(crate) fn circle_flattening_step(radius: f32, mut tolerance: f32) -> f32 {
    // Don't allow high tolerance values (compared to the radius) to avoid edge cases.
    tolerance = f32::min(tolerance, radius);
    2.0 * ((radius - tolerance) / radius).acos()
}

// Upper bound on the recursion depth of `tessellate_arc`.
//
// Each level of recursion doubles the number of segments, so 16 already stands for
// 65536 segments per arc, which is finer than anything that can be displayed.
const MAX_ARC_SUBDIVISIONS: u32 = 16;

fn num_arc_subdivisions(radius: f32, tolerance: f32, angle: f32) -> u32 {
    let step = circle_flattening_step(radius, tolerance);
    let num_segments = (angle.abs() / step).ceil();
    // Casting a float to an integer saturates and maps NaN to zero, so infinite and
    // NaN segment counts (respectively a zero step, and a zero step with a zero
    // angle) both land in range here.
    (num_segments.log2().round() as u32).min(MAX_ARC_SUBDIVISIONS)
}

fn flatten_quad<F>(curve: &QuadraticBezierSegment<f32>, tolerance: f32, cb: &mut F)
where
    F: FnMut(Point, f32, bool),
{
    if let Some(t_split) = find_sharp_turn(curve) {
        let (before, after) = curve.split(t_split);

        before.for_each_flattened_with_t(tolerance, &mut |line, t| {
            let is_flattening_step = t.end != 1.0;
            let t = t.end * t_split;
            cb(line.to, t, is_flattening_step);
        });

        after.for_each_flattened_with_t(tolerance, &mut |line, t| {
            let is_flattening_step = t.end != 1.0;
            let t = t_split + t.end * (1.0 - t_split);
            cb(line.to, t, is_flattening_step);
        });
    } else {
        curve.for_each_flattened_with_t(tolerance, &mut |line, t| {
            let is_flattening_step = t.end != 1.0;
            cb(line.to, t.end, is_flattening_step);
        });
    }
}

#[cfg(test)]
use crate::geometry_builder::*;
#[cfg(test)]
use crate::path::Path;

#[cfg(test)]
fn test_path(path: PathSlice, options: &StrokeOptions, expected_triangle_count: Option<u32>) {
    struct TestBuilder<'l> {
        builder: SimpleBuffersBuilder<'l>,
    }

    impl<'l> GeometryBuilder for TestBuilder<'l> {
        fn begin_geometry(&mut self) {
            self.builder.begin_geometry();
        }
        fn end_geometry(&mut self) {
            self.builder.end_geometry();
        }
        fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
            assert_ne!(a, b);
            assert_ne!(a, c);
            assert_ne!(b, c);
            let pa = self.builder.buffers().vertices[a.0 as usize];
            let pb = self.builder.buffers().vertices[b.0 as usize];
            let pc = self.builder.buffers().vertices[c.0 as usize];
            let threshold = -0.035; // Floating point errors :(
            assert!((pa - pb).cross(pc - pb) >= threshold);
            self.builder.add_triangle(a, b, c);
        }
        fn abort_geometry(&mut self) {
            panic!();
        }
    }

    impl<'l> StrokeGeometryBuilder for TestBuilder<'l> {
        fn add_stroke_vertex(
            &mut self,
            attributes: StrokeVertex,
        ) -> Result<VertexId, GeometryBuilderError> {
            assert!(!attributes.position().x.is_nan());
            assert!(!attributes.position().y.is_nan());
            assert!(!attributes.normal().x.is_nan());
            assert!(!attributes.normal().y.is_nan());
            assert!(!attributes.advancement().is_nan());
            self.builder.add_stroke_vertex(attributes)
        }
    }

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    let mut tess = StrokeTessellator::new();
    tess.tessellate_path(
        path,
        options,
        &mut TestBuilder {
            builder: simple_builder(&mut buffers),
        },
    )
    .unwrap();

    if let Some(triangles) = expected_triangle_count {
        assert_eq!(
            triangles,
            buffers.indices.len() as u32 / 3,
            "Unexpected number of triangles"
        );
    }
}

#[cfg(test)]
fn mesh_contains_point(mesh: &VertexBuffers<Point, u16>, point: Point) -> bool {
    mesh.indices.chunks_exact(3).any(|triangle| {
        let a = mesh.vertices[triangle[0] as usize];
        let b = mesh.vertices[triangle[1] as usize];
        let c = mesh.vertices[triangle[2] as usize];
        let ab = (b - a).cross(point - a);
        let bc = (c - b).cross(point - b);
        let ca = (a - c).cross(point - c);
        let epsilon = 1.0e-6;

        (ab >= -epsilon && bc >= -epsilon && ca >= -epsilon)
            || (ab <= epsilon && bc <= epsilon && ca <= epsilon)
    })
}

#[test]
fn test_square() {
    let mut builder = Path::builder_with_attributes(1);

    builder.begin(point(-1.0, 1.0), &[0.3]);
    builder.line_to(point(1.0, 1.0), &[0.3]);
    builder.line_to(point(1.0, -1.0), &[0.3]);
    builder.line_to(point(-1.0, -1.0), &[0.3]);
    builder.end(false);

    builder.begin(point(-1.0, -1.0), &[0.3]);
    builder.line_to(point(1.0, -1.0), &[0.3]);
    builder.line_to(point(1.0, 1.0), &[0.3]);
    builder.line_to(point(-1.0, 1.0), &[0.3]);
    builder.end(false);

    let path = builder.build();

    // Test both with and without the fixed width fast path.
    let options = [
        StrokeOptions::default().with_variable_line_width(0),
        StrokeOptions::default(),
    ];

    for options in options {
        test_path(
            path.as_slice(),
            &options
                .with_line_join(LineJoin::Miter)
                .with_line_cap(LineCap::Butt),
            Some(12),
        );

        test_path(
            path.as_slice(),
            &options
                .with_line_join(LineJoin::Bevel)
                .with_line_cap(LineCap::Square),
            Some(16),
        );

        test_path(
            path.as_slice(),
            &options
                .with_line_join(LineJoin::MiterClip)
                .with_miter_limit(1.0)
                .with_line_cap(LineCap::Round),
            None,
        );

        test_path(
            path.as_slice(),
            &options
                .with_tolerance(0.001)
                .with_line_join(LineJoin::Round)
                .with_line_cap(LineCap::Round),
            None,
        );
    }
}

#[test]
fn arcs_join_records_curve_differentials_before_flattening() {
    let mut builder = Path::builder_with_attributes(1);
    builder.begin(point(0.0, 0.0), &[1.0]);
    builder.line_to(point(10.0, 0.0), &[1.0]);
    builder.quadratic_bezier_to(point(10.0, 5.0), point(15.0, 5.0), &[1.0]);
    builder.cubic_bezier_to(
        point(20.0, 5.0),
        point(20.0, 10.0),
        point(15.0, 10.0),
        &[1.0],
    );
    builder.end(false);
    let path = builder.build();

    test_path(
        path.as_slice(),
        &StrokeOptions::default()
            .with_line_join(LineJoin::Arcs)
            .with_variable_line_width(0),
        None,
    );
    test_path(
        path.as_slice(),
        &StrokeOptions::default().with_line_join(LineJoin::Arcs),
        None,
    );
}

#[test]
fn arcs_join_preserves_differentials_between_quadratic_segments() {
    let mut builder = Path::builder();
    builder.begin(point(0.0, 0.0));
    builder.quadratic_bezier_to(point(4.0, 24.0), point(12.0, 10.0));
    builder.quadratic_bezier_to(point(16.0, -24.0), point(24.0, -10.0));
    builder.end(false);
    let path = builder.build();
    let options = StrokeOptions::default()
        .with_line_width(8.0)
        .with_tolerance(0.1);
    let mut arcs_mesh: VertexBuffers<Point, u16> = VertexBuffers::new();
    let mut round_mesh: VertexBuffers<Point, u16> = VertexBuffers::new();

    StrokeTessellator::new()
        .tessellate_path(
            &path,
            &options.with_line_join(LineJoin::Arcs),
            &mut simple_builder(&mut arcs_mesh),
        )
        .expect("the curve-to-curve arcs join should tessellate");
    StrokeTessellator::new()
        .tessellate_path(
            &path,
            &options.with_line_join(LineJoin::Round),
            &mut simple_builder(&mut round_mesh),
        )
        .expect("the round comparison should tessellate");

    assert_ne!(arcs_mesh.vertices, round_mesh.vertices);
}

#[test]
fn endpoint_data_stays_within_the_hot_path_size_budget() {
    assert!(core::mem::size_of::<EndpointData>() <= 112);
}

#[test]
fn arcs_join_records_the_implicit_closing_line() {
    let mut builder = Path::builder();
    builder.begin(point(0.0, 0.0));
    builder.line_to(point(10.0, 0.0));
    builder.line_to(point(0.0, 10.0));
    builder.end(true);
    let path = builder.build();

    test_path(
        path.as_slice(),
        &StrokeOptions::default().with_line_join(LineJoin::Arcs),
        None,
    );
}

#[test]
fn cubic_differentials_preserve_signed_endpoint_curvature() {
    let curve = CubicBezierSegment {
        from: point(0.0, 0.0),
        ctrl1: point(1.0, 0.0),
        ctrl2: point(1.0, 1.0),
        to: point(0.0, 1.0),
    };

    let differentials = cubic_differentials(&curve);

    let EndpointDifferential::Regular {
        unit_tangent: start_tangent,
        curvature: start_curvature,
    } = differentials.start
    else {
        panic!("the cubic start should be regular");
    };
    let EndpointDifferential::Regular {
        unit_tangent: end_tangent,
        curvature: end_curvature,
    } = differentials.end
    else {
        panic!("the cubic end should be regular");
    };
    assert_eq!(start_tangent, vector(1.0, 0.0));
    assert_eq!(end_tangent, vector(-1.0, 0.0));
    assert!((start_curvature - 2.0 / 3.0).abs() < 1e-12);
    assert!((end_curvature - 2.0 / 3.0).abs() < 1e-12);
}

#[test]
fn arcs_join_resolves_endpoint_differentials_into_support_geometry() {
    let join = EndpointData {
        position: point(10.0, 5.0),
        half_width: 2.0,
        line_join: LineJoin::Arcs,
        ..EndpointData::default()
    };
    let differentials = EndpointDifferentials {
        incoming: EndpointDifferential::Regular {
            unit_tangent: vector(1.0, 0.0),
            curvature: 0.0,
        },
        outgoing: EndpointDifferential::Regular {
            unit_tangent: vector(0.0, 1.0),
            curvature: 0.1,
        },
    };

    let resolution = resolve_arcs_join(&join, differentials, &StrokeOptions::default());

    assert!(matches!(resolution, Ok(JoinConstruction::Arcs(_))));
}

#[test]
fn arcs_join_emits_a_different_mesh_than_round_for_both_turn_directions() {
    let options = StrokeOptions::default()
        .with_line_width(4.0)
        .with_tolerance(0.01);

    for outgoing_y in [8.0, -8.0] {
        let mut path = Path::builder();
        path.begin(point(0.0, 0.0));
        path.line_to(point(10.0, 0.0));
        path.quadratic_bezier_to(point(10.0, outgoing_y), point(18.0, outgoing_y));
        path.end(false);
        let path = path.build();
        let mut arcs_mesh: VertexBuffers<Point, u16> = VertexBuffers::new();
        let mut round_mesh: VertexBuffers<Point, u16> = VertexBuffers::new();

        StrokeTessellator::new()
            .tessellate_path(
                &path,
                &options.with_line_join(LineJoin::Arcs),
                &mut simple_builder(&mut arcs_mesh),
            )
            .expect("the arcs join should tessellate");
        StrokeTessellator::new()
            .tessellate_path(
                &path,
                &options.with_line_join(LineJoin::Round),
                &mut simple_builder(&mut round_mesh),
            )
            .expect("the round join should tessellate");

        assert_ne!(arcs_mesh.vertices, round_mesh.vertices);
    }
}

#[test]
fn shallow_arcs_join_does_not_silently_become_round() {
    let mut path = Path::builder();
    path.begin(point(0.0, 0.0));
    path.line_to(point(10.0, 0.0));
    path.cubic_bezier_to(point(13.0, 0.2), point(16.0, 1.0), point(19.0, 1.5));
    path.end(false);
    let path = path.build();
    let options = StrokeOptions::default()
        .with_line_width(4.0)
        .with_tolerance(0.01);
    let mut arcs_mesh: VertexBuffers<Point, u16> = VertexBuffers::new();
    let mut round_mesh: VertexBuffers<Point, u16> = VertexBuffers::new();

    StrokeTessellator::new()
        .tessellate_path(
            &path,
            &options.with_line_join(LineJoin::Arcs),
            &mut simple_builder(&mut arcs_mesh),
        )
        .expect("the shallow arcs join should tessellate");
    StrokeTessellator::new()
        .tessellate_path(
            &path,
            &options.with_line_join(LineJoin::Round),
            &mut simple_builder(&mut round_mesh),
        )
        .expect("the round comparison should tessellate");

    assert_ne!(arcs_mesh.vertices, round_mesh.vertices);
}

#[test]
fn arcs_join_emits_the_svg_rectangle_for_opposite_parallel_tangents() {
    let mut path = Path::builder();
    path.begin(point(0.0, 0.0));
    path.line_to(point(10.0, 0.0));
    path.quadratic_bezier_to(point(2.0, 0.0), point(2.0, 8.0));
    path.end(false);
    let path = path.build();
    let options = StrokeOptions::default()
        .with_line_width(4.0)
        .with_miter_limit(3.0)
        .with_line_join(LineJoin::Arcs);
    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    StrokeTessellator::new()
        .tessellate_path(&path, &options, &mut simple_builder(&mut buffers))
        .expect("the parallel arcs fallback should tessellate");

    let far_left = point(16.0, 2.0);
    let far_right = point(16.0, -2.0);
    assert!(
        buffers
            .vertices
            .iter()
            .any(|position| (*position - far_left).square_length() < 1.0e-6)
            && buffers
                .vertices
                .iter()
                .any(|position| (*position - far_right).square_length() < 1.0e-6)
    );
}

#[test]
fn arcs_join_emits_radial_clips_for_sub_unit_miter_limits() {
    let options = StrokeOptions::default()
        .with_line_width(4.0)
        .with_miter_limit(0.5)
        .with_line_join(LineJoin::Arcs);

    for outgoing_y in [8.0, -8.0] {
        let mut path = Path::builder();
        path.begin(point(0.0, 0.0));
        path.line_to(point(10.0, 0.0));
        path.quadratic_bezier_to(point(10.0, outgoing_y), point(18.0, outgoing_y));
        path.end(false);
        let path = path.build();
        let curve = QuadraticBezierSegment {
            from: point(10.0, 0.0),
            ctrl: point(10.0, outgoing_y),
            to: point(18.0, outgoing_y),
        };
        let EndpointDifferential::Regular {
            unit_tangent: tangent,
            curvature,
        } = quadratic_differentials(&curve).start
        else {
            panic!("the quadratic start should be regular");
        };
        let expected = stroke_arcs::construct(JoinInput {
            at: Point64::new(10.0, 0.0),
            incoming: SegmentEnd {
                tangent: Vector64::new(1.0, 0.0),
                curvature: 0.0,
            },
            outgoing: SegmentEnd {
                tangent: Vector64::new(f64::from(tangent.x), f64::from(tangent.y)),
                curvature,
            },
            half_width: 2.0,
            miter_limit: 0.5,
        })
        .expect("the radial clip should resolve");
        let JoinConstruction::RadialClip(expected) = expected else {
            panic!("the sub-unit miter limit should select a radial clip");
        };
        let expected_incoming = point64_to_point(expected.incoming).expect("finite clip point");
        let expected_outgoing = point64_to_point(expected.outgoing).expect("finite clip point");
        let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

        StrokeTessellator::new()
            .tessellate_path(&path, &options, &mut simple_builder(&mut buffers))
            .expect("the radial arcs clip should tessellate");

        assert!(
            buffers
                .vertices
                .iter()
                .any(|position| (*position - expected_incoming).square_length() < 1.0e-6)
                && buffers
                    .vertices
                    .iter()
                    .any(|position| (*position - expected_outgoing).square_length() < 1.0e-6)
        );
        let clipped_corner = point(11.5, -outgoing_y.signum() * 1.5);
        assert!(!mesh_contains_point(&buffers, clipped_corner));
    }
}

#[test]
fn arcs_join_supports_a_zero_miter_limit() {
    let options = StrokeOptions::default()
        .with_line_width(4.0)
        .with_miter_limit(0.0)
        .with_line_join(LineJoin::Arcs);

    for outgoing_y in [8.0, -8.0] {
        let mut path = Path::builder();
        path.begin(point(0.0, 0.0));
        path.line_to(point(10.0, 0.0));
        path.quadratic_bezier_to(point(10.0, outgoing_y), point(18.0, outgoing_y));
        path.end(false);
        let path = path.build();

        test_path(path.as_slice(), &options, None);
    }
}

#[test]
fn arcs_join_keeps_explicit_svg_fallback_states() {
    let mut join = EndpointData {
        half_width: 2.0,
        line_join: LineJoin::Arcs,
        ..EndpointData::default()
    };
    let line_differentials = EndpointDifferentials {
        incoming: EndpointDifferential::Regular {
            unit_tangent: vector(1.0, 0.0),
            curvature: 0.0,
        },
        outgoing: EndpointDifferential::Regular {
            unit_tangent: vector(0.0, 1.0),
            curvature: 0.0,
        },
    };

    let options = StrokeOptions::default();
    let line_join = resolve_arcs_join(&join, line_differentials, &options);
    let line_geometry = prepare_arcs_join(&mut join, line_differentials, &options);
    let applied_line_fallback = join.line_join;
    join.line_join = LineJoin::Arcs;
    let degenerate_differentials = EndpointDifferentials {
        outgoing: EndpointDifferential::Degenerate,
        ..line_differentials
    };
    let degenerate_join = resolve_arcs_join(&join, degenerate_differentials, &options);
    let degenerate_geometry = prepare_arcs_join(&mut join, degenerate_differentials, &options);

    assert!(matches!(line_join, Ok(JoinConstruction::MiterClip)));
    assert!(line_geometry.is_none());
    assert_eq!(applied_line_fallback, LineJoin::MiterClip);
    assert!(degenerate_join.is_err());
    assert!(degenerate_geometry.is_none());
    assert_eq!(join.line_join, LineJoin::Round);
}

#[test]
fn test_empty_path() {
    let path = Path::builder().build();
    test_path(path.as_slice(), &StrokeOptions::default(), Some(0));

    let path = Path::builder_with_attributes(1).build();
    test_path(path.as_slice(), &StrokeOptions::default(), Some(0));
}

#[test]
fn test_empty_caps() {
    let mut builder = Path::builder_with_attributes(1);

    // moveto + close: empty cap.
    builder.begin(point(1.0, 0.0), &[1.0]);
    builder.end(true);

    // Only moveto + lineto at same position: empty cap.
    builder.begin(point(2.0, 0.0), &[1.0]);
    builder.line_to(point(2.0, 0.0), &[1.0]);
    builder.end(false);

    // Only moveto + lineto at same position: empty cap.
    builder.begin(point(3.0, 0.0), &[1.0]);
    builder.line_to(point(3.0, 0.0), &[1.0]);
    builder.end(true);

    // Only moveto + multiple lineto at same position: empty cap.
    builder.begin(point(3.0, 0.0), &[1.0]);
    builder.line_to(point(3.0, 0.0), &[1.0]);
    builder.line_to(point(3.0, 0.0), &[1.0]);
    builder.line_to(point(3.0, 0.0), &[1.0]);
    builder.end(true);

    // moveto then end (not closed): no empty cap.
    builder.begin(point(4.0, 0.0), &[1.0]);
    builder.end(false);

    let path = builder.build();

    let options = [
        StrokeOptions::default().with_variable_line_width(0),
        StrokeOptions::default(),
    ];

    for options in options {
        test_path(
            path.as_slice(),
            &options.with_line_cap(LineCap::Butt),
            Some(0),
        );
        test_path(
            path.as_slice(),
            &options.with_line_cap(LineCap::Square),
            Some(8),
        );
        test_path(
            path.as_slice(),
            &options.with_line_cap(LineCap::Round),
            None,
        );
    }
}

#[test]
fn test_too_many_vertices() {
    /// This test checks that the tessellator returns the proper error when
    /// the geometry builder run out of vertex ids.
    use crate::extra::rust_logo::build_logo_path;
    use crate::GeometryBuilder;

    struct Builder {
        max_vertices: u32,
    }

    impl GeometryBuilder for Builder {
        fn begin_geometry(&mut self) {}
        fn end_geometry(&mut self) {
            // Expected to abort the geometry.
            panic!();
        }
        fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
            assert_ne!(a, b);
            assert_ne!(a, c);
            assert_ne!(b, c);
        }
        fn abort_geometry(&mut self) {}
    }

    impl StrokeGeometryBuilder for Builder {
        fn add_stroke_vertex(&mut self, _: StrokeVertex) -> Result<VertexId, GeometryBuilderError> {
            if self.max_vertices == 0 {
                return Err(GeometryBuilderError::TooManyVertices);
            }
            self.max_vertices -= 1;
            Ok(VertexId(self.max_vertices))
        }
    }

    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = StrokeTessellator::new();
    let options = StrokeOptions::tolerance(0.05);

    assert_eq!(
        tess.tessellate(&path, &options, &mut Builder { max_vertices: 0 }),
        Err(TessellationError::GeometryBuilder(
            GeometryBuilderError::TooManyVertices
        )),
    );
    assert_eq!(
        tess.tessellate(&path, &options, &mut Builder { max_vertices: 10 }),
        Err(TessellationError::GeometryBuilder(
            GeometryBuilderError::TooManyVertices
        )),
    );

    assert_eq!(
        tess.tessellate(&path, &options, &mut Builder { max_vertices: 100 }),
        Err(TessellationError::GeometryBuilder(
            GeometryBuilderError::TooManyVertices
        )),
    );
}

#[test]
fn stroke_vertex_source_01() {
    let mut path = crate::path::Path::builder_with_attributes(1);
    let a = path.begin(point(0.0, 0.0), &[1.0]);
    let b = path.line_to(point(10.0, 10.0), &[2.0]);
    let c = path.quadratic_bezier_to(point(10.0, 20.0), point(0.0, 20.0), &[3.0]);
    path.end(true);

    let path = path.build();

    let mut tess = StrokeTessellator::new();
    tess.tessellate_with_ids(
        &mut path.id_iter(),
        &path,
        Some(&path),
        &StrokeOptions::default().with_variable_line_width(0),
        &mut CheckVertexSources {
            next_vertex: 0,
            a,
            b,
            c,
        },
    )
    .unwrap();

    struct CheckVertexSources {
        next_vertex: u32,
        a: EndpointId,
        b: EndpointId,
        c: EndpointId,
    }

    impl GeometryBuilder for CheckVertexSources {
        fn add_triangle(&mut self, _: VertexId, _: VertexId, _: VertexId) {}
        fn abort_geometry(&mut self) {}
    }

    fn eq(a: Point, b: Point) -> bool {
        (a.x - b.x).abs() < 0.00001 && (a.y - b.y).abs() < 0.00001
    }

    impl StrokeGeometryBuilder for CheckVertexSources {
        fn add_stroke_vertex(
            &mut self,
            mut attr: StrokeVertex,
        ) -> Result<VertexId, GeometryBuilderError> {
            let pos = attr.position_on_path();
            let src = attr.source();
            if eq(pos, point(0.0, 0.0)) {
                assert_eq!(src, VertexSource::Endpoint { id: self.a })
            } else if eq(pos, point(10.0, 10.0)) {
                assert_eq!(src, VertexSource::Endpoint { id: self.b })
            } else if eq(pos, point(0.0, 20.0)) {
                assert_eq!(src, VertexSource::Endpoint { id: self.c })
            } else {
                match src {
                    VertexSource::Edge { from, to, t } => {
                        assert_eq!(from, self.b);
                        assert_eq!(to, self.c);
                        assert!(t < 1.0);
                        assert!(t > 0.0);
                    }
                    _ => panic!("{:?} at {:?}", src, pos),
                }
            }

            let vertex = attr.interpolated_attributes();
            if eq(pos, point(0.0, 0.0)) {
                assert_eq!(vertex, &[1.0])
            } else if eq(pos, point(10.0, 10.0)) {
                assert_eq!(vertex, &[2.0])
            } else if eq(pos, point(0.0, 20.0)) {
                assert_eq!(vertex, &[3.0])
            } else {
                assert_eq!(vertex.len(), 1);
                assert!(vertex[0] > 2.0);
                assert!(vertex[0] < 3.0);
            }

            let id = self.next_vertex;
            self.next_vertex += 1;

            Ok(VertexId(id))
        }
    }
}

#[test]
fn test_line_width() {
    use crate::geom::euclid::approxeq::ApproxEq;
    use crate::math::{point, Point};
    let mut builder = crate::path::Path::builder();
    builder.begin(point(0.0, 1.0));
    builder.line_to(point(2.0, 1.0));
    builder.end(false);
    let path = builder.build();

    let options = StrokeOptions::DEFAULT.with_line_width(2.0);
    let mut geometry: VertexBuffers<Point, u16> = VertexBuffers::new();
    StrokeTessellator::new()
        .tessellate(
            path.iter(),
            &options,
            &mut crate::geometry_builder::simple_builder(&mut geometry),
        )
        .unwrap();

    for p in &geometry.vertices {
        assert!(
            p.approx_eq(&point(0.0, 0.0))
                || p.approx_eq(&point(0.0, 2.0))
                || p.approx_eq(&point(2.0, 0.0))
                || p.approx_eq(&point(2.0, 2.0))
        );
    }
}

trait IsNan {
    fn is_nan(&self) -> bool;
}

impl IsNan for f32 {
    fn is_nan(&self) -> bool {
        f32::is_nan(*self)
    }
}

impl IsNan for Point {
    fn is_nan(&self) -> bool {
        self.x.is_nan() || self.y.is_nan()
    }
}

impl IsNan for Vector {
    fn is_nan(&self) -> bool {
        self.x.is_nan() || self.y.is_nan()
    }
}

fn find_sharp_turn(curve: &QuadraticBezierSegment<f32>) -> Option<f32> {
    // TODO: The various thresholds here should take the line width into account.

    let baseline = curve.to - curve.from;
    let v = curve.ctrl - curve.from;
    let n = vector(-baseline.y, baseline.x);
    let v_dot_b = v.dot(baseline);
    let v_dot_n = v.dot(n);

    // If the projection of the control point on the baseline is between the endpoint, we
    // can only get a sharp turn with a control point that is very far away.
    let long_axis = if (v_dot_b >= 0.0 && v_dot_b <= baseline.dot(baseline))
        || v_dot_n.abs() * 2.0 >= v_dot_b.abs()
    {
        // The control point is far enough from the endpoints It can cause a sharp turn.
        if baseline.square_length() * 30.0 > v.square_length() {
            return None;
        }

        v
    } else {
        baseline
    };

    // Rotate the curve to find its extremum along the long axis, where we should split to
    // avoid the sharp turn.
    let rot = crate::geom::euclid::Rotation2D::new(-long_axis.angle_from_x_axis());
    let rotated = QuadraticBezierSegment {
        from: point(0.0, 0.0),
        ctrl: rot.transform_vector(v).to_point(),
        to: rot.transform_vector(baseline).to_point(),
    };

    rotated.local_x_extremum_t()
}

#[test]
fn test_triangle_winding() {
    use crate::math::{point, Point};
    use crate::GeometryBuilder;

    struct Builder {
        vertices: Vec<Point>,
    }

    impl GeometryBuilder for Builder {
        fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
            let a = self.vertices[a.to_usize()];
            let b = self.vertices[b.to_usize()];
            let c = self.vertices[c.to_usize()];
            assert!((b - a).cross(c - b) <= 0.0);
        }
    }

    impl StrokeGeometryBuilder for Builder {
        fn add_stroke_vertex(&mut self, v: StrokeVertex) -> Result<VertexId, GeometryBuilderError> {
            let id = VertexId(self.vertices.len() as u32);
            self.vertices.push(v.position());

            Ok(id)
        }
    }

    let mut path = Path::builder().with_svg();
    path.move_to(point(0.0, 0.0));
    path.quadratic_bezier_to(point(100.0, 0.0), point(100.0, 100.0));
    let path = path.build();

    let mut tess = StrokeTessellator::new();
    let options = StrokeOptions::tolerance(0.05);

    tess.tessellate(
        &path,
        &options,
        &mut Builder {
            vertices: Vec::new(),
        },
    )
    .unwrap();
}

#[test]
fn single_segment_closed() {
    let mut path = Path::builder().with_svg();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(100.0, 0.0));
    let path = path.build();

    let mut tess = StrokeTessellator::new();
    let options = StrokeOptions::tolerance(0.05);
    let mut output: VertexBuffers<Point, u16> = VertexBuffers::new();
    tess.tessellate(&path, &options, &mut simple_builder(&mut output))
        .unwrap();

    assert!(!output.indices.is_empty());

    let mut path = Path::builder_with_attributes(1);
    path.begin(point(0.0, 0.0), &[1.0]);
    path.line_to(point(100.0, 0.0), &[1.0]);
    path.end(false);
    let path = path.build();

    let mut tess = StrokeTessellator::new();
    let options = StrokeOptions::tolerance(0.05);
    let mut output: VertexBuffers<Point, u16> = VertexBuffers::new();
    tess.tessellate(&path, &options, &mut simple_builder(&mut output))
        .unwrap();

    assert!(!output.indices.is_empty());
}

#[test]
fn issue_819() {
    // In this test case, the last point of the path is within merge range
    // of both the first and second points while the latter ones aren't within
    // merge range of one-another. As a result they are both skipped when
    // closing the path.

    let mut path = Path::builder();
    path.begin(point(650.539978027344, 173.559997558594));
    path.line_to(point(650.609985351562, 173.529998779297));
    path.line_to(point(650.729980468750, 173.630004882812));
    path.line_to(point(650.559997558594, 173.570007324219));
    path.end(true);

    let mut tess = StrokeTessellator::new();
    let options = StrokeOptions::tolerance(0.1);
    let mut output: VertexBuffers<Point, u16> = VertexBuffers::new();
    tess.tessellate(&path.build(), &options, &mut simple_builder(&mut output))
        .unwrap();

    let mut path = Path::builder_with_attributes(1);
    path.begin(point(650.539978027344, 173.559997558594), &[0.0]);
    path.line_to(point(650.609985351562, 173.529998779297), &[0.0]);
    path.line_to(point(650.729980468750, 173.630004882812), &[0.0]);
    path.line_to(point(650.559997558594, 173.570007324219), &[0.0]);
    path.end(true);

    let mut tess = StrokeTessellator::new();
    let options = StrokeOptions::tolerance(0.1);
    let mut output: VertexBuffers<Point, u16> = VertexBuffers::new();
    tess.tessellate(&path.build(), &options, &mut simple_builder(&mut output))
        .unwrap();
}

#[test]
fn issue_821() {
    let mut tessellator = StrokeTessellator::new();

    let options = StrokeOptions::default()
        .with_tolerance(0.001)
        .with_line_cap(LineCap::Round)
        .with_variable_line_width(0);

    let mut path = Path::builder_with_attributes(1);
    path.begin(point(-45.192276, -69.800575), &[1.0]);
    path.line_to(point(-45.164116, -69.84931), &[1.0]);
    path.line_to(point(-45.135952, -69.90564), &[1.0]);
    path.line_to(point(-45.10779, -69.93248), &[1.0]);
    path.line_to(point(-45.052788, -69.96064), &[1.0]);
    path.line_to(point(-45.026604, -69.898155), &[1.0]);

    path.end(false);

    let path = path.build();
    let mut mesh = VertexBuffers::new();
    let mut builder = simple_builder(&mut mesh);
    tessellator
        .tessellate_path(&path, &options, &mut builder)
        .unwrap();
}

#[test]
fn issue_894() {
    struct VariableWidthStrokeCtor;
    impl StrokeVertexConstructor<[f32; 2]> for VariableWidthStrokeCtor {
        fn new_vertex(&mut self, vertex: StrokeVertex) -> [f32; 2] {
            vertex.position().to_array()
        }
    }

    const STROKE_WIDTH: lyon_path::AttributeIndex = 0;

    let mut builder = Path::builder_with_attributes(1);
    builder.begin(point(435.72, 368.42), &[38.82]);
    builder.line_to(point(433.53, 366.06), &[38.82]);
    builder.quadratic_bezier_to(point(431.35, 363.70), point(430.22, 362.52), &[39.59]);
    builder.quadratic_bezier_to(point(429.09, 361.34), point(429.05, 362.14), &[41.62]);
    builder.line_to(point(429.00, 362.95), &[41.63]);
    builder.end(false);
    let path = builder.build();
    let mut stroke_tessellator = StrokeTessellator::new();
    let mut geometry: crate::VertexBuffers<[f32; 2], u16> = crate::VertexBuffers::new();
    _ = stroke_tessellator.tessellate_path(
        &path,
        &StrokeOptions::tolerance(0.01)
            .with_line_cap(LineCap::Round)
            .with_variable_line_width(STROKE_WIDTH),
        &mut BuffersBuilder::new(&mut geometry, VariableWidthStrokeCtor),
    );
}

#[test]
fn issue_959() {
    let mut builder = Path::builder();
    builder.begin(point(10.0, 10.0));
    builder.line_to(point(50.0, 10.0));
    builder.line_to(point(50.0, 50.0));
    builder.end(false);
    let path = builder.build();

    let mut tessellator = StrokeTessellator::new();

    for line_width in [1e10, 1e20, 1e30, f32::MAX] {
        let options = StrokeOptions::default()
            .with_line_width(line_width)
            .with_line_cap(LineCap::Round)
            .with_line_join(LineJoin::Round);

        let mut mesh: VertexBuffers<Point, u32> = VertexBuffers::new();
        tessellator.tessellate_path(
            &path,
            &options,
            &mut BuffersBuilder::new(&mut mesh, |v: StrokeVertex| v.position()),
        ).unwrap();
    }
}

#[test]
fn correct_miter_clip_length() {
    let path_max_y = 300.0;
    let line_width = 30.0;
    let miter_limit = 3.0;

    let mut path = Path::builder();
    path.begin(point(-70.0, -200.0));
    path.line_to(point(0.0, path_max_y));
    path.line_to(point(70.0, -200.0));
    path.end(false);
    let path = path.build();

    let mut tessellator = StrokeTessellator::new();

    let options = StrokeOptions::default()
        .with_line_width(line_width)
        .with_line_join(LineJoin::MiterClip)
        .with_miter_limit(miter_limit);

    let mut mesh = VertexBuffers::new();
    let mut builder = simple_builder(&mut mesh);
    tessellator
        .tessellate_path(&path, &options, &mut builder)
        .unwrap();

    let max_y = mesh.vertices.iter()
        .map(|p| p.y)
        .reduce(|acc, y| acc.max(y))
        .unwrap();

    // miter_length =  line_width * miter_limit
    // miter_clip = miter_length * 0.5
    let expected_max_y = path_max_y + line_width * miter_limit * 0.5;

    assert_eq!(expected_max_y, max_y);
}
