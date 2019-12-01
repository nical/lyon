use crate::{FillOptions, Side, InternalError, TessellationResult, TessellationError};
use crate::geom::math::*;
use crate::geom::LineSegment;
use crate::geometry_builder::{FillGeometryBuilder, VertexId, VertexSource};
use crate::event_queue::*;
use crate::monotone::*;
use crate::path::{PathEvent, FillRule, Transition};
use std::f32;
use std::cmp::Ordering;
use std::ops::Range;

#[cfg(debug_assertions)]
use std::env;

type SpanIdx = i32;
type ActiveEdgeIdx = usize;

#[cfg(debug_assertions)]
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

#[cfg(not(debug_assertions))]
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
    vertex_events: Vec<(SpanIdx, Side)>,
    edges_to_split: Vec<ActiveEdgeIdx>,
    spans_to_end: Vec<SpanIdx>,
    merge_event: bool,
    split_event: bool,
    merge_split_event: bool,
    // TODO: store as range
    above_start: ActiveEdgeIdx,
    above_end: ActiveEdgeIdx,
    winding_before_point: WindingState,
}

impl ActiveEdgeScan {
    fn new() -> Self {
        ActiveEdgeScan {
            vertex_events: Vec::new(),
            edges_to_split: Vec::new(),
            spans_to_end: Vec::new(),
            merge_event: false,
            split_event: false,
            merge_split_event: false,
            above_start: 0,
            above_end: 0,
            winding_before_point: WindingState::new(),
        }
    }

    fn reset(&mut self) {
        self.vertex_events.clear();
        self.edges_to_split.clear();
        self.spans_to_end.clear();
        self.merge_event = false;
        self.split_event = false;
        self.merge_split_event = false;
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
    src_edge: TessEventId,

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
        output: &mut dyn FillGeometryBuilder,
    ) {
        let idx = span_idx as usize;

        debug_assert!(!self.spans[idx].remove);

        let span = &mut self.spans[idx];
        span.remove = true;
        span.tess.end(*position, id);
        span.tess.flush(output);
    }

    fn merge_spans(
        &mut self,
        left_span_idx: SpanIdx,
        current_position: &Point,
        current_vertex: VertexId,
        merge_position: &Point,
        merge_vertex: VertexId,
        output: &mut dyn FillGeometryBuilder,
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
    src_edge: TessEventId,
    winding: i16,
    range_end: f32,
}

pub struct FillTessellator {
    current_position: Point,
    current_vertex: VertexId,
    current_event_id: TessEventId,
    active: ActiveEdges,
    edges_below: Vec<PendingEdge>,
    fill_rule: FillRule,
    fill: Spans,
    log: bool,
    assume_no_intersection: bool,

    events: EventQueue,
}


impl FillTessellator {
    pub fn new() -> Self {
        #[cfg(debug_assertions)]
        let log = env::var("LYON_FORCE_LOGGING").is_ok();
        #[cfg(not(debug_assertions))]
        let log = false;

        FillTessellator {
            current_position: point(f32::MIN, f32::MIN),
            current_vertex: VertexId::INVALID,
            current_event_id: INVALID_EVENT_ID,
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
        }
    }

    pub fn create_event_queue(&mut self) -> EventQueue {
        std::mem::replace(&mut self.events, EventQueue::new())
    }

    pub fn tessellate_path<Iter>(
        &mut self,
        path: Iter,
        options: &FillOptions,
        builder: &mut dyn FillGeometryBuilder
    ) -> TessellationResult
    where
        Iter: IntoIterator<Item = PathEvent>,
    {

        let mut queue_builder = self.create_event_queue().into_builder();

        queue_builder.set_path(options.tolerance, path.into_iter());

        let mut event_queue = queue_builder.build();

        std::mem::swap(&mut self.events, &mut event_queue);

        self.tessellate_impl(options, builder)
    }

    pub fn tessellate_events(
        &mut self,
        events: &mut EventQueue,
        options: &FillOptions,
        builder: &mut dyn FillGeometryBuilder
    ) -> TessellationResult {

        std::mem::swap(&mut self.events, events);

        let result = self.tessellate_impl(options, builder);

        std::mem::swap(&mut self.events, events);

        result
    }

    fn tessellate_impl(
        &mut self,
        options: &FillOptions,
        builder: &mut dyn FillGeometryBuilder
    ) -> TessellationResult {
        self.reset();

        self.fill_rule = options.fill_rule;

        builder.begin_geometry();

        let result = self.tessellator_loop(builder);

        if let Err(e) = result {
            tess_log!(self, "Tessellation failed with error: {:?}.", e);
            builder.abort_geometry();

            return Err(e);
        }

        if !self.assume_no_intersection {
            debug_assert!(self.active.edges.is_empty());
            debug_assert!(self.fill.spans.is_empty());
        }

        // There shouldn't be any span left after the tessellation ends.
        // If for whatever reason (bug) there are, flush them so that we don't
        // miss the triangles they contain.
        for span in &mut self.fill.spans {
            if !span.remove {
                span.tess.flush(builder);
            }
        }

        self.fill.spans.clear();

        Ok(builder.end_geometry())
    }

    pub fn enable_logging(&mut self) {
        self.log = true;
    }

    fn tessellator_loop(
        &mut self,
        output: &mut dyn FillGeometryBuilder
    ) -> Result<(), TessellationError> {
        log_svg_preamble(self);

        let mut scan = ActiveEdgeScan::new();
        let mut _prev_position = point(std::f32::MIN, std::f32::MIN);
        self.current_event_id = self.events.first_id();
        while self.events.valid_id(self.current_event_id) {

            self.initialize_events(output)?;

            debug_assert!(is_after(self.current_position, _prev_position));
            _prev_position = self.current_position;

            if let Err(e) = self.process_events(&mut scan, output) {
                // Something went wrong, attempt to salvage the state of the sweep
                // line
                self.recover_from_error(e);
                // ... and try again.
                self.process_events(&mut scan, output)?
            }

            #[cfg(debug_assertions)]
            self.check_active_edges();

            self.current_event_id = self.events.next_id(self.current_event_id);
        }

        Ok(())
    }

    fn initialize_events(&mut self, output: &mut dyn FillGeometryBuilder) -> Result<(), TessellationError> {
        let current_event = self.current_event_id;

        tess_log!(self, "\n\n<!--         event #{}          -->", current_event);

        self.current_position = self.events.position(current_event);

        let mut src = VertexSourceIterator {
            events: &self.events,
            id: current_event,
        };

        self.current_vertex = output.add_fill_vertex(self.current_position, &mut src)?;

        let mut current_sibling = current_event;
        while self.events.valid_id(current_sibling) {
            let edge = &self.events.edge_data[current_sibling as usize];
            // We insert "fake" edges when there are end events
            // to make sure we process that vertex even if it has
            // no edge below.
            if edge.is_edge {
                let to = edge.to;
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

        Ok(())
    }

    /// An iteration of the sweep line algorithm.
    fn process_events(
        &mut self,
        scan: &mut ActiveEdgeScan,
        output: &mut dyn FillGeometryBuilder,
    ) -> Result<(), InternalError> {
        debug_assert!(!self.current_position.x.is_nan() && !self.current_position.y.is_nan());

        tess_log!(self, "<!--");
        tess_log!(self, "     events at {:?} {:?}         {} edges below",
            self.current_position,
            self.current_vertex,
            self.edges_below.len(),
        );

        // Step 1 - Scan the active edge list, deferring processing and detecting potential
        // ordering issues in the active edges.
        self.scan_active_edges(scan)?;

        // Step 2 - Do the necessary processing on edges that end at the current point.
        self.process_edges_above(scan, output);

        // Step 3 - Do the necessary processing on edges that start at the current point.
        self.process_edges_below(scan);

        // Step 4 - Insert/remove edges to the active edge as necessary and handle
        // potential self-intersections.
        self.update_active_edges(scan);

        tess_log!(self, "-->");

        #[cfg(debug_assertions)]
        self.log_active_edges();

        Ok(())
    }

    #[cfg(debug_assertions)]
    fn log_active_edges(&self) {

        tess_log!(self, r#"<g class="active-edges">"#);
        tess_log!(self, r#"<path d="M 0 {} L 1000 {}" class="sweep-line"/>"#,
            self.current_position.y, self.current_position.y
        );
        tess_log!(self, "<!-- active edges: -->");
        for e in &self.active.edges {
            if e.is_merge {
                tess_log!(self, r#"  <circle cx="{}" cy="{}" r="3px" class="merge"/>"#,
                    e.from.x, e.from.y
                );
            } else {
                tess_log!(self, r#"  <path d="M {} {} L {} {}" class="edge", winding="{}" sort_x="{:.}" min_x="{:.}"/>"#,
                    e.from.x, e.from.y,
                    e.to.x, e.to.y,
                    e.winding,
                    e.sort_x,
                    e.min_x,
                );
            }
        }
        tess_log!(self, "<!-- spans: {}-->", self.fill.spans.len());
        tess_log!(self, "</g>");
    }

    #[cfg(debug_assertions)]
    fn check_active_edges(&self) {
        let mut winding = 0;
        for edge in &self.active.edges {
            if edge.is_merge {
                assert!(self.fill_rule.is_in(winding));
            } else {
                assert!(!is_after(self.current_position, edge.to));
                winding += edge.winding;
            }
        }
        assert_eq!(winding, 0);
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
    fn scan_active_edges(&self, scan: &mut ActiveEdgeScan) -> Result<(), InternalError> {

        scan.reset();

        let current_x = self.current_position.x;
        let mut connecting_edges = false;
        let mut active_edge_idx = 0;
        let mut winding = WindingState::new();
        let mut previous_was_merge = false;

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
                previous_was_merge = true;

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
            previous_was_merge = false;
            active_edge_idx += 1;

            tess_log!(self, " > {:?} (span {})", winding.transition, winding.span_index);
        }

        scan.above_start = active_edge_idx;
        scan.winding_before_point = winding.clone();

        if previous_was_merge {
            scan.winding_before_point.span_index -= 1;
            scan.above_start -= 1;

            // First connecting edge is a merge.
            //  ...:./.      ...:...
            //  ...:/..  or  ...:...
            //  ...X...      ...X...
            //
            // The span on the left does not end here but it has a vertex
            // on its right side.
            //
            // The next loop can now assume that merge edges can't make the first
            // transition connecting with the current vertex,

            if !connecting_edges {
                // There are no edges left and right of the merge that connect with
                // the current vertex. In other words the merge is the only edge
                // connecting and there must be a split event formed by two edges
                // below the current vertex.
                //
                // In this case we don't end any span and we skip splitting. The merge
                // and the split cancel each-other out.
                //
                //  ...:...
                //  ...:...
                //  ...x...
                //  ../ \..
                scan.vertex_events.push((winding.span_index - 1, Side::Right));
                scan.vertex_events.push((winding.span_index, Side::Left));
                scan.merge_split_event = true;
                tess_log!(self, "split+merge");
            }
        }

        //  .......
        //  ...x...
        //  ../ \..
        scan.split_event = !connecting_edges
            && self.fill_rule.is_in(winding.number)
            && !scan.merge_split_event;


        // Step 2 - Iterate over edges connecting with the current point.

        tess_log!(self, "connecting_edges {} | edge {} | span {}", connecting_edges, active_edge_idx, winding.span_index);
        if connecting_edges {

            // First transition while connecting with the current vertex.
            let mut first_transition = if previous_was_merge {
                Transition::Out
            } else {
                Transition::None
            };

            // Previous transition while connecting with the current vertex.
            let mut previous_transition = if previous_was_merge {
                Transition::In
            } else {
                Transition::None
            };

            for active_edge in &self.active.edges[active_edge_idx..] {
                if active_edge.is_merge {
                    if winding.transition == Transition::Out {
                        return Err(InternalError::MergeVertexOutside);
                    }
                    debug_assert_eq!(previous_transition, Transition::In);

                    // Merge above the current vertex to resolve.
                    //
                    // Resolving a merge usually leads to a span adjacent to the merge
                    // ending.
                    //
                    // If there was already an edge connecting with the current vertex
                    // just left of the merge edge, we can end the span between that edge
                    // and the merge.
                    //
                    //    |
                    //    v
                    //  \...:...
                    //  .\..:...
                    //  ..\.:...
                    //  ...\:...
                    //  ....X...
                    scan.spans_to_end.push(winding.span_index);

                    // To deal with the right side of the merge, we simply pretend it
                    // transitioned into the shape. Next edge that transitions out (if any)
                    // will close out the span as if it was surrounded be regular edges.
                    //
                    //       |
                    //       v
                    //  ...:.../
                    //  ...:../
                    //  ...:./
                    //  ...:/
                    //  ...X
                    previous_transition = Transition::In;

                    winding.span_index += 1;
                    active_edge_idx += 1;

                    continue;
                }

                if !self.is_edge_connecting(active_edge, active_edge_idx, scan)? {
                    break;
                }

                winding.update(self.fill_rule, active_edge.winding);
                tess_log!(self, " x {:?} (span {})", winding.transition, winding.span_index);

                if winding.transition == Transition::In && winding.span_index >= self.fill.spans.len() as i32 {
                    return Err(InternalError::InsufficientNumberOfSpans);
                }

                if winding.transition == Transition::Out && previous_transition != Transition::None {
                    debug_assert_eq!(previous_transition, Transition::In);

                    // End event.
                    //
                    //  \.../
                    //   \./
                    //    x
                    //
                    scan.spans_to_end.push(winding.span_index);
                }

                if winding.transition != Transition::None {
                    previous_transition = winding.transition;

                    if first_transition == Transition::None {
                        first_transition = winding.transition;
                    }
                }

                active_edge_idx += 1;
            }

            let vertex_is_merge_event = first_transition == Transition::Out
                && previous_transition == Transition::In
                && self.edges_below.is_empty()
                && scan.edges_to_split.is_empty();

            tess_log!(self, "first_transition: {:?}, last_transition: {:?} is merge: {:?}",
                first_transition, previous_transition, vertex_is_merge_event,
            );

            if vertex_is_merge_event {
                //  .\   /.      .\ |./ /.
                //  ..\ /..      ..\|//...
                //  ...x...  or  ...x.....  (etc.)
                //  .......      .........
                scan.merge_event = true;
            }

            if first_transition == Transition::Out {
                //   ...|         ..\ /..
                //   ...x    or   ...x...  (etc.)
                //   ...|         ...:...
                let first_span_index = scan.winding_before_point.span_index;
                scan.vertex_events.push((first_span_index, Side::Right));
            }

            if previous_transition == Transition::In {
                //    |...        ..\ /..
                //    x...   or   ...x...  (etc.)
                //    |...        ...:...
                scan.vertex_events.push((winding.span_index, Side::Left));
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
                return Err(InternalError::IncorrectActiveEdgeOrder(1));
            }

            if points_are_equal(self.current_position, active_edge.to) {
                return Err(InternalError::IncorrectActiveEdgeOrder(2));
            }

            if active_edge.min_x < current_x
                && active_edge.solve_x_for_y(self.current_position.y) < current_x {
                return Err(InternalError::IncorrectActiveEdgeOrder(3));
            }
        }


        Ok(())
    }

    // Returns Ok(true) if the edge connects with the current vertex, Ok(false) otherwise.
    // Returns Err if the active edge order is wrong.
    fn is_edge_connecting(&self, active_edge: &ActiveEdge, active_edge_idx: usize, scan: &mut ActiveEdgeScan) -> Result<bool, InternalError> {
        if points_are_equal(self.current_position, active_edge.to) {
            return Ok(true)
        }

        let current_x = self.current_position.x;
        let threshold = 0.001;

        if active_edge.max_x + threshold < current_x || active_edge.to.y < self.current_position.y {
            return Err(InternalError::IncorrectActiveEdgeOrder(4));
        }

        if active_edge.min_x > current_x {
            return Ok(false);
        }

        let ex = if active_edge.from.y != active_edge.to.y {
            active_edge.solve_x_for_y(self.current_position.y)
        } else if active_edge.max_x >= current_x && active_edge.min_x <= current_x {
            current_x
        } else {
            active_edge.to.y
        };

        if (ex - current_x).abs() <= threshold {
            tess_log!(self, "vertex on an edge! {:?} -> {:?}", active_edge.from, active_edge.to);
            scan.edges_to_split.push(active_edge_idx);
            return Ok(true);
        }

        if ex < current_x {
            return Err(InternalError::IncorrectActiveEdgeOrder(5));
        }

        tess_log!(self, "ex = {:?} (diff={})", ex, ex - current_x);

        Ok(false)
    }

    fn process_edges_above(&mut self, scan: &mut ActiveEdgeScan, output: &mut dyn FillGeometryBuilder) {
        for &(span_index, side) in &scan.vertex_events {
            tess_log!(self, "   -> Vertex {:?} / {:?}", span_index, side);
            self.fill.spans[span_index as usize].tess.vertex(
                self.current_position,
                self.current_vertex,
                side,
            );
        }

        for &span_index in &scan.spans_to_end {
            tess_log!(self, "   -> End span {:?}", span_index);
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
            active_edge.min_x = active_edge.min_x.min(self.current_position.x)
        }

        if scan.merge_event {
            // Merge event.
            //
            //  ...\   /...
            //  ....\ /....
            //  .....x.....
            //
            let edge = &mut self.active.edges[scan.above_start];
            edge.is_merge = true;
            edge.from = edge.to;
            edge.min_x = edge.to.x;
            edge.max_x = edge.to.x;
            edge.winding = 0;
            edge.from_id = self.current_vertex;

            // take the merge edge out of the range so that it isn't removed later.
            scan.above_start += 1;
        }
    }

    fn process_edges_below(&mut self, scan: &mut ActiveEdgeScan) {
        let mut winding = scan.winding_before_point.clone();

        tess_log!(self, "connecting edges: {}..{} {:?}", scan.above_start, scan.above_end, winding.transition);
        tess_log!(self, "winding state before point: {:?}", winding);
        tess_log!(self, "edges below: {:?}", self.edges_below);

        self.sort_edges_below();

        if scan.split_event {
            debug_assert!(self.edges_below.len() >= 2);

            // Split event.
            //
            //  ...........
            //  .....x.....
            //  ..../ \....
            //  .../   \...
            //

            let left_enclosing_edge_idx = scan.above_start - 1;
            self.split_event(
                left_enclosing_edge_idx,
                winding.span_index,
            );
        }

        // Go through the edges that start at the current point and emit
        // start events for each time an in-out pair is found.

        let mut prev_transition_in = false;
        for pending_edge in &self.edges_below {

            winding.update(self.fill_rule, pending_edge.winding);

            tess_log!(self, "edge below: {:?} span {}", winding.transition, winding.span_index);

            match winding.transition {
                Transition::In => {
                    prev_transition_in = true;
                }
                Transition::Out => {
                    if prev_transition_in {
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

    fn update_active_edges(&mut self, scan: &ActiveEdgeScan) {
        let above = scan.above_start..scan.above_end;

        tess_log!(self, " remove {} edges ({}..{})", above.end - above.start, above.start, above.end);
        for active_edge_idx in above.clone().rev() {
            debug_assert!(
                self.active.edges[active_edge_idx].is_merge
                || !is_after(self.current_position, self.active.edges[active_edge_idx].to)
            );
            self.active.edges.remove(active_edge_idx);
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

    fn split_event(&mut self, left_enclosing_edge_idx: ActiveEdgeIdx, left_span_idx: SpanIdx) {
        let right_enclosing_edge_idx = left_enclosing_edge_idx + 1;

        // TODO: we should be reasoning in transitions and not in edges.
        let upper_left = self.active.edges[left_enclosing_edge_idx].from;
        let upper_right = self.active.edges[right_enclosing_edge_idx].from;

        let right_span_idx = left_span_idx + 1;

        let (upper_position, upper_id, new_span_idx) = if is_after(upper_left, upper_right) {
            //                |.....
            // upper_left --> x.....
            //               /.:....
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
            //                          ....:.\
            // current split vertex --> ...x...\
            //                          ../ \...\
            (
                upper_right,
                self.active.edges[right_enclosing_edge_idx].from_id,
                right_span_idx,
            )
        };

        self.fill.spans.insert(
            new_span_idx as usize,
            Span {
                tess: MonotoneTessellator::new().begin(upper_position, upper_id),
                remove: false,
            }
        );

        debug_assert!(!self.fill.spans[left_span_idx as usize].remove);
        debug_assert!(!self.fill.spans[right_span_idx as usize].remove);
        self.fill.spans[left_span_idx as usize].tess.vertex(self.current_position, self.current_vertex, Side::Right);
        self.fill.spans[right_span_idx as usize].tess.vertex(self.current_position, self.current_vertex, Side::Left);
    }

    fn handle_intersections(&mut self) {
        // Do intersection checks for all of the new edges against already active edges.
        //
        // If several intersections are found on the same edges we only keep the top-most.
        // the active and new edges are then truncated at the intersection position and the
        // lower parts are added to the event queue.
        //
        // In order to not break invariants of the sweep line we need to ensure that:
        // - the intersection position is never ordered before the current position,
        // - after truncation, edges continue being oriented downwards,
        // - the cached min_x value of the active edge is still correct.
        //
        // Floating-point precision (or the lack thereof) prevent us from taking the
        // above properties from granted even though they make sense from a purely
        // geometrical perspective. Therefore we have to take great care in checking
        // whether these invariants aren't broken by the insertion of the intersection,
        // manually fixing things up if need be and making sure to not break more
        // invariants in doing so.

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
                    // We can't early out because there might be edges further on the right
                    // that extend further on the left which would be missed.
                    //
                    // sweep line -> =o===/==/==
                    //                |\ /  /
                    //                | o  /
                    //  edge below -> |   /
                    //                |  /
                    //                | / <- missed active edge
                    //                |/
                    //                x <- missed intersection
                    //               /|
                    continue;
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

                if is_near(self.current_position, intersection_position) {
                    tess_log!(self, "fix intersection position to current_position");
                    intersection_position = self.current_position;
                    // We moved the intersection to the current position to avoid breaking ordering.
                    // This means we won't be adding an intersection event and we have to treat
                    // splitting the two edges in a special way:
                    // - the edge below does not need to be split.
                    // - the active edge is split so that it's upper part now ends at the current
                    //   position which means it must be removed, however removing edges ending at
                    //   the current position happens before the intersection checks. So instead we
                    //   modify it in place and don't add a new event.
                    active_edge.from = intersection_position;
                    active_edge.min_x = active_edge.min_x.min(intersection_position.x);
                    let src_range = &mut self.events.edge_data[active_edge.src_edge as usize].range;
                    let remapped_ta = remap_t_in_range(
                        ta as f32,
                        src_range.start..active_edge.range_end,
                    );
                    src_range.start = remapped_ta;

                    continue;
                }

                if intersection_position.y < self.current_position.y {
                    tess_log!(self, "fixup the intersection because of y coordinate");
                    intersection_position.y = self.current_position.y + 0.0001; // TODO
                } else if intersection_position.y == self.current_position.y
                    && intersection_position.x < self.current_position.x {
                    tess_log!(self, "fixup the intersection because of x coordinate");
                    intersection_position.y = self.current_position.y + 0.0001; // TODO
                }

                if is_near(intersection_position, edge_below.to) {
                    tess_log!(self, "intersection near below.to");
                    intersection_position = edge_below.to;
                } else if is_near(intersection_position, active_edge.to) {
                    tess_log!(self, "intersection near below.to");
                    intersection_position = active_edge.to;
                }

                let a_src_edge_data = self.events.edge_data[active_edge.src_edge as usize].clone();
                let b_src_edge_data = self.events.edge_data[edge_below.src_edge as usize].clone();

                let mut inserted_evt = None;

                if active_edge.to != intersection_position
                    && active_edge.from != intersection_position {
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
                            },
                            self.current_event_id,
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
                            },
                            self.current_event_id,
                        );
                    }

                    active_edge.to = intersection_position;
                    active_edge.min_x = active_edge.min_x.min(intersection_position.x);
                    active_edge.range_end = remapped_ta;
                }

                debug_assert!(active_edge.min_x <= active_edge.from.x);
                debug_assert!(active_edge.min_x <= active_edge.to.x);

                if edge_below.to != intersection_position
                    && self.current_position != intersection_position {

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
                            self.events.insert_sorted(intersection_position, edge_data, self.current_event_id);
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
                            },
                            self.current_event_id,
                        );
                    };

                    edge_below.to = intersection_position;
                    edge_below.range_end = remapped_tb;
                }
            }
        }

        //self.log_active_edges();
    }

    fn sort_active_edges(&mut self) {
        // Merge edges are a little subtle when it comes to sorting.
        // They are points rather than edges and the best we can do is
        // keep their relative ordering with their previous or next edge.
        // Unfortunately this can cause merge vertices to end up outside of
        // the shape.
        // After sorting we go through the active edges and rearrange merge
        // vertices to prevent that.

        let y = self.current_position.y;

        let mut has_merge_vertex = false;
        let mut prev_x = f32::NAN;
        for edge in &mut self.active.edges {
            if edge.is_merge {
                debug_assert!(!prev_x.is_nan());
                has_merge_vertex = true;
                edge.sort_x = prev_x;
            } else {
                debug_assert!(!is_after(self.current_position, edge.to));

                let x = if edge.to.y == y {
                    edge.to.x
                } else if edge.from.y == y {
                    edge.from.x
                } else {
                    edge.solve_x_for_y(y)
                };

                edge.sort_x = x.max(edge.min_x);
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

        if !has_merge_vertex {
            return;
        }

        let mut winding_number = 0;
        for i in 0..self.active.edges.len() {
            let needs_swap = {
                let edge = &self.active.edges[i];
                if edge.is_merge {
                    !self.fill_rule.is_in(winding_number)
                } else {
                    winding_number += edge.winding;
                    false
                }
            };

            if needs_swap {
                let mut w = winding_number;
                tess_log!(self, "Fixing up merge vertex after sort.");
                let mut idx = i;
                loop {
                    // Roll back previous edge winding and swap.
                    w -= self.active.edges[idx-1].winding;
                    self.active.edges.swap(idx, idx-1);

                    if self.fill_rule.is_in(w) {
                        break;
                    }

                    idx -= 1;
                }
            }
        }
    }

    fn recover_from_error(&mut self, _error: InternalError) {
        tess_log!(self, "Attempt to recover error {:?}", _error);

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

        tess_log!(self, "-->");

        #[cfg(debug_assertions)]
        self.log_active_edges();
    }

    fn sort_edges_below(&mut self) {
        self.edges_below.sort_by(|a, b| {
            b.angle.partial_cmp(&a.angle).unwrap_or(Ordering::Equal)
        });
    }

    fn reset(&mut self) {
        self.current_position = point(f32::MIN, f32::MIN);
        self.current_vertex = VertexId::INVALID;
        self.current_event_id = INVALID_EVENT_ID;
        self.active.edges.clear();
        self.edges_below.clear();
        self.fill.spans.clear();
    }
}

pub(crate) fn points_are_equal(a: Point, b: Point) -> bool {
    // TODO: Use the tolerance threshold?
    a == b
}


pub(crate) fn compare_positions(a: Point, b: Point) -> Ordering {
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
pub(crate) fn is_after(a: Point, b: Point) -> bool {
    a.y > b.y || (a.y == b.y && a.x > b.x)
}

#[inline]
pub(crate) fn is_near(a: Point, b: Point) -> bool {
    (a - b).square_length() < 0.0001
}

#[derive(Clone)]
pub struct VertexSourceIterator<'l> {
    events: &'l EventQueue,
    id: TessEventId,
}

impl<'l> Iterator for VertexSourceIterator<'l> {
    type Item = VertexSource;
    fn next(&mut self) -> Option<VertexSource> {
        if self.id == INVALID_EVENT_ID {
            return None;
        }

        let edge = &self.events.edge_data[self.id as usize];

        self.id = self.events.next_sibling_id(self.id);

        let t = edge.range.start;

        if t == 0.0 {
            Some(VertexSource::Endpoint { id: edge.from_id })
        } else if t == 1.0 {
            Some(VertexSource::Endpoint { id: edge.to_id })
        } else {
            Some(VertexSource::Edge {
                edge: edge.evt_id,
                from: edge.from_id,
                to: edge.to_id,
                t,
            })
        }
    }
}

fn remap_t_in_range(val: f32, range: Range<f32>) -> f32 {
    if range.end > range.start {
        let d = range.end - range.start;
        range.start + val * d
    } else {
        let d = range.start - range.end;
        range.end + (1.0 - val) * d
    }
}

fn log_svg_preamble(tess: &FillTessellator) {
    tess_log!(tess, r#"
<svg viewBox="0 0 1000 1000">

<style type="text/css">
<![CDATA[
  path.sweep-line {{
    stroke: red;
    fill: none;
  }}

  path.edge {{
    stroke: blue;
    fill: none;
  }}

  path.edge.select {{
    stroke: green;
    fill: none;
  }}

  circle.merge {{
    fill: yellow;
    stroke: orange;
    fill-opacity: 1;
  }}

  circle.current {{
    fill: white;
    stroke: grey;
    fill-opacity: 1;
  }}

  g.active-edges {{
    opacity: 0;
  }}

  g.active-edges.select {{
    opacity: 1;
  }}
]]>
</style>
"#
    );
}

#[cfg(test)]
use crate::geometry_builder::*;

#[cfg(test)]
fn eq(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < 0.00001 && (a.y - b.y).abs() < 0.00001
}

#[cfg(test)]
fn at_endpoint(src: &VertexSource, endpoint: EndpointId) -> bool {
    match src {
        VertexSource::Edge { .. } => false,
        VertexSource::Endpoint { id } => *id == endpoint,
    }
}

#[cfg(test)]
fn on_edge(src: &VertexSource, from_id: EndpointId, to_id: EndpointId, d: f32) -> bool {
    match src {
        VertexSource::Edge { t, from, to, .. } => {
            *from == from_id
                && *to == to_id
                && ((d - *t).abs() < 0.00001 || (1.0 - d - *t).abs() <= 0.00001)
        },
        VertexSource::Endpoint { .. } => false,
    }
}

#[test]
fn vertex_source_01() {
    use crate::path::generic::PathCommandsBuilder;

    let endpoints: Vec<Point> = vec![
        point(0.0, 0.0),
        point(1.0, 1.0),
        point(0.0, 2.0),
    ];

    let mut cmds = PathCommandsBuilder::new();
    cmds.move_to(EndpointId(0));
    cmds.line_to(EndpointId(1));
    cmds.line_to(EndpointId(2));
    cmds.close();

    let cmds = cmds.build();

    let mut queue = EventQueue::from_path_with_ids(
        0.1,
        cmds.id_events(),
        &(&endpoints[..], &endpoints[..]),
    );

    let mut tess = FillTessellator::new();
    tess.tessellate_events(
        &mut queue,
        &FillOptions::default(),
        &mut CheckVertexSources { next_vertex: 0 },
    ).unwrap();

    struct CheckVertexSources {
        next_vertex: u32,
    }

    impl GeometryBuilder for CheckVertexSources {
        fn begin_geometry(&mut self) {}
        fn end_geometry(&mut self) -> Count { Count { vertices: self.next_vertex, indices: 0 } }
        fn abort_geometry(&mut self) {}
        fn add_triangle(&mut self, _: VertexId, _: VertexId, _: VertexId) {}
    }

    impl FillGeometryBuilder for CheckVertexSources {
        fn add_fill_vertex(&mut self, v: Point, src: &mut dyn Iterator<Item=VertexSource>) -> Result<VertexId, GeometryBuilderError> {
            for src in src {
                if eq(v, point(0.0, 0.0)) { assert!(at_endpoint(&src, EndpointId(0))) }
                else if eq(v, point(1.0, 1.0)) { assert!(at_endpoint(&src, EndpointId(1))) }
                else if eq(v, point(0.0, 2.0)) { assert!(at_endpoint(&src, EndpointId(2))) }
                else { panic!() }
            }

            let id = self.next_vertex;
            self.next_vertex += 1;

            Ok(VertexId(id))
        }
    }
}

#[test]
fn vertex_source_02() {
    // Check the vertex sources of a simple self-intersecting shape.
    //    _
    //  _|_|_
    // | | | |
    // |_|_|_|
    //   |_|
    //

    use crate::path::generic::PathCommandsBuilder;

    let endpoints: Vec<Point> = vec![
        point(1.0, 0.0),
        point(2.0, 0.0),
        point(2.0, 4.0),
        point(1.0, 4.0),
        point(0.0, 1.0),
        point(0.0, 3.0),
        point(3.0, 3.0),
        point(3.0, 1.0),
    ];

    let mut cmds = PathCommandsBuilder::new();
    cmds.move_to(EndpointId(0));
    cmds.line_to(EndpointId(1));
    cmds.line_to(EndpointId(2));
    cmds.line_to(EndpointId(3));
    cmds.close();
    cmds.move_to(EndpointId(4));
    cmds.line_to(EndpointId(5));
    cmds.line_to(EndpointId(6));
    cmds.line_to(EndpointId(7));
    cmds.close();

    let cmds = cmds.build();

    let mut queue = EventQueue::from_path_with_ids(
        0.1,
        cmds.id_events(),
        &(&endpoints[..], &endpoints[..]),
    );

    let mut tess = FillTessellator::new();
    tess.tessellate_events(
        &mut queue,
        &FillOptions::default(),
        &mut CheckVertexSources { next_vertex: 0 },
    ).unwrap();

    struct CheckVertexSources {
        next_vertex: u32,
    }

    impl GeometryBuilder for CheckVertexSources {
        fn begin_geometry(&mut self) {}
        fn end_geometry(&mut self) -> Count { Count { vertices: self.next_vertex, indices: 0 } }
        fn abort_geometry(&mut self) {}
        fn add_triangle(&mut self, _: VertexId, _: VertexId, _: VertexId) {}
    }

    impl FillGeometryBuilder for CheckVertexSources {
        fn add_fill_vertex(&mut self, v: Point, src: &mut dyn Iterator<Item=VertexSource>) -> Result<VertexId, GeometryBuilderError> {
            for src in src {
                if eq(v, point(1.0, 0.0)) { assert!(at_endpoint(&src, EndpointId(0))); }
                else if eq(v, point(2.0, 0.0)) { assert!(at_endpoint(&src, EndpointId(1))); }
                else if eq(v, point(2.0, 4.0)) { assert!(at_endpoint(&src, EndpointId(2))); }
                else if eq(v, point(1.0, 4.0)) { assert!(at_endpoint(&src, EndpointId(3))); }
                else if eq(v, point(0.0, 1.0)) { assert!(at_endpoint(&src, EndpointId(4))); }
                else if eq(v, point(0.0, 3.0)) { assert!(at_endpoint(&src, EndpointId(5))); }
                else if eq(v, point(3.0, 3.0)) { assert!(at_endpoint(&src, EndpointId(6))); }
                else if eq(v, point(3.0, 1.0)) { assert!(at_endpoint(&src, EndpointId(7))); }
                else if eq(v, point(1.0, 1.0)) { assert!(on_edge(&src, EndpointId(7), EndpointId(4), 2.0/3.0) || on_edge(&src, EndpointId(3), EndpointId(0), 3.0/4.0)); }
                else if eq(v, point(2.0, 1.0)) { assert!(on_edge(&src, EndpointId(7), EndpointId(4), 1.0/3.0) || on_edge(&src, EndpointId(1), EndpointId(2), 1.0/4.0)); }
                else if eq(v, point(1.0, 3.0)) { assert!(on_edge(&src, EndpointId(5), EndpointId(6), 1.0/3.0) || on_edge(&src, EndpointId(3), EndpointId(0), 1.0/4.0)); }
                else if eq(v, point(2.0, 3.0)) { assert!(on_edge(&src, EndpointId(5), EndpointId(6), 2.0/3.0) || on_edge(&src, EndpointId(1), EndpointId(2), 3.0/4.0)); }
                else { panic!() }
            }

            let id = self.next_vertex;
            self.next_vertex += 1;

            Ok(VertexId(id))
        }
    }
}
