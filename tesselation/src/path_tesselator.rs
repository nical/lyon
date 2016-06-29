//! Tesselation routines for complex paths.

use std::f32::consts::PI;
use std::f32::NAN;
use std::cmp::{ Ordering };
use std::mem::swap;
use std::cmp::PartialOrd;

use super::{ vertex_id, VertexId };
use path::*;
use path_builder::{ PrimitiveBuilder, };
use vertex_builder::{ VertexBufferBuilder, Range, };
use math_utils::{
    is_below, tangent,
    segment_intersection,line_intersection, line_horizontal_intersection,
};
use basic_shapes::{ tesselate_quad };

use vodk_math::{ Vec2, vec2 };

#[cfg(test)]
use vertex_builder::{ VertexBuffers, simple_vertex_builder, };
#[cfg(test)]
use path_builder::{ flattened_path_builder, };

struct Intersection {
    point: Vec2,
    a_down: Vec2,
    b_down: Vec2,
}

pub type TesselatorResult = Result<(), ()>;

fn error<K>() -> Result<K, ()> {
    panic!();
    //return Err(());
}

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
    upper: Vec2,
    lower: Vec2,
    upper_id: VertexId,
    merge: bool,
}

#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: Vec2,
    id: VertexId,
}

#[derive(Copy, Clone, Debug)]
struct Edge {
    upper: Vec2,
    lower: Vec2,
}

struct Span {
    left: SpanEdge,
    right: SpanEdge,
    monotone_tesselator: MonotoneTesselator,
}

impl Span {
    fn begin(current: Vec2, id: VertexId, left: Vec2, right: Vec2) -> Span {
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
            monotone_tesselator: MonotoneTesselator::begin(current, id),
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

    fn merge_vertex(&mut self, vertex: Vec2, id: VertexId, side: Side) {
        self.set_upper_vertex(vertex, id, side);
        self.mut_edge(side).merge = true;
    }

    fn set_upper_vertex(&mut self, vertex: Vec2, id: VertexId, side: Side) {
        self.mut_edge(side).upper = vertex;
        self.mut_edge(side).upper_id = id;
        self.monotone_tesselator.vertex(vertex, id, side);
    }

    fn set_lower_vertex(&mut self, vertex: Vec2, side: Side) {
        //println!("    (set lower vertex at {:?}", vertex.tuple());
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

#[derive(Debug)]
struct EdgeBelow {
    // The upper vertex is the current vertex, we don't need to store it.
    lower: Vec2,
    angle: f32,
    test_intersections: bool,
}

struct Events {
    edges: Vec<Edge>,
    vertices: Vec<Vec2>,
}

/// A Context object that can tesselate fill operations for complex paths.
///
/// The Tesselator API is not stable yet.
pub struct Tesselator<'l, Output: VertexBufferBuilder<Vec2>+'l> {
    sweep_line: Vec<Span>,
    intersections: Vec<Intersection>,
    events: Option<Events>,
    below: Vec<EdgeBelow>,
    previous_position: Vec2,
    log: bool,
    output: &'l mut Output,
}

impl<'l, Output: VertexBufferBuilder<Vec2>> Tesselator<'l, Output> {
    /// Constructor.
    pub fn new(output: &'l mut Output) -> Tesselator<'l, Output> {
        Tesselator {
            sweep_line: Vec::with_capacity(16),
            events: Some(Events {
                edges: Vec::with_capacity(512),
                vertices: Vec::with_capacity(64),
            }),
            below: Vec::with_capacity(8),
            intersections: Vec::with_capacity(8),
            previous_position: vec2(NAN, NAN),
            log: false,
            output: output,
        }
    }

    /// Enable some verbose logging during the tesselation, for debugging purposes.
    pub fn enable_logging(&mut self) { self.log = true; }

    /// Compute the tesselation (fill).
    ///
    /// This is where most of the interesting things happen.
    pub fn tesselate(&mut self, path: PathSlice) -> TesselatorResult {
        let events = self.initialize_events(path);

        let mut last_pos = vec2(NAN, NAN);

        let mut edge_iter = events.edges.iter();
        let mut vertex_iter = events.vertices.iter();
        let mut next_edge = edge_iter.next();
        let mut next_vertex = vertex_iter.next();
        loop {
            let mut next_source = 0;
            let mut source_pos = vec2(0.0, 2000000.0); // TODO

            if false && !self.intersections.is_empty() {
                next_source = 1;
                source_pos = self.intersections[0].point;
            }

            if let Some(edge) = next_edge {
                if is_below(source_pos, edge.upper) {
                    next_source = 2;
                    source_pos = edge.upper;
                }
            }

            if let Some(vertex) = next_vertex {
                if is_below(source_pos, *vertex) {
                    next_source = 3;
                    source_pos = *vertex;
                }
            }

            if next_source == 0 {
                let res = self.process_vertex(last_pos);
                debug_assert!(self.sweep_line.len() == 0);
                return res;
            }

            let snapped_position = snap_v(source_pos);
            if snapped_position != last_pos {
                try!{ self.process_vertex(last_pos) };
                last_pos = snapped_position;
            }

            match next_source {
                1 => {
                    let inter = self.intersections.remove(0);
                    self.on_intersection_event(&inter);
                }
                2 => {
                    let edge = next_edge.unwrap();
                    let edge_vec = edge.lower - edge.upper;
                    self.below.push(EdgeBelow{
                        lower: edge.lower,
                        angle: -vec2(1.0, 0.0).directed_angle(edge_vec),
                        test_intersections: true,
                    });
                    next_edge = edge_iter.next();
                    if self.log {
                        println!(" edge at {:?} ->{:?}", snapped_position.tuple(), edge.lower.tuple());
                    }
                }
                3 => {
                    if self.log {
                        println!(" vertex at {:?}", snapped_position.tuple());
                    }
                    next_vertex = vertex_iter.next();
                }
                _ => { unreachable!(); }
            }
        }
    }

    fn initialize_events(&mut self, path: PathSlice) -> Events {
        let mut swap_events = None;
        swap(&mut self.events, &mut swap_events);
        let mut events = swap_events.unwrap();

        for sub_path in path.path_ids() {
            for vertex in path.vertex_ids(sub_path) {
                let mut a = path.vertex(vertex).position;
                let mut next = path.vertex(path.next(vertex)).position;

                let prev = path.vertex(path.previous(vertex)).position;
                let a_below_next = is_below(a, next);
                let a_below_prev = is_below(a, prev);

                if a_below_next && a_below_prev {
                    // End or merge event don't necessarily have edges below but we need to
                    // process them.
                    events.vertices.push(a);
                }

                if a_below_next {
                    swap(&mut a, &mut next);
                }

                if self.log {
                    println!(" event {:?} next {:?} {:?}, prev {:?} {:?}", a, next, a_below_next, prev, a_below_prev);
                }

                if snap_v(a) != snap_v(next) {
                    events.edges.push(Edge { upper: a, lower: next });
                }
            }
        }

        events.edges.sort_by(|a, b|{ compare_positions(a.upper, b.upper) });
        events.vertices.sort_by(|a, b|{ compare_positions(*a, *b) });

        if self.log {
            println!(" -- {} edges and {} vertices", events.edges.len(), events.vertices.len());
        }

        return events;
    }

    fn process_vertex(&mut self, current_position: Vec2) -> TesselatorResult {

        if self.log {
            println!("\n_______________\n");
        }

        let id = vertex_id(self.output.push_vertex(current_position));
        let current = Vertex { position: current_position, id: id };

        self.below.sort_by(|a, b| {
            a.angle.partial_cmp(&b.angle).unwrap_or(Ordering::Equal)
        });

        // Walk the sweep line to determine where we are with respect to the
        // existing spans.
        let mut start_span = 0;

        #[derive(Copy, Clone, Debug, PartialEq)]
        enum E { In, Out, LeftEdge, RightEdge };
        let mut status = E::Out;

        //self.print_sl();
        //self.print_sl_at(current_position.x);

        for span in &self.sweep_line {

            fn almos_eq(a: f32, b: f32) -> bool { (a - b).abs() < 0.00001 }

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

        // Step 1, wal the sweep line, handle left/right events, handle the spans that end
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
                    let test_intersections = self.below[0].test_intersections;
                    self.insert_edge(
                        start_span, Side::Right, edge, id,
                        test_intersections
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

        for span in &self.sweep_line[span_idx..] {
            let l = test_span_touches(&span.left, current_position);
            let r = test_span_touches(&span.right, current_position);
            if l { above_count += 1; }
            if r { above_count += 1; }
            if !l && !r {
                break;
            }
        }

        while above_count >= 2 {
            if self.log { println!("(end event)"); }
            self.on_end_event(current, span_idx);
            above_count -= 2;
        }

        if pending_merge {
            debug_assert!(above_count == 1);
            if self.log { println!("(merge event)"); }
            self.on_merge_event(current, start_span)
        } else if above_count == 1 {
            assert!(below_count > 0);
            if self.log { println!("(left event) {}", span_idx); }
            let down = self.below[self.below.len()-1].lower;
            // TODO check intersections ?
            self.on_left_event(span_idx, current_position, id, down, true);
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
                let left = self.below[0].lower;
                let right = self.below[below_count-1].lower;
                self.on_split_event(start_span, current_position, id, left, right);
                below_count -= 2;
                below_idx += 1;
            }

            while below_count >= 2 {
                if self.log { println!("(start event) {}", span_idx); }

                let non_existant_index = self.sweep_line.len();
                let l = self.below[below_idx].lower;
                let r = self.below[below_idx + 1].lower;
                self.check_intersections(non_existant_index, Side::Left, Edge { upper: current_position, lower: l });
                self.check_intersections(non_existant_index, Side::Right, Edge { upper: current_position, lower: r });
                self.sweep_line.insert(span_idx, Span::begin(current_position, id, l, r));

                below_idx += 2;
                below_count -= 2;
                span_idx += 1;
            }
        }

        //self.check_sl(current_position);

        self.below.clear();

        return Ok(());
    }

    fn on_left_event(&mut self,
        span_index: usize,
        current: Vec2, id: VertexId, next: Vec2,
        test_intersections: bool
    ) {
        if self.log {
            println!(" ++++++ Left event {}", id.handle);
        }

        if self.sweep_line[span_index].right.merge {
            //     \ /
            //  \   x   <-- merge vertex
            //   \ :
            // ll x   <-- current vertex
            //     \r
            self.sweep_line[span_index+1].set_lower_vertex(current, Side::Left);
            self.end_span(span_index, Vertex { position: current, id: id });
        }

        self.insert_edge(
            span_index, Side::Left,
            Edge { upper: current, lower: next }, id,
            test_intersections
        );
    }

    fn on_split_event(&mut self, span_index: usize, current: Vec2, id: VertexId, left: Vec2, right: Vec2) {
        if self.log {
            println!(" ++++++ Split event {} (span {})", id.handle, span_index);
        }

        let test_intersections = true; // TODO

        // look whether the span shares a merge vertex with the previous one
        if self.sweep_line[span_index].left.merge {
            let left_span = span_index-1;
            let right_span = span_index;
            //            \ /
            //             x   <-- merge vertex
            //  left_span  :  righ_span
            //             x   <-- current split vertex
            //           l/ \r
            self.insert_edge(
                left_span, Side::Right,
                Edge { upper: current, lower: left }, id,
                test_intersections
            );
            self.insert_edge(
                right_span, Side::Left,
                Edge { upper: current, lower: right }, id,
                test_intersections
            );
        } else {
            //      /
            //     x
            //    / :r2
            // ll/   x   <-- current split vertex
            //  left/ \right
            let ll = self.sweep_line[span_index].left;
            let r2 = Edge {
                upper: ll.upper,
                lower: current,
            };

            self.sweep_line.insert(
                span_index, Span::begin(ll.upper, ll.upper_id, ll.lower, current)
            );
            self.sweep_line[span_index+1].left.upper = r2.upper;
            self.sweep_line[span_index+1].left.lower = r2.lower;
            self.sweep_line[span_index+1].left.merge = false;

            self.insert_edge(
                span_index, Side::Right,
                Edge { upper: current, lower: left }, id,
                test_intersections
            );
            self.insert_edge(
                span_index+1, Side::Left,
                Edge { upper: current, lower: right }, id,
                test_intersections
            );
        }
    }

    fn on_end_event(&mut self, vertex: Vertex, span_index: usize) {
        if self.log {
            println!(" ++++++ End event {} (span {})", vertex.id.handle, span_index);
        }

        if self.sweep_line[span_index].right.merge {
            //   \ /
            //  \ x   <-- merge vertex
            //   \:/
            //    x   <-- current vertex
            self.end_span(span_index, vertex);
        }

        self.end_span(span_index, vertex);
    }

    fn on_merge_event(&mut self, vertex: Vertex, span_index: usize) {
        if self.log {
            println!(" ++++++ Merge event {} (span {})", vertex.id.handle, span_index);
        }

        assert!(span_index < self.sweep_line.len()-1);

        if self.sweep_line[span_index].right.merge {
            //     / \ /
            //  \ / .-x    <-- merge vertex
            //   x-'      <-- current merge vertex
            self.sweep_line[span_index+2].set_lower_vertex(vertex.position, Side::Left);
            self.end_span(span_index+1, vertex);
        }

        self.sweep_line[span_index].merge_vertex(vertex.position, vertex.id, Side::Right);
        self.sweep_line[span_index+1].merge_vertex(vertex.position, vertex.id, Side::Left);
    }

    fn insert_edge(&mut self,
        span_index: usize, side: Side,
        edge: Edge, id: VertexId,
        test_intersections: bool,
    ) {
        if test_intersections {
            self.check_intersections(span_index, side, edge);
        }
        self.sweep_line[span_index].edge(edge, id, side);
    }

    fn check_intersections(&mut self, span_index: usize, side: Side, edge: Edge) {
        if self.log {
            println!(" -- check for intersections in {} spans", self.sweep_line.len());
        }
        // TODO do the intersection check for all span edges but discard the ones where
        // the intersection point (snapped) is equal to the current position.
        // can also somehow remove the upper edges or mark them before running intersections.
        // ideally the code would laid out in a way that lets us batch intersection computations
        // with SIMD.
        for idx in 0..self.sweep_line.len() {
            if idx != span_index || side.is_right() {
                let left = self.sweep_line[idx].left.clone();
                self.test_intersection(&left, edge);
            }
            if idx != span_index || side.is_left() {
                let right = self.sweep_line[idx].right.clone();
                self.test_intersection(&right, edge);
            }
        }
    }

    fn test_intersection(&mut self, span_edge: &SpanEdge, edge: Edge) {

        if !span_edge.merge {
            if let Some(intersection) = segment_intersection(
                edge.upper, edge.lower,
                span_edge.upper, span_edge.lower,
            ) {
                let evt = Intersection {
                    point: intersection,
                    a_down: span_edge.lower,
                    b_down: edge.lower,
                };

                if self.log {
                    println!(" -- found an intersection at {:?}", intersection);
                    println!("    | {:?}->{:?} x {:?}->{:?}",
                        edge.upper.tuple(), edge.lower.tuple(),
                        span_edge.upper.tuple(), span_edge.lower.tuple(),
                    );
                }

                self.intersections.push(evt);
                self.intersections.sort_by(|a, b| { compare_positions(a.point, b.point) });
            }
        }
    }

    fn on_intersection_event(&mut self, intersection: &Intersection) {
        if self.log {
            println!(" ----- On intersection event at {:?} ", intersection.point);
        }
        self.below.push(EdgeBelow {
            lower: intersection.a_down,
            angle: -vec2(1.0, 0.0).directed_angle(intersection.a_down - intersection.point),
            test_intersections: false,
        });
        self.below.push(EdgeBelow {
            lower: intersection.b_down,
            angle: -vec2(1.0, 0.0).directed_angle(intersection.b_down - intersection.point),
            test_intersections: false,
        });
    }

    fn end_span(&mut self, span_index: usize, vertex: Vertex) {
        if self.log {
            println!("     end span {} (vertex: {})", span_index, vertex.id.handle);
        }
        {
            let tess = &mut self.sweep_line[span_index].monotone_tesselator;
            tess.end(vertex.position, vertex.id);
            tess.flush(self.output);
        }
        self.sweep_line.remove(span_index);
    }

    fn check_sl(&self, current: Vec2) {
        for span in &self.sweep_line {
            if !span.left.merge {
                assert!(!is_below(current, snap_v(span.left.lower)));
                assert!(!is_below(snap_v(span.left.upper), snap_v(span.left.lower)));
            }
            if !span.right.merge {
                assert!(!is_below(current, snap_v(span.right.lower)));
                assert!(!is_below(snap_v(span.right.upper), snap_v(span.right.lower)));
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
            print!("| l:{:?} ", snap_v(span.left.upper).tuple());
            print!(" r:{:?} |", snap_v(span.right.upper).tuple());
        }
        println!("]");
        print!("     [");
        for span in &self.sweep_line {
            if span.left.merge {
                print!("| l:   <merge>           ");
            } else {
                print!("| l:{:?} ", snap_v(span.left.lower).tuple());
            }
            if span.right.merge {
                print!(" r:   <merge>           |");
            } else {
                print!(" r:{:?} |", snap_v(span.right.lower).tuple());
            }
        }
        println!("]\n");
    }

    fn print_sl_at(&self, y: f32) {
        print!("\nat y={}  sl: [", y);
        for span in &self.sweep_line {
            if span.left.merge {
                print!("| l:<merge> ");
            } else {
                let lx = line_horizontal_intersection(span.left.upper, span.left.lower, y);
                print!("| l:{} ", lx);
            }
            if span.right.merge {
                print!(" r:<merge> |");
            } else {
                let rx = line_horizontal_intersection(span.right.upper, span.right.lower, y);
                print!(" r:{} |", rx);
            }
        }
        println!("]\n");
    }
}

fn compare_positions(a: Vec2, b: Vec2) -> Ordering {
    if a.y > b.y { return Ordering::Greater; }
    if a.y < b.y { return Ordering::Less; }
    if a.x > b.x { return Ordering::Greater; }
    if a.x < b.x { return Ordering::Less; }
    return Ordering::Equal;
}

fn test_span_side(span_edge: &SpanEdge, position: Vec2) -> bool {
    if span_edge.merge {
        return false;
    }
    if snap_v(span_edge.lower) == position {
        //println!(" ++++++ current:{}  span:{}", position.x, position.x);
        return true;
    }
    let x = line_horizontal_intersection(span_edge.upper, span_edge.lower, position.y);
    //println!(" ++++++ current:{}  span:{}", position.x, snap(x));
    return position.x < snap(x);
}

fn test_span_touches(span_edge: &SpanEdge, position: Vec2) -> bool {
    if span_edge.merge {
        return false;
    }
    let x = line_horizontal_intersection(span_edge.upper, span_edge.lower, position.y);
    //println!("          test current:{:?}  span:{:?}", position.x, snap(x));

    if snap_v(span_edge.lower) == position {
        return true;
    }

    return position.x == snap(x);
}

fn snap(v: f32) -> f32 { ((v * 100000.0) as i64) as f32 / 100000.0 }
fn snap_v(v: Vec2) -> Vec2 { Vec2::new(snap(v.x), snap(v.y)) }

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

        //println!(" monotone vertex: {:?} (was {:?})", current.pos.tuple(), self.previous.pos.tuple());
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

                if (current.pos - b.pos).directed_angle(a.pos - b.pos) <= PI {
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
        self.vertex(pos, id, side);
        //println!(" -- end monotone tesselator with stack {}", self.stack.len());
        self.stack.clear();
    }

    fn push_triangle(&mut self, a: &MonotoneVertex, b: &MonotoneVertex, c: &MonotoneVertex) {
        //println!(" #### triangle {} {} {}", a.id.handle, b.id.handle, c.id.handle);

        if (c.pos - b.pos).directed_angle(a.pos - b.pos) <= PI {
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

/// Parameters for the tesselator.
pub struct TesselatorOptions {
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

impl TesselatorOptions {
    pub fn new() -> TesselatorOptions {
        TesselatorOptions {
            vertex_aa: false,
            flatten_curves: true,
        }
    }

    pub fn with_vertex_aa(mut self) -> TesselatorOptions {
        self.vertex_aa = true;
        return self;
    }

}

pub fn tesselate_path_fill<'l, Output: VertexBufferBuilder<Vec2>>(
    path: PathSlice<'l>,
    options: &TesselatorOptions,
    output: &mut Output
) -> Result<(), ()> {
    if options.vertex_aa {
        println!("[tesselate_path_fill] Vertex anti-aliasing not implemented");
    }

    output.begin_geometry();
    let result = {
        let mut tess = Tesselator::new(output);
        tess.tesselate(path)
    };
    output.end_geometry();
    return result;
}

pub fn tesselate_path_stroke<Output: VertexBufferBuilder<Vec2>>(
    path: PathSlice,
    thickness: f32,
    options: &TesselatorOptions,
    output: &mut Output
) -> (Range, Range) {
    if options.vertex_aa {
        println!("[tesselate_path_stroke] Vertex anti-aliasing not implemented");
    }
    output.begin_geometry();
    for p in path.path_ids() {
        tesselate_sub_path_stroke(path.sub_path(p), thickness, output);
    }
    return output.end_geometry();
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

    let mut prev_v1: Vec2 = Default::default();
    let mut prev_v2: Vec2 = Default::default();
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
                    px, n1.directed_angle(n2), px.directed_angle2(prev, next),
                    prev.tuple(), px.tuple(), next.tuple(),
                );
                px + (px - prev) * amount / (px - prev).length()
            }
        }
    };
    return inter;
}

#[cfg(test)]
fn tesselate(path: PathSlice, log: bool) -> Result<usize, ()> {
    let mut buffers: VertexBuffers<Vec2> = VertexBuffers::new();
    {
        let mut vertex_builder = simple_vertex_builder(&mut buffers);
        let mut tess = Tesselator::new(&mut vertex_builder);
        if log {
            tess.enable_logging();
        }
        try!{ tess.tesselate(path) };
    }
    return Ok(buffers.indices.len()/3);
}

#[cfg(test)]
fn test_path(path: PathSlice, expected_triangle_count: Option<usize>) {
    let res = ::std::panic::catch_unwind(|| { tesselate(path, false) });

    if let Ok(Ok(num_triangles)) = res {
        if let Some(actual_triangles) = expected_triangle_count {
            assert_eq!(actual_triangles, num_triangles);
        }
        return;
    }

    tesselate(path, true).unwrap();

    ::lyon_extra::debugging::find_reduced_test_case(path, &|path: Path|{
        return tesselate(path.as_slice(), false).is_err();
    });
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
fn test_tesselator_simple_triangle() {
    let mut path = flattened_path_builder();
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(1.0, 1.0));
    path.line_to(vec2(0.0, 1.0));
    path.close();

    test_path_with_rotations(path.build(), 0.01, Some(1));
}

#[test]
fn test_tesselator_simple_monotone() {
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
fn test_tesselator_simple_split() {
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
fn test_tesselator_simple_merge_split() {
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
fn test_tesselator_simple_aligned() {
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
fn test_tesselator_simple_1() {
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
fn test_tesselator_simple_2() {
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
fn test_tesselator_hole_1() {
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
fn test_tesselator_degenerate_empty() {
    test_path(Path::new().as_slice(), Some(0));
}

#[test]
fn test_tesselator_degenerate_same_position() {
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
#[ignore]
fn test_tesselator_auto_intersection_type1() {
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
#[ignore]
fn test_tesselator_auto_intersection_type2() {
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
#[ignore]
fn test_tesselator_auto_intersection_multi() {
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
fn test_tesselator_rust_logo() {
    let mut path = flattened_path_builder();

    ::lyon_extra::rust_logo::build_logo_path(&mut path);

    test_path_with_rotations(path.build(), 0.011, None);
}

#[test]
#[ignore]
fn test_tesselator_rust_logo_with_intersection() {
    let mut path = flattened_path_builder();

    ::lyon_extra::rust_logo::build_logo_path(&mut path);

    path.move_to(vec2(10.0, 30.0));
    path.line_to(vec2(130.0, 30.0));
    path.line_to(vec2(130.0, 60.0));
    path.line_to(vec2(10.0, 60.0));
    path.close();

    let path = path.build();

    test_path_with_rotation(&path, 1.1439997, None);
}

#[test]
#[ignore]
fn test_tesselator_split_with_intersections() {
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
#[ignore]
fn test_colinear_1() {
    let mut builder = flattened_path_builder();
    builder.move_to(vec2(20.0, 150.0));
    builder.line_to(vec2(80.0, 150.0));
    builder.close();

    let path = builder.build();

    test_path_with_rotations(path, 0.01, None);
}

#[test]
#[ignore]
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
#[ignore] // TODO
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
fn test_coincident_simple_rotated_failing() {
    println!("--------------");
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
#[ignore] // TODO
fn test_identical_square_failing() {
    // Two identiacal sub paths. It's a pretty much the worst type of input for
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
