
use crate::{FillOptions, Side, InternalError, TessellationResult, TessellationError, VertexSource};
use crate::{FillGeometryBuilder, VertexId, Orientation};
use crate::geom::math::*;
use crate::geom::LineSegment;
use crate::event_queue::*;
use crate::monotone::*;
use crate::path::{PathEvent, IdEvent, PathSlice, FillRule, PositionStore, AttributeStore, EndpointId};
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
    is_in: bool,
}

impl WindingState {
    fn new() -> Self {
        // The span index starts at -1 so that entering the first span (of index 0) increments
        // it to zero.
        WindingState {
            span_index: -1,
            number: 0,
            is_in: false,
        }
    }

    fn update(&mut self, fill_rule: FillRule, edge_winding: i16) {
        self.number += edge_winding;
        self.is_in = fill_rule.is_in(self.number);
        if self.is_in {
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
    above: Range<ActiveEdgeIdx>,
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
            above: 0..0,
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
        self.above = 0 .. 0;
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
/// The `FillTessellator` takes a a description of the input path and
/// [`FillOptions`](struct.FillOptions.html) as input. The description of the path can be an
/// `PathEvent` iterator, or an iterator of `IdEvent` with an implementation of`PositionStore`
/// to retrieve positions form endpoint and control point ids, and optionally an `AttributeStore`
/// providing custom endpoint attributes that the tessellator can hand over to the geometry builder.
///
/// The output of the tessellator is produced by the
/// [`FillGeometryBuilder`](geometry_builder/trait.FillGeometryBuilder.html) (see the
/// [`geometry_builder` documentation](geometry_builder/index.html) for more details about
/// how tessellators produce their output geometry, and how to generate custom vertex layouts).
///
/// The [tessellator's wiki page](https://github.com/nical/lyon/wiki/Tessellator) is a good place
/// to learn more about how the tessellator's algorithm works. The source code also contains
/// inline documentation for the adventurous who want to delve into more details.
///
/// ## Associating custom attributes with vertices.
///
/// It is sometimes useful to be able to link vertices generated by the tessellator back
/// with the path's original data, for example to be able to add attributes that the tessellator
/// does not know about (vertex color, texture coordinates, etc.).
///
/// The fill tessellator has two mechanisms to help with these advanced use cases. One is
/// simple to use and one that, while more complicated to use, can cover advanced scenarios.
///
/// Before going delving into these mechanisms, it is important to understand that the
/// vertices generated by the tessellator don't always correspond to the vertices existing
/// in the original path.
/// - Self-intersections, for example, introduce a new vertex where two edges meet.
/// - When several vertices are at the same position, they are merged into a single vertex
///   from the point of view of the tessellator.
/// - The tessellator does not handle curves, and uses an approximation that introduces a
///   number of line segments and therefeore endpoints between the original endpoints of any
///   quadratic or cubic bézier curve.
///
/// This complicates the task of adding extra data to vertices without loosing the association
/// during tessellation.
///
/// ### Vertex sources
///
/// This is the complicated, but most powerful mechanism. The tessellator keeps track of where
/// each vertex comes from in the original path, and provides access to this information via
/// an iterator of [`VertexSource`](enum.VertexSource.html) in `FillAttributes::sources`.
///
/// It is most common for the vertex source iterator to yield a single `VertexSource::Endpoint`
/// source, which happens when the vertex directly corresponds to an endpoint of the original path.
/// More complicated cases can be expressed.
/// For example if a vertex is inserted at an intersection halfway in the edge AB and two thirds
/// of the way through edge BC, the source for this new vertex is `VertexSource::Edge { from: A, to: B, t: 0.5 }`
/// and VertexSource::Edge { from: C, to: D, t: 0.666666 }` where A, B, C and D are endpoint IDs.
///
/// To use this feature, make sure to use `FillTessellator::tessellate_with_ids` instead of
/// `FillTessellator::tessellate`.
///
/// ### Interpolated float attributes
///
/// Having to iterate over potentially several sources for each vertex can be cumbersome, in addition
/// to having to deal with generating proper values for the attributes of vertices that were introduced
/// at intersections or along curves.
///
/// In many scenarios, vertex attributes are made of floating point numbers and the most reasonable
/// way to generate new attributes is to linearly interpolate these numbers between the endpoints
/// of the edges they originate from.
///
/// Custom endpoint attributes are represented as `&[f32]` slices accessible via
/// `FillAttributes::interpolated_attributes`. All vertices, whether they originate from a single
/// endpoint or some more complex source, have exactly the same number of attributes.
/// Without having to know about the meaning of attributes, the tessellator can either
/// forward the slice of attributes from a provided `AttributeStore` when possible or
/// generate the values via linear interpolation.
///
/// To use this feature, make sure to use `FillTessellator::tessellate_path` or
/// `FillTessellator::tessellate_with_ids` instead of `FillTessellator::tessellate`.
///
/// Attributes are lazily computed when calling `FillAttributes::interpolated_attributes`.
/// In other words they don't add overhead when not used, however it is best to avoid calling
/// interpolated_attributes several times per vertex.
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
/// let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
///
/// {
///     let mut vertex_builder = simple_builder(&mut buffers);
///
///     // Create the tessellator.
///     let mut tessellator = FillTessellator::new();
///
///     // Compute the tessellation.
///     let result = tessellator.tessellate_path(
///         &path,
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
/// ```
/// # extern crate lyon_tessellation as tess;
/// # use tess::path::Path;
/// # use tess::path::builder::*;
/// # use tess::path::iterator::*;
/// # use tess::geom::math::{Point, point};
/// # use tess::geometry_builder::{VertexBuffers, simple_builder};
/// # use tess::*;
/// # fn main() {
/// // Create a path with three custom endpoint attributes.
/// let mut path_builder = Path::builder_with_attributes(3);
/// path_builder.move_to(point(0.0, 0.0), &[0.0, 0.1, 0.5]);
/// path_builder.line_to(point(1.0, 2.0), &[1.0, 1.0, 0.1]);
/// path_builder.line_to(point(2.0, 0.0), &[1.0, 0.0, 0.8]);
/// path_builder.line_to(point(1.0, 1.0), &[0.1, 0.3, 0.5]);
/// path_builder.close();
/// let path = path_builder.build();
///
/// struct MyVertex {
///     x: f32, y: f32,
///     r: f32, g: f32, b: f32, a: f32,
/// }
/// // A custom vertex constructor, see the geometry_builder module.
/// struct Ctor;
/// impl FillVertexConstructor<MyVertex> for Ctor {
///     fn new_vertex(&mut self, position: Point, mut attributes: FillAttributes) -> MyVertex {
///         let attrs = attributes.interpolated_attributes();
///         MyVertex {
///             x: position.x,
///             y: position.y,
///             r: attrs[0],
///             g: attrs[1],
///             b: attrs[2],
///             a: 1.0,
///         }
///     }
/// }
///
/// // Create the destination vertex and index buffers.
/// let mut buffers: VertexBuffers<MyVertex, u16> = VertexBuffers::new();
///
/// {
///     // We use our custom vertex constructor here.
///     let mut vertex_builder = BuffersBuilder::new(&mut buffers, Ctor);
///
///     // Create the tessellator.
///     let mut tessellator = FillTessellator::new();
///
///     // Compute the tessellation. Here we use tessellate_with_ids
///     // which has a slightly more complicated interface. The provides
///     // the iterator as well as storage for positions and attributes at
///     // the same time.
///     let result = tessellator.tessellate_with_ids(
///         path.id_iter(), // Iterator over ids in the path
///         &path,          // PositionStore
///         Some(&path),    // AttributeStore
///         &FillOptions::default(),
///         &mut vertex_builder
///     );
///     assert!(result.is_ok());
/// }
///
/// # }
/// ```
pub struct FillTessellator {
    current_position: Point,
    current_vertex: VertexId,
    current_event_id: TessEventId,
    active: ActiveEdges,
    edges_below: Vec<PendingEdge>,
    fill_rule: FillRule,
    orientation: Orientation,
    fill: Spans,
    log: bool,
    assume_no_intersection: bool,
    attrib_buffer: Vec<f32>,

    events: EventQueue,
}


impl FillTessellator {
    /// Constructor.
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
            orientation: Orientation::Vertical,
            fill: Spans {
                spans: Vec::new(),
            },
            log,
            assume_no_intersection: false,
            attrib_buffer: Vec::new(),

            events: EventQueue::new(),
        }
    }

    #[doc(hidden)]
    /// Create and EventQueue.
    pub fn create_event_queue(&mut self) -> EventQueue {
        std::mem::replace(&mut self.events, EventQueue::new())
    }

    /// Compute the tessellation from a path iterator.
    pub fn tessellate(
        &mut self,
        path: impl IntoIterator<Item = PathEvent>,
        options: &FillOptions,
        output: &mut dyn FillGeometryBuilder,
    ) -> TessellationResult {

        let mut queue_builder = self.create_event_queue().into_builder();

        queue_builder.set_path(options.tolerance, options.sweep_orientation, path.into_iter());

        let mut event_queue = queue_builder.build();

        std::mem::swap(&mut self.events, &mut event_queue);

        self.tessellate_impl(options, None, output)
    }

    /// Compute the tessellation using an iterator over endpoint and control
    /// point ids, storage for the positions and, optionally, storage for
    /// custom endpoint attributes.
    pub fn tessellate_with_ids(
        &mut self,
        path: impl IntoIterator<Item = IdEvent>,
        positions: &impl PositionStore,
        custom_attributes: Option<&dyn AttributeStore>,
        options: &FillOptions,
        output: &mut dyn FillGeometryBuilder,
    ) -> TessellationResult {

        let mut queue_builder = self.create_event_queue().into_builder();

        queue_builder.set_path_with_ids(options.tolerance, options.sweep_orientation, path.into_iter(), positions);

        let mut event_queue = queue_builder.build();

        std::mem::swap(&mut self.events, &mut event_queue);

        self.tessellate_impl(options, custom_attributes, output)
    }

    #[doc(hidden)]
    /// Compute the tessellation from a pre-built event queue.
    pub fn tessellate_events(
        &mut self,
        events: &mut EventQueue,
        custom_attributes: Option<&dyn AttributeStore>,
        options: &FillOptions,
        builder: &mut dyn FillGeometryBuilder
    ) -> TessellationResult {

        std::mem::swap(&mut self.events, events);

        let result = self.tessellate_impl(options, custom_attributes, builder);

        std::mem::swap(&mut self.events, events);

        result
    }

    /// Compute the tessellation from a path slice.
    ///
    /// The tessellator will internally only track vertex sources and interpolated
    /// attributes if the path has interpolated attributes.
    pub fn tessellate_path<'l>(
        &'l mut self,
        path: impl Into<PathSlice<'l>>,
        options: &'l FillOptions,
        builder: &'l mut dyn FillGeometryBuilder,
    ) -> TessellationResult {

        let path = path.into();

        if path.num_attributes() > 0 {
            self.tessellate_with_ids(
                path.id_iter(),
                &path,
                Some(&path),
                options,
                builder,
            )
        } else {
            self.tessellate(
                path.iter(),
                options,
                builder,
            )
        }
    }

    fn tessellate_impl(
        &mut self,
        options: &FillOptions,
        attrib_store: Option<&dyn AttributeStore>,
        builder: &mut dyn FillGeometryBuilder
    ) -> TessellationResult {
        self.reset();

        if let Some(store) = attrib_store {
            self.attrib_buffer.resize(store.num_attributes(), 0.0);
        } else {
            self.attrib_buffer.clear();
        }

        self.fill_rule = options.fill_rule;
        self.orientation = options.sweep_orientation;
        self.assume_no_intersection = !options.handle_intersections;

        builder.begin_geometry();

        let result = self.tessellator_loop(attrib_store, builder);

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

    /// Enable/disable some verbose logging during the tessellation, for
    /// debugging purposes.
    pub fn set_logging(&mut self, is_enabled: bool) {
        self.log = is_enabled;
    }

    fn tessellator_loop(
        &mut self,
        attrib_store: Option<&dyn AttributeStore>,
        output: &mut dyn FillGeometryBuilder
    ) -> Result<(), TessellationError> {
        log_svg_preamble(self);

        let mut scan = ActiveEdgeScan::new();
        let mut _prev_position = point(std::f32::MIN, std::f32::MIN);
        self.current_event_id = self.events.first_id();
        while self.events.valid_id(self.current_event_id) {

            self.initialize_events(attrib_store, output)?;

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

    fn initialize_events(
        &mut self,
        attrib_store: Option<&dyn AttributeStore>,
        output: &mut dyn FillGeometryBuilder,
    ) -> Result<(), TessellationError> {
        let current_event = self.current_event_id;

        tess_log!(self, "\n\n<!--         event #{}          -->", current_event);

        self.current_position = self.events.position(current_event);

        let position = match self.orientation {
            Orientation::Vertical => self.current_position,
            Orientation::Horizontal => reorient(self.current_position),
        };

        self.current_vertex = output.add_fill_vertex(
            position,
            FillAttributes {
                events: &self.events,
                current_event,
                attrib_store,
                attrib_buffer: &mut self.attrib_buffer,
            },
        )?;

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

            tess_log!(self, " > span: {}, in: {}", winding.span_index, winding.is_in);
        }

        scan.above.start = active_edge_idx;
        scan.winding_before_point = winding.clone();

        if previous_was_merge {
            scan.winding_before_point.span_index -= 1;
            scan.above.start -= 1;

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
            && winding.is_in
            && !scan.merge_split_event;

        // Step 2 - Iterate over edges connecting with the current point.

        tess_log!(self, "connecting_edges {} | edge {} | span {}", connecting_edges, active_edge_idx, winding.span_index);
        if connecting_edges {

            let in_before_vertex = winding.is_in;
            let mut first_connecting_edge = !previous_was_merge;

            for active_edge in &self.active.edges[active_edge_idx..] {
                if active_edge.is_merge {
                    if !winding.is_in {
                        return Err(InternalError::MergeVertexOutside);
                    }

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

                    winding.span_index += 1;
                    active_edge_idx += 1;
                    first_connecting_edge = false;

                    continue;
                }

                if !self.is_edge_connecting(active_edge, active_edge_idx, scan)? {
                    break;
                }

                if !first_connecting_edge && winding.is_in {
                    // End event.
                    //
                    //  \.../
                    //   \./
                    //    x
                    //
                    scan.spans_to_end.push(winding.span_index);
                }

                winding.update(self.fill_rule, active_edge.winding);

                tess_log!(self, " x span: {} in: {}", winding.span_index, winding.is_in);

                if winding.is_in && winding.span_index >= self.fill.spans.len() as i32 {
                    return Err(InternalError::InsufficientNumberOfSpans);
                }

                active_edge_idx += 1;
                first_connecting_edge = false;
            }

            let in_after_vertex = winding.is_in;

            let vertex_is_merge_event = in_before_vertex && in_after_vertex
                && self.edges_below.is_empty()
                && scan.edges_to_split.is_empty();

            if vertex_is_merge_event {
                //  .\   /.      .\ |./ /.
                //  ..\ /..      ..\|//...
                //  ...x...  or  ...x.....  (etc.)
                //  .......      .........
                scan.merge_event = true;
            }

            if in_before_vertex {
                //   ...|         ..\ /..
                //   ...x    or   ...x...  (etc.)
                //   ...|         ...:...
                let first_span_index = scan.winding_before_point.span_index;
                scan.vertex_events.push((first_span_index, Side::Right));
            }

            if in_after_vertex {
                //    |...        ..\ /..
                //    x...   or   ...x...  (etc.)
                //    |...        ...:...
                scan.vertex_events.push((winding.span_index, Side::Left));
            }
        }

        tess_log!(self, "edges after | {}", active_edge_idx);

        scan.above.end = active_edge_idx;


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
            tess_log!(self, "   -> Vertex event, span: {:?} / {:?} / id: {:?}", span_index, side, self.current_vertex);
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
            let edge = &mut self.active.edges[scan.above.start];
            edge.is_merge = true;
            edge.from = edge.to;
            edge.min_x = edge.to.x;
            edge.max_x = edge.to.x;
            edge.winding = 0;
            edge.from_id = self.current_vertex;

            // take the merge edge out of the range so that it isn't removed later.
            scan.above.start += 1;
        }
    }

    fn process_edges_below(&mut self, scan: &mut ActiveEdgeScan) {
        let mut winding = scan.winding_before_point.clone();

        tess_log!(self, "connecting edges: {}..{} in: {:?}", scan.above.start, scan.above.end, winding.is_in);
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

            tess_log!(self, "split event");

            let left_enclosing_edge_idx = scan.above.start - 1;
            self.split_event(
                left_enclosing_edge_idx,
                winding.span_index,
            );
        }

        // Go through the edges that start at the current point and emit
        // start events for each time an in-out pair is found.

        let mut first_pending_edge = true;
        for pending_edge in &self.edges_below {
            if !first_pending_edge && winding.is_in {
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
            winding.update(self.fill_rule, pending_edge.winding);

            tess_log!(self, "edge below: span: {}, in: {}", winding.span_index, winding.is_in);

            first_pending_edge = false;
        }
    }

    fn update_active_edges(&mut self, scan: &ActiveEdgeScan) {
        let above = scan.above.start..scan.above.end;

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

#[inline]
fn reorient(p: Point) -> Point {
    point(p.y, -p.x)
}

/// Extra vertex information from the `FillTessellator`, accessible when building vertices.
pub struct FillAttributes<'l> {
    events: &'l EventQueue,
    current_event: TessEventId,
    attrib_buffer: &'l mut[f32],
    attrib_store: Option<&'l dyn AttributeStore>,
}

impl<'l> FillAttributes<'l> {
    /// Return an iterator over the sources of the vertex.
    pub fn sources(&self) -> VertexSourceIterator {
        VertexSourceIterator {
            events: self.events,
            id: self.current_event,
        }
    }

    /// If the vertex source is a single endpoint id, return its ID, None otherwise.
    pub fn as_endpoint_id(&self) -> Option<EndpointId> {
        let second = self.events.next_sibling_id(self.current_event);
        if !self.events.valid_id(second) {
            let edge = &self.events.edge_data[self.current_event as usize];
            let t = edge.range.start;
            if t == 0.0 {
                return Some(edge.from_id);
            }
            if t == 1.0 {
                return Some(edge.to_id);
            }
        }

        None
    }

    /// Fetch or interpolate the custom attribute values at this vertex.
    pub fn interpolated_attributes(&mut self) -> &[f32] {
        if self.attrib_store.is_none() {
            return &[];
        }

        let store = self.attrib_store.unwrap();

        let second = self.events.next_sibling_id(self.current_event);
        if !self.events.valid_id(second) {
            let edge = &self.events.edge_data[self.current_event as usize];
            let t = edge.range.start;
            if t == 0.0 {
                return store.get(edge.from_id);
            }
            if t == 1.0 {
                return store.get(edge.to_id);
            }
        }

        let num_attributes = store.num_attributes();
        assert!(self.attrib_buffer.len() == num_attributes);

        // First source taken out of the loop to avoid initializing the buffer.
        {
            let edge = &self.events.edge_data[self.current_event as usize];
            let t = edge.range.start;

            let a = store.get(edge.from_id);
            let b = store.get(edge.to_id);

            assert!(a.len() == num_attributes);
            assert!(b.len() == num_attributes);
            for i in 0..num_attributes {
                self.attrib_buffer[i] = a[i] * (1.0 - t) + b[i] * t;
            }
        }

        let mut div = 1.0;
        let mut current_sibling = second;
        while self.events.valid_id(current_sibling) {
            let edge = &self.events.edge_data[current_sibling as usize];
            let t = edge.range.start;

            let a = store.get(edge.from_id);
            let b = store.get(edge.to_id);

            assert!(a.len() == num_attributes);
            assert!(b.len() == num_attributes);
            for i in 0..num_attributes {
                self.attrib_buffer[i] += a[i] * (1.0 - t) + b[i] * t;
            }

            div += 1.0;
            current_sibling = self.events.next_sibling_id(current_sibling);
        }

        if div > 1.0 {
            for attribute in &mut self.attrib_buffer[..] {
                *attribute /= div;
            }
        }

        self.attrib_buffer
    }
}

/// An iterator over the sources of a given vertex.
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

fn log_svg_preamble(_tess: &FillTessellator) {
    tess_log!(_tess, r#"
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
fn fill_vertex_source_01() {
    use crate::path::commands::PathCommands;
    use crate::path::AttributeSlice;

    let endpoints: &[Point] = &[
        point(0.0, 0.0),
        point(1.0, 1.0),
        point(0.0, 2.0),
    ];

    let attributes = &[
        1.0, 0.0, 0.0,
        0.0, 1.0, 0.0,
        0.0, 0.0, 1.0,
    ];

    let mut cmds = PathCommands::builder();
    cmds.move_to(EndpointId(0));
    cmds.line_to(EndpointId(1));
    cmds.line_to(EndpointId(2));
    cmds.close();

    let cmds = cmds.build();

    let mut queue = EventQueue::from_path_with_ids(
        0.1,
        FillOptions::DEFAULT_SWEEP_ORIENTATION,
        cmds.id_events(),
        &(endpoints, endpoints),
    );

    let mut tess = FillTessellator::new();
    tess.tessellate_events(
        &mut queue,
        Some(&AttributeSlice::new(attributes, 3)),
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
        fn add_fill_vertex(&mut self, v: Point, mut attr: FillAttributes) -> Result<VertexId, GeometryBuilderError> {
            for src in attr.sources() {
                if eq(v, point(0.0, 0.0)) { assert!(at_endpoint(&src, EndpointId(0))) }
                else if eq(v, point(1.0, 1.0)) { assert!(at_endpoint(&src, EndpointId(1))) }
                else if eq(v, point(0.0, 2.0)) { assert!(at_endpoint(&src, EndpointId(2))) }
                else { panic!() }
            }

            if eq(v, point(0.0, 0.0)) { assert_eq!(attr.interpolated_attributes(), &[1.0, 0.0, 0.0]) }
            else if eq(v, point(1.0, 1.0)) { assert_eq!(attr.interpolated_attributes(), &[0.0, 1.0, 0.0]) }
            else if eq(v, point(0.0, 2.0)) { assert_eq!(attr.interpolated_attributes(), &[0.0, 0.0, 1.0]) }

            let id = self.next_vertex;
            self.next_vertex += 1;

            Ok(VertexId(id))
        }
    }
}

#[test]
fn fill_vertex_source_02() {
    // Check the vertex sources of a simple self-intersecting shape.
    //    _
    //  _|_|_
    // | | | |
    // |_|_|_|
    //   |_|
    //

    let mut path = crate::path::Path::builder_with_attributes(3);
    let a = path.move_to(point(1.0, 0.0), &[1.0, 0.0, 1.0]);
    let b = path.line_to(point(2.0, 0.0), &[2.0, 0.0, 1.0]);
    let c = path.line_to(point(2.0, 4.0), &[3.0, 0.0, 1.0]);
    let d = path.line_to(point(1.0, 4.0), &[4.0, 0.0, 1.0]);
    path.close();
    let e = path.move_to(point(0.0, 1.0), &[0.0, 1.0, 2.0]);
    let f = path.line_to(point(0.0, 3.0), &[0.0, 2.0, 2.0]);
    let g = path.line_to(point(3.0, 3.0), &[0.0, 3.0, 2.0]);
    let h = path.line_to(point(3.0, 1.0), &[0.0, 4.0, 2.0]);
    path.close();

    let path = path.build();

    let mut queue = EventQueue::from_path_with_ids(
        0.1,
        FillOptions::DEFAULT_SWEEP_ORIENTATION,
        path.id_iter(),
        &path,
    );

    let mut tess = FillTessellator::new();
    tess.tessellate_events(
        &mut queue,
        Some(&path),
        &FillOptions::default(),
        &mut CheckVertexSources {
            next_vertex: 0,
            a, b, c, d, e, f, g, h,
        },
    ).unwrap();

    struct CheckVertexSources {
        next_vertex: u32,
        a: EndpointId,
        b: EndpointId,
        c: EndpointId,
        d: EndpointId,
        e: EndpointId,
        f: EndpointId,
        g: EndpointId,
        h: EndpointId,
    }

    impl GeometryBuilder for CheckVertexSources {
        fn begin_geometry(&mut self) {}
        fn end_geometry(&mut self) -> Count { Count { vertices: self.next_vertex, indices: 0 } }
        fn abort_geometry(&mut self) {}
        fn add_triangle(&mut self, _: VertexId, _: VertexId, _: VertexId) {}
    }

    impl FillGeometryBuilder for CheckVertexSources {
        fn add_fill_vertex(&mut self, v: Point, mut attributes: FillAttributes) -> Result<VertexId, GeometryBuilderError> {
            for src in attributes.sources() {
                if      eq(v, point(1.0, 0.0)) { assert!(at_endpoint(&src, self.a)); }
                else if eq(v, point(2.0, 0.0)) { assert!(at_endpoint(&src, self.b)); }
                else if eq(v, point(2.0, 4.0)) { assert!(at_endpoint(&src, self.c)); }
                else if eq(v, point(1.0, 4.0)) { assert!(at_endpoint(&src, self.d)); }
                else if eq(v, point(0.0, 1.0)) { assert!(at_endpoint(&src, self.e)); }
                else if eq(v, point(0.0, 3.0)) { assert!(at_endpoint(&src, self.f)); }
                else if eq(v, point(3.0, 3.0)) { assert!(at_endpoint(&src, self.g)); }
                else if eq(v, point(3.0, 1.0)) { assert!(at_endpoint(&src, self.h)); }
                else if eq(v, point(1.0, 1.0)) { assert!(on_edge(&src, self.h, self.e, 2.0/3.0) || on_edge(&src, self.d, self.a, 3.0/4.0)); }
                else if eq(v, point(2.0, 1.0)) { assert!(on_edge(&src, self.h, self.e, 1.0/3.0) || on_edge(&src, self.b, self.c, 1.0/4.0)); }
                else if eq(v, point(1.0, 3.0)) { assert!(on_edge(&src, self.f, self.g, 1.0/3.0) || on_edge(&src, self.d, self.a, 1.0/4.0)); }
                else if eq(v, point(2.0, 3.0)) { assert!(on_edge(&src, self.f, self.g, 2.0/3.0) || on_edge(&src, self.b, self.c, 3.0/4.0)); }
                else { panic!() }
            }

            fn assert_attr(a: &[f32], b: &[f32]) {
                for i in 0..a.len() {
                    let are_equal = (a[i] - b[i]).abs() < 0.001;
                    if !are_equal {
                        println!("{:?} != {:?}", a, b);
                    }
                    assert!(are_equal);
                }

                assert_eq!(a.len(), b.len());
            }

            let attribs = attributes.interpolated_attributes();
            if      eq(v, point(1.0, 0.0)) { assert_attr(attribs, &[1.0, 0.0, 1.0]); }
            else if eq(v, point(2.0, 0.0)) { assert_attr(attribs, &[2.0, 0.0, 1.0]); }
            else if eq(v, point(2.0, 4.0)) { assert_attr(attribs, &[3.0, 0.0, 1.0]); }
            else if eq(v, point(1.0, 4.0)) { assert_attr(attribs, &[4.0, 0.0, 1.0]); }
            else if eq(v, point(0.0, 1.0)) { assert_attr(attribs, &[0.0, 1.0, 2.0]); }
            else if eq(v, point(0.0, 3.0)) { assert_attr(attribs, &[0.0, 2.0, 2.0]); }
            else if eq(v, point(3.0, 3.0)) { assert_attr(attribs, &[0.0, 3.0, 2.0]); }
            else if eq(v, point(3.0, 1.0)) { assert_attr(attribs, &[0.0, 4.0, 2.0]); }
            else if eq(v, point(1.0, 1.0)) { assert_attr(attribs, &[0.875, 1.0, 1.5]); }
            else if eq(v, point(2.0, 1.0)) { assert_attr(attribs, &[1.125, 1.5, 1.5]); }
            else if eq(v, point(1.0, 3.0)) { assert_attr(attribs, &[1.625, 1.16666, 1.5]); }
            else if eq(v, point(2.0, 3.0)) { assert_attr(attribs, &[1.375, 1.33333, 1.5]); }

            let id = self.next_vertex;
            self.next_vertex += 1;

            Ok(VertexId(id))
        }
    }
}
