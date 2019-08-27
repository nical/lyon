// TODO[optim]
//
// # Segment intersection
//
// segment-segment intersection is currently the most perf-sensitive function by far.
// A quick experiment replacing segment_intersection by a dummy function that always
// return None made the tessellation of the logo twice faster.
// segment_intersection can be improved (it is currently a naive implementation).
// It would be interesting to have a fast-path for non-intersecting polygons, though.
// Other tessellators have similar optimizations (like FastUIDraw).
//
// # Allocations
//
// We spend some non-trivial amount of time allocating memory. The main source of allocations
// seems to be that we don't cache allocations for MonotoneTessellators, so we allocate
// vectors every time we start a new span.
//
// # Creating the FillEvents
//
// It's super slow right now.
//

use std::mem::{replace, swap};
use std::cmp::{PartialOrd, Ordering};

use sid::{Id, IdVec};

use crate::FillVertex as Vertex;
use crate::{FillOptions, FillRule, Side, OnError, TessellationError, TessellationResult, InternalError};
use crate::geom::math::*;
use crate::geom::{QuadraticBezierSegment, CubicBezierSegment};
use crate::geom::euclid::{self, Trig};
use crate::math_utils::*;
use crate::geometry_builder::{GeometryBuilder, GeometryBuilderError, Count, VertexId};
use crate::path::PathEvent;
use crate::path::builder::{Build, FlatPathBuilder};

#[cfg(feature="debugger")]
use crate::debugger::*;

#[cfg(test)]
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

#[cfg(not(test))]
macro_rules! tess_log {
    ($obj:ident, $fmt:expr) => ();
    ($obj:ident, $fmt:expr, $($arg:tt)*) => ();
}

struct ActiveEdgeTag;
struct PendingEdgeTag;
struct SpanTag;
type ActiveEdgeId = Id<ActiveEdgeTag, usize>;
type SpanId = Id<SpanTag, usize>;
type PendingEdgeId = Id<PendingEdgeTag, usize>;
type ActiveEdges = IdVec<ActiveEdgeId, ActiveEdge>;
type EdgesBelow = IdVec<PendingEdgeId, PendingEdge>;

#[cfg(feature="debugger")]
pub mod dbg {
    pub const MONOTONE_SPLIT: u32 = 1;
    pub const INTERSECTION_POINT: u32 = 2;
}

impl From<GeometryBuilderError> for TessellationError {
    fn from(e: GeometryBuilderError) -> Self {
        match e {
            GeometryBuilderError::InvalidVertex => TessellationError::InvalidVertex,
            GeometryBuilderError::TooManyVertices => TessellationError::TooManyVertices,
        }
    }
}
impl From<InternalError> for TessellationError {
    fn from(e: InternalError) -> Self {
        TessellationError::Internal(e)
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct Edge {
    pub(crate) upper: TessPoint,
    pub(crate) lower: TessPoint,
}

impl Edge {
    pub fn new(mut a: TessPoint, mut b: TessPoint) -> Self {
        if is_after(a, b) {
            swap(&mut a, &mut b);
        }
        Edge { upper: a, lower: b }
    }
}

#[derive(Copy, Clone, Debug)]
struct OrientedEdge {
    upper: TessPoint,
    lower: TessPoint,
    winding: i16,
}

impl OrientedEdge {
    fn new(mut a: TessPoint, mut b: TessPoint) -> Self {
        let mut winding = 1;
        if is_after(a, b) {
            swap(&mut a, &mut b);
            winding = -1;
        }
        OrientedEdge { upper: a, lower: b, winding }
    }

    fn with_winding(mut a: TessPoint, mut b: TessPoint, winding: i16) -> Self {
        debug_assert!(winding != 0);
        if is_after(a, b) {
            swap(&mut a, &mut b);
        }
        OrientedEdge { upper: a, lower: b, winding }
    }

    fn to_active_edge(&self, upper_id: VertexId) -> ActiveEdge {
        ActiveEdge {
            points: Edge {
                upper: self.upper,
                lower: self.lower,
            },
            upper_id,
            winding: self.winding,
            merge: false,
        }
    }

    fn edge(&self) -> Edge {
        Edge { upper: self.upper, lower: self.lower }
    }
}

#[derive(Clone, Debug)]
struct PendingEdge {
    // The upper vertex is the current vertex, we don't need to store it.
    lower: TessPoint,
    angle: f32,
    winding: i16,
}

impl PendingEdge {
    fn to_oriented_edge(&self, upper: TessPoint) -> OrientedEdge {
        OrientedEdge {
            upper,
            lower: self.lower,
            winding: self.winding,
        }
    }

    fn to_active_edge(&self, upper: TessPoint, upper_id: VertexId) -> ActiveEdge {
        ActiveEdge {
            points: Edge {
                upper,
                lower: self.lower,
            },
            upper_id,
            winding: self.winding,
            merge: false,
        }
    }

}

#[derive(Copy, Clone, Debug, PartialEq)]
enum PointType { In, Out, OnEdge(Side) }

/// A Context object that can tessellate fill operations for complex paths.
///
/// <svg version="1.1" viewBox="0 0 400 200" height="200" width="400">
///   <g transform="translate(0,-852.36216)">
///     <path style="fill:#aad400;stroke:none;" transform="translate(0,852.36216)" d="M 20 20 L 20 180 L 180.30273 180 L 180.30273 20 L 20 20 z M 100 55 L 145 145 L 55 145 L 100 55 z "/>
///     <path style="fill:#aad400;fill-rule:evenodd;stroke:#000000;stroke-width:1px;stroke-linecap:butt;stroke-linejoin:miter;stroke-" d="m 219.75767,872.36216 0,160.00004 160.30273,0 0,-160.00004 -160.30273,0 z m 80,35 45,90 -90,0 45,-90 z"/>
///     <path style="fill:none;stroke:#000000;stroke-linecap:round;stroke-linejoin:round;stroke-" d="m 220,1032.3622 35,-35.00004 125,35.00004 -35,-35.00004 35,-125 -80,35 -80,-35 35,125"/>
///     <circle r="5" cy="872.36218" cx="20" style="color:#000000;;fill:#ff6600;fill-;stroke:#000000;" />
///     <circle r="5" cx="180.10918" cy="872.61475" style="fill:#ff6600;stroke:#000000;"/>
///     <circle r="5" cy="1032.2189" cx="180.10918" style="fill:#ff6600;stroke:#000000;"/>
///     <circle r="5" cx="20.505075" cy="1032.4714" style="fill:#ff6600;stroke:#000000;"/>
///     <circle r="5" cy="907.21252" cx="99.802048" style="fill:#ff6600;stroke:#000000;"/>
///     <circle r="5" cx="55.102798" cy="997.36865" style="fill:#ff6600;stroke:#000000;"/>
///     <circle r="5" cy="997.62122" cx="145.25891" style="fill:#ff6600;stroke:#000000;"/>
///   </g>
/// </svg>
///
/// ## Overview
///
/// The most important structure is [`FillTessellator`](struct.FillTessellator.html).
/// It implements the path fill tessellation algorithm which is by far the most advanced
/// feature in all lyon crates.
///
/// The `FillTessellator` takes a [`FillEvents`](struct.FillEvents.html) object and
/// [`FillOptions`](struct.FillOptions.html) as input. The former is an intermediate representaion
/// of the path, containing all edges sorted from top to bottom. `FillOption` contains
/// some extra parameters (Some of which are not implemented yet).
///
/// The output of the tessellator is produced by the
/// [`GeometryBuilder`](geometry_builder/trait.GeometryBuilder.html) (see the
/// [`geometry_builder` documentation](geometry_builder/index.html) for more details about
/// how tessellators produce their output geometry, and how to generate custom vertex layouts).
///
/// The [tessellator's wiki page](https://github.com/nical/lyon/wiki/Tessellator) is a good place
/// to learn more about how the tessellator's algorithm works. The source code also contains
/// inline documentation for the adventurous who want to delve into more details.
///
/// # Examples
///
/// ```
/// # extern crate lyon_tessellation as tess;
/// # use tess::path::Path;
/// # use tess::path::builder::*;
/// # use tess::path::iterator::*;
/// # use tess::geom::math::{Point, point};
/// # use tess::geometry_builder::{VertexBuffers, simple_builder};
/// # use tess::*;
/// # fn main() {
/// // Create a simple path.
/// let mut path_builder = Path::builder();
/// path_builder.move_to(point(0.0, 0.0));
/// path_builder.line_to(point(1.0, 2.0));
/// path_builder.line_to(point(2.0, 0.0));
/// path_builder.line_to(point(1.0, 1.0));
/// path_builder.close();
/// let path = path_builder.build();
///
/// // Create the destination vertex and index buffers.
/// let mut buffers: VertexBuffers<FillVertex, u16> = VertexBuffers::new();
///
/// {
///     let mut vertex_builder = simple_builder(&mut buffers);
///
///     // Create the tessellator.
///     let mut tessellator = FillTessellator::new();
///
///     // Compute the tessellation.
///     let result = tessellator.tessellate_path(
///         path.iter(),
///         &FillOptions::default(),
///         &mut vertex_builder
///     );
///     assert!(result.is_ok());
/// }
///
/// println!("The generated vertices are: {:?}.", &buffers.vertices[..]);
/// println!("The generated indices are: {:?}.", &buffers.indices[..]);
///
/// # }
/// ```
///
/// # Limitations
///
/// The fill tessellator internally works with 16.16 fixed point numbers. This
/// means that it is unable to represent numbers with absolute values larger
/// than 32767.0 without running into overflows and undefined behavior.
///
/// # How the fill tessellator works
///
/// Learn more about how the algorithm works on the [tessellator wiki page](https://github.com/nical/lyon/wiki/Tessellator).
///
pub struct FillTessellator {
    // The edges that intersect with the sweep line.
    active_edges: ActiveEdges,
    // The edges that we are about to become active edges
    // (directly below the current point).
    pending_edges: Vec<PendingEdge>,
    // The current position of the sweep line.
    current_position: TessPoint,

    // various options
    options: FillOptions,
    log: bool,

    // Events and intersections that haven't been processed yet.
    events: FillEvents,
    intersections: Vec<OrientedEdge>,

    monotone_tessellators: IdVec<SpanId, MonotoneTessellator>,
    tess_pool: Vec<MonotoneTessellator>,

    error: Option<TessellationError>,

    #[cfg(feature="debugger")]
    debugger: Option<Box<dyn Debugger2D>>,
}

impl FillTessellator {
    /// Constructor.
    pub fn new() -> Self {
        FillTessellator {
            events: FillEvents::new(),
            active_edges: ActiveEdges::with_capacity(16),
            pending_edges: Vec::with_capacity(8),
            monotone_tessellators: IdVec::with_capacity(16),
            intersections: Vec::with_capacity(8),
            current_position: TessPoint::new(FixedPoint32::min_val(), FixedPoint32::min_val()),
            error: None,
            options: FillOptions::DEFAULT,
            log: false,
            tess_pool: Vec::with_capacity(8),

            #[cfg(feature="debugger")]
            debugger: None,
        }
    }

    /// Compute the tessellation from a path iterator.
    pub fn tessellate_path<Iter>(
        &mut self,
        it: Iter,
        options: &FillOptions,
        output: &mut dyn GeometryBuilder<Vertex>,
    ) -> TessellationResult
    where
        Iter: IntoIterator<Item = PathEvent<Point, Point>>,
    {
        let mut events = replace(&mut self.events, FillEvents::new());
        events.clear();
        events.set_path(options.tolerance, it.into_iter());
        let result = self.tessellate_events(&events, options, output);
        self.events = events;

        result
    }

    /// Compute the tessellation from pre-sorted events.
    pub fn tessellate_events(
        &mut self,
        events: &FillEvents,
        options: &FillOptions,
        output: &mut dyn GeometryBuilder<Vertex>,
    ) -> TessellationResult {
        if options.fill_rule != FillRule::EvenOdd {
            println!("warning: Fill rule {:?} is not supported yet.", options.fill_rule);
            match options.on_error {
                OnError::Stop => { return Err(TessellationError::UnsupportedParamater); }
                OnError::Panic => { panic!("Unsupported fill rule"); }
                OnError::Recover => {}
            }
        }

        self.options = *options;

        self.begin_tessellation(output);

        self.tessellator_loop(events, output);

        let mut error = None;
        swap(&mut error, &mut self.error);
        if let Some(err) = error {
            output.abort_geometry();
            self.reset();
            return Err(err);
        }

        let res = self.end_tessellation(output);
        self.reset();

        Ok(res)
    }

    /// Enable some verbose logging during the tessellation, for debugging purposes.
    pub fn enable_logging(&mut self) { self.log = true; }

    #[cfg(feature="debugger")]
    pub fn install_debugger(&mut self, dbg: Box<dyn Debugger2D>) {
        self.debugger = Some(dbg)
    }

    fn panic_on_errors(&self) -> bool {
        self.options.on_error == OnError::Panic
    }

    fn reset(&mut self) {
        self.active_edges.clear();
        self.monotone_tessellators.clear();
        self.pending_edges.clear();
    }

    fn begin_tessellation(&mut self, output: &mut dyn GeometryBuilder<Vertex>) {
        debug_assert!(self.active_edges.is_empty());
        debug_assert!(self.monotone_tessellators.is_empty());
        debug_assert!(self.pending_edges.is_empty());
        output.begin_geometry();
    }

    fn end_tessellation(
        &mut self,
        output: &mut dyn GeometryBuilder<Vertex>,
    ) -> Count {
        if self.panic_on_errors() {
            debug_assert!(self.active_edges.is_empty());
            debug_assert!(self.monotone_tessellators.is_empty());
            debug_assert!(self.pending_edges.is_empty());
        }
        self.reset();
        output.end_geometry()
    }

    fn tessellator_loop(
        &mut self,
        events: &FillEvents,
        output: &mut dyn GeometryBuilder<Vertex>,
    ) {
        self.current_position = TessPoint::new(FixedPoint32::min_val(), FixedPoint32::min_val());

        let mut edge_iter = events.edges.iter();
        let mut vertex_iter = events.vertices.iter();
        let mut next_edge = edge_iter.next();
        let mut next_vertex = vertex_iter.next();
        loop {
            if self.error.is_some() && self.options.on_error != OnError::Recover {
                return;
            }

            let mut next_position = None;
            let mut pending_events = false;

            // We look for the next event by pulling from three sources: the list edges,
            // the list of vertices that don't have a edges immediately under them (end
            // or merge events), and the list of intersections that we find along the way.

            // Look at the sorted list of edges.
            while let Some(edge) = next_edge {
                if edge.upper == self.current_position {
                    next_edge = edge_iter.next();
                    if edge.lower == self.current_position {
                        continue;
                    }

                    let angle = edge_angle(edge.lower - edge.upper);
                    self.pending_edges.push(PendingEdge {
                        lower: edge.lower,
                        angle,
                        winding: edge.winding,
                    });
                    tess_log!(self, " edge at {:?} -> {:?} (angle={:?})", edge.upper, edge.lower, angle);

                    pending_events = true;
                    continue;
                }

                next_position = Some(edge.upper);
                break;
            }

            // Look at the sorted list of vertices.
            while let Some(vertex) = next_vertex {
                if *vertex == self.current_position {
                    next_vertex = vertex_iter.next();
                    pending_events = true;
                    tess_log!(self, " vertex at {:?}", self.current_position);
                    continue;
                }
                if next_position.is_none() || is_after(next_position.unwrap(), *vertex) {
                    next_position = Some(*vertex);
                }
                break;
            }

            // Look at the sorted list of intersections.
            while !self.intersections.is_empty() {
                let intersection_position = self.intersections[0].upper;
                if intersection_position == self.current_position {
                    let inter = self.intersections.remove(0);

                    if inter.lower != self.current_position {
                        self.pending_edges.push(
                            PendingEdge {
                                lower: inter.lower,
                                angle: edge_angle(inter.lower - self.current_position),
                                winding: inter.winding,
                            }
                        );
                    }

                    pending_events = true;
                    continue;
                }
                if next_position.is_none() || is_after(next_position.unwrap(), intersection_position) {
                    next_position = Some(intersection_position);
                }
                break;
            }

            if pending_events {
                let num_intersections = self.intersections.len();
                if let Err(e) = self.process_vertex(output) {
                    self.builder_error(e);
                }

                if num_intersections != self.intersections.len() {

                    self.update_intersections();

                    // The next position may have changed so we go back to the beginning
                    // of the loop to determine the next position.
                    // We could only look at the list of intersections since it is the
                    // only one that changed but it probably does not make a difference
                    // speed-wise and the code is simpler this way.
                    continue;
                }
            }


            if let Some(position) = next_position {
                self.current_position = position;
                tess_log!(self, "\n\n -- current_position is now {:?}", position);
            } else {
                return;
            }
        }
    }

    #[inline(never)]
    fn update_intersections(&mut self) {
        // We found an intersection during process_vertex, it has been added
        // to self.intersections.

        // Sort the intersection list.
        self.intersections.sort_by(|a, b| compare_positions(a.upper, b.upper));

        // Because intersections aren't precise, splitting active edges may break
        // their ordering.

        // Go through the active edges and reorder them if need be.
        // This is a poor man's sorting implementation that works best when the
        // set is already sorted which is the vast majority of cases for us.
        let stop = ActiveEdgeId::new(self.active_edges.len());
        let mut id = ActiveEdgeId::new(0);
        let mut prev_x = None;
        while id.handle < stop.handle {
            let edge = self.active_edges[id].points;
            let v = edge.lower - edge.upper;

            // TODO: This doesn't deal well with horizontal edges, skip them for now.
            if v.y.raw() == 0 {
                prev_x = None;
                id = id + 1;
                continue;
            }

            // Intersect the edge with the horizontal line passing at the current position.
            let dy = self.current_position.y - edge.upper.y;
            // TODO: this division is super high in the profiles when there are many intersections.
            let x = edge.upper.x + dy.mul_div(v.x, v.y);

            if let Some(px) = prev_x {
                if px > x {
                    // Swap.
                    let tmp = self.active_edges[id - 1];
                    self.active_edges[id - 1] = self.active_edges[id];
                    self.active_edges[id] = tmp;
                    // The edge we moved to the left might not be ordered with edges
                    // even more to the left, so backtrack the scan.
                    if id.handle > 1 {
                        id = id - 2
                    }
                    prev_x = None;

                    continue;
                }
            }

            id = id + 1;
            prev_x = Some(x);
        }
    }

    fn add_vertex_with_normal(
        &mut self,
        prev: &TessPoint,
        next: &TessPoint,
        output: &mut dyn GeometryBuilder<Vertex>
    ) -> Result<VertexId, GeometryBuilderError> {
        let position = to_f32_point(self.current_position);
        let prev = to_f32_point(*prev);
        let next = to_f32_point(*next);
        let normal = compute_normal(
            (position - prev).normalize(),
            (next - position).normalize(),
        );

        output.add_vertex(Vertex { position, normal })
    }

    fn process_vertex(
        &mut self,
        output: &mut dyn GeometryBuilder<Vertex>,
    ) -> Result<(), GeometryBuilderError> {
        // This is where the interesting things happen.
        // We go through the sweep line to find all of the edges that end at the current
        // position, and through the list of edges that start at the current position
        // (self.pending_edges, which we build in tessellator_loop).
        // we decide what to do depending on the spacial configuration of these edges.
        //
        // The logic here really need to be simplified, it is the trickiest part of the
        // tessellator.

        let (
            // Whether the point is inside, outside or on an edge.
            point_type,
            // First active edge that ends at the current point
            first_edge_above,
            // Number of active edges that end at the current point.
            mut num_edges_above,
            // The number used by the fill rule to determine what is in or out.
            _winding_number,
        ) = self.find_interesting_active_edges();

        // We'll bump above_idx as we process active edges that interact with
        // the current point.
        let mut above_idx = first_edge_above;
        // The index of the next pending edge to be processed.
        let mut pending_edge_id = 0;

        // Go through all pending edges, sort them and handle pairs of overlapping edges.
        // Doing this here avoids some potentially tricky cases with intersections
        // later.
        prepare_pending_edges(&mut self.pending_edges, &mut self.intersections);

        self.log_sl(first_edge_above);
        tess_log!(self, "{:?}", point_type);
        tess_log!(self, "above:{}", num_edges_above);

        let mut vertex_id = if !self.options.compute_normals {
            let vector_position = to_f32_point(self.current_position);
            output.add_vertex(
                Vertex {
                    position: vector_position,
                    normal: vector(0.0, 0.0),
                }
            )?
        } else {
            // placeholder
            VertexId(0)
        };

        // The number of pending edges that haven't been processed.
        let mut num_pending_edges = self.pending_edges.len();
        let mut pending_merge = false;

        // Step 1, walk the sweep line, handle left/right events, handle the spans that end
        // at this vertex, as well as merge events.
        if self.active_edges.has_id(first_edge_above) && point_type == PointType::OnEdge(Side::Right) {
            debug_assert!(odd(first_edge_above));
            if num_pending_edges == 0 {
                // we are on the right side of a span but there is nothing below, it means
                // that we are at a merge event.
                //
                //  .\  |../  /
                //  ..\ |./ /..
                //  -->\|//....
                //  ....x......
                //
                // we'll merge with the right most edge, there may be end events
                // in the middle so we handle the merge event later. Since end
                // events remove their spans, we don't need to remember the current
                // span index to process the merge.
                debug_assert!(num_edges_above >= 2);
                pending_merge = true;
                num_edges_above -= 2;
            } else {
                // Right event.
                //
                //   ..../
                //   ...x
                //   ....\
                //
                tess_log!(self, "(right event) {:?}", above_idx);
                debug_assert!(num_edges_above > 0);
                debug_assert!(num_pending_edges > 0);

                if self.options.compute_normals {
                    let vertex_above = self.active_edges[above_idx].points.upper;
                    let edge_to = self.pending_edges[0].lower;
                    vertex_id = self.add_vertex_with_normal(&edge_to, &vertex_above, output)?;
                }
                self.insert_edge(above_idx, 0, vertex_id);

                // Update the initial state for the pass that will handle
                // the edges below the current vertex.
                pending_edge_id += 1;
                num_pending_edges -= 1;
                num_edges_above -= 1;
            }

            above_idx = above_idx + 1;
        }

        while num_edges_above >= 2 {
            // End event.
            //
            // Pairs of edges that end at the current position form "end events".
            // By construction we know that they are on the same spans.
            // There can be several pairs of such edges.
            //
            //  \...../   \...|  /../
            //   \.../     \..| /./
            //    \./   or   \|//   etc.
            //     x          x
            //
            tess_log!(self, "(end event) {:?}", above_idx);

            if self.options.compute_normals {
                let left = self.active_edges[above_idx].points.upper;
                let right = self.active_edges[above_idx+1].points.upper;
                vertex_id = self.add_vertex_with_normal(&left, &right, output)?;
            }

            self.resolve_merge_vertices(above_idx, vertex_id, output);
            self.end_span(above_idx, vertex_id, output);

            num_edges_above -= 2;
        }

        if pending_merge {
            // Merge event.
            //
            // ...\   /...
            // ....\ /....
            // .....x.....
            //
            tess_log!(self, "(merge event) {:?}", above_idx);
            debug_assert_eq!(num_edges_above, 0);

            if self.options.compute_normals {
                let left = self.active_edges[first_edge_above].points.upper;
                let right = self.active_edges[first_edge_above+1].points.upper;
                vertex_id = self.add_vertex_with_normal(&right, &left, output)?;
            }

            self.merge_event(vertex_id, first_edge_above, output);

        } else if num_edges_above == 1 && num_pending_edges > 0 {
            // Left event.
            //
            //     /...
            //    x....
            //     \...
            //

            let vertex_below_id = self.pending_edges.len() - 1;
            tess_log!(self, "(left event) {:?}    -> {:?}", above_idx, self.pending_edges[vertex_below_id].lower);

            if self.options.compute_normals {
                let vertex_above = self.active_edges[above_idx].points.upper;
                let vertex_below = self.pending_edges[vertex_below_id].lower;
                vertex_id = self.add_vertex_with_normal(&vertex_above, &vertex_below, output)?;
            }
            self.resolve_merge_vertices(above_idx, vertex_id, output);
            self.insert_edge(above_idx, vertex_below_id, vertex_id);

            num_pending_edges -= 1;
            num_edges_above -= 1;
        }
        // Since we took care of left and right events already we should not have
        // an odd number of pending edges to work with by now.
        if num_pending_edges % 2 != 0 {
            if self.error(InternalError::E01) {
                return Ok(());
            }
            // TODO - We are in an invalid state, and trying to continue tessellating
            // anyway. The code below assumes we have an even number
            // of pending edges so let's pretend.
            num_pending_edges -= 1;
        }

        // Step 2, handle edges below the current vertex.
        if num_pending_edges > 0 {
            if point_type == PointType::In {
                debug_assert!(num_pending_edges >= 2);
                debug_assert!(even(above_idx));
                // Split event.
                //
                // .....x.....
                // ..../ \....
                // .../   \...
                //
                tess_log!(self, "(split event) {:?}", above_idx);

                let left_idx = 0;
                let right_idx = num_pending_edges - 1;
                if self.options.compute_normals {
                    let left_vertex = self.pending_edges[left_idx].lower;
                    let right_vertex = self.pending_edges[right_idx].lower;
                    vertex_id = self.add_vertex_with_normal(&left_vertex, &right_vertex, output)?;
                }

                self.split_event(above_idx, left_idx, right_idx, vertex_id, output);

                num_pending_edges -= 2;
                // split_event inserts two active edges at the current offset
                // and we need skip them.
                above_idx = above_idx + 2;
            }

            while num_pending_edges >= 2 {

                // Start event.
                //
                //      x
                //     /.\
                //    /...\
                //
                tess_log!(self, "(start event) {:?}", above_idx);

                if self.options.compute_normals {
                    let left = self.pending_edges[pending_edge_id].lower;
                    let right = self.pending_edges[pending_edge_id + 1].lower;
                    vertex_id = self.add_vertex_with_normal(&right, &left, output)?;
                }

                self.start_event(above_idx, vertex_id, pending_edge_id);

                pending_edge_id += 2;
                num_pending_edges -= 2;
                above_idx = above_idx + 2;
            }
        }

        self.debug_check_sl();

        self.pending_edges.clear();

        if num_edges_above != 0 || num_pending_edges != 0 {
            self.error(InternalError::E02);
        }

        Ok(())
    }

    fn find_interesting_active_edges(
        &mut self,
    ) -> (PointType, ActiveEdgeId, usize, i16) {
        let mut winding_number = 0;
        let mut point_type = None;
        let mut num_edges_above = 0;
        let mut first_edge_above = ActiveEdgeId::new(self.active_edges.len());

        // Go through the sweep line to find the first edge that ends at the current
        // position (if any) or if the current position is inside or outside the shape.
        // After finding the first edge that touches this point, keep iterating to find
        // other edges potentially touching this point.
        for (i, active_edge) in self.active_edges.iter_mut().enumerate() {
            if active_edge.merge {
                continue;
            }

            winding_number += active_edge.winding;

            let edge_idx = ActiveEdgeId::new(i);
            let side = if even(edge_idx) { Side::Left } else { Side::Right };

            let at_endpoint = active_edge.points.lower == self.current_position;
            let mut on_edge = false;
            let mut edge_after_point = false;

            if !at_endpoint {
                compare_edge_against_position(
                    &active_edge.points,
                    self.current_position,
                    &mut on_edge,
                    &mut edge_after_point,
                );
            }

            //tess_log!(self,
            //    "## point:{} edge:{} past:{}",
            //    at_endpoint, on_edge, edge_after_point
            //);
            if at_endpoint || on_edge {
                // If at_endpoint or on_edge is true then edge_after_point
                // should be false, otherwise we may break out of this loop
                // too early.
                debug_assert!(!edge_after_point);
                num_edges_above += 1;
                if point_type.is_none() {
                    point_type = Some(PointType::OnEdge(side));
                    first_edge_above = edge_idx;
                }
            }

            if on_edge {
                // The current point is on an edge we need to split the edge into the part
                // above and the part below. See test_point_on_edge_left for an example of
                // geometry that can lead to this scenario.

                // Split the edge.
                self.pending_edges.push(PendingEdge {
                    lower: active_edge.points.lower,
                    angle: edge_angle(active_edge.points.lower - self.current_position),
                    winding: active_edge.winding,
                });
                active_edge.points.lower = self.current_position;
            }

            if edge_after_point {
                if point_type.is_none() {
                    if side.is_left() {
                        //       |....
                        //    x  |....
                        //       |....
                        point_type = Some(PointType::Out);
                        first_edge_above = edge_idx;
                    } else {
                        //  .....|
                        //  ..x..|
                        //  .....|
                        point_type = Some(PointType::In);
                        // Use the edge on the left side of the span we are in.
                        first_edge_above = edge_idx - 1;
                    }
                }

                // No need to iterate further. Since the sweep line is sorted,
                // we know that there won't be any more edges touching this point.
                break;
            }
        }

        (
            point_type.unwrap_or(PointType::Out),
            first_edge_above,
            num_edges_above,
            winding_number
        )
    }

    // Look for eventual merge vertices on this span above the current vertex, and connect
    // them to the current vertex.
    // This should be called when processing a vertex that is on the left side of a span.
    fn resolve_merge_vertices(
        &mut self,
        edge_idx: ActiveEdgeId,
        id: VertexId,
        output: &mut dyn GeometryBuilder<Vertex>,
    ) {
        // we are expecting this to be called with the index of the beginning (left side)
        // of a span, so the index should be even. This won't necessary hold true when we
        // add support for other fill rules.
        debug_assert!(even(edge_idx));

        while self.active_edges[edge_idx + 1].merge {
            #[cfg(feature="debugger")]
            self.debugger_monotone_split(&self.active_edges[edge_idx + 2].points.upper, &self.current_position);

            //     \ /
            //  \   x   <-- merge vertex
            //   \ :
            //    x   <-- current vertex
            self.active_edges[edge_idx + 2].set_lower_vertex(self.current_position);
            self.end_span(edge_idx, id, output);
        }
    }

    fn start_event(
        &mut self,
        edge_idx: ActiveEdgeId,
        vertex_id: VertexId,
        pending_edge_id: usize,
    ) {
        //      x  <-- current position
        //     /.\
        //    /...\
        //
        self.handle_intersections(pending_edge_id);
        self.handle_intersections(pending_edge_id + 1);

        self.active_edges.insert_slice(edge_idx, &[
            self.pending_edges[pending_edge_id].to_active_edge(self.current_position, vertex_id),
            self.pending_edges[pending_edge_id + 1].to_active_edge(self.current_position, vertex_id),
        ]);

        let pos = self.current_position;
        self.insert_span(span_for_edge(edge_idx), pos, vertex_id);
    }

    fn split_event(
        &mut self,
        edge_idx: ActiveEdgeId,
        pending_left_id: usize,
        pending_right_id: usize,
        id: VertexId,
        output: &mut dyn GeometryBuilder<Vertex>,
    ) {
        debug_assert!(even(edge_idx));
        // Look whether the span shares a merge vertex with the previous one
        if self.active_edges[edge_idx].merge {
            let left_span_edge = edge_idx - 1;
            let right_span_edge = edge_idx;
            debug_assert!(self.active_edges[left_span_edge].merge);

            #[cfg(feature="debugger")]
            self.debugger_monotone_split(&self.active_edges[edge_idx].points.lower, &self.current_position);

            //            \ /
            //             x   <-- merge vertex
            //  left_span  :  righ_span
            //             x   <-- current split vertex
            //           l/ \r
            self.insert_edge(left_span_edge, pending_left_id, id);
            self.insert_edge(right_span_edge, pending_right_id, id);

            // There may be more merge vertices chained on the right of the current span, now
            // we are in the same configuration as a left event.
            self.resolve_merge_vertices(edge_idx, id, output);
        } else if is_after(self.active_edges[edge_idx].points.upper, self.active_edges[edge_idx + 1].points.upper) {
            //      /
            //     x
            //  l2/ :r2
            //   /   x   <-- current split vertex
            //  left/ \right
            self.handle_intersections(pending_left_id);
            self.handle_intersections(pending_right_id);

            let left_idx = edge_idx + 1;
            self.active_edges.insert_slice(left_idx, &[
                self.pending_edges[pending_left_id].to_active_edge(self.current_position, id),
                self.pending_edges[pending_right_id].to_active_edge(self.current_position, id),
            ]);

            let left_span = span_for_edge(left_idx);
            let right_span = left_span + 1;

            let l2_upper = self.active_edges[edge_idx].points.upper;
            let l2_id = self.active_edges[edge_idx].upper_id;

            self.insert_span(left_span, l2_upper, l2_id);

            let vector_position = to_f32_point(self.current_position);
            self.monotone_tessellators[left_span].vertex(vector_position, id, Side::Right);
            self.monotone_tessellators[right_span].vertex(vector_position, id, Side::Left);

            #[cfg(feature="debugger")]
            self.debugger_monotone_split(&l2_upper, &self.current_position);
        } else {
            //                              |
            //                              x
            //                           l2: \r2
            // current split vertex -->   x   \
            //                       left/ \right
            self.handle_intersections(pending_left_id);
            self.handle_intersections(pending_right_id);

            let r2_upper = self.active_edges[edge_idx + 1].points.upper;
            let r2_id = self.active_edges[edge_idx + 1].upper_id;

            let left_idx = edge_idx + 1;
            self.active_edges.insert_slice(left_idx, &[
                self.pending_edges[pending_left_id].to_active_edge(self.current_position, id),
                self.pending_edges[pending_right_id].to_active_edge(self.current_position, id),
            ]);

            let left_span = span_for_edge(left_idx);
            let right_span = left_span + 1;

            self.insert_span(right_span, r2_upper, r2_id);

            let vector_position = to_f32_point(self.current_position);
            self.monotone_tessellators[left_span].vertex(vector_position, id, Side::Right);
            self.monotone_tessellators[right_span].vertex(vector_position, id, Side::Left);

            #[cfg(feature="debugger")]
            self.debugger_monotone_split(&r2_upper, &self.current_position);
        }
    }

    #[cfg(feature="debugger")]
    fn debugger_monotone_split(&self, a: &TessPoint, b: &TessPoint) {
        if let Some(ref dbg) = self.debugger {
            dbg.edge(
                &to_f32_point(*a),
                &to_f32_point(*b),
                DARK_RED,
                dbg::MONOTONE_SPLIT,
            );
        }
    }

    fn merge_event(
        &mut self,
        id: VertexId,
        edge_idx: ActiveEdgeId,
        output: &mut dyn GeometryBuilder<Vertex>,
    ) {
        debug_assert!(odd(edge_idx));
        debug_assert!(self.active_edges.has_id(edge_idx + 2));

        let left_span_edge = edge_idx;
        let right_span_edge = edge_idx + 1;

        //     / \ /
        //  \ / .-x    <-- merge vertex
        //   x-'      <-- current merge vertex
        self.resolve_merge_vertices(right_span_edge, id, output);

        let vector_position = to_f32_point(self.current_position);

        self.active_edges[left_span_edge].merge_vertex(self.current_position, id);
        self.active_edges[right_span_edge].merge_vertex(self.current_position, id);

        self.monotone_tessellators[span_for_edge(left_span_edge)].vertex(vector_position, id, Side::Right);
        self.monotone_tessellators[span_for_edge(right_span_edge)].vertex(vector_position, id, Side::Left);
    }

    fn insert_edge(
        &mut self,
        edge_idx: ActiveEdgeId,
        pending_edge_id: usize,
        id: VertexId,
    ) {
        let upper = self.current_position;
        // TODO horrible hack: set the merge flag on the edge we are about to replace temporarily
        // so that it does not get in the way of the intersection detection.
        self.active_edges[edge_idx].merge = true;

        debug_assert!(!is_after(upper, self.pending_edges[pending_edge_id].lower));

        self.handle_intersections(pending_edge_id);

        // This sets the merge flag to false.
        self.active_edges[edge_idx] = self.pending_edges[pending_edge_id].to_active_edge(upper, id);

        let side = if even(edge_idx) { Side::Left } else { Side::Right };
        let vector_position = to_f32_point(upper);
        self.monotone_tessellators[span_for_edge(edge_idx)].vertex(vector_position, id, side);
    }

    fn handle_intersections(&mut self, new_edge_idx: usize) {
        // Test and for intersections against the edges in the sweep line.
        // If an intersecton is found, the edge is split and retains only the part
        // above the intersection point. The lower part is kept with the intersection
        // to be processed later when the sweep line reaches it.
        // If there are several intersections we only keep the one that is closest to
        // the sweep line.
        //
        // TODO: This function is more complicated (and slower) than it needs to be.

        if self.options.assume_no_intersections {
            return;
        }

        let mut new_edge = self.pending_edges[new_edge_idx].to_oriented_edge(self.current_position);

        let original_edge = new_edge.edge();
        let mut intersection = None;

        for (edge_idx, edge) in self.active_edges.iter_mut().enumerate() {
            // Test for an intersection against the span's left edge.
            if !edge.merge {
                if let Some(position) = segment_intersection(&new_edge.edge(), &edge.points) {
                    tess_log!(self, " -- found an intersection at {:?}
                                    |    {:?}->{:?} x {:?}->{:?}",
                        position,
                        original_edge.upper, original_edge.lower,
                        edge.points.upper, edge.points.lower,
                    );

                    intersection = Some((position, ActiveEdgeId::new(edge_idx)));
                    // From now on only consider potential intersections above the one we found,
                    // by removing the lower part from the segment we test against.
                    new_edge.lower = position;
                }
            }
        }

        if intersection.is_none() {
            return;
        }

        let (mut intersection, edge_idx) = intersection.unwrap();

        // Because precision issues, it can happen that the intersection appear to be
        // "above" the current vertex (in fact it is at the same y but on its left which
        // counts as above). Since we can't come back in time to process the intersection
        // before the current vertex, we can only cheat by moving the intersection down by
        // one unit.
        if !is_after(intersection, self.current_position) {
            intersection.y = self.current_position.y + FixedPoint32::epsilon();
            new_edge.lower = intersection;
        }

        self.pending_edges[new_edge_idx].lower = new_edge.lower;

        let active_edge_lower;
        let active_edge_winding;
        {
            let active_edge = &mut self.active_edges[edge_idx];
            active_edge_lower = active_edge.points.lower;
            active_edge_winding = active_edge.winding;
            active_edge.points.lower = intersection;
        }

        self.intersections.push(OrientedEdge::with_winding(
            intersection,
            original_edge.lower,
            new_edge.winding
        ));
        self.intersections.push(OrientedEdge::with_winding(
            intersection,
            active_edge_lower,
            active_edge_winding
        ));

        #[cfg(feature="debugger")] {
            if let Some(ref mut dbg) = self.debugger {
                dbg.point(&to_f32_point(intersection), RED, dbg::INTERSECTION_POINT);
            }
        }

        // We sill sort the intersection vector lazily.
    }

    fn end_span(
        &mut self,
        edge_idx: ActiveEdgeId,
        id: VertexId,
        output: &mut dyn GeometryBuilder<Vertex>,
    ) {
        // This assertion is only valid with the EvenOdd fill rule.
        debug_assert!(even(edge_idx));
        let span_idx = span_for_edge(edge_idx);

        let vector_position = to_f32_point(self.current_position);
        {
            let tess = &mut self.monotone_tessellators[span_idx];
            tess.end(vector_position, id);
            tess.flush(output);
        }

        self.active_edges.remove(edge_idx + 1);
        self.active_edges.remove(edge_idx);

        let to_recycle = self.monotone_tessellators.remove(span_idx);
        self.tess_pool.push(to_recycle);
    }

    fn insert_span(&mut self, span: SpanId, pos: TessPoint, vertex: VertexId) {
        let tess = self.tess_pool.pop().unwrap_or_else(
            ||{ MonotoneTessellator::new() }
        ).begin(to_f32_point(pos), vertex);

        self.monotone_tessellators.insert(span, tess);
    }

    #[cold]
    #[inline(never)]
    fn error(&mut self, err: InternalError) -> bool {
        tess_log!(self, " !! FillTessellator Error {:?}", err);
        if self.panic_on_errors() {
            panic!();
        }
        if self.error.is_none() {
            self.error = Some(TessellationError::Internal(err));
        }

        self.options.on_error == OnError::Stop
    }

    #[cold]
    #[inline(never)]
    fn builder_error(&mut self, e: GeometryBuilderError) {
        tess_log!(self, " !! GeometryBuilder error");
        if self.panic_on_errors() {
            panic!();
        }
        if self.error.is_none() {
            self.error = Some(match e {
                GeometryBuilderError::TooManyVertices => TessellationError::TooManyVertices,
                GeometryBuilderError::InvalidVertex => TessellationError::InvalidVertex,
            });
        }
    }

    #[cfg(not(debug_assertions))]
    fn debug_check_sl(&mut self) {}

    #[cfg(debug_assertions)]
    fn debug_check_sl(&mut self) {
        let mut ok = true;
        for edge in &self.active_edges {
            if edge.merge {
                continue;
            }
            if is_after(self.current_position, edge.points.lower) {
                tess_log!(self,
                    "current vertex {:?} should not be below lower {:?}",
                    self.current_position,
                    edge.points.lower
                );
                ok = false;
                break;
            }
            if is_after(edge.points.upper, edge.points.lower) {
                tess_log!(self,
                    "upper vertex {:?} should not be below lower vertex {:?}",
                    edge.points.upper,
                    edge.points.lower
                );
                ok = false;
                break;
            }
        }

        if !ok {
            self.error(InternalError::E03);
        }

        self.log_sl_winding();
        // TODO: There are a few failing tests to fix before this assertion
        // can be made.
        //assert_eq!(winding_number, 0);
    }

    #[cfg(not(test))]
    fn log_sl(&self, _: ActiveEdgeId) {}

    #[cfg(test)]
    fn log_sl(&self, first_edge_above: ActiveEdgeId) {
        if !self.log {
            return;
        }
        println!("\n\n ----- current: {:?} ------ offset {:?} in sl", self.current_position, first_edge_above.handle);
        for b in &self.pending_edges {
            println!("   -- below: {:?}", b);
        }
    }

    fn log_sl_winding(&self) {
        if !self.log {
            return;
        }
        print!("winding: |");
        for edge in &self.active_edges {
            print!("{}", match edge.winding {
                1 => "+",
                -1 => "-",
                0 => "*",
                _ => panic!(),
            });
        }
        println!("|");
    }

    #[cfg(not(test))]
    fn log_sl_points(&self) {}

    #[cfg(test)]
    fn log_sl_points(&self) {
        print!("\n sl: [");
        let mut left = true;
        for edge in &self.active_edges {
            let sep = if left { "|" } else { " " };
            print!("{} {:?} ", sep, edge.points.upper);
            left = !left;
        }
        println!("]");
        print!("     [");
        let mut left = true;
        for edge in &self.active_edges {
            let sep = if left { "|" } else { " " };
            print!("{}", sep);
            if edge.merge {
                print!("   <merge>           ");
            } else {
                print!("{:?} ", edge.points.lower);
            }
            left = !left;
        }
        println!("]\n");
    }
}

impl Default for FillTessellator {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
fn even(edge: ActiveEdgeId) -> bool { edge.handle % 2 == 0 }

#[inline]
fn odd(edge: ActiveEdgeId) -> bool { edge.handle % 2 == 1 }

#[inline]
fn span_for_edge(edge: ActiveEdgeId) -> SpanId {
    // TODO: this assumes the even-odd fill rule.
    SpanId::new(edge.handle / 2)
}

fn compare_positions(a: TessPoint, b: TessPoint) -> Ordering {
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

    Ordering::Equal
}

// Checks whether the edge touches the current position and if not,
// whether the edge is on the right side of the current position.
fn compare_edge_against_position(
    edge: &Edge,
    position: TessPoint,
    on_edge: &mut bool,
    edge_passed_point: &mut bool,
) {
    let threshold = FixedPoint32::epsilon() * 50;

    // This early-out test gives a noticeable performance improvement.
    let (min, max) = edge.upper.x.min_max(edge.lower.x);
    let point_before = position.x < min - threshold;
    let point_after = position.x > max + threshold;

    if point_before || point_after {
        // This should be by far the hotest path in this function.
        *on_edge = false;
        *edge_passed_point = point_before;
        return;
    }

    let v = edge.lower - edge.upper;
    if v.y.is_zero() {
        // Horizontal edge
        debug_assert_eq!(edge.upper.y, edge.lower.y);
        debug_assert_eq!(edge.upper.y, position.y);
        *on_edge = edge.upper.x <= position.x && edge.lower.x >= position.x;
        *edge_passed_point = edge.upper.x > position.x + threshold;
        return;
    }

    // Intersect the edge with the horizontal line passing at the current position.
    let dy = position.y - edge.upper.y;
    let x = edge.upper.x + dy.mul_div(v.x, v.y);

    //println!("dx = {} ({})", x - position.x, (x - position.x).raw());
    *on_edge = (x - position.x).abs() <= threshold;
    *edge_passed_point = !*on_edge && position.x < x;
}

fn prepare_pending_edges(
    pending_edges: &mut Vec<PendingEdge>,
    intersections: &mut Vec<OrientedEdge>,
) {
    pending_edges.sort_by(|a, b| a.angle.partial_cmp(&b.angle).unwrap_or(Ordering::Equal));

    if pending_edges.len() >= 2 {
        let mut to_remove = Vec::new();
        let mut i = 0;
        while i + 1 < pending_edges.len() {
            // This theshold may need to be adjusted if we run into more
            // precision issues with how angles are computed.
            let threshold = 0.0035;
            let edge_a = &pending_edges[i];
            let edge_b = &pending_edges[i+1];
            // TODO: Skipping edges loses track of the correct winding number,
            // hence the check that the winding isn't affected but it is far
            // from ideal.
            let doesnt_affect_winding = true; //edge_a.winding + edge_b.winding == 0;
            if (edge_a.angle - edge_b.angle).abs() < threshold && doesnt_affect_winding {
                to_remove.push(i);
                if edge_a.lower != edge_b.lower {
                    let furthest = if is_after(edge_a.lower, edge_b.lower) { i } else { i + 1 };
                    let winding = pending_edges[furthest].winding;
                    intersections.push(OrientedEdge::with_winding(edge_a.lower, edge_b.lower, winding));
                }
                i += 2;
            } else {
                i += 1;
            }
        }

        while let Some(idx) = to_remove.pop() {
            pending_edges.remove(idx+1);
            pending_edges.remove(idx);
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct ActiveEdge {
    points: Edge,
    upper_id: VertexId,
    winding: i16,
    merge: bool,
}

impl ActiveEdge {
    fn merge_vertex(&mut self, vertex: TessPoint, id: VertexId) {
        self.points.upper = vertex;
        self.upper_id = id;
        self.winding = 0; // TODO ??
        self.merge = true;
    }

    fn set_lower_vertex(&mut self, pos: TessPoint) {
        self.points.lower = pos;
        self.merge = false;
    }
}


// Defines an ordering between two points
//
// A point is considered after another point if it is below (y pointing downward) the point.
// If two points have the same y coordinate, the one on the right (x pointing to the right)
// is the one after.
#[inline]
fn is_after<T: PartialOrd>(a: euclid::default::Point2D<T>, b: euclid::default::Point2D<T>) -> bool {
    a.y > b.y || (a.y == b.y && a.x > b.x)
}

// translate to and from the internal coordinate system.
#[inline]
fn to_internal(v: Point) -> TessPoint { TessPoint::new(fixed(v.x), fixed(v.y)) }
#[inline]
fn to_f32_point(v: TessPoint) -> Point { point(v.x.to_f32(), v.y.to_f32()) }
#[inline]
fn to_f32_vector(v: TessVector) -> Vector { vector(v.x.to_f32(), v.y.to_f32()) }

#[inline]
fn edge_angle(v: TessVector) -> f32 {
    -Trig::fast_atan2(v.y.to_f32(), v.x.to_f32())
}

/// A sequence of edges sorted from top to bottom, to be used as the tessellator's input.
pub struct FillEvents {
    edges: Vec<OrientedEdge>,
    vertices: Vec<TessPoint>,
}

impl FillEvents {
    pub fn from_path<Iter: Iterator<Item = PathEvent<Point, Point>>>(tolerance: f32, it: Iter) -> Self {
        let mut events = FillEvents::new();
        events.set_path(tolerance, it);

        events
    }

    pub fn new() -> Self {
        FillEvents {
            edges: Vec::new(),
            vertices: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.edges.clear();
        self.vertices.clear();
    }

    pub fn set_path<Iter: Iterator<Item = PathEvent<Point, Point>>>(&mut self, tolerance: f32, it: Iter) {
        self.clear();
        let mut tmp = FillEvents::new();
        swap(self, &mut tmp);

        let mut builder = EventsBuilder::new();
        builder.recycle(tmp);
        builder.tolerance = tolerance;

        for evt in it {
            match evt {
                PathEvent::Begin { at } => {
                    builder.move_to(at);
                }
                PathEvent::Line { to, .. } => {
                    builder.line_to(to);
                }
                PathEvent::Quadratic { from, ctrl, to } => {
                    builder.quadratic_segment(
                        QuadraticBezierSegment { from, ctrl, to }
                    );
                }
                PathEvent::Cubic { from, ctrl1, ctrl2, to } => {
                    builder.cubic_segment(
                        CubicBezierSegment { from, ctrl1, ctrl2, to }
                    );
                }
                PathEvent::End { close: true, .. } => {
                    builder.close();
                }
                PathEvent::End { close: false, .. } => {}
            }
        }

        swap(self, &mut builder.build());
    }
}

pub(crate) struct EventsBuilder {
    edges: Vec<OrientedEdge>,
    vertices: Vec<TessPoint>,

    first: TessPoint,
    second: TessPoint,
    previous: TessPoint,
    current: TessPoint,
    nth: u32,
    tolerance: f32,
}

impl EventsBuilder {
    pub(crate) fn new() -> Self {
        EventsBuilder {
            edges: Vec::new(),
            vertices: Vec::new(),

            first: TessPoint::new(fixed(0.0), fixed(0.0)),
            second: TessPoint::new(fixed(0.0), fixed(0.0)),
            previous: TessPoint::new(fixed(0.0), fixed(0.0)),
            current: TessPoint::new(fixed(0.0), fixed(0.0)),
            nth: 0,
            tolerance: 0.1,
        }
    }

    fn recycle(&mut self, events: FillEvents) {
        self.edges = events.edges;
        self.vertices = events.vertices;
    }

    fn add_edge(&mut self, a: TessPoint, b: TessPoint) {
        if a != b {
            self.edges.push(OrientedEdge::new(a, b));
        }
    }

    fn vertex(&mut self, previous: TessPoint, current: TessPoint, next: TessPoint) {
        if is_after(current, previous) && is_after(current, next) {
            self.vertices.push(current);
        }
    }

    fn quadratic_segment(&mut self, mut segment: QuadraticBezierSegment<f32>) {
        // Swap the curve so that it always goes downwards. This way if two
        // paths share the same edge with different windings, the flattening will
        // play out the same way, which avoid cracks.

        // We have to put special care into properly tracking the previous and second
        // points as if we hadn't swapped.

        let segment_from = to_internal(segment.from);
        let segment_to = to_internal(segment.to);
        if segment_from == segment_to {
            return;
        }

        let needs_swap = is_after(segment_from, segment_to);

        if needs_swap {
            swap(&mut segment.from, &mut segment.to);
        }

        let mut from = to_internal(segment.from);
        let mut prev = from;
        let mut first = None;
        let mut nth = self.nth;
        segment.for_each_flattened(self.tolerance, &mut|to| {
            let to = to_internal(to);
            if first == None {
                first = Some(to)
                // We can't call vertex(prev, from, to) in the first iteration
                // because if we swapped the edge, we don't have a proper value for
                // the previous vertex yet.
                // We'll handle it after the loop.
            } else {
                self.vertex(prev, from, to);
            }

            self.add_edge(from, to);

            prev = from;
            from = to;
            nth += 1;
        });

        let first = first.unwrap();
        let (second, previous) = if needs_swap { (prev, first) } else { (first, prev) };

        if self.nth == 0 {
            self.second = second;
        } else {
            // Handle the first vertex we took out of the loop above.
            self.vertex(self.previous, segment_from, second);
        }

        self.previous = previous;
        self.current = segment_to;
        self.nth = nth;
    }

    fn cubic_segment(&mut self, mut segment: CubicBezierSegment<f32>) {
        // This goes through the same gymnastics as quadratic_segment.

        let segment_from = to_internal(segment.from);
        let segment_to = to_internal(segment.to);
        if segment_from == segment_to && to_internal(segment.ctrl1) == to_internal(segment.ctrl2) {
            return;
        }

        let needs_swap = is_after(segment_from, segment_to);

        if needs_swap {
            swap(&mut segment.from, &mut segment.to);
            swap(&mut segment.ctrl1, &mut segment.ctrl2);
        }

        let mut from = to_internal(segment.from);
        let mut prev = from;
        let mut first = None;
        let mut nth = self.nth;
        segment.for_each_flattened(self.tolerance, &mut|to| {
            let to = to_internal(to);
            if first == None {
                first = Some(to)
            } else {
                self.vertex(prev, from, to);
            }

            self.add_edge(from, to);

            prev = from;
            from = to;
            nth += 1;
        });

        let first = first.unwrap();

        let second = if needs_swap { prev } else { first };
        if self.nth == 0 {
            self.second = second;
        } else {
            self.vertex(self.previous, segment_from, second);
        }

        self.previous = if needs_swap { first } else { prev };
        self.current = segment_to;
        self.nth = nth;
    }
}

impl Build for EventsBuilder {
    type PathType = FillEvents;

    fn build(mut self) -> FillEvents {
        self.close();

        self.edges.sort_by(|a, b| compare_positions(a.upper, b.upper));
        self.vertices.sort_by(|a, b| compare_positions(*a, *b));

        FillEvents {
            edges: self.edges,
            vertices: self.vertices,
        }
    }

    fn build_and_reset(&mut self) -> FillEvents {
        self.close();

        self.first = TessPoint::new(fixed(0.0), fixed(0.0));
        self.second = TessPoint::new(fixed(0.0), fixed(0.0));
        self.previous = TessPoint::new(fixed(0.0), fixed(0.0));
        self.current = TessPoint::new(fixed(0.0), fixed(0.0));
        self.nth = 0;

        self.edges.sort_by(|a, b| compare_positions(a.upper, b.upper));
        self.vertices.sort_by(|a, b| compare_positions(*a, *b));

        FillEvents {
            edges: replace(&mut self.edges, Vec::new()),
            vertices: replace(&mut self.vertices, Vec::new()),
        }
    }
}

impl FlatPathBuilder for EventsBuilder {
    fn move_to(&mut self, to: Point) {
        // Because of the internal 16.16 fixed point representation,
        // the tessellator is unable to work with numbers that are
        // bigger than 32767.0.
        //debug_assert!(to.x.abs() <= 32767.0);
        //debug_assert!(to.y.abs() <= 32767.0);
        self.close();
        let next = to_internal(to);
        if self.nth > 1 {
            let current = self.current;
            let previous = self.previous;
            let first = self.first;
            let second = self.second;
            self.add_edge(current, first);
            self.vertex(previous, current, first);
            self.vertex(current, first, second);
        }
        self.first = next;
        self.current = next;
        self.nth = 0;
    }

    fn line_to(&mut self, to: Point) {
        //debug_assert!(to.x.abs() <= 32767.0);
        //debug_assert!(to.y.abs() <= 32767.0);
        let next = to_internal(to);
        if next == self.current {
            return;
        }
        if self.nth == 0 {
            self.second = next;
        }
        let current = self.current;
        let previous = self.previous;
        self.add_edge(current, next);
        if self.nth > 0 {
            self.vertex(previous, current, next);
        }
        self.previous = self.current;
        self.current = next;
        self.nth += 1;
    }

    fn close(&mut self) {
        let current = self.current;
        let first = self.first;
        let previous = self.previous;
        let second = self.second;
        if self.current != self.first {
            if self.nth > 0 {
                self.add_edge(current, first);
                self.vertex(previous, current, first);
            }
            if self.nth > 1 {
                self.vertex(current, first, second);
            }
        } else if self.nth > 1 {
            self.vertex(previous, first, second);
        }
        self.nth = 0;
        self.current = self.first;
    }

    fn current_position(&self) -> Point {
        to_f32_point(self.current)
    }
}

/// Helper class that generates a triangulation from a sequence of vertices describing a monotone
/// polygon (used internally by the `FillTessellator`).
struct MonotoneTessellator {
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
    pub fn new() -> Self {
        MonotoneTessellator {
            stack: Vec::with_capacity(16),
            triangles: Vec::with_capacity(128),
            // Some placeholder value that will be replaced right away.
            previous: MonotoneVertex {
                pos: Point::new(0.0, 0.0),
                id: VertexId(0),
                side: Side::Left,
            },
        }
    }

    pub fn begin(mut self, pos: Point, id: VertexId) -> MonotoneTessellator {
        let first = MonotoneVertex {
            pos,
            id,
            side: Side::Left,
        };
        self.previous = first;

        self.triangles.clear();
        self.stack.clear();
        self.stack.push(first);

        self
    }

    pub fn vertex(&mut self, pos: Point, id: VertexId, side: Side) {
        let current = MonotoneVertex { pos, id, side };

        // cf. test_fixed_to_f32_precision
        // TODO: investigate whether we could do the conversion without this
        // precision issue. Otherwise we could also make MonotoneTessellator
        // manipulate fixed-point values instead of f32 to preverve the assertion.
        //
        //debug_assert!(is_after(current.pos, self.previous.pos));
        debug_assert!(!self.stack.is_empty());

        let changed_side = current.side != self.previous.side;

        if changed_side {
            for i in 0..(self.stack.len() - 1) {
                let mut a = self.stack[i];
                let mut b = self.stack[i + 1];

                let winding = (a.pos - b.pos).cross(current.pos - b.pos) >= 0.0;

                if !winding {
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

                if current.side.is_right() {
                    swap(&mut a, &mut b);
                }

                let cross = (current.pos - b.pos).cross(a.pos - b.pos);
                if cross >= 0.0 {
                    self.push_triangle(&b, &a, &current);
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
        let threshold = -0.042; // Floating point errors stroke again :(
        debug_assert!((a.pos - b.pos).cross(c.pos - b.pos) >= threshold);
        self.triangles.push((a.id, b.id, c.id));
    }

    fn flush(&mut self, output: &mut dyn GeometryBuilder<Vertex>) {
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
        let mut tess = MonotoneTessellator::new().begin(point(0.0, 0.0), VertexId(0));
        tess.vertex(point(-1.0, 1.0), VertexId(1), Side::Left);
        tess.end(point(1.0, 2.0), VertexId(2));
        assert_eq!(tess.triangles.len(), 1);
    }
    println!(" ------------ ");
    {
        let mut tess = MonotoneTessellator::new().begin(point(0.0, 0.0), VertexId(0));
        tess.vertex(point(1.0, 1.0), VertexId(1), Side::Right);
        tess.vertex(point(-1.5, 2.0), VertexId(2), Side::Left);
        tess.vertex(point(-1.0, 3.0), VertexId(3), Side::Left);
        tess.vertex(point(1.0, 4.0), VertexId(4), Side::Right);
        tess.end(point(0.0, 5.0), VertexId(5));
        assert_eq!(tess.triangles.len(), 4);
    }
    println!(" ------------ ");
    {
        let mut tess = MonotoneTessellator::new().begin(point(0.0, 0.0), VertexId(0));
        tess.vertex(point(1.0, 1.0), VertexId(1), Side::Right);
        tess.vertex(point(3.0, 2.0), VertexId(2), Side::Right);
        tess.vertex(point(1.0, 3.0), VertexId(3), Side::Right);
        tess.vertex(point(1.0, 4.0), VertexId(4), Side::Right);
        tess.vertex(point(4.0, 5.0), VertexId(5), Side::Right);
        tess.end(point(0.0, 6.0), VertexId(6));
        assert_eq!(tess.triangles.len(), 5);
    }
    println!(" ------------ ");
    {
        let mut tess = MonotoneTessellator::new().begin(point(0.0, 0.0), VertexId(0));
        tess.vertex(point(-1.0, 1.0), VertexId(1), Side::Left);
        tess.vertex(point(-3.0, 2.0), VertexId(2), Side::Left);
        tess.vertex(point(-1.0, 3.0), VertexId(3), Side::Left);
        tess.vertex(point(-1.0, 4.0), VertexId(4), Side::Left);
        tess.vertex(point(-4.0, 5.0), VertexId(5), Side::Left);
        tess.end(point(0.0, 6.0), VertexId(6));
        assert_eq!(tess.triangles.len(), 5);
    }
    println!(" ------------ ");
}
