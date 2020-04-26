use crate::math::*;
use crate::geom::euclid::Trig;
use crate::geom::utils::{directed_angle, normalized_tangent, tangent};
use crate::geom::{CubicBezierSegment, Line, QuadraticBezierSegment};
use crate::math_utils::compute_normal;
use crate::path::builder::{Build, PathBuilder};
use crate::path::private::DebugValidator;
use crate::path::{AttributeStore, EndpointId, IdEvent, PathEvent, PathSlice, PositionStore, Winding};
use crate::path::polygon::Polygon;
use crate::{GeometryBuilderError, StrokeGeometryBuilder, VertexId};
use crate::{
    LineCap, LineJoin, Order, Side, StrokeOptions, TessellationError, TessellationResult,
    VertexSource,
};

use std::f32::consts::PI;
const EPSILON: f32 = 1e-4;

/// A Context object that can tessellate stroke operations for complex paths.
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
/// See https://github.com/nical/lyon/wiki/Stroke-tessellation for some notes
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
}

impl StrokeTessellator {
    pub fn new() -> Self {
        StrokeTessellator {
            attrib_buffer: Vec::new(),
        }
    }

    /// Compute the tessellation from a path iterator.
    pub fn tessellate(
        &mut self,
        input: impl IntoIterator<Item = PathEvent>,
        options: &StrokeOptions,
        builder: &mut dyn StrokeGeometryBuilder,
    ) -> TessellationResult {
        let mut stroker = StrokeBuilder::new(options, &(), &mut self.attrib_buffer, builder);

        for evt in input {
            stroker.path_event(evt);
            if let Some(error) = stroker.error {
                stroker.output.abort_geometry();
                return Err(error);
            }
        }

        stroker.build()
    }

    /// Compute the tessellation from a path iterator.
    pub fn tessellate_with_ids(
        &mut self,
        path: impl IntoIterator<Item = IdEvent>,
        positions: &impl PositionStore,
        custom_attributes: Option<&dyn AttributeStore>,
        options: &StrokeOptions,
        builder: &mut dyn StrokeGeometryBuilder,
    ) -> TessellationResult {
        let custom_attributes = custom_attributes.unwrap_or(&());
        let mut stroker = StrokeBuilder::new(
            options,
            custom_attributes,
            &mut self.attrib_buffer,
            builder,
        );

        stroker.tessellate_with_ids(path, positions);

        stroker.build()
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
        let path = path.into();

        if path.num_attributes() > 0 {
            self.tessellate_with_ids(path.id_iter(), &path, Some(&path), options, builder)
        } else {
            self.tessellate(path.iter(), options, builder)
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
    /// use lyon_tessellation::path::traits::*;
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
    ) -> StrokeBuilder<'l> {
        StrokeBuilder::new(options, &(), &mut self.attrib_buffer, output)
    }

    /// Tessellate the stroke for a `Polygon`.
    pub fn tessellate_polygon(
        &mut self,
        polygon: Polygon<Point>,
        options: &StrokeOptions,
        output: &mut dyn StrokeGeometryBuilder,
    ) -> TessellationResult {
        self.tessellate(
            polygon.path_events(),
            options,
            output,
        )
    }

    /// Tessellate the stroke for an axis-aligned rectangle.
    pub fn tessellate_rectangle(
        &mut self,
        rect: &Rect,
        options: &StrokeOptions,
        output: &mut dyn StrokeGeometryBuilder,
    ) -> TessellationResult {
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

macro_rules! add_vertex {
    ($builder: expr, position: $position: expr) => {{
        let mut position = $position;

        if $builder.options.apply_line_width {
            position += $builder.attributes.normal * $builder.options.line_width / 2.0;
        }

        let res = $builder
            .output
            .add_stroke_vertex(position, StrokeAttributes(&mut $builder.attributes));

        match res {
            Ok(v) => v,
            Err(e) => {
                $builder.builder_error(e);
                VertexId(0)
            }
        }
    }};
}

/// A builder that tessellates a stroke directly without allocating any intermediate data structure.
pub struct StrokeBuilder<'l> {
    first: Point,
    previous: Point,
    current: Point,
    second: Point,
    first_endpoint: EndpointId,
    previous_endpoint: EndpointId,
    current_endpoint: EndpointId,
    current_t: f32,
    second_endpoint: EndpointId,
    second_t: f32,
    previous_left_id: VertexId,
    previous_right_id: VertexId,
    second_left_id: VertexId,
    second_right_id: VertexId,
    previous_normal: Vector,
    previous_front_side: Side,
    nth: u32,
    length: f32,
    sub_path_start_length: f32,
    options: StrokeOptions,
    error: Option<TessellationError>,
    output: &'l mut dyn StrokeGeometryBuilder,
    attributes: StrokeAttributesData<'l>,
    validator: DebugValidator,
    next_endpoint_id: EndpointId,
}

impl<'l> Build for StrokeBuilder<'l> {
    type PathType = TessellationResult;

    fn build(self) -> TessellationResult {
        self.validator.build();

        Ok(self.output.end_geometry())
    }
}

impl<'l> PathBuilder for StrokeBuilder<'l> {
    fn begin(&mut self, to: Point) -> EndpointId {
        self.validator.begin();
        let id = self.next_endpoint_id();
        self.begin(to, id);

        id
    }

    fn end(&mut self, close: bool) {
        self.validator.end();
        if close {
            self.close();
        } else {
            self.end_subpath();
        }
    }

    fn line_to(&mut self, to: Point) -> EndpointId {
        self.validator.edge();
        let id = self.next_endpoint_id();
        self.edge_to(to, id, 1.0, true);

        id
    }

    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) -> EndpointId {
        self.validator.edge();
        let id = self.next_endpoint_id();
        let mut first = true;
        QuadraticBezierSegment {
            from: self.current,
            ctrl,
            to,
        }
        .for_each_flattened_with_t(self.options.tolerance, &mut |point, t| {
            self.edge_to(point, id, t, first);
            first = false;
        });

        id
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) -> EndpointId {
        self.validator.edge();
        let id = self.next_endpoint_id();
        let mut first = true;
        CubicBezierSegment {
            from: self.current,
            ctrl1,
            ctrl2,
            to,
        }
        .for_each_flattened_with_t(self.options.tolerance, &mut |point, t| {
            self.edge_to(point, id, t, first);
            first = false;
        });

        id
    }

    fn add_rectangle(&mut self, rect: &Rect, winding: Winding) {
        if rect.size.width.abs() < self.options.line_width
            || rect.size.height.abs() < self.options.line_width {

            let _ = tess_thin_rectangle(rect, &self.options, self.output);
            return;
        }

        match winding {
            Winding::Positive => self.add_polygon(Polygon {
                points: &[
                    rect.min(),
                    point(rect.max_x(), rect.min_y()),
                    rect.max(),
                    point(rect.min_x(), rect.max_y()),
                ],
                closed: true,
            }),
            Winding::Negative => self.add_polygon(Polygon {
                points: &[
                    rect.min(),
                    point(rect.min_x(), rect.max_y()),
                    rect.max(),
                    point(rect.max_x(), rect.min_y()),
                ],
                closed: true,
            }),
        };
    }
}

impl<'l> StrokeBuilder<'l> {
    fn new(
        options: &StrokeOptions,
        attrib_store: &'l dyn AttributeStore,
        attrib_buffer: &'l mut Vec<f32>,
        output: &'l mut dyn StrokeGeometryBuilder,
    ) -> Self {
        attrib_buffer.clear();
        for _ in 0..attrib_store.num_attributes() {
            attrib_buffer.push(0.0);
        }

        output.begin_geometry();

        let zero = Point::new(0.0, 0.0);
        StrokeBuilder {
            first: zero,
            second: zero,
            previous: zero,
            current: zero,
            previous_normal: Vector::new(0.0, 0.0),
            previous_left_id: VertexId(0),
            previous_right_id: VertexId(0),
            second_left_id: VertexId(0),
            second_right_id: VertexId(0),
            current_endpoint: EndpointId::INVALID,
            first_endpoint: EndpointId::INVALID,
            previous_endpoint: EndpointId::INVALID,
            second_endpoint: EndpointId::INVALID,
            current_t: 0.0,
            second_t: 0.0,
            previous_front_side: Side::Left, // per convention
            nth: 0,
            length: 0.0,
            sub_path_start_length: 0.0,
            options: *options,
            error: None,
            output,
            attributes: StrokeAttributesData {
                normal: vector(0.0, 0.0),
                advancement: 0.0,
                buffer: attrib_buffer,
                store: attrib_store,
                side: Side::Left,
                src: VertexSource::Endpoint {
                    id: EndpointId::INVALID,
                },
                buffer_is_valid: false,
            },
            validator: DebugValidator::new(),
            next_endpoint_id: EndpointId(0),
        }
    }

    fn set_options(&mut self, options: &StrokeOptions) {
        self.options = *options;
    }

    #[inline]
    pub fn set_line_join(&mut self, join: LineJoin) {
        self.options.line_join = join;
    }

    #[inline]
    pub fn set_start_cap(&mut self, cap: LineCap) {
        self.options.start_cap = cap;
    }

    #[inline]
    pub fn set_end_cap(&mut self, cap: LineCap) {
        self.options.end_cap = cap;
    }

    #[inline]
    pub fn set_miter_limit(&mut self, limit: f32) {
        self.options.miter_limit = limit;
    }

    fn next_endpoint_id(&mut self) -> EndpointId {
        let id = self.next_endpoint_id;
        self.next_endpoint_id.0 += 1;

        id
    }

    #[cold]
    fn builder_error(&mut self, e: GeometryBuilderError) {
        if self.error.is_none() {
            self.error = Some(e.into());
        }
    }

    fn tessellate_with_ids(
        &mut self,
        path: impl IntoIterator<Item = IdEvent>,
        positions: &impl PositionStore,
    ) {
        for evt in path.into_iter() {
            match evt {
                IdEvent::Begin { at } => {
                    self.validator.begin();
                    self.begin(positions.get_endpoint(at), at);
                }
                IdEvent::Line { to, .. } => {
                    self.validator.edge();
                    self.edge_to(positions.get_endpoint(to), to, 1.0, true);
                }
                IdEvent::Quadratic { ctrl, to, .. } => {
                    self.validator.edge();
                    let mut first = true;
                    // TODO: This is hacky: edge_to advances the previous
                    // endpoint to the current one but we don't want that
                    // when flattening a curve so we reset it after each
                    // iteration.
                    let previous_endpoint = self.current_endpoint;
                    QuadraticBezierSegment {
                        from: self.current,
                        ctrl: positions.get_control_point(ctrl),
                        to: positions.get_endpoint(to),
                    }
                    .for_each_flattened_with_t(
                        self.options.tolerance,
                        &mut |point, t| {
                            self.edge_to(point, to, t, first);
                            self.previous_endpoint = previous_endpoint;
                            first = false;
                        },
                    );
                }
                IdEvent::Cubic {
                    ctrl1, ctrl2, to, ..
                } => {
                    self.validator.edge();
                    let mut first = true;
                    let previous_endpoint = self.current_endpoint;
                    CubicBezierSegment {
                        from: self.current,
                        ctrl1: positions.get_control_point(ctrl1),
                        ctrl2: positions.get_control_point(ctrl2),
                        to: positions.get_endpoint(to),
                    }
                    .for_each_flattened_with_t(
                        self.options.tolerance,
                        &mut |point, t| {
                            self.edge_to(point, to, t, first);
                            self.previous_endpoint = previous_endpoint;
                            first = false;
                        },
                    );
                }
                IdEvent::End { close, .. } => {
                    self.validator.end();
                    if close {
                        self.close();
                    } else {
                        self.end_subpath();
                    }
                }
            }
        }
    }

    fn begin(&mut self, position: Point, endpoint: EndpointId) {
        self.first = position;
        self.current = position;
        self.first_endpoint = endpoint;
        self.current_endpoint = endpoint;
        self.current_t = 0.0;
        self.nth = 0;
        self.sub_path_start_length = self.length;
    }

    fn close(&mut self) {
        // If we close almost at the first edge, then we have to
        // skip connecting the last and first edges otherwise the
        // normal will be plagued with floating point precision
        // issues.
        let threshold = 0.001;
        if (self.first - self.current).square_length() > threshold {
            let first = self.first;
            self.edge_to(first, self.first_endpoint, 1.0, true);
        }

        if self.nth > 1 {
            let second = self.second;
            self.edge_to(second, self.second_endpoint, self.second_t, true);

            self.attributes.normal = self.previous_normal;
            self.attributes.side = Side::Left;
            self.attributes.src = VertexSource::Endpoint {
                id: self.previous_endpoint,
            };
            self.attributes.advancement = self.sub_path_start_length;
            self.attributes.buffer_is_valid = false;

            let first_left_id = add_vertex!(self, position: self.previous);

            self.attributes.normal = -self.previous_normal;
            self.attributes.side = Side::Right;

            let first_right_id = add_vertex!(self, position: self.previous);

            self.output
                .add_triangle(first_right_id, first_left_id, self.second_right_id);
            self.output
                .add_triangle(first_left_id, self.second_left_id, self.second_right_id);
        }

        self.sub_path_start_length = self.length;
    }

    fn tessellate_empty_square_cap(&mut self) {
        self.attributes.normal = vector(1.0, 1.0);
        self.attributes.side = Side::Right;

        let a = add_vertex!(self, position: self.current);

        self.attributes.normal = vector(1.0, -1.0);
        self.attributes.side = Side::Left;

        let b = add_vertex!(self, position: self.current);

        self.attributes.normal = vector(-1.0, -1.0);
        self.attributes.side = Side::Left;

        let c = add_vertex!(self, position: self.current);

        self.attributes.normal = vector(-1.0, 1.0);
        self.attributes.side = Side::Right;

        let d = add_vertex!(self, position: self.current);

        self.output.add_triangle(a, b, c);
        self.output.add_triangle(a, c, d);
    }

    fn tessellate_empty_round_cap(&mut self) {
        let center = self.current;

        self.attributes.normal = vector(-1.0, 0.0);
        self.attributes.side = Side::Left;

        let left_id = add_vertex!(self, position: center);

        self.attributes.normal = vector(1.0, 0.0);
        self.attributes.side = Side::Right;

        let right_id = add_vertex!(self, position: center);

        self.tessellate_round_cap(center, vector(0.0, -1.0), left_id, right_id, true);
        self.tessellate_round_cap(center, vector(0.0, 1.0), left_id, right_id, false);
    }

    fn end_subpath(&mut self) {
        self.attributes.src = VertexSource::Endpoint {
            id: self.current_endpoint,
        };
        self.attributes.buffer_is_valid = false;

        if self.nth == 0 {
            self.attributes.advancement = 0.0;

            match self.options.start_cap {
                LineCap::Square => {
                    // Even if there is no edge, if we are using square caps we have to place a square
                    // at the current position.
                    self.tessellate_empty_square_cap();
                }
                LineCap::Round => {
                    // Same thing for round caps.
                    self.tessellate_empty_round_cap();
                }
                _ => {}
            }
        }

        // last edge
        if self.nth > 0 {
            let current = self.current;
            let d = self.current - self.previous;
            if self.options.end_cap == LineCap::Square {
                // The easiest way to implement square caps is to lie about the current position
                // and move it slightly to accommodate for the width/2 extra length.
                self.current += d.normalize();
            }
            let p = self.current + d;
            self.edge_to(p, self.previous_endpoint, 1.0, true);
            // Restore the real current position.
            self.current = current;

            if self.options.end_cap == LineCap::Round {
                let left_id = self.previous_left_id;
                let right_id = self.previous_right_id;
                self.tessellate_round_cap(current, d, left_id, right_id, false);
            }
        }
        // first edge
        if self.nth > 1 {
            let mut first = self.first;
            let d = first - self.second;

            if self.options.start_cap == LineCap::Square {
                first += d.normalize();
            }

            let n2 = normalized_tangent(d);
            let n1 = -n2;

            self.attributes.advancement = self.sub_path_start_length;

            self.attributes.normal = n1;
            self.attributes.side = Side::Left;

            let first_left_id = add_vertex!(self, position: first);

            self.attributes.normal = n2;
            self.attributes.side = Side::Right;

            let first_right_id = add_vertex!(self, position: first);

            if self.options.start_cap == LineCap::Round {
                self.tessellate_round_cap(first, d, first_left_id, first_right_id, true);
            }

            self.output
                .add_triangle(first_right_id, first_left_id, self.second_right_id);
            self.output
                .add_triangle(first_left_id, self.second_left_id, self.second_right_id);
        }
    }

    fn edge_to(&mut self, to: Point, endpoint: EndpointId, t: f32, with_join: bool) {
        if to == self.current {
            return;
        }

        if self.nth == 0 {
            // We don't have enough information to compute the previous
            // vertices (and thus the current join) yet.
            self.previous = self.first;
            self.previous_endpoint = self.first_endpoint;
            self.current = to;
            self.current_endpoint = endpoint;
            self.nth += 1;
            return;
        }

        let previous_edge = self.current - self.previous;
        let next_edge = to - self.current;
        let join_type = if with_join {
            self.options.line_join
        } else {
            LineJoin::Miter
        };

        let (start_left_id, start_right_id, end_left_id, end_right_id, front_side) =
            self.tessellate_join(previous_edge, next_edge, join_type);

        // Tessellate the edge
        if self.nth > 1 {
            match self.previous_front_side {
                Side::Left => {
                    self.output.add_triangle(
                        self.previous_right_id,
                        self.previous_left_id,
                        start_right_id,
                    );
                    self.output
                        .add_triangle(self.previous_left_id, start_left_id, start_right_id);
                }
                Side::Right => {
                    self.output.add_triangle(
                        self.previous_right_id,
                        self.previous_left_id,
                        start_left_id,
                    );
                    self.output
                        .add_triangle(self.previous_right_id, start_left_id, start_right_id);
                }
            }
        }

        if self.nth == 1 {
            self.second = self.current;
            self.second_endpoint = self.current_endpoint;
            self.second_t = t;
            self.second_left_id = start_left_id;
            self.second_right_id = start_right_id;
        }

        self.previous_front_side = front_side;
        self.previous = self.current;
        self.previous_endpoint = self.current_endpoint;
        self.previous_left_id = end_left_id;
        self.previous_right_id = end_right_id;
        self.current = to;
        self.current_endpoint = endpoint;
        self.current_t = t;

        self.nth += 1;
    }

    fn tessellate_round_cap(
        &mut self,
        center: Point,
        dir: Vector,
        left: VertexId,
        right: VertexId,
        is_start: bool,
    ) {
        let radius = self.options.line_width.abs();
        if radius < 1e-4 {
            return;
        }

        let arc_len = 0.5 * PI * radius;
        let step = circle_flattening_step(radius, self.options.tolerance);
        let num_segments = (arc_len / step).ceil();
        let num_recursions = num_segments.log2() as u32 * 2;

        let dir = dir.normalize();

        let quarter_angle = if is_start { -PI * 0.5 } else { PI * 0.5 };
        let mid_angle = directed_angle(vector(1.0, 0.0), dir);
        let left_angle = mid_angle + quarter_angle;
        let right_angle = mid_angle - quarter_angle;

        self.attributes.normal = dir;
        self.attributes.side = Side::Left;

        let mid_vertex = add_vertex!(self, position: center);

        let (v1, v2, v3) = if is_start {
            (left, right, mid_vertex)
        } else {
            (left, mid_vertex, right)
        };
        self.output.add_triangle(v1, v2, v3);

        let apply_width = if self.options.apply_line_width {
            self.options.line_width * 0.5
        } else {
            0.0
        };

        if let Err(e) = tess_round_cap(
            center,
            (left_angle, mid_angle),
            radius,
            left,
            mid_vertex,
            num_recursions,
            Side::Left,
            apply_width,
            !is_start,
            &mut self.attributes,
            self.output,
        ) {
            self.builder_error(e);
        }
        if let Err(e) = tess_round_cap(
            center,
            (mid_angle, right_angle),
            radius,
            mid_vertex,
            right,
            num_recursions,
            Side::Right,
            apply_width,
            !is_start,
            &mut self.attributes,
            self.output,
        ) {
            self.builder_error(e);
        }
    }

    fn tessellate_back_join(
        &mut self,
        prev_tangent: Vector,
        next_tangent: Vector,
        prev_length: f32,
        next_length: f32,
        front_side: Side,
        front_normal: Vector,
    ) -> (VertexId, VertexId, Option<Order>) {
        // We must watch out for special cases where the previous or next edge is small relative
        // to the line width inducing an overlap of the stroke of both edges.

        let d_next = -self.options.line_width / 2.0 * front_normal.dot(next_tangent) - next_length;
        let d_prev = -self.options.line_width / 2.0 * front_normal.dot(-prev_tangent) - prev_length;

        let (d, t2, order) = if d_prev > d_next {
            (d_prev, next_tangent, Order::Before)
        } else {
            (d_next, -prev_tangent, Order::After)
        };

        // Case of an overlapping stroke
        // We must build the back join with two vertices in order to respect the correct shape
        // This will induce some overlapping triangles and collinear triangles
        //
        // TODO: Sometimes flattening generates a very small edge at the end of the curve which causes
        // precision issues and cracks in the output geometry here. Work around it with a threshold
        // but it would be better if the flattening code was able to avoid that in the first place.
        if d > 0.01 {
            let n2: Vector = match front_side {
                Side::Right => vector(t2.y, -t2.x),
                Side::Left => vector(-t2.y, t2.x),
            } * if order.is_after() { -1.0 } else { 1.0 };
            let back_end_vertex_normal = -n2;
            let back_start_vertex_normal = vector(0.0, 0.0);

            self.attributes.normal = back_start_vertex_normal;
            self.attributes.side = front_side.opposite();

            let back_start_vertex = add_vertex!(self, position: self.current);

            self.attributes.normal = back_end_vertex_normal;
            self.attributes.side = front_side.opposite();

            let back_end_vertex = add_vertex!(self, position: self.current);
            // return
            return match order {
                Order::Before => (back_start_vertex, back_end_vertex, Some(order)),
                Order::After => (back_end_vertex, back_start_vertex, Some(order)),
            };
        }

        self.attributes.normal = -front_normal;
        self.attributes.side = front_side.opposite();

        // Standard Case
        let back_start_vertex = add_vertex!(self, position: self.current);

        let back_end_vertex = back_start_vertex;
        (back_start_vertex, back_end_vertex, None)
    }

    fn tessellate_join(
        &mut self,
        previous_edge: Vector,
        next_edge: Vector,
        mut join_type: LineJoin,
    ) -> (VertexId, VertexId, VertexId, VertexId, Side) {
        // This function needs to differentiate the "front" of the join (aka. the pointy side)
        // from the back. The front is where subdivision or adjustments may be needed.
        let prev_tangent = previous_edge.normalize();
        let next_tangent = next_edge.normalize();
        let previous_edge_length = previous_edge.length();
        let next_edge_length = next_edge.length();
        self.length += previous_edge_length;

        let src = if self.current_t == 0.0 || self.current_t == 1.0 {
            VertexSource::Endpoint {
                id: self.current_endpoint,
            }
        } else {
            VertexSource::Edge {
                from: self.previous_endpoint,
                to: self.current_endpoint,
                t: self.current_t,
            }
        };

        let normal = compute_normal(prev_tangent, next_tangent);

        let (front_side, front_normal) = if next_tangent.cross(prev_tangent) >= 0.0 {
            (Side::Left, normal)
        } else {
            (Side::Right, -normal)
        };

        self.attributes.src = src;
        self.attributes.buffer_is_valid = false;
        self.attributes.advancement = self.length;

        let (back_start_vertex, back_end_vertex, order) = self.tessellate_back_join(
            prev_tangent,
            next_tangent,
            previous_edge_length,
            next_edge_length,
            front_side,
            front_normal,
        );

        let threshold = 0.95;
        if prev_tangent.dot(next_tangent) >= threshold {
            // The two edges are almost aligned, just use a simple miter join.
            // TODO: the 0.95 threshold above is completely arbitrary and needs
            // adjustments.
            join_type = LineJoin::Miter;
        } else if join_type == LineJoin::Miter && self.miter_limit_is_exceeded(normal) {
            // Per SVG spec: If the stroke-miterlimit is exceeded, the line join
            // falls back to bevel.
            join_type = LineJoin::Bevel;
        } else if join_type == LineJoin::MiterClip && !self.miter_limit_is_exceeded(normal) {
            join_type = LineJoin::Miter;
        }

        let back_join_vertex = if let Some(_order) = order {
            match _order {
                Order::Before => back_start_vertex,
                Order::After => back_end_vertex,
            }
        } else {
            back_start_vertex
        };

        let (start_vertex, end_vertex) = match join_type {
            LineJoin::Round => {
                self.tessellate_round_join(prev_tangent, next_tangent, front_side, back_join_vertex)
            }
            LineJoin::Bevel => {
                self.tessellate_bevel_join(prev_tangent, next_tangent, front_side, back_join_vertex)
            }
            LineJoin::MiterClip => self.tessellate_miter_clip_join(
                prev_tangent,
                next_tangent,
                front_side,
                back_join_vertex,
                normal,
            ),
            // Fallback to Miter for unimplemented line joins
            _ => {
                self.attributes.normal = front_normal;
                self.attributes.side = front_side;
                let end_vertex = add_vertex!(self, position: self.current);
                self.previous_normal = normal;

                if let Some(_order) = order {
                    let t2 = match _order {
                        Order::Before => next_tangent,
                        Order::After => prev_tangent,
                    };
                    let n1: Vector = match front_side {
                        Side::Right => vector(t2.y, -t2.x),
                        Side::Left => vector(-t2.y, t2.x),
                    };

                    self.attributes.normal = n1;
                    self.attributes.side = front_side;

                    let start_vertex = add_vertex!(self, position: self.current);
                    self.output
                        .add_triangle(start_vertex, end_vertex, back_join_vertex);

                    match _order {
                        Order::Before => (end_vertex, start_vertex),
                        Order::After => (start_vertex, end_vertex),
                    }
                } else {
                    (end_vertex, end_vertex)
                }
            }
        };

        if back_end_vertex != back_start_vertex {
            let (a, b, c) = if let Some(_order) = order {
                match _order {
                    Order::Before => (back_end_vertex, end_vertex, back_start_vertex),
                    Order::After => (back_end_vertex, start_vertex, back_start_vertex),
                }
            } else {
                (back_end_vertex, end_vertex, back_start_vertex)
            };
            // preserve correct ccw winding
            match front_side {
                Side::Left => self.output.add_triangle(a, b, c),
                Side::Right => self.output.add_triangle(a, c, b),
            }
        }

        match front_side {
            Side::Left => (
                start_vertex,
                back_start_vertex,
                end_vertex,
                back_end_vertex,
                front_side,
            ),
            Side::Right => (
                back_start_vertex,
                start_vertex,
                back_end_vertex,
                end_vertex,
                front_side,
            ),
        }
    }

    fn tessellate_bevel_join(
        &mut self,
        prev_tangent: Vector,
        next_tangent: Vector,
        front_side: Side,
        back_vertex: VertexId,
    ) -> (VertexId, VertexId) {
        let neg_if_right = if front_side.is_left() { 1.0 } else { -1.0 };
        let previous_normal = vector(-prev_tangent.y, prev_tangent.x);
        let next_normal = vector(-next_tangent.y, next_tangent.x);

        self.attributes.normal = previous_normal * neg_if_right;
        self.attributes.side = front_side;

        let start_vertex = add_vertex!(self, position: self.current);

        self.attributes.normal = next_normal * neg_if_right;
        self.attributes.side = front_side;

        let last_vertex = add_vertex!(self, position: self.current);

        self.previous_normal = next_normal;

        let (v1, v2, v3) = if front_side.is_left() {
            (start_vertex, last_vertex, back_vertex)
        } else {
            (last_vertex, start_vertex, back_vertex)
        };
        self.output.add_triangle(v1, v2, v3);

        (start_vertex, last_vertex)
    }

    fn tessellate_round_join(
        &mut self,
        prev_tangent: Vector,
        next_tangent: Vector,
        front_side: Side,
        back_vertex: VertexId,
    ) -> (VertexId, VertexId) {
        let join_angle = get_join_angle(prev_tangent, next_tangent);

        let max_radius_segment_angle =
            compute_max_radius_segment_angle(self.options.line_width / 2.0, self.options.tolerance);
        let num_segments = (join_angle.abs() as f32 / max_radius_segment_angle).ceil() as u32;
        debug_assert!(num_segments > 0);
        // Calculate angle of each step
        let segment_angle = join_angle as f32 / num_segments as f32;

        let neg_if_right = if front_side.is_left() { 1.0 } else { -1.0 };

        // Calculate the initial front normal
        let initial_normal = vector(-prev_tangent.y, prev_tangent.x) * neg_if_right;

        self.attributes.normal = initial_normal;
        self.attributes.side = front_side;

        let mut last_vertex = add_vertex!(self, position: self.current);
        let start_vertex = last_vertex;

        // Plot each point along the radius by using a matrix to
        // rotate the normal at each step
        let (sin, cos) = segment_angle.sin_cos();
        let rotation_matrix = [[cos, sin], [-sin, cos]];

        let mut n = initial_normal;
        for _ in 0..num_segments {
            // incrementally rotate the normal
            n = vector(
                n.x * rotation_matrix[0][0] + n.y * rotation_matrix[0][1],
                n.x * rotation_matrix[1][0] + n.y * rotation_matrix[1][1],
            );

            self.attributes.normal = n;
            self.attributes.side = front_side;

            let current_vertex = add_vertex!(self, position: self.current);

            let (v1, v2, v3) = if front_side.is_left() {
                (back_vertex, last_vertex, current_vertex)
            } else {
                (back_vertex, current_vertex, last_vertex)
            };
            self.output.add_triangle(v1, v2, v3);

            last_vertex = current_vertex;
        }

        self.previous_normal = n * neg_if_right;

        (start_vertex, last_vertex)
    }

    fn tessellate_miter_clip_join(
        &mut self,
        prev_tangent: Vector,
        next_tangent: Vector,
        front_side: Side,
        back_vertex: VertexId,
        normal: Vector,
    ) -> (VertexId, VertexId) {
        let neg_if_right = if front_side.is_left() { 1.0 } else { -1.0 };
        let previous_normal: Vector = vector(-prev_tangent.y, prev_tangent.x);
        let next_normal: Vector = vector(-next_tangent.y, next_tangent.x);

        let (v1, v2) = self.get_clip_intersections(previous_normal, next_normal, normal);

        self.attributes.normal = v1 * neg_if_right;
        self.attributes.side = front_side;

        let start_vertex = add_vertex!(self, position: self.current);

        self.attributes.normal = v2 * neg_if_right;
        self.attributes.side = front_side;

        let last_vertex = add_vertex!(self, position: self.current);

        self.previous_normal = normal;

        let (v1, v2, v3) = if front_side.is_left() {
            (back_vertex, start_vertex, last_vertex)
        } else {
            (back_vertex, last_vertex, start_vertex)
        };
        self.output.add_triangle(v1, v2, v3);

        (start_vertex, last_vertex)
    }

    fn miter_limit_is_exceeded(&self, normal: Vector) -> bool {
        normal.square_length() > self.options.miter_limit * self.options.miter_limit * 0.5
    }

    fn get_clip_intersections(
        &self,
        previous_normal: Vector,
        next_normal: Vector,
        normal: Vector,
    ) -> (Vector, Vector) {
        let clip_line = Line {
            point: normal.normalize().to_point() * self.options.miter_limit  * 0.5,
            vector: tangent(normal),
        };

        let prev_line = Line {
            point: previous_normal.to_point(),
            vector: tangent(previous_normal),
        };

        let next_line = Line {
            point: next_normal.to_point(),
            vector: tangent(next_normal),
        };

        let i1 = clip_line
            .intersection(&prev_line)
            .unwrap_or(normal.to_point())
            .to_vector();
        let i2 = clip_line
            .intersection(&next_line)
            .unwrap_or(normal.to_point())
            .to_vector();

        (i1, i2)
    }
}

// Computes the max angle of a radius segment for a given tolerance
fn compute_max_radius_segment_angle(radius: f32, tolerance: f32) -> f32 {
    let t = radius - tolerance;
    ((radius * radius - t * t) * 4.0).sqrt() / radius
}

fn get_join_angle(prev_tangent: Vector, next_tangent: Vector) -> f32 {
    let mut join_angle = Trig::fast_atan2(prev_tangent.y, prev_tangent.x)
        - Trig::fast_atan2(next_tangent.y, next_tangent.x);

    // Make sure to stay within the [-Pi, Pi] range.
    if join_angle > PI {
        join_angle -= 2.0 * PI;
    } else if join_angle < -PI {
        join_angle += 2.0 * PI;
    }

    join_angle
}

fn tess_round_cap(
    center: Point,
    angle: (f32, f32),
    radius: f32,
    va: VertexId,
    vb: VertexId,
    num_recursions: u32,
    side: Side,
    line_width: f32,
    invert_winding: bool,
    attributes: &mut StrokeAttributesData,
    output: &mut dyn StrokeGeometryBuilder,
) -> Result<(), GeometryBuilderError> {
    if num_recursions == 0 {
        return Ok(());
    }

    let mid_angle = (angle.0 + angle.1) * 0.5;

    let normal = vector(mid_angle.cos(), mid_angle.sin());

    attributes.normal = normal;
    attributes.side = side;

    let vertex =
        output.add_stroke_vertex(center + normal * line_width, StrokeAttributes(attributes))?;

    let (v1, v2, v3) = if invert_winding {
        (vertex, vb, va)
    } else {
        (vertex, va, vb)
    };
    output.add_triangle(v1, v2, v3);

    tess_round_cap(
        center,
        (angle.0, mid_angle),
        radius,
        va,
        vertex,
        num_recursions - 1,
        side,
        line_width,
        invert_winding,
        attributes,
        output,
    )?;
    tess_round_cap(
        center,
        (mid_angle, angle.1),
        radius,
        vertex,
        vb,
        num_recursions - 1,
        side,
        line_width,
        invert_winding,
        attributes,
        output,
    )
}

// A fall-back that avoids off artifacts with zero-area rectangles as
// well as overlapping triangles if the rectangle is smaller than the
// line width in any dimension.
#[inline(never)]
fn tess_thin_rectangle(
    rect: &Rect,
    options: &StrokeOptions,
    output: &mut dyn StrokeGeometryBuilder,
) -> Result<(), TessellationError> {
    let rect = if options.apply_line_width {
        let w = options.line_width * 0.5;
        rect.inflate(w, w)
    } else {
        *rect
    };

    let mut attributes = StrokeAttributesData {
        normal: vector(-1.0, -1.0),
        advancement: 0.0,
        side: Side::Left,
        src: VertexSource::Endpoint {
            id: EndpointId::INVALID,
        },
        store: &(),
        buffer: &mut [],
        buffer_is_valid: true,
    };

    let a = output.add_stroke_vertex(rect.origin, StrokeAttributes(&mut attributes))?;

    attributes.normal = vector(-1.0, 1.0);
    attributes.advancement += rect.size.height;

    let b = output.add_stroke_vertex(point(rect.min_x(), rect.max_y()), StrokeAttributes(&mut attributes))?;

    attributes.side = Side::Right;

    attributes.normal = vector(1.0, 1.0);
    attributes.advancement += rect.size.width;

    let c = output.add_stroke_vertex(point(rect.max_x(), rect.max_y()), StrokeAttributes(&mut attributes))?;

    attributes.normal = vector(1.0, -1.0);
    attributes.advancement += rect.size.height;

    let d = output.add_stroke_vertex(point(rect.max_x(), rect.min_y()), StrokeAttributes(&mut attributes))?;

    output.add_triangle(a, b, c);
    output.add_triangle(a, c, d);

    Ok(())
}

/// Extra vertex information from the `StrokeTessellator`.
pub(crate) struct StrokeAttributesData<'l> {
    pub(crate) normal: Vector,
    pub(crate) advancement: f32,
    pub(crate) side: Side,
    pub(crate) src: VertexSource,
    pub(crate) store: &'l dyn AttributeStore,
    pub(crate) buffer: &'l mut [f32],
    pub(crate) buffer_is_valid: bool,
}

/// Extra vertex information from the `StrokeTessellator` accessible when building vertices.
pub struct StrokeAttributes<'a, 'b>(pub(crate) &'b mut StrokeAttributesData<'a>);

impl<'a, 'b> StrokeAttributes<'a, 'b> {
    /// Normal at this vertex such that extruding the vertices along the normal would
    /// produce a stroke of width 2.0 (1.0 on each side). This vector is not normalized.
    #[inline]
    pub fn normal(&self) -> Vector {
        self.0.normal
    }

    /// How far along the path this vertex is.
    #[inline]
    pub fn advancement(&self) -> f32 {
        self.0.advancement
    }

    /// Whether the vertex is on the left or right side of the path.
    #[inline]
    pub fn side(&self) -> Side {
        self.0.side
    }

    /// Returns the source of this vertex.
    #[inline]
    pub fn source(&self) -> VertexSource {
        self.0.src
    }

    /// Returns the source of this vertex.
    #[inline]
    pub fn interpolated_attributes(&mut self) -> &[f32] {
        if self.0.buffer_is_valid {
            return &self.0.buffer[..];
        }

        match self.0.src {
            VertexSource::Endpoint { id } => self.0.store.get(id),
            VertexSource::Edge { from, to, t } => {
                let a = self.0.store.get(from);
                let b = self.0.store.get(to);
                for i in 0..self.0.buffer.len() {
                    self.0.buffer[i] = a[i] * (1.0 - t) + b[i] * t;
                }
                self.0.buffer_is_valid = true;

                &self.0.buffer[..]
            }
        }
    }
}

fn circle_flattening_step(radius: f32, mut tolerance: f32) -> f32 {
    // Don't allow high tolerance values (compared to the radius) to avoid edge cases.
    tolerance = f32::min(tolerance, radius);
    2.0 * f32::sqrt(2.0 * tolerance * radius - tolerance * tolerance)
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
        fn end_geometry(&mut self) -> Count {
            self.builder.end_geometry()
        }
        fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
            assert!(a != b);
            assert!(a != c);
            assert!(b != c);
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
            position: Point,
            attributes: StrokeAttributes,
        ) -> Result<VertexId, GeometryBuilderError> {
            assert!(!position.x.is_nan());
            assert!(!position.y.is_nan());
            assert!(!attributes.normal().x.is_nan());
            assert!(!attributes.normal().y.is_nan());
            assert!(attributes.normal().square_length() != 0.0);
            assert!(!attributes.advancement().is_nan());
            self.builder.add_stroke_vertex(position, attributes)
        }
    }

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    let mut tess = StrokeTessellator::new();
    let count = tess
        .tessellate(
            path,
            &options,
            &mut TestBuilder {
                builder: simple_builder(&mut buffers),
            },
        )
        .unwrap();

    if let Some(triangles) = expected_triangle_count {
        assert_eq!(
            triangles,
            count.indices / 3,
            "Unexpected number of triangles"
        );
    }
}

#[test]
fn test_square() {
    let mut builder = Path::builder();

    builder.begin(point(-1.0, 1.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(1.0, -1.0));
    builder.line_to(point(-1.0, -1.0));
    builder.end(true);

    let path1 = builder.build();

    let mut builder = Path::builder();

    builder.begin(point(-1.0, -1.0));
    builder.line_to(point(1.0, -1.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(-1.0, 1.0));
    builder.end(true);

    let path2 = builder.build();

    test_path(
        path1.as_slice(),
        &StrokeOptions::default().with_line_join(LineJoin::Miter),
        Some(8),
    );
    test_path(
        path2.as_slice(),
        &StrokeOptions::default().with_line_join(LineJoin::Miter),
        Some(8),
    );

    test_path(
        path1.as_slice(),
        &StrokeOptions::default().with_line_join(LineJoin::Bevel),
        Some(12),
    );
    test_path(
        path2.as_slice(),
        &StrokeOptions::default().with_line_join(LineJoin::Bevel),
        Some(12),
    );

    test_path(
        path1.as_slice(),
        &StrokeOptions::default()
            .with_line_join(LineJoin::MiterClip)
            .with_miter_limit(1.0),
        Some(12),
    );
    test_path(
        path2.as_slice(),
        &StrokeOptions::default()
            .with_line_join(LineJoin::MiterClip)
            .with_miter_limit(1.0),
        Some(12),
    );

    test_path(
        path1.as_slice(),
        &StrokeOptions::tolerance(0.001).with_line_join(LineJoin::Round),
        None,
    );
    test_path(
        path2.as_slice(),
        &StrokeOptions::tolerance(0.001).with_line_join(LineJoin::Round),
        None,
    );
}

#[test]
fn test_empty_path() {
    let path = Path::builder().build();
    test_path(path.as_slice(), &StrokeOptions::default(), Some(0));
}

#[test]
fn test_empty_caps() {
    let mut builder = Path::builder();

    builder.add_point(point(1.0, 0.0));
    builder.add_point(point(2.0, 0.0));
    builder.add_point(point(3.0, 0.0));

    let path = builder.build();

    test_path(
        path.as_slice(),
        &StrokeOptions::default().with_line_cap(LineCap::Butt),
        Some(0),
    );
    test_path(
        path.as_slice(),
        &StrokeOptions::default().with_line_cap(LineCap::Square),
        Some(6),
    );
    test_path(
        path.as_slice(),
        &StrokeOptions::default().with_line_cap(LineCap::Round),
        None,
    );
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
        fn add_triangle(&mut self, _a: VertexId, _b: VertexId, _c: VertexId) {}
        fn end_geometry(&mut self) -> Count {
            Count {
                vertices: 0,
                indices: 0,
            }
        }
        fn abort_geometry(&mut self) {}
    }

    impl StrokeGeometryBuilder for Builder {
        fn add_stroke_vertex(
            &mut self,
            _position: Point,
            _: StrokeAttributes,
        ) -> Result<VertexId, GeometryBuilderError> {
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
        Err(TessellationError::TooManyVertices),
    );
    assert_eq!(
        tess.tessellate(&path, &options, &mut Builder { max_vertices: 10 }),
        Err(TessellationError::TooManyVertices),
    );

    assert_eq!(
        tess.tessellate(&path, &options, &mut Builder { max_vertices: 100 }),
        Err(TessellationError::TooManyVertices),
    );
}

#[test]
fn stroke_vertex_source_01() {
    let mut path = crate::path::Path::builder_with_attributes(1);
    let a = path.begin(point(0.0, 0.0), &[1.0]);
    let b = path.line_to(point(1.0, 1.0), &[2.0]);
    let c = path.quadratic_bezier_to(point(1.0, 2.0), point(0.0, 2.0), &[3.0]);
    path.end(true);

    let path = path.build();

    let mut tess = StrokeTessellator::new();
    tess.tessellate_with_ids(
        &mut path.id_iter(),
        &path,
        Some(&path),
        &StrokeOptions::default().dont_apply_line_width(),
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
        fn begin_geometry(&mut self) {}
        fn end_geometry(&mut self) -> Count {
            Count {
                vertices: self.next_vertex,
                indices: 0,
            }
        }
        fn abort_geometry(&mut self) {}
        fn add_triangle(&mut self, _: VertexId, _: VertexId, _: VertexId) {}
    }

    fn eq(a: Point, b: Point) -> bool {
        (a.x - b.x).abs() < 0.00001 && (a.y - b.y).abs() < 0.00001
    }

    impl StrokeGeometryBuilder for CheckVertexSources {
        fn add_stroke_vertex(
            &mut self,
            v: Point,
            mut attr: StrokeAttributes,
        ) -> Result<VertexId, GeometryBuilderError> {
            let src = attr.source();
            if eq(v, point(0.0, 0.0)) {
                assert_eq!(src, VertexSource::Endpoint { id: self.a })
            } else if eq(v, point(1.0, 1.0)) {
                assert_eq!(src, VertexSource::Endpoint { id: self.b })
            } else if eq(v, point(0.0, 2.0)) {
                assert_eq!(src, VertexSource::Endpoint { id: self.c })
            } else {
                match src {
                    VertexSource::Edge { from, to, t } => {
                        assert_eq!(from, self.b);
                        assert_eq!(to, self.c);
                        assert!(t < 1.0);
                        assert!(t > 0.0);
                    }
                    _ => panic!(),
                }
            }

            let attributes = attr.interpolated_attributes();
            if eq(v, point(0.0, 0.0)) {
                assert_eq!(attributes, &[1.0])
            } else if eq(v, point(1.0, 1.0)) {
                assert_eq!(attributes, &[2.0])
            } else if eq(v, point(0.0, 2.0)) {
                assert_eq!(attributes, &[3.0])
            } else {
                assert_eq!(attributes.len(), 1);
                assert!(attributes[0] > 2.0);
                assert!(attributes[0] < 3.0);
            }

            let id = self.next_vertex;
            self.next_vertex += 1;

            Ok(VertexId(id))
        }
    }
}
