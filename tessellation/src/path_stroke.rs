//! Tessellation routines for path stroke operations.
//!
//! The current implementation is pretty bad and does not deal with overlap in an svg-compliant
//! way.

use std::f32::NAN;

use math::*;
use lyon_path::*;
use geometry_builder::{ GeometryBuilder, Count, };
use math_utils::{ tangent, directed_angle, directed_angle2, line_intersection, };
use basic_shapes::{ tessellate_quad };

pub type StrokeResult = Result<Count, ()>;

pub struct StrokeTessellator {}

impl StrokeTessellator {
    pub fn new() -> StrokeTessellator { StrokeTessellator {} }

    pub fn tessellate_path<Output: GeometryBuilder<Point>>(
        &mut self,
        path: PathSlice,
        options: &StrokeOptions,
        output: &mut Output
    ) -> StrokeResult {
        output.begin_geometry();
        for p in path.path_ids() {
            tessellate_sub_path_stroke(path.sub_path(p), options.stroke_width, output);
        }
        let ranges = output.end_geometry();
        return Ok(ranges);
    }
}

fn tessellate_sub_path_stroke<Output: GeometryBuilder<Point>>(
    path: SubPathSlice,
    stroke_width: f32,
    output: &mut Output
) {
    let is_closed = path.info().is_closed;

    let first = path.first();
    let mut i = first;
    let mut done = false;

    let mut prev_v1 = vec2(NAN, NAN);
    let mut prev_v2 = vec2(NAN, NAN);
    loop {
        let mut p1 = path.vertex(i).position;
        let mut p2 = path.vertex(i).position;

        let extruded = extrude_along_tangent(path, i, stroke_width, is_closed);
        let d = extruded - p1;

        p1 = p1 + (d * 0.5);
        p2 = p2 - (d * 0.5);

        if i != first || done {
            // TODO: should reuse vertices instead of tessellating quads
            tessellate_quad(prev_v1, prev_v2, p2, p1, output);
        }

        if done {
            break;
        }

        prev_v1 = p1;
        prev_v2 = p2;

        i = path.next(i);

        if i == first {
            if !is_closed {
                break;
            }
            done = true;
        }
    }
}

fn extrude_along_tangent(
    path: SubPathSlice,
    i: VertexId,
    amount: f32,
    is_closed: bool
) -> Vec2 {

    let px = path.vertex(i).position;
    let _next = path.next_vertex(i).position;
    let _prev = path.previous_vertex(i).position;

    let prev = if i == path.first() && !is_closed { px + px - _next } else { _prev };
    let next = if i == path.last() && !is_closed { px + px - _prev } else { _next };

    let n1 = tangent(px - prev) * amount;
    let n2 = tangent(next - px) * amount;

    // Segment P1-->PX
    let pn1  = prev + n1; // prev extruded along the tangent n1
    let pn1x = px + n1; // px extruded along the tangent n1
    // Segment PX-->P2
    let pn2  = next + n2;
    let pn2x = px + n2;

    let inter = match line_intersection(pn1, pn1x, pn2x, pn2) {
        Some(v) => { v }
        None => {
            if (n1 - n2).square_length() < 0.000001 {
                pn1x
            } else {
                // TODO: the angle is very narrow, use rounded corner instead
                //panic!("Not implemented yet");
                println!("!! narrow angle at {:?} {:?} {:?} | {:?} {:?} {:?}",
                    px, directed_angle(n1, n2), directed_angle2(px, prev, next),
                    prev.tuple(), px.tuple(), next.tuple(),
                );
                px + (px - prev) * amount / (px - prev).length()
            }
        }
    };
    return inter;
}

/// Line cap as defined by the SVG specification.
///
/// See: https://svgwg.org/specs/strokes/#StrokeLinecapProperty
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LineCap {
    Butt,
    Round,
    Square
}

/// Line join as defined by the SVG specification.
///
/// See: https://svgwg.org/specs/strokes/#StrokeLinejoinProperty
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LineJoin {
    Miter,
    MiterClip,
    Round,
    Bevel,
    Arcs,
}

/// Parameters for the tessellator.
///
/// Not used yet (only one configuration supported).
pub struct StrokeOptions {
    /// Thickness of the stroke.
    pub stroke_width: f32,

    /// See the SVG secification.
    pub line_cap: LineCap,

    /// See the SVG secification.
    pub line_join: LineJoin,

    /// See the SVG secification.
    pub miter_limit: f32,

    /// Maximum allowed distance to the path when building an approximation.
    pub tolerance: f32,

    /// The number of tesselator units per world unit.
    ///
    /// As the tesselator is internally using integer coordinates, this parameter defines
    /// the precision and range of the tesselator.
    pub unit_scale: f32,

    /// An anti-aliasing trick extruding a 1-px wide strip around the edges with
    /// a gradient to smooth the edges.
    ///
    /// Not implemented yet!
    pub vertex_aa: bool,

    // To be able to add fields without making it a breaking change, add an empty private field
    // which makes it impossible to create a StrokeOptions without the calling constructor.
    _private: (),
}

impl StrokeOptions {
    pub fn stroke_width(stroke_width: f32) -> StrokeOptions {
        StrokeOptions {
            stroke_width: stroke_width,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            miter_limit: 10.0,
            tolerance: 0.1,
            unit_scale: 1000.0,
            vertex_aa: false,
            _private: (),
        }
    }

    pub fn default() -> StrokeOptions { StrokeOptions::stroke_width(1.0) }

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

    pub fn with_stroke_width(mut self, width: f32) -> StrokeOptions {
        self.stroke_width = width;
        return self;
    }

    pub fn with_unit_scale(mut self, scale: f32) -> StrokeOptions {
        self.unit_scale = scale;
        return self;
    }

    pub fn with_vertex_aa(mut self) -> StrokeOptions {
        self.vertex_aa = true;
        return self;
    }
}
