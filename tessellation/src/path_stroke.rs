//! Tessellation routines for path stroke operations.
//!
//! The current implementation is pretty bad and does not deal with overlap in an svg-compliant
//! way.

use std::f32::NAN;

use math::*;
use path::*;
use vertex_builder::{ VertexBufferBuilder, Range, };
use math_utils::{ tangent, directed_angle, directed_angle2, line_intersection, };
use basic_shapes::{ tesselate_quad };
use super::{ VertexId };

pub type StrokeResult = Result<(Range, Range), ()>;

pub struct StrokeTesselator {}

impl StrokeTesselator {
    pub fn new() -> StrokeTesselator { StrokeTesselator {} }

    pub fn tesselate<Output: VertexBufferBuilder<Vec2>>(
        &mut self,
        path: PathSlice,
        thickness: f32,
        output: &mut Output
    )  -> StrokeResult {
        output.begin_geometry();
        for p in path.path_ids() {
            tesselate_sub_path_stroke(path.sub_path(p), thickness, output);
        }
        let ranges = output.end_geometry();
        return Ok(ranges);
    }
}

fn tesselate_sub_path_stroke<Output: VertexBufferBuilder<Vec2>>(
    path: SubPathSlice,
    thickness: f32,
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

        let extruded = extrude_along_tangent(path, i, thickness, is_closed);
        let d = extruded - p1;

        p1 = p1 + (d * 0.5);
        p2 = p2 - (d * 0.5);

        if i != first || done {
            // TODO: should reuse vertices instead of tesselating quads
            tesselate_quad(prev_v1, prev_v2, p2, p1, output);
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
