//! Tesselation routines for complex path fill operations.

use std::f32::consts::PI;
use std::i32;
use std::cmp::{ Ordering };
use std::mem::swap;
use std::cmp::PartialOrd;
use std::cmp;

use super::{ vertex_id, VertexId };
use math::*;
use path::*;
use vertex_builder::{ VertexBufferBuilder, Range, };
use math_utils::{
    is_below, is_below_int, directed_angle, directed_angle2,
    segment_intersection_int, line_horizontal_intersection_int,
};

#[cfg(test)]
use path_builder::{ PrimitiveBuilder, };
#[cfg(test)]
use vertex_builder::{ VertexBuffers, simple_vertex_builder };
#[cfg(test)]
use path_builder::{ flattened_path_builder, };

struct Intersection {
    point: IntVec2,
    lower1: IntVec2,
    lower2: IntVec2,
}

pub type FillResult = Result<(Range, Range), ()>;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Side { Left, Right }

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

#[derive(Copy, Clone, Debug)]
struct SpanEdge {
    upper: IntVec2,
    lower: IntVec2,
    upper_id: VertexId,
    merge: bool,
}

#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: IntVec2,
    id: VertexId,
}

#[derive(Copy, Clone, Debug)]
struct Edge {
    upper: IntVec2,
    lower: IntVec2,
}

struct Span {
    left: SpanEdge,
    right: SpanEdge,
}

impl Span {
    fn begin(current: IntVec2, id: VertexId, left: IntVec2, right: IntVec2) -> Span {
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

    fn merge_vertex(&mut self, vertex: IntVec2, id: VertexId, side: Side) {
        self.set_upper_vertex(vertex, id, side);
        self.mut_edge(side).merge = true;
    }

    fn set_upper_vertex(&mut self, vertex: IntVec2, id: VertexId, side: Side) {
        self.mut_edge(side).upper = vertex;
        self.mut_edge(side).upper_id = id;
    }

    fn set_lower_vertex(&mut self, vertex: IntVec2, side: Side) {
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

#[derive(Clone, Debug)]
struct EdgeBelow {
    // The upper vertex is the current vertex, we don't need to store it.
    lower: IntVec2,
    angle: f32,
}

struct Events {
    edges: Vec<Edge>,
    vertices: Vec<IntVec2>,
}

/// A Context object that can tesselate fill operations for complex paths.
///
/// The Tesselator API is not stable yet.
pub struct FillTesselator {
    sweep_line: Vec<Span>,
    monotone_tesselators: Vec<MonotoneTesselator>,
    intersections: Vec<Edge>,
    below: Vec<EdgeBelow>,
    previous_position: IntVec2,
    scale: f32,
    inv_scale: f32,
    translation: Vec2,
    log: bool,
}

impl FillTesselator {
    /// Constructor.
    pub fn new() -> FillTesselator {
        FillTesselator {
            sweep_line: Vec::with_capacity(16),
            monotone_tesselators: Vec::with_capacity(16),
            below: Vec::with_capacity(8),
            intersections: Vec::with_capacity(8),
            previous_position: int_vec2(i32::MIN, i32::MIN),
            scale: 1000.0,
            inv_scale: 0.001,
            translation: vec2(0.0, 0.0),
            log: false,
        }
    }

    /// Enable some verbose logging during the tesselation, for debugging purposes.
    pub fn enable_logging(&mut self) { self.log = true; }

    /// The length in world space of one tesselator unit.
    /// The tesselator unit defines the precision of the tesselator.
    pub fn set_unit_scale(&mut self, factor: f32) {
        self.scale = factor;
        self.inv_scale = 1.0 / factor;
    }

    /// A translation can be defined in addition to the unit scale to avoid overflowing
    /// the tesselator's coordinate range.
    pub fn set_translation(&mut self, v: Vec2) {
        self.translation = v;
    }

    /// Compute the tesselation (fill).
    ///
    /// This is where most of the interesting things happen.
    pub fn tesselate<Output: VertexBufferBuilder<Vec2>>(&mut self, path: PathSlice, output: &mut Output) -> FillResult {

        self.begin_tesselation(output);

        let events = self.initialize_events(path);

        let mut current_position = int_vec2(i32::MIN, i32::MIN);

        let mut edge_iter = events.edges.iter();
        let mut vertex_iter = events.vertices.iter();
        let mut next_edge = edge_iter.next();
        let mut next_vertex = vertex_iter.next();
        loop {
            let mut next_position = None;
            let mut pending_events = false;

            while let Some(edge) = next_edge {
                if edge.upper == current_position {
                    let edge_vec = self.to_vec2(edge.lower - edge.upper);
                    next_edge = edge_iter.next();
                    self.below.push(EdgeBelow{
                        lower: edge.lower,
                        angle: -directed_angle(vec2(1.0, 0.0), edge_vec),
                    });
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
                if next_position.is_none() || is_below_int(next_position.unwrap(), *vertex) {
                    next_position = Some(*vertex);
                }
                break;
            }

            while !self.intersections.is_empty() {
                let intersection_position = self.intersections[0].upper;
                if intersection_position == current_position {
                    let inter = self.intersections.remove(0);

                    let vec = self.to_vec2(inter.lower - current_position);
                    self.below.push(EdgeBelow {
                        lower: inter.lower,
                        angle: -directed_angle(vec2(1.0, 0.0), vec),
                    });

                    pending_events = true;
                    continue
                }
                if next_position.is_none() || is_below_int(next_position.unwrap(), intersection_position) {
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
                let ranges = self.end_tesselation(output);
                return Ok(ranges);
            }
        }
    }

    fn begin_tesselation<Output: VertexBufferBuilder<Vec2>>(&mut self, output: &mut Output) {
        debug_assert!(self.sweep_line.is_empty());
        debug_assert!(self.monotone_tesselators.is_empty());
        debug_assert!(self.below.is_empty());
        output.begin_geometry();
    }

    fn end_tesselation<Output: VertexBufferBuilder<Vec2>>(&mut self, output: &mut Output) -> (Range, Range) {
        debug_assert!(self.sweep_line.is_empty());
        debug_assert!(self.monotone_tesselators.is_empty());
        debug_assert!(self.below.is_empty());
        return output.end_geometry();
    }

    fn initialize_events(&mut self, path: PathSlice) -> Events {
        let mut events = Events {
            edges: Vec::with_capacity(512),
            vertices: Vec::with_capacity(64),
        };

        for sub_path in path.path_ids() {
            for vertex in path.vertex_ids(sub_path) {
                let mut a = self.to_internal(path.vertex(vertex).position);
                let mut next = self.to_internal(path.vertex(path.next(vertex)).position);
                let prev = self.to_internal(path.vertex(path.previous(vertex)).position);

                let a_below_next = is_below_int(a, next);
                let a_below_prev = is_below_int(a, prev);

                if a_below_next && a_below_prev {
                    // End or merge event don't necessarily have edges below but we need to
                    // process them.
                    events.vertices.push(a);
                }

                if a_below_next {
                    swap(&mut a, &mut next);
                }

                if a == next {
                    continue;
                }

                if self.log {
                    println!(" event {:?} next {:?} {:?}, prev {:?} {:?}", a, next, a_below_next, prev, a_below_prev);
                }

                events.edges.push(Edge { upper: a, lower: next });
            }
        }

        events.edges.sort_by(|a, b|{ compare_positions(a.upper, b.upper) });
        events.vertices.sort_by(|a, b|{ compare_positions(*a, *b) });

        if self.log {
            println!(" -- {} edges and {} vertices", events.edges.len(), events.vertices.len());
        }

        return events;
    }

    fn process_vertex<Output: VertexBufferBuilder<Vec2>>(&mut self, current_position: IntVec2, output: &mut Output) {

        if self.log {
            println!("\n_______________\n");
        }

        let vec2_position = self.to_vec2(current_position);
        let id = vertex_id(output.push_vertex(vec2_position));
        let current = Vertex { position: current_position, id: id };

        self.below.sort_by(|a, b| {
            a.angle.partial_cmp(&b.angle).unwrap_or(Ordering::Equal)
        });

        if self.log {
            for b in &self.below {
                println!(" below angle {:?}", b.angle);
            }
        }

        // Walk the sweep line to determine where we are with respect to the
        // existing spans.
        let mut start_span = 0;

        #[derive(Copy, Clone, Debug, PartialEq)]
        enum E { In, Out, LeftEdge, RightEdge };
        let mut status = E::Out;

        if self.log {
            self.print_sl();
            self.print_sl_at(current_position.y);
        }

        for span in &self.sweep_line {

            if test_span_touches(&span.left, current_position) {
                status = E::LeftEdge;
                break;
            }

            if test_span_side(&span.left, current_position) {
                status = E::Out;
                break;
            }

            if test_span_touches(&span.right, current_position) {
                status = E::RightEdge;
                break;
            }

            if test_span_side(&span.right, current_position) {
                status = E::In;
                break;
            }

            start_span += 1;
        }

        if self.log {
            self.log_sl();
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
                    if self.log { println!("(right event) {}", start_span); }
                    let edge = Edge {
                        upper: current_position,
                        lower: self.below[0].lower,
                    };
                    self.insert_edge(
                        start_span, Side::Right, edge, id,
                    );
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

        if self.log {
            println!(" -- start_span {} span_idx {}", start_span, span_idx);
        }

        // Count the number of remaining edges above the sweep line that end at
        // the current position.
        for span in &self.sweep_line[span_idx..] {
            let left = test_span_touches(&span.left, current_position);
            let right = test_span_touches(&span.right, current_position);
            // Here it is tempting to assume that we can only have end events
            // if left_interects && right_intersects, but we also need to take merge
            // vertices into account.
            if left {
                if self.log { println!(" -- touch left"); }
                above_count += 1;
            }
            if right {
                if self.log { println!(" -- touch right"); }
                above_count += 1;
            }

            // We can't assume that if left and right are false we are already past
            // the current point because both sides of the span could be in the merge state.

            // If right_intersects, left should intersect too, unless it is a merge.
            debug_assert!(!right || left || span.left.merge);
        }

        if self.log {
            println!(" -- above count {}", above_count);
        }

        // Pairs of edges that end at the current position form "end events".
        // By construction we know that they are on the same spans.
        while above_count >= 2 {
            if self.log { println!("(end event)"); }

            self.resolve_merge_vertices(span_idx, current.position, current.id, output);
            self.end_span(span_idx, current, output);

            above_count -= 2;
        }

        if pending_merge {
            debug_assert!(above_count == 1);
            if self.log { println!("(merge event)"); }
            self.merge_event(current, start_span, output)
        } else if above_count == 1 {
            if self.log { println!("(left event) {}", span_idx); }
            assert!(below_count > 0);

            self.resolve_merge_vertices(span_idx, current_position, id, output);

            let vertex_below = self.below[self.below.len()-1].lower;
            self.insert_edge(
                span_idx, Side::Left,
                Edge { upper: current_position, lower: vertex_below }, id,
            );

            below_count -= 1;
        }

        // Since we took care of left and right event already we should not have
        // an odd number of edges to work with below the current vertex by now.
        assert!(below_count % 2 == 0);

        // reset span_idx for the next pass.
        let mut span_idx = start_span;

        // Step 2, handle edges below the current vertex.
        if below_count > 0 {
            if status == E::In {
                if self.log { println!("(split event)"); }
                let left = self.below[0].clone();
                let right = self.below[below_count-1].clone();
                self.split_event(start_span, current_position, id, left, right, output);
                below_count -= 2;
                below_idx += 1;
            }

            while below_count >= 2 {
                if self.log { println!("(start event) {}", span_idx); }

                let l = self.below[below_idx].lower;
                let r = self.below[below_idx + 1].lower;
                let mut left_edge = Edge { upper: current_position, lower: l };
                let mut right_edge = Edge { upper: current_position, lower: r };
                self.check_intersections(&mut left_edge);
                self.check_intersections(&mut right_edge);
                self.sweep_line.insert(span_idx, Span::begin(current_position, id, left_edge.lower, right_edge.lower));
                let vec2_position = self.to_vec2(current_position);
                self.monotone_tesselators.insert(span_idx, MonotoneTesselator::begin(vec2_position, id));

                below_idx += 2;
                below_count -= 2;
                span_idx += 1;
            }
        }

        self.check_sl(current_position);

        self.below.clear();
    }

    // Look for eventual merge vertices on this span above the current vertex, and connect
    // them to the current vertex.
    // This should be called when processing a vertex that is on the left side of a span.
    fn resolve_merge_vertices<Output: VertexBufferBuilder<Vec2>>(&mut self,
        span_idx: usize,
        current: IntVec2, id: VertexId,
        output: &mut Output
    ) {
        while self.sweep_line[span_idx].right.merge {
            //     \ /
            //  \   x   <-- merge vertex
            //   \ :
            //    x   <-- current vertex
            self.sweep_line[span_idx+1].set_lower_vertex(current, Side::Left);
            self.end_span(span_idx, Vertex { position: current, id: id }, output);
        }
    }

    fn split_event<Output: VertexBufferBuilder<Vec2>>(&mut self,
        span_idx: usize, current: IntVec2, id: VertexId,
        left: EdgeBelow, right: EdgeBelow,
        output: &mut Output
    ) {
        if self.log {
            println!(" ++++++ Split event {} (span {})", id.handle, span_idx);
        }

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
            let vec2_position = self.to_vec2(ll.upper);
            self.monotone_tesselators.insert(
                span_idx, MonotoneTesselator::begin(vec2_position, ll.upper_id)
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

    fn merge_event<Output: VertexBufferBuilder<Vec2>>(&mut self, vertex: Vertex, span_idx: usize, output: &mut Output) {
        if self.log {
            println!(" ++++++ Merge event {} (span {})", vertex.id.handle, span_idx);
        }

        assert!(span_idx < self.sweep_line.len()-1);

        let left_span = span_idx;
        let right_span = span_idx+1;

        //     / \ /
        //  \ / .-x    <-- merge vertex
        //   x-'      <-- current merge vertex
        self.resolve_merge_vertices(right_span, vertex.position, vertex.id, output);

        let vec2_position = self.to_vec2(vertex.position);

        self.sweep_line[left_span].merge_vertex(vertex.position, vertex.id, Side::Right);
        self.monotone_tesselators[left_span].vertex(vec2_position, vertex.id, Side::Right);

        self.sweep_line[right_span].merge_vertex(vertex.position, vertex.id, Side::Left);
        self.monotone_tesselators[right_span].vertex(vec2_position, vertex.id, Side::Left);
    }

    fn insert_edge(&mut self,
        span_idx: usize, side: Side,
        mut edge: Edge, id: VertexId,
    ) {
        // TODO horrible hack: set the merge flag on the edge we are about to replace temporarily
        // so that it doesn not get in the way of the intersection detection.
        self.sweep_line[span_idx].mut_edge(side).merge = true;
        self.check_intersections(&mut edge);
        // This sets the merge flag to false.
        self.sweep_line[span_idx].edge(edge, id, side);
        let vec2_position = self.to_vec2(edge.upper);
        self.monotone_tesselators[span_idx].vertex(vec2_position, id, side);

    }

    fn check_intersections(&mut self, edge: &mut Edge) {
        let original_edge = *edge;
        let mut intersection = None;
        let mut span_idx = 0;

        for span in &mut self.sweep_line {

            // Test for an intersection against the span's left edge.
            if !span.left.merge {
                if let Some(position) = segment_intersection_int(
                    edge.upper, edge.lower,
                    span.left.upper, span.left.lower,
                ) {
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
                            lower1: original_edge.lower, lower2: span.left.lower
                        },

                        span_idx, Side::Left
                    ));
                    // From now on only consider potential intersections above the one we found,
                    // by removing the lower part from the segment we test against.
                    edge.lower = position;
                }
            }

            // Same thing for the span's right edge.
            if !span.right.merge {
                if let Some(position) = segment_intersection_int(
                    edge.upper, edge.lower,
                    span.right.upper, span.right.lower,
                ) {
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
                            lower1: original_edge.lower, lower2: span.right.lower
                        },
                        span_idx, Side::Right
                    ));
                    edge.lower = position;
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
            if !is_below_int(evt.point, current_position) {
                evt.point.y = current_position.y + 1;
                edge.lower = evt.point;
            }

            let mut e1 = Edge { upper: evt.point, lower: evt.lower1 };
            let mut e2 = Edge { upper: evt.point, lower: evt.lower2 };
            // Same deal with the precision issues here. In this case we can just flip the new
            // edge so that its upper member is indeed above the lower one.
            if is_below_int(e1.upper, e1.lower) { swap(&mut e1.upper, &mut e1.lower); }
            if is_below_int(e2.upper, e2.lower) { swap(&mut e2.upper, &mut e2.lower); }

            if self.log {
                println!(" set span[{:?}].{:?}.lower = {:?} (was {:?}",
                    span_idx, side, evt.point.tuple(),
                    self.sweep_line[span_idx].mut_edge(side).lower.tuple()
                );
            }

            self.sweep_line[span_idx].mut_edge(side).lower = evt.point;
            self.intersections.push(e1);
            self.intersections.push(e2);
            // TODO lazily sort intersections next time we read from the vector or
            // do a sorted insertion.
            self.intersections.sort_by(|a, b| { compare_positions(a.upper, b.upper) });
        }
    }

    fn end_span<Output: VertexBufferBuilder<Vec2>>(&mut self, span_idx: usize, vertex: Vertex, output: &mut Output) {
        if self.log {
            println!("     end span {} (vertex: {})", span_idx, vertex.id.handle);
        }
        let vec2_position = self.to_vec2(vertex.position);
        {
            let tess = &mut self.monotone_tesselators[span_idx];
            tess.end(vec2_position, vertex.id);
            tess.flush(output);
        }
        self.sweep_line.remove(span_idx);
        self.monotone_tesselators.remove(span_idx);
    }

    fn to_internal(&self, v: Vec2) -> IntVec2 {
        let v = v + self.translation;
        int_vec2((v.x * self.scale) as i32, (v.y * self.scale) as i32)
    }

    fn to_vec2(&self, v: IntVec2) -> Vec2 {
        vec2(v.x as f32 * self.inv_scale, v.y as f32 * self.inv_scale) - self.translation
    }

    fn check_sl(&self, current: IntVec2) {
        for span in &self.sweep_line {
            if !span.left.merge {
                assert!(!is_below_int(current, span.left.lower));
                assert!(!is_below_int(span.left.upper, span.left.lower));
            }
            if !span.right.merge {
                assert!(!is_below_int(current, span.right.lower));
                assert!(!is_below_int(span.right.upper, span.right.lower));
            }
        }
    }

    /// Print the current state of the sweep line for debgging purposes.
    fn log_sl(&self) {
        print!("\n|  sl: ");
        for span in &self.sweep_line {
            let ml = if span.left.merge { "*" } else { " " };
            let mr = if span.right.merge { "*" } else { " " };
            print!("| {:?}{}  {:?}{}|  ", span.left.upper_id.handle, ml, span.right.upper_id.handle, mr);
        }
        println!("");
    }

    fn print_sl(&self) {
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

    fn print_sl_at(&self, y: i32) {
        print!("\nat y={}  sl: [", y);
        for span in &self.sweep_line {
            if span.left.merge {
                print!("| l:<merge> ");
            } else {
                let lx = line_horizontal_intersection_int(span.left.upper, span.left.lower, y);
                print!("| l:{:?} ", lx);
            }
            if span.right.merge {
                print!(" r:<merge> |");
            } else {
                let rx = line_horizontal_intersection_int(span.right.upper, span.right.lower, y);
                print!(" r:{:?} |", rx);
            }
        }
        println!("]\n");
    }
}

fn compare_positions(a: IntVec2, b: IntVec2) -> Ordering {
    if a.y > b.y { return Ordering::Greater; }
    if a.y < b.y { return Ordering::Less; }
    if a.x > b.x { return Ordering::Greater; }
    if a.x < b.x { return Ordering::Less; }
    return Ordering::Equal;
}

fn test_span_side(span_edge: &SpanEdge, position: IntVec2) -> bool {
    if span_edge.merge {
        return false;
    }

    if span_edge.lower == position {
        return true;
    }

    let from = span_edge.upper;
    let to = span_edge.lower;

    let vx = (to.x - from.x) as i64;
    let vy = (to.y - from.y) as i64;
    if vy == 0 {
        // If the segment is horizontal, pick the biggest x value (the right-most point).
        // That's arbitrary, not sure it is the right thing to do.
        return cmp::max(position.x, to.x) >= position.x;
    }
    // shuffled around from:
    // edge_from.x + (point.y - edge_from.y) * vx / vy >= point.x
    // in order to remove the division.
    return (position.y - from.y) as i64 * vx >= (position.x - from.x) as i64 * vy;
}

fn test_span_touches(span_edge: &SpanEdge, position: IntVec2) -> bool {
    if span_edge.merge {
        return false;
    }

    if span_edge.lower == position {
        return true;
    }

    let from = span_edge.upper;
    let to = span_edge.lower;

    let vx = (to.x - from.x) as i64;
    let vy = (to.y - from.y) as i64;
    if vy == 0 {
        return cmp::max(position.x, to.x) >= position.x;
    }
    return (position.y - from.y) as i64 * vx == (position.x - from.x) as i64 * vy;
}

/// helper class that generates a triangulation from a sequence of vertices describing a monotone
/// polygon.
struct MonotoneTesselator {
    stack: Vec<MonotoneVertex>,
    previous: MonotoneVertex,
    triangles: Vec<(u16, u16, u16)>,
}

#[derive(Copy, Clone, Debug)]
struct MonotoneVertex {
    pos: Vec2,
    id: VertexId,
    side: Side,
}

impl MonotoneTesselator {
    pub fn begin(pos: Vec2, id: VertexId) -> MonotoneTesselator {
        let first = MonotoneVertex { pos: pos, id: id, side: Side::Left };

        //println!(" @@@ begin monotone vertex: {:?}", pos.tuple());

        let mut tess = MonotoneTesselator {
            stack: Vec::with_capacity(16),
            triangles: Vec::with_capacity(128),
            previous: first,
        };

        tess.stack.push(first);

        return tess;
    }

    pub fn vertex(&mut self, pos: Vec2, id: VertexId, side: Side) {
        let current = MonotoneVertex{ pos: pos, id: id, side: side };
        let right_side = current.side == Side::Right;

        //println!(" @@@ monotone vertex: {:?} (was {:?})", current.pos.tuple(), self.previous.pos.tuple());
        assert!(is_below(current.pos, self.previous.pos));
        assert!(!self.stack.is_empty());

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

    pub fn end(&mut self, pos: Vec2, id: VertexId) {
        let side = self.previous.side.opposite();
        //println!(" @@@ end monotone tesselator with stack {}", self.stack.len());
        self.vertex(pos, id, side);
        self.stack.clear();
    }

    fn push_triangle(&mut self, a: &MonotoneVertex, b: &MonotoneVertex, c: &MonotoneVertex) {
        //println!(" #### triangle {} {} {}", a.id.handle, b.id.handle, c.id.handle);

        if directed_angle2(b.pos, c.pos, a.pos) <= PI {
            self.triangles.push((a.id.handle, b.id.handle, c.id.handle));
        } else {
            self.triangles.push((b.id.handle, a.id.handle, c.id.handle));
        }
    }

    fn flush<Output: VertexBufferBuilder<Vec2>>(&mut self, output: &mut Output) {
        for &(a, b, c) in &self.triangles {
            output.push_indices(a, b, c);
        }
        self.triangles.clear();
    }
}

#[test]
fn test_monotone_tess() {
    println!(" ------------ ");
    {
        let mut tess = MonotoneTesselator::begin(vec2(0.0, 0.0), vertex_id(0));
        tess.vertex(vec2(-1.0, 1.0), vertex_id(1), Side::Left);
        tess.end(vec2(1.0, 2.0), vertex_id(2));
        assert_eq!(tess.triangles.len(), 1);
    }
    println!(" ------------ ");
    {
        let mut tess = MonotoneTesselator::begin(vec2(0.0, 0.0), vertex_id(0));
        tess.vertex(vec2(1.0, 1.0), vertex_id(1), Side::Right);
        tess.vertex(vec2(-1.5, 2.0), vertex_id(2), Side::Left);
        tess.vertex(vec2(-1.0, 3.0), vertex_id(3), Side::Left);
        tess.vertex(vec2(1.0, 4.0), vertex_id(4), Side::Right);
        tess.end(vec2(0.0, 5.0), vertex_id(5));
        assert_eq!(tess.triangles.len(), 4);
    }
    println!(" ------------ ");
    {
        let mut tess = MonotoneTesselator::begin(vec2(0.0, 0.0), vertex_id(0));
        tess.vertex(vec2(1.0, 1.0), vertex_id(1), Side::Right);
        tess.vertex(vec2(3.0, 2.0), vertex_id(2), Side::Right);
        tess.vertex(vec2(1.0, 3.0), vertex_id(3), Side::Right);
        tess.vertex(vec2(1.0, 4.0), vertex_id(4), Side::Right);
        tess.vertex(vec2(4.0, 5.0), vertex_id(5), Side::Right);
        tess.end(vec2(0.0, 6.0), vertex_id(6));
        assert_eq!(tess.triangles.len(), 5);
    }
    println!(" ------------ ");
    {
        let mut tess = MonotoneTesselator::begin(vec2(0.0, 0.0), vertex_id(0));
        tess.vertex(vec2(-1.0, 1.0), vertex_id(1), Side::Left);
        tess.vertex(vec2(-3.0, 2.0), vertex_id(2), Side::Left);
        tess.vertex(vec2(-1.0, 3.0), vertex_id(3), Side::Left);
        tess.vertex(vec2(-1.0, 4.0), vertex_id(4), Side::Left);
        tess.vertex(vec2(-4.0, 5.0), vertex_id(5), Side::Left);
        tess.end(vec2(0.0, 6.0), vertex_id(6));
        assert_eq!(tess.triangles.len(), 5);
    }
    println!(" ------------ ");
}

pub enum FillRule {
    EvenOdd,
    NonZero,
}

/// Parameters for the tesselator.
pub struct TesselatorConfig {
    /// See the SVG specification.
    ///
    /// Currently, only the EvenOdd rule is implemented.
    pub fill_rule: FillRule,

    /// An anti-aliasing trick extruding a 1-px wide strip around the edges with
    /// a gradient to smooth the edges.
    ///
    /// Not implemented yet!
    pub vertex_aa: bool,

    /// If set to false, the tesselator will separate the quadratic bezier segments
    /// from the rest of the shape so that their tesselation can be done separately,
    /// for example in a fragment shader.
    ///
    /// Not implemented yet!
    pub flatten_curves: bool,
}

impl TesselatorConfig {
    pub fn new() -> TesselatorConfig { TesselatorConfig::even_odd() }

    pub fn even_odd() -> TesselatorConfig {
        TesselatorConfig {
            fill_rule: FillRule::EvenOdd,
            vertex_aa: false,
            flatten_curves: true,
        }
    }

    pub fn non_zero() -> TesselatorConfig {
        TesselatorConfig {
            fill_rule: FillRule::NonZero,
            vertex_aa: false,
            flatten_curves: true,
        }
    }

    pub fn with_vertex_aa(mut self) -> TesselatorConfig {
        self.vertex_aa = true;
        return self;
    }
}

#[cfg(test)]
fn tesselate(path: PathSlice, log: bool) -> Result<usize, ()> {
    let mut buffers: VertexBuffers<Vec2> = VertexBuffers::new();
    {
        let mut vertex_builder = simple_vertex_builder(&mut buffers);
        let mut tess = FillTesselator::new();
        if log {
            tess.enable_logging();
        }
        try!{ tess.tesselate(path, &mut vertex_builder) };
    }
    return Ok(buffers.indices.len()/3);
}

#[cfg(test)]
fn test_path(path: PathSlice, expected_triangle_count: Option<usize>) {
    let res = ::std::panic::catch_unwind(|| { tesselate(path, false) });

    if let Ok(Ok(num_triangles)) = res {
        if let Some(expected_triangles) = expected_triangle_count {
            if num_triangles != expected_triangles {
                tesselate(path, true).unwrap();
                panic!("expected {} triangles, got {}", expected_triangles, num_triangles);
            }
        }
        return;
    }

    ::lyon_extra::debugging::find_reduced_test_case(path, &|path: Path|{
        return tesselate(path.as_slice(), false).is_err();
    });

    tesselate(path, true).unwrap();
    panic!();
}

#[cfg(test)]
fn test_path_with_rotations(path: Path, step: f32, expected_triangle_count: Option<usize>) {
    let mut angle = 0.0;

    while angle < PI * 2.0 {
        println!("\n\n ==================== angle = {}", angle);
        test_path_with_rotation(&path, angle, expected_triangle_count);
        angle += step;
    }
}

#[cfg(test)]
fn test_path_with_rotation(path: &Path, angle: f32, expected_triangle_count: Option<usize>) {
    let mut tranformed_path = path.clone();
    let cos = angle.cos();
    let sin = angle.sin();
    for v in tranformed_path.mut_vertices() {
        let (x, y) = (v.position.x, v.position.y);
        v.position.x = x*cos + y*sin;
        v.position.y = y*cos - x*sin;
    }
    test_path(tranformed_path.as_slice(), expected_triangle_count);
}

#[test]
fn test_simple_triangle() {
    let mut path = flattened_path_builder();
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(1.0, 1.0));
    path.line_to(vec2(0.0, 1.0));
    path.close();

    test_path_with_rotations(path.build(), 0.01, Some(1));
}

#[test]
fn test_simple_monotone() {
    let mut path = flattened_path_builder();
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
    let mut path = flattened_path_builder();
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
    let mut path = flattened_path_builder();
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
    let mut path = flattened_path_builder();
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
    let mut path = flattened_path_builder();
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
    let mut path = flattened_path_builder();
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
    let mut path = flattened_path_builder();
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
    let mut path = flattened_path_builder();
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
    let mut path = flattened_path_builder();
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
    let mut path = flattened_path_builder();
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
    let mut path = flattened_path_builder();
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
    let mut path = flattened_path_builder();

    ::lyon_extra::rust_logo::build_logo_path(&mut path);

    test_path_with_rotations(path.build(), 0.011, None);
}

#[test]
fn test_rust_logo_with_intersection() {
    let mut path = flattened_path_builder();

    ::lyon_extra::rust_logo::build_logo_path(&mut path);

    path.move_to(vec2(10.0, 30.0));
    path.line_to(vec2(130.0, 30.0));
    path.line_to(vec2(130.0, 60.0));
    path.line_to(vec2(10.0, 60.0));
    path.close();

    let path = path.build();

    test_path_with_rotations(path, 0.011, None);
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
    let mut path = flattened_path_builder();

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
    let mut path = flattened_path_builder();

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
    let mut path = flattened_path_builder();

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
    let mut path = flattened_path_builder();

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
    let mut path = flattened_path_builder();

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
    let mut builder = flattened_path_builder();

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

    let mut builder = flattened_path_builder();

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
fn reduced_test_case() {
    let mut builder = flattened_path_builder();

    builder.move_to(vec2(-21.004179, -71.57515));
    builder.line_to(vec2(-21.927473, -70.94977));
    builder.line_to(vec2(-23.024633, -70.68942));
    builder.close();

    builder.move_to(vec2(16.036617, -27.254852));
    builder.line_to(vec2(-62.83691, -117.69249));
    builder.line_to(vec2(38.646027, -46.973236));
    builder.close();

    test_path(builder.build().as_slice(), None);
}

#[test]
fn test_colinear_1() {
    let mut builder = flattened_path_builder();
    builder.move_to(vec2(20.0, 150.0));
    builder.line_to(vec2(80.0, 150.0));
    builder.close();

    let path = builder.build();

    test_path_with_rotations(path, 0.01, None);
}

#[test]
fn test_colinear_2() {
    let mut builder = flattened_path_builder();
    builder.move_to(vec2(20.0, 150.0));
    builder.line_to(vec2(80.0, 150.0));
    builder.line_to(vec2(20.0, 150.0));
    builder.close();

    let path = builder.build();

    test_path_with_rotations(path, 0.01, None);
}

#[test]
#[ignore] // TODO
fn test_colinear_3() {
    let mut builder = flattened_path_builder();
    // The path goes through many points along a line.
    builder.move_to(vec2(0.0, 1.0));
    builder.line_to(vec2(0.0, 3.0));
    builder.line_to(vec2(0.0, 5.0));
    builder.line_to(vec2(0.0, 4.0));
    builder.line_to(vec2(0.0, 2.0));
    builder.close();

    let path = builder.build();

    tesselate(path.as_slice(), true).unwrap();
}

#[test]
fn test_colinear_4() {
    // The path goes back and forth along a line.
    let mut builder = flattened_path_builder();
    builder.move_to(vec2(0.0, 2.0));
    builder.line_to(vec2(0.0, 1.0));
    builder.line_to(vec2(0.0, 3.0));
    builder.line_to(vec2(0.0, 0.0));
    builder.close();

    let path = builder.build();

    tesselate(path.as_slice(), true).unwrap();
}

#[test]
fn test_coincident_simple() {
    // 0___5
    //  \ /
    // 1 x 4
    //  /_\
    // 2   3

    // A self-intersecting path with two points at the same position.
    let mut builder = flattened_path_builder();
    builder.move_to(vec2(0.0, 0.0));
    builder.line_to(vec2(1.0, 1.0)); // <--
    builder.line_to(vec2(0.0, 2.0));
    builder.line_to(vec2(2.0, 2.0));
    builder.line_to(vec2(1.0, 1.0)); // <--
    builder.line_to(vec2(2.0, 0.0));
    builder.close();

    let path = builder.build();

    tesselate(path.as_slice(), true).unwrap();
    //test_path_with_rotations(path, 0.01, None);
}

#[test]
fn test_coincident_simple_2() {
    // A self-intersecting path with two points at the same position.
    let mut builder = flattened_path_builder();
    builder.move_to(vec2(0.0, 0.0));
    builder.line_to(vec2(1.0, 1.0)); // <--
    builder.line_to(vec2(2.0, 0.0));
    builder.line_to(vec2(2.0, 2.0));
    builder.line_to(vec2(1.0, 1.0)); // <--
    builder.line_to(vec2(0.0, 2.0));
    builder.close();

    let path = builder.build();

    tesselate(path.as_slice(), true).unwrap();
}

#[test]
fn test_coincident_simple_rotated() {
    // Same as test_coincident_simple with the usual rotations
    // applied.
    let mut builder = flattened_path_builder();
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
    // Two identical sub paths. It's a pretty much the worst type of input for
    // the tesselator as far as I know.
    let mut builder = flattened_path_builder();
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

    tesselate(path.as_slice(), true).unwrap();
}
