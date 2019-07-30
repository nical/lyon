use crate::{FillOptions, Side};
use crate::geom::math::*;
use crate::geom::{LineSegment};
//use crate::geom::cubic_to_quadratic::cubic_to_monotonic_quadratics;
use crate::geometry_builder::{GeometryBuilder, VertexId};
use crate::path_fill::MonotoneTessellator;
//use crate::path::builder::*;
use crate::path::{PathEvent, Path, PathSlice, FillRule, Transition, EndpointId, CtrlPointId};
use std::{u32, f32};
use std::cmp::Ordering;
use std::ops::Range;
use std::env;
use std::mem::swap;

#[cfg(feature="debugger")]
use crate::debugger::*;
#[cfg(feature="debugger")]
use crate::path_fill::dbg;

pub type Vertex = Point;

#[cfg(not(feature = "release"))]
macro_rules! tess_log {
    ($obj:ident, $fmt:expr) => (
        if $obj.log {
            println!($fmt);
        }
    );
    ($obj:ident, $fmt:expr, $($arg:tt)*) => (
        if $obj.log {
            println!($fmt, $($arg)*);
        }
    );
}

#[cfg(feature = "release")]
macro_rules! tess_log {
    ($obj:ident, $fmt:expr) => ();
    ($obj:ident, $fmt:expr, $($arg:tt)*) => ();
}

#[derive(Copy, Clone, Debug)]
struct WindingState {
    span_index: SpanIdx,
    number: i16,
    transition: Transition,
}

impl WindingState {
    fn update(&mut self, fill_rule: FillRule, edge_winding: i16) {
        let prev_winding_number = self.number;
        self.number += edge_winding;
        self.transition = fill_rule.transition(prev_winding_number, self.number);
        if self.transition == Transition::In {
            self.span_index += 1;
        }
    }
}

struct ActiveEdge {
    min_x: f32,
    max_x: f32,

    from: Point,
    to: Point,
    ctrl: Point,

    winding: i16,
    is_merge: bool,

    from_id: VertexId,
    src_edge: usize,

    // Only valid when sorting the active edges.
    sort_x: f32,
}

struct ActiveEdges {
    edges: Vec<ActiveEdge>,
}

type SpanIdx = i32;

struct Span {
    tess: MonotoneTessellator,
    remove: bool,
}

struct Spans {
    spans: Vec<Span>,
}

impl Spans {
    fn begin_span(&mut self, span_idx: SpanIdx, position: &Point, vertex: VertexId) {
        let idx = span_idx as usize;
        self.spans.insert(
            idx,
            Span {
                tess: MonotoneTessellator::new().begin(*position, vertex),
                remove: false,
            }
        );
    }

    fn end_span(
        &mut self,
        span_idx: SpanIdx,
        position: &Point,
        id: VertexId,
        output: &mut dyn GeometryBuilder<Vertex>,
    ) {
        let idx = span_idx as usize;

        let span = &mut self.spans[idx];
        span.remove = true;
        span.tess.end(*position, id);
        span.tess.flush_experimental(output);
    }

    fn split_span(
        &mut self,
        span_idx: SpanIdx,
        split_position: &Point,
        split_id: VertexId,
        a_position: &Point,
        a_id: VertexId
    ) {
        let idx = span_idx as usize;

        //        /....
        // a --> x.....
        //      /.\....
        //     /...x... <-- current split vertex
        //    /.../ \..

        self.spans.insert(
            idx,
            Span {
                tess: MonotoneTessellator::new().begin(*a_position, a_id),
                remove: false,
            }
        );

        self.spans[idx].tess.vertex(*split_position, split_id, Side::Right);
        self.spans[idx + 1].tess.vertex(*split_position, split_id, Side::Left);
    }

    fn merge_spans(
        &mut self,
        span_idx: SpanIdx,
        current_position: &Point,
        current_vertex: VertexId,
        merge_position: &Point,
        merge_vertex: VertexId,
        output: &mut dyn GeometryBuilder<Vertex>,
    ) {
        //  \...\ /.
        //   \...x..  <-- merge vertex
        //    \./...  <-- active_edge
        //     x....  <-- current vertex

        let idx = span_idx as usize;
        if self.spans.len() <= idx + 1 {
            // TODO: we can only run into this if the order of the sweep line
            // is invalid. Need to re-sort it.
            return
        }

        self.spans[idx].tess.vertex(
            *merge_position,
            merge_vertex,
            Side::Right,
        );

        self.spans[idx + 1].tess.vertex(
            *merge_position,
            merge_vertex,
            Side::Left,
        );

        self.end_span(
            span_idx,
            current_position,
            current_vertex,
            output,
        );
    }

    fn cleanup_spans(&mut self) {
        // Get rid of the spans that were marked for removal.
        self.spans.retain(|span|{ !span.remove });
    }
}

#[derive(Debug)]
struct PendingEdge {
    to: Point,
    ctrl: Point,

    angle: f32,

    // Index in events.edge_data
    src_edge: usize,

    winding: i16,
}

impl ActiveEdge {
    fn solve_x_for_y(&self, y: f32) -> f32 {
        // TODO: curves.
        LineSegment {
            from: self.from,
            to: self.to,
        }.solve_x_for_y(y)
    }
}

pub struct FillTessellator {
    current_position: Point,
    active: ActiveEdges,
    edges_below: Vec<PendingEdge>,
    fill_rule: FillRule,
    fill: Spans,
    log: bool,
    assume_no_intersection: bool,

    events: EventQueue,

    #[cfg(feature="debugger")]
    debugger: Option<Box<dyn Debugger2D>>,
}


impl FillTessellator {
    pub fn new() -> Self {
        FillTessellator {
            current_position: point(f32::MIN, f32::MIN),
            active: ActiveEdges {
                edges: Vec::new(),
            },
            edges_below: Vec::new(),
            fill_rule: FillRule::EvenOdd,
            fill: Spans {
                spans: Vec::new(),
            },
            log: env::var("LYON_FORCE_LOGGING").is_ok(),
            assume_no_intersection: false,

            events: EventQueue::new(),

            #[cfg(feature="debugger")]
            debugger: None,
        }
    }

    pub fn tessellate_path(
        &mut self,
        path: &Path,
        options: &FillOptions,
        builder: &mut dyn GeometryBuilder<Vertex>
    ) {
        self.fill_rule = options.fill_rule;

        let mut tx_builder = EventQueueBuilder::with_capacity(128);
        tx_builder.set_path(path.as_slice());
        self.events = tx_builder.build();

        builder.begin_geometry();

        self.tessellator_loop(path, builder);

        //assert!(self.active.edges.is_empty());
        //assert!(self.fill.spans.is_empty());
        // TODO: go over the remaining spans and end them.

        builder.end_geometry();

        tess_log!(self, "\n ***************** \n");
    }

    pub fn enable_logging(&mut self) {
        self.log = true;
    }

    fn tessellator_loop(
        &mut self,
        path: &Path,
        output: &mut dyn GeometryBuilder<Vertex>
    ) {
        let mut prev_position = point(-1000000000.0, -1000000000.0);
        let mut current_event = self.events.first_id();
        while self.events.valid_id(current_event) {
            self.current_position = self.events.position(current_event);
            debug_assert!(is_after(self.current_position, prev_position));
            prev_position = self.current_position;

            tess_log!(self, "\n --- event #{}", current_event);

            let src = VertexSourceIterator {
                events: &self.events,
                id: current_event,
            };

            let vertex_id = output.add_vertex_exp(self.current_position, src).unwrap();

            let mut current_sibling = current_event;
            while self.events.valid_id(current_sibling) {
                let edge = &self.events.edge_data[current_sibling];
                // We insert "fake" edges when there are end events
                // to make sure we process that vertex even if it has
                // no edge below.
                if edge.to == EndpointId::INVALID {
                    current_sibling = self.events.next_sibling_id(current_sibling);
                    continue;
                }
                let to = path[edge.to];
                let ctrl = if edge.ctrl != CtrlPointId::INVALID {
                    // path[edge.ctrl]
                    point(f32::NAN, f32::NAN) // TODO
                } else {
                    point(f32::NAN, f32::NAN)
                };
                self.edges_below.push(PendingEdge {
                    ctrl,
                    to,
                    angle: (to - self.current_position).angle_from_x_axis().radians,
                    src_edge: current_sibling,
                    winding: edge.winding,
                });

                current_sibling = self.events.next_sibling_id(current_sibling);
            }

            if !self.process_events(vertex_id, output) {
                // Something went wrong, attempt to salvage the state of the sweep
                // line and try again.
                self.recover_from_error();
                assert!(self.process_events(vertex_id, output));
            }

            current_event = self.events.next_id(current_event);
        }
    }

    fn process_events(
        &mut self,
        current_vertex: VertexId,
        output: &mut dyn GeometryBuilder<Vertex>,
    ) -> bool {
        debug_assert!(!self.current_position.x.is_nan() && !self.current_position.y.is_nan());

        let current_x = self.current_position.x;

        tess_log!(self, "\n --- events at [{},{}] {:?}         {} edges below",
            current_x, self.current_position.y,
            current_vertex,
            self.edges_below.len(),
        );

        // The span index starts at -1 so that entering the first span (of index 0) increments
        // it to zero.
        let mut winding = WindingState {
            span_index: -1,
            number: 0,
            transition: Transition::None,
        };

        let mut connecting_edges = false;
        let mut edges_to_split: Vec<usize> = Vec::new();

        // First go through the sweep line and visit all edges that end at the
        // current position.

        let mut active_edge_idx = 0;
        // Iterate over edges before the current point.
        for active_edge in &self.active.edges {
            if active_edge.is_merge {
                // \.....\ /...../
                //  \.....x...../   <--- merge vertex
                //   \....:..../
                // ---\---:---/----  <-- sweep line
                //     \..:../

                // An unresolved merge vertex implies the left and right spans are
                // adjacent and there is no transition between the two which means
                // we need to bump the span index manually.
                winding.span_index += 1;
                active_edge_idx += 1;

                continue;
            }

            let egde_is_before_current_point = if active_edge.max_x < current_x {
                true
            } else if active_edge.min_x > current_x {
                false
            } else if points_are_equal(self.current_position, active_edge.to) {
                // We just found our first edge that connects with the current point.
                // We might find other ones in the next loop.
                connecting_edges = true;
                false
            } else {
                let ex = active_edge.solve_x_for_y(self.current_position.y);
                if ex > current_x {
                    false
                } else if ex == current_x {
                    connecting_edges = true;
                    false
                } else {
                    true
                }
            };

            if !egde_is_before_current_point {
                break;
            }

            winding.update(self.fill_rule, active_edge.winding);
            active_edge_idx += 1;

            tess_log!(self, " > {:?}", winding.transition);
        }

        let mut above_start = active_edge_idx;
        let winding_before_point = winding.clone();
        let mut is_first_transition = true;
        let mut prev_transition_in = None;
        let mut pending_merge = None;
        let mut pending_right = None;
        let mut merges_to_resolve: Vec<(SpanIdx, usize)> = Vec::new();
        let mut spans_to_end = Vec::new();

        tess_log!(self, "connecting_edges {} | {}", connecting_edges, active_edge_idx);
        if connecting_edges {
            // Iterate over edges connecting with the current point.
            for active_edge in &mut self.active.edges[active_edge_idx..] {
                if active_edge.is_merge {
                    tess_log!(self, "merge to resolve {}", active_edge_idx);

                    merges_to_resolve.push((winding.span_index, active_edge_idx));
                    active_edge.to = self.current_position;
                    winding.span_index += 1;
                    active_edge_idx += 1;

                    continue;
                }

                if !points_are_equal(self.current_position, active_edge.to) {
                    // This edge does not connect with the current position.
                    // Before breaking out of this loop, check for errors.
                    let mut is_error = active_edge.max_x < current_x;
                    let mut is_on_edge = false;

                    if !is_error
                        && active_edge.max_x >= current_x
                        && active_edge.min_x <= current_x {

                        let ex = active_edge.solve_x_for_y(self.current_position.y);
                        tess_log!(self, "ex = {:?}", ex);
                        if ex == current_x {
                            tess_log!(self, " -- vertex on an edge!");
                            is_on_edge = true;
                            edges_to_split.push(active_edge_idx);
                        } else if ex < current_x {
                            is_error = true;
                        }
                    }

                    if is_error {
                        // Ran into an edge that is before the current point.
                        tess_log!(self, "error A");
                        return false;
                    }

                    if !is_on_edge {
                        break;
                    }
                }

                winding.update(self.fill_rule, active_edge.winding);
                tess_log!(self, " x {:?}", winding.transition);

                match (winding.transition, is_first_transition) {
                    (Transition::In, _) => {
                        prev_transition_in = Some(active_edge_idx);
                    }
                    (Transition::Out, true) => {
                        if self.edges_below.is_empty() {
                            // Merge event.
                            pending_merge = Some(active_edge_idx);
                        } else {
                            // Right event.
                            pending_right = Some(active_edge_idx);
                        }
                    }
                    (Transition::Out, false) => {
                        let in_idx = prev_transition_in.unwrap();
                        tess_log!(self, " ** end ** edges: [{},{}] span: {}",
                            in_idx, active_edge_idx,
                            winding.span_index
                        );

                        if winding.span_index < self.fill.spans.len() as i32 {
                            spans_to_end.push(winding.span_index);
                            winding.span_index += 1; // not sure
                        } else {
                            tess_log!(self, "error B");
                            //return false; // TODO
                        }
                    }
                    (Transition::None, _) => {}
                }

                if winding.transition != Transition::None {
                    is_first_transition = false;
                }

                active_edge_idx += 1;
            }
        }

        let mut above_end = active_edge_idx;

        // Now Iterate over edges after the current point.
        // We only do this to detect errors.
        for active_edge in &mut self.active.edges[active_edge_idx..] {
            if active_edge.is_merge {
                continue;
            }

            if active_edge.max_x < current_x {
                tess_log!(self, "error C");
                return false;
            }

            if points_are_equal(self.current_position, active_edge.to) {
                tess_log!(self, "error D");
                return false;
            }

            if active_edge.min_x < current_x
                && active_edge.solve_x_for_y(self.current_position.y) < current_x {
                tess_log!(self, "error E");
                return false;
            }
        }

        for (span_index, edge_idx) in merges_to_resolve {
            //  \...\ /.
            //   \...x..  <-- merge vertex
            //    \./...  <-- active_edge
            //     x....  <-- current vertex
            let active_edge: &mut ActiveEdge = &mut self.active.edges[edge_idx];
            let merge_vertex: VertexId = active_edge.from_id;
            let merge_position = active_edge.from;
            //println!("merge vertex {:?} -> {:?}", merge_vertex, current_vertex);
            self.fill.merge_spans(
                span_index,
                &self.current_position,
                current_vertex,
                &merge_position,
                merge_vertex,
                output,
            );

            active_edge.is_merge = false;

            tess_log!(self, " Resolve merge event {} at {:?} ending span {}", edge_idx, active_edge.to, span_index);
            #[cfg(feature="debugger")]
            debugger_monotone_split(&self.debugger, &merge_position, &self.current_position);
        }

        for span_index in spans_to_end {
            self.fill.end_span(
                span_index,
                &self.current_position,
                current_vertex,
                output,
            );
        }

        self.fill.cleanup_spans();

        for edge_idx in edges_to_split {
            let active_edge: &mut ActiveEdge = &mut self.active.edges[edge_idx];
            let to = active_edge.to;

            self.edges_below.push(PendingEdge {
                ctrl: point(f32::NAN, f32::NAN),
                to,

                angle: (to - self.current_position).angle_from_x_axis().radians,

                src_edge: active_edge.src_edge,

                winding: active_edge.winding,
            });

            active_edge.to = self.current_position;
        }

        winding = winding_before_point;

        tess_log!(self, "connecting edges: {}..{} {:?}", above_start, above_end, winding.transition);
        tess_log!(self, "winding state before point: {:?}", winding);

        self.sort_edges_below();

        if let Some(in_idx) = pending_merge {
            // Merge event.
            //
            //  ...\   /...
            //  ....\ /....
            //  .....x.....
            //

            tess_log!(self, " ** merge ** edges: [{},{}] span: {}",
                in_idx, above_end - 1,
                winding.span_index
            );

            let e = &mut self.active.edges[in_idx];
            e.is_merge = true;
            e.from = e.to;
            e.ctrl = e.to;
            e.min_x = e.to.x;
            e.max_x = e.to.x;
            e.winding = 0;
            e.from_id = current_vertex;
        }

        // The range of pending edges below the current vertex to look at in the
        // last loop (not always the full range if we process split events).
        let mut below = 0..self.edges_below.len();

        if self.fill_rule.is_in(winding.number)
            && above_start == above_end
            && self.edges_below.len() >= 2 {

            // Split event.
            //
            //  ...........
            //  .....x.....
            //  ..../ \....
            //  .../   \...
            //

            let edge_above = above_start - 1;

            let upper_pos = self.active.edges[edge_above].from;
            let upper_id = self.active.edges[edge_above].from_id;
            tess_log!(self, " ** split ** edge {} span: {} upper {:?}", edge_above, winding.span_index, upper_pos);

            if self.active.edges[edge_above].is_merge {
                // Split vertex under a merge vertex
                //
                //  ...\ /...
                //  ....x....   <-- merge vertex (upper)
                //  ....:....
                //  ----x----   <-- current split vertex
                //  .../ \...
                //
                tess_log!(self, "   -> merge+split");
                let span_index = winding.span_index as usize;

                self.fill.spans[span_index - 1].tess.vertex(
                    upper_pos,
                    upper_id,
                    Side::Right,
                );
                self.fill.spans[span_index - 1].tess.vertex(
                    self.current_position,
                    current_vertex,
                    Side::Right,
                );

                self.fill.spans[span_index].tess.vertex(
                    upper_pos,
                    upper_id,
                    Side::Left,
                );
                self.fill.spans[span_index].tess.vertex(
                    self.current_position,
                    current_vertex,
                    Side::Left,
                );

                self.active.edges.remove(edge_above);
                above_start -= 1;
                above_end -= 1;
            } else {
                self.fill.split_span(
                    winding.span_index,
                    &self.current_position,
                    current_vertex,
                    &upper_pos,
                    upper_id,
                );
            }

            #[cfg(feature="debugger")]
            debugger_monotone_split(&self.debugger, &upper_pos, &self.current_position);

            winding.update(self.fill_rule, self.edges_below[0].winding);

            below.start += 1;
            below.end -= 1;
        }

        // Go through the edges starting at the current point and emit
        // start events.

        let mut prev_transition_in = None;

        for i in below {
            let pending_edge = &self.edges_below[i];

            winding.update(self.fill_rule, pending_edge.winding);

            tess_log!(self, "edge below {}: {:?} span {}", i, winding.transition, winding.span_index);

            if let Some(idx) = pending_right {
                // Right event.
                //
                //  ..\
                //  ...x
                //  ../
                //
                debug_assert!(winding.transition == Transition::Out);
                tess_log!(self, " ** right ** edge: {} span: {}", idx, winding.span_index);

                self.fill.spans[winding.span_index as usize].tess.vertex(
                    self.current_position,
                    current_vertex,
                    Side::Right,
                );

                pending_right = None;

                continue;
            }

            match winding.transition {
                Transition::In => {
                    if i == self.edges_below.len() - 1 {
                        // Left event.
                        //
                        //     /...
                        //    x....
                        //     \...
                        //
                        tess_log!(self, " ** left ** edge {} span: {}", above_start, winding.span_index);

                        self.fill.spans[winding.span_index as usize].tess.vertex(
                            self.current_position,
                            current_vertex,
                            Side::Left,
                        );
                    } else {
                        prev_transition_in = Some(i);
                    }
                }
                Transition::Out => {
                    if let Some(in_idx) = prev_transition_in {

                        tess_log!(self, " ** start ** edges: [{},{}] span: {}", in_idx, i, winding.span_index);

                        // Start event.
                        //
                        //      x
                        //     /.\
                        //    /...\
                        //

                        // TODO: if this is an intersection we must create a vertex
                        // and use it instead of the upper endpoint of the edge.
                        // TODO: we should use from_id but right now it points to the
                        // vertex in the path object and not the one we added with
                        // add_vertex
                        //let vertex = self.edges_below[in_idx].from_id;
                        let vertex = current_vertex;
                        tess_log!(self, " begin span {} ({})", winding.span_index, self.fill.spans.len());
                        self.fill.begin_span(
                            winding.span_index,
                            &self.current_position,
                            vertex
                        );
                    }
                }
                Transition::None => {
                    tess_log!(self, "(transition: none)");
                }
            }
        }

        self.update_active_edges(above_start..above_end, current_vertex);

        tess_log!(self, "sweep line: {}", self.active.edges.len());
        for e in &self.active.edges {
            if e.is_merge {
                tess_log!(self, "| (merge) {}", e.from);
            } else {
                tess_log!(self, "| {} -> {}", e.from, e.to);
            }
        }
        tess_log!(self, "spans: {}", self.fill.spans.len());

        true
    }

    fn update_active_edges(&mut self, above: Range<usize>, current_vertex_id: VertexId) {
        // Remove all edges from the "above" range except merge
        // vertices.
        tess_log!(self, " remove {} edges ({}..{})", above.end - above.start, above.start, above.end);
        let mut rm_index = above.start;
        for _ in 0..(above.end - above.start) {
            if self.active.edges[rm_index].is_merge {
                rm_index += 1
            } else {
                self.active.edges.remove(rm_index);
            }
        }

        if !self.assume_no_intersection {
            self.handle_intersections();
        }

        // Insert the pending edges.
        let from = self.current_position;
        let first_edge_below = above.start;
        for (i, edge) in self.edges_below.drain(..).enumerate() {
            let idx = first_edge_below + i;
            self.active.edges.insert(idx, ActiveEdge {
                min_x: from.x.min(edge.to.x),
                max_x: from.x.max(edge.to.x),
                sort_x: 0.0,
                from,
                to: edge.to,
                ctrl: edge.ctrl,
                winding: edge.winding,
                is_merge: false,
                from_id: current_vertex_id,
                src_edge: edge.src_edge,
            });
        }
    }

    fn handle_intersections(&mut self) {
        for edge_below in &mut self.edges_below {
            let below_min_x = self.current_position.x.min(edge_below.to.x);
            let below_max_x = self.current_position.x.max(edge_below.to.x);

            let below_segment = LineSegment {
                from: self.current_position.to_f64(),
                to: edge_below.to.to_f64(),
            };

            let epsilon = 0.001;
            let mut tb_min = 1.0 - epsilon;
            let mut intersection = None;
            for (i, active_edge) in self.active.edges.iter().enumerate() {
                if active_edge.is_merge || below_min_x > active_edge.max_x {
                    continue;
                }
                if below_max_x < active_edge.min_x {
                    break;
                }

                let active_segment = LineSegment {
                    from: active_edge.from.to_f64(),
                    to: active_edge.to.to_f64(),
                };

                if let Some((ta, tb)) = active_segment.intersection_t(&below_segment) {
                    if tb < tb_min && tb > epsilon && ta > epsilon && ta < 1.0 - epsilon {
                        // we only want the closest intersection;
                        tb_min = tb;
                        intersection = Some((ta, tb, i));
                    }
                }
            }

            if let Some((ta, tb, active_edge_idx)) = intersection {
                let mut intersection_position = below_segment.sample(tb).to_f32();

                tess_log!(self, "-> intersection at: {:?} : {:?}", intersection_position, intersection);
                tess_log!(self, "   from {:?}->{:?} and {:?}->{:?}",
                    self.active.edges[active_edge_idx].from,
                    self.active.edges[active_edge_idx].to,
                    self.current_position,
                    edge_below.to,
                );

                if intersection_position.y < self.current_position.y {
                    tess_log!(self, "fixup the intersection because of y coordinate");
                    intersection_position.y = self.current_position.y + std::f32::EPSILON; // TODO
                } else if intersection_position.y == self.current_position.y
                    && intersection_position.x < self.current_position.x {
                    tess_log!(self, "fixup the intersection because of x coordinate");
                    intersection_position.x = self.current_position.x;
                }

                let active_edge = &mut self.active.edges[active_edge_idx];

                let a_src_edge_data = self.events.edge_data[active_edge.src_edge].clone();
                let b_src_edge_data = self.events.edge_data[edge_below.src_edge].clone();

                use arrayvec::ArrayVec;
                let mut intersections: ArrayVec<[EdgeData; 2]> = ArrayVec::new();

                if ta > 0.0 && ta < 1.0 {
                    if !is_after(active_edge.to, intersection_position) {
                    }
                    debug_assert!(is_after(active_edge.to, intersection_position));
                    active_edge.to = intersection_position;
                    intersections.push(EdgeData {
                        range: ta as f32 .. a_src_edge_data.range.end,
                        .. a_src_edge_data
                    });
                }

                if tb > 0.0 && tb < 1.0 {
                    debug_assert!(is_after(edge_below.to, intersection_position));
                    edge_below.to = intersection_position;
                    intersections.push(EdgeData {
                        range: tb as f32..b_src_edge_data.range.end,
                        .. b_src_edge_data
                    });
                }

                if let Some(edge_data) = intersections.pop() {
                    let evt_idx = self.events.insert_sorted(intersection_position, edge_data);

                    if let Some(edge_data) = intersections.pop() {
                        self.events.insert_sibling(
                            evt_idx,
                            intersection_position,
                            edge_data,
                        );
                    }
                }
            }
        }
    }

    fn sort_active_edges(&mut self) {
        // Merge edges are a little subtle when it comes to sorting.
        // They are points rather than edges and the best we can do is
        // keep their relative ordering with their previous or next edge.

        let y = self.current_position.y;

        let mut prev_x = f32::NAN;
        for edge in &mut self.active.edges {
            if edge.is_merge {
                debug_assert!(!prev_x.is_nan());
                edge.sort_x = prev_x;
            } else {
                let x = if edge.to.y == y {
                    edge.to.x
                } else if edge.from.y == y {
                    edge.from.x
                } else {
                    edge.solve_x_for_y(y)
                };

                edge.sort_x = x;
                prev_x = x;
            }
        }

        self.active.edges.sort_by(|a, b| {
            match a.sort_x.partial_cmp(&b.sort_x).unwrap() {
                Ordering::Less => Ordering::Less,
                Ordering::Greater => Ordering::Greater,
                Ordering::Equal => {
                    match (a.is_merge, b.is_merge) {
                        (false, false) => {
                            let angle_a = (a.to - a.from).angle_from_x_axis().radians;
                            let angle_b = (b.to - b.from).angle_from_x_axis().radians;
                            angle_b.partial_cmp(&angle_a).unwrap_or(Ordering::Equal)
                        }
                        (true, false) => { Ordering::Greater }
                        (false, true) => { Ordering::Less }
                        (true, true) => { Ordering::Equal }
                    }
                }
            }
        });
    }

    fn recover_from_error(&mut self) {
        tess_log!(self, "Attempt to recover from error");

        self.sort_active_edges();

        debug_assert!(self.active.edges.first().map(|e| !e.is_merge).unwrap_or(true));
        // This can only happen if we ignore self-intersections,
        // so we are in a pretty broken state already.
        // There isn't a fully correct solution for this (other
        // than properly detecting self intersections and not
        // getting into this situation), but the rest of the code
        // doesn't deal with merge edges being at the last position
        // so we artificially move them to avoid that.
        // TODO: with self-intersections properly handled it may make more sense
        // to turn this into an assertion.
        let len = self.active.edges.len();
        if len > 1 && self.active.edges[len - 1].is_merge {
            self.active.edges.swap(len - 1, len - 2);
        }

        // The span index starts at -1 so that entering the first span (of index 0) increments
        // it to zero.
        let mut winding = WindingState {
            span_index: -1,
            number: 0,
            transition: Transition::None,
        };

        for edge in &self.active.edges {
            if edge.is_merge {
                winding.span_index += 1;
            } else {
                winding.update(self.fill_rule, edge.winding);
            }

            if winding.span_index > self.fill.spans.len() as i32 {
                self.fill.begin_span(
                    winding.span_index,
                    &edge.from,
                    edge.from_id,
                );
            }
        }

        tess_log!(self, "sweep line: {}", self.active.edges.len());
        for e in &self.active.edges {
            if e.is_merge {
                tess_log!(self, "| (merge) {}", e.from);
            } else {
                tess_log!(self, "| {} -> {}", e.from, e.to);
            }
        }
        tess_log!(self, "spans: {}", self.fill.spans.len());
    }

    fn sort_edges_below(&mut self) {
        // TODO: we'll need a better criterion than the tangent angle with quadratic béziers.
        self.edges_below.sort_by(|a, b| {
            b.angle.partial_cmp(&a.angle).unwrap_or(Ordering::Equal)
        });
    }

    #[cfg(feature="debugger")]
    pub fn install_debugger(&mut self, dbg: Box<dyn Debugger2D>) {
        self.debugger = Some(dbg)
    }

}

#[cfg(feature="debugger")]
fn debugger_monotone_split(debugger: &Option<Box<dyn Debugger2D>>, a: &Point, b: &Point) {
    if let Some(ref dbg) = debugger {
        dbg.edge(a, b, DARK_RED, dbg::MONOTONE_SPLIT);
    }
}

fn points_are_equal(a: Point, b: Point) -> bool {
    // TODO: Use the tolerance threshold.
    a == b
}


fn compare_positions(a: Point, b: Point) -> Ordering {
    if a.y > b.y {
        return Ordering::Greater;
    }
    if a.y < b.y {
        return Ordering::Less;
    }
    if a.x > b.x {
        return Ordering::Greater;
    }
    if a.x < b.x {
        return Ordering::Less;
    }
    return Ordering::Equal;
}

#[inline]
fn is_after(a: Point, b: Point) -> bool {
    a.y > b.y || (a.y == b.y && a.x > b.x)
}

pub struct Event {
    next_sibling: usize,
    next_event: usize,
    position: Point,
}

#[derive(Clone, Debug)]
struct EdgeData {
    from: EndpointId,
    ctrl: CtrlPointId,
    to: EndpointId,
    range: std::ops::Range<f32>,
    winding: i16,
}

pub struct EventQueue {
    events: Vec<Event>,
    edge_data: Vec<EdgeData>,
    first: usize,
    sorted: bool,
}

use std::usize;

impl EventQueue {
    pub fn new() -> Self {
        EventQueue {
            events: Vec::new(),
            edge_data: Vec::new(),
            first: 0,
            sorted: false,
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        EventQueue {
            events: Vec::with_capacity(cap),
            edge_data: Vec::with_capacity(cap),
            first: 0,
            sorted: false,
        }
    }

    pub fn reserve(&mut self, n: usize) {
        self.events.reserve(n);
    }

    pub fn push(&mut self, position: Point) {
        let next_event = self.events.len() + 1;
        self.events.push(Event {
            position,
            next_sibling: usize::MAX,
            next_event,
        });
        self.sorted = false;
    }

    fn insert_sorted(&mut self, position: Point, data: EdgeData) -> usize {
        debug_assert!(self.sorted);

        let idx = self.events.len();
        self.events.push(Event {
            position,
            next_sibling: usize::MAX,
            next_event: usize::MAX,
        });
        self.edge_data.push(data);

        let mut prev = self.first;
        let mut current = self.first;
        while self.valid_id(current) {
            let pos = self.events[current].position;

            if pos == position {
                debug_assert!(prev != current);
                self.events[idx].next_sibling = self.events[current].next_sibling;
                self.events[current].next_sibling = idx;
                break;
            } else if is_after(pos, position) {
                self.events[prev].next_event = idx;
                self.events[idx].next_event = current;
                break;
            }

            prev = current;
            current = self.next_id(current);
        }

        idx
    }

    fn insert_sibling(&mut self, sibling: usize, position: Point, data: EdgeData) {
        let idx = self.events.len();
        let next_sibling = self.events[sibling].next_sibling;

        self.events.push(Event {
            position,
            next_event: usize::MAX,
            next_sibling,
        });

        self.edge_data.push(data);

        self.events[sibling].next_sibling = idx;
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.first = 0;
        self.sorted = false;
    }

    pub fn first_id(&self) -> usize { self.first }

    pub fn next_id(&self, id: usize) -> usize { self.events[id].next_event }

    pub fn next_sibling_id(&self, id: usize) -> usize { self.events[id].next_sibling }

    pub fn valid_id(&self, id: usize) -> bool { id < self.events.len() }

    pub fn position(&self, id: usize) -> Point { self.events[id].position }

    pub fn sort(&mut self) {
        // This is more or less a bubble-sort, the main difference being that elements with the same
        // position are grouped in a "sibling" linked list.

        if self.sorted {
            return;
        }
        self.sorted = true;

        if self.events.len() <= 1 {
            return;
        }

        let mut current = 0;
        let mut prev = 0;
        let mut last = self.events.len() - 1;
        let mut swapped = false;

        #[cfg(test)]
        let mut iter_count = self.events.len() * self.events.len();

        loop {
            #[cfg(test)] {
                assert!(iter_count > 0);
                iter_count -= 1;
            }

            let rewind = current == last ||
                !self.valid_id(current) ||
                !self.valid_id(self.next_id(current));

            if rewind {
                last = prev;
                prev = self.first;
                current = self.first;
                if !swapped || last == self.first {
                    return;
                }
                swapped = false;
            }

            let next = self.next_id(current);
            let a = self.events[current].position;
            let b = self.events[next].position;
            match compare_positions(a, b) {
                Ordering::Less => {
                    // Already ordered.
                    prev = current;
                    current = next;
                }
                Ordering::Greater => {
                    // Need to swap current and next.
                    if prev != current && prev != next {
                        self.events[prev].next_event = next;
                    }
                    if current == self.first {
                        self.first = next;
                    }
                    if next == last {
                        last = current;
                    }
                    let next_next = self.next_id(next);
                    self.events[current].next_event = next_next;
                    self.events[next].next_event = current;
                    swapped = true;
                    prev = next;
                }
                Ordering::Equal => {
                    // Append next to current's sibling list.
                    let next_next = self.next_id(next);
                    self.events[current].next_event = next_next;
                    let mut current_sibling = current;
                    let mut next_sibling = self.next_sibling_id(current);
                    while self.valid_id(next_sibling) {
                        current_sibling = next_sibling;
                        next_sibling = self.next_sibling_id(current_sibling);
                    }
                    self.events[current_sibling].next_sibling = next;
                }
            }
        }
    }

    fn log(&self) {
        let mut iter_count = self.events.len() * self.events.len();

        println!("--");
        let mut current = self.first;
        while current < self.events.len() {
            assert!(iter_count > 0);
            iter_count -= 1;

            print!("[");
            let mut current_sibling = current;
            while current_sibling < self.events.len() {
                print!("{:?},", self.events[current_sibling].position);
                current_sibling = self.events[current_sibling].next_sibling;
            }
            print!("]  ");
            current = self.events[current].next_event;
        }
        println!("\n--");
    }

    fn assert_sorted(&self) {
        let mut current = self.first;
        let mut pos = point(f32::MIN, f32::MIN);
        while self.valid_id(current) {
            assert!(is_after(self.events[current].position, pos));
            pos = self.events[current].position;
            let mut current_sibling = current;
            while self.valid_id(current_sibling) {
                assert_eq!(self.events[current_sibling].position, pos);
                current_sibling = self.next_sibling_id(current_sibling);
            }
            current = self.next_id(current);
        }
    }
}

struct EventQueueBuilder {
    current: Point,
    current_id: EndpointId,
    first: Point,
    first_id: EndpointId,
    prev: Point,
    second: Point,
    nth: u32,
    tx: EventQueue,
}

impl EventQueueBuilder {
    fn with_capacity(cap: usize) -> Self {
        EventQueueBuilder {
            current: point(f32::NAN, f32::NAN),
            first: point(f32::NAN, f32::NAN),
            prev: point(f32::NAN, f32::NAN),
            second: point(f32::NAN, f32::NAN),
            current_id: EndpointId::INVALID,
            first_id: EndpointId::INVALID,
            nth: 0,
            tx: EventQueue::with_capacity(cap),
        }
    }

    fn set_path(&mut self, path: PathSlice) {
        if path.is_empty() {
            return;
        }
        let mut cursor = path.cursor();
        loop {
            let vertex_id = cursor.endpoint_id();
            match cursor.event(path) {
                PathEvent::MoveTo(to) => {
                    self.move_to(to, vertex_id);
                }
                PathEvent::Line(segment) => {
                    self.line_to(segment.to, vertex_id);
                }
                PathEvent::Quadratic(segment) => {
                    // TODO: properly get these ids!
                    self.quad_to(segment.to, CtrlPointId(vertex_id.0), EndpointId(vertex_id.0 + 1));
                }
                PathEvent::Close(..) => {
                    self.close();
                }
                _ => { unimplemented!(); }
            }

            if !cursor.next(path) {
                break;
            }
        }
        self.close();
    }

    fn vertex_event(&mut self, at: Point) {
        self.tx.push(at);
        self.tx.edge_data.push(EdgeData {
            from: EndpointId::INVALID,
            ctrl: CtrlPointId::INVALID,
            to: EndpointId::INVALID,
            range: 0.0..1.0,
            winding: 0,
        });
    }

    fn close(&mut self) {
        if self.nth == 0 {
            return;
        }

        // Unless we are already back to the first point we no need to
        // to insert an edge.
        let first = self.first;
        if self.current != self.first {
            let first_id = self.first_id;
            self.line_to(first, first_id)
        }

        // Since we can only check for the need of a vertex event when
        // we have a previous edge, we skipped it for the first edge
        // and have to do it now.
        if is_after(self.first, self.prev) && is_after(self.first, self.second) {
            self.vertex_event(first);
        }

        self.nth = 0;
    }

    fn move_to(&mut self, to: Point, to_id: EndpointId) {
        if self.nth > 0 {
            self.close();
        }

        self.nth = 0;
        self.first = to;
        self.current = to;
        self.first_id = to_id;
        self.current_id = to_id;
    }

    fn line_to(&mut self, to: Point, to_id: EndpointId) {
        self.quad_to(to, CtrlPointId::INVALID, to_id);
    }

    fn quad_to(&mut self, to: Point, ctrl_id: CtrlPointId, mut to_id: EndpointId) {
        if self.current == to {
            return;
        }

        let next_id = to_id;
        let mut from = self.current;
        let mut from_id = self.current_id;
        let mut winding = 1;
        if is_after(from, to) {
            if self.nth > 0 && is_after(from, self.prev) {
                self.vertex_event(from);
            }

            from = to;
            swap(&mut from_id, &mut to_id);
            winding = -1;
        }

        //println!("Edge {:?}/{:?} {:?} ->", from_id, to_id, from);
        debug_assert!(from_id != EndpointId::INVALID);
        debug_assert!(to_id != EndpointId::INVALID);
        self.tx.push(from);
        self.tx.edge_data.push(EdgeData {
            from: from_id,
            ctrl: ctrl_id,
            to: to_id,
            range: 0.0..1.0,
            winding,
        });

        if self.nth == 0 {
            self.second = to;
        }

        self.nth += 1;
        self.prev = self.current;
        self.current = to;
        self.current_id = next_id;
    }

    fn build(mut self) -> EventQueue {
        self.close();

        if let Some(evt) = self.tx.events.last_mut() {
            // before sorting it is convenient to set the next event to point
            // to the size of the vector so that it corresponds to the next
            // event that will be added.
            // For the built event queue however, the contract is that
            // invalid next events are equal to usize::MAX so that we can
            // insert new event without creating a loop.
            evt.next_event = usize::MAX;
        }

        self.tx.sort();

        self.tx
    }
}

pub struct VertexSourceIterator<'l> {
    events: &'l EventQueue,
    id: usize,
}

pub enum VertexSource {
    Endpoint { endpoint: EndpointId },
    OnEdge { from: EndpointId, to: EndpointId, ctrl: Option<EndpointId>, t: f32 },
}

impl<'l> Iterator for VertexSourceIterator<'l> {
    type Item = VertexSource;
    fn next(&mut self) -> Option<VertexSource> {
        if self.id == usize::MAX {
            return None;
        }

        let endpoint = EndpointId(self.events.edge_data[self.id].from.0);

        self.id = self.events.next_sibling_id(self.id);

        Some(VertexSource::Endpoint { endpoint })
    }
}

#[test]
fn test_traversal_sort_1() {
    let mut tx = EventQueue::new();
    tx.push(point(0.0, 0.0));
    tx.push(point(4.0, 0.0));
    tx.push(point(2.0, 0.0));
    tx.push(point(3.0, 0.0));
    tx.push(point(4.0, 0.0));
    tx.push(point(0.0, 0.0));
    tx.push(point(6.0, 0.0));

    tx.sort();
    tx.assert_sorted();
}

#[test]
fn test_traversal_sort_2() {
    let mut tx = EventQueue::new();
    tx.push(point(0.0, 0.0));
    tx.push(point(0.0, 0.0));
    tx.push(point(0.0, 0.0));
    tx.push(point(0.0, 0.0));

    tx.sort();
    tx.assert_sorted();
}

#[test]
fn test_traversal_sort_3() {
    let mut tx = EventQueue::new();
    tx.push(point(0.0, 0.0));
    tx.push(point(1.0, 0.0));
    tx.push(point(2.0, 0.0));
    tx.push(point(3.0, 0.0));
    tx.push(point(4.0, 0.0));
    tx.push(point(5.0, 0.0));

    tx.sort();
    tx.assert_sorted();
}

#[test]
fn test_traversal_sort_4() {
    let mut tx = EventQueue::new();
    tx.push(point(5.0, 0.0));
    tx.push(point(4.0, 0.0));
    tx.push(point(3.0, 0.0));
    tx.push(point(2.0, 0.0));
    tx.push(point(1.0, 0.0));
    tx.push(point(0.0, 0.0));

    tx.sort();
    tx.assert_sorted();
}

#[test]
fn test_traversal_sort_5() {
    let mut tx = EventQueue::new();
    tx.push(point(5.0, 0.0));
    tx.push(point(5.0, 0.0));
    tx.push(point(4.0, 0.0));
    tx.push(point(4.0, 0.0));
    tx.push(point(3.0, 0.0));
    tx.push(point(3.0, 0.0));
    tx.push(point(2.0, 0.0));
    tx.push(point(2.0, 0.0));
    tx.push(point(1.0, 0.0));
    tx.push(point(1.0, 0.0));
    tx.push(point(0.0, 0.0));
    tx.push(point(0.0, 0.0));

    tx.sort();
    tx.assert_sorted();
}

#[cfg(test)]
use crate::geometry_builder::{VertexBuffers, simple_builder};

#[test]
fn new_tess_triangle() {
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(5.0, 1.0));
    builder.line_to(point(3.0, 5.0));
    builder.close();

    let path = builder.build();

    let mut tess = FillTessellator::new();
    tess.enable_logging();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &path,
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );
}

#[test]
fn new_tess0() {
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(5.0, 0.0));
    builder.line_to(point(5.0, 5.0));
    builder.line_to(point(0.0, 5.0));
    builder.close();
    builder.move_to(point(1.0, 1.0));
    builder.line_to(point(4.0, 1.0));
    builder.line_to(point(4.0, 4.0));
    builder.line_to(point(1.0, 4.0));
    builder.close();

    let path = builder.build();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &path,
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );
}

#[test]
fn new_tess1() {

    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(5.0, -5.0));
    builder.line_to(point(10.0, 0.0));
    builder.line_to(point(9.0, 5.0));
    builder.line_to(point(10.0, 10.0));
    builder.line_to(point(5.0, 6.0));
    builder.line_to(point(0.0, 10.0));
    builder.line_to(point(1.0, 5.0));
    builder.close();

    builder.move_to(point(20.0, -1.0));
    builder.line_to(point(25.0, 1.0));
    builder.line_to(point(25.0, 9.0));
    builder.close();


    let path = builder.build();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &path,
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );
}

#[test]
fn new_tess_merge() {

    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));  // start
    builder.line_to(point(5.0, 5.0));  // merge
    builder.line_to(point(5.0, 1.0));  // start
    builder.line_to(point(10.0, 6.0)); // merge
    builder.line_to(point(11.0, 2.0)); // start
    builder.line_to(point(11.0, 10.0));// end
    builder.line_to(point(0.0, 9.0));  // left
    builder.close();

    let path = builder.build();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &path,
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );

    // "M 0 0 L 5 5 L 5 1 L 10 6 L 11 2 L 11 10 L 0 9 Z"
}

#[test]
fn test_intersection_1() {
    let mut builder = Path::builder();

    builder.move_to(point(118.82771, 64.41283));
    builder.line_to(point(23.451895, 50.336365));
    builder.line_to(point(123.39044, 68.36287));
    builder.close();

    builder.move_to(point(80.39975, 58.73177));
    builder.line_to(point(80.598236, 60.38033));
    builder.line_to(point(63.05017, 63.488304));
    builder.close();

    let path = builder.build();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &path,
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );

    // SVG path syntax:
    // "M 118.82771 64.41283 L 23.451895 50.336365 L 123.39044 68.36287 ZM 80.39975 58.73177 L 80.598236 60.38033 L 63.05017 63.488304 Z"
}

#[test]
fn new_tess_points_too_close() {
    // The first and last point are almost equal but not quite.

    let mut builder = Path::builder();

    builder.move_to(point(52.90753, -72.15962));
    builder.line_to(point(45.80301, -70.96051));
    builder.line_to(point(50.91391, -83.96548));
    builder.line_to(point(52.90654, -72.159454));
    builder.close();

    let mut tess = FillTessellator::new();
    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );

    // SVG path syntax:
    // "M 52.90753 -72.15962 L 45.80301 -70.96051 L 50.91391 -83.96548 L 52.90654 -72.159454 Z"
}

#[test]
fn new_tess_coincident_simple() {
    let mut builder = Path::builder();

    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(0.0, 1.0));
    builder.line_to(point(0.0, 1.0));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );
}

#[test]
fn new_tess_overlapping_1() {
    let mut builder = Path::builder();

    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(2.0, 2.0));
    builder.line_to(point(3.0, 1.0));
    builder.line_to(point(0.0, 4.0));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );
}

#[test]
fn reduced_test_case_01() {
    let mut builder = Path::builder();

    builder.move_to(point(0.73951757, 0.3810749));
    builder.line_to(point(0.4420668, 0.05925262));
    builder.line_to(point(0.54023945, 0.16737175));
    builder.line_to(point(0.8839954, 0.39966547));
    builder.line_to(point(0.77066493, 0.67880523));
    builder.line_to(point(0.48341691, 0.09270251));
    builder.line_to(point(0.053493023, 0.18919432));
    builder.line_to(point(0.6088793, 0.57187665));
    builder.line_to(point(0.2899257, 0.09821439));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );

    // SVG path syntax:
    // "M 0.73951757 0.3810749 L 0.4420668 0.05925262 L 0.54023945 0.16737175 L 0.8839954 0.39966547 L 0.77066493 0.67880523 L 0.48341691 0.09270251 L 0.053493023 0.18919432 L 0.6088793 0.57187665 L 0.2899257 0.09821439 Z"
}
