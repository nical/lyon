use crate::{FillOptions, Side};
use crate::geom::math::*;
use crate::geom::{LineSegment};
//use crate::geom::cubic_to_quadratic::cubic_to_monotonic_quadratics;
use crate::geometry_builder::{GeometryBuilder, VertexId};
use crate::path_fill::MonotoneTessellator;
//use crate::path::builder::*;
use crate::path::{PathEvent, Path, FillRule, Transition, EndpointId, Position};
use crate::path::generic::PathEventId;
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

type SpanIdx = i32;
type EventId = u32;
type ActiveEdgeIdx = usize;
const INVALID_EVENT_ID: u32 = u32::MAX;

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
    fn new() -> Self {
        // The span index starts at -1 so that entering the first span (of index 0) increments
        // it to zero.
        WindingState {
            span_index: -1,
            number: 0,
            transition: Transition::None,
        }
    }

    fn update(&mut self, fill_rule: FillRule, edge_winding: i16) {
        let prev_winding_number = self.number;
        self.number += edge_winding;
        self.transition = fill_rule.transition(prev_winding_number, self.number);
        if self.transition == Transition::In {
            self.span_index += 1;
        }
    }
}

struct ActiveEdgeScan {
    edges_to_split: Vec<ActiveEdgeIdx>,
    spans_to_end: Vec<SpanIdx>,
    merges_to_resolve: Vec<(SpanIdx, ActiveEdgeIdx)>,
    pending_right: Option<ActiveEdgeIdx>,
    pending_merge: Option<ActiveEdgeIdx>,
    above_start: ActiveEdgeIdx,
    above_end: ActiveEdgeIdx,
    winding_before_point: WindingState,
}

impl ActiveEdgeScan {
    fn new() -> Self {
        ActiveEdgeScan {
            edges_to_split: Vec::new(),
            spans_to_end: Vec::new(),
            merges_to_resolve: Vec::new(),
            pending_right: None,
            pending_merge: None,
            above_start: 0,
            above_end: 0,
            winding_before_point: WindingState::new(),
        }
    }

    fn reset(&mut self) {
        self.edges_to_split.clear();
        self.spans_to_end.clear();
        self.merges_to_resolve.clear();
        self.pending_right = None;
        self.pending_merge = None;
        self.above_start = 0;
        self.above_end = 0;
        self.winding_before_point = WindingState::new();
    }
}

#[derive(Debug)]
struct ActiveEdge {
    min_x: f32,
    max_x: f32,

    from: Point,
    to: Point,

    winding: i16,
    is_merge: bool,

    from_id: VertexId,
    src_edge: EventId,

    // Only valid when sorting the active edges.
    sort_x: f32,

    range_end: f32,
}

impl ActiveEdge {
    fn solve_x_for_y(&self, y: f32) -> f32 {
        // Because of float precision hazard, solve_x_for_y can
        // return something slightly out of the min/max range which
        // causes the ordering to be inconsistent with the way the
        // scan phase uses the min/max range.
        LineSegment {
            from: self.from,
            to: self.to,
        }.solve_x_for_y(y).max(self.min_x).min(self.max_x)
    }
}

struct ActiveEdges {
    edges: Vec<ActiveEdge>,
}

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

        debug_assert!(!self.spans[idx].remove);

        let span = &mut self.spans[idx];
        span.remove = true;
        span.tess.end(*position, id);
        span.tess.flush_experimental(output);
    }

    fn split_span(
        &mut self,
        new_span_idx: SpanIdx,
        left_span_idx: SpanIdx,
        right_span_idx: SpanIdx,
        split_position: &Point,
        split_id: VertexId,
        upper_position: &Point,
        upper_id: VertexId,
    ) {
        self.spans.insert(
            new_span_idx as usize,
            Span {
                tess: MonotoneTessellator::new().begin(*upper_position, upper_id),
                remove: false,
            }
        );

        debug_assert!(!self.spans[left_span_idx as usize].remove);
        debug_assert!(!self.spans[right_span_idx as usize].remove);
        self.spans[left_span_idx as usize].tess.vertex(*split_position, split_id, Side::Right);
        self.spans[right_span_idx as usize].tess.vertex(*split_position, split_id, Side::Left);
    }

    fn merge_spans(
        &mut self,
        left_span_idx: SpanIdx,
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

        let right_span_idx = left_span_idx + 1;

        debug_assert!(!self.spans[left_span_idx as usize].remove);
        self.spans[left_span_idx as usize].tess.vertex(
            *merge_position,
            merge_vertex,
            Side::Right,
        );

        debug_assert!(!self.spans[right_span_idx as usize].remove);
        self.spans[right_span_idx as usize].tess.vertex(
            *merge_position,
            merge_vertex,
            Side::Left,
        );

        self.end_span(
            left_span_idx,
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
    angle: f32,
    // Index in events.edge_data
    src_edge: EventId,
    winding: i16,
    range_end: f32,
}

pub struct FillTessellator {
    current_position: Point,
    current_vertex: VertexId,
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
        #[cfg(not(feature = "release"))]
        let log = env::var("LYON_FORCE_LOGGING").is_ok();
        #[cfg(feature = "release")]
        let log = false;

        FillTessellator {
            current_position: point(f32::MIN, f32::MIN),
            current_vertex: VertexId::INVALID,
            active: ActiveEdges {
                edges: Vec::new(),
            },
            edges_below: Vec::new(),
            fill_rule: FillRule::EvenOdd,
            fill: Spans {
                spans: Vec::new(),
            },
            log,
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
        tx_builder.set_path(path.iter());
        self.events = tx_builder.build();

        builder.begin_geometry();

        self.tessellator_loop(builder);

        if !self.assume_no_intersection {
            debug_assert!(self.active.edges.is_empty());
            // TODO: only a few tests break from this assertions
            //debug_assert!(self.fill.spans.is_empty());
        }

        // There shouldn't be any span left after the tessellation ends.
        // In practice there can be some in complicated self-intersection
        // situations, so flush them in case they hold some uncommitted
        // geometry.
        for span in &mut self.fill.spans {
            if !span.remove {
                span.tess.flush_experimental(builder);
            }
        }

        self.fill.spans.clear();

        builder.end_geometry();

        tess_log!(self, "\n ***************** \n");
    }

    pub fn enable_logging(&mut self) {
        self.log = true;
    }

    fn tessellator_loop(
        &mut self,
        output: &mut dyn GeometryBuilder<Vertex>
    ) {
        let mut scan = ActiveEdgeScan::new();
        let mut _prev_position = point(-1000000000.0, -1000000000.0); // TODO
        let mut current_event = self.events.first_id();
        while self.events.valid_id(current_event) {

            self.initialize_events(current_event, output);

            debug_assert!(is_after(self.current_position, _prev_position));
            _prev_position = self.current_position;

            if !self.process_events(&mut scan, output) {
                // Something went wrong, attempt to salvage the state of the sweep
                // line and try again.
                self.recover_from_error();
                assert!(self.process_events(&mut scan, output));
            }

            current_event = self.events.next_id(current_event);
        }
    }

    fn initialize_events(&mut self, current_event: EventId, output: &mut dyn GeometryBuilder<Vertex>) {
        tess_log!(self, "\n --- event #{}", current_event);

        self.current_position = self.events.position(current_event);

        let src = VertexSourceIterator {
            events: &self.events,
            id: current_event,
        };

        self.current_vertex = output.add_vertex_exp(self.current_position, src).unwrap();

        let mut current_sibling = current_event;
        while self.events.valid_id(current_sibling) {
            let edge = &self.events.edge_data[current_sibling as usize];
            // We insert "fake" edges when there are end events
            // to make sure we process that vertex even if it has
            // no edge below.
            if edge.is_edge {
                let to = edge.to;

                if !is_after(to, self.current_position) {
                    tess_log!(self, "edge: {:?}  current: {:?} to: {:?}", edge, self.current_position, to);
                }
                debug_assert!(is_after(to, self.current_position));
                self.edges_below.push(PendingEdge {
                    to,
                    angle: (to - self.current_position).angle_from_x_axis().radians,
                    src_edge: current_sibling,
                    winding: edge.winding,
                    range_end: edge.range.end,
                });
            }

            current_sibling = self.events.next_sibling_id(current_sibling);
        }        
    }

    /// An iteration of the sweep line algorithm.
    fn process_events(
        &mut self,
        scan: &mut ActiveEdgeScan,
        output: &mut dyn GeometryBuilder<Vertex>,
    ) -> bool {
        debug_assert!(!self.current_position.x.is_nan() && !self.current_position.y.is_nan());

        tess_log!(self, "\n --- events at {:?} {:?}         {} edges below",
            self.current_position,
            self.current_vertex,
            self.edges_below.len(),
        );

        // Step 1 - Scan the active edge list, deferring processing and detecting potential
        // ordering issues in the active edges.
        if !self.scan_active_edges(scan) {
            return false;
        }

        // Step 2 - Do the necessary processing on edges that end at the current point.
        self.process_edges_above(scan, output);

        // Step 3 - Do the necessary processing on edges that start at the current point.
        self.process_edges_below(scan);

        // Step 4 - Insert/remove edges to the active edge as necessary and handle
        // potential self-intersections.
        self.update_active_edges(scan.above_start..scan.above_end);

        #[cfg(not(feature = "release"))]
        self.log_active_edges();

        true
    }

    #[cfg(not(feature = "release"))]
    fn log_active_edges(&self) {
        tess_log!(self, "sweep line: {}", self.active.edges.len());
        for (i, e) in self.active.edges.iter().enumerate() {
            if e.is_merge {
                tess_log!(self, "{} | (merge) {} sort:{}  {:?}", i, e.from, e.sort_x, e.from_id);
            } else {
                tess_log!(self, "{} | {} -> {} ({})   x:[{}..{}] sort:{}  {:?}", i, e.from, e.to, e.winding, e.min_x, e.max_x, e.sort_x, e.from_id);
            }
        }
        tess_log!(self, "spans: {}", self.fill.spans.len());
    }

    /// Scan the active edges to find the information we will need for the tessellation, without
    /// modifying the state of the sweep line and active spans.
    ///
    /// During this scan we also check that the ordering of the active edges is correct.
    /// If an error is detected we bail out of the scan which will cause us to sort the active
    /// edge list and try to scan again (this is why have to defer any modification to after
    /// the scan).
    ///
    /// The scan happens in three steps:
    /// - 1) Loop over the edges on the left of the current point to compute the winding number.
    /// - 2) Loop over the edges that connect with the current point to determine what processing
    ///      is needed (for example end events or right events).
    /// - 3) Loop over the edges on the right of the current point to detect potential edges that should
    ///      have been handled in the previous phases.
    fn scan_active_edges(&self, scan: &mut ActiveEdgeScan) -> bool {

        scan.reset();

        let current_x = self.current_position.x;
        let mut connecting_edges = false;
        let mut active_edge_idx = 0;
        let mut winding = WindingState::new();

        // Step 1 - Iterate over edges *before* the current point.
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

            let threshold = 0.001;
            let egde_is_before_current_point = if points_are_equal(self.current_position, active_edge.to) {
                // We just found our first edge that connects with the current point.
                // We might find other ones in the next iterations.
                connecting_edges = true;
                false
            } else if active_edge.max_x < current_x {
                true
            } else if active_edge.min_x > current_x {
                tess_log!(self, "min_x({:?}) > current_x({:?})", active_edge.min_x, current_x);
                false
            } else {
                let ex = active_edge.solve_x_for_y(self.current_position.y);

                if (ex - current_x).abs() <= threshold {
                    connecting_edges = true;
                    false
                } else if ex > current_x {
                    tess_log!(self, "ex({:?}) > current_x({:?})", ex, current_x);
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

        scan.above_start = active_edge_idx;
        scan.winding_before_point = winding.clone();


        // Step 2 - Iterate over edges connecting with the current point.

        let mut is_first_transition = true;
        let mut prev_transition_in = None;

        tess_log!(self, "connecting_edges {} | {}", connecting_edges, active_edge_idx);
        if connecting_edges {
            for active_edge in &self.active.edges[active_edge_idx..] {
                if active_edge.is_merge {
                    tess_log!(self, "merge to resolve {}", active_edge_idx);

                    if self.fill.spans.len() as SpanIdx <= winding.span_index + 1 {
                        tess_log!(self, "error: not enough active spans for merge event.");
                        return false;
                    }

                    scan.merges_to_resolve.push((winding.span_index, active_edge_idx));
                    winding.span_index += 1;
                    active_edge_idx += 1;

                    continue;
                }

                if !points_are_equal(self.current_position, active_edge.to) {
                    let threshold = 0.001;

                    let mut is_error = active_edge.max_x + threshold < current_x || active_edge.to.y < self.current_position.y;
                    let mut is_on_edge = false;

                    if !is_error && active_edge.min_x <= current_x {

                        let ex = active_edge.solve_x_for_y(self.current_position.y);
                        tess_log!(self, "ex = {:?}", ex);
                        if (ex - current_x).abs() <= threshold {
                            tess_log!(self, " -- vertex on an edge! {:?} -> {:?}", active_edge.from, active_edge.to);
                            is_on_edge = true;
                            scan.edges_to_split.push(active_edge_idx);
                            // TODO: This is hacky: We only register a merge vertex
                            // if there is no edge below the current vertex, except
                            // that the "vertex on an edge" case can add an edge
                            // below the current vertex after we have registered the
                            // merge vertex. So we invert the decision.
                            if let Some(idx) = scan.pending_merge {
                                scan.pending_right = Some(idx);
                                scan.pending_merge = None;
                            }
                        } else if ex < current_x {
                            is_error = true;
                        }
                        tess_log!(self, "ex = {:?} (diff={})", ex, ex - current_x);
                    }

                    if is_error {
                        // Ran into an edge that is before the current point.
                        tess_log!(self, "error A");
                        return false;
                    }

                    if !is_on_edge {
                        tess_log!(self, "!is_on_edge -> break {:?};", active_edge);
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
                        // Only register a merge vertex if there is no edge below
                        // the current vertex. In the vast majority of case we can
                        // tell by looking at the current state of edges_below and
                        // edges_to_split, but some times new elements can be added
                        // to edges_to_split which invalidates this decision. So we
                        // have to be careful about rolling back pending_edges if
                        // we push into edges_to_split.
                        let no_edge_below = self.edges_below.is_empty()
                            && scan.edges_to_split.is_empty();

                        if no_edge_below {
                            // Merge event.
                            //
                            //  .\  |../  /
                            //  ..\ |./ /..
                            //  -->\|//....
                            //  ....x......
                            //
                            scan.pending_merge = Some(active_edge_idx);
                        } else {
                            // Right event.
                            //
                            //   ..../
                            //   ...x
                            //   ....\
                            //
                            scan.pending_right = Some(active_edge_idx);
                        }
                    }
                    (Transition::Out, false) => {
                        let in_idx = prev_transition_in.unwrap();
                        tess_log!(self, " ** end ** edges: [{},{}] span: {}",
                            in_idx, active_edge_idx,
                            winding.span_index
                        );

                        if winding.span_index < self.fill.spans.len() as i32 {
                            // End event.
                            //
                            //  \.../
                            //   \./
                            //    x
                            //
                            scan.spans_to_end.push(winding.span_index);
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

        tess_log!(self, "edges after | {}", active_edge_idx);

        scan.above_end = active_edge_idx;


        // Step 3 - Now Iterate over edges after the current point.
        // We only do this to detect errors.

        for active_edge in &self.active.edges[active_edge_idx..] {
            if active_edge.is_merge {
                continue;
            }

            if active_edge.max_x < current_x {
                tess_log!(self, "error C");
                return false;
            }

            if points_are_equal(self.current_position, active_edge.to) {
                tess_log!(self, "error D {:?} == {:?}", self.current_position, active_edge.to);
                return false;
            }

            if active_edge.min_x < current_x
                && active_edge.solve_x_for_y(self.current_position.y) < current_x {
                tess_log!(self, "error E");
                return false;
            }
        }


        true
    }

    fn process_edges_above(&mut self, scan: &mut ActiveEdgeScan, output: &mut dyn GeometryBuilder<Vertex>) {
        tess_log!(self, "merges to resolve: {:?}", scan.merges_to_resolve);
        for &(span_index, edge_idx) in &scan.merges_to_resolve {
            //  \...\ /.
            //   \...x..  <-- merge vertex
            //    \./...  <-- active_edge
            //     x....  <-- current vertex
            let active_edge: &mut ActiveEdge = &mut self.active.edges[edge_idx];
            let merge_vertex: VertexId = active_edge.from_id;
            let merge_position = active_edge.from;
            active_edge.to = self.current_position;

            tess_log!(self, " Resolve merge event {} at {:?} ending span {}", edge_idx, active_edge.to, span_index);

            self.fill.merge_spans(
                span_index,
                &self.current_position,
                self.current_vertex,
                &merge_position,
                merge_vertex,
                output,
            );

            active_edge.is_merge = false;

            #[cfg(feature="debugger")]
            debugger_monotone_split(&self.debugger, &merge_position, &self.current_position);
        }

        for &span_index in &scan.spans_to_end {
            self.fill.end_span(
                span_index,
                &self.current_position,
                self.current_vertex,
                output,
            );
        }

        self.fill.cleanup_spans();

        for &edge_idx in &scan.edges_to_split {
            let active_edge = &mut self.active.edges[edge_idx];
            let to = active_edge.to;

            self.edges_below.push(PendingEdge {
                to,
                angle: (to - self.current_position).angle_from_x_axis().radians,
                src_edge: active_edge.src_edge,
                winding: active_edge.winding,
                range_end: active_edge.range_end,
            });
            tess_log!(self,
                "add edge below {:?} -> {:?} ({:?})",
                self.current_position,
                self.edges_below.last().unwrap().to,
                active_edge.winding,
            );

            active_edge.to = self.current_position;
        }
    }

    fn process_edges_below(&mut self, scan: &mut ActiveEdgeScan) {
        let mut winding = scan.winding_before_point.clone();

        tess_log!(self, "connecting edges: {}..{} {:?}", scan.above_start, scan.above_end, winding.transition);
        tess_log!(self, "winding state before point: {:?}", winding);

        self.sort_edges_below();

        if let Some(in_idx) = scan.pending_merge {
            // Merge event.
            //
            //  ...\   /...
            //  ....\ /....
            //  .....x.....
            //

            tess_log!(self, " ** merge ** edges: [{},{}] span: {}",
                in_idx, scan.above_end - 1,
                winding.span_index
            );

            let e = &mut self.active.edges[in_idx];
            e.is_merge = true;
            e.from = e.to;
            e.min_x = e.to.x;
            e.max_x = e.to.x;
            e.winding = 0;
            e.from_id = self.current_vertex;
        }

        // The range of pending edges below the current vertex to look at in the
        // last loop (not always the full range if we process split events).
        let mut below = 0..self.edges_below.len();

        if self.fill_rule.is_in(winding.number)
            && scan.above_start == scan.above_end
            && self.edges_below.len() >= 2 {

            // Split event.
            //
            //  ...........
            //  .....x.....
            //  ..../ \....
            //  .../   \...
            //

            let left_enclosing_edge_idx = scan.above_start - 1;
            if self.active.edges[left_enclosing_edge_idx].is_merge {
                self.merge_split_event(
                    left_enclosing_edge_idx,
                    winding.span_index - 1,
                );
                // Remove the merge edge from the active edge list.
                self.active.edges.remove(left_enclosing_edge_idx);
                scan.above_start -= 1;
                scan.above_end -= 1;
            } else {
                self.split_event(
                    left_enclosing_edge_idx,
                    winding.span_index,
                );
            }

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

            if let Some(idx) = scan.pending_right {
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
                    self.current_vertex,
                    Side::Right,
                );

                scan.pending_right = None;

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
                        tess_log!(self, " ** left ** edge {} span: {}", scan.above_start, winding.span_index);

                        self.fill.spans[winding.span_index as usize].tess.vertex(
                            self.current_position,
                            self.current_vertex,
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

                        tess_log!(self, " begin span {} ({})", winding.span_index, self.fill.spans.len());
                        self.fill.begin_span(
                            winding.span_index,
                            &self.current_position,
                            self.current_vertex,
                        );
                    }
                }
                Transition::None => {
                    tess_log!(self, "(transition: none)");
                }
            }
        }
    }

    fn update_active_edges(&mut self, above: Range<usize>) {
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
            assert!(from != edge.to);
            let idx = first_edge_below + i;
            self.active.edges.insert(idx, ActiveEdge {
                min_x: from.x.min(edge.to.x),
                max_x: from.x.max(edge.to.x),
                sort_x: 0.0,
                from,
                to: edge.to,
                winding: edge.winding,
                is_merge: false,
                from_id: self.current_vertex,
                src_edge: edge.src_edge,
                range_end: edge.range_end,
            });
        }
    }

    fn merge_split_event(&mut self, merge_idx: ActiveEdgeIdx, left_span_idx: SpanIdx) {
        let upper_pos = self.active.edges[merge_idx].from;
        let upper_id = self.active.edges[merge_idx].from_id;
        tess_log!(self, " ** merge + split ** edge {} span: {} merge pos {:?}", merge_idx, left_span_idx, upper_pos);

        // Split vertex under a merge vertex
        //
        //  ...\ /...
        //  ....x....   <-- merge vertex
        //  ....:....
        //  ----x----   <-- current split vertex
        //  .../ \...
        //

        let left_span_idx = left_span_idx as usize;
        let right_span_idx = left_span_idx + 1;

        self.fill.spans[left_span_idx].tess.vertex(
            upper_pos,
            upper_id,
            Side::Right,
        );
        self.fill.spans[left_span_idx].tess.vertex(
            self.current_position,
            self.current_vertex,
            Side::Right,
        );

        self.fill.spans[right_span_idx].tess.vertex(
            upper_pos,
            upper_id,
            Side::Left,
        );
        self.fill.spans[right_span_idx].tess.vertex(
            self.current_position,
            self.current_vertex,
            Side::Left,
        );
    }

    fn split_event(&mut self, left_enclosing_edge_idx: ActiveEdgeIdx, left_span_idx: SpanIdx) {
        let right_enclosing_edge_idx = left_enclosing_edge_idx + 1;

        let upper_left = self.active.edges[left_enclosing_edge_idx].from;
        let upper_right = self.active.edges[right_enclosing_edge_idx].from;

        let right_span_idx = left_span_idx + 1;

        let (upper_position, upper_id, new_span_idx) = if is_after(upper_left, upper_right) {
            //                |.....
            // upper_left --> x.....
            //               /.\....
            //              /...x... <-- current split vertex
            //             /.../ \..
            (
                upper_left,
                self.active.edges[left_enclosing_edge_idx].from_id,
                left_span_idx,
            )
        } else {
            //                          .....|
            //                          .....x <-- upper_right
            //                          ..../.\
            // current split vertex --> ...x...\
            //                          ../ \...\
            (
                upper_right,
                self.active.edges[right_enclosing_edge_idx].from_id,
                right_span_idx,
            )
        };

        self.fill.split_span(
            new_span_idx, left_span_idx, right_span_idx,
            &self.current_position, self.current_vertex,
            &upper_position, upper_id,
        );
    }

    fn handle_intersections(&mut self) {
        for edge_below in &mut self.edges_below {
            let below_min_x = self.current_position.x.min(edge_below.to.x);
            let below_max_x = self.current_position.x.max(edge_below.to.x);

            let below_segment = LineSegment {
                from: self.current_position.to_f64(),
                to: edge_below.to.to_f64(),
            };

            let mut tb_min = 1.0;
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
                    if tb < tb_min && tb > 0.0 && ta > 0.0 && ta <= 1.0 {
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

                let active_edge = &mut self.active.edges[active_edge_idx];

                if is_after(self.current_position, intersection_position) {
                    intersection_position = self.current_position;
                }

                if is_after(active_edge.from, intersection_position) {
                    intersection_position = active_edge.from;
                }

                if is_near(intersection_position, edge_below.to) {
                    tess_log!(self, "intersection near below.to");
                    intersection_position = edge_below.to;
                }

                if is_near(intersection_position, active_edge.to) {
                    tess_log!(self, "intersection near below.to");
                    intersection_position = active_edge.to;
                }

                if is_after(intersection_position, edge_below.to) {
                    intersection_position = edge_below.to;
                }

                if is_after(intersection_position, active_edge.to) {
                    intersection_position = active_edge.to;
                }

                if intersection_position.y < self.current_position.y {
                    tess_log!(self, "fixup the intersection because of y coordinate");
                    intersection_position.y = self.current_position.y + std::f32::EPSILON; // TODO
                } else if intersection_position.y == self.current_position.y
                    && intersection_position.x < self.current_position.x {
                    tess_log!(self, "fixup the intersection because of x coordinate");
                    intersection_position.x = self.current_position.x;
                }

                let a_src_edge_data = self.events.edge_data[active_edge.src_edge as usize].clone();
                let b_src_edge_data = self.events.edge_data[edge_below.src_edge as usize].clone();

                let mut inserted_evt = None;

                if active_edge.to != intersection_position
                    && active_edge.from != intersection_position {
                    // TODO: the remapped ts look incorrect sometimes.
                    let remapped_ta = remap_t_in_range(
                        ta as f32,
                        a_src_edge_data.range.start..active_edge.range_end,
                    );

                    if is_after(active_edge.to, intersection_position) {
                        // Should take this branch most of the time.
                        inserted_evt = Some(self.events.insert_sorted(
                            intersection_position,
                            EdgeData {
                                range: remapped_ta as f32 .. active_edge.range_end,
                                winding: active_edge.winding,
                                to: active_edge.to,
                                is_edge: true,
                                .. a_src_edge_data
                            }
                        ));
                    } else {
                        tess_log!(self, "flip active edge after intersection");
                        self.events.insert_sorted(
                            active_edge.to,
                            EdgeData {
                                range: active_edge.range_end .. remapped_ta as f32,
                                winding: -active_edge.winding,
                                to: intersection_position,
                                is_edge: true,
                                .. a_src_edge_data
                            }
                        );
                    }

                    active_edge.to = intersection_position;
                    active_edge.range_end = remapped_ta;
                }

                if edge_below.to != intersection_position
                    && self.current_position != intersection_position {
                    debug_assert!(is_after(edge_below.to, intersection_position));

                    let remapped_tb = remap_t_in_range(
                        tb as f32,
                        b_src_edge_data.range.start..edge_below.range_end,
                    );

                    if is_after(edge_below.to, intersection_position) {
                        let edge_data = EdgeData {
                            range: remapped_tb as f32 .. edge_below.range_end,
                            winding: edge_below.winding,
                            to: edge_below.to,
                            is_edge: true,
                            .. b_src_edge_data
                        };

                        if let Some(idx) = inserted_evt {
                            // Should take this branch most of the time.
                            self.events.insert_sibling(idx, intersection_position, edge_data);
                        } else {
                            self.events.insert_sorted(intersection_position, edge_data);
                        }
                    } else {
                        tess_log!(self, "flip edge below after intersection");
                        self.events.insert_sorted(
                            edge_below.to,
                            EdgeData {
                                range: edge_below.range_end .. remapped_tb as f32,
                                winding: -edge_below.winding,
                                to: intersection_position,
                                is_edge: true,
                                .. b_src_edge_data
                            }
                        );
                    };

                    edge_below.to = intersection_position;
                    edge_below.range_end = remapped_tb;
                }
            }
        }

        self.log_active_edges();
    }

    fn sort_active_edges(&mut self) {
        // Merge edges are a little subtle when it comes to sorting.
        // They are points rather than edges and the best we can do is
        // keep their relative ordering with their previous or next edge.

        let y = self.current_position.y;

        // TODO: the code that updates the active edge list sometimes misses
        // some edges to remove, which we fix here. See why that is and hopefully
        // avoid handling this here.
        let mut edges_to_remove = Vec::new();

        let mut prev_x = f32::NAN;
        for (i, edge) in self.active.edges.iter_mut().enumerate() {
            if edge.is_merge {
                debug_assert!(!prev_x.is_nan());
                edge.sort_x = prev_x;
            } else {
                if is_after(self.current_position, edge.to) {
                    edges_to_remove.push(i);
                    continue;
                }

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

        for idx in edges_to_remove.iter().rev() {
            self.active.edges.swap_remove(*idx);
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

        let mut winding = WindingState::new();

        for edge in &self.active.edges {
            if edge.is_merge {
                winding.span_index += 1;
            } else {
                winding.update(self.fill_rule, edge.winding);
            }

            if winding.span_index >= self.fill.spans.len() as i32 {
                self.fill.begin_span(
                    winding.span_index,
                    &edge.from,
                    edge.from_id,
                );
            }
        }

        #[cfg(not(target = "release"))]
        self.log_active_edges();
    }

    fn sort_edges_below(&mut self) {
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
    // TODO: Use the tolerance threshold?
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

#[inline]
fn is_near(a: Point, b: Point) -> bool {
    (a - b).square_length() < 0.001
}

pub struct Event {
    next_sibling: EventId,
    next_event: EventId,
    position: Point,
}

#[derive(Clone, Debug)]
struct EdgeData {
    evt_id: PathEventId,
    to: Point,
    range: std::ops::Range<f32>,
    winding: i16,
    is_edge: bool,
}

pub struct EventQueue {
    events: Vec<Event>,
    edge_data: Vec<EdgeData>,
    first: EventId,
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
        self.events.push(Event {
            position,
            next_sibling: INVALID_EVENT_ID,
            next_event: INVALID_EVENT_ID,
        });
        self.sorted = false;
    }

    // Could start searching at the tessellator's current event id.
    fn insert_sorted(&mut self, position: Point, data: EdgeData) -> EventId {
        debug_assert!(self.sorted);
        debug_assert!(is_after(data.to, position));

        let idx = self.events.len() as EventId;
        self.events.push(Event {
            position,
            next_sibling: INVALID_EVENT_ID,
            next_event: INVALID_EVENT_ID,
        });
        self.edge_data.push(data);

        let mut prev = self.first;
        let mut current = self.first;
        while self.valid_id(current) {
            let pos = self.events[current as usize].position;

            if pos == position {
                debug_assert!(prev != current);
                self.events[idx as usize].next_sibling = self.events[current as usize].next_sibling;
                self.events[current as usize].next_sibling = idx;
                break;
            } else if is_after(pos, position) {
                self.events[prev as usize].next_event = idx;
                self.events[idx as usize].next_event = current;
                break;
            }

            prev = current;
            current = self.next_id(current);
        }

        idx
    }

    fn insert_sibling(&mut self, sibling: EventId, position: Point, data: EdgeData) {
        let idx = self.events.len() as EventId;
        let next_sibling = self.events[sibling as usize].next_sibling;

        self.events.push(Event {
            position,
            next_event: INVALID_EVENT_ID,
            next_sibling,
        });

        self.edge_data.push(data);

        self.events[sibling as usize].next_sibling = idx;
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.first = 0;
        self.sorted = false;
    }

    pub fn first_id(&self) -> EventId { self.first }

    pub fn next_id(&self, id: EventId) -> EventId { self.events[id as usize].next_event }

    pub fn next_sibling_id(&self, id: EventId) -> EventId { self.events[id as usize].next_sibling }

    pub fn valid_id(&self, id: EventId) -> bool { (id as usize) < self.events.len() }

    pub fn position(&self, id: EventId) -> Point { self.events[id as usize].position }

    fn sort(&mut self) {
        self.sorted = true;

        if self.events.is_empty() {
            return;
        }

        let range = 0..self.events.len();
        self.first = self.merge_sort(range);
    }

    /// Merge sort with two twists:
    /// - Events at the same position are grouped into a "sibling" list.
    /// - We take advantage of having events stored contiguously in a vector
    ///   by recursively splitting ranges of the array instead of traversing
    ///   the lists to find a split point.
    fn merge_sort(&mut self, range: Range<usize>) -> EventId {
        let split = (range.start + range.end) / 2;

        if split == range.start {
            return range.start as EventId;
        }

        let a_head = self.merge_sort(range.start..split);
        let b_head = self.merge_sort(split..range.end);

        self.merge(a_head, b_head)
    }

    fn merge(&mut self, a: EventId, b: EventId) -> EventId {
        if a == INVALID_EVENT_ID {
            return b;
        } else if b == INVALID_EVENT_ID {
            return a;
        }

        debug_assert!(a != b);

        return match compare_positions(self.events[a as usize].position, self.events[b as usize].position) {
            Ordering::Less => {
                let a_next = self.events[a as usize].next_event;
                self.events[a as usize].next_event = self.merge(a_next, b);

                a
            }
            Ordering::Greater => {
                let b_next = self.events[b as usize].next_event;
                self.events[b as usize].next_event = self.merge(a, b_next);

                b
            }
            Ordering::Equal => {
                // Add b to a's sibling list.
                let a_sib = self.find_last_sibling(a) as usize;
                self.events[a_sib].next_sibling = b;

                let b_next = self.events[b as usize].next_event;
                self.merge(a, b_next)
            }
        }
    }

    fn find_last_sibling(&self, id: EventId) -> EventId {
        let mut current_sibling = id;
        let mut next_sibling = self.next_sibling_id(id);
        while self.valid_id(next_sibling) {
            current_sibling = next_sibling;
            next_sibling = self.next_sibling_id(current_sibling);
        }

        current_sibling
    }

    fn log(&self) {
        let mut iter_count = self.events.len() * self.events.len();

        println!("--");
        let mut current = self.first;
        while (current as usize) < self.events.len() {
            assert!(iter_count > 0);
            iter_count -= 1;

            print!("[");
            let mut current_sibling = current;
            while (current_sibling as usize) < self.events.len() {
                print!("{:?},", self.events[current_sibling as usize].position);
                current_sibling = self.events[current_sibling as usize].next_sibling;
            }
            print!("]  ");
            current = self.events[current as usize].next_event;
        }
        println!("\n--");
    }

    fn assert_sorted(&self) {
        self.log();
        let mut current = self.first;
        let mut pos = point(f32::MIN, f32::MIN);
        let mut n = 0;
        while self.valid_id(current) {
            assert!(is_after(self.events[current as usize].position, pos));
            pos = self.events[current as usize].position;
            let mut current_sibling = current;
            while self.valid_id(current_sibling) {
                n += 1;
                assert_eq!(self.events[current_sibling as usize].position, pos);
                current_sibling = self.next_sibling_id(current_sibling);
            }
            current = self.next_id(current);
        }
        assert_eq!(n, self.events.len());
    }
}

struct EventQueueBuilder {
    current: Point,
    prev: Point,
    second: Point,
    nth: u32,
    tx: EventQueue,
    prev_evt_is_edge: bool,
}

impl EventQueueBuilder {
    fn with_capacity(cap: usize) -> Self {
        EventQueueBuilder {
            current: point(f32::NAN, f32::NAN),
            prev: point(f32::NAN, f32::NAN),
            second: point(f32::NAN, f32::NAN),
            nth: 0,
            tx: EventQueue::with_capacity(cap),
            prev_evt_is_edge: false,
        }
    }

    fn set_path(&mut self, path: impl Iterator<Item=PathEvent<Point, Point>>) {
        let mut evt_id = PathEventId(0);
        for evt in path {
            match evt {
                PathEvent::Begin { at } => {
                    self.begin(at);
                }
                PathEvent::Line { to, .. } => {
                    self.add_edge(to, evt_id);
                }
                PathEvent::Quadratic { to, .. } => {
                    // TODO: properly deal with curves!
                    self.add_edge(to, evt_id);
                }
                PathEvent::Cubic { to, .. } => {
                    // TODO: properly deal with curves!
                    self.add_edge(to, evt_id);
                }
                PathEvent::End { first, .. } => {
                    self.end(first, evt_id);
                }
            }

            evt_id.0 += 1;
        }

        // Should finish with an end event.
        debug_assert!(!self.prev_evt_is_edge);
    }

    fn set_path_with_event_ids(&mut self, path: impl Iterator<Item=(PathEvent<Point, Point>, PathEventId)>) {
        for (evt, evt_id) in path {
            match evt {
                PathEvent::Begin { at } => {
                    self.begin(at.position());
                }
                PathEvent::Line { to, .. } => {
                    self.add_edge(to.position(), evt_id);
                }
                PathEvent::Quadratic { to, .. } => {
                    // TODO: properly deal with curves!
                    self.add_edge(to.position(), evt_id);
                }
                PathEvent::Cubic { to, .. } => {
                    // TODO: properly deal with curves!
                    self.add_edge(to.position(), evt_id);
                }
                PathEvent::End { first, .. } => {
                    self.end(first.position(), evt_id);
                }
            }
        }

        // Should finish with an end event.
        debug_assert!(!self.prev_evt_is_edge);
    }

    fn vertex_event(&mut self, at: Point, evt_id: PathEventId) {
        self.tx.push(at);
        self.tx.edge_data.push(EdgeData {
            to: point(f32::NAN, f32::NAN),
            range: 0.0..0.0,
            winding: 0,
            evt_id,
            is_edge: false,
        });
    }

    fn end(&mut self, first: Point, evt_id: PathEventId) {
        if self.nth == 0 {
            return;
        }

        // Unless we are already back to the first point, we need to
        // to insert an edge.
        if self.current != first {
            self.add_edge(first, evt_id)
        }

        // Since we can only check for the need of a vertex event when
        // we have a previous edge, we skipped it for the first edge
        // and have to do it now.
        if is_after(first, self.prev) && is_after(first, self.second) {
            self.vertex_event(first, evt_id);
        }

        self.prev_evt_is_edge = false;

        self.nth = 0;
    }

    fn begin(&mut self, to: Point) {
        debug_assert!(!self.prev_evt_is_edge);

        self.nth = 0;
        self.current = to;
    }

    fn add_edge(&mut self, to: Point, evt_id: PathEventId) {
        debug_assert!(evt_id != PathEventId::INVALID);

        if self.current == to {
            return;
        }

        let mut evt_pos = self.current;
        let mut evt_to = to;
        let mut winding = 1;
        let mut t0 = 0.0;
        let mut t1 = 1.0;
        if is_after(evt_pos, to) {
            if self.nth > 0 && is_after(evt_pos, self.prev) {
                self.vertex_event(evt_pos, evt_id);
            }

            evt_to = evt_pos;
            evt_pos = to;
            swap(&mut t0, &mut t1);
            winding = -1;
        }

        self.tx.push(evt_pos);
        self.tx.edge_data.push(EdgeData {
            evt_id,
            to: evt_to,
            range: t0..t1,
            winding,
            is_edge: true,
        });

        if self.nth == 0 {
            self.second = to;
        }

        self.nth += 1;
        self.prev = self.current;
        self.current = to;
        self.prev_evt_is_edge = true;
    }

    fn build(mut self) -> EventQueue {
        debug_assert!(!self.prev_evt_is_edge);

        self.tx.sort();

        self.tx
    }
}

pub struct VertexSourceIterator<'l> {
    events: &'l EventQueue,
    id: EventId,
}

pub enum VertexSource {
    Endpoint { endpoint: EndpointId },
    OnEdge { id: PathEventId, t: f32 },
}

impl<'l> Iterator for VertexSourceIterator<'l> {
    type Item = VertexSource;
    fn next(&mut self) -> Option<VertexSource> {
        if self.id == INVALID_EVENT_ID {
            return None;
        }

        let edge = &self.events.edge_data[self.id as usize];

        self.id = self.events.next_sibling_id(self.id);

        Some(VertexSource::OnEdge {
            id: edge.evt_id,
            t: edge.range.start,
        })
    }
}

fn remap_t_in_range(val: f32, range: Range<f32>) -> f32 {
    if range.end > range.start {
        let d = range.end - range.start;
        range.start + val * d
    } else {
        let d = range.start - range.end;
        range.end + val * d
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

#[test]
fn reduced_test_case_02() {
    let mut builder = Path::builder();

    builder.move_to(point(-849.0441, 524.5503));
    builder.line_to(point(857.67084, -518.10205));
    builder.line_to(point(900.9668, -439.50897));
    builder.line_to(point(-892.3401, 445.9572));
    builder.line_to(point(-478.20224, -872.66327));
    builder.line_to(point(486.82892, 879.1116));
    builder.line_to(point(406.3725, 918.8378));
    builder.line_to(point(-397.74573, -912.3896));
    builder.line_to(point(-314.0522, -944.7439));
    builder.line_to(point(236.42209, 975.91394));
    builder.line_to(point(-227.79541, -969.4657));
    builder.line_to(point(-139.66971, -986.356));
    builder.line_to(point(148.29639, 992.80426));
    builder.line_to(point(-50.38492, -995.2788));
    builder.line_to(point(39.340546, -996.16223));
    builder.line_to(point(-30.713806, 1002.6105));
    builder.line_to(point(-120.157104, 995.44745));
    builder.line_to(point(128.78381, -988.9992));
    builder.line_to(point(217.22491, -973.84735));
    builder.line_to(point(-208.5982, 980.2956));
    builder.line_to(point(303.95184, -950.8286));
    builder.line_to(point(388.26636, -920.12854));
    builder.line_to(point(-379.63965, 926.5768));
    builder.line_to(point(-460.8624, 888.4425));
    builder.line_to(point(469.48914, -881.99426));
    builder.line_to(point(546.96686, -836.73254));
    builder.line_to(point(-538.3402, 843.1808));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );

    // SVG path syntax:
    // "M -849.0441 524.5503 L 857.67084 -518.10205 L 900.9668 -439.50897 L -892.3401 445.9572 L -478.20224 -872.66327 L 486.82892 879.1116 L 406.3725 918.8378 L -397.74573 -912.3896 L -314.0522 -944.7439 L 236.42209 975.91394 L -227.79541 -969.4657 L -139.66971 -986.356 L 148.29639 992.80426 L -50.38492 -995.2788 L 39.340546 -996.16223 L -30.713806 1002.6105 L -120.157104 995.44745 L 128.78381 -988.9992 L 217.22491 -973.84735 L -208.5982 980.2956 L 303.95184 -950.8286 L 388.26636 -920.12854 L -379.63965 926.5768 L -460.8624 888.4425 L 469.48914 -881.99426 L 546.96686 -836.73254 L -538.3402 843.1808 Z"
}

#[test]
fn reduced_test_case_03() {
    let mut builder = Path::builder();

    builder.move_to(point(997.2859, 38.078064));
    builder.line_to(point(-1000.8505, -48.24139));
    builder.line_to(point(-980.1207, -212.09396));
    builder.line_to(point(976.556, 201.93065));
    builder.line_to(point(929.13965, 360.13647));
    builder.line_to(point(-932.70435, -370.29977));
    builder.line_to(point(-859.89484, -518.5434));
    builder.line_to(point(856.33014, 508.38007));
    builder.line_to(point(760.1136, 642.6178));
    builder.line_to(point(-763.6783, -652.7811));
    builder.line_to(point(-646.6792, -769.3514));
    builder.line_to(point(643.1145, 759.188));
    builder.line_to(point(508.52423, 854.91095));
    builder.line_to(point(-512.0889, -865.0742));
    builder.line_to(point(-363.57895, -937.33875));
    builder.line_to(point(360.01428, 927.1754));
    builder.line_to(point(201.63538, 974.01044));
    builder.line_to(point(-205.20004, -984.1737));
    builder.line_to(point(-41.272438, -1004.30164));
    builder.line_to(point(37.707764, 994.1383));
    builder.line_to(point(-127.297035, 987.01013));
    builder.line_to(point(123.73236, -997.1734));
    builder.line_to(point(285.31345, -962.9835));
    builder.line_to(point(-288.8781, 952.82025));
    builder.line_to(point(-442.62796, 892.5013));
    builder.line_to(point(439.0633, -902.6646));
    builder.line_to(point(580.7881, -817.8619));
    builder.line_to(point(-584.3528, 807.6986));
    builder.line_to(point(-710.18646, 700.7254));
    builder.line_to(point(706.62177, -710.8888));
    builder.line_to(point(813.13196, -584.6631));
    builder.line_to(point(-816.69666, 574.49976));
    builder.line_to(point(-900.9784, 432.46442));
    builder.line_to(point(897.4137, -442.62775));
    builder.line_to(point(957.1676, -288.65726));
    builder.line_to(point(-960.7323, 278.49396));
    builder.line_to(point(-994.3284, 116.7885));
    builder.line_to(point(990.76373, -126.95181));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );

    // SVG path syntax:
    // "M 997.2859 38.078064 L -1000.8505 -48.24139 L -980.1207 -212.09396 L 976.556 201.93065 L 929.13965 360.13647 L -932.70435 -370.29977 L -859.89484 -518.5434 L 856.33014 508.38007 L 760.1136 642.6178 L -763.6783 -652.7811 L -646.6792 -769.3514 L 643.1145 759.188 L 508.52423 854.91095 L -512.0889 -865.0742 L -363.57895 -937.33875 L 360.01428 927.1754 L 201.63538 974.01044 L -205.20004 -984.1737 L -41.272438 -1004.30164 L 37.707764 994.1383 L -127.297035 987.01013 L 123.73236 -997.1734 L 285.31345 -962.9835 L -288.8781 952.82025 L -442.62796 892.5013 L 439.0633 -902.6646 L 580.7881 -817.8619 L -584.3528 807.6986 L -710.18646 700.7254 L 706.62177 -710.8888 L 813.13196 -584.6631 L -816.69666 574.49976 L -900.9784 432.46442 L 897.4137 -442.62775 L 957.1676 -288.65726 L -960.7323 278.49396 L -994.3284 116.7885 L 990.76373 -126.95181 Z"
}

#[test]
fn reduced_test_case_04() {
    let mut builder = Path::builder();

    builder.move_to(point(540.7645, 838.81036));
    builder.line_to(point(-534.48315, -847.5593));
    builder.line_to(point(-347.42682, -940.912));
    builder.line_to(point(151.33032, 984.5845));
    builder.line_to(point(-145.04895, -993.33344));
    builder.line_to(point(63.80545, -1002.5327));
    builder.line_to(point(-57.52408, 993.78375));
    builder.line_to(point(-263.7273, 959.35864));
    builder.line_to(point(270.00864, -968.1076));
    builder.line_to(point(464.54828, -891.56274));
    builder.line_to(point(-458.26697, 882.81384));
    builder.line_to(point(-632.64087, 767.49457));
    builder.line_to(point(638.9222, -776.2435));
    builder.line_to(point(785.5095, -627.18994));
    builder.line_to(point(-779.22815, 618.4409));
    builder.line_to(point(-891.62213, 442.1673));
    builder.line_to(point(897.9035, -450.91632));
    builder.line_to(point(971.192, -255.12662));
    builder.line_to(point(-964.9106, 246.37766));
    builder.line_to(point(-927.4177, -370.5181));
    builder.line_to(point(933.6991, 361.7691));
    builder.line_to(point(837.23865, 547.24194));
    builder.line_to(point(-830.9573, -555.9909));
    builder.line_to(point(-698.0427, -717.3555));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );

    // SVG path syntax:
    // "M 540.7645 838.81036 L -534.48315 -847.5593 L -347.42682 -940.912 L 151.33032 984.5845 L -145.04895 -993.33344 L 63.80545 -1002.5327 L -57.52408 993.78375 L -263.7273 959.35864 L 270.00864 -968.1076 L 464.54828 -891.56274 L -458.26697 882.81384 L -632.64087 767.49457 L 638.9222 -776.2435 L 785.5095 -627.18994 L -779.22815 618.4409 L -891.62213 442.1673 L 897.9035 -450.91632 L 971.192 -255.12662 L -964.9106 246.37766 L -927.4177 -370.5181 L 933.6991 361.7691 L 837.23865 547.24194 L -830.9573 -555.9909 L -698.0427 -717.3555 Z"
}

#[test]
fn reduced_test_case_05() {
    let mut builder = Path::builder();

    builder.move_to(point(540.7645, 838.81036));
    builder.line_to(point(-534.48315, -847.5593));
    builder.line_to(point(-347.42682, -940.912));
    builder.line_to(point(353.70816, 932.163));
    builder.line_to(point(151.33032, 984.5845));
    builder.line_to(point(-145.04895, -993.33344));
    builder.line_to(point(63.80545, -1002.5327));
    builder.line_to(point(-263.7273, 959.35864));
    builder.line_to(point(270.00864, -968.1076));
    builder.line_to(point(464.54828, -891.56274));
    builder.line_to(point(-458.26697, 882.81384));
    builder.line_to(point(-632.64087, 767.49457));
    builder.line_to(point(638.9222, -776.2435));
    builder.line_to(point(785.5095, -627.18994));
    builder.line_to(point(-779.22815, 618.4409));
    builder.line_to(point(-891.62213, 442.1673));
    builder.line_to(point(897.9035, -450.91632));
    builder.line_to(point(971.192, -255.12662));
    builder.line_to(point(-964.9106, 246.37766));
    builder.line_to(point(-995.89075, 39.628937));
    builder.line_to(point(1002.1721, -48.3779));
    builder.line_to(point(989.48975, 160.29398));
    builder.line_to(point(-983.2084, -169.04297));
    builder.line_to(point(-927.4177, -370.5181));
    builder.line_to(point(933.6991, 361.7691));
    builder.line_to(point(837.23865, 547.24194));
    builder.line_to(point(-830.9573, -555.9909));
    builder.line_to(point(-698.0427, -717.3555));
    builder.line_to(point(704.3241, 708.6065));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );

    // SVG path syntax:
    // "M 540.7645 838.81036 L -534.48315 -847.5593 L -347.42682 -940.912 L 353.70816 932.163 L 151.33032 984.5845 L -145.04895 -993.33344 L 63.80545 -1002.5327 L -263.7273 959.35864 L 270.00864 -968.1076 L 464.54828 -891.56274 L -458.26697 882.81384 L -632.64087 767.49457 L 638.9222 -776.2435 L 785.5095 -627.18994 L -779.22815 618.4409 L -891.62213 442.1673 L 897.9035 -450.91632 L 971.192 -255.12662 L -964.9106 246.37766 L -995.89075 39.628937 L 1002.1721 -48.3779 L 989.48975 160.29398 L -983.2084 -169.04297 L -927.4177 -370.5181 L 933.6991 361.7691 L 837.23865 547.24194 L -830.9573 -555.9909 L -698.0427 -717.3555 L 704.3241 708.6065 Z"
}

#[test]
fn reduced_test_case_06() {
    let mut builder = Path::builder();

    builder.move_to(point(831.9957, 561.9206));
    builder.line_to(point(-829.447, -551.4562));
    builder.line_to(point(-505.64172, -856.7632));
    builder.line_to(point(508.19046, 867.2276));
    builder.line_to(point(83.98413, 1001.80585));
    builder.line_to(point(-81.435394, -991.34143));
    builder.line_to(point(359.1525, -928.5361));
    builder.line_to(point(-356.60376, 939.0005));
    builder.line_to(point(-726.3096, 691.25085));
    builder.line_to(point(728.8583, -680.78644));
    builder.line_to(point(-951.90845, 307.6267));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );

    // SVG path syntax:
    // "M 831.9957 561.9206 L -829.447 -551.4562 L -505.64172 -856.7632 L 508.19046 867.2276 L 83.98413 1001.80585 L -81.435394 -991.34143 L 359.1525 -928.5361 L -356.60376 939.0005 L -726.3096 691.25085 L 728.8583 -680.78644 L -951.90845 307.6267 Z"
}

#[test]
fn reduced_test_case_07() {
    let mut builder = Path::builder();

    builder.move_to(point(960.5097, -271.01678));
    builder.line_to(point(-967.03217, 262.446));
    builder.line_to(point(-987.3192, -182.13324));
    builder.line_to(point(980.7969, 173.56247));
    builder.line_to(point(806.1792, 582.91675));
    builder.line_to(point(-812.7016, -591.48755));
    builder.line_to(point(-477.76422, -884.53925));
    builder.line_to(point(471.24182, 875.9685));
    builder.line_to(point(42.32347, 994.6751));
    builder.line_to(point(-48.845886, -1003.2459));
    builder.line_to(point(389.10114, -924.0962));
    builder.line_to(point(-395.62357, 915.5254));
    builder.line_to(point(-755.85846, 654.19574));
    builder.line_to(point(749.3361, -662.7665));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );

    // SVG path syntax:
    // "M 960.5097 -271.01678 L -967.03217 262.446 L -987.3192 -182.13324 L 980.7969 173.56247 L 806.1792 582.91675 L -812.7016 -591.48755 L -477.76422 -884.53925 L 471.24182 875.9685 L 42.32347 994.6751 L -48.845886 -1003.2459 L 389.10114 -924.0962 L -395.62357 915.5254 L -755.85846 654.19574 L 749.3361 -662.7665 Z"
}

#[test]
fn reduced_test_case_08() {
    let mut builder = Path::builder();

    builder.move_to(point(-85.92998, 24.945076));
    builder.line_to(point(-79.567345, 28.325748));
    builder.line_to(point(-91.54697, 35.518726));
    builder.line_to(point(-85.92909, 24.945545));
    builder.close();

    builder.move_to(point(-57.761955, 34.452206));
    builder.line_to(point(-113.631676, 63.3717));
    builder.line_to(point(-113.67784, 63.347214));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );

    // SVG path syntax:
    // "M -85.92998 24.945076 L -79.567345 28.325748 L -91.54697 35.518726 L -85.92909 24.945545 ZM -57.761955 34.452206 L -113.631676 63.3717 L -113.67784 63.347214 Z"
}

#[test]
fn reduced_test_case_09() {
    let mut builder = Path::builder();

    builder.move_to(point(659.9835, 415.86328));
    builder.line_to(point(70.36328, 204.36978));
    builder.line_to(point(74.12529, 89.01107));
    builder.close();

    builder.move_to(point(840.2258, 295.46188));
    builder.line_to(point(259.41193, 272.18054));
    builder.line_to(point(728.914, 281.41678));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );

    // SVG path syntax:
    // "M 659.9835 415.86328 L 70.36328 204.36978 L 74.12529 89.01107 ZM 840.2258 295.46188 L 259.41193 272.18054 L 728.914 281.41678 Z"
}

#[test]
fn reduced_test_case_10() {
    let mut builder = Path::builder();

    builder.move_to(point(993.5114, -94.67855));
    builder.line_to(point(-938.76056, -355.94995));
    builder.line_to(point(933.8779, 346.34995));
    builder.line_to(point(-693.6775, -727.42883));
    builder.line_to(point(-311.68665, -955.7822));
    builder.line_to(point(306.80408, 946.1823));
    builder.line_to(point(-136.43655, 986.182));
    builder.line_to(point(131.55396, -995.782));
    builder.line_to(point(548.25525, -839.50555));
    builder.line_to(point(-553.13776, 829.9056));
    builder.line_to(point(-860.76697, 508.30533));
    builder.line_to(point(855.88434, -517.90533));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );

    // SVG path syntax:
    // "M 993.5114 -94.67855 L -938.76056 -355.94995 L 933.8779 346.34995 L -693.6775 -727.42883 L -311.68665 -955.7822 L 306.80408 946.1823 L -136.43655 986.182 L 131.55396 -995.782 L 548.25525 -839.50555 L -553.13776 829.9056 L -860.76697 508.30533 L 855.88434 -517.90533 Z"
}

#[test]
fn reduced_test_case_11() {
    let mut builder = Path::builder();

    builder.move_to(point(10.0095005, 0.89995164));
    builder.line_to(point(10.109498, 10.899451));
    builder.line_to(point(0.10999817, 10.99945));
    builder.close();

    builder.move_to(point(19.999, -0.19999667));
    builder.line_to(point(20.098999, 9.799503));
    builder.line_to(point(10.099499, 9.899502));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );

    // SVG path syntax:
    // "M 10.0095005 0.89995164 L 10.109498 10.899451 L 0.10999817 10.99945 ZM 19.999 -0.19999667 L 20.098999 9.799503 L 10.099499 9.899502 Z"
}

#[test]
fn reduced_test_case_12() {
    let mut builder = Path::builder();

    builder.move_to(point(5.5114865, -8.40378));
    builder.line_to(point(14.377752, -3.7789207));
    builder.line_to(point(9.7528925, 5.0873456));
    builder.close();

    builder.move_to(point(4.62486, -8.866266));
    builder.line_to(point(18.115986, -13.107673));
    builder.line_to(point(13.491126, -4.2414064));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    );

    // SVG path syntax:
    // "M 5.5114865 -8.40378 L 14.377752 -3.7789207 L 9.7528925 5.0873456 ZM 4.62486 -8.866266 L 18.115986 -13.107673 L 13.491126 -4.2414064 Z"
}
