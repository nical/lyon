//! # Path fill tessellator
//!
//! Tessellation routines for complex path fill operations.
//!
//! <svg version="1.1" viewBox="0 0 400 200" height="200" width="400">
//!   <g transform="translate(0,-852.36216)">
//!     <path style="fill:#aad400;stroke:none;" transform="translate(0,852.36216)" d="M 20 20 L 20 180 L 180.30273 180 L 180.30273 20 L 20 20 z M 100 55 L 145 145 L 55 145 L 100 55 z "/>
//!     <path style="fill:#aad400;fill-rule:evenodd;stroke:#000000;stroke-width:1px;stroke-linecap:butt;stroke-linejoin:miter;stroke-" d="m 219.75767,872.36216 0,160.00004 160.30273,0 0,-160.00004 -160.30273,0 z m 80,35 45,90 -90,0 45,-90 z"/>
//!     <path style="fill:none;stroke:#000000;stroke-linecap:round;stroke-linejoin:round;stroke-" d="m 220,1032.3622 35,-35.00004 125,35.00004 -35,-35.00004 35,-125 -80,35 -80,-35 35,125"/>
//!     <circle r="5" cy="872.36218" cx="20" style="color:#000000;;fill:#ff6600;fill-;stroke:#000000;" />
//!     <circle r="5" cx="180.10918" cy="872.61475" style="fill:#ff6600;stroke:#000000;"/>
//!     <circle r="5" cy="1032.2189" cx="180.10918" style="fill:#ff6600;stroke:#000000;"/>
//!     <circle r="5" cx="20.505075" cy="1032.4714" style="fill:#ff6600;stroke:#000000;"/>
//!     <circle r="5" cy="907.21252" cx="99.802048" style="fill:#ff6600;stroke:#000000;"/>
//!     <circle r="5" cx="55.102798" cy="997.36865" style="fill:#ff6600;stroke:#000000;"/>
//!     <circle r="5" cy="997.62122" cx="145.25891" style="fill:#ff6600;stroke:#000000;"/>
//!   </g>
//! </svg>
//!
//! ## Overview
//!
//! The most important structure is [FillTessellator](struct.FillTessellator.html).
//! It implements the path fill tessellation algorithm which is by far the most advanced
//! feature in all lyon crates.
//!
//! The FillTessellator takes a [FillEvents](struct.FillEvents.html) object and
//! [FillOptions](struct.FillOptions.html) as input. The former is an intermediate representaion
//! of the path, containing all edges sorted from top to bottom. FillOption contains
//! some extra parameters (Some of which are not implemented yet).
//!
//! The output of the tessellator is produced by the
//! [BezierGeometryBuilder](../geometry_builder/trait.BezierGeometryBuilder.html) (see the
//! [geometry_builder documentation](../geometry_builder/index.html) for more details about
//! how tessellators produce their output geometry, and how to generate custom vertex layouts).
//!
//! The [tessellator's wiki page](https://github.com/nical/lyon/wiki/Tessellator) is a good place
//! to learn more about how the tessellator's algorithm works. The source code also contains
//! inline documentation for the adventurous who want to delve into more details.
//!
//! # Examples
//!
//! ```
//! # extern crate lyon_tessellation;
//! # extern crate lyon_core;
//! # extern crate lyon_path;
//! # extern crate lyon_path_builder;
//! # extern crate lyon_path_iterator;
//! # use lyon_path::Path;
//! # use lyon_path_builder::*;
//! # use lyon_path_iterator::*;
//! # use lyon_core::math::{Point, point};
//! # use lyon_tessellation::geometry_builder::{VertexBuffers, simple_builder};
//! # use lyon_tessellation::path_fill::*;
//! # fn main() {
//! // Create a simple path.
//! let mut path_builder = Path::builder();
//! path_builder.move_to(point(0.0, 0.0));
//! path_builder.line_to(point(1.0, 2.0));
//! path_builder.line_to(point(2.0, 0.0));
//! path_builder.line_to(point(1.0, 1.0));
//! path_builder.close();
//! let path = path_builder.build();
//!
//! // Create the destination vertex and index buffers.
//! let mut buffers: VertexBuffers<Point> = VertexBuffers::new();
//!
//! {
//!     // Create the destination vertex and index buffers.
//!     let mut vertex_builder = simple_builder(&mut buffers);
//!
//!     // Create the tessellator.
//!     let mut tessellator = FillTessellator::new();
//!
//!     // Allocate the FillEvents object and initialize it from a path iterator.
//!     let events = FillEvents::from_iter(path.path_iter().flattened(0.05));
//!
//!     // Compute the tessellation.
//!     let result = tessellator.tessellate_events(
//!         &events,
//!         &FillOptions::default(),
//!         &mut vertex_builder
//!     );
//!     assert!(result.is_ok());
//! }
//!
//! println!("The generated vertices are: {:?}.", &buffers.vertices[..]);
//! println!("The generated indices are: {:?}.", &buffers.indices[..]);
//!
//! # }
//! ```

// TODO[optim]
//
// # Segment intersection
//
// segment-segment intersection is currently the most perf-sensituve function by far.
// A quick experiment replacing segment_intersection by a dummy function that always
// return None made the tessellation of the log twice faster.
// segment_intersection can be improved (it is currently a nive implementation).
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

use std::f32::consts::PI;
use std::mem::swap;
use std::cmp::{ PartialOrd, Ordering };
use std::cmp;

use math::*;
use geometry_builder::{ BezierGeometryBuilder, Count, VertexId };
use core::{ FlattenedEvent };
use math_utils::{ directed_angle, directed_angle2 };

#[cfg(test)]
use geometry_builder::{ VertexBuffers, simple_builder };
#[cfg(test)]
use path::{ Path, PathSlice };
#[cfg(test)]
use path_iterator::PathIterator;
#[cfg(test)]
use path_builder::BaseBuilder;
#[cfg(test)]
use extra::rust_logo::build_logo_path;

#[cfg(debug)]
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

#[cfg(not(debug))]
macro_rules! tess_log {
    ($obj:ident, $fmt:expr) => ();
    ($obj:ident, $fmt:expr, $($arg:tt)*) => ();
}

/// The fill tessellator's result type.
pub type FillResult = Result<Count, FillError>;

/// The fill tessellator's error enumeration.
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
    pub _handle_intersections: bool,
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
            _handle_intersections: true,
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
        self.sweep_line.clear();
        self.monotone_tessellators.clear();
        self.below.clear();
    }

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

            // We look for the next event by pulling from three sources: the list edges,
            // the list of vertices that don't have a edges immediately under them (end
            // or merge events), and the list of intersections that we find along the way.

            // Look at the sorted list of edges.
            while let Some(edge) = next_edge {
                if edge.upper == current_position {
                    next_edge = edge_iter.next();
                    if edge.lower != current_position {
                        self.below.push(EdgeBelow{
                            lower: edge.lower,
                            angle: compute_angle(edge.lower - edge.upper)
                        });
                    }
                    pending_events = true;
                    tess_log!(self, " edge at {:?} -> {:?}", edge.upper, edge.lower);
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
                        self.below.push(EdgeBelow {
                            lower: inter.lower,
                            angle: compute_angle(inter.lower - current_position),
                        });
                    }

                    pending_events = true;
                    continue
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
                    self.intersections.sort_by(|a, b| { compare_positions(a.upper, b.upper) });

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

    fn process_vertex<Output: BezierGeometryBuilder<Point>>(&mut self, current_position: TessPoint, output: &mut Output) {
        // This is where the interesting things happen.
        // We go through the sweep line to find all of the edges that end at the current
        // position, and through the list of edges that start at the current position
        // (self.below, which we build in tessellator_loop).
        // we decide what to do depending on the spacial configuration of these edges.
        //
        // The logic here really need to be simplified, it is the trickiest part of the
        // tessellator.

        let vec2_position = to_vec2(current_position);
        let id = output.add_vertex(vec2_position);

        // Walk the sweep line to determine where we are with respect to the
        // existing spans.
        let mut start_span = 0;


        // Go through the sweep line to find the first edge that ends at the current
        // position (if any) or if the current position is inside or outside the shape.
        #[derive(Copy, Clone, Debug, PartialEq)]
        enum E { In, Out, LeftEdge, RightEdge };
        let mut status = E::Out;

        for span in &mut self.sweep_line {
            if !span.left.merge && span.left.lower == current_position {
                status = E::LeftEdge;
                break;
            }

            if test_span_touches(&span.left, current_position) {
                // The current point is on an edge we need to split the edge into the part
                // above and the part below. See test_point_on_edge_left for an example of
                // geometry that can lead to this scenario.

                // Split the edge.
                self.below.push(EdgeBelow {
                    lower: span.left.lower,
                    angle: compute_angle(span.left.lower - current_position)
                });
                span.left.lower = current_position;

                status = E::LeftEdge;
                break;
            }

            if test_span_side(&span.left, current_position) {
                status = E::Out;
                break;
            }

            if !span.right.merge && span.right.lower == current_position {
                // TODO: this can lead to incorrect results if the next span also touches.
                status = E::RightEdge;
                break;
            }

            if test_span_touches(&span.right, current_position) {
                // Same situation as above, the current point is on an edge (but this
                // time it is the right side instead of the left side of a span).

                // Split the edge.
                self.below.push(EdgeBelow {
                    lower: span.right.lower,
                    angle: compute_angle(span.right.lower - current_position)
                });
                span.right.lower = current_position;

                status = E::RightEdge;
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
            self.log_sl(current_position, start_span);
        }

        // The index of the next edge below the current vertex, to be processed.
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
                    tess_log!(self, "(right event) {}", start_span);

                    let edge_to = self.below[0].lower;
                    self.insert_edge(start_span, Side::Right, current_position, edge_to, id);

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
            tess_log!(self, "(end event) {}", span_idx);

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
            tess_log!(self, "(merge event) {}", start_span);

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
            tess_log!(self, "(left event) {}    -> {:?}", span_idx, vertex_below);
            self.insert_edge(span_idx, Side::Left, current_position, vertex_below, id);

            below_count -= 1;
        }

        // Since we took care of left and right events already we should not have
        // an odd number of edges to work with below the current vertex by now.
        debug_assert!(below_count % 2 == 0);

        // Reset span_idx for the next pass.
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
                tess_log!(self, "(split event) {}", start_span);

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
                tess_log!(self, "(start event) {}", span_idx);

                let l = self.below[below_idx].lower;
                let r = self.below[below_idx + 1].lower;
                let mut left_edge = Edge { upper: current_position, lower: l };
                let mut right_edge = Edge { upper: current_position, lower: r };

                // Look whether the two edges are colinear:
                if self.below[below_idx].angle != self.below[below_idx+1].angle {
                    // In most cases (not colinear):
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
                    } else if is_after(l, r) {
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
            self.insert_edge(left_span, Side::Right, current, left.lower, id);
            self.insert_edge(right_span, Side::Left, current, right.lower, id);

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

            self.insert_edge(span_idx, Side::Right, current, left.lower, id);
            self.insert_edge(span_idx+1, Side::Left, current, right.lower, id);
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
        upper: TessPoint, lower: TessPoint, id: VertexId,
    ) {
        debug_assert!(!is_after(upper, lower));
        // TODO horrible hack: set the merge flag on the edge we are about to replace temporarily
        // so that it doesn not get in the way of the intersection detection.
        let mut edge = Edge { upper: upper, lower: lower };
        self.sweep_line[span_idx].mut_edge(side).merge = true;
        self.check_intersections(&mut edge);
        // This sets the merge flag to false.
        self.sweep_line[span_idx].edge(edge, id, side);
        let vec2_position = to_vec2(edge.upper);
        self.monotone_tessellators[span_idx].vertex(vec2_position, id, side);

    }

    fn check_intersections(&mut self, edge: &mut Edge) {
        // Test and for intersections against the edges in the sweep line.
        // If an intersecton is found, the edge is split and retains only the part
        // above the intersection point. The lower part is kept with the intersection
        // to be processed later when the sweep line reaches it.
        // If there are several intersections we only keep the one that is closest to
        // the sweep line.
        //
        // TODO: This function is more complicated (and slower) than it needs to be.

        if !self._handle_intersections {
            return;
        }

        struct Intersection { point: TessPoint, lower1: TessPoint, lower2: Option<TessPoint> }

        let original_edge = *edge;
        let mut intersection = None;
        let mut span_idx = 0;

        for span in &mut self.sweep_line {

            // Test for an intersection against the span's left edge.
            if !span.left.merge {
                match segment_intersection(
                    edge.upper, edge.lower,
                    span.left.upper, span.left.lower,
                ) {
                    SegmentInteresection::One(position) => {
                        tess_log!(self, " -- found an intersection at {:?}
                                        |    {:?}->{:?} x {:?}->{:?}",
                            position,
                            original_edge.upper, original_edge.lower,
                            span.left.upper, span.left.lower,
                        );

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
                    SegmentInteresection::Two(_p1, p2) => {
                        tess_log!(self, " -- found two intersections {:?} and {:?}", _p1, p2);

                        intersection = Some((
                            Intersection {
                                point: p2,
                                lower1: if is_after(original_edge.lower, span.left.lower)
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
                match segment_intersection(
                    edge.upper, edge.lower,
                    span.right.upper, span.right.lower,
                ) {
                    SegmentInteresection::One(position) => {
                        tess_log!(self, " -- found an intersection at {:?}
                                        |    {:?}->{:?} x {:?}->{:?}",
                            position,
                            original_edge.upper, original_edge.lower,
                            span.right.upper, span.right.lower,
                        );
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
                    SegmentInteresection::Two(_p1, p2) => {
                        tess_log!(self, " -- found two intersections {:?} and {:?}", _p1, p2);

                        intersection = Some((
                            Intersection {
                                point: p2,
                                lower1: if is_after(original_edge.lower, span.right.lower)
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
            if !is_after(evt.point, current_position) {
                evt.point.y = current_position.y + FixedPoint32::epsilon();
                edge.lower = evt.point;
            }

            let mut e1 = Edge { upper: evt.point, lower: evt.lower1 };
            if is_after(e1.upper, e1.lower) { swap(&mut e1.upper, &mut e1.lower); }

            let e2 = if let Some(lower2) = evt.lower2 {
                let mut e2 = Edge { upper: evt.point, lower: lower2 };
                // Same deal with the precision issues here. In this case we can just flip the new
                // edge so that its upper member is indeed above the lower one.
                if is_after(e2.upper, e2.lower) { swap(&mut e2.upper, &mut e2.lower); }
                Some(e2)
            } else { None };

            tess_log!(self, " set span[{:?}].{:?}.lower = {:?} (was {:?}",
                span_idx, side, evt.point,
                self.sweep_line[span_idx].mut_edge(side).lower
            );

            self.sweep_line[span_idx].mut_edge(side).lower = evt.point;
            self.intersections.push(e1);
            if let Some(e2) = e2 {
                self.intersections.push(e2);
            }

            // We sill sort the intersection vector lazily.
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
        tess_log!(self, " !! FillTessellator Error {:?}", err);
        self.error = Some(err);
    }

    fn debug_check_sl(&self, current: TessPoint) {
        for span in &self.sweep_line {
            if !span.left.merge {
                debug_assert!(!is_after(current, span.left.lower),
                    "current {:?} should not be below lower left{:?}",
                    current, span.left.lower
                );
                debug_assert!(!is_after(span.left.upper, span.left.lower),
                    "upper left {:?} should not be below lower left {:?}",
                    span.left.upper, span.left.lower
                );
            }
            if !span.right.merge {
                debug_assert!(!is_after(current, span.right.lower));
                debug_assert!(!is_after(span.right.upper, span.right.lower));
            }
        }
    }

    fn log_sl(&self, current_position: TessPoint, start_span: usize) {
        println!("\n\n\n\n");
        self.log_sl_ids();
        self.log_sl_points_at(current_position.y);
        println!("\n ----- current: {:?} ------ offset {:?} in sl", current_position, start_span);
        for b in &self.below {
            println!("   -- below: {:?}", b);
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
            print!("| l:{:?} ", span.left.upper);
            print!(" r:{:?} |", span.right.upper);
        }
        println!("]");
        print!("     [");
        for span in &self.sweep_line {
            if span.left.merge {
                print!("| l:   <merge>           ");
            } else {
                print!("| l:{:?} ", span.left.lower);
            }
            if span.right.merge {
                print!(" r:   <merge>           |");
            } else {
                print!(" r:{:?} |", span.right.lower);
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

fn line_horizontal_intersection_fixed(
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

    let tmp: FixedPoint64 = (y - a.y).to_fp64();
    return Some(a.x + tmp.mul_div(vx.to_fp64(), vy.to_fp64()).to_fp32());
}

#[derive(PartialEq, Debug)]
enum SegmentInteresection {
    One(TessPoint),
    Two(TessPoint, TessPoint),
    None,
}

// TODO[optim]: This function shows up pretty high in the profiles.
fn segment_intersection(
    a1: TessPoint, b1: TessPoint, // The new edge.
    a2: TessPoint, b2: TessPoint  // An already inserted edge.
) -> SegmentInteresection {

    fn tess_point(x: f64, y: f64) -> TessPoint {
        TessPoint::new(FixedPoint32::from_f64(x), FixedPoint32::from_f64(y))
    }

    //println!(" -- test intersection {:?} {:?} x {:?} {:?}", a1, b1, a2, b2);

    // TODO: moving this down after the v1_cross_v2 == 0.0 branch fixes test
    // test_colinear_touching_squares2_failing
    if a1 == b2 || a1 == a2 || b1 == a2 || b1 == b2 {
        return SegmentInteresection::None;
    }

    let a1 = F64Point::new(a1.x.to_f64(), a1.y.to_f64());
    let b1 = F64Point::new(b1.x.to_f64(), b1.y.to_f64());
    let a2 = F64Point::new(a2.x.to_f64(), a2.y.to_f64());
    let b2 = F64Point::new(b2.x.to_f64(), b2.y.to_f64());

    let v1 = b1 - a1;
    let v2 = b2 - a2;

    debug_assert!(v2.x != 0.0 || v2.y != 0.0, "zero-length edge");

    let v1_cross_v2 = v1.cross(v2);
    let a2_a1_cross_v1 = (a2 - a1).cross(v1);

    if v1_cross_v2 == 0.0 {
        return SegmentInteresection::None;
    }

/*
    // TODO: we almost never take this branch.
    // Note: Skia's tessellator doesn't 
    if v1_cross_v2 == 0.0 {
        if a2_a1_cross_v1 != 0.0 {
            return SegmentInteresection::None;
        }
        // The two segments are colinear.

        let v1_sqr_len = v1.x * v1.x + v1.y * v1.y;
        let v2_sqr_len = v2.x * v2.x + v2.y * v2.y;

        // We know that a1 cannot be above a2 so if b1 is between a2 and b2, we have
        // the order a2 -> a1 -> b1 -> b2.
        let v2_dot_b1a2 = v2.dot(b1 - a2);
        if v2_dot_b1a2 > 0.0 && v2_dot_b1a2 < v2_sqr_len {
            //println!(" -- colinear intersection");
            return SegmentInteresection::Two(
                tess_point(a1.x, a1.y),
                tess_point(b1.x, b1.y),
            );
        }

        // We know that a1 cannot be above a2 and if b1 is below b2, so if
        // b2 is between a1 and b1, then we have the order a2 -> a1 -> b2 -> b1.
        let v1_dot_b2a1 = v1.dot(b2 - a1);
        if v1_dot_b2a1 > 0.0 && v1_dot_b2a1 < v1_sqr_len {
            //println!(" -- colinear intersection");
            return SegmentInteresection::Two(
                tess_point(a1.x, a1.y),
                tess_point(b2.x, b2.y),
            );
        }

        return SegmentInteresection::None;
    }
*/

    let sign_v1_cross_v2 = if v1_cross_v2 > 0.0 { 1.0 } else { -1.0 };
    let abs_v1_cross_v2 = v1_cross_v2 * sign_v1_cross_v2;

    // t and u should be divided by v1_cross_v2, but we postpone that to not lose precision.
    // We have to respect the sign of v1_cross_v2 (and therefore t and u) so we apply it now and
    // will use the absolute value of v1_cross_v2 afterwards.
    let t = (a2 - a1).cross(v2) * sign_v1_cross_v2;
    let u = a2_a1_cross_v1 * sign_v1_cross_v2;

    if t > 0.0 && t < abs_v1_cross_v2 && u > 0.0 && u <= abs_v1_cross_v2 {

        let res = a1 + (v1 * t) / abs_v1_cross_v2;
        debug_assert!(res.y <= b1.y && res.y <= b2.y);

        if res != a1 && res != b1 && res != a2 && res != b2 {
            return SegmentInteresection::One(tess_point(res.x, res.y));
        }
    }

    return SegmentInteresection::None;
}

// Returns true if the position is on the right side of the edge.
fn test_span_side(span_edge: &SpanEdge, position: TessPoint) -> bool {
    if span_edge.merge {
        return false;
    }

    // TODO: do we need this?
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

    fn edge(&mut self, edge: Edge, id: VertexId, side: Side) {
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


/// Defines an ordering between two points
///
/// A point is considered after another point if it is below (y pointing downward) the point.
/// If two points have the same y coordinate, the one on the right (x pointing to the right)
/// is the one after.
pub fn is_after<T: PartialOrd, U>(a: TypedPoint2D<T, U>, b: TypedPoint2D<T, U>) -> bool {
    a.y > b.y || (a.y == b.y && a.x > b.x)
}

// translate to and from the internal coordinate system.
fn to_internal(v: Point) -> TessPoint { TessPoint::new(fixed(v.x), fixed(v.y)) }
fn to_vec2(v: TessPoint) -> Point { vec2(v.x.to_f32(), v.y.to_f32()) }

fn compute_angle(v: TessVec2) -> f32 {
    // TODO: compute directed angles using fixed point vectors.
    -directed_angle(vec2(1.0, 0.0), to_vec2(v))
}

/// A sequence of edges sorted from top to bottom, to be used as the tessellator's input.
pub struct FillEvents {
    edges: Vec<Edge>,
    vertices: Vec<TessPoint>,
}

impl FillEvents {
    pub fn from_iter<Iter: Iterator<Item=FlattenedEvent>>(it: Iter) -> Self {
        EventsBuilder::new().build(it)
    }

    pub fn new() -> Self { FillEvents { edges: Vec::new(), vertices: Vec::new() } }

    pub fn clear(&mut self) {
        self.edges.clear();
        self.vertices.clear();
    }

    pub fn set_path_iter<Iter: Iterator<Item=FlattenedEvent>>(&mut self, it: Iter) {
        self.clear();
        let mut tmp = FillEvents::new();
        ::std::mem::swap(self, &mut tmp);
        let mut builder = EventsBuilder::new();
        builder.recycle(tmp);
        let mut tmp = builder.build(it);
        ::std::mem::swap(self, &mut tmp);
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

    fn recycle(&mut self, events: FillEvents) {
        self.edges = events.edges;
        self.vertices = events.vertices;
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

        if is_after(a, b) {
            swap(&mut a, &mut b);
        }

        self.edges.push(Edge { upper: a, lower: b });
    }

    fn vertex(&mut self, previous: TessPoint, current: TessPoint, next: TessPoint) {
        if is_after(current, previous) && is_after(current, next) {
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
enum Side { Left, Right }

impl Side {
    fn opposite(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    fn is_left(self) -> bool { self == Side::Left }

    fn is_right(self) -> bool { self == Side::Right }
}

/// Helper class that generates a triangulation from a sequence of vertices describing a monotone
/// polygon (used internally by the FillTessellator).
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

        debug_assert!(is_after(current.pos, self.previous.pos));
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
    path.line_to(point( 0.0, 6.0));
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
fn test_degenerate_empty() {
    test_path(Path::new().as_slice(), Some(0));
}

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
    let mut path = Path::builder().with_svg();

    build_logo_path(&mut path);

    test_path_with_rotations(path.build(), 0.011, None);
}

#[test]
fn test_rust_logo_with_intersection() {
    let mut path = Path::builder().with_svg();

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

    scale_path(&mut path, 0.005);
    test_path(path.as_slice(), None);
}

#[test]
fn test_rust_logo_scale_down2() {
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

    tessellate(path.as_slice(), true).unwrap();
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

    tessellate(path.as_slice(), true).unwrap();
}

#[test]
#[ignore]
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
    builder.move_to(point(0.0,  0.0));
    builder.line_to(point(10.0, 0.0));
    builder.line_to(point(10.0, 10.0));
    builder.line_to(point(0.0,  10.0));

    builder.move_to(point(10.0, 1.0));
    builder.line_to(point(20.0, 1.0));
    builder.line_to(point(20.0, 11.0));
    builder.line_to(point(10.0, 11.0));

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
    builder.move_to(point(0.0,  1.0));
    builder.line_to(point(10.0, 1.0));
    builder.line_to(point(10.0, 11.0));
    builder.line_to(point(0.0,  11.0));

    builder.move_to(point(10.0, 0.0));
    builder.line_to(point(20.0, 0.0));
    builder.line_to(point(20.0, 10.0));
    builder.line_to(point(10.0, 10.0));

    builder.close();

    let path = builder.build();

    tessellate(path.as_slice(), true).unwrap();
}


#[test]
#[ignore] // TODO
fn reduced_test_case() {
    let mut builder = Path::builder().flattened(0.05);

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
    builder.move_to(point(0.0,  1.0));
    builder.line_to(point(10.0, 1.0));
    builder.line_to(point(10.0, 11.0));
    builder.line_to(point(0.0,  11.0));

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
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(0.0, 100.0));
    builder.line_to(point(-50.0, 100.0));
    builder.line_to(point(0.0, 50.0));
    builder.line_to(point(50.0, 100.0));
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
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 1.0)); // <--
    builder.line_to(point(0.0, 2.0));
    builder.line_to(point(2.0, 2.0));
    builder.line_to(point(1.0, 1.0)); // <--
    builder.line_to(point(2.0, 0.0));
    builder.close();

    let path = builder.build();

    tessellate(path.as_slice(), true).unwrap();
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

    tessellate(path.as_slice(), true).unwrap();
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

    tessellate(path.as_slice(), true).unwrap();
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
