//! Tessellation routines for complex path fill operations.

use std::f32::consts::PI;
use std::i32;
use std::mem::swap;
use std::cmp::{ PartialOrd, Ordering };
use std::cmp;

use math::*;
use geometry_builder::{ BezierGeometryBuilder, Count, VertexId };
use core::{ FlattenedEvent };
use math_utils::{
    is_below, is_below_fixed, directed_angle, directed_angle2,
};

#[cfg(test)]
use geometry_builder::{ VertexBuffers, simple_builder };
#[cfg(test)]
use path::{ Path, PathSlice };
#[cfg(test)]
use path_iterator::PathIterator;
#[cfg(test)]
use path_builder::PathBuilder;
#[cfg(test)]
use extra::rust_logo::build_logo_path;

pub type FillResult = Result<Count, FillError>;

#[derive(Clone, Debug)]
pub enum FillError {
    Unknown,
}

#[derive(Copy, Clone, Debug)]
struct Edge {
    upper: TessPoint,
    lower: TessPoint,
}

#[derive(Clone, Debug)]
struct EdgeBelow {
    // The upper vertex is the current vertex, we don't need to store it.
    lower: TessPoint,
    angle: f32,
}

// These two constants define the precision of the tessellator.
const UNIT_SCALE: f32 = 1000.0; // number of tessellator unit per world unit.
const UNIT_INV_SCALE: f32 = 0.001; // Size of a tessellator unit in world units.

/// A Context object that can tessellate fill operations for complex paths.
///
/// The Tessellator API is not stable yet. For example it is not clear whether we will use
/// separate Tessellator structs for some of the different configurations (vertex-aa, etc),
/// or if evertything can be implemented with the same algorithm.
pub struct FillTessellator {
    sweep_line: Vec<Span>,
    monotone_tessellators: Vec<MonotoneTessellator>,
    intersections: Vec<Edge>,
    below: Vec<EdgeBelow>,
    previous_position: TessPoint,
    error: Option<FillError>,
    log: bool,
}

impl FillTessellator {
    /// Constructor.
    pub fn new() -> FillTessellator {
        FillTessellator {
            sweep_line: Vec::with_capacity(16),
            monotone_tessellators: Vec::with_capacity(16),
            below: Vec::with_capacity(8),
            intersections: Vec::with_capacity(8),
            previous_position: TessPoint::new(FixedPoint32::min_val(), FixedPoint32::min_val()),
            error: None,
            log: false,
        }
    }

    /// Compute the tessellation.
    pub fn tessellate_events<Output: BezierGeometryBuilder<Point>>(&mut self,
        events: &FillEvents,
        options: &FillOptions,
        output: &mut Output
    ) -> FillResult {
        if options.vertex_aa {
            println!("warning: Vertex-aa is not supported yet.");
        }

        if options.fill_rule != FillRule::EvenOdd {
            println!("warning: Fill rule {:?} is not supported yet.", options.fill_rule);
        }

        self.begin_tessellation(output);

        self.tessellator_loop(&events, output);

        let mut error = None;
        swap(&mut error, &mut self.error);
        if let Some(err) = error {
            output.abort_geometry();
            return Err(err);
        }

        let res = self.end_tessellation(output);
        return Ok(res);
    }

    /// Enable some verbose logging during the tessellation, for debugging purposes.
    pub fn enable_logging(&mut self) { self.log = true; }

    fn begin_tessellation<Output: BezierGeometryBuilder<Point>>(&mut self, output: &mut Output) {
        debug_assert!(self.sweep_line.is_empty());
        debug_assert!(self.monotone_tessellators.is_empty());
        debug_assert!(self.below.is_empty());
        output.begin_geometry();
    }

    fn end_tessellation<Output: BezierGeometryBuilder<Point>>(&mut self, output: &mut Output) -> Count {
        debug_assert!(self.sweep_line.is_empty());
        debug_assert!(self.monotone_tessellators.is_empty());
        debug_assert!(self.below.is_empty());
        return output.end_geometry();
    }

    fn tessellator_loop<Output: BezierGeometryBuilder<Point>>(&mut self,
        events: &FillEvents,
        output: &mut Output
    ) {
        let mut current_position = TessPoint::new(FixedPoint32::min_val(), FixedPoint32::min_val());

        let mut edge_iter = events.edges.iter();
        let mut vertex_iter = events.vertices.iter();
        let mut next_edge = edge_iter.next();
        let mut next_vertex = vertex_iter.next();
        loop {
            if self.error.is_some() {
                return;
            }

            let mut next_position = None;
            let mut pending_events = false;

            while let Some(edge) = next_edge {
                if edge.upper == current_position {
                    let edge_vec = to_vec2(edge.lower - edge.upper);
                    next_edge = edge_iter.next();
                    if edge.lower != current_position {
                        self.below.push(EdgeBelow{
                            lower: edge.lower,
                            angle: -directed_angle(vec2(1.0, 0.0), edge_vec),
                        });
                    }
                    pending_events = true;
                    if self.log {
                        println!(" edge at {:?} -> {:?}", edge.upper.tuple(), edge.lower.tuple());
                    }
                    continue;
                }

                next_position = Some(edge.upper);
                break;
            }

            while let Some(vertex) = next_vertex {
                if *vertex == current_position {
                    next_vertex = vertex_iter.next();
                    pending_events = true;
                    if self.log {
                        println!(" vertex at {:?}", current_position.tuple());
                    }
                    continue;
                }
                if next_position.is_none() || is_below_fixed(next_position.unwrap(), *vertex) {
                    next_position = Some(*vertex);
                }
                break;
            }

            while !self.intersections.is_empty() {
                let intersection_position = self.intersections[0].upper;
                if intersection_position == current_position {
                    let inter = self.intersections.remove(0);

                    if inter.lower != current_position {
                        let vec = to_vec2(inter.lower - current_position);
                        self.below.push(EdgeBelow {
                            lower: inter.lower,
                            angle: -directed_angle(vec2(1.0, 0.0), vec),
                        });
                    }

                    pending_events = true;
                    continue
                }
                if next_position.is_none() || is_below_fixed(next_position.unwrap(), intersection_position) {
                    next_position = Some(intersection_position);
                }
                break;
            }

            let mut found_intersections = false;
            if pending_events {
                let num_intersections = self.intersections.len();
                self.process_vertex(current_position, output);
                found_intersections = num_intersections != self.intersections.len();
            }

            if found_intersections {
                continue;
            }

            if let Some(position) = next_position {
                current_position = position;
                if self.log { println!(" -- current_position is now {:?}", position.tuple()); }
            } else {
                return;
            }
        }
    }

    fn process_vertex<Output: BezierGeometryBuilder<Point>>(&mut self, current_position: TessPoint, output: &mut Output) {

        let vec2_position = to_vec2(current_position);
        let id = output.add_vertex(vec2_position);

        // Walk the sweep line to determine where we are with respect to the
        // existing spans.
        let mut start_span = 0;

        #[derive(Copy, Clone, Debug, PartialEq)]
        enum E { In, Out, LeftEdge, RightEdge };
        let mut status = E::Out;

        for span in &mut self.sweep_line {
            if !span.left.merge && span.left.lower == current_position {
                status = E::LeftEdge;
                break;
            }

            if test_span_touches(&span.left, current_position) {
                // TODO: compute directed angles using fixed point vectors.
                self.below.push(EdgeBelow {
                    lower: span.left.lower,
                    angle: -directed_angle(vec2(1.0, 0.0), to_vec2(span.left.lower - current_position))
                });
                span.left.lower = current_position;
                status = E::LeftEdge;
                //println!(" ---- touch left");
                break;
            }

            if test_span_side(&span.left, current_position) {
                status = E::Out;
                break;
            }

            if !span.right.merge && span.right.lower == current_position {
                // TODO: this leads to incorrect results if the next span also touches.
                // see test_colinear_touching_squares2.
                status = E::RightEdge;
                break;
            }

            if test_span_touches(&span.right, current_position) {
                self.below.push(EdgeBelow {
                    lower: span.right.lower,
                    angle: -directed_angle(vec2(1.0, 0.0), to_vec2(span.right.lower - current_position))
                });
                span.right.lower = current_position;
                status = E::RightEdge;
                //println!(" ---- touch right");
                break;
            }

            if test_span_side(&span.right, current_position) {
                status = E::In;
                break;
            }

            start_span += 1;
        }

        self.below.sort_by(|a, b| {
            a.angle.partial_cmp(&b.angle).unwrap_or(Ordering::Equal)
        });

        if self.log {
            println!("\n\n\n\n");
            self.log_sl_ids();
            self.log_sl_points_at(current_position.y);
            println!("\n ----- current: {:?} ------ {:?} {:?}", current_position, start_span, status);
            for b in &self.below {
                println!("   -- below: {:?}", b);
            }
        }

        // the index of the next edge below the current vertex, to be processed.
        let mut below_idx = 0;

        let mut span_idx = start_span;
        let mut pending_merge = false;
        let mut above_count = 0;
        let mut below_count = self.below.len();

        // Step 1, walk the sweep line, handle left/right events, handle the spans that end
        // at this vertex, as well as merge events.
        if start_span < self.sweep_line.len() {
            if status == E::RightEdge {
                if below_count == 0 {
                    // we'll merge with the right most edge, there may be end events
                    // in the middle so we handle the merge event later. Since end
                    // events remove their spans, we don't need to remember the current
                    // span index to process the merge.
                    pending_merge = true;
                } else {
                    // Right event.
                    //
                    //   ..../
                    //   ...x
                    //   ....\
                    //
                    if self.log { println!("(right event) {}", start_span); }

                    let edge = Edge { upper: current_position, lower: self.below[0].lower };
                    self.insert_edge(start_span, Side::Right, edge, id);

                    // update the initial state for the pass that will handle
                    // the edges below the current vertex.
                    start_span += 1;
                    below_idx += 1;
                    below_count -= 1;
                    status = E::Out;
                }
                span_idx += 1;
            }
        }

        // Count the number of remaining edges above the sweep line that end at
        // the current position.
        for span in &self.sweep_line[span_idx..] {
            let left = !span.left.merge && span.left.lower == current_position;
            let right = !span.right.merge && span.right.lower == current_position;
            // Here it is tempting to assume that we can only have end events
            // if left && right, but we also need to take merge vertices into account.
            if left { above_count += 1; }
            if right { above_count += 1; }

            // We can't assume that if left and right are false we are already past
            // the current point because both sides of the span could be in the merge state.

            // If right is true, left should be true as well, unless it is a merge.
            debug_assert!(!right || left || span.left.merge,
                "The right edge {:?} touches but the left edge {:?} does not. merge: {}",
                span.right.lower,
                span.left.lower,
                span.left.merge
            );
        }

        // Pairs of edges that end at the current position form "end events".
        // By construction we know that they are on the same spans.
        while above_count >= 2 {
            // End event.
            //
            //   \.../
            //    \./
            //     x
            //
            if self.log { println!("(end event) {}", span_idx); }

            self.resolve_merge_vertices(span_idx, current_position, id, output);
            self.end_span(span_idx, current_position, id, output);

            above_count -= 2;
        }

        if pending_merge {
            // Merge event.
            //
            // ...\   /...
            // ....\ /....
            // .....x.....
            //
            if self.log { println!("(merge event) {}", start_span); }

            debug_assert!(above_count == 1);
            self.merge_event(current_position, id, start_span, output)

        } else if above_count == 1 {
            // Left event.
            //
            //     /...
            //    x....
            //     \...
            //

            debug_assert!(below_count > 0);
            self.resolve_merge_vertices(span_idx, current_position, id, output);

            let vertex_below = self.below[self.below.len()-1].lower;
            if self.log { println!("(left event) {}    -> {:?}", span_idx, vertex_below); }
            self.insert_edge(
                span_idx, Side::Left,
                Edge { upper: current_position, lower: vertex_below }, id,
            );

            below_count -= 1;
        }

        // Since we took care of left and right event already we should not have
        // an odd number of edges to work with below the current vertex by now.
        debug_assert!(below_count % 2 == 0);

        // reset span_idx for the next pass.
        let mut span_idx = start_span;

        // Step 2, handle edges below the current vertex.
        if below_count > 0 {
            if status == E::In {
                // Split event.
                //
                // .....x.....
                // ..../ \....
                // .../   \...
                //
                if self.log { println!("(split event) {}", start_span); }

                let left = self.below[0].clone();
                let right = self.below[below_count-1].clone();
                self.split_event(start_span, current_position, id, left, right, output);
                below_count -= 2;
                below_idx += 1;
            }

            while below_count >= 2 {
                // Start event.
                //
                //      x
                //     /.\
                //    /...\
                //
                if self.log { println!("(start event) {}", span_idx); }

                let l = self.below[below_idx].lower;
                let r = self.below[below_idx + 1].lower;
                let mut left_edge = Edge { upper: current_position, lower: l };
                let mut right_edge = Edge { upper: current_position, lower: r };

                if self.below[below_idx].angle != self.below[below_idx+1].angle {
                    // In most cases:
                    self.check_intersections(&mut left_edge);
                    self.check_intersections(&mut right_edge);
                    self.sweep_line.insert(span_idx, Span::begin(current_position, id, left_edge.lower, right_edge.lower));
                    let vec2_position = to_vec2(current_position);
                    self.monotone_tessellators.insert(span_idx, MonotoneTessellator::begin(vec2_position, id));
                } else {
                    // If the two edges are colinear we "postpone" the beginning of this span
                    // since at this level there is nothing to fill in a zero-area span.
                    //
                    //     x  <- current point
                    //     |  <- zero area to fill while the span sides overlap.
                    //     x  <- postponed start event
                    //     |\
                    //     x.\
                    //
                    // TODO: this doesn't work if there is an intersection with another span above
                    // the postponed position.

                    if l == r {
                        // just skip these two egdes.
                    } else if is_below_fixed(l, r) {
                        self.intersections.push(Edge { upper: r, lower: l });
                    } else {
                        self.intersections.push(Edge { upper: l, lower: r });
                    }
                }
                below_idx += 2;
                below_count -= 2;
                span_idx += 1;
            }
        }

        self.debug_check_sl(current_position);

        self.below.clear();
    }

    // Look for eventual merge vertices on this span above the current vertex, and connect
    // them to the current vertex.
    // This should be called when processing a vertex that is on the left side of a span.
    fn resolve_merge_vertices<Output: BezierGeometryBuilder<Point>>(&mut self,
        span_idx: usize,
        current: TessPoint, id: VertexId,
        output: &mut Output
    ) {
        while self.sweep_line[span_idx].right.merge {
            //     \ /
            //  \   x   <-- merge vertex
            //   \ :
            //    x   <-- current vertex
            self.sweep_line[span_idx+1].set_lower_vertex(current, Side::Left);
            self.end_span(span_idx, current, id, output);
        }
    }

    fn split_event<Output: BezierGeometryBuilder<Point>>(&mut self,
        span_idx: usize, current: TessPoint, id: VertexId,
        left: EdgeBelow, right: EdgeBelow,
        output: &mut Output
    ) {
        // Look whether the span shares a merge vertex with the previous one
        if self.sweep_line[span_idx].left.merge {
            let left_span = span_idx-1;
            let right_span = span_idx;
            //            \ /
            //             x   <-- merge vertex
            //  left_span  :  righ_span
            //             x   <-- current split vertex
            //           l/ \r
            self.insert_edge(
                left_span, Side::Right,
                Edge { upper: current, lower: left.lower }, id,
            );
            self.insert_edge(
                right_span, Side::Left,
                Edge { upper: current, lower: right.lower }, id,
            );

            // There may be more merge vertices chained on the right of the current span, now
            // we are in the same configuration as a left event.
            self.resolve_merge_vertices(span_idx, current, id, output);
        } else {
            //      /
            //     x
            //    / :r2
            // ll/   x   <-- current split vertex
            //  left/ \right
            let ll = self.sweep_line[span_idx].left;
            let r2 = Edge {
                upper: ll.upper,
                lower: current,
            };

            self.sweep_line.insert(
                span_idx, Span::begin(ll.upper, ll.upper_id, ll.lower, current)
            );
            let vec2_position = to_vec2(ll.upper);
            self.monotone_tessellators.insert(
                span_idx, MonotoneTessellator::begin(vec2_position, ll.upper_id)
            );
            self.sweep_line[span_idx+1].left.upper = r2.upper;
            self.sweep_line[span_idx+1].left.lower = r2.lower;
            self.sweep_line[span_idx+1].left.merge = false;

            self.insert_edge(
                span_idx, Side::Right,
                Edge { upper: current, lower: left.lower }, id,
            );
            self.insert_edge(
                span_idx+1, Side::Left,
                Edge { upper: current, lower: right.lower }, id,
            );
        }
    }

    fn merge_event<Output: BezierGeometryBuilder<Point>>(&mut self,
        position: TessPoint, id: VertexId,
        span_idx: usize,
        output: &mut Output
    ) {
        debug_assert!(span_idx < self.sweep_line.len()-1);

        let left_span = span_idx;
        let right_span = span_idx+1;

        //     / \ /
        //  \ / .-x    <-- merge vertex
        //   x-'      <-- current merge vertex
        self.resolve_merge_vertices(right_span, position, id, output);

        let vec2_position = to_vec2(position);

        self.sweep_line[left_span].merge_vertex(position, id, Side::Right);
        self.monotone_tessellators[left_span].vertex(vec2_position, id, Side::Right);

        self.sweep_line[right_span].merge_vertex(position, id, Side::Left);
        self.monotone_tessellators[right_span].vertex(vec2_position, id, Side::Left);
    }

    fn insert_edge(&mut self,
        span_idx: usize, side: Side,
        mut edge: Edge, id: VertexId,
    ) {
        debug_assert!(!is_below_fixed(edge.upper, edge.lower));
        // TODO horrible hack: set the merge flag on the edge we are about to replace temporarily
        // so that it doesn not get in the way of the intersection detection.
        self.sweep_line[span_idx].mut_edge(side).merge = true;
        self.check_intersections(&mut edge);
        // This sets the merge flag to false.
        self.sweep_line[span_idx].edge(edge, id, side);
        let vec2_position = to_vec2(edge.upper);
        self.monotone_tessellators[span_idx].vertex(vec2_position, id, side);

    }

    fn check_intersections(&mut self, edge: &mut Edge) {
        struct Intersection { point: TessPoint, lower1: TessPoint, lower2: Option<TessPoint> }

        let original_edge = *edge;
        let mut intersection = None;
        let mut span_idx = 0;

        for span in &mut self.sweep_line {

            // Test for an intersection against the span's left edge.
            if !span.left.merge {
                match segment_intersection_int(
                    edge.upper, edge.lower,
                    span.left.upper, span.left.lower,
                ) {
                    SegmentInteresection::One(position) => {
                        if self.log {
                            println!(" -- found an intersection at {:?}", position);
                            println!("    | {:?}->{:?} x {:?}->{:?}",
                                original_edge.upper.tuple(), original_edge.lower.tuple(),
                                span.left.upper.tuple(), span.left.lower.tuple(),
                            );
                        }

                        intersection = Some((
                            Intersection {
                                point: position,
                                lower1: original_edge.lower,
                                lower2: Some(span.left.lower)
                            },

                            span_idx, Side::Left
                        ));
                        // From now on only consider potential intersections above the one we found,
                        // by removing the lower part from the segment we test against.
                        edge.lower = position;
                    }
                    SegmentInteresection::Two(p1, p2) => {
                        println!(" -- found two intersections {:?} and {:?}", p1.tuple(), p2.tuple());

                        intersection = Some((
                            Intersection {
                                point: p2,
                                lower1: if is_below_fixed(original_edge.lower, span.left.lower)
                                            { original_edge.lower } else { span.left.lower },
                                lower2: None
                            },

                            span_idx, Side::Left
                        ));
                        edge.lower = p2;
                    }
                    _ => {}
                }
            }

            // Same thing for the span's right edge.
            if !span.right.merge {
                match segment_intersection_int(
                    edge.upper, edge.lower,
                    span.right.upper, span.right.lower,
                ) {
                    SegmentInteresection::One(position) => {

                        if self.log {
                            println!(" -- found an intersection at {:?}", position);
                            println!("    | {:?}->{:?} x {:?}->{:?}",
                                original_edge.upper.tuple(), original_edge.lower.tuple(),
                                span.right.upper.tuple(), span.right.lower.tuple(),
                            );
                        }
                        intersection = Some((
                            Intersection {
                                point: position,
                                lower1: original_edge.lower,
                                lower2: Some(span.right.lower)
                            },
                            span_idx, Side::Right
                        ));
                        edge.lower = position;
                    }
                    SegmentInteresection::Two(p1, p2) => {
                        println!(" -- found two intersections {:?} and {:?}", p1.tuple(), p2.tuple());

                        intersection = Some((
                            Intersection {
                                point: p2,
                                lower1: if is_below_fixed(original_edge.lower, span.right.lower)
                                            { original_edge.lower } else { span.right.lower },
                                lower2: None
                            },

                            span_idx, Side::Right
                        ));
                        edge.lower = p2;
                    }
                    _ => {}
                }
            }

            span_idx += 1;
        }

        if let Some((mut evt, span_idx, side)) = intersection {
            let current_position = original_edge.upper;

            // Because precision issues, it can happen that the intersection appear to be
            // "above" the current vertex (in fact it is at the same y but on its left which
            // counts as above). Since we can't come back in time to process the intersection
            // before the current vertex, we can only cheat by moving the interseciton down by
            // one unit.
            if !is_below_fixed(evt.point, current_position) {
                evt.point.y = current_position.y + FixedPoint32::epsilon();
                edge.lower = evt.point;
            }

            let mut e1 = Edge { upper: evt.point, lower: evt.lower1 };
            if is_below_fixed(e1.upper, e1.lower) { swap(&mut e1.upper, &mut e1.lower); }

            let e2 = if let Some(lower2) = evt.lower2 {
                let mut e2 = Edge { upper: evt.point, lower: lower2 };
                // Same deal with the precision issues here. In this case we can just flip the new
                // edge so that its upper member is indeed above the lower one.
                if is_below_fixed(e2.upper, e2.lower) { swap(&mut e2.upper, &mut e2.lower); }
                Some(e2)
            } else { None };

            if self.log {
                println!(" set span[{:?}].{:?}.lower = {:?} (was {:?}",
                    span_idx, side, evt.point.tuple(),
                    self.sweep_line[span_idx].mut_edge(side).lower.tuple()
                );
            }

            self.sweep_line[span_idx].mut_edge(side).lower = evt.point;
            self.intersections.push(e1);
            if let Some(e2) = e2 {
                self.intersections.push(e2);
            }
            // TODO lazily sort intersections next time we read from the vector or
            // do a sorted insertion.
            self.intersections.sort_by(|a, b| { compare_positions(a.upper, b.upper) });
        }
    }

    fn end_span<Output: BezierGeometryBuilder<Point>>(&mut self,
        span_idx: usize, position: TessPoint, id: VertexId, output: &mut Output
    ) {
        let vec2_position = to_vec2(position);
        {
            let tess = &mut self.monotone_tessellators[span_idx];
            tess.end(vec2_position, id);
            tess.flush(output);
        }
        self.sweep_line.remove(span_idx);
        self.monotone_tessellators.remove(span_idx);
    }

    fn error(&mut self, err: FillError) {
        if self.log {
            println!(" !! FillTessellator Error {:?}", err);
        }
        self.error = Some(err);
    }

    fn debug_check_sl(&self, current: TessPoint) {
        for span in &self.sweep_line {
            if !span.left.merge {
                debug_assert!(!is_below_fixed(current, span.left.lower),
                    "current {:?} should not be below lower left{:?}",
                    current, span.left.lower
                );
                debug_assert!(!is_below_fixed(span.left.upper, span.left.lower),
                    "upper left {:?} should not be below lower left {:?}",
                    span.left.upper, span.left.lower
                );
            }
            if !span.right.merge {
                debug_assert!(!is_below_fixed(current, span.right.lower));
                debug_assert!(!is_below_fixed(span.right.upper, span.right.lower));
            }
        }
    }

    fn log_sl_ids(&self) {
        print!("\n|  sl: ");
        for span in &self.sweep_line {
            let ml = if span.left.merge { "*" } else { " " };
            let mr = if span.right.merge { "*" } else { " " };
            print!("| {:?}{}  {:?}{}|  ", span.left.upper_id.offset(), ml, span.right.upper_id.offset(), mr);
        }
        println!("");
    }

    fn log_sl_points(&self) {
        print!("\n sl: [");
        for span in &self.sweep_line {
            print!("| l:{:?} ", span.left.upper.tuple());
            print!(" r:{:?} |", span.right.upper.tuple());
        }
        println!("]");
        print!("     [");
        for span in &self.sweep_line {
            if span.left.merge {
                print!("| l:   <merge>           ");
            } else {
                print!("| l:{:?} ", span.left.lower.tuple());
            }
            if span.right.merge {
                print!(" r:   <merge>           |");
            } else {
                print!(" r:{:?} |", span.right.lower.tuple());
            }
        }
        println!("]\n");
    }

    fn log_sl_points_at(&self, y: FixedPoint32) {
        print!("\nat y={:?}  sl: [", y);
        for span in &self.sweep_line {
            if span.left.merge {
                print!("| l:<merge> ");
            } else {
                let lx = line_horizontal_intersection_fixed(span.left.upper, span.left.lower, y);
                print!("| l:{:?} ", lx);
            }
            if span.right.merge {
                print!(" r:<merge> |");
            } else {
                let rx = line_horizontal_intersection_fixed(span.right.upper, span.right.lower, y);
                print!(" r:{:?} |", rx);
            }
        }
        println!("]\n");
    }
}

fn compare_positions(a: TessPoint, b: TessPoint) -> Ordering {
    if a.y > b.y { return Ordering::Greater; }
    if a.y < b.y { return Ordering::Less; }
    if a.x > b.x { return Ordering::Greater; }
    if a.x < b.x { return Ordering::Less; }
    return Ordering::Equal;
}

pub fn line_horizontal_intersection_fixed(
    a: TessPoint,
    b: TessPoint,
    y: FixedPoint32
) -> Option<FixedPoint32> {
    let vx = b.x - a.x;
    let vy = b.y - a.y;

    if vy.is_zero() {
        // the line is horizontal
        return None;
    }

    return Some(a.x + (y - a.y).to_fp64().mul_div(vx.to_fp64(), vy.to_fp64()).to_fp32());
}

enum SegmentInteresection {
    One(TessPoint),
    Two(TessPoint, TessPoint),
    None,
}

fn segment_intersection_int(
    a1: TessPoint, b1: TessPoint, // The new edge.
    a2: TessPoint, b2: TessPoint  // An already inserted edge.
) -> SegmentInteresection {

    fn tess_point(x: i64, y: i64) -> TessPoint {
        TessPoint::new(FixedPoint32::from_raw(x as i32), FixedPoint32::from_raw(y as i32))
    }

    //println!(" -- test intersection {:?} {:?} x {:?} {:?}", a1, b1, a2, b2);

    // Locally work with 64 bit integers to avoid overflow. We don't need to apply
    // the fixed point's division because the equation preserves the unit, letting
    // us work directly with the raw integer representation and avoid precision loss
    // when performing divisions.
    let a1 = Int64Point::new(a1.x.raw() as i64, a1.y.raw() as i64);
    let b1 = Int64Point::new(b1.x.raw() as i64, b1.y.raw() as i64);
    let a2 = Int64Point::new(a2.x.raw() as i64, a2.y.raw() as i64);
    let b2 = Int64Point::new(b2.x.raw() as i64, b2.y.raw() as i64);

    let v1 = b1 - a1;
    let v2 = b2 - a2;

    debug_assert!(v2.x != 0 || v2.y != 0, "zero-length edge");

    let v1_cross_v2 = v1.cross(v2);
    let a2_a1_cross_v1 = (a2 - a1).cross(v1);

    //println!(" -- v1_cross_v2 {}, a2_a1_cross_v1 {}", v1_cross_v2, a2_a1_cross_v1);

    if v1_cross_v2 == 0 {
        if a2_a1_cross_v1 != 0 {
            return SegmentInteresection::None;
        }
        // The two segments are colinear.
        //println!(" -- colinear segments");

        let v1_sqr_len = v1.x * v1.x + v1.y * v1.y;
        let v2_sqr_len = v2.x * v2.x + v2.y * v2.y;

        // We know that a1 cannot be above a2 so if b1 is between a2 and b2, we have
        // the order a2 -> a1 -> b1 -> b2.
        let v2_dot_b1a2 = v2.dot(b1 - a2);
        if v2_dot_b1a2 > 0 && v2_dot_b1a2 < v2_sqr_len {
            //println!(" -- colinear intersection");
            return SegmentInteresection::Two(
                tess_point(a1.x, a1.y),
                tess_point(b1.x, b1.y),
            );
        }

        // We know that a1 cannot be above a2 and if b1 is below b2, so if
        // b2 is between a1 and b1, then we have the order a2 -> a1 -> b2 -> b1.
        let v1_dot_b2a1 = v1.dot(b2 - a1);
        if v1_dot_b2a1 > 0 && v1_dot_b2a1 < v1_sqr_len {
            //println!(" -- colinear intersection");
            return SegmentInteresection::Two(
                tess_point(a1.x, a1.y),
                tess_point(b2.x, b2.y),
            );
        }

        return SegmentInteresection::None;
    }

    if a1 == b2 || a1 == a2 || b1 == a2 || b1 == b2 {
        //println!(" -- segments touch");
        return SegmentInteresection::None;
    }

    let sign_v1_cross_v2 = if v1_cross_v2 > 0 { 1 } else { -1 };
    let abs_v1_cross_v2 = v1_cross_v2 * sign_v1_cross_v2;

    // t and u should be divided by v1_cross_v2, but we postpone that to not lose precision.
    // We have to respect the sign of v1_cross_v2 (and therefore t and u) so we apply it now and
    // will use the absolute value of v1_cross_v2 afterwards.
    let t = (a2 - a1).cross(v2) * sign_v1_cross_v2;
    let u = a2_a1_cross_v1 * sign_v1_cross_v2;

    if t > 0 && t < abs_v1_cross_v2 && u > 0 && u <= abs_v1_cross_v2 {

        let res = a1 + (v1 * t) / abs_v1_cross_v2;

        debug_assert!(res.y <= b1.y && res.y <= b2.y);

        //debug_assert!(res != a1);

        if res != a1 && res != b1 && res != a2 && res != b2 {
        //if res != a1 && res != a2 {
        let d = (v1 * t) / abs_v1_cross_v2 - v1;
        println!(" -- intersection: t = {} u= {} v1xv2 = {}, d = {:?}", t, u, v1_cross_v2, d);
        return SegmentInteresection::One(tess_point(res.x, res.y));
        }
    }

    //if t > 0 && t - abs_v1_cross_v2 < 100 {
    //    println!(" -- almost intersecting {} {} : {}", t, abs_v1_cross_v2, t - abs_v1_cross_v2 );
    //}

    return SegmentInteresection::None;
}

fn test_span_side(span_edge: &SpanEdge, position: TessPoint) -> bool {
    if span_edge.merge {
        return false;
    }

    if span_edge.lower == position {
        return true;
    }

    let from = span_edge.upper;
    let to = span_edge.lower;

    let vx = (to.x - from.x).raw() as i64;
    let vy = (to.y - from.y).raw() as i64;
    if vy == 0 {
        // If the segment is horizontal, pick the biggest x value (the right-most point).
        // That's arbitrary, not sure it is the right thing to do.
        return cmp::max(position.x.raw(), to.x.raw()) > position.x.raw();
    }
    // shuffled around from:
    // edge_from.x + (point.y - edge_from.y) * vx / vy > point.x
    // in order to remove the division.
    return (position.y - from.y).raw() as i64 * vx > (position.x - from.x).raw() as i64 * vy;
}

fn test_span_touches(span_edge: &SpanEdge, position: TessPoint) -> bool {
    if let Some(x) = line_horizontal_intersection_fixed(span_edge.upper, span_edge.lower, position.y) {
        return (x - position.x).abs() <= FixedPoint32::epsilon() * 2;
    }
    debug_assert!(span_edge.upper.y == span_edge.lower.y);
    return span_edge.upper.y == position.y
        && span_edge.upper.x < position.x
        && span_edge.lower.x > position.x;
}

struct Span {
    left: SpanEdge,
    right: SpanEdge,
}

#[derive(Copy, Clone, Debug)]
struct SpanEdge {
    upper: TessPoint,
    lower: TessPoint,
    upper_id: VertexId,
    merge: bool,
}

impl Span {
    fn begin(current: TessPoint, id: VertexId, left: TessPoint, right: TessPoint) -> Span {
        Span {
            left: SpanEdge {
                upper: current,
                lower: left,
                upper_id: id,
                merge: false,
            },
            right: SpanEdge {
                upper: current,
                lower: right,
                upper_id: id,
                merge: false,
            },
        }
    }

    fn edge(&mut self,
        edge: Edge,
        id: VertexId,
        side: Side
    ) {
        self.set_upper_vertex(edge.upper, id, side);
        self.set_lower_vertex(edge.lower, side);
    }

    fn merge_vertex(&mut self, vertex: TessPoint, id: VertexId, side: Side) {
        self.set_upper_vertex(vertex, id, side);
        self.mut_edge(side).merge = true;
    }

    fn set_upper_vertex(&mut self, vertex: TessPoint, id: VertexId, side: Side) {
        self.mut_edge(side).upper = vertex;
        self.mut_edge(side).upper_id = id;
    }

    fn set_lower_vertex(&mut self, vertex: TessPoint, side: Side) {
        let mut edge = self.mut_edge(side);
        edge.lower = vertex;
        edge.merge = false;
    }

    fn mut_edge(&mut self, side: Side) -> &mut SpanEdge {
        return match side {
            Side::Left => { &mut self.left }
            Side::Right => { &mut self.right }
        };
    }
}

// translate to and from the internal coordinate system.
fn to_internal(v: Point) -> TessPoint { TessPoint::new(fixed(v.x), fixed(v.y)) }
fn to_vec2(v: TessPoint) -> Point { vec2(v.x.to_f32(), v.y.to_f32()) }

pub struct FillEvents {
    edges: Vec<Edge>,
    vertices: Vec<TessPoint>,
}

impl FillEvents {
    pub fn from_iter<Iter: Iterator<Item=FlattenedEvent>>(it: Iter) -> Self {
        EventsBuilder::new().build(it)
    }
}

struct EventsBuilder {
    edges: Vec<Edge>,
    vertices: Vec<TessPoint>,
}

impl EventsBuilder {

    fn new() -> Self {
        EventsBuilder {
            edges: Vec::new(),
            vertices: Vec::new(),
        }
    }

    fn build<Iter: Iterator<Item=FlattenedEvent>>(mut self, inputs: Iter) -> FillEvents {
        let mut first = TessPoint::new(fixed(0.0), fixed(0.0));
        let mut second = TessPoint::new(fixed(0.0), fixed(0.0));
        let mut previous = TessPoint::new(fixed(0.0), fixed(0.0));
        let mut current = TessPoint::new(fixed(0.0), fixed(0.0));
        let mut nth = 0;
        for evt in inputs {
            match evt {
                FlattenedEvent::LineTo(next) => {
                    let next = to_internal(next);
                    if next == current {
                        continue;
                    }
                    if nth == 0 {
                        second = next;
                    }
                    self.add_edge(current, next);
                    if nth > 0 {
                        self.vertex(previous, current, next);
                    }
                    previous = current;
                    current = next;
                    nth += 1;
                }
                FlattenedEvent::Close => {
                    if current != first {
                        if nth > 0 {
                            self.add_edge(current, first);
                            self.vertex(previous, current, first);
                        }
                        if nth > 1 {
                            self.vertex(current, first, second);
                        }
                    } else {
                        if nth > 1 {
                            self.vertex(previous, first, second);
                        }
                    }
                    nth = 0;
                    current = first;
                }
                FlattenedEvent::MoveTo(next) => {
                    let next = to_internal(next);
                    if nth > 1 {
                        self.add_edge(current, first);
                        self.vertex(previous, current, first);
                        self.vertex(current, first, second);
                    }
                    first = next;
                    current = next;
                    nth = 0;
                }
            }
        }

        self.edges.sort_by(|a, b|{ compare_positions(a.upper, b.upper) });
        self.vertices.sort_by(|a, b|{ compare_positions(*a, *b) });

        return FillEvents {
            edges: self.edges,
            vertices: self.vertices,
        };
    }

    fn add_edge(&mut self, mut a: TessPoint, mut b: TessPoint) {
        if a == b {
            return;
        }

        if is_below_fixed(a, b) {
            swap(&mut a, &mut b);
        }

        //println!(" -- add edge evt {:?}, {:?}", a, b);

        self.edges.push(Edge { upper: a, lower: b });
    }

    fn vertex(&mut self, previous: TessPoint, current: TessPoint, next: TessPoint) {
        if is_below_fixed(current, previous) && is_below_fixed(current, next) {
            //println!(" -- add vertex evt {:?}", current);
            self.vertices.push(current);
        }
    }
}

#[test]
fn test_iter_builder() {

    let mut builder = Path::builder();
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(0.0, 1.0));

    builder.move_to(point(10.0, 0.0));
    builder.line_to(point(11.0, 0.0));
    builder.line_to(point(11.0, 1.0));
    builder.line_to(point(10.0, 1.0));
    builder.close();

    let path = builder.build();

    let events = EventsBuilder::new().build(path.path_iter().flattened(0.05));
    let mut buffers: VertexBuffers<Point> = VertexBuffers::new();
    let mut vertex_builder = simple_builder(&mut buffers);
    let mut tess = FillTessellator::new();
    tess.enable_logging();
    tess.tessellate_events(&events, &FillOptions::default(), &mut vertex_builder).unwrap();
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FillRule {
    EvenOdd,
    NonZero,
}

/// Parameters for the tessellator.
pub struct FillOptions {
    /// Maximum allowed distance to the path when building an approximation.
    pub tolerance: f32,

    /// See the SVG specification.
    ///
    /// Currently, only the EvenOdd rule is implemented.
    pub fill_rule: FillRule,

    /// An anti-aliasing trick extruding a 1-px wide strip around the edges with
    /// a gradient to smooth the edges.
    ///
    /// Not implemented yet!
    pub vertex_aa: bool,

    // To be able to add fields without making it a breaking change, add an empty private field
    // which makes it impossible to create a FillOptions without the calling constructor.
    _private: (),
}

impl FillOptions {
    pub fn default() -> FillOptions { FillOptions::even_odd() }

    pub fn even_odd() -> FillOptions {
        FillOptions {
            tolerance: 0.1,
            fill_rule: FillRule::EvenOdd,
            vertex_aa: false,
            _private: (),
        }
    }

    pub fn non_zero() -> FillOptions {
        let mut options = FillOptions::default();
        options.fill_rule = FillRule::NonZero;
        return options;
    }

    pub fn with_tolerance(mut self, tolerance: f32) -> FillOptions {
        self.tolerance = tolerance;
        return self;
    }

    pub fn with_vertex_aa(mut self) -> FillOptions {
        self.vertex_aa = true;
        return self;
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Side { Left, Right }

impl Side {
    pub fn opposite(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    pub fn is_left(self) -> bool { self == Side::Left }

    pub fn is_right(self) -> bool { self == Side::Right }
}

/// Helper class that generates a triangulation from a sequence of vertices describing a monotone
/// polygon (used internally by the FillTessellator).
pub struct MonotoneTessellator {
    stack: Vec<MonotoneVertex>,
    previous: MonotoneVertex,
    triangles: Vec<(VertexId, VertexId, VertexId)>,
}

#[derive(Copy, Clone, Debug)]
struct MonotoneVertex {
    pos: Point,
    id: VertexId,
    side: Side,
}

impl MonotoneTessellator {
    pub fn begin(pos: Point, id: VertexId) -> MonotoneTessellator {
        let first = MonotoneVertex { pos: pos, id: id, side: Side::Left };

        let mut tess = MonotoneTessellator {
            stack: Vec::with_capacity(16),
            triangles: Vec::with_capacity(128),
            previous: first,
        };

        tess.stack.push(first);

        return tess;
    }

    pub fn vertex(&mut self, pos: Point, id: VertexId, side: Side) {
        let current = MonotoneVertex{ pos: pos, id: id, side: side };
        let right_side = current.side == Side::Right;

        debug_assert!(is_below(current.pos, self.previous.pos));
        debug_assert!(!self.stack.is_empty());

        let changed_side = current.side != self.previous.side;

        if changed_side {
            for i in 0..(self.stack.len() - 1) {
                let mut a = self.stack[i];
                let mut b = self.stack[i+1];

                if right_side {
                    swap(&mut a, &mut b);
                }

                self.push_triangle(&a, &b, &current);
            }
            self.stack.clear();
            self.stack.push(self.previous);
        } else {
            let mut last_popped = self.stack.pop();
            while !self.stack.is_empty() {
                let mut a = last_popped.unwrap();
                let mut b = *self.stack.last().unwrap();

                if right_side {
                    swap(&mut a, &mut b);
                }

                if directed_angle2(b.pos, current.pos, a.pos) <= PI {
                    self.push_triangle(&a, &b, &current);
                    last_popped = self.stack.pop();
                } else {
                    break;
                }
            }
            if let Some(item) = last_popped {
                self.stack.push(item);
            }
        }

        self.stack.push(current);
        self.previous = current;

        return;
    }

    pub fn end(&mut self, pos: Point, id: VertexId) {
        let side = self.previous.side.opposite();
        self.vertex(pos, id, side);
        self.stack.clear();
    }

    fn push_triangle(&mut self, a: &MonotoneVertex, b: &MonotoneVertex, c: &MonotoneVertex) {
        //println!(" #### triangle {} {} {}", a.id.offset(), b.id.offset(), c.id.offset());

        if directed_angle2(b.pos, c.pos, a.pos) <= PI {
            self.triangles.push((a.id, b.id, c.id));
        } else {
            self.triangles.push((b.id, a.id, c.id));
        }
    }

    fn flush<Output: BezierGeometryBuilder<Point>>(&mut self, output: &mut Output) {
        for &(a, b, c) in &self.triangles {
            output.add_triangle(a, b, c);
        }
        self.triangles.clear();
    }
}

#[test]
fn test_monotone_tess() {
    println!(" ------------ ");
    {
        let mut tess = MonotoneTessellator::begin(vec2(0.0, 0.0), VertexId(0));
        tess.vertex(vec2(-1.0, 1.0), VertexId(1), Side::Left);
        tess.end(vec2(1.0, 2.0), VertexId(2));
        assert_eq!(tess.triangles.len(), 1);
    }
    println!(" ------------ ");
    {
        let mut tess = MonotoneTessellator::begin(vec2(0.0, 0.0), VertexId(0));
        tess.vertex(vec2(1.0, 1.0), VertexId(1), Side::Right);
        tess.vertex(vec2(-1.5, 2.0), VertexId(2), Side::Left);
        tess.vertex(vec2(-1.0, 3.0), VertexId(3), Side::Left);
        tess.vertex(vec2(1.0, 4.0), VertexId(4), Side::Right);
        tess.end(vec2(0.0, 5.0), VertexId(5));
        assert_eq!(tess.triangles.len(), 4);
    }
    println!(" ------------ ");
    {
        let mut tess = MonotoneTessellator::begin(vec2(0.0, 0.0), VertexId(0));
        tess.vertex(vec2(1.0, 1.0), VertexId(1), Side::Right);
        tess.vertex(vec2(3.0, 2.0), VertexId(2), Side::Right);
        tess.vertex(vec2(1.0, 3.0), VertexId(3), Side::Right);
        tess.vertex(vec2(1.0, 4.0), VertexId(4), Side::Right);
        tess.vertex(vec2(4.0, 5.0), VertexId(5), Side::Right);
        tess.end(vec2(0.0, 6.0), VertexId(6));
        assert_eq!(tess.triangles.len(), 5);
    }
    println!(" ------------ ");
    {
        let mut tess = MonotoneTessellator::begin(vec2(0.0, 0.0), VertexId(0));
        tess.vertex(vec2(-1.0, 1.0), VertexId(1), Side::Left);
        tess.vertex(vec2(-3.0, 2.0), VertexId(2), Side::Left);
        tess.vertex(vec2(-1.0, 3.0), VertexId(3), Side::Left);
        tess.vertex(vec2(-1.0, 4.0), VertexId(4), Side::Left);
        tess.vertex(vec2(-4.0, 5.0), VertexId(5), Side::Left);
        tess.end(vec2(0.0, 6.0), VertexId(6));
        assert_eq!(tess.triangles.len(), 5);
    }
    println!(" ------------ ");
}

#[cfg(test)]
fn tessellate(path: PathSlice, log: bool) -> Result<usize, FillError> {
    let mut buffers: VertexBuffers<Point> = VertexBuffers::new();
    {
        let mut vertex_builder = simple_builder(&mut buffers);
        let mut tess = FillTessellator::new();
        if log {
            tess.enable_logging();
        }
        let events = FillEvents::from_iter(path.path_iter().flattened(0.05));
        try!{
            tess.tessellate_events(&events, &FillOptions::default(), &mut vertex_builder)
        };
    }
    return Ok(buffers.indices.len()/3);
}

#[cfg(test)]
fn test_path(path: PathSlice, expected_triangle_count: Option<usize>) {
    let res = ::std::panic::catch_unwind(|| { tessellate(path, false) });

    if let Ok(Ok(num_triangles)) = res {
        if let Some(expected_triangles) = expected_triangle_count {
            if num_triangles != expected_triangles {
                tessellate(path, true).unwrap();
                panic!("expected {} triangles, got {}", expected_triangles, num_triangles);
            }
        }
        return;
    }

    ::extra::debugging::find_reduced_test_case(path, &|path: Path|{
        return tessellate(path.as_slice(), false).is_err();
    });

    tessellate(path, true).unwrap();
    panic!();
}

#[cfg(test)]
fn test_path_with_rotations(path: Path, step: f32, expected_triangle_count: Option<usize>) {
    let mut angle = 0.0;

    while angle < PI * 2.0 {
        println!("\n\n ==================== angle = {}", angle);

        let mut tranformed_path = path.clone();
        let cos = angle.cos();
        let sin = angle.sin();
        for v in tranformed_path.mut_points() {
            let (x, y) = (v.x, v.y);
            v.x = x*cos + y*sin;
            v.y = y*cos - x*sin;
        }

        test_path(tranformed_path.as_slice(), expected_triangle_count);

        angle += step;
    }
}

#[test]
fn test_simple_triangle() {
    let mut path = Path::builder();
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(1.0, 1.0));
    path.line_to(vec2(0.0, 1.0));
    path.close();

    test_path_with_rotations(path.build(), 0.01, Some(1));
}

#[test]
fn test_simple_monotone() {
    let mut path = Path::builder();
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(-1.0, 1.0));
    path.line_to(vec2(-3.0, 2.0));
    path.line_to(vec2(-1.0, 3.0));
    path.line_to(vec2(-4.0, 5.0));
    path.line_to(vec2( 0.0, 6.0));
    path.close();

    let path = path.build();
    test_path(path.as_slice(), Some(4));
}

#[test]
fn test_simple_split() {
    let mut path = Path::builder();
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(2.0, 1.0));
    path.line_to(vec2(2.0, 3.0));
    path.line_to(vec2(1.0, 2.0));
    path.line_to(vec2(0.0, 3.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, Some(3));
}

#[test]
fn test_simple_merge_split() {
    let mut path = Path::builder();
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(1.0, 1.0));
    path.line_to(vec2(2.0, 0.0));
    path.line_to(vec2(2.0, 3.0));
    path.line_to(vec2(1.0, 2.0));
    path.line_to(vec2(0.0, 3.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, Some(4));
}

#[test]
fn test_simple_aligned() {
    let mut path = Path::builder();
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(1.0, 0.0));
    path.line_to(vec2(2.0, 0.0));
    path.line_to(vec2(2.0, 1.0));
    path.line_to(vec2(2.0, 2.0));
    path.line_to(vec2(1.0, 2.0));
    path.line_to(vec2(0.0, 2.0));
    path.line_to(vec2(0.0, 1.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, Some(6));
}

#[test]
fn test_simple_1() {
    let mut path = Path::builder();
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(1.0, 1.0));
    path.line_to(vec2(2.0, 0.0));
    path.line_to(vec2(1.0, 3.0));
    path.line_to(vec2(0.5, 4.0));
    path.line_to(vec2(0.0, 3.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, Some(4));
}

#[test]
fn test_simple_2() {
    let mut path = Path::builder();
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(1.0, 0.0));
    path.line_to(vec2(2.0, 0.0));
    path.line_to(vec2(3.0, 0.0));
    path.line_to(vec2(3.0, 1.0));
    path.line_to(vec2(3.0, 2.0));
    path.line_to(vec2(3.0, 3.0));
    path.line_to(vec2(2.0, 3.0));
    path.line_to(vec2(1.0, 3.0));
    path.line_to(vec2(0.0, 3.0));
    path.line_to(vec2(0.0, 2.0));
    path.line_to(vec2(0.0, 1.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, Some(10));
}

#[test]
fn test_hole_1() {
    let mut path = Path::builder();
    path.move_to(vec2(-11.0, 5.0));
    path.line_to(vec2(0.0, -5.0));
    path.line_to(vec2(10.0, 5.0));
    path.close();

    path.move_to(vec2(-5.0, 2.0));
    path.line_to(vec2(0.0, -2.0));
    path.line_to(vec2(4.0, 2.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, Some(6));
}

#[test]
fn test_degenerate_empty() {
    test_path(Path::new().as_slice(), Some(0));
}

#[test]
fn test_degenerate_same_position() {
    let mut path = Path::builder();
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(0.0, 0.0));
    path.line_to(vec2(0.0, 0.0));
    path.line_to(vec2(0.0, 0.0));
    path.line_to(vec2(0.0, 0.0));
    path.line_to(vec2(0.0, 0.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, None);
}

#[test]
fn test_auto_intersection_type1() {
    //  o.___
    //   \   'o
    //    \ /
    //     x  <-- intersection!
    //    / \
    //  o.___\
    //       'o
    let mut path = Path::builder();
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(2.0, 1.0));
    path.line_to(vec2(0.0, 2.0));
    path.line_to(vec2(2.0, 3.0));
    path.close();

    let path = path.build();
    test_path(path.as_slice(), Some(2));
}

#[test]
fn test_auto_intersection_type2() {
    //  o
    //  |\   ,o
    //  | \ / |
    //  |  x  | <-- intersection!
    //  | / \ |
    //  o'   \|
    //        o
    let mut path = Path::builder();
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(2.0, 3.0));
    path.line_to(vec2(2.0, 1.0));
    path.line_to(vec2(0.0, 2.0));
    path.close();

    let path = path.build();
    test_path(path.as_slice(), Some(2));
}

#[test]
fn test_auto_intersection_multi() {
    //      .
    //  ___/_\___
    //  | /   \ |
    //  |/     \|
    // /|       |\
    // \|       |/
    //  |\     /|
    //  |_\___/_|
    //     \ /
    //      '
    let mut path = Path::builder();
    path.move_to(vec2(20.0, 20.0));
    path.line_to(vec2(60.0, 20.0));
    path.line_to(vec2(60.0, 60.0));
    path.line_to(vec2(20.0, 60.0));
    path.close();

    path.move_to(vec2(40.0, 10.0));
    path.line_to(vec2(70.0, 40.0));
    path.line_to(vec2(40.0, 70.0));
    path.line_to(vec2(10.0, 40.0));
    path.close();

    let path = path.build();
    test_path(path.as_slice(), Some(8));
}

#[test]
fn test_rust_logo() {
    let mut path = Path::builder().with_svg();

    build_logo_path(&mut path);

    test_path_with_rotations(path.build(), 0.011, None);
}

#[test]
fn test_rust_logo_with_intersection() {
    let mut path = Path::builder().with_svg();

    build_logo_path(&mut path);

    path.move_to(vec2(10.0, 30.0));
    path.line_to(vec2(130.0, 30.0));
    path.line_to(vec2(130.0, 60.0));
    path.line_to(vec2(10.0, 60.0));
    path.close();

    let path = path.build();

    test_path_with_rotations(path, 0.011, None);
}

#[cfg(test)]
fn scale_path(path: &mut Path, scale: f32) {
    for v in path.mut_points() {
        *v = *v * scale;
    }
}

#[test]
fn test_rust_logo_scale_up() {
    // The goal of this test is to check how resistent the tessellator is against integer
    // overflows, and catch regressions.

    let mut builder = Path::builder().with_svg();
    build_logo_path(&mut builder);
    let mut path = builder.build();

    scale_path(&mut path, 10000.0);
    test_path(path.as_slice(), None);
}

#[test]
#[ignore] // TODO
fn test_rust_logo_scale_up_failing() {
    // This test triggers integers overflow in the tessellator.
    // In order to fix this type issue we need to:
    // * Look at the computation that is casuing trouble and see if it can be expressed in
    //   a way that is less subject to overflows.
    // * See if we can define a safe interval where no path can trigger overflows and scale
    //   all paths to this interval internally in the tessellator.
    let mut builder = Path::builder().with_svg();
    build_logo_path(&mut builder);
    let mut path = builder.build();

    scale_path(&mut path, 100000.0);
    test_path(path.as_slice(), None);
}

#[test]
fn test_rust_logo_scale_down() {
    // The goal of this test is to check that the tessellator can handle very small geometry.

    let mut builder = Path::builder().with_svg();
    build_logo_path(&mut builder);
    let mut path = builder.build();

    scale_path(&mut path, 0.02);
    test_path(path.as_slice(), None);
}

#[test]
#[ignore] // TODO
fn test_rust_logo_scale_down_failing() {
    // Issues with very small paths.

    let mut builder = Path::builder().with_svg();
    build_logo_path(&mut builder);
    let mut path = builder.build();

    scale_path(&mut path, 0.0001);
    test_path(path.as_slice(), None);
}

#[test]
fn test_double_merge() {
    // This test triggers the code path where a merge event is resolved during another
    // merge event.
    //     / \ /
    //  \ / .-x    <-- merge vertex
    //   x-'      <-- current merge vertex
    //
    // The test case generated from a reduced rotation of
    // test_rust_logo_with_intersection
    let mut path = Path::builder();

    path.move_to(vec2(80.041534, 19.24472));
    path.line_to(vec2(76.56131, 23.062233));
    path.line_to(vec2(67.26949, 23.039438));
    path.line_to(vec2(65.989944, 23.178522));
    path.line_to(vec2(59.90927, 19.969215));
    path.line_to(vec2(56.916714, 25.207449));
    path.line_to(vec2(50.333813, 23.25274));
    path.line_to(vec2(48.42367, 28.978098));
    path.close();
    path.move_to(vec2(130.32213, 28.568213));
    path.line_to(vec2(130.65213, 58.5664));
    path.line_to(vec2(10.659382, 59.88637));
    path.close();

    test_path(path.build().as_slice(), None);
}

#[test]
fn test_chained_merge_end() {
    // This test creates a succession of merge events that need to be resolved during
    // an end event.
    // |\/\  /\    /|  <-- merge
    // \   \/  \  / /  <-- merge
    //  \       \/ /   <-- merge
    //   \        /
    //    \      /
    //     \    /
    //      \  /
    //       \/        < -- end
    let mut path = Path::builder();

    path.move_to(vec2(1.0, 0.0));
    path.line_to(vec2(2.0, 1.0)); // <-- merge
    path.line_to(vec2(3.0, 0.0));
    path.line_to(vec2(4.0, 2.0)); // <-- merge
    path.line_to(vec2(5.0, 0.0));
    path.line_to(vec2(6.0, 3.0)); // <-- merge
    path.line_to(vec2(7.0, 0.0));
    path.line_to(vec2(5.0, 8.0)); // <-- end
    path.close();

    test_path(path.build().as_slice(), Some(6));
}

#[test]
fn test_chained_merge_left() {
    // This test creates a succession of merge events that need to be resolved during
    // a left event.
    // |\/\  /\    /|  <-- merge
    // |   \/  \  / |  <-- merge
    // |        \/  |  <-- merge
    // |            |
    //  \           |  <-- left
    //   \          |
    let mut path = Path::builder();

    path.move_to(vec2(1.0, 0.0));
    path.line_to(vec2(2.0, 1.0)); // <-- merge
    path.line_to(vec2(3.0, 0.0));
    path.line_to(vec2(4.0, 2.0)); // <-- merge
    path.line_to(vec2(5.0, 0.0));
    path.line_to(vec2(6.0, 3.0)); // <-- merge
    path.line_to(vec2(7.0, 0.0));
    path.line_to(vec2(7.0, 5.0));
    path.line_to(vec2(0.0, 4.0)); // <-- left
    path.close();

    test_path(path.build().as_slice(), Some(7));
}

#[test]
fn test_chained_merge_merge() {
    // This test creates a succession of merge events that need to be resolved during
    // another merge event.
    //      /\/\  /\    /|  <-- merge
    //     /    \/  \  / |  <-- merge
    //    /          \/  |  <-- merge
    // |\/               |  <-- merge (resolving)
    // |_________________|
    let mut path = Path::builder();

    path.move_to(vec2(1.0, 0.0));
    path.line_to(vec2(2.0, 1.0)); // <-- merge
    path.line_to(vec2(3.0, 0.0));
    path.line_to(vec2(4.0, 2.0)); // <-- merge
    path.line_to(vec2(5.0, 0.0));
    path.line_to(vec2(6.0, 3.0)); // <-- merge
    path.line_to(vec2(7.0, 0.0));
    path.line_to(vec2(7.0, 5.0));
    path.line_to(vec2(-1.0, 5.0));
    path.line_to(vec2(-1.0, 0.0));
    path.line_to(vec2(0.0, 4.0)); // <-- merge (resolving)
    path.close();

    test_path(path.build().as_slice(), Some(9));
}

#[test]
fn test_chained_merge_split() {
    // This test creates a succession of merge events that need to be resolved during
    // a split event.
    // |\/\  /\    /|  <-- merge
    // |   \/  \  / |  <-- merge
    // |        \/  |  <-- merge
    // |            |
    // |     /\     |  <-- split
    let mut path = Path::builder();

    path.move_to(vec2(1.0, 0.0));
    path.line_to(vec2(2.0, 1.0)); // <-- merge
    path.line_to(vec2(3.0, 0.0));
    path.line_to(vec2(4.0, 2.0)); // <-- merge
    path.line_to(vec2(5.0, 0.0));
    path.line_to(vec2(6.0, 3.0)); // <-- merge
    path.line_to(vec2(7.0, 0.0));
    path.line_to(vec2(7.0, 5.0));
    path.line_to(vec2(4.0, 4.0)); // <-- split
    path.line_to(vec2(1.0, 5.0));
    path.close();

    test_path(path.build().as_slice(), Some(8));
}

// TODO: Check that chained merge events can't mess with the way we handle complex events.

#[test]
fn test_intersection_horizontal_precision() {
    // TODO make a cleaner test case exercising the same edge case.
    // This test has an almost horizontal segment e1 going from right to left intersected
    // by another segment e2. Since e1 is almost horizontal the intersection point ends up
    // with the same y coordinate and at the left of the current position when it is found.
    // The difficulty is that the intersection is therefore technically "above" the current
    // position, but we can't allow that because the ordering of the events is a strong
    // invariant of the algorithm.
    let mut builder = Path::builder();

    builder.move_to(vec2(-34.619564, 111.88655));
    builder.line_to(vec2(-35.656174, 111.891));
    builder.line_to(vec2(-39.304527, 121.766914));
    builder.close();

    builder.move_to(vec2(1.4426613, 133.40884));
    builder.line_to(vec2(-27.714422, 140.47032));
    builder.line_to(vec2(-55.960342, 23.841988));
    builder.close();

    test_path(builder.build().as_slice(), None);
}

#[test]
fn test_split_with_intersections() {
    // This is a reduced test case that was showing a bug where duplicate intersections
    // were found during a split event, due to the sweep line beeing into a temporarily
    // inconsistent state when insert_edge was called.

    let mut builder = Path::builder();

    builder.move_to(vec2(-21.004179, -71.57515));
    builder.line_to(vec2(-21.927473, -70.94977));
    builder.line_to(vec2(-23.024633, -70.68942));
    builder.close();
    builder.move_to(vec2(16.036617, -27.254852));
    builder.line_to(vec2(-62.83691, -117.69249));
    builder.line_to(vec2(38.646027, -46.973236));
    builder.close();

    let path = builder.build();

    test_path(path.as_slice(), None);
}

#[test]
fn test_colinear_1() {
    let mut builder = Path::builder();
    builder.move_to(vec2(20.0, 150.0));
    builder.line_to(vec2(80.0, 150.0));
    builder.close();

    let path = builder.build();

    test_path_with_rotations(path, 0.01, None);
}

#[test]
fn test_colinear_2() {
    let mut builder = Path::builder();
    builder.move_to(vec2(20.0, 150.0));
    builder.line_to(vec2(80.0, 150.0));
    builder.line_to(vec2(20.0, 150.0));
    builder.close();

    let path = builder.build();

    test_path_with_rotations(path, 0.01, None);
}

#[test]
fn test_colinear_3() {
    let mut builder = Path::builder();
    // The path goes through many points along a line.
    builder.move_to(vec2(0.0, 1.0));
    builder.line_to(vec2(0.0, 3.0));
    builder.line_to(vec2(0.0, 5.0));
    builder.line_to(vec2(0.0, 4.0));
    builder.line_to(vec2(0.0, 2.0));
    builder.close();

    let path = builder.build();

    tessellate(path.as_slice(), true).unwrap();
}

#[test]
fn test_colinear_4() {
    // The path goes back and forth along a line.
    let mut builder = Path::builder();
    builder.move_to(vec2(0.0, 2.0));
    builder.line_to(vec2(0.0, 1.0));
    builder.line_to(vec2(0.0, 3.0));
    builder.line_to(vec2(0.0, 0.0));
    builder.close();

    let path = builder.build();

    tessellate(path.as_slice(), true).unwrap();
}

#[test]
fn test_colinear_touching_squares() {
    // Two squares touching.
    //
    // x-----x-----x
    // |     |     |
    // |     |     |
    // x-----x-----x
    //
    let mut builder = Path::builder();
    builder.move_to(vec2(0.0, 0.0));
    builder.line_to(vec2(1.0, 0.0));
    builder.line_to(vec2(1.0, 1.0));
    builder.line_to(vec2(0.0, 1.0));

    builder.move_to(vec2(1.0, 0.0));
    builder.line_to(vec2(2.0, 0.0));
    builder.line_to(vec2(2.0, 1.0));
    builder.line_to(vec2(1.0, 1.0));

    builder.close();

    let path = builder.build();

    tessellate(path.as_slice(), true).unwrap();
}

#[test]
fn test_colinear_touching_squares2() {
    // Two squares touching.
    //
    // x-----x
    // |     x-----x
    // |     |     |
    // x-----x     |
    //       x-----x
    //
    let mut builder = Path::builder();
    builder.move_to(vec2(0.0,  0.0));
    builder.line_to(vec2(10.0, 0.0));
    builder.line_to(vec2(10.0, 10.0));
    builder.line_to(vec2(0.0,  10.0));

    builder.move_to(vec2(10.0, 1.0));
    builder.line_to(vec2(20.0, 1.0));
    builder.line_to(vec2(20.0, 11.0));
    builder.line_to(vec2(10.0, 11.0));

    builder.close();

    let path = builder.build();

    tessellate(path.as_slice(), true).unwrap();
}

#[test]
fn test_colinear_touching_squares3() {
    // Two squares touching.
    //
    //       x-----x
    // x-----x     |
    // |     |     |
    // |     x-----x
    // x-----x
    //
    let mut builder = Path::builder();
    builder.move_to(vec2(0.0,  1.0));
    builder.line_to(vec2(10.0, 1.0));
    builder.line_to(vec2(10.0, 11.0));
    builder.line_to(vec2(0.0,  11.0));

    builder.move_to(vec2(10.0, 0.0));
    builder.line_to(vec2(20.0, 0.0));
    builder.line_to(vec2(20.0, 10.0));
    builder.line_to(vec2(10.0, 10.0));

    builder.close();

    let path = builder.build();

    tessellate(path.as_slice(), true).unwrap();
}


#[test]
//#[ignore] // TODO
fn reduced_test_case() {
    let mut builder = Path::builder().flattened(0.05);

    builder.move_to(vec2(5.924612, -8.117819));
    builder.line_to(vec2(14.548652, -3.0556107));
    builder.line_to(vec2(9.4864435, 5.568429));
    builder.close();

    builder.move_to(vec2(5.062208, -8.62404));
    builder.line_to(vec2(18.748455, -12.185871));
    builder.line_to(vec2(13.686248, -3.5618315));
    builder.close();

    test_path(builder.build().as_slice(), None);
}

#[test]
#[ignore] // TODO
fn test_colinear_touching_squares_rotated_failing() {
    // Two squares touching.
    //
    //       x-----x
    // x-----x     |
    // |     |     |
    // |     x-----x
    // x-----x
    //
    let mut builder = Path::builder();
    builder.move_to(vec2(0.0,  1.0));
    builder.line_to(vec2(10.0, 1.0));
    builder.line_to(vec2(10.0, 11.0));
    builder.line_to(vec2(0.0,  11.0));

    builder.move_to(vec2(10.0, 0.0));
    builder.line_to(vec2(20.0, 0.0));
    builder.line_to(vec2(20.0, 10.0));
    builder.line_to(vec2(10.0, 10.0));

    builder.close();

    let path = builder.build();

    test_path_with_rotations(path, 0.01, None)
}

#[test]
fn test_point_on_edge_right() {
    //     a
    //    /|
    //   / x  <--- point exactly on edge ab
    //  / /|\
    // x-' | \
    //     b--x
    //
    let mut builder = Path::builder();
    builder.move_to(vec2(0.0, 0.0));
    builder.line_to(vec2(0.0, 100.0));
    builder.line_to(vec2(50.0, 100.0));
    builder.line_to(vec2(0.0, 50.0));
    builder.line_to(vec2(-50.0, 100.0));
    builder.close();

    let path = builder.build();

    tessellate(path.as_slice(), true).unwrap();
}

#[test]
fn test_point_on_edge_left() {
    //     a
    //     |\
    //     x \  <--- point exactly on edge ab
    //    /|\ \
    //   / | `-x
    //  x--b
    //
    let mut builder = Path::builder();
    builder.move_to(vec2(0.0, 0.0));
    builder.line_to(vec2(0.0, 100.0));
    builder.line_to(vec2(-50.0, 100.0));
    builder.line_to(vec2(0.0, 50.0));
    builder.line_to(vec2(50.0, 100.0));
    builder.close();

    let path = builder.build();

    tessellate(path.as_slice(), true).unwrap();
}

#[test]
fn test_coincident_simple() {
    // 0___5
    //  \ /
    // 1 x 4
    //  /_\
    // 2   3

    // A self-intersecting path with two points at the same position.
    let mut builder = Path::builder();
    builder.move_to(vec2(0.0, 0.0));
    builder.line_to(vec2(1.0, 1.0)); // <--
    builder.line_to(vec2(0.0, 2.0));
    builder.line_to(vec2(2.0, 2.0));
    builder.line_to(vec2(1.0, 1.0)); // <--
    builder.line_to(vec2(2.0, 0.0));
    builder.close();

    let path = builder.build();

    tessellate(path.as_slice(), true).unwrap();
}

#[test]
fn test_coincident_simple_2() {
    // A self-intersecting path with two points at the same position.
    let mut builder = Path::builder();
    builder.move_to(vec2(0.0, 0.0));
    builder.line_to(vec2(1.0, 1.0)); // <--
    builder.line_to(vec2(2.0, 0.0));
    builder.line_to(vec2(2.0, 2.0));
    builder.line_to(vec2(1.0, 1.0)); // <--
    builder.line_to(vec2(0.0, 2.0));
    builder.close();

    let path = builder.build();

    tessellate(path.as_slice(), true).unwrap();
}

#[test]
fn test_coincident_simple_rotated() {
    // Same as test_coincident_simple with the usual rotations
    // applied.
    let mut builder = Path::builder();
    builder.move_to(vec2(0.0, 0.0));
    builder.line_to(vec2(1.0, 1.0)); // <--
    builder.line_to(vec2(0.0, 2.0));
    builder.line_to(vec2(2.0, 2.0));
    builder.line_to(vec2(1.0, 1.0)); // <--
    builder.line_to(vec2(2.0, 0.0));
    builder.close();

    let path = builder.build();

    test_path_with_rotations(path, 0.01, None);
}

#[test]
fn test_identical_squares() {
    // Two identical sub paths. It is pretty much the worst type of input for
    // the tessellator as far as I know.
    let mut builder = Path::builder();
    builder.move_to(vec2(0.0, 0.0));
    builder.line_to(vec2(1.0, 0.0));
    builder.line_to(vec2(1.0, 1.0));
    builder.line_to(vec2(0.0, 1.0));
    builder.close();
    builder.move_to(vec2(0.0, 0.0));
    builder.line_to(vec2(1.0, 0.0));
    builder.line_to(vec2(1.0, 1.0));
    builder.line_to(vec2(0.0, 1.0));
    builder.close();

    let path = builder.build();

    tessellate(path.as_slice(), true).unwrap();
}

#[test]
fn test_close_at_first_position() {
    // This path closes at the first position which requires some special handling in the event
    // builder in order to properly add the last vertex events (since first == current, we can't
    // test against the angle of (current, first, second)).
    let mut builder = Path::builder();

    builder.move_to(vec2(107.400665, 91.79798));
    builder.line_to(vec2(108.93136, 91.51076));
    builder.line_to(vec2(107.84248, 91.79686));
    builder.line_to(vec2(107.400665, 91.79798));
    builder.close();

    test_path(builder.build().as_slice(), None);
}
