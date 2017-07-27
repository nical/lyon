//! ## Path stroke tessellator
//!
//! Tessellation routines for path stroke operations.
//!
//! ## Overview
//!
//! The stroke tessellation algorithm simply generates a strip of triangles along
//! the path. This method is fast and simple to implement, howerver it means that
//! if the path overlap with itself (for example in the case of a self-intersecting
//! path), some triangles will overlap in the interesecting region, which may not
//! be the desired behavior. This needs to be kept in mind when rendering transparent
//! SVG strokes since the spec mandates that each point along a semi-transparent path
//! is shaded once no matter how many times the path overlaps with itself at this
//! location.
//!
//! The main interface is the [`StrokeTessellator`](struct.StrokeTessellator.html),
//! which exposes a similar interface to its
//! [fill equivalent](../path_fill/struct.FillTessellator.html).
//!
//! This stroke tessellator takes an iterator of path events as inputs as well as
//! a [`StrokeOption`](struct.StrokeOptions.html), and produces its outputs using
//! a [`GeometryBuilder`](../geometry_builder/trait.GeometryBuilder.html).
//!
//!
//! See the [`geometry_builder` module documentation](../geometry_builder/index.html)
//! for more details about how to output custom vertex layouts.
//!
//! # Examples
//!
//! ```
//! # extern crate lyon_tessellation;
//! # extern crate lyon_core;
//! # extern crate lyon_path;
//! # extern crate lyon_path_builder;
//! # extern crate lyon_path_iterator;
//! # use lyon_path::Path;
//! # use lyon_path_builder::*;
//! # use lyon_path_iterator::*;
//! # use lyon_core::math::{Point, point};
//! # use lyon_tessellation::geometry_builder::{VertexBuffers, simple_builder};
//! # use lyon_tessellation::path_stroke::*;
//! # use lyon_tessellation::StrokeVertex as Vertex;
//! # fn main() {
//! // Create a simple path.
//! let mut path_builder = Path::builder();
//! path_builder.move_to(point(0.0, 0.0));
//! path_builder.line_to(point(1.0, 2.0));
//! path_builder.line_to(point(2.0, 0.0));
//! path_builder.line_to(point(1.0, 1.0));
//! path_builder.close();
//! let path = path_builder.build();
//!
//! // Create the destination vertex and index buffers.
//! let mut buffers: VertexBuffers<Vertex> = VertexBuffers::new();
//!
//! {
//!     // Create the destination vertex and index buffers.
//!     let mut vertex_builder = simple_builder(&mut buffers);
//!
//!     // Create the tessellator.
//!     let mut tessellator = StrokeTessellator::new();
//!
//!     // Compute the tessellation.
//!     tessellator.tessellate(
//!         path.path_iter().flattened(0.05),
//!         &StrokeOptions::default(),
//!         &mut vertex_builder
//!     );
//! }
//!
//! println!("The generated vertices are: {:?}.", &buffers.vertices[..]);
//! println!("The generated indices are: {:?}.", &buffers.indices[..]);
//!
//! # }
//! ```

// See https://github.com/nical/lyon/wiki/Stroke-tessellation for some notes
// about how the path stroke tessellator is implemented.

use math::*;
use core::FlattenedEvent;
use bezier::utils::normalized_tangent;
use geometry_builder::{VertexId, GeometryBuilder, Count};
use path_builder::BaseBuilder;
use StrokeVertex as Vertex;
use Side;

/// A Context object that can tessellate stroke operations for complex paths.
pub struct StrokeTessellator {}

impl StrokeTessellator {
    pub fn new() -> StrokeTessellator { StrokeTessellator {} }

    pub fn tessellate<Input, Output>(
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
    length: f32,
    sub_path_start_length: f32,
    options: StrokeOptions,
    output: &'l mut Output,
}

impl<'l, Output: 'l + GeometryBuilder<Vertex>> BaseBuilder for StrokeBuilder<'l, Output> {
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
                   length: 0.0,
                   sub_path_start_length: 0.0,
                   options: *options,
                   output: builder,
               };
    }

    pub fn set_options(&mut self, options: &StrokeOptions) { self.options = *options; }

    fn finish(&mut self) {
        match self.options.line_cap {
            LineCap::Butt | LineCap::Square => {}
            _ => {
                println!(
                    "[StrokeTessellator] umimplemented {:?} line cap, defaulting to LineCap::Butt.",
                    self.options.line_cap
                );
            }
        }

        let hw = 0.5;

        if self.options.line_cap == LineCap::Square && self.nth == 0 {
            // Even if there is no edge, if we are using square caps we have to place a square
            // at the current position.
            let a = add_vertex!(
                self,
                Vertex {
                    position: self.current,
                    normal: vec2(-hw, -hw),
                    advancement: 0.0,
                    side: Side::Left,
                }
            );
            let b = add_vertex!(
                self,
                Vertex {
                    position: self.current,
                    normal: vec2(hw, -hw),
                    advancement: 0.0,
                    side: Side::Left,
                }
            );
            let c = add_vertex!(
                self,
                Vertex {
                    position: self.current,
                    normal: vec2(hw, hw),
                    advancement: 0.0,
                    side: Side::Right,
                }
            );
            let d = add_vertex!(
                self,
                Vertex {
                    position: self.current,
                    normal: vec2(-hw, hw),
                    advancement: 0.0,
                    side: Side::Right,
                }
            );
            self.output.add_triangle(a, b, c);
            self.output.add_triangle(a, c, d);
        }

        // last edge
        if self.nth > 0 {
            let current = self.current;
            let d = self.current - self.previous;
            if self.options.line_cap == LineCap::Square {
                // The easiest way to implement square caps is to lie about the current position
                // and move it slightly to accommodate for the width/2 extra length.
                self.current += d.normalize() * hw;
            }
            let p = self.current + d;
            self.edge_to(p);
            // Restore the real current position.
            self.current = current;
        }

        // first edge
        if self.nth > 1 {
            let mut first = self.first;
            let d = first - self.second;

            if self.options.line_cap == LineCap::Square {
                first += d.normalize() * hw;
            }

            let n2 = normalized_tangent(d) * 0.5;
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

            self.output.add_triangle(first_right_id, first_left_id, self.second_right_id);
            self.output.add_triangle(first_left_id, self.second_left_id, self.second_right_id);
        }
    }

    fn edge_to(&mut self, to: Point) {
        if (self.current - to).square_length() < 1e-5 {
            return;
        }

        if self.nth == 0 {
            // We don't have enough information to compute a and b yet.
            self.previous = self.first;
            self.current = to;
            self.nth += 1;
            return;
        }

        self.length += (self.current - self.previous).length();

        let normal = get_angle_normal(self.previous, self.current, to);

        let (start_left_id, start_right_id, end_left_id, end_right_id) = if self.nth > 0 {
            // Tesselate join
            self.tesselate_join(to, normal)
        } else {
            // Tesselate a cap at the start
            let left_id = add_vertex!(
                self,
                Vertex {
                    position: self.current,
                    normal: normal,
                    advancement: self.length,
                    side: Side::Left,
                }
            );

            let right_id = add_vertex!(
                self,
                Vertex {
                    position: self.current,
                    normal: -normal,
                    advancement: self.length,
                    side: Side::Right,
                }
            );

            (left_id, right_id, left_id, right_id)
        };

        // Tesselate edge
        if self.nth > 1 {
            self.output.add_triangle(self.previous_left_id, self.previous_right_id, start_left_id);
            self.output.add_triangle(self.previous_right_id, start_left_id, start_right_id);
        }

        self.previous = self.current;
        self.prev_normal = normal;
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

    fn tesselate_join(&mut self, to: Point, normal: Vec2) -> (VertexId, VertexId, VertexId, VertexId) {
        // Calculate which side is at the "front" of the join (aka. the pointy side)
        let a_line = self.current - self.previous;
        let b_line = to - self.current;
        let join_angle = a_line.y.atan2(a_line.x) - b_line.y.atan2(b_line.x);
        let front_side = if join_angle > 0.0 {
            Side::Left
        } else {
            Side::Right
        };

        // If the "front" is on the right, invert the normal
        let normal = match front_side {
            Side::Left => normal,
            Side::Right => -normal,
        };

        // Add a vertex at the back of the join
        let back_vertex = add_vertex!(
            self,
            Vertex {
                position: self.current,
                normal: -normal,
                advancement: self.length,
                side: front_side.opposite(),
            }
        );

        let (start_vertex, end_vertex) = match self.options.line_join {
            LineJoin::Miter => {
                let v = add_vertex!(
                    self,
                    Vertex {
                        position: self.current,
                        normal: normal,
                        advancement: self.length,
                        side: front_side,
                    }
                );

                (v, v)
            }

            LineJoin::Round => {
                let max_radius_segment_angle = compute_max_radius_segment_angle(self.options.line_width / 2.0, self.options.tolerance);
                let num_segments = (join_angle.abs() as f32 / max_radius_segment_angle).ceil() as u32;
                if num_segments != 0 {
                    // Calculate angle of each step
                    let segment_angle = join_angle as f32 / num_segments as f32;

                    // Calculate initial normal
                    let mut normal = if front_side == Side::Right {
                        vec2(a_line.y, -a_line.x).normalize() * 0.5
                    } else {
                        vec2(-a_line.y, a_line.x).normalize() * 0.5
                    };

                    let mut last_vertex = add_vertex!(
                        self,
                        Vertex {
                            position: self.current,
                            normal: normal,
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

                    for _ in 0..num_segments {
                        // Calculate normal
                        normal = vec2(
                            normal.x * rotation_matrix[0][0] + normal.y * rotation_matrix[0][1],
                            normal.x * rotation_matrix[1][0] + normal.y * rotation_matrix[1][1]
                        );

                        let current_vertex = add_vertex!(
                            self,
                            Vertex {
                                position: self.current,
                                normal: normal,
                                advancement: self.length,
                                side: front_side,
                            }
                        );

                        self.output.add_triangle(back_vertex, last_vertex, current_vertex);
                        last_vertex = current_vertex;
                    }

                    (start_vertex, last_vertex)
                } else {
                    // The join is perfectly straight
                    // TODO: Could we remove these vertices?
                    let v = add_vertex!(
                        self,
                        Vertex {
                            position: self.current,
                            normal: normal,
                            advancement: self.length,
                            side: front_side,
                        }
                    );

                    (v, v)
                }
            }

            // Fallback to Miter for unimplemented line joins
            _ => {
                println!("[StrokeTessellator] unimplemented line join.");
                let v = add_vertex!(
                    self,
                    Vertex {
                        position: self.current,
                        normal: normal,
                        advancement: self.length,
                        side: front_side,
                    }
                );

                (v, v)
            }
        };

        match front_side {
            Side::Left => (start_vertex, back_vertex, end_vertex, back_vertex),
            Side::Right => (back_vertex, start_vertex, back_vertex, end_vertex),
        }
    }
}

fn get_angle_normal(previous: Point, current: Point, next: Point) -> Vec2 {
    let epsilon = 1e-4;
    let half_width = 0.5;

    let v1 = (next - current).normalize();
    let v2 = (current - previous).normalize();
    let n1 = vec2(-v1.y, v1.x);

    let v12 = v1 + v2;

    if v12.square_length() < epsilon {
        return n1 * half_width;
    }

    let tangent = v12.normalize();
    let n = vec2(-tangent.y, tangent.x);

    let inv_len = n.dot(n1);

    if inv_len.abs() < epsilon {
        return n1 * half_width;
    }

    return n * half_width / inv_len;
}

/// Computes the max angle of a radius segment for a given tolerance
pub fn compute_max_radius_segment_angle(radius: f32, tolerance: f32) -> f32 {
    let t = radius - tolerance;
    ((radius * radius - t * t) * 4.0).sqrt() / radius
}

/// Parameters for the tessellator.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct StrokeOptions {
    /// See the SVG specification.
    pub line_cap: LineCap,

    /// See the SVG specification.
    ///
    /// Not implemented yet!
    pub line_join: LineJoin,

    /// Line width
    pub line_width: f32,

    /// See the SVG specification.
    ///
    /// Not implemented yet!
    pub miter_limit: f32,

    /// Maximum allowed distance to the path when building an approximation.
    pub tolerance: f32,

    /// An anti-aliasing trick extruding a 1-px wide strip around the edges with
    /// a gradient to smooth the edges.
    ///
    /// Not implemented yet!
    pub vertex_aa: bool,

    /// Apply line width
    ///
    /// When set to false, the generated vertices will all be positioned in the centre
    /// of the line. The width can be applied later on (eg in a vertex shader) by adding
    /// the vertex normal multiplied by the line with to each vertex position.
    pub apply_line_width: bool,

    // To be able to add fields without making it a breaking change, add an empty private field
    // which makes it impossible to create a StrokeOptions without calling the constructor.
    _private: (),
}

impl StrokeOptions {
    pub fn default() -> StrokeOptions {
        StrokeOptions {
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            line_width: 1.0,
            miter_limit: 10.0,
            tolerance: 0.1,
            vertex_aa: false,
            apply_line_width: true,
            _private: (),
        }
    }

    pub fn with_tolerance(mut self, tolerance: f32) -> StrokeOptions {
        self.tolerance = tolerance;
        return self;
    }

    pub fn with_line_cap(mut self, cap: LineCap) -> StrokeOptions {
        self.line_cap = cap;
        return self;
    }

    pub fn with_line_join(mut self, join: LineJoin) -> StrokeOptions {
        self.line_join = join;
        return self;
    }

    pub fn with_line_width(mut self, width: f32) -> StrokeOptions {
        self.line_width = width;
        return self;
    }

    pub fn with_miter_limit(mut self, limit: f32) -> StrokeOptions {
        self.miter_limit = limit;
        return self;
    }

    pub fn with_vertex_aa(mut self) -> StrokeOptions {
        self.vertex_aa = true;
        return self;
    }

    pub fn dont_apply_line_width(mut self) -> StrokeOptions {
        self.apply_line_width = false;
        return self;
    }
}


/// Line cap as defined by the SVG specification.
///
/// See: https://svgwg.org/specs/strokes/#StrokeLinecapProperty
///
/// <svg viewBox="0 0 400 399.99998" height="400" width="400">
///   <g transform="translate(0,-652.36229)">
///     <path style="opacity:1;fill:#80b3ff;stroke:#000000;stroke-width:1;stroke-linejoin:round;" d="m 240,983 a 30,30 0 0 1 -25,-15 30,30 0 0 1 0,-30.00001 30,30 0 0 1 25.98076,-15 l 0,30 z"/>
///     <path style="fill:#80b3ff;stroke:#000000;stroke-width:1px;stroke-linecap:butt;" d="m 390,782.6 -150,0 0,-60 150,0.5"/>
///     <circle style="opacity:1;fill:#ff7f2a;stroke:#000000;stroke-width:1;stroke-linejoin:round;" r="10" cy="752.89227" cx="240.86813"/>
///     <path style="fill:none;stroke:#000000;stroke-width:1px;stroke-linejoin:round;" d="m 240,722.6 150,60"/>
///     <path style="fill:#80b3ff;stroke:#000000;stroke-width:1px;stroke-linecap:butt;" d="m 390,882 -180,0 0,-60 180,0.4"/>
///     <circle style="opacity:1;fill:#ff7f2a;stroke:#000000;stroke-width:1;stroke-linejoin:round;" cx="239.86813" cy="852.20868" r="10" />
///     <path style="fill:none;stroke:#000000;stroke-width:1px;stroke-linejoin:round;" d="m 210.1,822.3 180,60"/>
///     <path style="fill:#80b3ff;stroke:#000000;stroke-width:1px;stroke-linecap:butt;" d="m 390,983 -150,0 0,-60 150,0.4"/>
///     <circle style="opacity:1;fill:#ff7f2a;stroke:#000000;stroke-width:1;stroke-linejoin:round;" cx="239.86813" cy="953.39734" r="10" />
///     <path style="fill:none;stroke:#000000;stroke-width:1px;stroke-linejoin:round;" d="m 390,983 -150,-60 L 210,953 l 30,30 -21.5,-9.5 L 210,953 218.3,932.5 240,923.4"/>
///     <text y="757.61273" x="183.65314" style="font-style:normal;font-weight:normal;font-size:20px;line-height:125%;font-family:Sans;text-align:end;text-anchor:end;fill:#000000;stroke:none;">
///        <tspan y="757.61273" x="183.65314">LineCap::Butt</tspan>
///        <tspan y="857.61273" x="183.65314">LineCap::Square</tspan>
///        <tspan y="957.61273" x="183.65314">LineCap::Round</tspan>
///      </text>
///   </g>
/// </svg>
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LineCap {
    /// The stroke for each subpath does not extend beyond its two endpoints.
    /// A zero length subpath will therefore not have any stroke.
    Butt,
    /// At the end of each subpath, the shape representing the stroke will be
    /// extended by a rectangle with the same width as the stroke width and
    /// whose length is half of the stroke width. If a subpath has zero length,
    /// then the resulting effect is that the stroke for that subpath consists
    /// solely of a square with side length equal to the stroke width, centered
    /// at the subpath's point.
    Square,
    /// [Not implemented] At each end of each subpath, the shape representing
    /// the stroke will be extended by a half circle with a radius equal to the
    /// stroke width. If a subpath has zero length, then the resulting effect is
    /// that the stroke for that subpath consists solely of a full circle centered
    /// at the subpath's point.
    Round,
}

/// Line join as defined by the SVG specification.
///
/// See: https://svgwg.org/specs/strokes/#StrokeLinejoinProperty
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LineJoin {
    /// A sharp corner is to be used to join path segments.
    Miter,
    /// [Not implemented] Same as a miter join, but if the miter limit is exceeded,
    /// the miter is clipped at a miter length equal to the miter limit value
    /// multiplied by the stroke width.
    MiterClip,
    /// A round corner is to be used to join path segments.
    Round,
    /// [Not implemented] A bevelled corner is to be used to join path segments.
    /// The bevel shape is a triangle that fills the area between the two stroked
    /// segments.
    Bevel,
}
