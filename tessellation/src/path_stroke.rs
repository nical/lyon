use math::*;
use core::FlattenedEvent;
use bezier::utils::{normalized_tangent, directed_angle, fast_atan2};
use geometry_builder::{VertexId, GeometryBuilder, Count};
use basic_shapes::circle_flattening_step;
use path_builder::FlatPathBuilder;
use path_iterator::PathIterator;
use StrokeVertex as Vertex;
use {Side, LineCap, LineJoin, StrokeOptions};

use std::f32::consts::PI;

/// A Context object that can tessellate stroke operations for complex paths.
///
/// ## Overview
///
/// The stroke tessellation algorithm simply generates a strip of triangles along
/// the path. This method is fast and simple to implement, howerver it means that
/// if the path overlap with itself (for example in the case of a self-intersecting
/// path), some triangles will overlap in the interesecting region, which may not
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
/// a [`GeometryBuilder`](geometry_builder/trait.GeometryBuilder.html).
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
/// # extern crate lyon_tessellation;
/// # extern crate lyon_core;
/// # extern crate lyon_path;
/// # extern crate lyon_path_builder;
/// # extern crate lyon_path_iterator;
/// # use lyon_path::Path;
/// # use lyon_path_builder::*;
/// # use lyon_path_iterator::*;
/// # use lyon_core::math::{Point, point};
/// # use lyon_tessellation::geometry_builder::{VertexBuffers, simple_builder};
/// # use lyon_tessellation::*;
/// # use lyon_tessellation::StrokeVertex as Vertex;
/// # fn main() {
/// // Create a simple path.
/// let mut path_builder = Path::builder();
/// path_builder.move_to(point(0.0, 0.0));
/// path_builder.line_to(point(1.0, 2.0));
/// path_builder.line_to(point(2.0, 0.0));
/// path_builder.line_to(point(1.0, 1.0));
/// path_builder.close();
/// let path = path_builder.build();
///
/// // Create the destination vertex and index buffers.
/// let mut buffers: VertexBuffers<Vertex> = VertexBuffers::new();
///
/// {
///     // Create the destination vertex and index buffers.
///     let mut vertex_builder = simple_builder(&mut buffers);
///
///     // Create the tessellator.
///     let mut tessellator = StrokeTessellator::new();
///
///     // Compute the tessellation.
///     tessellator.tessellate_path(
///         path.path_iter(),
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
pub struct StrokeTessellator {}

impl StrokeTessellator {
    pub fn new() -> StrokeTessellator { StrokeTessellator {} }

    /// Compute the tessellation from a path iterator.
    pub fn tessellate_path<Input, Output>(
        &mut self,
        input: Input,
        options: &StrokeOptions,
        builder: &mut Output,
    ) -> Count
    where
        Input: PathIterator,
        Output: GeometryBuilder<Vertex>,
    {
        self.tessellate_flattened_path(
            input.flattened(options.tolerance),
            options,
            builder,
        )
    }

    /// Compute the tessellation from a flattened path iterator.
    pub fn tessellate_flattened_path<Input, Output>(
        &mut self,
        input: Input,
        options: &StrokeOptions,
        builder: &mut Output,
    ) -> Count
    where
        Input: Iterator<Item = FlattenedEvent>,
        Output: GeometryBuilder<Vertex>,
    {
        builder.begin_geometry();
        {
            let mut stroker = StrokeBuilder::new(options, builder);

            for evt in input {
                stroker.flat_event(evt);
            }

            stroker.build();
        }
        return builder.end_geometry();
    }
}

macro_rules! add_vertex {
    ($builder: expr, $vertex: expr) => {{
        let mut v = $vertex;

        if $builder.options.apply_line_width {
            v.position += v.normal * $builder.options.line_width / 2.0;
        }

        $builder.output.add_vertex(v)
    }}
}

/// A builder that tessellates a stroke directly without allocating any intermediate data structure.
pub struct StrokeBuilder<'l, Output: 'l> {
    first: Point,
    previous: Point,
    current: Point,
    second: Point,
    previous_left_id: VertexId,
    previous_right_id: VertexId,
    second_left_id: VertexId,
    second_right_id: VertexId,
    prev_normal: Vec2,
    nth: u32,
    sub_path_idx: u32,
    length: f32,
    sub_path_start_length: f32,
    options: StrokeOptions,
    output: &'l mut Output,
}

impl<'l, Output: 'l + GeometryBuilder<Vertex>> FlatPathBuilder for StrokeBuilder<'l, Output> {
    type PathType = ();

    fn move_to(&mut self, to: Point) {
        self.finish();

        self.first = to;
        self.current = to;
        self.nth = 0;
        self.sub_path_start_length = self.length;
    }

    fn line_to(&mut self, to: Point) { self.edge_to(to); }

    fn close(&mut self) {
        let first = self.first;
        self.edge_to(first);
        if self.nth > 1 {
            let second = self.second;
            self.edge_to(second);

            let first_left_id = add_vertex!(
                self,
                Vertex {
                    position: self.first,
                    normal: self.prev_normal,
                    advancement: self.sub_path_start_length,
                    side: Side::Left,
                }
            );
            let first_right_id = add_vertex!(
                self,
                Vertex {
                    position: self.first,
                    normal: -self.prev_normal,
                    advancement: self.sub_path_start_length,
                    side: Side::Right,
                }
            );

            self.output.add_triangle(first_right_id, first_left_id, self.second_right_id);
            self.output.add_triangle(first_left_id, self.second_left_id, self.second_right_id);
        }
        self.nth = 0;
        self.current = self.first;
        self.sub_path_start_length = self.length;
        self.sub_path_idx += 1;
    }

    fn current_position(&self) -> Point { self.current }

    fn build(mut self) {
        self.finish();
    }

    fn build_and_reset(&mut self) {
        self.first = Point::new(0.0, 0.0);
        self.previous = Point::new(0.0, 0.0);
        self.current = Point::new(0.0, 0.0);
        self.second = Point::new(0.0, 0.0);
        self.prev_normal = Vec2::new(0.0, 0.0);
        self.nth = 0;
        self.length = 0.0;
        self.sub_path_start_length = 0.0;
    }
}

impl<'l, Output: 'l + GeometryBuilder<Vertex>> StrokeBuilder<'l, Output> {
    pub fn new(options: &StrokeOptions, builder: &'l mut Output) -> Self {
        let zero = Point::new(0.0, 0.0);
        return StrokeBuilder {
            first: zero,
            second: zero,
            previous: zero,
            current: zero,
            prev_normal: Vec2::new(0.0, 0.0),
            previous_left_id: VertexId(0),
            previous_right_id: VertexId(0),
            second_left_id: VertexId(0),
            second_right_id: VertexId(0),
            nth: 0,
            sub_path_idx: 0,
            length: 0.0,
            sub_path_start_length: 0.0,
            options: *options,
            output: builder,
        };
    }

    pub fn set_options(&mut self, options: &StrokeOptions) { self.options = *options; }

    fn tessellate_empty_square_cap(&mut self) {
        let a = add_vertex!(
            self,
            Vertex {
                position: self.current,
                normal: vec2(-1.0, -1.0),
                advancement: 0.0,
                side: Side::Left,
            }
        );
        let b = add_vertex!(
            self,
            Vertex {
                position: self.current,
                normal: vec2(1.0, -1.0),
                advancement: 0.0,
                side: Side::Left,
            }
        );
        let c = add_vertex!(
            self,
            Vertex {
                position: self.current,
                normal: vec2(1.0, 1.0),
                advancement: 0.0,
                side: Side::Right,
            }
        );
        let d = add_vertex!(
            self,
            Vertex {
                position: self.current,
                normal: vec2(-1.0, 1.0),
                advancement: 0.0,
                side: Side::Right,
            }
        );
        self.output.add_triangle(a, b, c);
        self.output.add_triangle(a, c, d);
    }

    fn tessellate_empty_round_cap(&mut self) {
        let center = self.current;
        let left_id = add_vertex!(
            self,
            Vertex {
                position: center,
                normal: vec2(-1.0, 0.0),
                advancement: 0.0,
                side: Side::Left,
            }
        );
        let right_id = add_vertex!(
            self,
            Vertex {
                position: center,
                normal: vec2(1.0, 0.0),
                advancement: 0.0,
                side: Side::Right,
            }
        );
        self.tessellate_round_cap(center, vec2(0.0, -1.0), left_id, right_id, true);
        self.tessellate_round_cap(center, vec2(0.0, 1.0), left_id, right_id, false);
    }

    fn finish(&mut self) {
        if self.nth == 0 && self.sub_path_idx > 0 {
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
            self.edge_to(p);
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

            let first_left_id = add_vertex!(
                self,
                Vertex {
                    position: first,
                    normal: n1,
                    advancement: self.sub_path_start_length,
                    side: Side::Left,
                }
            );
            let first_right_id = add_vertex!(
                self,
                Vertex {
                    position: first,
                    normal: n2,
                    advancement: self.sub_path_start_length,
                    side: Side::Right,
                }
            );


            if self.options.start_cap == LineCap::Round {
                self.tessellate_round_cap(first, d, first_left_id, first_right_id, true);
            }

            self.output.add_triangle(first_right_id, first_left_id, self.second_right_id);
            self.output.add_triangle(first_left_id, self.second_left_id, self.second_right_id);

        }
        self.sub_path_idx += 1;
    }

    fn edge_to(&mut self, to: Point) {
        if (self.current - to).square_length() < 1e-5 {
            return;
        }

        if self.nth == 0 {
            // We don't have enough information to compute the previous
            // vertices (and thus the current join) yet.
            self.previous = self.first;
            self.current = to;
            self.nth += 1;
            return;
        }

        let previous_edge = self.current - self.previous;
        let previous_edge_length = previous_edge.length();
        let next_tangent = (to - self.current).normalize();
        self.length += previous_edge_length;

        let (
            start_left_id,
            start_right_id,
            end_left_id,
            end_right_id,
        ) = self.tessellate_join(
            previous_edge / previous_edge_length,
            next_tangent
        );

        // Tessellate the edge
        if self.nth > 1 {
            self.output.add_triangle(self.previous_left_id, self.previous_right_id, start_left_id);
            self.output.add_triangle(self.previous_right_id, start_left_id, start_right_id);
        }

        self.previous = self.current;
        self.previous_left_id = end_left_id;
        self.previous_right_id = end_right_id;
        self.current = to;

        if self.nth == 1 {
            self.second = self.previous;
            self.second_left_id = start_left_id;
            self.second_right_id = start_right_id;
        }

        self.nth += 1;
    }

    fn tessellate_round_cap(
        &mut self,
        center: Point,
        dir: Vec2,
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
        let advancement = self.length;

        let quarter_angle = if is_start { -PI * 0.5 } else { PI * 0.5 };
        let mid_angle = directed_angle(vec2(1.0, 0.0), dir);
        let left_angle = mid_angle + quarter_angle;
        let right_angle = mid_angle - quarter_angle;

        let mid_vertex = add_vertex!(
            self,
            Vertex {
                position: center,
                normal: dir,
                advancement: advancement,
                side: Side::Left,
            }
        );

        self.output.add_triangle(left, mid_vertex, right);

        let apply_width = if self.options.apply_line_width {
            self.options.line_width * 0.5
        } else {
            0.0
        };

        tess_round_cap(
            center,
            (left_angle, mid_angle),
            radius,
            left, mid_vertex,
            num_recursions,
            advancement,
            Side::Left,
            apply_width,
            self.output
        );
        tess_round_cap(
            center,
            (mid_angle, right_angle),
            radius,
            mid_vertex, right,
            num_recursions,
            advancement,
            Side::Right,
            apply_width,
            self.output
        );
    }

    fn tessellate_join(&mut self,
        prev_tangent: Vec2,
        next_tangent: Vec2,
    ) -> (VertexId, VertexId, VertexId, VertexId) {
        // This function needs to differentiate the "front" of the join (aka. the pointy side)
        // from the back. The front is where subdivision or adjustments may be needed.

        let normal = get_angle_normal(prev_tangent, next_tangent);

        let (front_side, front_normal) = if next_tangent.cross(prev_tangent) >= 0.0 {
            (Side::Left, normal)
        } else {
            (Side::Right, -normal)
        };

        // Add a vertex at the back of the join
        let back_vertex = add_vertex!(
            self,
            Vertex {
                position: self.current,
                normal: -front_normal,
                advancement: self.length,
                side: front_side.opposite(),
            }
        );

        let limit = self.options.miter_limit;
        let join_type = if prev_tangent.dot(next_tangent) >= 0.95 {
            // The two edges are almost aligned, just use a simple miter join.
            // TODO: the 0.95 threshold above is completely arbitrary and needs
            // adjustments.
            LineJoin::Miter
        } else if self.options.line_join == LineJoin::Miter && normal.square_length() > limit * limit {
            // Per SVG spec: If the stroke-miterlimit is exceeded, the line join
            // falls back to bevel.
            LineJoin::Bevel
        } else {
            self.options.line_join
        };

        let (start_vertex, end_vertex) = match join_type {
            LineJoin::Round => {
                self.tessellate_round_join(
                    prev_tangent,
                    next_tangent,
                    front_side,
                    back_vertex
                )
            }
            LineJoin::Bevel => {
                self.tessellate_bevel_join(
                    prev_tangent,
                    next_tangent,
                    front_side,
                    back_vertex
                )
            }
            // Fallback to Miter for unimplemented line joins
            _ => {
                let v = add_vertex!(
                    self,
                    Vertex {
                        position: self.current,
                        normal: front_normal,
                        advancement: self.length,
                        side: front_side,
                    }
                );
                self.prev_normal = normal;

                (v, v)
            }
        };

        match front_side {
            Side::Left => (start_vertex, back_vertex, end_vertex, back_vertex),
            Side::Right => (back_vertex, start_vertex, back_vertex, end_vertex),
        }
    }

    fn tessellate_bevel_join(
        &mut self,
        prev_tangent: Vec2,
        next_tangent: Vec2,
        front_side: Side,
        back_vertex: VertexId,
    ) -> (VertexId, VertexId) {
        let neg_if_right = if front_side.is_left() { 1.0 } else { -1.0 };
        let prev_normal = vec2(-prev_tangent.y, prev_tangent.x);
        let next_normal = vec2(-next_tangent.y, next_tangent.x);

        let start_vertex = add_vertex!(
            self,
            Vertex {
                position: self.current,
                normal: prev_normal * neg_if_right,
                advancement: self.length,
                side: front_side,
            }
        );
        let last_vertex = add_vertex!(
            self,
            Vertex {
                position: self.current,
                normal: next_normal * neg_if_right,
                advancement: self.length,
                side: front_side,
            }
        );
        self.prev_normal = next_normal;
        self.output.add_triangle(start_vertex, last_vertex, back_vertex);

        (start_vertex, last_vertex)
    }
    fn tessellate_round_join(
        &mut self,
        prev_tangent: Vec2,
        next_tangent: Vec2,
        front_side: Side,
        back_vertex: VertexId,
    ) -> (VertexId, VertexId) {
        let mut join_angle = fast_atan2(prev_tangent.y, prev_tangent.x) - fast_atan2(next_tangent.y, next_tangent.x);

        // Make sure to stay within the [-Pi, Pi] range.
        if join_angle > PI {
            join_angle -= 2.0 * PI;
        } else if join_angle < -PI {
            join_angle += 2.0 * PI;
        }

        let max_radius_segment_angle = compute_max_radius_segment_angle(self.options.line_width / 2.0, self.options.tolerance);
        let num_segments = (join_angle.abs() as f32 / max_radius_segment_angle).ceil() as u32;
        assert!(num_segments > 0);
        // Calculate angle of each step
        let segment_angle = join_angle as f32 / num_segments as f32;

        let neg_if_right = if front_side.is_left() { 1.0 } else { -1.0 };

        // Calculate the initial front normal
        let initial_normal = vec2(-prev_tangent.y, prev_tangent.x) * neg_if_right;

        let mut last_vertex = add_vertex!(
            self,
            Vertex {
                position: self.current,
                normal: initial_normal,
                advancement: self.length,
                side: front_side,
            }
        );
        let start_vertex = last_vertex;

        // Plot each point along the radius by using a matrix to
        // rotate the normal at each step
        let (sin, cos) = segment_angle.sin_cos();
        let rotation_matrix = [
            [cos, sin],
            [-sin, cos],
        ];

        let mut n = initial_normal;
        for _ in 0..num_segments {
            // incrementally rotate the normal
            n = vec2(
                n.x * rotation_matrix[0][0] + n.y * rotation_matrix[0][1],
                n.x * rotation_matrix[1][0] + n.y * rotation_matrix[1][1]
            );

            let current_vertex = add_vertex!(
                self,
                Vertex {
                    position: self.current,
                    normal: n,
                    advancement: self.length,
                    side: front_side,
                }
            );

            self.output.add_triangle(back_vertex, last_vertex, current_vertex);
            last_vertex = current_vertex;
        }

        self.prev_normal = n * neg_if_right;

        (start_vertex, last_vertex)
    }
}

fn get_angle_normal(v1: Vec2, v2: Vec2) -> Vec2 {
    let epsilon = 1e-4;

    let n1 = vec2(-v1.y, v1.x);

    let v12 = v1 + v2;

    if v12.square_length() < epsilon {
        return n1;
    }

    let tangent = v12.normalize();
    let n = vec2(-tangent.y, tangent.x);

    let inv_len = n.dot(n1);

    if inv_len.abs() < epsilon {
        return n1;
    }

    return n / inv_len;
}

/// Computes the max angle of a radius segment for a given tolerance
pub fn compute_max_radius_segment_angle(radius: f32, tolerance: f32) -> f32 {
    let t = radius - tolerance;
    ((radius * radius - t * t) * 4.0).sqrt() / radius
}

fn tess_round_cap<Output: GeometryBuilder<Vertex>>(
    center: Point,
    angle: (f32, f32),
    radius: f32,
    va: VertexId,
    vb: VertexId,
    num_recursions: u32,
    advancement: f32,
    side: Side,
    line_width: f32,
    output: &mut Output
) {
    if num_recursions == 0 {
        return;
    }

    let mid_angle = (angle.0 + angle.1) * 0.5;

    let normal = vec2(mid_angle.cos(), mid_angle.sin());

    let vertex = output.add_vertex(Vertex {
        position: center + normal * line_width,
        normal: normal,
        advancement,
        side,
    });

    output.add_triangle(va, vertex, vb);

    tess_round_cap(
        center,
        (angle.0, mid_angle),
        radius,
        va,
        vertex,
        num_recursions - 1,
        advancement,
        side,
        line_width,
        output
    );
    tess_round_cap(
        center,
        (mid_angle, angle.1),
        radius,
        vertex,
        vb,
        num_recursions - 1,
        advancement,
        side,
        line_width,
        output
    );
}
