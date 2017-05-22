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
//! The main interface is the [StrokeTessellator](struct.StrokeTessellator.html),
//! which exposes a similar interface to its
//! [fill equivalent](../path_fill/struct.FillTessellator.html).
//!
//! This stroke tessellator takes an iterator of path events as inputs as well as
//! a [StrokeOption](struct.StrokeOptions.html), and produces its outputs using
//! a [BezierGeometryBuilder](../geometry_builder/trait.BezierGeometryBuilder.html).
//!
//!
//! See the [geometry_builder module documentation](../geometry_builder/index.html)
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
//!     let result = tessellator.tessellate(
//!         path.path_iter().flattened(0.05),
//!         &StrokeOptions::default(),
//!         &mut vertex_builder
//!     );
//!     assert!(result.is_ok());
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
use geometry_builder::{VertexId, GeometryBuilder, Count};
use math_utils::{tangent, line_intersection};
use path_builder::BaseBuilder;
use StrokeVertex as Vertex;
use Side;

pub type StrokeResult = Result<Count, ()>;

/// A Context object that can tessellate stroke operations for complex paths.
pub struct StrokeTessellator {}

impl StrokeTessellator {
    pub fn new() -> StrokeTessellator { StrokeTessellator {} }

    pub fn tessellate<Input, Output>(
        &mut self,
        input: Input,
        options: &StrokeOptions,
        builder: &mut Output,
    ) -> StrokeResult
    where
        Input: Iterator<Item = FlattenedEvent>,
        Output: GeometryBuilder<Vertex>,
    {
        builder.begin_geometry();
        let mut stroker = StrokeBuilder::new(options, builder);

        for evt in input {
            stroker.flat_event(evt);
        }

        return stroker.build();
    }
}

/// A builder that tessellates a stroke directly without allocating any intermediate data structure.
pub struct StrokeBuilder<'l, Output: 'l> {
    first: Point,
    previous: Point,
    current: Point,
    second: Point,
    previous_a_id: VertexId,
    previous_b_id: VertexId,
    second_a_id: VertexId,
    second_b_id: VertexId,
    nth: u32,
    options: StrokeOptions,
    output: &'l mut Output,
}

impl<'l, Output: 'l + GeometryBuilder<Vertex>> BaseBuilder for StrokeBuilder<'l, Output> {
    type PathType = StrokeResult;

    fn move_to(&mut self, to: Point) {
        self.finish();

        self.first = to;
        self.current = to;
        self.nth = 0;
    }

    fn line_to(&mut self, to: Point) { self.edge_to(to); }

    fn close(&mut self) {
        let first = self.first;
        self.edge_to(first);
        if self.nth > 1 {
            let second = self.second;
            self.edge_to(second);
            self.output.add_triangle(self.previous_b_id, self.previous_a_id, self.second_b_id);
            self.output.add_triangle(self.previous_a_id, self.second_a_id, self.second_b_id);
        }
        self.nth = 0;
        self.current = self.first;
    }

    fn current_position(&self) -> Point { self.current }

    fn build(mut self) -> StrokeResult {
        self.finish();
        return Ok(self.output.end_geometry());
    }

    fn build_and_reset(&mut self) -> StrokeResult {
        self.first = Point::new(0.0, 0.0);
        self.previous = Point::new(0.0, 0.0);
        self.current = Point::new(0.0, 0.0);
        self.second = Point::new(0.0, 0.0);
        self.nth = 0;
        return Ok(self.output.end_geometry());
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
                   previous_a_id: VertexId(0),
                   previous_b_id: VertexId(0),
                   second_a_id: VertexId(0),
                   second_b_id: VertexId(0),
                   nth: 0,
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
            let a = self.output.add_vertex(
                Vertex {
                    position: self.current,
                    normal: vec2(-hw, -hw),
                    side: Side::Left,
                }
            );
            let b = self.output.add_vertex(
                Vertex {
                    position: self.current,
                    normal: vec2(hw, -hw),
                    side: Side::Left,
                }
            );
            let c = self.output.add_vertex(
                Vertex {
                    position: self.current,
                    normal: vec2(hw, hw),
                    side: Side::Right,
                }
            );
            let d = self.output.add_vertex(
                Vertex {
                    position: self.current,
                    normal: vec2(-hw, hw),
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
                self.current = self.current + d.normalized() * hw;
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
                first = first + d.normalized() * hw;
            }

            let n2 = tangent(d) * 0.5;
            let n1 = -n2;

            let first_a_id = self.output.add_vertex(
                Vertex {
                    position: first,
                    normal: n1,
                    side: Side::Left,
                }
            );
            let first_b_id = self.output.add_vertex(
                Vertex {
                    position: first,
                    normal: n2,
                    side: Side::Right,
                }
            );

            self.output.add_triangle(first_b_id, first_a_id, self.second_b_id);
            self.output.add_triangle(first_a_id, self.second_a_id, self.second_b_id);
        }
    }

    fn edge_to(&mut self, to: Point) {
        if self.current == to {
            return;
        }
        if self.nth == 0 {
            // We don't have enough information to compute a and b yet.
            self.previous = self.first;
            self.current = to;
            self.nth += 1;
            return;
        }
        let (na, nb, maybe_nc) = get_angle_info(self.previous, self.current, to);
        let a_id = self.output.add_vertex(
            Vertex {
                position: self.current,
                normal: na,
                side: Side::Left,
            }
        );
        let b_id = self.output.add_vertex(
            Vertex {
                position: self.current,
                normal: nb,
                side: Side::Right,
            }
        );

        let (nc, c_id) = if let Some(n) = maybe_nc {
            (
                n,
                self.output.add_vertex(
                    Vertex {
                        position: self.current,
                        normal: n,
                        side: Side::Left, // TODO
                    }
                )
            )
        } else {
            (nb, b_id)
        };

        if self.nth > 1 {
            self.output.add_triangle(self.previous_b_id, self.previous_a_id, b_id);
            self.output.add_triangle(self.previous_a_id, a_id, b_id);
        }

        self.previous = self.current;
        self.previous_a_id = a_id;
        self.previous_b_id = c_id;
        self.current = to;

        if self.nth == 1 {
            self.second = self.previous;
            self.second_a_id = a_id;
            self.second_b_id = c_id;
        }

        if maybe_nc.is_some() {
            let current = self.current;
            self.tessellate_angle(current, na, a_id, nb, b_id, nc, c_id);
        }

        self.nth += 1;
    }

    fn tessellate_angle(
        &mut self,
        _position: Point,
        _na: Vec2,
        a_id: VertexId,
        _nb: Vec2,
        b_id: VertexId,
        _nc: Vec2,
        c_id: VertexId,
    ) {
        // TODO: Properly support all types of angles.
        self.output.add_triangle(b_id, a_id, c_id);
    }
}

fn get_angle_info(previous: Point, current: Point, next: Point) -> (Point, Point, Option<Point>) {
    let amount = 0.5;
    let n1 = tangent(current - previous) * amount;
    let n2 = tangent(next - current) * amount;

    // Segment P1-->PX
    let pn1 = previous + n1; // prev extruded along the tangent n1
    let pn1x = current + n1; // px extruded along the tangent n1
    // Segment PX-->P2
    let pn2 = next + n2;
    let pn2x = current + n2;

    let inter = match line_intersection(pn1, pn1x, pn2x, pn2) {
        Some(v) => v,
        None => {
            if (n1 - n2).square_length() < 0.000001 {
                pn1x
            } else {
                println!("[StrokeTessellator] unimplemented narrow angle."); // TODO
                current + (current - previous) * amount / (current - previous).length()
            }
        }
    };
    return (inter - current, current - inter, None);
}

/// Parameters for the tessellator.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct StrokeOptions {
    /// See the SVG secification.
    pub line_cap: LineCap,

    /// See the SVG secification.
    ///
    /// Not implemented yet!
    pub line_join: LineJoin,

    /// See the SVG secification.
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

    // To be able to add fields without making it a breaking change, add an empty private field
    // which makes it impossible to create a StrokeOptions without calling the constructor.
    _private: (),
}

impl StrokeOptions {
    pub fn default() -> StrokeOptions {
        StrokeOptions {
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            miter_limit: 10.0,
            tolerance: 0.1,
            vertex_aa: false,
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

    pub fn with_miter_limit(mut self, limit: f32) -> StrokeOptions {
        self.miter_limit = limit;
        return self;
    }

    pub fn with_vertex_aa(mut self) -> StrokeOptions {
        self.vertex_aa = true;
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
    /// [Not implemented] A round corner is to be used to join path segments.
    Round,
    /// [Not implemented] A bevelled corner is to be used to join path segments.
    /// The bevel shape is a triangle that fills the area between the two stroked
    /// segments.
    Bevel,
}
