use math_utils::compute_normal;
use geom::math::*;
use geom::{QuadraticBezierSegment, CubicBezierSegment, LineSegment, Arc};
use geom::utils::{normalized_tangent, directed_angle};
use geom::euclid::Trig;
use geometry_builder::{VertexId, GeometryBuilder, Count};
use basic_shapes::circle_flattening_step;
use path::builder::{FlatPathBuilder, PathBuilder};
use path::iterator::PathIterator;
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
/// # extern crate lyon_tessellation as tess;
/// # use tess::path::default::Path;
/// # use tess::path::builder::*;
/// # use tess::path::iterator::*;
/// # use tess::geom::math::*;
/// # use tess::geometry_builder::{VertexBuffers, simple_builder};
/// # use tess::*;
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
/// let mut buffers: VertexBuffers<StrokeVertex, u16> = VertexBuffers::new();
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
#[derive(Default)]
pub struct StrokeTessellator {}

impl StrokeTessellator {
    pub fn new() -> Self { StrokeTessellator {} }

    /// Compute the tessellation from a path iterator.
    pub fn tessellate_path<Input>(
        &mut self,
        input: Input,
        options: &StrokeOptions,
        builder: &mut GeometryBuilder<Vertex>,
    ) -> Count
    where
        Input: PathIterator,
    {
        builder.begin_geometry();
        {
            let mut stroker = StrokeBuilder::new(options, builder);

            for evt in input {
                stroker.path_event(evt);
            }

            stroker.build();
        }
        builder.end_geometry()
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
pub struct StrokeBuilder<'l> {
    first: Point,
    previous: Point,
    current: Point,
    second: Point,
    previous_left_id: VertexId,
    previous_right_id: VertexId,
    second_left_id: VertexId,
    second_right_id: VertexId,
    prev_normal: Vector,
    nth: u32,
    length: f32,
    sub_path_start_length: f32,
    options: StrokeOptions,
    previous_command_was_move: bool,
    output: &'l mut GeometryBuilder<Vertex>,
}

impl<'l> FlatPathBuilder for StrokeBuilder<'l> {
    type PathType = ();

    fn move_to(&mut self, to: Point) {
        self.finish();

        self.first = to;
        self.current = to;
        self.nth = 0;
        self.sub_path_start_length = self.length;
        self.previous_command_was_move = true;
    }

    fn line_to(&mut self, to: Point) {
        self.previous_command_was_move = false;
        self.edge_to(to, true);
    }

    fn close(&mut self) {
        // If we close almost at the first edge, then we have to
        // skip connecting the last and first edges otherwise the
        // normal will be plagued with floating point precision
        // issues.
        let threshold = 0.001;
        if (self.first - self.current).square_length() > threshold {
            let first = self.first;
            self.edge_to(first, true);
        }

        if self.nth > 1 {
            let second = self.second;
            self.edge_to(second, true);

            let first_left_id = add_vertex!(
                self,
                Vertex {
                    position: self.previous,
                    normal: self.prev_normal,
                    advancement: self.sub_path_start_length,
                    side: Side::Left,
                }
            );
            let first_right_id = add_vertex!(
                self,
                Vertex {
                    position: self.previous,
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
        self.previous_command_was_move = false;
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
        self.prev_normal = Vector::new(0.0, 0.0);
        self.nth = 0;
        self.length = 0.0;
        self.sub_path_start_length = 0.0;
        self.previous_command_was_move = false;
    }
}

impl<'l> PathBuilder for StrokeBuilder<'l> {
    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
        self.previous_command_was_move = false;
        let mut first = true;
        QuadraticBezierSegment {
            from: self.current,
            ctrl,
            to,
        }.for_each_flattened(
            self.options.tolerance,
            &mut |point| {
                self.edge_to(point, first);
                first = false;
            }
        );
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
        self.previous_command_was_move = false;
        let mut first = true;
        CubicBezierSegment {
            from: self.current,
            ctrl1,
            ctrl2,
            to,
        }.for_each_flattened(
            self.options.tolerance,
            &mut |point| {
                self.edge_to(point, first);
                first = false;
            }
        );
    }

    fn arc(
        &mut self,
        center: Point,
        radii: Vector,
        sweep_angle: Angle,
        x_rotation: Angle
    ) {
        let start_angle = (self.current - center).angle_from_x_axis() - x_rotation;
        let mut first = true;
        Arc {
            center,
            radii,
            start_angle,
            sweep_angle,
            x_rotation,
        }.for_each_flattened(
            self.options.tolerance,
            &mut |point| {
                self.edge_to(point, first);
                first = false;
            }
        );
    }
}

impl<'l> StrokeBuilder<'l> {
    pub fn new(
        options: &StrokeOptions,
        builder: &'l mut GeometryBuilder<Vertex>,
    ) -> Self {
        let zero = Point::new(0.0, 0.0);
        StrokeBuilder {
            first: zero,
            second: zero,
            previous: zero,
            current: zero,
            prev_normal: Vector::new(0.0, 0.0),
            previous_left_id: VertexId(0),
            previous_right_id: VertexId(0),
            second_left_id: VertexId(0),
            second_right_id: VertexId(0),
            nth: 0,
            length: 0.0,
            sub_path_start_length: 0.0,
            options: *options,
            previous_command_was_move: false,
            output: builder,
        }
    }

    pub fn set_options(&mut self, options: &StrokeOptions) { self.options = *options; }

    fn tessellate_empty_square_cap(&mut self) {
        let a = add_vertex!(
            self,
            Vertex {
                position: self.current,
                normal: vector(1.0, 1.0),
                advancement: 0.0,
                side: Side::Right,
            }
        );
        let b = add_vertex!(
            self,
            Vertex {
                position: self.current,
                normal: vector(1.0, -1.0),
                advancement: 0.0,
                side: Side::Left,
            }
        );
        let c = add_vertex!(
            self,
            Vertex {
                position: self.current,
                normal: vector(-1.0, -1.0),
                advancement: 0.0,
                side: Side::Left,
            }
        );
        let d = add_vertex!(
            self,
            Vertex {
                position: self.current,
                normal: vector(-1.0, 1.0),
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
                normal: vector(-1.0, 0.0),
                advancement: 0.0,
                side: Side::Left,
            }
        );
        let right_id = add_vertex!(
            self,
            Vertex {
                position: center,
                normal: vector(1.0, 0.0),
                advancement: 0.0,
                side: Side::Right,
            }
        );
        self.tessellate_round_cap(center, vector(0.0, -1.0), left_id, right_id, true);
        self.tessellate_round_cap(center, vector(0.0, 1.0), left_id, right_id, false);
    }

    fn finish(&mut self) {
        if self.nth == 0 && self.previous_command_was_move {
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
            self.edge_to(p, true);
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
    }

    fn edge_to(&mut self, to: Point, with_join: bool) {
        if to == self.current {
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

        let join_type = if with_join { self.options.line_join } else { LineJoin::Miter };

        let (
            start_left_id,
            start_right_id,
            end_left_id,
            end_right_id,
        ) = self.tessellate_join(
            previous_edge / previous_edge_length,
            next_tangent,
            join_type,
        );

        // Tessellate the edge
        if self.nth > 1 {
            self.output.add_triangle(self.previous_right_id, self.previous_left_id, start_left_id);
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
        let advancement = self.length;

        let quarter_angle = if is_start { -PI * 0.5 } else { PI * 0.5 };
        let mid_angle = directed_angle(vector(1.0, 0.0), dir);
        let left_angle = mid_angle + quarter_angle;
        let right_angle = mid_angle - quarter_angle;

        let mid_vertex = add_vertex!(
            self,
            Vertex {
                position: center,
                normal: dir,
                advancement,
                side: Side::Left,
            }
        );

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

        tess_round_cap(
            center,
            (left_angle, mid_angle),
            radius,
            left, mid_vertex,
            num_recursions,
            advancement,
            Side::Left,
            apply_width,
            !is_start,
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
            !is_start,
            self.output
        );
    }

    fn tessellate_join(&mut self,
        prev_tangent: Vector,
        next_tangent: Vector,
        mut join_type: LineJoin,
    ) -> (VertexId, VertexId, VertexId, VertexId) {
        // This function needs to differentiate the "front" of the join (aka. the pointy side)
        // from the back. The front is where subdivision or adjustments may be needed.

        let normal = compute_normal(prev_tangent, next_tangent);

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

        let threshold = 0.95; // TODO: look for a good constant here.
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
            LineJoin::MiterClip => {
                self.tessellate_miter_clip_join(
                    prev_tangent,
                    next_tangent,
                    front_side,
                    back_vertex,
                    normal
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
        prev_tangent: Vector,
        next_tangent: Vector,
        front_side: Side,
        back_vertex: VertexId,
    ) -> (VertexId, VertexId) {
        let neg_if_right = if front_side.is_left() { 1.0 } else { -1.0 };
        let prev_normal = vector(-prev_tangent.y, prev_tangent.x);
        let next_normal = vector(-next_tangent.y, next_tangent.x);

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

        let max_radius_segment_angle = compute_max_radius_segment_angle(self.options.line_width / 2.0, self.options.tolerance);
        let num_segments = (join_angle.abs() as f32 / max_radius_segment_angle).ceil() as u32;
        debug_assert!(num_segments > 0);
        // Calculate angle of each step
        let segment_angle = join_angle as f32 / num_segments as f32;

        let neg_if_right = if front_side.is_left() { 1.0 } else { -1.0 };

        // Calculate the initial front normal
        let initial_normal = vector(-prev_tangent.y, prev_tangent.x) * neg_if_right;

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
            n = vector(
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

            let (v1, v2, v3) = if front_side.is_left() {
                (back_vertex, last_vertex, current_vertex)
            } else {
                (back_vertex, current_vertex, last_vertex)
            };
            self.output.add_triangle(v1, v2, v3);

            last_vertex = current_vertex;
        }

        self.prev_normal = n * neg_if_right;

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
        let prev_normal: Vector = vector(-prev_tangent.y, prev_tangent.x);
        let next_normal: Vector = vector(-next_tangent.y, next_tangent.x);

        let (v1, v2) = self.get_clip_intersections(prev_normal, next_normal, normal);

        let start_vertex = add_vertex!(
            self,
            Vertex {
                position: self.current,
                normal: v1 * neg_if_right,
                advancement: self.length,
                side: front_side,
            }
        );

        let last_vertex = add_vertex!(
            self,
            Vertex {
                position: self.current,
                normal: v2 * neg_if_right,
                advancement: self.length,
                side: front_side,
            }
        );

        self.prev_normal = normal;

        let (v1, v2, v3) = if front_side.is_left() {
            (back_vertex, start_vertex, last_vertex)
        } else {
            (back_vertex, last_vertex, start_vertex)
        };
        self.output.add_triangle(v1, v2, v3);

        (start_vertex, last_vertex)
    }

    fn miter_limit_is_exceeded(&self, normal: Vector ) -> bool {
        normal.square_length() > self.options.miter_limit * self.options.miter_limit
    }

    fn get_clip_intersections(&self, prev_normal: Vector, next_normal: Vector, normal: Vector) -> (Vector, Vector) {
        let miter_length = self.options.miter_limit * self.options.line_width;
        let normal_limit = normal.normalize() * miter_length;

        let normal_limit_perp = LineSegment{
            from: point(normal_limit.x - normal_limit.y, normal_limit.y + normal_limit.x),
            to: point(normal_limit.x + normal_limit.y, normal_limit.y - normal_limit.x)
        };

        let prev_normal = prev_normal.to_point();
        let next_normal = next_normal.to_point();
        let normal = normal.to_point();

        let l1 = LineSegment{ from : prev_normal, to: normal };
        let l2 = LineSegment{ from: next_normal, to: normal };

        let i1 = l1.intersection(&normal_limit_perp).unwrap_or(prev_normal).to_vector();
        let i2 = l2.intersection(&normal_limit_perp).unwrap_or(next_normal).to_vector();

        (i1, i2)
    }
}

// Computes the max angle of a radius segment for a given tolerance
fn compute_max_radius_segment_angle(radius: f32, tolerance: f32) -> f32 {
    let t = radius - tolerance;
    ((radius * radius - t * t) * 4.0).sqrt() / radius
}

fn get_join_angle(prev_tangent: Vector, next_tangent: Vector) -> f32 {
    let mut join_angle = Trig::fast_atan2(prev_tangent.y, prev_tangent.x) - Trig::fast_atan2(next_tangent.y, next_tangent.x);

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
    advancement: f32,
    side: Side,
    line_width: f32,
    invert_winding: bool,
    output: &mut GeometryBuilder<Vertex>
) {
    if num_recursions == 0 {
        return;
    }

    let mid_angle = (angle.0 + angle.1) * 0.5;

    let normal = vector(mid_angle.cos(), mid_angle.sin());

    let vertex = output.add_vertex(Vertex {
        position: center + normal * line_width,
        normal,
        advancement,
        side,
    });

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
        advancement,
        side,
        line_width,
        invert_winding,
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
        invert_winding,
        output
    );
}

#[cfg(test)]
use path::default::{Path, PathSlice};
#[cfg(test)]
use geometry_builder::{
    SimpleBuffersBuilder, simple_builder, VertexBuffers,
};

#[cfg(test)]
fn test_path(
    path: PathSlice,
    options: &StrokeOptions,
    expected_triangle_count: Option<u32>
) {

    struct TestBuilder<'l> {
        builder: SimpleBuffersBuilder<'l, Vertex>,
    }

    impl<'l> GeometryBuilder<Vertex> for TestBuilder<'l> {
        fn begin_geometry(&mut self) {
            self.builder.begin_geometry();
        }
        fn end_geometry(&mut self) -> Count {
            self.builder.end_geometry()
        }
        fn add_vertex(&mut self, vertex: Vertex) -> VertexId {
            assert!(!vertex.position.x.is_nan());
            assert!(!vertex.position.y.is_nan());
            assert!(!vertex.normal.x.is_nan());
            assert!(!vertex.normal.y.is_nan());
            assert!(vertex.normal.square_length() != 0.0);
            assert!(!vertex.advancement.is_nan());
            self.builder.add_vertex(vertex)
        }
        fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
            assert!(a != b);
            assert!(a != c);
            assert!(b != c);
            let pa = self.builder.buffers().vertices[a.0 as usize].position;
            let pb = self.builder.buffers().vertices[b.0 as usize].position;
            let pc = self.builder.buffers().vertices[c.0 as usize].position;
            let threshold = -0.035; // Floating point errors :(
            assert!((pa - pb).cross(pc - pb) >= threshold);
            self.builder.add_triangle(a, b, c);
        }
        fn abort_geometry(&mut self) {
            panic!();
        }
    }

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    let mut tess = StrokeTessellator::new();
    let count = tess.tessellate_path(
        path.path_iter(),
        &options,
        &mut TestBuilder {
            builder: simple_builder(&mut buffers)
        }
    );

    if let Some(triangles) = expected_triangle_count {
        assert_eq!(triangles, count.indices / 3, "Unexpected number of triangles");
    }
}

#[test]
fn test_square() {
    let mut builder = Path::builder();

    builder.move_to(point(-1.0, 1.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(1.0, -1.0));
    builder.line_to(point(-1.0, -1.0));
    builder.close();

    let path1 = builder.build();

    let mut builder = Path::builder();

    builder.move_to(point(-1.0, -1.0));
    builder.line_to(point(1.0, -1.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(-1.0, 1.0));
    builder.close();

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
        &StrokeOptions::default().with_line_join(LineJoin::MiterClip).with_miter_limit(1.0),
        Some(12),
    );
    test_path(
        path2.as_slice(),
        &StrokeOptions::default().with_line_join(LineJoin::MiterClip).with_miter_limit(1.0),
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
    test_path(
        path.as_slice(),
        &StrokeOptions::default(),
        Some(0),
    );
}

#[test]
fn test_empty_caps() {
    let mut builder = Path::builder();

    builder.move_to(point(1.0, 0.0));
    builder.move_to(point(2.0, 0.0));
    builder.move_to(point(3.0, 0.0));

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
