// TODO[optim]
//
// # Segment intersection
//
// segment-segment intersection is currently the most perf-sensituve function by far.
// A quick experiment replacing segment_intersection by a dummy function that always
// return None made the tessellation of the log twice faster.
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

use FillVertex as Vertex;
use Side;
use math::*;
use geometry_builder::{GeometryBuilder, Count, VertexId};
use core::FlattenedEvent;
use bezier::utils::fast_atan2;
use math_utils::segment_intersection;
use path_builder::FlatPathBuilder;
use path_iterator::PathIterator;

#[cfg(test)]
use geometry_builder::{VertexBuffers, simple_builder};
#[cfg(test)]
use path::{Path, PathSlice};
#[cfg(test)]
use extra::rust_logo::build_logo_path;

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
struct EdgeBelowTag;
struct SpanTag;
type ActiveEdgeId = Id<ActiveEdgeTag, usize>;
type SpanId = Id<SpanTag, usize>;
type EdgeBelowId = Id<EdgeBelowTag, usize>;
type ActiveEdges = IdVec<ActiveEdgeId, ActiveEdge>;
type EdgesBelow = IdVec<EdgeBelowId, EdgeBelow>;

/// The fill tessellator's result type.
pub type FillResult = Result<Count, FillError>;

/// The fill tessellator's error enumeration.
#[derive(Clone, Debug)]
pub enum FillError {
    Unknown,
}

#[derive(Copy, Clone, Debug)]
pub struct Edge {
    pub upper: TessPoint,
    pub lower: TessPoint,
}

impl Edge {
    pub fn new(mut a: TessPoint, mut b: TessPoint) -> Self {
        if is_after(a, b) {
            swap(&mut a, &mut b);
        }
        Edge { upper: a, lower: b }
    }
}

#[derive(Clone, Debug)]
struct EdgeBelow {
    // The upper vertex is the current vertex, we don't need to store it.
    lower: TessPoint,
    angle: f32,
}

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
/// # extern crate lyon_tessellation;
/// # extern crate lyon_core;
/// # extern crate lyon_path;
/// # extern crate lyon_path_builder;
/// # extern crate lyon_path_iterator;
/// # use lyon_path::Path;
/// # use lyon_path_builder::*;
/// # use lyon_path_iterator::*;
/// # use lyon_core::math::{Point, point};
/// # use lyon_tessellation::geometry_builder::{VertexBuffers, simple_builder};
/// # use lyon_tessellation::*;
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
/// let mut buffers: VertexBuffers<FillVertex> = VertexBuffers::new();
///
/// {
///     // Create the destination vertex and index buffers.
///     let mut vertex_builder = simple_builder(&mut buffers);
///
///     // Create the tessellator.
///     let mut tessellator = FillTessellator::new();
///
///     // Compute the tessellation.
///     let result = tessellator.tessellate_flattened_path(
///         path.path_iter().flattened(0.05),
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
/// # How the fill tessellator works
///
/// Learn more about how the algrorithm works on the [tessellator wiki page](https://github.com/nical/lyon/wiki/Tessellator).
///
pub struct FillTessellator {
    events: FillEvents,
    monotone_tessellators: IdVec<SpanId, MonotoneTessellator>,
    active_edges: ActiveEdges,
    intersections: Vec<Edge>,
    below: Vec<EdgeBelow>,
    previous_position: TessPoint,
    error: Option<FillError>,
    log: bool,
    handle_intersections: bool,
    tess_pool: Vec<MonotoneTessellator>,
}

impl FillTessellator {
    /// Constructor.
    pub fn new() -> FillTessellator {
        FillTessellator {
            events: FillEvents::new(),
            active_edges: ActiveEdges::with_capacity(16),
            monotone_tessellators: IdVec::with_capacity(16),
            below: Vec::with_capacity(8),
            intersections: Vec::with_capacity(8),
            previous_position: TessPoint::new(FixedPoint32::min_val(), FixedPoint32::min_val()),
            error: None,
            log: false,
            handle_intersections: true,
            tess_pool: Vec::with_capacity(8),
        }
    }

    /// Compute the tessellation from a path iterator.
    pub fn tessellate_path<Iter, Output>(
        &mut self,
        it: Iter,
        options: &FillOptions,
        output: &mut Output,
    ) -> FillResult
    where
        Iter: PathIterator,
        Output: GeometryBuilder<Vertex>,
    {
        self.tessellate_flattened_path(
            it.flattened(options.tolerance),
            options,
            output
        )
    }

    /// Compute the tessellation from a flattened path iterator.
    pub fn tessellate_flattened_path<Iter, Output>(
        &mut self,
        it: Iter,
        options: &FillOptions,
        output: &mut Output,
    ) -> FillResult
    where
        Iter: Iterator<Item = FlattenedEvent>,
        Output: GeometryBuilder<Vertex>,
    {
        let mut events = replace(&mut self.events, FillEvents::new());
        events.clear();
        events.set_path_iter(it);
        let result = self.tessellate_events(&events, options, output);
        self.events = events;
        return result;
    }


    /// Compute the tessellation from pre-sorted events.
    pub fn tessellate_events<Output>(
        &mut self,
        events: &FillEvents,
        options: &FillOptions,
        output: &mut Output,
    ) -> FillResult
    where
        Output: GeometryBuilder<Vertex>,
    {
        if options.fill_rule != FillRule::EvenOdd {
            println!("warning: Fill rule {:?} is not supported yet.", options.fill_rule);
        }

        self.handle_intersections = !options.assume_no_intersections;

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
        return Ok(res);
    }

    /// Enable some verbose logging during the tessellation, for debugging purposes.
    pub fn enable_logging(&mut self) { self.log = true; }

    fn reset(&mut self) {
        self.active_edges.clear();
        self.monotone_tessellators.clear();
        self.below.clear();
    }

    fn begin_tessellation<Output: GeometryBuilder<Vertex>>(&mut self, output: &mut Output) {
        debug_assert!(self.active_edges.len() == 0);
        debug_assert!(self.monotone_tessellators.is_empty());
        debug_assert!(self.below.is_empty());
        output.begin_geometry();
    }

    fn end_tessellation<Output: GeometryBuilder<Vertex>>(
        &mut self,
        output: &mut Output,
    ) -> Count {
        debug_assert!(self.active_edges.len() == 0);
        debug_assert!(self.monotone_tessellators.is_empty());
        debug_assert!(self.below.is_empty());
        return output.end_geometry();
    }

    fn tessellator_loop<Output: GeometryBuilder<Vertex>>(
        &mut self,
        events: &FillEvents,
        output: &mut Output,
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

            // We look for the next event by pulling from three sources: the list edges,
            // the list of vertices that don't have a edges immediately under them (end
            // or merge events), and the list of intersections that we find along the way.

            // Look at the sorted list of edges.
            while let Some(edge) = next_edge {
                if edge.upper == current_position {
                    next_edge = edge_iter.next();
                    if edge.lower == current_position {
                        continue;
                    }

                    self.below.push(EdgeBelow {
                        lower: edge.lower,
                        angle: edge_angle(edge.lower - edge.upper),
                    });
                    tess_log!(self, " edge at {:?} -> {:?}", edge.upper, edge.lower);

                    pending_events = true;
                    continue;
                }

                next_position = Some(edge.upper);
                break;
            }

            // Look at the sorted list of vertices.
            while let Some(vertex) = next_vertex {
                if *vertex == current_position {
                    next_vertex = vertex_iter.next();
                    pending_events = true;
                    tess_log!(self, " vertex at {:?}", current_position);
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
                if intersection_position == current_position {
                    let inter = self.intersections.remove(0);

                    if inter.lower != current_position {
                        self.below.push(
                            EdgeBelow {
                                lower: inter.lower,
                                angle: edge_angle(inter.lower - current_position),
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
                self.process_vertex(current_position, output);

                if num_intersections != self.intersections.len() {
                    // We found an intersection durign process_vertex, it has been added
                    // to self.intersections.

                    // Sort the intersection list.
                    self.intersections.sort_by(|a, b| compare_positions(a.upper, b.upper));

                    // The next position may have changed so we go back to the beginning
                    // of the loop to determine the next position.
                    // We could only look at the list of intersections since it is the
                    // only one that changed but it probably does not make a difference
                    // speed-wise and the code is simpler this way.
                    continue;
                }
            }


            if let Some(position) = next_position {
                current_position = position;
                tess_log!(self, " -- current_position is now {:?}", position);
            } else {
                return;
            }
        }
    }

    fn process_vertex<Output: GeometryBuilder<Vertex>>(
        &mut self,
        current_position: TessPoint,
        output: &mut Output,
    ) {
        // This is where the interesting things happen.
        // We go through the sweep line to find all of the edges that end at the current
        // position, and through the list of edges that start at the current position
        // (self.below, which we build in tessellator_loop).
        // we decide what to do depending on the spacial configuration of these edges.
        //
        // The logic here really need to be simplified, it is the trickiest part of the
        // tessellator.

        let vec2_position = to_f32_point(current_position);
        let id = output.add_vertex(
            Vertex {
                position: vec2_position,
                normal: vec2(0.0, 0.0),
            }
        );


        // Go through the sweep line to find the first edge that ends at the current
        // position (if any) or if the current position is inside or outside the shape.
        #[derive(Copy, Clone, Debug, PartialEq)]
        enum E {
            In,
            Out,
            LeftEdge,
            RightEdge,
        };
        let mut status = E::Out;

        // Walk the sweep line to determine where we are with respect to the
        // existing spans.
        let mut edge_idx = Id::new(0);

        for active_edge in &mut self.active_edges {
            let is_left = even(edge_idx);

            if active_edge.merge {
                edge_idx = edge_idx + 1;
                continue;
            }

            if active_edge.points.lower == current_position {
                status = if is_left { E::LeftEdge } else { E::RightEdge };
                break;
            }

            let (touches, edge_passed_current) = compare_edge_against_position(
                &active_edge.points,
                current_position
            );

            if touches {
                // The current point is on an edge we need to split the edge into the part
                // above and the part below. See test_point_on_edge_left for an example of
                // geometry that can lead to this scenario.

                // Split the edge.
                self.below.push(EdgeBelow {
                    lower: active_edge.points.lower,
                    angle: edge_angle(active_edge.points.lower - current_position),
                });
                active_edge.points.lower = current_position;

                status = if is_left { E::LeftEdge } else { E::RightEdge };
                break;
            }

            if edge_passed_current {
                status = if is_left {
                    //       |....
                    //    x  |....
                    //       |....
                    E::Out
                } else {
                    //  .....|
                    //  ..x..|
                    //  .....|
                    E::In
                };
                if !is_left {
                    // Use the edge on the left side of the span we are in.
                    edge_idx = edge_idx - 1;
                }
                break;
            }

            edge_idx = edge_idx + 1;
        }

        self.below.sort_by(|a, b| a.angle.partial_cmp(&b.angle).unwrap_or(Ordering::Equal));

        // Go through all edges below and handle pairs of overlapping edges.
        // Doing this here avoids some potentially tricky cases with intersections
        // later.
        if self.below.len() >= 2 {
            let mut to_remove = Vec::new();
            let mut i = 0;
            while i < self.below.len() - 1 {
                if self.below[i].angle == self.below[i+1].angle {
                    to_remove.push(i);
                    let lower1 = self.below[i].lower;
                    let lower2 = self.below[i + 1].lower;
                    if lower1 != lower2 {
                        self.intersections.push(Edge::new(lower1, lower2));
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            while let Some(idx) = to_remove.pop() {
                self.below.remove(idx+1);
                self.below.remove(idx);
            }
        }

        let start_edge = edge_idx;

        if self.log {
            self.log_sl(current_position, edge_idx);
            //println!("{:?}", status);
            //println!("below:{:?}", self.below);
        }

        // The index of the next edge below the current vertex, to be processed.
        let mut below_idx = 0;

        let mut pending_merge = false;
        let mut below_count = self.below.len();

        // Step 1, walk the sweep line, handle left/right events, handle the spans that end
        // at this vertex, as well as merge events.
        if self.active_edges.has_id(edge_idx) {
            if status == E::RightEdge {
                debug_assert!(odd(edge_idx));
                if below_count == 0 {
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
                    pending_merge = true;
                } else {
                    // Right event.
                    //
                    //   ..../
                    //   ...x
                    //   ....\
                    //
                    tess_log!(self, "(right event) {:?}", edge_idx);

                    let edge_to = self.below[0].lower;
                    self.insert_edge(edge_idx, current_position, edge_to, id);

                    // update the initial state for the pass that will handle
                    // the edges below the current vertex.
                    below_idx += 1;
                    below_count -= 1;
                    status = E::Out;
                }
                edge_idx = edge_idx + 1;
            }
        }

        // Count the number of remaining edges above the sweep line that end at
        // the current position.
        let mut above_count = 0;
        for active_edge in self.active_edges.range_from(edge_idx).iter() {
            if !active_edge.merge {
                if active_edge.points.lower != current_position {
                    break;
                }

                above_count += 1;
            }
        }

        //println!("above count: {}", above_count);

        while above_count >= 2 {
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
            tess_log!(self, "(end event) {:?}", edge_idx);

            self.resolve_merge_vertices(edge_idx, current_position, id, output);
            self.end_span(edge_idx, current_position, id, output);

            above_count -= 2;
        }

        if pending_merge {
            // Merge event.
            //
            // ...\   /...
            // ....\ /....
            // .....x.....
            //
            tess_log!(self, "(merge event) {:?}", edge_idx);

            debug_assert_eq!(above_count, 1);
            self.merge_event(current_position, id, start_edge, output);

        } else if above_count == 1 {
            // Left event.
            //
            //     /...
            //    x....
            //     \...
            //

            debug_assert!(below_count > 0);
            self.resolve_merge_vertices(edge_idx, current_position, id, output);

            let vertex_below = self.below[self.below.len() - 1].lower;
            tess_log!(self, "(left event) {:?}    -> {:?}", edge_idx, vertex_below);
            self.insert_edge(edge_idx, current_position, vertex_below, id);

            below_count -= 1;
        }

        // Since we took care of left and right events already we should not have
        // an odd number of edges to work with below the current vertex by now.
        debug_assert!(below_count % 2  == 0);

        // Step 2, handle edges below the current vertex.
        if below_count > 0 {
            if status == E::In {
                debug_assert!(even(edge_idx));
                // Split event.
                //
                // .....x.....
                // ..../ \....
                // .../   \...
                //
                tess_log!(self, "(split event) {:?}", edge_idx);

                let left = self.below[0].clone();
                let right = self.below[below_count - 1].clone();
                self.split_event(edge_idx, current_position, id, left, right, output);
                below_count -= 2;

                // split_event inserts edges, and separates the span we were in, so
                // we aren't inside of it anymore.
                // Consequently offset the edge_idx by a span.
                edge_idx = edge_idx + 2;
            }

            while below_count >= 2 {
                let left_idx = below_idx;
                let right_idx = below_idx + 1;

                // Start event.
                //
                //      x
                //     /.\
                //    /...\
                //
                tess_log!(self, "(start event) {:?}", edge_idx);

                // TODO: this is very similar to calling insert_edge twice, see if that should
                // be simplified.

                let mut left_edge = Edge {
                    upper: current_position,
                    lower: self.below[left_idx].lower,
                };
                let mut right_edge = Edge {
                    upper: current_position,
                    lower: self.below[right_idx].lower,
                };

                self.check_intersections(&mut left_edge);
                self.check_intersections(&mut right_edge);

                self.active_edges.insert_slice(edge_idx, &[
                    ActiveEdge {
                        points: left_edge,
                        upper_id: id,
                        merge: false,
                    },
                    ActiveEdge {
                        points: right_edge,
                        upper_id: id,
                        merge: false,
                    },
                ]);

                self.insert_span(span_for_edge(edge_idx), current_position, id);

                below_idx += 2;
                below_count -= 2;
                edge_idx = edge_idx + 2;
            }
        }

        self.debug_check_sl(current_position);

        self.below.clear();
    }

    // Look for eventual merge vertices on this span above the current vertex, and connect
    // them to the current vertex.
    // This should be called when processing a vertex that is on the left side of a span.
    fn resolve_merge_vertices<Output: GeometryBuilder<Vertex>>(
        &mut self,
        edge_idx: ActiveEdgeId,
        current: TessPoint,
        id: VertexId,
        output: &mut Output,
    ) {
        // we are expecting this to be called with the index of the beginning (left side)
        // of a span, so the index should be even. This won't necessary hold true when we
        // add support for other fill rules.
        debug_assert!(even(edge_idx));

        while self.active_edges[edge_idx + 1].merge {
            //     \ /
            //  \   x   <-- merge vertex
            //   \ :
            //    x   <-- current vertex
            self.active_edges[edge_idx + 2].set_lower_vertex(current);
            self.end_span(edge_idx, current, id, output);
        }
    }

    fn split_event<Output: GeometryBuilder<Vertex>>(
        &mut self,
        edge_idx: ActiveEdgeId,
        current: TessPoint,
        id: VertexId,
        left: EdgeBelow,
        right: EdgeBelow,
        output: &mut Output,
    ) {
        debug_assert!(even(edge_idx));
        // Look whether the span shares a merge vertex with the previous one
        if self.active_edges[edge_idx].merge {
            let left_span_edge = edge_idx - 1;
            let right_span_edge = edge_idx;
            debug_assert!(self.active_edges[left_span_edge].merge);
            //            \ /
            //             x   <-- merge vertex
            //  left_span  :  righ_span
            //             x   <-- current split vertex
            //           l/ \r
            self.insert_edge(left_span_edge, current, left.lower, id);
            self.insert_edge(right_span_edge, current, right.lower, id);

            // There may be more merge vertices chained on the right of the current span, now
            // we are in the same configuration as a left event.
            self.resolve_merge_vertices(edge_idx, current, id, output);
        } else {
            //      /
            //     x
            //    / :r2
            // ll/   x   <-- current split vertex
            //  left/ \right
            let ll = self.active_edges[edge_idx];
            let r2 = Edge {
                upper: ll.points.upper,
                lower: current,
            };

            // TODO this can be simplified, we are reinserting ll and the mutating the edge that was ll before.
            self.active_edges.insert_slice(edge_idx, &[
                ActiveEdge {
                    points: ll.points,
                    upper_id: ll.upper_id,
                    merge: false,
                },
                ActiveEdge {
                    points: r2,
                    upper_id: ll.upper_id,
                    merge: false,
                },
            ]);

            self.active_edges[edge_idx + 2] = ActiveEdge {
                points: r2,
                upper_id: id,
                merge: false,
            };

            self.insert_span(span_for_edge(edge_idx), ll.points.upper, ll.upper_id);

            self.insert_edge(edge_idx + 1, current, left.lower, id);
            self.insert_edge(edge_idx + 2, current, right.lower, id);
        }
    }

    fn merge_event<Output: GeometryBuilder<Vertex>>(
        &mut self,
        position: TessPoint,
        id: VertexId,
        edge_idx: ActiveEdgeId,
        output: &mut Output,
    ) {
        debug_assert!(odd(edge_idx));
        debug_assert!(self.active_edges.has_id(edge_idx + 2));

        let left_span_edge = edge_idx;
        let right_span_edge = edge_idx + 1;

        //     / \ /
        //  \ / .-x    <-- merge vertex
        //   x-'      <-- current merge vertex
        self.resolve_merge_vertices(right_span_edge, position, id, output);

        let vec2_position = to_f32_point(position);

        self.active_edges[left_span_edge].merge_vertex(position, id);
        self.active_edges[right_span_edge].merge_vertex(position, id);

        self.monotone_tessellators[span_for_edge(left_span_edge)].vertex(vec2_position, id, Side::Right);
        self.monotone_tessellators[span_for_edge(right_span_edge)].vertex(vec2_position, id, Side::Left);
    }

    fn insert_edge(
        &mut self,
        edge_idx: ActiveEdgeId,
        upper: TessPoint,
        lower: TessPoint,
        id: VertexId,
    ) {
        debug_assert!(!is_after(upper, lower));
        // TODO horrible hack: set the merge flag on the edge we are about to replace temporarily
        // so that it doesn not get in the way of the intersection detection.
        self.active_edges[edge_idx].merge = true;
        let mut edge = Edge {
            upper: upper,
            lower: lower,
        };
        self.check_intersections(&mut edge);
        // This sets the merge flag to false.
        self.active_edges[edge_idx] = ActiveEdge { points: edge, upper_id: id, merge: false };

        let side = if even(edge_idx) { Side::Left } else { Side::Right };
        let vec2_position = to_f32_point(upper);
        self.monotone_tessellators[span_for_edge(edge_idx)].vertex(vec2_position, id, side);
    }

    fn check_intersections(&mut self, new_edge: &mut Edge) {
        // Test and for intersections against the edges in the sweep line.
        // If an intersecton is found, the edge is split and retains only the part
        // above the intersection point. The lower part is kept with the intersection
        // to be processed later when the sweep line reaches it.
        // If there are several intersections we only keep the one that is closest to
        // the sweep line.
        //
        // TODO: This function is more complicated (and slower) than it needs to be.

        if !self.handle_intersections {
            return;
        }

        struct Intersection {
            point: TessPoint,
            lower1: TessPoint,
            lower2: TessPoint,
        }

        let original_edge = new_edge.clone();
        let mut intersection = None;

        for (edge_idx, edge) in self.active_edges.iter_mut().enumerate() {
            // Test for an intersection against the span's left edge.
            if !edge.merge {
                if let Some(position) = segment_intersection(&new_edge, &edge.points) {
                    tess_log!(self, " -- found an intersection at {:?}
                                    |    {:?}->{:?} x {:?}->{:?}",
                        position,
                        original_edge.upper, original_edge.lower,
                        edge.points.upper, edge.points.lower,
                    );

                    intersection = Some((
                        Intersection {
                            point: position,
                            lower1: original_edge.lower,
                            lower2: edge.points.lower,
                        },
                        ActiveEdgeId::new(edge_idx),
                    ));
                    // From now on only consider potential intersections above the one we found,
                    // by removing the lower part from the segment we test against.
                    new_edge.lower = position;
                }
            }
        }

        let (mut intersection, edge_idx) = match intersection {
            Some((evt, idx)) => (evt, idx),
            None => { return; }
        };

        let current_position = original_edge.upper;

        // Because precision issues, it can happen that the intersection appear to be
        // "above" the current vertex (in fact it is at the same y but on its left which
        // counts as above). Since we can't come back in time to process the intersection
        // before the current vertex, we can only cheat by moving the interseciton down by
        // one unit.
        if !is_after(intersection.point, current_position) {
            intersection.point.y = current_position.y + FixedPoint32::epsilon();
            new_edge.lower = intersection.point;
        }

        tess_log!(
            self,
            " set edge[{:?}].poinst.lower = {:?} (was {:?})",
            edge_idx,
            intersection.point,
            self.active_edges[edge_idx].points.lower
        );

        self.active_edges[edge_idx].points.lower = intersection.point;
        self.intersections.push(Edge::new(intersection.point, intersection.lower1));
        self.intersections.push(Edge::new(intersection.point, intersection.lower2));

        // We sill sort the intersection vector lazily.
    }

    fn end_span<Output: GeometryBuilder<Vertex>>(
        &mut self,
        edge_idx: ActiveEdgeId,
        position: TessPoint,
        id: VertexId,
        output: &mut Output,
    ) {
        // This assertion is only valid with the EvenOdd fill rule.
        debug_assert!(even(edge_idx));
        let span_idx = span_for_edge(edge_idx);

        let vec2_position = to_f32_point(position);
        {
            let tess = &mut self.monotone_tessellators[span_idx];
            tess.end(vec2_position, id);
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

    fn error(&mut self, err: FillError) {
        tess_log!(self, " !! FillTessellator Error {:?}", err);
        self.error = Some(err);
    }

    #[cfg(not(debug))]
    fn debug_check_sl(&self, _: TessPoint) {}

    #[cfg(debug)]
    fn debug_check_sl(&self, current: TessPoint) {
        for edge in &self.active_edges {
            if !edge.merge {
                debug_assert!(
                    !is_after(current, edge.points.lower),
                    "current {:?} should not be below lower {:?}",
                    current,
                    edge.points.lower
                );
                debug_assert!(
                    !is_after(edge.points.upper, edge.points.lower),
                    "upper left {:?} should not be below lower {:?}",
                    edge.points.upper,
                    edge.points.lower
                );
            }
        }
    }

    #[cfg(not(debug))]
    fn log_sl(&self, _: TessPoint, _: ActiveEdgeId) {}

    #[cfg(debug)]
    fn log_sl(&self, current_position: TessPoint, start_edge: ActiveEdgeId) {
        println!("\n\n");
        self.log_sl_ids();
        self.log_sl_points_at(current_position.y);
        println!("\n ----- current: {:?} ------ offset {:?} in sl", current_position, start_edge.handle);
        for b in &self.below {
            println!("   -- below: {:?}", b);
        }
    }

    #[cfg(not(debug))]
    fn log_sl_ids(&self) {}

    #[cfg(debug)]
    fn log_sl_ids(&self) {
        print!("\n|  sl: ");
        let mut left = true;
        for edge in &self.active_edges {
            let m = if edge.merge { "*" } else { " " };
            let sep = if left { " |" } else { " " };
            print!(
                "{} {:?}{} ",
                sep,
                edge.upper_id.offset(), m,
            );
            left = !left
        }
        println!("");
    }

    #[cfg(not(debug))]
    fn log_sl_points(&self) {}

    #[cfg(debug)]
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

    #[cfg(not(debug))]
    fn log_sl_points_at(&self, _: FixedPoint32) {}

    #[cfg(debug)]
    fn log_sl_points_at(&self, y: FixedPoint32) {
        print!("\nat y={:?}  sl: [", y);
        let mut left = true;
        for edge in &self.active_edges {
            let sep = if left { "|" } else { " " };
            print!("{}", sep);
            if edge.merge {
                print!("<merge> ");
            } else {
                let x = line_horizontal_intersection_fixed(&edge.points, y);
                print!("{:?} ", x);
            }
            left = !left;
        }
        println!("]\n");
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
    return Ordering::Equal;
}

// Returns whether the edge touches the current position and if not,
// whether the edge is on the right side of the current position.
fn compare_edge_against_position(edge: &Edge, position: TessPoint) -> (bool, bool) {
    // This early-out test gives a noticeable performance improvement.
    let (min, max) = edge.upper.x.min_max(edge.lower.x);
    let is_before = position.x < min;
    let is_after = position.x > max;
    if is_before || is_after {
        // This should be by far the hotest path in this function.
        return (false, is_before);
    }

    let v = edge.lower - edge.upper;
    if v.y.is_zero() {
        // Horizontal edge
        debug_assert_eq!(edge.upper.y, edge.lower.y);
        debug_assert_eq!(edge.upper.y, position.y);
        let touches = edge.upper.x <= position.x && edge.lower.x >= position.x;
        return (touches, false);
    }

    // Intersect the edge with the horizontal line passing at the current position.
    let dy: FixedPoint64 = (position.y - edge.upper.y).to_fp64();
    let x = edge.upper.x + dy.mul_div(v.x.to_fp64(), v.y.to_fp64()).to_fp32();

    let threshold = FixedPoint32::epsilon() * 2;
    if (x - position.x).abs() <= threshold {
        return (true, is_before);
    }

    return (false, position.x < x);
}

#[derive(Copy, Clone, Debug)]
struct ActiveEdge {
    points: Edge,
    upper_id: VertexId,
    merge: bool,
}

impl ActiveEdge {
    fn merge_vertex(&mut self, vertex: TessPoint, id: VertexId) {
        self.points.upper = vertex;
        self.upper_id = id;
        self.merge = true;
    }

    fn set_lower_vertex(&mut self, pos: TessPoint) {
        self.points.lower = pos;
        self.merge = false;
    }
}


/// Defines an ordering between two points
///
/// A point is considered after another point if it is below (y pointing downward) the point.
/// If two points have the same y coordinate, the one on the right (x pointing to the right)
/// is the one after.
#[inline]
pub fn is_after<T: PartialOrd, U>(a: TypedPoint2D<T, U>, b: TypedPoint2D<T, U>) -> bool {
    a.y > b.y || (a.y == b.y && a.x > b.x)
}

// translate to and from the internal coordinate system.
#[inline]
fn to_internal(v: Point) -> TessPoint { TessPoint::new(fixed(v.x), fixed(v.y)) }
#[inline]
fn to_f32_point(v: TessPoint) -> Point { point(v.x.to_f32(), v.y.to_f32()) }
#[inline]
fn to_f32_vec2(v: TessVec2) -> Vec2 { vec2(v.x.to_f32(), v.y.to_f32()) }

#[inline]
fn edge_angle(v: TessVec2) -> f32 {
    return -fast_atan2(v.y.to_f32(), v.x.to_f32());
}

/// A sequence of edges sorted from top to bottom, to be used as the tessellator's input.
pub struct FillEvents {
    edges: Vec<Edge>,
    vertices: Vec<TessPoint>,
}

impl FillEvents {
    pub fn from_iterator<Iter: Iterator<Item = FlattenedEvent>>(it: Iter) -> Self {
        EventsBuilder::new().build_iter(it)
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

    pub fn set_path_iter<Iter: Iterator<Item = FlattenedEvent>>(&mut self, it: Iter) {
        self.clear();
        let mut tmp = FillEvents::new();
        swap(self, &mut tmp);
        let mut builder = EventsBuilder::new();
        builder.recycle(tmp);
        let mut tmp = builder.build_iter(it);
        swap(self, &mut tmp);
    }
}

pub(crate) struct EventsBuilder {
    edges: Vec<Edge>,
    vertices: Vec<TessPoint>,

    first: TessPoint,
    second: TessPoint,
    previous: TessPoint,
    current: TessPoint,
    nth: u32,
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
        }
    }

    fn recycle(&mut self, events: FillEvents) {
        self.edges = events.edges;
        self.vertices = events.vertices;
    }

    fn build_iter<Iter: Iterator<Item = FlattenedEvent>>(mut self, inputs: Iter) -> FillEvents {
        for evt in inputs {
            match evt {
                FlattenedEvent::MoveTo(to) => { self.move_to(to) }
                FlattenedEvent::LineTo(to) => { self.line_to(to) }
                FlattenedEvent::Close => { self.close(); }
            }
        }

        return self.build();
    }

    fn add_edge(&mut self, a: TessPoint, b: TessPoint) {
        if a != b {
            self.edges.push(Edge::new(a, b));
        }
    }

    fn vertex(&mut self, previous: TessPoint, current: TessPoint, next: TessPoint) {
        if is_after(current, previous) && is_after(current, next) {
            self.vertices.push(current);
        }
    }
}

impl FlatPathBuilder for EventsBuilder {
    type PathType = FillEvents;

    fn move_to(&mut self, to: Point) {
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
        } else {
            if self.nth > 1 {
                self.vertex(previous, first, second);
            }
        }
        self.nth = 0;
        self.current = self.first;
    }

    fn build(mut self) -> FillEvents {
        self.close();

        self.edges.sort_by(|a, b| compare_positions(a.upper, b.upper));
        self.vertices.sort_by(|a, b| compare_positions(*a, *b));

        return FillEvents {
            edges: self.edges,
            vertices: self.vertices,
        };
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

        return FillEvents {
            edges: replace(&mut self.edges, Vec::new()),
            vertices: replace(&mut self.vertices, Vec::new()),
        };
    }

    fn current_position(&self) -> Point {
        to_f32_point(self.current)
    }
}

#[cfg(test)]
use path_builder::*;

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

    let events = EventsBuilder::new().build_iter(path.path_iter().flattened(0.05));
    let mut buffers: VertexBuffers<Vertex> = VertexBuffers::new();
    let mut vertex_builder = simple_builder(&mut buffers);
    let mut tess = FillTessellator::new();
    tess.enable_logging();
    tess.tessellate_events(&events, &FillOptions::default(), &mut vertex_builder).unwrap();
}

/// The fill rule defines how to determine what is inside and what is outside of the shape.
///
/// See the SVG specification.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FillRule {
    EvenOdd,
    NonZero,
}

/// Parameters for the tessellator.
pub struct FillOptions {
    /// Maximum allowed distance to the path when building an approximation.
    ///
    /// See [Flattening and tolerance](index.html#flattening-and-tolerance).
    ///
    /// Default value: `0.1`.
    pub tolerance: f32,

    /// Set the fill rule.
    ///
    /// See the [SVG specification](https://www.w3.org/TR/SVG/painting.html#FillRuleProperty).
    /// Currently, only the `EvenOdd` rule is implemented.
    ///
    /// Default value: `EvenOdd`.
    pub fill_rule: FillRule,

    /// A fast path to avoid some expensive operations if the path is known to
    /// not have any self-intersections.
    ///
    /// Do not set this to `true` if the path may have intersecting edges else
    /// the tessellator may panic or produce incorrect results. In doubt, do not
    /// change the default value.
    ///
    /// Default value: `false`.
    pub assume_no_intersections: bool,

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
            assume_no_intersections: false,
            _private: (),
        }
    }

    pub fn tolerance(tolerance: f32) -> Self {
        FillOptions::default().with_tolerance(tolerance)
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

    pub fn assume_no_intersections(mut self) -> FillOptions {
        self.assume_no_intersections = true;
        return self;
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
            pos: pos,
            id: id,
            side: Side::Left,
        };
        self.previous = first;

        self.triangles.clear();
        self.stack.clear();
        self.stack.push(first);

        self
    }

    pub fn vertex(&mut self, pos: Point, id: VertexId, side: Side) {
        let current = MonotoneVertex {
            pos: pos,
            id: id,
            side: side,
        };

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

                let winding = (current.pos - b.pos).cross(a.pos - b.pos) >= 0.0;

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
        debug_assert!((c.pos - b.pos).cross(a.pos - b.pos) >= 0.0);
        self.triangles.push((a.id, b.id, c.id));
    }

    fn flush<Output: GeometryBuilder<Vertex>>(&mut self, output: &mut Output) {
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

#[cfg(test)]
fn tessellate_path(path: PathSlice, log: bool) -> Result<usize, FillError> {
    let mut buffers: VertexBuffers<Vertex> = VertexBuffers::new();
    {
        let mut vertex_builder = simple_builder(&mut buffers);
        let mut tess = FillTessellator::new();
        if log {
            tess.enable_logging();
        }
        try!{
            tess.tessellate_flattened_path(path.path_iter().flattened(0.05), &FillOptions::default(), &mut vertex_builder)
        };
    }
    return Ok(buffers.indices.len() / 3);
}

#[cfg(test)]
fn test_path(path: PathSlice, expected_triangle_count: Option<usize>) {
    let res = ::std::panic::catch_unwind(|| tessellate_path(path, false));

    if let Ok(Ok(num_triangles)) = res {
        if let Some(expected_triangles) = expected_triangle_count {
            if num_triangles != expected_triangles {
                tessellate_path(path, true).unwrap();
                panic!("expected {} triangles, got {}", expected_triangles, num_triangles);
            }
        }
        return;
    }

    ::extra::debugging::find_reduced_test_case(
        path,
        &|path: Path| { return tessellate_path(path.as_slice(), false).is_err(); },
    );

    tessellate_path(path, true).unwrap();
    panic!();
}

#[cfg(test)]
fn test_path_with_rotations(path: Path, step: f32, expected_triangle_count: Option<usize>) {
    use std::f32::consts::PI;

    let mut angle = 0.0;
    while angle < PI * 2.0 {
        println!("\n\n ==================== angle = {}", angle);

        let mut tranformed_path = path.clone();
        let cos = angle.cos();
        let sin = angle.sin();
        for v in tranformed_path.mut_points() {
            let (x, y) = (v.x, v.y);
            v.x = x * cos + y * sin;
            v.y = y * cos - x * sin;
        }

        test_path(tranformed_path.as_slice(), expected_triangle_count);

        angle += step;
    }
}

#[test]
fn test_simple_triangle() {
    let mut path = Path::builder();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(1.0, 1.0));
    path.line_to(point(0.0, 1.0));
    path.close();

    test_path_with_rotations(path.build(), 0.01, Some(1));
}

#[test]
fn test_simple_monotone() {
    let mut path = Path::builder();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(-1.0, 1.0));
    path.line_to(point(-3.0, 2.0));
    path.line_to(point(-1.0, 3.0));
    path.line_to(point(-4.0, 5.0));
    path.line_to(point(0.0, 6.0));
    path.close();

    let path = path.build();
    test_path(path.as_slice(), Some(4));
}

#[test]
fn test_simple_split() {
    let mut path = Path::builder();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(2.0, 1.0));
    path.line_to(point(2.0, 3.0));
    path.line_to(point(1.0, 2.0));
    path.line_to(point(0.0, 3.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, Some(3));
}

#[test]
fn test_simple_merge_split() {
    let mut path = Path::builder();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(1.0, 1.0));
    path.line_to(point(2.0, 0.0));
    path.line_to(point(2.0, 3.0));
    path.line_to(point(1.0, 2.0));
    path.line_to(point(0.0, 3.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, Some(4));
}

#[test]
fn test_simple_aligned() {
    let mut path = Path::builder();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(1.0, 0.0));
    path.line_to(point(2.0, 0.0));
    path.line_to(point(2.0, 1.0));
    path.line_to(point(2.0, 2.0));
    path.line_to(point(1.0, 2.0));
    path.line_to(point(0.0, 2.0));
    path.line_to(point(0.0, 1.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, Some(6));
}

#[test]
fn test_simple_1() {
    let mut path = Path::builder();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(1.0, 1.0));
    path.line_to(point(2.0, 0.0));
    path.line_to(point(1.0, 3.0));
    path.line_to(point(0.5, 4.0));
    path.line_to(point(0.0, 3.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, Some(4));
}

#[test]
fn test_simple_2() {
    let mut path = Path::builder();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(1.0, 0.0));
    path.line_to(point(2.0, 0.0));
    path.line_to(point(3.0, 0.0));
    path.line_to(point(3.0, 1.0));
    path.line_to(point(3.0, 2.0));
    path.line_to(point(3.0, 3.0));
    path.line_to(point(2.0, 3.0));
    path.line_to(point(1.0, 3.0));
    path.line_to(point(0.0, 3.0));
    path.line_to(point(0.0, 2.0));
    path.line_to(point(0.0, 1.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, Some(10));
}

#[test]
fn test_hole_1() {
    let mut path = Path::builder();
    path.move_to(point(-11.0, 5.0));
    path.line_to(point(0.0, -5.0));
    path.line_to(point(10.0, 5.0));
    path.close();

    path.move_to(point(-5.0, 2.0));
    path.line_to(point(0.0, -2.0));
    path.line_to(point(4.0, 2.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, Some(6));
}

#[test]
fn test_degenerate_empty() { test_path(Path::new().as_slice(), Some(0)); }

#[test]
fn test_degenerate_same_position() {
    let mut path = Path::builder();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(0.0, 0.0));
    path.line_to(point(0.0, 0.0));
    path.line_to(point(0.0, 0.0));
    path.line_to(point(0.0, 0.0));
    path.line_to(point(0.0, 0.0));
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
    path.move_to(point(0.0, 0.0));
    path.line_to(point(2.0, 1.0));
    path.line_to(point(0.0, 2.0));
    path.line_to(point(2.0, 3.0));
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
    path.move_to(point(0.0, 0.0));
    path.line_to(point(2.0, 3.0));
    path.line_to(point(2.0, 1.0));
    path.line_to(point(0.0, 2.0));
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
    path.move_to(point(20.0, 20.0));
    path.line_to(point(60.0, 20.0));
    path.line_to(point(60.0, 60.0));
    path.line_to(point(20.0, 60.0));
    path.close();

    path.move_to(point(40.0, 10.0));
    path.line_to(point(70.0, 40.0));
    path.line_to(point(40.0, 70.0));
    path.line_to(point(10.0, 40.0));
    path.close();

    let path = path.build();
    test_path(path.as_slice(), Some(8));
}

#[test]
fn test_rust_logo() {
    let mut path = Path::builder().flattened(0.011).with_svg();

    build_logo_path(&mut path);

    test_path_with_rotations(path.build(), 0.011, None);
}

#[test]
fn test_rust_logo_with_intersection() {
    let mut path = Path::builder().flattened(0.011).with_svg();

    build_logo_path(&mut path);

    path.move_to(point(10.0, 30.0));
    path.line_to(point(130.0, 30.0));
    path.line_to(point(130.0, 60.0));
    path.line_to(point(10.0, 60.0));
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

    scale_path(&mut path, 8000.0);
    test_path(path.as_slice(), None);
}

#[test]
fn test_rust_logo_scale_up_2() {
    // This test triggers integers overflow in the tessellator.
    // In order to fix this type issue we need to:
    // * Look at the computation that is casuing trouble and see if it can be expressed in
    //   a way that is less subject to overflows.
    // * See if we can define a safe interval where no path can trigger overflows and scale
    //   all paths to this interval internally in the tessellator.
    let mut builder = Path::builder().flattened(0.011).with_svg();
    build_logo_path(&mut builder);
    let mut path = builder.build();

    scale_path(&mut path, 100000.0);
    test_path(path.as_slice(), None);
}

#[test]
fn test_rust_logo_scale_down() {
    // The goal of this test is to check that the tessellator can handle very small geometry.

    let mut builder = Path::builder().flattened(0.011).with_svg();
    build_logo_path(&mut builder);
    let mut path = builder.build();

    scale_path(&mut path, 0.005);
    test_path(path.as_slice(), None);
}

#[test]
fn test_rust_logo_scale_down2() {
    // Issues with very small paths.

    let mut builder = Path::builder().flattened(0.011).with_svg();
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

    path.move_to(point(80.041534, 19.24472));
    path.line_to(point(76.56131, 23.062233));
    path.line_to(point(67.26949, 23.039438));
    path.line_to(point(65.989944, 23.178522));
    path.line_to(point(59.90927, 19.969215));
    path.line_to(point(56.916714, 25.207449));
    path.line_to(point(50.333813, 23.25274));
    path.line_to(point(48.42367, 28.978098));
    path.close();
    path.move_to(point(130.32213, 28.568213));
    path.line_to(point(130.65213, 58.5664));
    path.line_to(point(10.659382, 59.88637));
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

    path.move_to(point(1.0, 0.0));
    path.line_to(point(2.0, 1.0)); // <-- merge
    path.line_to(point(3.0, 0.0));
    path.line_to(point(4.0, 2.0)); // <-- merge
    path.line_to(point(5.0, 0.0));
    path.line_to(point(6.0, 3.0)); // <-- merge
    path.line_to(point(7.0, 0.0));
    path.line_to(point(5.0, 8.0)); // <-- end
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

    path.move_to(point(1.0, 0.0));
    path.line_to(point(2.0, 1.0)); // <-- merge
    path.line_to(point(3.0, 0.0));
    path.line_to(point(4.0, 2.0)); // <-- merge
    path.line_to(point(5.0, 0.0));
    path.line_to(point(6.0, 3.0)); // <-- merge
    path.line_to(point(7.0, 0.0));
    path.line_to(point(7.0, 5.0));
    path.line_to(point(0.0, 4.0)); // <-- left
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

    path.move_to(point(1.0, 0.0));
    path.line_to(point(2.0, 1.0)); // <-- merge
    path.line_to(point(3.0, 0.0));
    path.line_to(point(4.0, 2.0)); // <-- merge
    path.line_to(point(5.0, 0.0));
    path.line_to(point(6.0, 3.0)); // <-- merge
    path.line_to(point(7.0, 0.0));
    path.line_to(point(7.0, 5.0));
    path.line_to(point(-1.0, 5.0));
    path.line_to(point(-1.0, 0.0));
    path.line_to(point(0.0, 4.0)); // <-- merge (resolving)
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

    path.move_to(point(1.0, 0.0));
    path.line_to(point(2.0, 1.0)); // <-- merge
    path.line_to(point(3.0, 0.0));
    path.line_to(point(4.0, 2.0)); // <-- merge
    path.line_to(point(5.0, 0.0));
    path.line_to(point(6.0, 3.0)); // <-- merge
    path.line_to(point(7.0, 0.0));
    path.line_to(point(7.0, 5.0));
    path.line_to(point(4.0, 4.0)); // <-- split
    path.line_to(point(1.0, 5.0));
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

    builder.move_to(point(-34.619564, 111.88655));
    builder.line_to(point(-35.656174, 111.891));
    builder.line_to(point(-39.304527, 121.766914));
    builder.close();

    builder.move_to(point(1.4426613, 133.40884));
    builder.line_to(point(-27.714422, 140.47032));
    builder.line_to(point(-55.960342, 23.841988));
    builder.close();

    test_path(builder.build().as_slice(), None);
}

#[test]
fn test_overlapping_with_intersection() {
    // There are two overlapping segments a-b and b-a intersecting a segment
    // c-d.
    // This test used to fail until overlapping edges got dealt with before
    // intersection detection. The issue was that the one of the overlapping
    // edges would intersect properly and the second would end up in the degenerate
    // case where it would pass though a pain between two segments without
    // registering as an intersection.
    //
    //       a
    //     / | \
    //    c--|--d
    //       |
    //       b

    let mut builder = Path::builder();

    builder.move_to(point(2.0, -1.0));
    builder.line_to(point(2.0, -3.0));
    builder.line_to(point(3.0, -2.0));
    builder.line_to(point(1.0, -2.0));
    builder.line_to(point(2.0, -3.0));
    builder.close();

    test_path(builder.build().as_slice(), None);
}

#[test]
fn test_split_with_intersections() {
    // This is a reduced test case that was showing a bug where duplicate intersections
    // were found during a split event, due to the sweep line beeing into a temporarily
    // inconsistent state when insert_edge was called.

    let mut builder = Path::builder();

    builder.move_to(point(-21.004179, -71.57515));
    builder.line_to(point(-21.927473, -70.94977));
    builder.line_to(point(-23.024633, -70.68942));
    builder.close();
    builder.move_to(point(16.036617, -27.254852));
    builder.line_to(point(-62.83691, -117.69249));
    builder.line_to(point(38.646027, -46.973236));
    builder.close();

    let path = builder.build();

    test_path(path.as_slice(), None);
}

#[test]
fn test_colinear_1() {
    let mut builder = Path::builder();
    builder.move_to(point(20.0, 150.0));
    builder.line_to(point(80.0, 150.0));
    builder.close();

    let path = builder.build();

    test_path_with_rotations(path, 0.01, None);
}

#[test]
fn test_colinear_2() {
    let mut builder = Path::builder();
    builder.move_to(point(20.0, 150.0));
    builder.line_to(point(80.0, 150.0));
    builder.line_to(point(20.0, 150.0));
    builder.close();

    let path = builder.build();

    test_path_with_rotations(path, 0.01, None);
}

#[test]
fn test_colinear_3() {
    let mut builder = Path::builder();
    // The path goes through many points along a line.
    builder.move_to(point(0.0, 1.0));
    builder.line_to(point(0.0, 3.0));
    builder.line_to(point(0.0, 5.0));
    builder.line_to(point(0.0, 4.0));
    builder.line_to(point(0.0, 2.0));
    builder.close();

    let path = builder.build();

    tessellate_path(path.as_slice(), true).unwrap();
}

#[test]
fn test_colinear_4() {
    // The path goes back and forth along a line.
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 2.0));
    builder.line_to(point(0.0, 1.0));
    builder.line_to(point(0.0, 3.0));
    builder.line_to(point(0.0, 0.0));
    builder.close();

    let path = builder.build();

    tessellate_path(path.as_slice(), true).unwrap();
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
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(0.0, 1.0));

    builder.move_to(point(1.0, 0.0));
    builder.line_to(point(2.0, 0.0));
    builder.line_to(point(2.0, 1.0));
    builder.line_to(point(1.0, 1.0));

    builder.close();

    let path = builder.build();

    tessellate_path(path.as_slice(), true).unwrap();
}

#[test]
#[ignore] //TODO
fn back_along_previous_edge_failing() {
    // This test case seems to have edges that come back along the previous edge.
    // it was found by accidentally advancing with a negative t during flattening.
    let mut builder = Path::builder();

    builder.move_to(point(0.007982401, 0.0121872));
    builder.line_to(point(0.008415101, 0.0116545));
    builder.line_to(point(0.008623006, 0.011589845));
    builder.line_to(point(0.008464893, 0.011639819));
    builder.line_to(point(0.0122631, 0.0069716));
    builder.close();

    test_path(builder.build().as_slice(), None);
}

#[test]
fn test_colinear_touching_squares2_failing() {
    // Two squares touching.
    //
    // x-----x
    // |     x-----x
    // |     |     |
    // x-----x     |
    //       x-----x
    //
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(10.0, 0.0));
    builder.line_to(point(10.0, 10.0));
    builder.line_to(point(0.0, 10.0));

    builder.move_to(point(10.0, 1.0));
    builder.line_to(point(20.0, 1.0));
    builder.line_to(point(20.0, 11.0));
    builder.line_to(point(10.0, 11.0));

    builder.close();

    let path = builder.build();

    tessellate_path(path.as_slice(), true).unwrap();
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
    builder.move_to(point(0.0, 1.0));
    builder.line_to(point(10.0, 1.0));
    builder.line_to(point(10.0, 11.0));
    builder.line_to(point(0.0, 11.0));

    builder.move_to(point(10.0, 0.0));
    builder.line_to(point(20.0, 0.0));
    builder.line_to(point(20.0, 10.0));
    builder.line_to(point(10.0, 10.0));

    builder.close();

    let path = builder.build();

    tessellate_path(path.as_slice(), true).unwrap();
}


#[test]
fn test_unknown_issue_1() {
    // This test case used to fail but does not fail anymore, probably thanks to
    // the fixed-to-f32 workaround (cf.) test_fixed_to_f32_precision.
    // TODO: figure out what the issue was and what fixed it.
    let mut builder = Path::builder();

    builder.move_to(point(-3.3709216, 9.467676));
    builder.line_to(point(-13.078612, 7.0675235));
    builder.line_to(point(-10.67846, -2.6401677));
    builder.close();

    builder.move_to(point(-4.800305, 19.415382));
    builder.line_to(point(-14.507996, 17.01523));
    builder.line_to(point(-12.107843, 7.307539));
    builder.close();

    test_path(builder.build().as_slice(), None);
}

#[test]
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
    builder.move_to(point(0.0, 1.0));
    builder.line_to(point(10.0, 1.0));
    builder.line_to(point(10.0, 11.0));
    builder.line_to(point(0.0, 11.0));

    builder.move_to(point(10.0, 0.0));
    builder.line_to(point(20.0, 0.0));
    builder.line_to(point(20.0, 10.0));
    builder.line_to(point(10.0, 10.0));

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
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(0.0, 100.0));
    builder.line_to(point(50.0, 100.0));
    builder.line_to(point(0.0, 50.0));
    builder.line_to(point(-50.0, 100.0));
    builder.close();

    let path = builder.build();

    tessellate_path(path.as_slice(), true).unwrap();
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
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(0.0, 100.0));
    builder.line_to(point(-50.0, 100.0));
    builder.line_to(point(0.0, 50.0));
    builder.line_to(point(50.0, 100.0));
    builder.close();

    let path = builder.build();

    tessellate_path(path.as_slice(), true).unwrap();
}

#[test]
fn test_point_on_edge2() {
    // Point b (from edges a-b and b-c) is positionned exactly on
    // the edge d-e.
    //
    //     d-----+
    //     |     |
    //  a--b--c  |
    //  |  |  |  |
    //  +-----+  |
    //     |     |
    //     e-----+
    let mut builder = Path::builder();

    builder.move_to(point(1.0, 1.0));
    builder.line_to(point(2.0, 1.0));
    builder.line_to(point(3.0, 1.0));
    builder.line_to(point(3.0, 2.0));
    builder.line_to(point(1.0, 2.0));
    builder.close();

    builder.move_to(point(2.0, 0.0));
    builder.line_to(point(2.0, 3.0));
    builder.line_to(point(4.0, 3.0));
    builder.line_to(point(4.0, 0.0));
    builder.close();

    test_path(builder.build().as_slice(), None);
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
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 1.0)); // <--
    builder.line_to(point(0.0, 2.0));
    builder.line_to(point(2.0, 2.0));
    builder.line_to(point(1.0, 1.0)); // <--
    builder.line_to(point(2.0, 0.0));
    builder.close();

    let path = builder.build();

    tessellate_path(path.as_slice(), true).unwrap();
}

#[test]
fn test_coincident_simple_2() {
    // A self-intersecting path with two points at the same position.
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 1.0)); // <--
    builder.line_to(point(2.0, 0.0));
    builder.line_to(point(2.0, 2.0));
    builder.line_to(point(1.0, 1.0)); // <--
    builder.line_to(point(0.0, 2.0));
    builder.close();

    let path = builder.build();

    tessellate_path(path.as_slice(), true).unwrap();
}

#[test]
fn test_coincident_simple_rotated() {
    // Same as test_coincident_simple with the usual rotations
    // applied.
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 1.0)); // <--
    builder.line_to(point(0.0, 2.0));
    builder.line_to(point(2.0, 2.0));
    builder.line_to(point(1.0, 1.0)); // <--
    builder.line_to(point(2.0, 0.0));
    builder.close();

    let path = builder.build();

    test_path_with_rotations(path, 0.01, None);
}

#[test]
fn test_identical_squares() {
    // Two identical sub paths. It is pretty much the worst type of input for
    // the tessellator as far as I know.
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(0.0, 1.0));
    builder.close();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(0.0, 1.0));
    builder.close();

    let path = builder.build();

    tessellate_path(path.as_slice(), true).unwrap();
}

#[test]
fn test_close_at_first_position() {
    // This path closes at the first position which requires some special handling in the event
    // builder in order to properly add the last vertex events (since first == current, we can't
    // test against the angle of (current, first, second)).
    let mut builder = Path::builder();

    builder.move_to(point(107.400665, 91.79798));
    builder.line_to(point(108.93136, 91.51076));
    builder.line_to(point(107.84248, 91.79686));
    builder.line_to(point(107.400665, 91.79798));
    builder.close();

    test_path(builder.build().as_slice(), None);
}

#[test]
fn test_fixed_to_f32_precision() {
    // This test appears to hit a precision issue in the conversion from fixed 16.16
    // to f32, causing a point to appear slightly above another when it should not.
    let mut builder = Path::builder();

    builder.move_to(point(68.97998, 796.05));
    builder.line_to(point(61.27998, 805.35));
    builder.line_to(point(55.37999, 799.14996));
    builder.line_to(point(68.98, 796.05));
    builder.close();

    test_path(builder.build().as_slice(), None);
}

#[test]
fn test_no_close() {
    let mut builder = Path::builder();

    builder.move_to(point(1.0, 1.0));
    builder.line_to(point(5.0, 1.0));
    builder.line_to(point(1.0, 5.0));

    test_path(builder.build().as_slice(), None);
}

#[test]
fn test_empty_path() {
    test_path(Path::new().as_slice(), Some(0));
}
