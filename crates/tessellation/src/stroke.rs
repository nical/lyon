use crate::geom::utils::{normalized_tangent, tangent};
use crate::geom::{CubicBezierSegment, Line, LineSegment, QuadraticBezierSegment};
use crate::math::*;
use crate::math_utils::compute_normal;
use crate::path::builder::{Build, PathBuilder};
use crate::path::polygon::Polygon;
use crate::path::private::DebugValidator;
use crate::path::{
    AttributeStore, EndpointId, IdEvent, PathEvent, PathSlice, PositionStore, Winding,
};
use crate::{
    LineCap, LineJoin, Side, StrokeOptions, TessellationError, TessellationResult, VertexSource,
    SimpleAttributeStore,
};
use crate::{StrokeGeometryBuilder, VertexId};
use crate::variable_stroke::{VariableStrokeBuilder, VariableStrokeBuilderImpl};

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
}

impl StrokeTessellator {
    pub fn new() -> Self {
        StrokeTessellator {
            attrib_buffer: Vec::new(),
            builder_attrib_store: SimpleAttributeStore::new(0),
        }
    }

    /// Compute the tessellation from a path iterator.
    pub fn tessellate(
        &mut self,
        input: impl IntoIterator<Item = PathEvent>,
        options: &StrokeOptions,
        builder: &mut dyn StrokeGeometryBuilder,
    ) -> TessellationResult {
        assert!(options.variable_line_width.is_none());

        self.attrib_buffer.clear();
        let mut stroker = StrokeBuilderImpl::new(options, &mut self.attrib_buffer, builder);

        let mut id = EndpointId(0);
        let mut validator = DebugValidator::new();

        for event in input {
            match event {
                PathEvent::Begin { at } => {
                    validator.begin();
                    stroker.begin(at, id);
                    id.0 += 1;
                }
                PathEvent::Line { to, .. } => {
                    validator.edge();
                    stroker.edge_to(to, id, 1.0, true, &());
                    id.0 += 1;
                }
                PathEvent::Quadratic { ctrl, to, .. } => {
                    validator.edge();
                    stroker.quadratic_bezier_to(ctrl, to, id, &());
                    id.0 += 1;
                }
                PathEvent::Cubic {ctrl1, ctrl2, to, .. } => {
                    validator.edge();
                    stroker.cubic_bezier_to(ctrl1, ctrl2, to, id, &());
                    id.0 += 1;
                }
                PathEvent::End { close, .. } => {
                    validator.end();
                    stroker.end(close, &());
                }
            }

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
        output: &mut dyn StrokeGeometryBuilder,
    ) -> TessellationResult {
        let custom_attributes = custom_attributes.unwrap_or(&());

        self.attrib_buffer.clear();
        for _ in 0..custom_attributes.num_attributes() {
            self.attrib_buffer.push(0.0);
        }

        if options.variable_line_width.is_some() {
            let stroker = VariableStrokeBuilderImpl::new(
                options,
                &mut self.attrib_buffer,
                output,
            );

            stroker.tessellate_with_ids(
                path,
                positions,
                custom_attributes,
            )
        } else {
            let stroker = StrokeBuilderImpl::new(
                options,
                &mut self.attrib_buffer,
                output,
            );

            stroker.tessellate_with_ids(
                path,
                positions,
                custom_attributes,
            )
        }
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
    /// builder.begin(point(0.0, 0.0), &[]);
    /// builder.line_to(point(10.0, 0.0), &[]);
    /// builder.line_to(point(10.0, 10.0), &[]);
    /// builder.line_to(point(0.0, 10.0), &[]);
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
        assert!(options.variable_line_width.is_none()); // TODO

        self.builder_attrib_store.reset(0);
        self.attrib_buffer.clear();
        StrokeBuilder {
            builder: StrokeBuilderImpl::new(options, &mut self.attrib_buffer, output),
            attrib_store: &mut self.builder_attrib_store,
            validator: DebugValidator::new(),
        }
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
    ) -> VariableStrokeBuilder<'l> {

        self.builder_attrib_store.reset(num_attributes);
        self.attrib_buffer.clear();
        for _ in 0..num_attributes {
            self.attrib_buffer.push(0.0);
        }

        VariableStrokeBuilder::new(
            options,
            &mut self.attrib_buffer,
            &mut self.builder_attrib_store,
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
        builder.add_rectangle(rect, Winding::Positive, &[]);

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
        builder.add_circle(center, radius, Winding::Positive, &[]);

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
        builder.add_ellipse(center, radii, x_rotation, winding, &[]);

        builder.build()
    }
}

/// A builder that tessellates a stroke directly without allocating any intermediate data structure.
struct StrokeBuilderImpl<'l> {
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
    previous_front_side: Side,
    nth: u32,
    length: f32,
    sub_path_start_length: f32,
    options: StrokeOptions,
    error: Option<TessellationError>,
    output: &'l mut dyn StrokeGeometryBuilder,
    vertex: StrokeVertexData<'l>,
    next_endpoint_id: EndpointId,
}

pub struct StrokeBuilder<'l> {
    builder: StrokeBuilderImpl<'l>,
    attrib_store: &'l mut SimpleAttributeStore,
    validator: DebugValidator,
}

impl<'l> StrokeBuilder<'l> {
    #[inline]
    pub fn set_line_join(&mut self, join: LineJoin) {
        self.builder.options.line_join = join;
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
}

impl<'l> Build for StrokeBuilder<'l> {
    type PathType = TessellationResult;

    fn build(self) -> TessellationResult {
        self.builder.build()
    }
}

impl<'l> PathBuilder for StrokeBuilder<'l> {
    fn num_attributes(&self) -> usize {
        self.attrib_store.num_attributes()
    }

    fn begin(&mut self, to: Point, attributes: &[f32]) -> EndpointId {
        self.validator.begin();
        let id = self.attrib_store.add(attributes);
        self.builder.begin(to, id);

        id
    }

    fn end(&mut self, close: bool) {
        self.validator.end();
        self.builder.end(close, self.attrib_store);
    }

    fn line_to(&mut self, to: Point, attributes: &[f32]) -> EndpointId {
        let id = self.attrib_store.add(attributes);
        self.validator.edge();
        self.builder.edge_to(to, id, 1.0, true, self.attrib_store);

        id
    }

    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point, attributes: &[f32]) -> EndpointId {
        self.validator.edge();
        let id = self.attrib_store.add(attributes);
        self.builder.quadratic_bezier_to(ctrl, to, id, self.attrib_store);

        id
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point, attributes: &[f32]) -> EndpointId {
        self.validator.edge();
        let id = self.attrib_store.add(attributes);
        self.builder.cubic_bezier_to(ctrl1, ctrl2, to, id, self.attrib_store);

        id
    }

    fn add_rectangle(&mut self, rect: &Box2D, winding: Winding, attributes: &[f32]) {
        // The thin rectangle approximation for works best with miter joins. We
        // only use it with other joins if the rectangle is much smaller than the
        // line width.
        let threshold = match self.builder.options.line_join {
            LineJoin::Miter => 1.0,
            _ => 0.05,
        } * self.builder.options.line_width;

        if rect.width().abs() < threshold || rect.height().abs() < threshold {
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

impl<'l> StrokeBuilderImpl<'l> {
    fn new(
        options: &StrokeOptions,
        attrib_buffer: &'l mut Vec<f32>,
        output: &'l mut dyn StrokeGeometryBuilder,
    ) -> Self {
        output.begin_geometry();

        let zero = Point::new(0.0, 0.0);
        StrokeBuilderImpl {
            first: zero,
            second: zero,
            previous: zero,
            current: zero,
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
            previous_front_side: Side::Positive, // per convention
            nth: 0,
            length: 0.0,
            sub_path_start_length: 0.0,
            options: *options,
            error: None,
            output,
            vertex: StrokeVertexData {
                position_on_path: zero,
                normal: vector(0.0, 0.0),
                half_width: options.line_width * 0.5,
                advancement: 0.0,
                buffer: attrib_buffer,
                side: Side::Positive,
                src: VertexSource::Endpoint {
                    id: EndpointId::INVALID,
                },
                buffer_is_valid: false,
            },
            next_endpoint_id: EndpointId(0),
        }
    }

    fn set_options(&mut self, options: &StrokeOptions) {
        self.options = *options;
    }

    fn next_endpoint_id(&mut self) -> EndpointId {
        let id = self.next_endpoint_id;
        self.next_endpoint_id.0 += 1;

        id
    }

    #[cold]
    fn error<E: Into<TessellationError>>(&mut self, e: E) {
        if self.error.is_none() {
            self.error = Some(e.into());
        }
    }

    fn tessellate_with_ids(
        mut self,
        path: impl IntoIterator<Item = IdEvent>,
        positions: &impl PositionStore,
        attributes: &dyn AttributeStore,
    ) -> TessellationResult {
        let mut validator = DebugValidator::new();
        for evt in path.into_iter() {
            match evt {
                IdEvent::Begin { at } => {
                    validator.begin();
                    self.begin(positions.get_endpoint(at), at);
                }
                IdEvent::Line { to, .. } => {
                    validator.edge();
                    self.edge_to(positions.get_endpoint(to), to, 1.0, true, attributes);
                }
                IdEvent::Quadratic { ctrl, to, .. } => {
                    validator.edge();
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
                            self.edge_to(point, to, t, first, attributes);
                            self.previous_endpoint = previous_endpoint;
                            first = false;
                        },
                    );
                }
                IdEvent::Cubic {
                    ctrl1, ctrl2, to, ..
                } => {
                    validator.edge();
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
                            self.edge_to(point, to, t, first, attributes);
                            self.previous_endpoint = previous_endpoint;
                            first = false;
                        },
                    );
                }
                IdEvent::End { close, .. } => {
                    validator.end();

                    if close {
                        self.close(attributes);
                    } else if let Err(e) = self.end_subpath(attributes) {
                        self.error(e);
                    }
                }
            }

            if let Some(err) = self.error {
                self.output.abort_geometry();
                return Err(err);
            }
        }

        self.build()
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

    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point, id: EndpointId, attributes: &dyn AttributeStore) {
        let mut first = true;
        let curve = QuadraticBezierSegment {
            from: self.current,
            ctrl,
            to,
        };
        curve.for_each_flattened_with_t(self.options.tolerance, &mut |point, t| {
            self.edge_to(point, id, t, first, attributes);
            first = false;
        });
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point, id: EndpointId, attributes: &dyn AttributeStore) {
        let mut first = true;
        let curve = CubicBezierSegment {
            from: self.current,
            ctrl1,
            ctrl2,
            to,
        };
        curve.for_each_flattened_with_t(self.options.tolerance, &mut |point, t| {
            self.edge_to(point, id, t, first, attributes);
            first = false;
        });
    }

    fn close(&mut self, attributes: &dyn AttributeStore) {
        // If we close almost at the first edge, then we have to
        // skip connecting the last and first edges otherwise the
        // normal will be plagued with floating point precision
        // issues.
        let threshold = 0.001;
        if (self.first - self.current).square_length() > threshold {
            let first = self.first;
            self.edge_to(first, self.first_endpoint, 1.0, true, attributes);
            if self.error.is_some() {
                return;
            }
        }

        if self.nth > 1 {
            let second = self.second;
            self.edge_to(second, self.second_endpoint, self.second_t, true, attributes);
            if self.error.is_some() {
                return;
            }

            self.output.add_triangle(
                self.previous_right_id,
                self.previous_left_id,
                self.second_right_id,
            );
            self.output.add_triangle(
                self.previous_left_id,
                self.second_left_id,
                self.second_right_id,
            );
        }

        self.sub_path_start_length = self.length;
    }

    fn end(&mut self, close: bool, attributes: &dyn AttributeStore) {
        if self.error.is_some() {
            return;
        }

        if close {
            self.close(attributes);
        } else if let Err(e) = self.end_subpath(attributes) {
            self.error(e);
        }
    }

    fn end_subpath(&mut self, attributes: &dyn AttributeStore) -> Result<(), TessellationError> {
        self.vertex.src = VertexSource::Endpoint {
            id: self.current_endpoint,
        };
        self.vertex.buffer_is_valid = false;

        if self.nth == 0 {
            self.vertex.advancement = 0.0;

            match self.options.start_cap {
                LineCap::Square => {
                    // Even if there is no edge, if we are using square caps we have to place a square
                    // at the current position.
                    tessellate_empty_square_cap(self.current, &mut self.vertex, attributes, self.output)?;
                }
                LineCap::Round => {
                    // Same thing for round caps.
                    tessellate_empty_round_cap(self.current, &self.options, &mut self.vertex, attributes, self.output)?;
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
            self.edge_to(p, self.previous_endpoint, 1.0, true, attributes);
            if let Some(e) = &self.error {
                return Err(e.clone());
            }

            // Restore the real current position.
            self.current = current;

            if self.options.end_cap == LineCap::Round {
                let left_id = self.previous_left_id;
                let right_id = self.previous_right_id;
                let left_normal = vector(-d.y, d.x);
                tessellate_round_cap(
                    current,
                    self.options.line_width * 0.5,
                    left_normal,
                    left_id,
                    right_id,
                    d,
                    &self.options,
                    false,
                    &mut self.vertex,
                    attributes,
                    self.output,
                )?;
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

            self.vertex.src = VertexSource::Endpoint {
                id: self.first_endpoint,
            };
            self.vertex.advancement = self.sub_path_start_length;
            self.vertex.position_on_path = first;

            self.vertex.normal = n1;
            self.vertex.side = Side::Positive;

            let first_left_id = self
                .output
                .add_stroke_vertex(StrokeVertex(&mut self.vertex, attributes))?;

            self.vertex.normal = n2;
            self.vertex.side = Side::Negative;

            let first_right_id = self
                .output
                .add_stroke_vertex(StrokeVertex(&mut self.vertex, attributes))?;

            if self.options.start_cap == LineCap::Round {
                let right_normal = vector(-d.y, d.x);
                tessellate_round_cap(
                    first,
                    self.options.line_width * 0.5,
                    right_normal,
                    first_right_id,
                    first_left_id,
                    d,
                    &self.options,
                    true,
                    &mut self.vertex,
                    attributes,
                    self.output,
                )?;
            }

            self.output
                .add_triangle(first_right_id, first_left_id, self.second_right_id);
            self.output
                .add_triangle(first_left_id, self.second_left_id, self.second_right_id);
        }

        Ok(())
    }

    fn edge_to(&mut self, to: Point, endpoint: EndpointId, t: f32, with_join: bool, attributes: &dyn AttributeStore) {
        if (to - self.current).square_length() < self.options.tolerance * self.options.tolerance {
            return;
        }

        if self.error.is_some() {
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
            match self.tessellate_join(previous_edge, next_edge, join_type, attributes) {
                Ok(value) => value,
                Err(e) => {
                    self.error(e);
                    return;
                }
            };

        debug_assert!(end_left_id != end_right_id);

        // Tessellate the edge
        if self.nth > 1 {
            match self.previous_front_side {
                Side::Positive => {
                    self.output.add_triangle(
                        self.previous_right_id,
                        self.previous_left_id,
                        start_right_id,
                    );
                    self.output
                        .add_triangle(self.previous_left_id, start_left_id, start_right_id);
                }
                Side::Negative => {
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

    fn create_back_vertex(
        &mut self,
        prev_tangent: Vector,
        next_tangent: Vector,
        prev_length: f32,
        next_length: f32,
        front_side: Side,
        front_normal: Vector,
        attributes: &dyn AttributeStore,
    ) -> Result<Option<VertexId>, TessellationError> {
        // We must watch out for special cases where the previous or next edge is small relative
        // to the line width. Our workaround only applies to "sharp" angles (more than 90 degrees).
        let angle_is_sharp = next_tangent.dot(prev_tangent) < 0.0;

        if angle_is_sharp {
            // Project the back vertex on the previous and next edges and subtract the edge length
            // to see if the back vertex ends up further than the opposite endpoint of the edge.
            let extruded_normal = front_normal * self.options.line_width * 0.5;
            let d_next = extruded_normal.dot(-next_tangent) - next_length;
            let d_prev = extruded_normal.dot(prev_tangent) - prev_length;

            if d_next.min(d_prev) > 0.0 {
                // Case of an overlapping stroke. In order to prevent the back vertex to create a
                // spike outside of the stroke, we simply don't create it and we'll "fold" the join
                // instead.
                return Ok(None);
            }
        }

        // Common case.

        self.vertex.normal = -front_normal;
        self.vertex.side = front_side.opposite();

        let back_vertex = self
            .output
            .add_stroke_vertex(StrokeVertex(&mut self.vertex, attributes))?;

        Ok(Some(back_vertex))
    }

    fn tessellate_join(
        &mut self,
        previous_edge: Vector,
        next_edge: Vector,
        mut join_type: LineJoin,
        attributes: &dyn AttributeStore,
    ) -> Result<(VertexId, VertexId, VertexId, VertexId, Side), TessellationError> {
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
            (Side::Positive, normal)
        } else {
            (Side::Negative, -normal)
        };

        self.vertex.src = src;
        self.vertex.buffer_is_valid = false;
        self.vertex.advancement = self.length;
        self.vertex.position_on_path = self.current;

        let back_vertex = self.create_back_vertex(
            prev_tangent,
            next_tangent,
            previous_edge_length,
            next_edge_length,
            front_side,
            front_normal,
            attributes,
        )?;

        let threshold = 0.95;
        if prev_tangent.dot(next_tangent) >= threshold {
            // The two edges are almost aligned, just use a simple miter join.
            // TODO: the 0.95 threshold above is completely arbitrary and needs
            // adjustments.
            join_type = LineJoin::Miter;
        } else if join_type == LineJoin::Miter
            && (self.miter_limit_is_exceeded(normal) || back_vertex.is_none())
        {
            // Per SVG spec: If the stroke-miterlimit is exceeded, the line join
            // falls back to bevel.
            join_type = LineJoin::Bevel;
        } else if join_type == LineJoin::MiterClip
            && back_vertex.is_some()
            && !self.miter_limit_is_exceeded(normal)
        {
            join_type = LineJoin::Miter;
        }

        let (front_start_vertex, front_end_vertex) = match join_type {
            LineJoin::Round => {
                self.tessellate_round_join(prev_tangent, next_tangent, front_side, back_vertex, attributes)?
            }
            LineJoin::Bevel => {
                self.tessellate_bevel_join(prev_tangent, next_tangent, front_side, back_vertex, attributes)?
            }
            LineJoin::MiterClip => self.tessellate_miter_clip_join(
                prev_tangent,
                next_tangent,
                front_side,
                back_vertex,
                normal,
                attributes,
            )?,
            LineJoin::Miter => {
                self.vertex.normal = front_normal;
                self.vertex.side = front_side;
                let front_vertex = self
                    .output
                    .add_stroke_vertex(StrokeVertex(&mut self.vertex, attributes))?;

                debug_assert!(back_vertex.is_some());

                (front_vertex, front_vertex)
            }
        };

        let mut start_0 = front_start_vertex;
        let mut start_1 = back_vertex.unwrap_or(front_end_vertex);
        let mut end_0 = front_end_vertex;
        let mut end_1 = back_vertex.unwrap_or(front_start_vertex);
        if front_side == Side::Negative {
            std::mem::swap(&mut start_0, &mut start_1);
            std::mem::swap(&mut end_0, &mut end_1);
        }

        Ok((start_0, start_1, end_0, end_1, front_side))
    }

    fn tessellate_bevel_join(
        &mut self,
        prev_tangent: Vector,
        next_tangent: Vector,
        front_side: Side,
        back_vertex: Option<VertexId>,
        attribute_store: &dyn AttributeStore,
    ) -> Result<(VertexId, VertexId), TessellationError> {
        let sign = front_side.to_f32();
        let previous_normal = vector(-prev_tangent.y, prev_tangent.x);
        let next_normal = vector(-next_tangent.y, next_tangent.x);

        self.vertex.normal = previous_normal * sign;
        self.vertex.side = front_side;

        let front_start_vertex = self
            .output
            .add_stroke_vertex(StrokeVertex(&mut self.vertex, attribute_store))?;

        self.vertex.normal = next_normal * sign;
        self.vertex.side = front_side;

        let front_end_vertex = self
            .output
            .add_stroke_vertex(StrokeVertex(&mut self.vertex, attribute_store))?;

        if let Some(back_vertex) = back_vertex {
            let (v1, v2, v3) = if front_side.is_positive() {
                (front_start_vertex, front_end_vertex, back_vertex)
            } else {
                (front_end_vertex, front_start_vertex, back_vertex)
            };
            self.output.add_triangle(v1, v2, v3);
        }

        Ok((front_start_vertex, front_end_vertex))
    }

    fn tessellate_round_join(
        &mut self,
        prev_tangent: Vector,
        next_tangent: Vector,
        front_side: Side,
        back_vertex: Option<VertexId>,
        attributes: &dyn AttributeStore,
    ) -> Result<(VertexId, VertexId), TessellationError> {
        let radius = self.options.line_width * 0.5;
        let sign = front_side.to_f32();

        // Calculate the initial front normal.
        let start_normal = vector(-prev_tangent.y, prev_tangent.x) * sign;
        let end_normal = vector(-next_tangent.y, next_tangent.x) * sign;

        // We need to pick the final angle such that it's
        let mut start_angle = start_normal.angle_from_x_axis();
        let diff = start_angle.angle_to(end_normal.angle_from_x_axis());
        let mut end_angle = start_angle + diff;

        // Compute the required number of subdivisions,
        let arc_len = radius.abs() * diff.radians.abs();
        let step = circle_flattening_step(radius, self.options.tolerance);
        let num_segments = (arc_len / step).ceil();
        let num_subdivisions = num_segments.log2() as u32 * 2;

        // Create start and end front vertices.
        self.vertex.side = front_side;

        self.vertex.normal = start_normal;
        let front_start_vertex = self
            .output
            .add_stroke_vertex(StrokeVertex(&mut self.vertex, attributes))?;

        self.vertex.normal = end_normal;
        let front_end_vertex = self
            .output
            .add_stroke_vertex(StrokeVertex(&mut self.vertex, attributes))?;

        let mut v0 = front_start_vertex;
        let mut v1 = front_end_vertex;

        if front_side.is_negative() {
            std::mem::swap(&mut v0, &mut v1);
            std::mem::swap(&mut start_angle, &mut end_angle);
        }

        // Add the triangle joining the back vertex and the start/end front vertices.
        if let Some(back_vertex) = back_vertex {
            self.output.add_triangle(back_vertex, v0, v1);
        }

        tessellate_arc(
            (start_angle.radians, end_angle.radians),
            radius,
            v0,
            v1,
            num_subdivisions,
            &mut self.vertex,
            attributes,
            self.output,
        )?;

        Ok((front_start_vertex, front_end_vertex))
    }

    fn tessellate_miter_clip_join(
        &mut self,
        prev_tangent: Vector,
        next_tangent: Vector,
        front_side: Side,
        back_vertex: Option<VertexId>,
        normal: Vector,
        attribute_store: &dyn AttributeStore,
    ) -> Result<(VertexId, VertexId), TessellationError> {
        let sign = front_side.to_f32();
        let previous_normal: Vector = vector(-prev_tangent.y, prev_tangent.x);
        let next_normal: Vector = vector(-next_tangent.y, next_tangent.x);

        let (v1, v2) = self.get_clip_intersections(previous_normal, next_normal, normal);

        self.vertex.normal = v1 * sign;
        self.vertex.side = front_side;

        let front_start_vertex = self
            .output
            .add_stroke_vertex(StrokeVertex(&mut self.vertex, attribute_store))?;

        self.vertex.normal = v2 * sign;
        self.vertex.side = front_side;

        let front_end_vertex = self
            .output
            .add_stroke_vertex(StrokeVertex(&mut self.vertex, attribute_store))?;

        if let Some(back_vertex) = back_vertex {
            let (v1, v2, v3) = if front_side.is_positive() {
                (back_vertex, front_start_vertex, front_end_vertex)
            } else {
                (back_vertex, front_end_vertex, front_start_vertex)
            };
            self.output.add_triangle(v1, v2, v3);
        }

        Ok((front_start_vertex, front_end_vertex))
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
            point: normal.normalize().to_point() * self.options.miter_limit * 0.5,
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
            .unwrap_or_else(|| normal.to_point())
            .to_vector();
        let i2 = clip_line
            .intersection(&next_line)
            .unwrap_or_else(|| normal.to_point())
            .to_vector();

        (i1, i2)
    }

    fn build(self) -> TessellationResult {
        Ok(self.output.end_geometry())
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
    output: &mut dyn StrokeGeometryBuilder,
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
    let arc_len = radius.abs() * diff.radians.abs();
    let step = circle_flattening_step(radius, options.tolerance);
    let num_segments = (arc_len / step).ceil();
    let num_subdivisions = num_segments.log2() as u32 * 2;

    vertex.position_on_path = center;
    vertex.half_width = radius;
    vertex.side = first_side;

    vertex.normal = edge_normal.normalize();
    let mid_vertex = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;

    output.add_triangle(start_vertex, mid_vertex, end_vertex);

    tessellate_arc(
        (start_angle.radians, mid_angle.radians),
        radius,
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
        radius,
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
    output: &mut dyn StrokeGeometryBuilder,
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
    output: &mut dyn StrokeGeometryBuilder,
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
    radius: f32,
    va: VertexId,
    vb: VertexId,
    num_recursions: u32,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut dyn StrokeGeometryBuilder,
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
        radius,
        va,
        vertex_id,
        num_recursions - 1,
        vertex,
        attributes,
        output,
    )?;
    tessellate_arc(
        (mid_angle, angle.1),
        radius,
        vertex_id,
        vb,
        num_recursions - 1,
        vertex,
        attributes,
        output,
    )
}

// A fall-back that avoids off artifacts with zero-area rectangles as
// well as overlapping triangles if the rectangle is much smaller than the
// line width in any dimension.
#[inline(never)]
fn approximate_thin_rectangle(builder: &mut StrokeBuilder, rect: &Box2D, attributes: &[f32]) {
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
        LineJoin::Round => LineCap::Round,
        _ => LineCap::Square,
    };

    builder.builder.options.line_width += d;
    builder.builder.options.start_cap = cap;
    builder.builder.options.end_cap = cap;

    builder.add_line_segment(&LineSegment { from, to }, attributes);

    // Restore the builder options.
    builder.builder.options = options;
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
    #[inline]
    pub fn normal(&self) -> Vector {
        self.0.normal
    }

    /// Position of this vertex on the path, unaffected by the line width.
    #[inline]
    pub fn position_on_path(&self) -> Point {
        self.0.position_on_path
    }

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

    /// Returns the source of this vertex.
    #[inline]
    pub fn interpolated_attributes(&mut self) -> &[f32] {
        if self.0.buffer_is_valid {
            return &self.0.buffer[..];
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
            attributes: StrokeVertex,
        ) -> Result<VertexId, GeometryBuilderError> {
            assert!(!attributes.position().x.is_nan());
            assert!(!attributes.position().y.is_nan());
            assert!(!attributes.normal().x.is_nan());
            assert!(!attributes.normal().y.is_nan());
            assert!(attributes.normal().square_length() != 0.0);
            assert!(!attributes.advancement().is_nan());
            self.builder.add_stroke_vertex(attributes)
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

    builder.add_point(point(1.0, 0.0), &[]);
    builder.add_point(point(2.0, 0.0), &[]);
    builder.add_point(point(3.0, 0.0), &[]);

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
        fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
            assert!(a != b);
            assert!(a != c);
            assert!(b != c);
        }
        fn end_geometry(&mut self) -> Count {
            // Expected to abort the geometry.
            panic!();
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
    let mut tess = StrokeTessellator::new();

    let check = &mut CheckVertexSources {
        next_vertex: 0,
        a: EndpointId(0),
        b: EndpointId(1),
        c: EndpointId(2),
    };

    {
        // Test the direct builder API.
        let mut builder = tess.builder_with_attributes(
            1,
            &StrokeOptions::DEFAULT,
            check,
        );

        let a = builder.begin(point(0.0, 0.0), &[1.0]);
        let b = builder.line_to(point(1.0, 1.0), &[2.0]);
        let c = builder.quadratic_bezier_to(point(1.0, 2.0), point(0.0, 2.0), &[3.0]);
        builder.end(true);
        builder.build().unwrap();

        assert_eq!(a, EndpointId(0));
        assert_eq!(b, EndpointId(1));
        assert_eq!(c, EndpointId(2));
    }

    // Test the path API.
    let mut path = crate::path::Path::builder_with_attributes(1);
    let a = path.begin(point(0.0, 0.0), &[1.0]);
    let b = path.line_to(point(1.0, 1.0), &[2.0]);
    let c = path.quadratic_bezier_to(point(1.0, 2.0), point(0.0, 2.0), &[3.0]);
    path.end(true);

    let path = path.build();

    tess.tessellate_with_ids(
        &mut path.id_iter(),
        &path,
        Some(&path),
        &StrokeOptions::default(),
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
            mut attr: StrokeVertex,
        ) -> Result<VertexId, GeometryBuilderError> {
            let pos = attr.position_on_path();
            let src = attr.source();
            if eq(pos, point(0.0, 0.0)) {
                assert_eq!(src, VertexSource::Endpoint { id: self.a })
            } else if eq(pos, point(1.0, 1.0)) {
                assert_eq!(src, VertexSource::Endpoint { id: self.b })
            } else if eq(pos, point(0.0, 2.0)) {
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
            if eq(pos, point(0.0, 0.0)) {
                assert_eq!(attributes, &[1.0])
            } else if eq(pos, point(1.0, 1.0)) {
                assert_eq!(attributes, &[2.0])
            } else if eq(pos, point(0.0, 2.0)) {
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
