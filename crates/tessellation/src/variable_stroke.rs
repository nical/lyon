use crate::geom::{
    Line, LineSegment, QuadraticBezierSegment, CubicBezierSegment,
    arrayvec::ArrayVec,
    utils::tangent,
};
use crate::math::*;
use crate::math_utils::compute_normal;
use crate::path::private::DebugValidator;
use crate::path::{
    AttributeStore, EndpointId, IdEvent, PositionStore, Polygon, Winding,
    builder::{PathBuilder, Build},
};
use crate::{
    LineCap, LineJoin, Side, StrokeOptions, TessellationError, VertexSource,
    SimpleAttributeStore, StrokeGeometryBuilder, VertexId, TessellationResult,
};

use crate::stroke::{StrokeVertex, StrokeVertexData};

use std::f32::consts::PI;
const EPSILON: f32 = 1e-4;

const SIDE_POSITIVE: usize = 0;
const SIDE_NEGATIVE: usize = 1;

#[derive(Copy, Clone, Debug)]
struct SidePoints {
    prev: Point,
    next: Point,
    single_vertex: Option<Point>,
    prev_vertex: VertexId,
    next_vertex: VertexId,
}

#[derive(Copy, Clone, Debug)]
struct EndpointData {
    position: Point,
    half_width: f32,
    advancement: f32,
    line_join: LineJoin,
    src: VertexSource,
    side_points: [SidePoints; 2],
    fold: [bool; 2],
}

impl Default for EndpointData {
    fn default() -> Self {
        EndpointData {
            position: Point::zero(),
            half_width: std::f32::NAN,
            advancement: std::f32::NAN,
            line_join: LineJoin::Miter,
            src: VertexSource::Endpoint { id: EndpointId::INVALID },
            side_points: [
                SidePoints {
                    prev: Point::zero(), prev_vertex: VertexId(std::u32::MAX),
                    next: Point::zero(), next_vertex: VertexId(std::u32::MAX),
                    single_vertex: None,
                };
                2
            ],
            fold: [false, false],
        }
    }
}

/// A builder object that tessellates a stroked path via the `PathBuilder`
/// interface.
///
/// Can be created using `StrokeTessellator::builder_with_attributes`.
pub struct VariableStrokeBuilder<'l> {
    builder: VariableStrokeBuilderImpl<'l>,
    attrib_store: &'l mut SimpleAttributeStore,
    validator: DebugValidator,
    prev: (Point, EndpointId, f32),
}

impl<'l> VariableStrokeBuilder<'l> {
    pub(crate) fn new(
        options: &StrokeOptions,
        attrib_buffer: &'l mut Vec<f32>,
        attrib_store: &'l mut SimpleAttributeStore,
        output: &'l mut dyn StrokeGeometryBuilder,
    ) -> Self {
        VariableStrokeBuilder {
            builder: VariableStrokeBuilderImpl::new(
                options,
                attrib_buffer,
                output,
            ),
            attrib_store,
            validator: DebugValidator::new(),
            prev: (Point::zero(), EndpointId::INVALID, 0.0),
        }
    }

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

    fn get_width(&self, attributes: &[f32]) -> f32 {
        if let Some(idx) = self.builder.options.variable_line_width {
            self.builder.options.line_width * attributes[idx as usize]
        } else {
            self.builder.options.line_width
        }
    }
}

impl<'l> PathBuilder for VariableStrokeBuilder<'l> {
    fn num_attributes(&self) -> usize {
        self.attrib_store.num_attributes()
    }

    fn begin(&mut self, to: Point, attributes: &[f32]) -> EndpointId {
        self.validator.begin();
        let id = self.attrib_store.add(attributes);
        let width = self.get_width(attributes);
        self.builder.begin(to, id, width, self.attrib_store);

        id
    }

    fn end(&mut self, close: bool) {
        self.validator.end();
        self.builder.end(close, self.attrib_store);
    }

    fn line_to(&mut self, to: Point, attributes: &[f32]) -> EndpointId {
        let id = self.attrib_store.add(attributes);
        let width = self.get_width(attributes);
        self.validator.edge();
        self.builder.line_to(to, id, width, self.attrib_store);

        self.prev = (to, id, width);

        id
    }

    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point, attributes: &[f32]) -> EndpointId {
        self.validator.edge();
        let id = self.attrib_store.add(attributes);

        let start_width = self.prev.2;
        let end_width = self.get_width(attributes);

        QuadraticBezierSegment {
            from: self.prev.0,
            ctrl,
            to,
        }.for_each_flattened_with_t(
            self.builder.options.tolerance,
            &mut |position, t| {
                let width = start_width * (1.0 - t) + end_width * t;
                let (line_join, src) = if t >= 1.0 {
                    (self.builder.options.line_join, VertexSource::Endpoint { id })
                } else {
                    (LineJoin::Miter, VertexSource::Edge { from: self.prev.1, to: id, t })
                };

                let r = self.builder.step(
                    EndpointData {
                        position,
                        half_width: width * 0.5,
                        line_join,
                        src,
                        ..Default::default()
                    },
                    self.attrib_store,
                );

                if let Err(e) = r {
                    self.builder.error(e);
                }
            },
        );

        self.prev = (to, id, end_width);

        id
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point, attributes: &[f32]) -> EndpointId {
        self.validator.edge();
        let id = self.attrib_store.add(attributes);

        let start_width = self.prev.2;
        let end_width = self.get_width(attributes);

        CubicBezierSegment {
            from: self.prev.0,
            ctrl1,
            ctrl2,
            to,
        }.for_each_flattened_with_t(
            self.builder.options.tolerance,
            &mut |point, t| {
                let width = start_width * (1.0 - t) + end_width * t;
                let (line_join, src) = if t >= 1.0 {
                    (self.builder.options.line_join, VertexSource::Endpoint { id })
                } else {
                    (LineJoin::Miter, VertexSource::Edge { from: self.prev.1, to: id, t })
                };

                let r = self.builder.step(
                    EndpointData {
                        position: point,
                        half_width: width * 0.5,
                        line_join,
                        src,
                        ..Default::default()
                    },
                    self.attrib_store,
                );

                if let Err(e) = r {
                    self.builder.error(e);
                }
            },
        );

        self.prev = (to, id, end_width);

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

        if self.builder.options.variable_line_width.is_none()
            && (rect.width().abs() < threshold || rect.height().abs() < threshold) {

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

impl<'l> Build for VariableStrokeBuilder<'l> {
    type PathType = TessellationResult;

    fn build(self) -> TessellationResult {
        self.builder.build()
    }
}

/// A builder that tessellates a stroke directly without allocating any intermediate data structure.
pub(crate) struct VariableStrokeBuilderImpl<'l> {
    options: StrokeOptions,
    error: Option<TessellationError>,
    output: &'l mut dyn StrokeGeometryBuilder,
    vertex: StrokeVertexData<'l>,
    point_buffer: PointBuffer,
    firsts: ArrayVec<EndpointData, 2>,
    previous: Option<EndpointData>,
    sub_path_start_advancement: f32,
}

impl<'l> VariableStrokeBuilderImpl<'l> {
    pub(crate) fn new(
        options: &StrokeOptions,
        attrib_buffer: &'l mut Vec<f32>,
        output: &'l mut dyn StrokeGeometryBuilder,
    ) -> Self {
        output.begin_geometry();

        let zero = Point::new(0.0, 0.0);
        VariableStrokeBuilderImpl {
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
        }
    }

    #[cold]
    fn error<E: Into<TessellationError>>(&mut self, e: E) {
        if self.error.is_none() {
            self.error = Some(e.into());
        }
    }

    pub(crate) fn tessellate_with_ids(
        mut self,
        path: impl IntoIterator<Item = IdEvent>,
        positions: &impl PositionStore,
        attributes: &dyn AttributeStore,
    ) -> TessellationResult {
        let base_width = self.options.line_width;
        let attrib_index = self.options.variable_line_width.unwrap() as usize;

        let mut validator = DebugValidator::new();

        let mut current_endpoint = EndpointId(std::u32::MAX);
        let mut current_position = point(std::f32::NAN, std::f32::NAN);

        for evt in path.into_iter() {
            match evt {
                IdEvent::Begin { at } => {
                    validator.begin();
                    let width = base_width * attributes.get(at)[attrib_index];
                    current_endpoint = at;
                    current_position = positions.get_endpoint(at);
                    self.begin(current_position, at, width, attributes);
                }
                IdEvent::Line { to, .. } => {
                    validator.edge();
                    let width = base_width * attributes.get(to)[attrib_index];
                    current_endpoint = to;
                    current_position = positions.get_endpoint(to);
                    self.line_to(current_position, to, width, attributes);
                }
                IdEvent::Quadratic { ctrl, to, .. } => {
                    validator.edge();
                    let start_width = base_width * attributes.get(current_endpoint)[attrib_index];
                    let end_width = base_width * attributes.get(to)[attrib_index];

                    let from = current_endpoint;
                    let from_pos = current_position;
                    current_endpoint = to;
                    current_position = positions.get_endpoint(to);

                    QuadraticBezierSegment {
                        from: from_pos,
                        ctrl: positions.get_control_point(ctrl),
                        to: current_position,
                    }.for_each_flattened_with_t(
                        self.options.tolerance,
                        &mut |position, t| {
                            let width = start_width * (1.0 - t) + end_width * t;
                            let (line_join, src) = if t >= 1.0 {
                                (self.options.line_join, VertexSource::Endpoint { id: to })
                            } else {
                                (LineJoin::Miter, VertexSource::Edge { from, to, t })
                            };

                            let r = self.step(
                                EndpointData {
                                    position,
                                    half_width: width * 0.5,
                                    line_join,
                                    src,
                                    ..Default::default()
                                },
                                attributes,
                            );

                            if let Err(e) = r {
                                self.error(e);
                            }
                        },
                    );
                }
                IdEvent::Cubic { ctrl1, ctrl2, to, .. } => {
                    validator.edge();

                    let start_width = base_width * attributes.get(current_endpoint)[attrib_index];
                    let end_width = base_width * attributes.get(to)[attrib_index];

                    let from = current_endpoint;
                    let from_pos = current_position;
                    current_endpoint = to;
                    current_position = positions.get_endpoint(to);

                    CubicBezierSegment {
                        from: from_pos,
                        ctrl1: positions.get_control_point(ctrl1),
                        ctrl2: positions.get_control_point(ctrl2),
                        to: current_position,
                    }.for_each_flattened_with_t(
                        self.options.tolerance,
                        &mut |point, t| {
                            let width = start_width * (1.0 - t) + end_width * t;
                            let (line_join, src) = if t >= 1.0 {
                                (self.options.line_join, VertexSource::Endpoint { id: to })
                            } else {
                                (LineJoin::Miter, VertexSource::Edge { from, to, t })
                            };

                            let r = self.step(
                                EndpointData {
                                    position: point,
                                    half_width: width * 0.5,
                                    line_join,
                                    src,
                                    ..Default::default()
                                },
                                attributes,
                            );

                            if let Err(e) = r {
                                self.error(e);
                            }
                        },
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

    fn begin(&mut self, position: Point, endpoint: EndpointId, width: f32, attributes: &dyn AttributeStore) {
        let half_width = width * 0.5;
        let r = self.step(
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

        if let Err(e) = r {
            self.error(e);            
        }
    }

    fn line_to(&mut self, to: Point, endpoint: EndpointId, width: f32, attributes: &dyn AttributeStore) {
        let half_width = width * 0.5;
        let r = self.step(
            EndpointData {
                position: to,
                half_width,
                line_join: self.options.line_join,
                src: VertexSource::Endpoint { id: endpoint },
                ..Default::default()
            },
            attributes,
        );

        if let Err(e) = r {
            self.error(e);
        }
    }

    fn end(&mut self, close: bool, attributes: &dyn AttributeStore) {
        let res = if close && self.point_buffer.count() > 2{
            self.close(attributes)
        } else {
            self.end_with_caps(attributes)
        };

        self.point_buffer.clear();
        self.firsts.clear();

        if let Err(e) = res {
            self.error(e);
        }
    }

    fn build(self) -> TessellationResult {
        if let Some(err) = self.error {
            self.output.abort_geometry();
            return Err(err);
        }

        Ok(self.output.end_geometry())
    }

    fn close(&mut self, attributes: &dyn AttributeStore) -> Result<(), TessellationError> {
        let p = self.firsts[0];
        self.step(p, attributes)?;

        if self.firsts.len() >= 2 {
            let p2 = self.firsts[1];
            self.step(p2, attributes)?;

            let (p0, p1) = self.point_buffer.last_two_mut();

            add_edge_triangles(p0, p1, self.output);
        }

        Ok(())
    }

    fn end_with_caps(&mut self, attributes: &dyn AttributeStore) -> Result<(), TessellationError> {
        if self.point_buffer.count() > 2 {
            // Last edge.

            // Add a fake fake point p2 aligned with p0 and p1 so that we can tessellate
            // the join for p1. 
            let (p0, p1) = self.point_buffer.last_two_mut();
            tessellate_last_edge(p0, p1, false,  &self.options, &mut self.vertex, attributes, self.output)?;
            self.sub_path_start_advancement = p1.advancement;

            // First edge.
            let mut p0 = self.firsts[0];
            let p1 = &self.firsts[1];
            tessellate_first_edge(&mut p0, p1, &self.options, &mut self.vertex, attributes, self.output)?;
        }

        if self.point_buffer.count() == 2 {
            let (p0, p1) = self.point_buffer.last_two_mut();
            tessellate_last_edge(p0, p1, true, &self.options, &mut self.vertex, attributes, self.output)?;
            self.sub_path_start_advancement = p1.advancement;
            tessellate_first_edge(p0, p1, &self.options, &mut self.vertex, attributes, self.output)?;
            add_edge_triangles(p0, p1, self.output);
        }

        if self.point_buffer.count() == 1 {
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
        }

        Ok(())
    }

    fn step(&mut self, mut next: EndpointData, attributes: &dyn AttributeStore) -> Result<(), TessellationError> {
        let count = self.point_buffer.count();

        if count > 0 && (self.point_buffer.last_mut().position - next.position).square_length() < self.options.tolerance {
            // TODO: should do something like:
            // - add the endpoint
            // - only allow two consecutive endpoints at the same position
            // - if the join type is round, maybe tessellate a round cap for the largest one
            return Ok(());
        }

        if count > 0 {
            let p0 = self.point_buffer.last_mut();
            // Compute the position of the vertices that act as reference the edge between
            // p0 and next
            compute_edge_attachment_positions(p0, &mut next);
        }

        if count > 1 {
            let (prev, join) = self.point_buffer.last_two_mut();

            compute_join_side_positions(prev, join, &next, self.options.miter_limit, SIDE_POSITIVE);
            compute_join_side_positions(prev, join, &next, self.options.miter_limit, SIDE_NEGATIVE);


            self.vertex.src = join.src;
            self.vertex.position_on_path = join.position;
            self.vertex.half_width = join.half_width;
            self.vertex.advancement = join.advancement;
            self.vertex.buffer_is_valid = false;
            add_join_base_vertices(join, &mut self.vertex, attributes, self.output, Side::Negative)?;
            add_join_base_vertices(join, &mut self.vertex, attributes, self.output, Side::Positive)?;

            if count > 2 {
                add_edge_triangles(prev, join, self.output);
            }

            tessellate_join(join, &self.options, &mut self.vertex, attributes, self.output)?;

            if count == 2 {
                self.firsts.push(*prev);
                self.firsts.push(*join);
            }
        }

        self.point_buffer.push(next);

        Ok(())
    }
}

fn compute_edge_attachment_positions(p0: &mut EndpointData, p1: &mut EndpointData) {
    let edge = p1.position - p0.position;
    let d = edge.length();
    let edge_angle = edge.angle_from_x_axis().radians;

    // Extra angle produced by the varying stroke width. 
    // sin(vwidth_angle) = (hw1 - hw0) / d
    let vwidth_angle = ((p1.half_width - p0.half_width) / d).asin();

    compute_side_attachment_positions(p0, p1, edge_angle, vwidth_angle, SIDE_POSITIVE);
    compute_side_attachment_positions(p0, p1, edge_angle, vwidth_angle, SIDE_NEGATIVE);

    p1.advancement = p0.advancement + d;
}

fn compute_side_attachment_positions(p0: &mut EndpointData, p1: &mut EndpointData, edge_angle: f32, vwidth_angle: f32, side: usize) {

    let nl = side_sign(side);

    let normal_angle = edge_angle + nl * (PI * 0.5 + vwidth_angle);
    let normal = vector(normal_angle.cos(), normal_angle.sin());

    p0.side_points[side].next = p0.position + normal * p0.half_width;
    p1.side_points[side].prev = p1.position + normal * p1.half_width;
}

fn add_edge_triangles(p0: &EndpointData, p1: &EndpointData, output: &mut dyn StrokeGeometryBuilder) {    
    let mut p0_neg = p0.side_points[SIDE_NEGATIVE].next_vertex;
    let mut p0_pos = p0.side_points[SIDE_POSITIVE].next_vertex;
    let mut p1_neg = p1.side_points[SIDE_NEGATIVE].prev_vertex;
    let mut p1_pos = p1.side_points[SIDE_POSITIVE].prev_vertex;

    if p0.fold[SIDE_POSITIVE] {
        p0_neg = p0.side_points[SIDE_NEGATIVE].prev_vertex;
    }
    if p0.fold[SIDE_NEGATIVE] {
        p0_pos = p0.side_points[SIDE_POSITIVE].prev_vertex;
    }
    if p1.fold[SIDE_POSITIVE] {
        p1_neg = p1.side_points[SIDE_NEGATIVE].next_vertex;
    }
    if p1.fold[SIDE_NEGATIVE] {
        p1_pos = p1.side_points[SIDE_POSITIVE].next_vertex;
    }

    output.add_triangle(p0_neg, p0_pos, p1_pos);

    output.add_triangle(p0_neg, p1_pos, p1_neg);
}

fn tessellate_join(
    join: &mut EndpointData,
    options: &StrokeOptions,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut dyn StrokeGeometryBuilder,
) -> Result<(), TessellationError> {
    let side_needs_join = [
        join.side_points[SIDE_POSITIVE].single_vertex.is_none(),
        join.side_points[SIDE_NEGATIVE].single_vertex.is_none(),
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

        if join.line_join == LineJoin::Round {
            tessellate_round_join(join, side, options, vertex, attributes, output)?;
        }
    }

    Ok(())
}

fn tessellate_round_join(
    join: &mut EndpointData,
    side: usize,
    options: &StrokeOptions,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut dyn StrokeGeometryBuilder,
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

    if side == SIDE_POSITIVE {
        // Flip to keep consistent winding order.
        std::mem::swap(&mut start_angle, &mut end_angle);
        std::mem::swap(&mut start_vertex, &mut end_vertex);
    }

    // Compute the required number of subdivisions,
    let arc_len = radius.abs() * diff.radians.abs();
    let step = circle_flattening_step(radius, options.tolerance);
    let num_segments = (arc_len / step).ceil();
    let num_subdivisions = num_segments.log2() as u32 * 2;

    vertex.side = if side == SIDE_POSITIVE { Side::Positive } else { Side::Negative };

    crate::stroke::tessellate_arc(
        (start_angle.radians, end_angle.radians),
        radius,
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
    output: &mut dyn StrokeGeometryBuilder,
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
    } else{
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
fn compute_join_side_positions(prev: &EndpointData, join: &mut EndpointData, next: &EndpointData, miter_limit: f32, side: usize) {
    let sign = side_sign(side);
    let v0 = (join.side_points[side].prev - prev.side_points[side].next).normalize();
    let v1 = (next.side_points[side].prev - join.side_points[side].next).normalize();
    let inward =  v0.cross(v1) * sign > 0.0;
    let forward = v0.dot(v1) > 0.0;

    let normal = compute_normal(v0, v1) * sign;
    let path_v0 = (join.position - prev.position).normalize();
    let path_v1 = (next.position - join.position).normalize();

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

        if d_next.min(d_prev) > 0.0 {
            // Case of an overlapping stroke. In order to prevent the back vertex to create a
            // spike outside of the stroke, we simply don't create it and we'll "fold" the join
            // instead.
            join.fold[side] = true;
        }
    }

    // For concave sides we'll simply connect at the intersection of the two side edges.
    let concave = inward && normal_same_side && !join.fold[side];

    if concave {
        let p = join.position + normal * join.half_width;
        join.side_points[side].single_vertex = Some(p);
    } else if (join.line_join == LineJoin::Miter || join.line_join == LineJoin::MiterClip)
        && !miter_limit_is_exceeded(normal, miter_limit) {

        let p = join.position + normal * join.half_width;
        join.side_points[side].single_vertex = Some(p);
    } else if join.line_join == LineJoin::MiterClip {
        // It is convenient to handle the miter-clip case here by simply moving
        // tow points on this side to the clip line.
        // This way the rest of the code doesn't differentiate between miter and miter-clip.
        let n0 = join.side_points[side].prev - join.position;
        let n1 = join.side_points[side].next - join.position;
        let (prev_normal, next_normal) = get_clip_intersections(n0, n1, normal, miter_limit * join.half_width);
        join.side_points[side].prev = join.position + prev_normal;
        join.side_points[side].next = join.position + next_normal;
    }
}

fn tessellate_last_edge(
    p0: &EndpointData,
    p1: &mut EndpointData,
    is_first_edge: bool,
    options: &StrokeOptions,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut dyn StrokeGeometryBuilder,
) -> Result<(), TessellationError> {
    // p0 and p1 are the last two points of the the sub-path.
    // We use a fake point p2 to generate the edge from p0 to p1.

    let v = p1.position - p0.position;
    let mut p2 = *p1;
    p2.position += v;
    p2.side_points[SIDE_POSITIVE].prev += v;
    p2.side_points[SIDE_NEGATIVE].prev += v;

    vertex.src = p1.src;
    vertex.position_on_path = p1.position;
    vertex.advancement = p1.advancement;
    vertex.half_width = p1.half_width;
    vertex.buffer_is_valid = false;

    let sides = [Side::Positive, Side::Negative];

    for side in 0..2 {
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
            p1.side_points[SIDE_NEGATIVE].prev - p1.position,
            p1.side_points[SIDE_NEGATIVE].prev_vertex,
            p1.side_points[SIDE_POSITIVE].prev_vertex,
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
    output: &mut dyn StrokeGeometryBuilder,
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
                vector: side_position - second.side_points[side].next,
            };

            let intersection = clip_line.intersection(&side_line).unwrap_or(first.side_points[side].next);
            side_position = intersection;
        }

        vertex.side = sides[side];
        vertex.normal = (side_position - first.position) / first.half_width;
        first.side_points[side].next_vertex = output.add_stroke_vertex(StrokeVertex(vertex, attributes))?;
    }

    // Tessellate the edge between prev and join.
    add_edge_triangles(first, second, output);

    match options.start_cap {
        LineCap::Round => crate::stroke::tessellate_round_cap(
            first.position,
            first.half_width,
            first.side_points[SIDE_POSITIVE].next - first.position,
            first.side_points[SIDE_POSITIVE].next_vertex,
            first.side_points[SIDE_NEGATIVE].next_vertex,
            first.position - second.position,
            options,
            true,
            vertex,
            attributes,
            output,
        ),
        _ => {
            Ok(())
        }
    }
}

fn get_clip_intersections(
    previous_normal: Vector,
    next_normal: Vector,
    normal: Vector,
    miter_limit: f32,
) -> (Vector, Vector) {
    let clip_line = Line {
        point: normal.normalize().to_point() * miter_limit,
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

fn miter_limit_is_exceeded(normal: Vector, miter_limit: f32) -> bool {
    normal.square_length() > miter_limit * miter_limit * 0.5
}

fn side_sign(side: usize) -> f32 {
    if side == SIDE_NEGATIVE { -1.0 } else { 1.0 }
}

fn circle_flattening_step(radius: f32, mut tolerance: f32) -> f32 {
    // Don't allow high tolerance values (compared to the radius) to avoid edge cases.
    tolerance = f32::min(tolerance, radius);
    2.0 * f32::sqrt(2.0 * tolerance * radius - tolerance * tolerance)
}

// A fall-back that avoids off artifacts with zero-area rectangles as
// well as overlapping triangles if the rectangle is much smaller than the
// line width in any dimension.
#[inline(never)]
fn approximate_thin_rectangle(builder: &mut VariableStrokeBuilder, rect: &Box2D, attributes: &[f32]) {
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

struct PointBuffer {
    points: [EndpointData; 3],
    start: usize,
    count: usize,
}

impl PointBuffer {
    fn new() -> Self {
        PointBuffer {
            points: [EndpointData::default(); 3],
            start: 0,
            count: 0,
        }
    }

    fn push(&mut self, point: EndpointData) {
        if self.count < 3 {
            self.points[self.count] = point;
            self.count += 1;
            return
        }

        self.points[self.start] = point;
        self.start += 1;
        if self.start == 3 {
            self.start = 0;
        }
    }

    fn clear(&mut self) {
        self.count = 0;
        self.start = 0;
    }

    fn count(&self) -> usize { self.count }

    fn get(&self, idx: usize) -> &EndpointData {
        assert!(idx < self.count);
        let idx = (idx + self.start) % 3;

        &self.points[idx]
    }

    fn get_reverse(&self, idx: usize) -> &EndpointData {
        assert!(idx < self.count);
        self.get(self.count - 1 - idx)
    }

    fn get_mut(&mut self, idx: usize) -> &mut EndpointData {
        assert!(idx < self.count);
        let idx = (idx + self.start) % 3;

        &mut self.points[idx]
    }

    fn last_mut(&mut self) -> &mut EndpointData {
        assert!(self.count > 0);
        self.get_mut(self.count - 1)
    }

    fn last_two_mut(&mut self) -> (&mut EndpointData, &mut EndpointData) {
        assert!(self.count >= 2);
        let i0 = (self.start + self.count - 1) % 3;
        let i1 = (self.start + self.count - 2) % 3;
        unsafe {(
            &mut *(self.points.get_unchecked_mut(i1) as *mut _),
            &mut *(self.points.get_unchecked_mut(i0) as *mut _),
        )}
    }
}

#[cfg(test)]
use crate::geometry_builder::*;
#[cfg(test)]
use crate::path::{Path, PathSlice};
#[cfg(test)]
use crate::StrokeTessellator;

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
            let threshold = 0.035; // Floating point errors :(
            let correct_winding = (pb - pa).cross(pc - pb) <= threshold;
            if !correct_winding {
                println!(" {:?} {:?} {:?}", pa, pb, pc);
            }
            assert!(correct_winding);
            self.builder.add_triangle(a, b, c);
        }
        fn abort_geometry(&mut self) {
            panic!();
        }
    }

    impl<'l> StrokeGeometryBuilder for TestBuilder<'l> {
        fn add_stroke_vertex(
            &mut self,
            vertex: StrokeVertex,
        ) -> Result<VertexId, GeometryBuilderError> {
            assert!(!vertex.position().x.is_nan());
            assert!(!vertex.position().y.is_nan());
            assert!(!vertex.normal().x.is_nan());
            assert!(!vertex.normal().y.is_nan());
            assert!(vertex.normal().square_length() != 0.0);
            assert!(!vertex.advancement().is_nan());
            self.builder.add_stroke_vertex(vertex)
        }
    }

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    let mut tess = StrokeTessellator::new();
    let count = tess
        .tessellate_path(
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
    let mut builder = Path::builder_with_attributes(1);

    builder.begin(point(-1.0, 1.0), &[0.3]);
    builder.line_to(point(1.0, 1.0), &[0.3]);
    builder.line_to(point(1.0, -1.0), &[0.3]);
    builder.line_to(point(-1.0, -1.0), &[0.3]);
    builder.end(true);

    let path1 = builder.build();

    let mut builder = Path::builder_with_attributes(1);

    builder.begin(point(-1.0, -1.0), &[0.3]);
    builder.line_to(point(1.0, -1.0), &[0.3]);
    builder.line_to(point(1.0, 1.0), &[0.3]);
    builder.line_to(point(-1.0, 1.0), &[0.3]);
    builder.end(true);

    let path2 = builder.build();

    test_path(
        path1.as_slice(),
        &StrokeOptions::default()
            .with_line_join(LineJoin::Miter)
            .with_variable_line_width(0),
        Some(8),
    );
    test_path(
        path2.as_slice(),
        &StrokeOptions::default()
            .with_line_join(LineJoin::Miter)
            .with_variable_line_width(0),
        Some(8),
    );

    test_path(
        path1.as_slice(),
        &StrokeOptions::default()
            .with_line_join(LineJoin::Bevel)
            .with_variable_line_width(0),
        Some(12),
    );
    test_path(
        path2.as_slice(),
        &StrokeOptions::default()
            .with_line_join(LineJoin::Bevel)
            .with_variable_line_width(0),
        Some(12),
    );

    test_path(
        path1.as_slice(),
        &StrokeOptions::default()
            .with_line_join(LineJoin::MiterClip)
            .with_miter_limit(1.0)
            .with_variable_line_width(0),
        Some(12),
    );
    test_path(
        path2.as_slice(),
        &StrokeOptions::default()
            .with_line_join(LineJoin::MiterClip)
            .with_miter_limit(1.0)
            .with_variable_line_width(0),
        Some(12),
    );

    test_path(
        path1.as_slice(),
        &StrokeOptions::tolerance(0.001)
            .with_line_join(LineJoin::Round)
            .with_variable_line_width(0),
        None,
    );
    test_path(
        path2.as_slice(),
        &StrokeOptions::tolerance(0.001)
            .with_line_join(LineJoin::Round)
            .with_variable_line_width(0),
        None,
    );
}

#[test]
fn test_empty_path() {
    let path = Path::builder_with_attributes(1).build();
    test_path(path.as_slice(), &StrokeOptions::default(), Some(0));
}

#[test]
fn test_empty_caps() {
    let mut builder = Path::builder_with_attributes(1);

    builder.begin(point(1.0, 0.0), &[1.0]);
    builder.end(false);
    builder.begin(point(2.0, 0.0), &[1.0]);
    builder.end(false);
    builder.begin(point(3.0, 0.0), &[1.0]);
    builder.end(false);

    let path = builder.build();

    test_path(
        path.as_slice(),
        &StrokeOptions::default()
            .with_line_cap(LineCap::Butt)
            .with_variable_line_width(0),
        Some(0),
    );
    test_path(
        path.as_slice(),
        &StrokeOptions::default()
            .with_line_cap(LineCap::Square)
            .with_variable_line_width(0),
        Some(6),
    );
    test_path(
        path.as_slice(),
        &StrokeOptions::default()
            .with_line_cap(LineCap::Round)
            .with_variable_line_width(0),
        None,
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
