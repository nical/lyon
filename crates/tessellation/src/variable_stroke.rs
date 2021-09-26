use crate::geom::utils::tangent;
use crate::geom::{Line, QuadraticBezierSegment, CubicBezierSegment, arrayvec::ArrayVec};
use crate::math::*;
use crate::math_utils::compute_normal;
use crate::path::private::DebugValidator;
use crate::path::{
    AttributeStore, EndpointId, IdEvent, PositionStore,
};
use crate::{
    LineCap, LineJoin, Side, StrokeOptions, TessellationError, VertexSource,
};
use crate::{StrokeGeometryBuilder, VertexId, TessellationResult};

use std::f32::consts::PI;
const EPSILON: f32 = 1e-4;

use crate::stroke::{StrokeVertex, StrokeVertexData};

const SIDE_LEFT: usize = 0;
const SIDE_RIGHT: usize = 1;

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

/// A builder that tessellates a stroke directly without allocating any intermediate data structure.
pub(crate) struct VariableStrokeBuilder<'l> {
    options: StrokeOptions,
    error: Option<TessellationError>,
    output: &'l mut dyn StrokeGeometryBuilder,
    attributes: StrokeVertexData<'l>,
    point_buffer: PointBuffer,
    firsts: ArrayVec<EndpointData, 2>,
    previous: Option<EndpointData>,
    sub_path_start_advancement: f32,
}

impl<'l> VariableStrokeBuilder<'l> {
    pub(crate) fn new(
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
        VariableStrokeBuilder {
            options: *options,
            error: None,
            output,
            attributes: StrokeVertexData {
                position_on_path: zero,
                normal: vector(0.0, 0.0),
                half_width: options.line_width * 0.5,
                advancement: 0.0,
                buffer: attrib_buffer,
                store: attrib_store,
                side: Side::Left,
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
                    let width = base_width * self.attributes.store.get(at)[attrib_index];
                    current_endpoint = at;
                    current_position = positions.get_endpoint(at);
                    self.begin(current_position, at, width);
                }
                IdEvent::Line { to, .. } => {
                    validator.edge();
                    let width = base_width * self.attributes.store.get(to)[attrib_index];
                    current_endpoint = to;
                    current_position = positions.get_endpoint(to);
                    self.line_to(current_position, to, width);
                }
                IdEvent::Quadratic { ctrl, to, .. } => {
                    validator.edge();
                    let start_width = base_width * self.attributes.store.get(current_endpoint)[attrib_index];
                    let end_width = base_width * self.attributes.store.get(to)[attrib_index];

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
                            let line_join = if t >= 1.0 {
                                self.options.line_join
                            } else {
                                LineJoin::Miter
                            };

                            let r = self.step(EndpointData {
                                position,
                                half_width: width * 0.5,
                                line_join,
                                src: VertexSource::Edge { from, to, t },
                                ..Default::default()
                            });

                            if let Err(e) = r {
                                self.error(e);
                            }
                        },
                    );
                }
                IdEvent::Cubic { ctrl1, ctrl2, to, .. } => {
                    validator.edge();

                    let start_width = base_width * self.attributes.store.get(current_endpoint)[attrib_index];
                    let end_width = base_width * self.attributes.store.get(to)[attrib_index];

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
                            let line_join = if t >= 1.0 {
                                self.options.line_join
                            } else {
                                LineJoin::Miter
                            };

                            let r = self.step(EndpointData {
                                position: point,
                                half_width: width * 0.5,
                                line_join,
                                src: VertexSource::Edge { from, to, t },
                                ..Default::default()
                            });

                            if let Err(e) = r {
                                self.error(e);
                            }
                        },
                    );
                }
                IdEvent::End { close, .. } => {
                    validator.end();
                    self.end(close);
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

    fn begin(&mut self, position: Point, endpoint: EndpointId, width: f32) {
        let half_width = width * 0.5;
        let r = self.step(EndpointData {
            position,
            half_width,
            advancement: self.sub_path_start_advancement,
            line_join: self.options.line_join,
            src: VertexSource::Endpoint { id: endpoint },
            ..Default::default()
        });

        if let Err(e) = r {
            self.error(e);            
        }
    }

    fn line_to(&mut self, to: Point, endpoint: EndpointId, width: f32) {
        let half_width = width * 0.5;
        let r = self.step(EndpointData {
            position: to,
            half_width,
            line_join: self.options.line_join,
            src: VertexSource::Endpoint { id: endpoint },
            ..Default::default()
        });

        if let Err(e) = r {
            self.error(e);
        }
    }

    fn end(&mut self, close: bool) {
        let res = if close && self.point_buffer.count() > 2{
            self.close()
        } else {
            self.end_with_caps()
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

    fn close(&mut self) -> Result<(), TessellationError> {
        let p = self.firsts[0];
        self.step(p)?;

        if self.firsts.len() >= 2 {
            let p2 = self.firsts[1];
            self.step(p2)?;

            let (p0, p1) = self.point_buffer.last_two_mut();

            add_edge_triangles(p0, p1, self.output);
        }

        Ok(())
    }

    fn end_with_caps(&mut self) -> Result<(), TessellationError> {
        if self.point_buffer.count() > 2 {
            // Last edge.

            // Add a fake fake point p2 aligned with p0 and p1 so that we can tessellate
            // the join for p1. 
            let (p0, p1) = self.point_buffer.last_two_mut();
            tessellate_last_edge(p0, p1, false,  &self.options, &mut self.attributes, self.output)?;
            self.sub_path_start_advancement = p1.advancement;

            // First edge.
            let mut p0 = self.firsts[0];
            let p1 = &self.firsts[1];
            tessellate_first_edge(&mut p0, p1, &self.options, &mut self.attributes, self.output)?;
        }

        if self.point_buffer.count() == 2 {
            let (p0, p1) = self.point_buffer.last_two_mut();
            tessellate_last_edge(p0, p1, true, &self.options, &mut self.attributes, self.output)?;
            self.sub_path_start_advancement = p1.advancement;
            tessellate_first_edge(p0, p1, &self.options, &mut self.attributes, self.output)?;
            add_edge_triangles(p0, p1, self.output);
        }

        if self.point_buffer.count() == 1 {
            let point = self.point_buffer.get(0);

            self.attributes.advancement = point.advancement;
            self.attributes.src = point.src;
            self.attributes.half_width = point.half_width;

            match self.options.start_cap {
                LineCap::Square => {
                    // Even if there is no edge, if we are using square caps we have to place a square
                    // at the current position.
                    crate::stroke::tessellate_empty_square_cap(point.position, &mut self.attributes, self.output)?;
                }
                LineCap::Round => {
                    // Same thing for round caps.
                    crate::stroke::tessellate_empty_round_cap(point.position, &self.options, &mut self.attributes, self.output)?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn step(&mut self, mut next: EndpointData) -> Result<(), TessellationError> {
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

            compute_join_side_positions(prev, join, &next, self.options.miter_limit, SIDE_LEFT);
            compute_join_side_positions(prev, join, &next, self.options.miter_limit, SIDE_RIGHT);


            self.attributes.src = join.src;
            self.attributes.position_on_path = join.position;
            self.attributes.half_width = join.half_width;
            self.attributes.advancement = join.advancement;
            add_join_base_vertices(join, &mut self.attributes, self.output, Side::Left)?;
            add_join_base_vertices(join, &mut self.attributes, self.output, Side::Right)?;

            if count > 2 {
                add_edge_triangles(prev, join, self.output);
            }

            tessellate_join(join, &self.options, &mut self.attributes, self.output)?;

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

    compute_side_attachment_positions(p0, p1, edge_angle, vwidth_angle, SIDE_LEFT);
    compute_side_attachment_positions(p0, p1, edge_angle, vwidth_angle, SIDE_RIGHT);

    p1.advancement = p0.advancement + d;
}

fn compute_side_attachment_positions(p0: &mut EndpointData, p1: &mut EndpointData, edge_angle: f32, vwidth_angle: f32, side: usize) {

    let nl = negative_if_left(side);

    let normal_angle = edge_angle + nl * (PI * 0.5 + vwidth_angle);
    let normal = vector(normal_angle.cos(), normal_angle.sin());

    p0.side_points[side].next = p0.position + normal * p0.half_width;
    p1.side_points[side].prev = p1.position + normal * p1.half_width;
}

fn add_edge_triangles(p0: &EndpointData, p1: &EndpointData, output: &mut dyn StrokeGeometryBuilder) {    
    let mut p0_left = p0.side_points[SIDE_LEFT].next_vertex;
    let mut p0_right = p0.side_points[SIDE_RIGHT].next_vertex;
    let mut p1_left = p1.side_points[SIDE_LEFT].prev_vertex;
    let mut p1_right = p1.side_points[SIDE_RIGHT].prev_vertex;

    let mut _p0_left = p0.side_points[SIDE_LEFT].next;
    let mut _p0_right = p0.side_points[SIDE_RIGHT].next;
    let mut _p1_left = p1.side_points[SIDE_LEFT].prev;
    let mut _p1_right = p1.side_points[SIDE_RIGHT].prev;

    if p0.fold[SIDE_LEFT] {
        p0_left = p0.side_points[SIDE_RIGHT].prev_vertex;
    }
    if p0.fold[SIDE_RIGHT] {
        p0_right = p0.side_points[SIDE_LEFT].prev_vertex;
    }
    if p1.fold[SIDE_LEFT] {
        p1_left = p1.side_points[SIDE_RIGHT].next_vertex;
    }
    if p1.fold[SIDE_RIGHT] {
        p1_right = p1.side_points[SIDE_LEFT].next_vertex;
    }

    output.add_triangle(p0_left, p0_right, p1_right);

    output.add_triangle(p0_left, p1_right, p1_left);
}

fn tessellate_join(
    join: &mut EndpointData,
    options: &StrokeOptions,
    attributes: &mut StrokeVertexData,
    output: &mut dyn StrokeGeometryBuilder,
) -> Result<(), TessellationError> {
    let side_needs_join = [
        join.side_points[SIDE_LEFT].single_vertex.is_none(),
        join.side_points[SIDE_RIGHT].single_vertex.is_none(),
    ];

    if !join.fold[SIDE_LEFT] && !join.fold[SIDE_RIGHT] {
        // Tessellate the interior of the join.
        match side_needs_join {
            [true, true] => {
                let a = join.side_points[SIDE_LEFT].prev;
                let b = join.side_points[SIDE_LEFT].next;
                let c = join.side_points[SIDE_RIGHT].next;
                assert!((a - b).cross(c - b) > 0.0);

                output.add_triangle(
                    join.side_points[SIDE_LEFT].prev_vertex,
                    join.side_points[SIDE_LEFT].next_vertex,
                    join.side_points[SIDE_RIGHT].next_vertex,
                );

                let a = join.side_points[SIDE_LEFT].prev;
                let b = join.side_points[SIDE_RIGHT].next;
                let c = join.side_points[SIDE_RIGHT].prev;
                assert!((a - b).cross(c - b) > 0.0);
                output.add_triangle(
                    join.side_points[SIDE_LEFT].prev_vertex,
                    join.side_points[SIDE_RIGHT].next_vertex,
                    join.side_points[SIDE_RIGHT].prev_vertex,
                );
            }
            [false, true] => {
                let a = join.side_points[SIDE_LEFT].prev;
                let b = join.side_points[SIDE_RIGHT].prev;
                let c = join.side_points[SIDE_RIGHT].next;
                assert!((a - b).cross(c - b) > 0.0);
                output.add_triangle(
                    join.side_points[SIDE_LEFT].prev_vertex,
                    join.side_points[SIDE_RIGHT].prev_vertex,
                    join.side_points[SIDE_RIGHT].next_vertex,
                );
            }
            [true, false] => {
                let a = join.side_points[SIDE_LEFT].prev;
                let b = join.side_points[SIDE_RIGHT].prev;
                let c = join.side_points[SIDE_LEFT].next;
                assert!((a - b).cross(c - b) > 0.0);
                output.add_triangle(
                    join.side_points[SIDE_LEFT].prev_vertex,
                    join.side_points[SIDE_RIGHT].prev_vertex,
                    join.side_points[SIDE_LEFT].next_vertex,
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
            tessellate_round_join(join, side, options, attributes, output)?;
        }
    }

    Ok(())
}

fn tessellate_round_join(
    join: &mut EndpointData,
    side: usize,
    options: &StrokeOptions,
    attributes: &mut StrokeVertexData,
    output: &mut dyn StrokeGeometryBuilder,
) -> Result<(), TessellationError> {
    let center = join.position;
    let radius = join.half_width;

    let start_normal = join.side_points[side].prev - center;
    let end_normal = join.side_points[side].next - center;

    let mut start_vertex = join.side_points[side].prev_vertex;
    let mut end_vertex = join.side_points[side].next_vertex;

    let angle_sign = if side == SIDE_LEFT { 1.0 } else { -1.0 };

    let mut start_angle = start_normal.angle_from_x_axis();
    let mut diff = start_angle.angle_to(end_normal.angle_from_x_axis());
    if diff.radians * angle_sign < 0.0 {
        diff.radians -= 2.0 * PI;
    }
    let mut end_angle = start_angle + diff;

    if side == SIDE_LEFT {
        // Flip to keep consistent winding order.
        std::mem::swap(&mut start_angle, &mut end_angle);
        std::mem::swap(&mut start_vertex, &mut end_vertex);
    }

    // Compute the required number of subdivisions,
    let arc_len = radius.abs() * diff.radians.abs();
    let step = circle_flattening_step(radius, options.tolerance);
    let num_segments = (arc_len / step).ceil();
    let num_subdivisions = num_segments.log2() as u32 * 2;

    attributes.position_on_path = center;
    attributes.half_width = radius;
    attributes.advancement = join.advancement;
    attributes.side = if side == SIDE_LEFT { Side::Left } else { Side::Right };
    attributes.src = join.src;

    crate::stroke::tessellate_arc(
        (start_angle.radians, end_angle.radians),
        radius,
        start_vertex,
        end_vertex,
        num_subdivisions,
        attributes.side,
        attributes,
        output,
    )
}

fn add_join_base_vertices(
    join: &mut EndpointData,
    attributes: &mut StrokeVertexData,
    output: &mut dyn StrokeGeometryBuilder,
    side: Side,
) -> Result<(), TessellationError> {
    attributes.side = side;

    let side = match side {
        Side::Left => SIDE_LEFT,
        Side::Right => SIDE_RIGHT,
    };

    if let Some(pos) = join.side_points[side].single_vertex {
        attributes.normal = (pos - join.position) / join.half_width;
        let vertex = output.add_stroke_vertex(StrokeVertex(attributes))?;
        join.side_points[side].prev_vertex = vertex;
        join.side_points[side].next_vertex = vertex;
    } else{
        attributes.normal = (join.side_points[side].prev - join.position) / join.half_width;
        let prev_vertex = output.add_stroke_vertex(StrokeVertex(attributes))?;

        attributes.normal = (join.side_points[side].next - join.position) / join.half_width;
        let next_vertex = output.add_stroke_vertex(StrokeVertex(attributes))?;

        join.side_points[side].prev_vertex = prev_vertex;
        join.side_points[side].next_vertex = next_vertex;
    }

    Ok(())
}

// TODO: the naming is a bit confusing. We do half of the work to compute the join's side positions
// in compute_side_attachment_positions.
fn compute_join_side_positions(prev: &EndpointData, join: &mut EndpointData, next: &EndpointData, miter_limit: f32, side: usize) {
    let nl = negative_if_left(side);
    let v0 = (join.side_points[side].prev - prev.side_points[side].next).normalize();
    let v1 = (next.side_points[side].prev - join.side_points[side].next).normalize();
    let inward =  v0.cross(v1) * nl > 0.0;
    let forward = v0.dot(v1) > 0.0;

    let normal = compute_normal(v0, v1) * nl;
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
        let next_length = next.advancement - next.advancement;
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
        let (prev_normal, next_normal) = get_clip_intersections(n0, n1, normal, miter_limit);
        join.side_points[side].prev = join.position + prev_normal;
        join.side_points[side].next = join.position + next_normal;
    }
}

fn tessellate_last_edge(
    p0: &EndpointData,
    p1: &mut EndpointData,
    is_first_edge: bool,
    options: &StrokeOptions,
    attributes: &mut StrokeVertexData,
    output: &mut dyn StrokeGeometryBuilder,
) -> Result<(), TessellationError> {
    // p0 and p1 are the last two points of the the sub-path.
    // We use a fake point p2 to generate the edge from p0 to p1.

    let v = p1.position - p0.position;
    let mut p2 = *p1;
    p2.position += v;
    p2.side_points[SIDE_LEFT].prev += v;
    p2.side_points[SIDE_RIGHT].prev += v;

    attributes.src = p1.src;
    attributes.position_on_path = p1.position;
    attributes.advancement = p1.advancement;
    attributes.half_width = p1.half_width;

    for side in 0..2 {
        attributes.normal = (p1.side_points[side].prev - p1.position) / p1.half_width;
        let prev_vertex = output.add_stroke_vertex(StrokeVertex(attributes))?;
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
            p1.side_points[SIDE_RIGHT].prev - p1.position,
            p1.side_points[SIDE_RIGHT].prev_vertex,
            p1.side_points[SIDE_LEFT].prev_vertex,
            v,
            options,
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
    attributes: &mut StrokeVertexData,
    output: &mut dyn StrokeGeometryBuilder,
) -> Result<(), TessellationError> {

    attributes.src = first.src;
    attributes.position_on_path = first.position;
    attributes.advancement = first.advancement;
    attributes.half_width = first.half_width;

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

        attributes.normal = (side_position - first.position) / first.half_width;
        first.side_points[side].next_vertex = output.add_stroke_vertex(StrokeVertex(attributes))?;
    }

    // Tessellate the edge between prev and join.
    add_edge_triangles(first, second, output);

    match options.start_cap {
        LineCap::Round => crate::stroke::tessellate_round_cap(
            first.position,
            first.half_width,
            first.side_points[SIDE_LEFT].next - first.position,
            first.side_points[SIDE_LEFT].next_vertex,
            first.side_points[SIDE_RIGHT].next_vertex,
            first.position - second.position,
            options,
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
        point: normal.normalize().to_point() * miter_limit * 0.5,
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

fn negative_if_left(side: usize) -> f32 {
    if side == SIDE_LEFT { -1.0 } else { 1.0 }
}

fn circle_flattening_step(radius: f32, mut tolerance: f32) -> f32 {
    // Don't allow high tolerance values (compared to the radius) to avoid edge cases.
    tolerance = f32::min(tolerance, radius);
    2.0 * f32::sqrt(2.0 * tolerance * radius - tolerance * tolerance)
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

