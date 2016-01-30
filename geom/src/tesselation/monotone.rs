//!
//! Y-monotone decomposition and triangulation of shapes.
//!
//! This module provides the tools to generate triangles from arbitrary shapes with connectivity
//! information (using a half-edge connectivity kernel).
//!
//! The implementation inspired by the book Computational Geometry, Algorithms And Applications 3rd edition.
//!
//! Note that a lot of the comments and variable names in this module assume a coordinate
//! system where y is pointing downwards
//!
//!
//! # Examples
//!
//! ```ignore
//! extern crate vodk_math;
//! extern crate geom;
//! use geom::halfedge::{ ConnectivityKernel, FaceId };
//! use vodk_id::id_vector::*;
//! use geom::monotone::*;
//!
//! fn triangulate_faces(
//!     kernel: &mut ConnectivityKernel,
//!     faces: &[FaceId],
//!     vertices: &[Vec2],
//!     out_triangles: &mut TriangleStream
//! ) -> usize {
//!     let mut new_faces: Vec<FaceId> = vec![];
//!     for &f in faces {
//!         new_faces.push(f);
//!     }
//!     let vertex_positions = IdSlice::new(vertices);
//!     let mut ctx = DecompositionContext::new();
//!     for f in faces {
//!         let res = ctx.y_monotone_decomposition(kernel, *f, vertex_positions, &mut new_faces);
//!         assert_eq!(res, Ok(()));
//!     }
//!
//!     let mut ctx = TriangulationContext::new();
//!     for f in new_faces {
//!         debug_assert!(is_y_monotone(kernel, vertex_positions, f));
//!         let res = ctx.y_monotone_triangulation(
//!             kernel, f,
//!             vertex_positions,
//!             out_triangles
//!         );
//!         assert_eq!(res, Ok(()));
//!     }
//! }
//!
//! ```
//!

use std::cmp::{Ordering, PartialOrd};
use std::collections::HashMap;
use std::mem::swap;
use std::f32::consts::PI;

use half_edge::kernel::*;
use half_edge::iterators::{ DirectedEdgeCirculator};
use half_edge::vectors::{ Position2D, Vec2, vec2_sub, directed_angle };

use tesselation::vertex_builder::{ VertexBufferBuilder };
use tesselation::polygon::*;
use tesselation::path::WindingOrder;
use tesselation::polygon_partition::{ Diagonals, partition_polygon };

use vodk_alloc::*;
use vodk_id::*;
use vodk_id::id_vector::*;

#[derive(Debug, Copy, Clone)]
enum VertexType {
    Start,
    End,
    Split,
    Merge,
    Left,
    Right,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DecompositionError {
    OpenPath,
    InvertedWindingOrder,
    MissingFace,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TriangulationError {
    NotMonotone,
    InvalidPath,
    MissingFace,
}

fn is_below(a: Vec2, b: Vec2) -> bool { a.y() > b.y() || (a.y() == b.y() && a.x() > b.x()) }

fn get_vertex_type(prev: Vec2, current: Vec2, next: Vec2) -> VertexType {
    // assuming clockwise vertex_positions winding order
    let interrior_angle = directed_angle(vec2_sub(prev, current), vec2_sub(next, current));

    // If the interrior angle is exactly 0 we'll have degenerate (invisible 0-area) triangles
    // which is yucks but we can live with it for the sake of being robust against degenerate
    // inputs. So special-case them so that they don't get considered as Merge ot Split vertices
    // otherwise there can be no monotone decomposition of a shape where all points are on the
    // same line.

    if is_below(current, prev) && is_below(current, next) {
        if interrior_angle < PI && interrior_angle != 0.0 {
            return VertexType::Merge;
        } else {
            return VertexType::End;
        }
    }

    if !is_below(current, prev) && !is_below(current, next) {
        if interrior_angle < PI && interrior_angle != 0.0 {
            return VertexType::Split;
        } else {
            return VertexType::Start;
        }
    }

    if prev.y() == next.y() {
        return if prev.x() < next.x() { VertexType::Right } else { VertexType::Left };
    }
    return if prev.y() < next.y() { VertexType::Right } else { VertexType::Left };
}

fn intersect_segment_with_horizontal(a: [f32;2], b: [f32;2], y: f32) -> f32 {
    let vx = b.x() - a.x();
    let vy = b.y() - a.y();
    if vy == 0.0 {
        // If the segment is horizontal, pick the biggest x value (the right-most point).
        // That's an arbitrary decision that serves the purpose of y-monotone decomposition
        return a.x().max(b.x());
    }
    return a.x() + (y - a.y()) * vx / vy;
}

#[cfg(test)]
fn assert_almost_eq(a: f32, b:f32) {
    if (a - b).abs() < 0.0001 { return; }
    println!("expected {} and {} to be equal", a, b);
    panic!();
}

#[test]
fn test_intersect_segment_horizontal() {
    assert_almost_eq(
        intersect_segment_with_horizontal([0.0, 0.0], [0.0, 2.0], 1.0),
        0.0,
    );
    assert_almost_eq(
        intersect_segment_with_horizontal([0.0, 2.0], [2.0, 0.0], 1.0),
        1.0,
    );
    assert_almost_eq(
        intersect_segment_with_horizontal([0.0, 1.0], [3.0, 0.0], 0.0),
        3.0,
    );
}

// Contains the immutable state required to manage the sweep line state (but not
// not the mutable stats like the sweep line itself because the borrow checker
// makes that impractical).
struct SweepLineBuilder<'l, P:'l+Position2D> {
    kernel: &'l ConnectivityKernel,
    vertices: IdSlice<'l, VertexId, P>,
    current_vertex: Vec2
}

impl<'l, P: 'l+Position2D> SweepLineBuilder<'l, P> {
    fn vertex_position(&self, e: EdgeId) -> Vec2 {
        return self.vertices[self.kernel[e].vertex].position();
    }

    fn add(&self, sweep_line: &mut Vec<EdgeId>, e: EdgeId) {
        sweep_line.push(e);
        // sort from left to right (increasing x values)
        sweep_line.sort_by(|ea, eb| {
            let a1 = self.vertex_position(*ea);
            let a2 = self.vertex_position(self.kernel[*ea].next);
            let b1 = self.vertex_position(*eb);
            let b2 = self.vertex_position(self.kernel[*eb].next);
            let xa = intersect_segment_with_horizontal(a1, a2, self.current_vertex.y());
            let xb = intersect_segment_with_horizontal(b1, b2, self.current_vertex.y());
            return xa.partial_cmp(&xb).unwrap();
        });
        println!(" sweep status is: {:?}", sweep_line);
    }

    fn remove(&self, sweep_line: &mut Vec<EdgeId>, e: EdgeId) {
        println!(" remove {} from sweep line", e.handle);
        sweep_line.retain(|item|{ *item != e });
    }

    // Search the sweep status to find the edge directly to the right of the current vertex.
    fn find_right_of_current_vertex(&self, sweep_line: &Vec<EdgeId>) -> EdgeId {
        for &e in sweep_line {
            let a = self.vertex_position(e);
            let b = self.vertex_position(self.kernel[e].next);
            let x = intersect_segment_with_horizontal(a, b, self.current_vertex.y());
            println!(" -- split: search sweep status {} x: {}", e.handle, x);

            if x >= self.current_vertex.x() {
                return e;
            }
        }
        panic!("Could not find the edge directly right of e on the sweep line");
    }
}

struct SweepLineBuilder2<'l, P:'l+Position2D> {
    polygon: &'l ComplexPolygon,
    vertices: IdSlice<'l, VertexId, P>,
    current_vertex: Vec2
}

impl<'l, P: 'l+Position2D> SweepLineBuilder2<'l, P> {
    fn vertex_position(&self, e: ComplexPointId) -> Vec2 {
        return self.vertices[self.polygon.vertex(e)].position();
    }

    fn add(&self, sweep_line: &mut Vec<ComplexPointId>, e: ComplexPointId) {
        sweep_line.push(e);
        // sort from left to right (increasing x values)
        sweep_line.sort_by(|ea, eb| {
            let a1 = self.vertex_position(*ea);
            let a2 = self.vertex_position(self.polygon.next(*ea));
            let b1 = self.vertex_position(*eb);
            let b2 = self.vertex_position(self.polygon.next(*eb));
            let xa = intersect_segment_with_horizontal(a1, a2, self.current_vertex.y());
            let xb = intersect_segment_with_horizontal(b1, b2, self.current_vertex.y());
            return xa.partial_cmp(&xb).unwrap();
        });
        println!(" sweep status is: {:?}", sweep_line);
    }

    fn remove(&self, sweep_line: &mut Vec<ComplexPointId>, e: ComplexPointId) {
        println!(" remove {:?} from sweep line", e);
        sweep_line.retain(|item|{ *item != e });
    }

    // Search the sweep status to find the edge directly to the right of the current vertex.
    fn find_right_of_current_vertex(&self, sweep_line: &Vec<ComplexPointId>) -> ComplexPointId {
        for &e in sweep_line {
            let a = self.vertex_position(e);
            let b = self.vertex_position(self.polygon.next(e));
            let x = intersect_segment_with_horizontal(a, b, self.current_vertex.y());
            println!(" -- search sweep status {:?} x: {}", e, x);

            if x >= self.current_vertex.x() {
                return e;
            }
        }
        panic!("Could not find the edge directly right of e on the sweep line");
    }
}


fn connect<Faces: Write<FaceId>>(
    kernel: &mut ConnectivityKernel,
    mut a: EdgeId,
    mut b: EdgeId,
    new_faces: &mut Faces
) {
    let first_a = a;
    let first_b = b;

    println!(" > connect vertices {} {} edges {} {} ",
        kernel[a].vertex.to_index(),
        kernel[b].vertex.to_index(),
        a.to_index(), b.to_index()
    );

    // Look for a and b such that they share the same face.
    loop {
        let mut ok = false;
        loop {
            if kernel[a].face == kernel[b].face  {
                ok = true;
                break;
            }
            a = kernel.next_edge_id_around_vertex(a).unwrap();
            debug_assert_eq!(kernel[a].vertex, kernel[first_a].vertex);
            if a == first_a { break; }
        }
        if ok { break; }
        b = kernel.next_edge_id_around_vertex(b).unwrap();
        debug_assert_eq!(kernel[b].vertex, kernel[first_b].vertex);
        debug_assert!(b != first_b);
    }

    debug_assert_eq!(kernel[a].face, kernel[b].face);

    println!(" > actually connect vertices {} {} edges {} {}",
        kernel[a].vertex.to_index(),
        kernel[b].vertex.to_index(),
        a.to_index(), b.to_index()
    );

    if let Some(face) = kernel.connect_edges2(a, b) {
        new_faces.write(face);
    }
}


fn connect_with_helper_if_merge_vertex(current_edge: EdgeId,
                                       helper_edge: EdgeId,
                                       helpers: &mut HashMap<EdgeId, (EdgeId, VertexType)>,
                                       connections: &mut Vec<(EdgeId, EdgeId)>) {
    if let Some(&(h, VertexType::Merge)) = helpers.get(&helper_edge) {
        println!("   right: removed helper from edge {}", helper_edge.handle);
        connections.push((h, current_edge));
        println!("connection {}->{}", current_edge.handle, h.handle);
    }
}

fn connect_with_helper_if_merge_vertex2(current_edge: ComplexPointId,
                                       helper_edge: ComplexPointId,
                                       helpers: &mut HashMap<ComplexPointId, (ComplexPointId, VertexType)>,
                                       diagonals: &mut Diagonals<ComplexPolygon>) {
    if let Some(&(h, VertexType::Merge)) = helpers.get(&helper_edge) {
        diagonals.add_diagonal(h, current_edge);
        println!("      helper {:?} of {:?} is a merge vertex", h, helper_edge);
        println!(" **** connection {:?}->{:?}", h, current_edge);
    }
}

/// Can perform y-monotone decomposition on a connectivity kernel.
///
/// This object holds on to the memory that was allocated during previous
/// decompositions in order to avoid allocating during the next decompositions
/// if possible.
pub struct DecompositionContext {
    sorted_edges_storage: Allocation,
    // list of edges that intercept the sweep line, sorted by increasing x coordinate
    sweep_state_storage: Allocation,
    helper: HashMap<EdgeId, (EdgeId, VertexType)>,
}

impl DecompositionContext {
    /// Constructor
    pub fn new() -> DecompositionContext {
        DecompositionContext {
            sorted_edges_storage: Allocation::empty(),
            sweep_state_storage: Allocation::empty(),
            helper: HashMap::new(),
        }
    }

    /// Constructor which pre-allocates memory
    pub fn with_capacity(edges: usize, sweep: usize, helpers: usize) -> DecompositionContext {
        let edges_vec: Vec<EdgeId> = Vec::with_capacity(edges);
        let sweep_vec: Vec<EdgeId> = Vec::with_capacity(sweep);
        DecompositionContext {
            sorted_edges_storage: vec::recycle(edges_vec),
            sweep_state_storage: vec::recycle(sweep_vec),
            helper: HashMap::with_capacity(helpers),
        }
    }

    /// Applies an y_monotone decomposition of a face in a connectivity kernel.
    ///
    /// This operation will add faces and edges to the connectivity kernel.
    pub fn y_monotone_decomposition<'l,
        P: Position2D,
        Faces: Write<FaceId> // TODO: output connections instead
    >(
        &mut self,
        kernel: &'l mut ConnectivityKernel,
        face_id: FaceId,
        vertex_positions: IdSlice<'l, VertexId, P>,
        new_faces: &'l mut Faces
    ) -> Result<(), DecompositionError> {
        self.helper.clear();

        if !kernel.contains_face(face_id) {
            return Err(DecompositionError::MissingFace);
        }

        let mut storage = Allocation::empty();
        swap(&mut self.sweep_state_storage, &mut storage);
        let mut sweep_state: Vec<EdgeId> = create_vec_from(storage);

        let mut storage = Allocation::empty();
        swap(&mut self.sorted_edges_storage, &mut storage);
        let mut sorted_edges: Vec<EdgeId> = create_vec_from(storage);

        sorted_edges.extend(kernel.walk_edge_ids_around_face(face_id));

        println!("Unsorted edges before holes: {:?}", sorted_edges);

        // also add holes in the sorted edge list
        for &inner in &kernel[face_id].inner_edges {
            debug_assert_eq!(kernel[inner].face, face_id);
            sorted_edges.extend(kernel.walk_edge_ids(inner));
        }

        println!("Unsorted edges: {:?}", sorted_edges);

        let mut connections: Vec<(EdgeId, EdgeId)> = Vec::new();

        // sort indices by increasing y coordinate of the corresponding vertex
        sorted_edges.sort_by(|a, b| {
            let va = vertex_positions[kernel[*a].vertex].position();
            let vb = vertex_positions[kernel[*b].vertex].position();
            if va.y() > vb.y() { return Ordering::Greater; }
            if va.y() < vb.y() { return Ordering::Less; }
            if va.x() > vb.x() { return Ordering::Greater; }
            if va.x() < vb.x() { return Ordering::Less; }
            return Ordering::Equal;
        });

        for &e in &sorted_edges {
            let edge = kernel[e];
            let current_vertex = vertex_positions[edge.vertex].position();
            let previous_vertex = vertex_positions[kernel[edge.prev].vertex].position();
            let next_vertex = vertex_positions[kernel[edge.next].vertex].position();
            let vertex_type = get_vertex_type(previous_vertex, current_vertex, next_vertex);
            let sweep_line = SweepLineBuilder {
                kernel: kernel,
                vertices: vertex_positions,
                current_vertex: current_vertex,
            };

            println!("\n\n ** current vertex: {} edge {} pos {:?} type {:?}",
                edge.vertex.to_index(), e.to_index(), vertex_positions[edge.vertex].position(), vertex_type
            );
            match vertex_type {
                VertexType::Start => {
                    sweep_line.add(&mut sweep_state, e);
                    self.helper.insert(e, (e, vertex_type));
                }
                VertexType::End => {
                    connect_with_helper_if_merge_vertex(e, edge.prev, &mut self.helper, &mut connections);
                    sweep_line.remove(&mut sweep_state, edge.prev);
                }
                VertexType::Split => {
                    let ej = sweep_line.find_right_of_current_vertex(&sweep_state);
                    if let Some(&(helper_edge,_)) = self.helper.get(&ej) {
                        connections.push((e, helper_edge));
                        println!("connection {}->{}", e.handle, helper_edge.handle);
                    } else {
                        println!(" !!! no helper for edge {}", ej.handle);
                        panic!();
                    }
                    self.helper.insert(ej, (e, vertex_type));

                    sweep_line.add(&mut sweep_state, e);
                    self.helper.insert(e, (e, vertex_type));
                }
                VertexType::Merge => {
                    connect_with_helper_if_merge_vertex(e, edge.prev, &mut self.helper, &mut connections);
                    sweep_line.remove(&mut sweep_state, edge.prev);

                    let ej = sweep_line.find_right_of_current_vertex(&sweep_state);
                    connect_with_helper_if_merge_vertex(e, ej, &mut self.helper, &mut connections);
                    self.helper.insert(ej, (e, vertex_type));
                }
                VertexType::Right => {
                    // TODO remove helper(edge.prev) ?
                    connect_with_helper_if_merge_vertex(e, edge.prev, &mut self.helper, &mut connections);
                    self.helper.remove(&edge.prev);
                    sweep_line.remove(&mut sweep_state, edge.prev);

                    sweep_line.add(&mut sweep_state, e);
                    self.helper.insert(e, (e, vertex_type));
                }
                VertexType::Left => {
                    let ej = sweep_line.find_right_of_current_vertex(&sweep_state);
                    connect_with_helper_if_merge_vertex(e, ej, &mut self.helper, &mut connections);

                    self.helper.insert(ej, (e, vertex_type));
                }
            }
        }

        //let mut replaced_edges: HashMap<u16, EdgeId> = HashMap::new();
        for (a, b) in connections {
            //if let Some(new_a) = replaced_edges.remove(&a.handle) {
            //    a = new_a;
            //}
            //if let Some(new_b) = replaced_edges.remove(&b.handle) {
            //    b = new_b;
            //}
            //let a_prev = kernel[a].prev;
            connect(kernel, a, b, new_faces);
            //replaced_edges.insert(a.handle, kernel[a_prev].next);
        }

        // Keep the buffers to avoid reallocating it next time, if possible.
        self.sweep_state_storage = vec::recycle(sweep_state);
        self.sorted_edges_storage = vec::recycle(sorted_edges);

        return Ok(());
    }
}

/// Can perform y-monotone decomposition on a connectivity kernel.
///
/// This object holds on to the memory that was allocated during previous
/// decompositions in order to avoid allocating during the next decompositions
/// if possible.
pub struct DecompositionContext2 {
    sorted_edges_storage: Allocation,
    // list of edges that intercept the sweep line, sorted by increasing x coordinate
    sweep_state_storage: Allocation,
    helper: HashMap<ComplexPointId, (ComplexPointId, VertexType)>,
}

impl DecompositionContext2 {
    pub fn new() -> DecompositionContext2 {
        DecompositionContext2 {
            sorted_edges_storage: Allocation::empty(),
            sweep_state_storage: Allocation::empty(),
            helper: HashMap::new(),
        }
    }

    /// Applies an y_monotone decomposition of a face in a connectivity kernel.
    ///
    /// This operation will add faces and edges to the connectivity kernel.
    pub fn y_monotone_polygon_decomposition<'l,
        P: Position2D
    >(
        &mut self,
        polygon: &'l ComplexPolygon,
        vertex_positions: IdSlice<'l, VertexId, P>,
        diagonals: &'l mut Diagonals<ComplexPolygon>
    ) -> Result<(), DecompositionError> {
        self.helper.clear();

        let mut storage = Allocation::empty();
        swap(&mut self.sweep_state_storage, &mut storage);
        let mut sweep_state: Vec<ComplexPointId> = create_vec_from(storage);

        let mut storage = Allocation::empty();
        swap(&mut self.sorted_edges_storage, &mut storage);
        let mut sorted_edges: Vec<ComplexPointId> = create_vec_from(storage);

        for sub_poly in polygon.polygon_ids() {
            println!(" +++++ sub poly {:?}", sub_poly);
            if sub_poly != polygon_id(0) {
                let winding = compute_winding_order(polygon.get_sub_polygon(sub_poly).unwrap(), vertex_positions);
                debug_assert_eq!(winding, Some(WindingOrder::CounterClockwise)
                );
            }
            sorted_edges.extend(polygon.point_ids(sub_poly));
        }
        debug_assert!(sorted_edges.len() == polygon.num_vertices());

        println!("Unsorted edges: {:?}", sorted_edges);

        // sort indices by increasing y coordinate of the corresponding vertex
        sorted_edges.sort_by(|a, b| {
            let va = vertex_positions[polygon.vertex(*a)].position();
            let vb = vertex_positions[polygon.vertex(*b)].position();
            if va.y() > vb.y() { return Ordering::Greater; }
            if va.y() < vb.y() { return Ordering::Less; }
            if va.x() > vb.x() { return Ordering::Greater; }
            if va.x() < vb.x() { return Ordering::Less; }
            return Ordering::Equal;
        });

        for &e in &sorted_edges {
            //let edge = kernel[e];
            let prev = polygon.previous(e);
            let next = polygon.next(e);
            let current_vertex = vertex_positions[polygon.vertex(e)].position();
            let previous_vertex = vertex_positions[polygon.vertex(prev)].position();
            let next_vertex = vertex_positions[polygon.vertex(next)].position();
            let vertex_type = get_vertex_type(previous_vertex, current_vertex, next_vertex);
            let sweep_line = SweepLineBuilder2 {
                polygon: polygon,
                vertices: vertex_positions,
                current_vertex: current_vertex,
            };
            println!("\n ============= point {:?}   type {:?}", e, vertex_type);
            match vertex_type {
                VertexType::Start => {
                    sweep_line.add(&mut sweep_state, e);
                    self.helper.insert(e, (e, vertex_type));
                }
                VertexType::End => {
                    connect_with_helper_if_merge_vertex2(e, prev, &mut self.helper, diagonals);
                    sweep_line.remove(&mut sweep_state, prev);
                }
                VertexType::Split => {
                    let ej = sweep_line.find_right_of_current_vertex(&sweep_state);
                    if let Some(&(helper_edge,_)) = self.helper.get(&ej) {
                        diagonals.add_diagonal(e, helper_edge);
                        println!(" **** connection {:?}->{:?}", e, helper_edge);
                    } else {
                        panic!();
                    }
                    self.helper.insert(ej, (e, vertex_type));

                    sweep_line.add(&mut sweep_state, e);
                    self.helper.insert(e, (e, vertex_type));
                }
                VertexType::Merge => {
                    println!(" - ");
                    connect_with_helper_if_merge_vertex2(e, prev, &mut self.helper, diagonals);
                    sweep_line.remove(&mut sweep_state, prev);

                    println!(" - ");
                    let ej = sweep_line.find_right_of_current_vertex(&sweep_state);
                    connect_with_helper_if_merge_vertex2(e, ej, &mut self.helper, diagonals);
                    self.helper.insert(ej, (e, vertex_type));
                }
                VertexType::Right => {
                    // TODO remove helper(edge.prev) ?
                    connect_with_helper_if_merge_vertex2(e, prev, &mut self.helper, diagonals);
                    self.helper.remove(&prev);
                    sweep_line.remove(&mut sweep_state, prev);

                    sweep_line.add(&mut sweep_state, e);
                    self.helper.insert(e, (e, vertex_type));
                }
                VertexType::Left => {
                    let ej = sweep_line.find_right_of_current_vertex(&sweep_state);
                    connect_with_helper_if_merge_vertex2(e, ej, &mut self.helper, diagonals);

                    self.helper.insert(ej, (e, vertex_type));
                }
            }
        }

        // Keep the buffers to avoid reallocating it next time, if possible.
        self.sweep_state_storage = vec::recycle(sweep_state);
        self.sorted_edges_storage = vec::recycle(sorted_edges);

        return Ok(());
    }
}

/// Returns true if the face is y-monotone in O(n).
pub fn is_y_monotone<'l, Pos: Position2D>(
    kernel: &'l ConnectivityKernel,
    vertex_positions: IdSlice<'l, VertexId, Pos>,
    face: FaceId
) -> bool {
    for e in kernel.walk_edge_ids_around_face(face) {
        let edge = kernel[e];
        let current_vertex = vertex_positions[edge.vertex].position();
        let previous_vertex = vertex_positions[kernel[edge.prev].vertex].position();
        let next_vertex = vertex_positions[kernel[edge.next].vertex].position();
        match get_vertex_type(previous_vertex, current_vertex, next_vertex) {
            VertexType::Split | VertexType::Merge => {
                println!("not y monotone because of vertices {} {} {} edges {} {} {} \n --- shape: ",
                    kernel[edge.prev].vertex.to_index(), edge.vertex.to_index(), kernel[edge.next].vertex.to_index(),
                    edge.prev.to_index(), e.to_index(), edge.next.to_index()
                );
                for e in kernel.walk_edge_ids_around_face(face) {
                    println!(" vertex {}  edge {} pos {:?}",
                        kernel[e].vertex.to_index(), e.to_index(),
                        vertex_positions[kernel[e].vertex].position()
                    );
                }
                println!(" ---");
                return false;
            }
            _ => {}
        }
    }
    return true;
}

/// Returns true if the face is y-monotone in O(n).
pub fn is_y_monotone_polygon<'l, Pos: Position2D>(
    polygon: PolygonView<'l>,
    vertex_positions: IdSlice<'l, VertexId, Pos>,
) -> bool {
    for point in polygon.point_ids() {
        let previous = vertex_positions[polygon.previous_vertex(point)].position();
        let current = vertex_positions[polygon.vertex(point)].position();
        let next = vertex_positions[polygon.next_vertex(point)].position();

        match get_vertex_type(previous, current, next) {
            VertexType::Split | VertexType::Merge => {
                return false;
            }
            _ => {}
        }
    }
    return true;
}

pub trait Write<T> { fn write(&mut self, data: T); }

/// A dummy implementation that doesn't write anything. Useful when ignoring the output
/// of an algorithm.
impl<T> Write<T> for () { fn write(&mut self, _data: T) {} }

/// Write into a Vec.
impl<T> Write<T> for Vec<T> { fn write(&mut self, data: T) { self.push(data) } }

/// Writes triangles as indices in a &[u16].
pub struct SliceTriangleWriter<'l> {
    indices: &'l mut[u16],
    offset: usize,
}

impl<'l> Write<[VertexId; 3]> for SliceTriangleWriter<'l> {
    fn write(&mut self, indices: [VertexId; 3]) {
        debug_assert!(indices[0] != indices[1]);
        debug_assert!(indices[0] != indices[2]);
        debug_assert!(indices[1] != indices[2]);
        self.indices[self.offset] = indices[0].to_index() as u16;
        self.indices[self.offset+1] = indices[1].to_index() as u16;
        self.indices[self.offset+2] = indices[2].to_index() as u16;
        self.offset += 3;
    }
}

impl<'l> SliceTriangleWriter<'l> {
    pub fn new(buffer: &'l mut[u16]) -> SliceTriangleWriter {
        SliceTriangleWriter {
            indices: buffer,
            offset: 0,
        }
    }
}

/// Can perform y-monotone triangulation on a connectivity kernel.
///
/// This object holds on to the memory that was allocated during previous
/// triangulations, in order to avoid allocating during the next triangulations
/// if possible.
pub struct TriangulationContext {
    vertex_stack_storage: Allocation,
}

impl TriangulationContext {
    /// Constructor.
    pub fn new() -> TriangulationContext {
        TriangulationContext {
            vertex_stack_storage: Allocation::empty()
        }
    }

    /// Computes an y-monotone triangulation of a face in the connectivity kernel,
    /// outputing triangles by pack of 3 vertex indices in a TriangleStream.
    ///
    /// Returns the number of indices that were added to the stream.
    pub fn y_monotone_triangulation<'l,
        P: Position2D,
        Output: VertexBufferBuilder<Vec2>
    >(
        &mut self,
        kernel: &'l ConnectivityKernel,
        face_id: FaceId,
        vertex_positions: IdSlice<'l, VertexId, P>,
        output: &mut Output,
    ) -> Result<(), TriangulationError> {
        if !kernel.contains_face(face_id) {
            return Err(TriangulationError::MissingFace);
        }

        let mut up = DirectedEdgeCirculator::new(kernel, kernel[face_id].first_edge, Direction::Forward);
        let mut down = up.clone();
        loop {
            down = down.next();
            if vertex_positions[up.vertex_id()].y() != vertex_positions[down.vertex_id()].y() {
                break;
            }
            if down == up {
                // Avoid an infnite loop in the degenerate case where all vertices are in the same position.
                break;
            }
        }

        if vertex_positions[up.vertex_id()].y() < vertex_positions[down.vertex_id()].y() {
            up.set_direction(Direction::Backward);
        } else {
            down.set_direction(Direction::Backward);
        }

        // find the bottom-most vertex (with the highest y value)
        let mut big_y = vertex_positions[down.vertex_id()].y();
        let guard = down;
        loop {
            debug_assert_eq!(down.face_id(), face_id);
            down = down.next();
            let new_y = vertex_positions[down.vertex_id()].y();
            if new_y < big_y {
                down = down.prev();
                break;
            }
            big_y = new_y;
            if down == guard {
                // We have looped through all vertices already because of
                // a degenerate input, avoid looping infinitely.
                break;
            }
        }

        // find the top-most vertex (with the smallest y value)
        let mut small_y = vertex_positions[up.vertex_id()].y();
        let guard = up;
        loop {
            debug_assert_eq!(up.face_id(), face_id);
            up = up.next();
            let new_y = vertex_positions[up.vertex_id()].y();
            if new_y > small_y {
                up = up.prev();
                break;
            }
            small_y = new_y;
            if up == guard {
                // We have looped through all vertices already because of
                // a degenerate input, avoid looping infinitely.
                break;
            }
        }

        // now that we have the top-most vertex, we will circulate simulataneously
        // from the left and right chains until we reach the bottom-most vertex

        // main chain
        let mut m = up.clone();

        // opposite chain
        let mut o = up.clone();
        m.set_direction(Direction::Forward);
        o.set_direction(Direction::Backward);

        m = m.next();
        o = o.next();

        if vertex_positions[m.vertex_id()].y() > vertex_positions[o.vertex_id()].y() {
            swap(&mut m, &mut o);
        }

        m = m.prev();
        // previous
        let mut p = m;

        // vertices already visited, waiting to be connected
        let mut storage = Allocation::empty();
        swap(&mut storage, &mut self.vertex_stack_storage);
        let mut vertex_stack: Vec<DirectedEdgeCirculator> = create_vec_from(storage);

        let mut triangle_count = 0;
        let mut i: i32 = 0;
        loop {
            // walk edges from top to bottom, alternating between the left and
            // right chains. The chain we are currently iterating over is the
            // main chain (m) and the other one the opposite chain (o).
            // p is the previous iteration, regardless of which chain it is on.
            if vertex_positions[m.vertex_id()].y() > vertex_positions[o.vertex_id()].y() || m == down {
                swap(&mut m, &mut o);
            }

            if i < 2 {
                vertex_stack.push(m);
            } else {
                if vertex_stack.len() > 0 && m.direction() != vertex_stack[vertex_stack.len()-1].direction() {
                    for i in 0..vertex_stack.len() - 1 {
                        let id_1 = vertex_stack[i].vertex_id();
                        let id_2 = vertex_stack[i+1].vertex_id();
                        let id_opp = m.vertex_id();

                        output.push_indices(id_opp.handle, id_1.handle, id_2.handle);
                        triangle_count += 1;
                    }

                    vertex_stack.clear();

                    vertex_stack.push(p);
                    vertex_stack.push(m);

                } else {

                    let mut last_popped = vertex_stack.pop();

                    loop {
                        if vertex_stack.len() < 1 {
                            break;
                        }
                        let mut id_1 = vertex_stack[vertex_stack.len()-1].vertex_id();
                        let id_2 = last_popped.unwrap().vertex_id();
                        let mut id_3 = m.vertex_id();

                        if m.direction() == Direction::Backward {
                            swap(&mut id_1, &mut id_3);
                        }

                        let v1 = vertex_positions[id_1].position();
                        let v2 = vertex_positions[id_2].position();
                        let v3 = vertex_positions[id_3].position();
                        if directed_angle(vec2_sub(v1, v2), vec2_sub(v3, v2)) > PI {
                            output.push_indices(id_1.handle, id_2.handle, id_3.handle);
                            triangle_count += 1;

                            last_popped = vertex_stack.pop();

                        } else {
                            break;
                        }
                    } // loop 2

                    if let Some(item) = last_popped {
                        vertex_stack.push(item);
                    }
                    vertex_stack.push(m);

                }
            }

            if m == down {
                if o == down {
                    break;
                }
            }

            i += 1;
            p = m;
            m = m.next();
            debug_assert!(vertex_positions[m.vertex_id()].y() >= vertex_positions[p.vertex_id()].y());
        }
        debug_assert_eq!(triangle_count, kernel.walk_edge_ids_around_face(face_id).count() as usize - 2);

        // Keep the buffer to avoid reallocating it next time, if possible.
        self.vertex_stack_storage = vec::recycle(vertex_stack);
        return Ok(());
    }
}

/// Can perform y-monotone triangulation on a connectivity kernel.
///
/// This object holds on to the memory that was allocated during previous
/// triangulations, in order to avoid allocating during the next triangulations
/// if possible.
pub struct TriangulationContext2 {
    vertex_stack_storage: Allocation,
}

impl TriangulationContext2 {
    /// Constructor.
    pub fn new() -> TriangulationContext2 {
        TriangulationContext2 {
            vertex_stack_storage: Allocation::empty()
        }
    }

    /// Computes an y-monotone triangulation of a face in the connectivity kernel,
    /// outputing triangles by pack of 3 vertex indices in a TriangleStream.
    ///
    /// Returns the number of indices that were added to the stream.
    pub fn y_monotone_triangulation<'l,
        P: Position2D,
        Output: VertexBufferBuilder<Vec2>
    >(
        &mut self,
        polygon: PolygonView<'l>,
        vertex_positions: IdSlice<'l, VertexId, P>,
        output: &mut Output,
    ) -> Result<(), TriangulationError> {
        println!(" ------ monotone triangulation, polygon with {} vertices", polygon.num_vertices());

        // for convenience
        let vertex = |circ: Circulator| { vertex_positions[polygon.vertex(circ.point)].position() };
        let next = |circ: Circulator| { Circulator { point: polygon.advance(circ.point, circ.direction), direction: circ.direction } };
        let previous = |circ: Circulator| { Circulator { point: polygon.advance(circ.point, circ.direction.reverse()), direction: circ.direction } };

        #[derive(Copy, Clone, Debug, PartialEq)]
        struct Circulator {
            point: PointId,
            direction: Direction,
        }

        let mut up = Circulator { point: polygon.first_point(), direction: Direction::Forward };
        let mut down = up.clone();

        loop {
            down = next(down);
            if vertex(up).y() != vertex(down).y() {
                break;
            }
            if down == up {
                // Avoid an infnite loop in the degenerate case where all vertices are in the same position.
                break;
            }
        }

        up.direction = if is_below(vertex(up), vertex(down)) { Direction::Forward }
                       else { Direction::Backward };

        down.direction = up.direction.reverse();

        // Find the bottom-most vertex (with the highest y value)
        let mut big_y = vertex(down);
        let guard = down;
        loop {
            down = next(down);
            let new_y = vertex(down);
            if is_below(big_y, new_y) {
                down = previous(down);
                break;
            }
            big_y = new_y;
            if down == guard {
                // We have looped through all vertices already because of
                // a degenerate input, avoid looping infinitely.
                break;
            }
        }
        // find the top-most vertex (with the smallest y value)
        let mut small_y = vertex(up);
        let guard = up;
        loop {
            up = next(up);
            let new_y = vertex(up);
            if is_below(new_y, small_y) {
                up = previous(up);
                break;
            }
            small_y = new_y;
            if up == guard {
                // We have looped through all vertices already because of
                // a degenerate input, avoid looping infinitely.
                break;
            }
        }

        // now that we have the top-most vertex, we will circulate simulataneously
        // from the left and right chains until we reach the bottom-most vertex

        // main chain
        let mut m = up.clone();

        // opposite chain
        let mut o = up.clone();
        m.direction = Direction::Forward;
        o.direction = Direction::Backward;

        m = next(m);
        o = next(o);

        if is_below(vertex(m), vertex(o)) {
            swap(&mut m, &mut o);
        }

        m = previous(m);
        // previous
        let mut p = m;

        // vertices already visited, waiting to be connected
        let mut storage = Allocation::empty();
        swap(&mut storage, &mut self.vertex_stack_storage);
        let mut vertex_stack: Vec<Circulator> = create_vec_from(storage);

        let mut triangle_count = 0;
        let mut i: i32 = 0;

        println!(" -- up: {:?}  down: {:?}", up.point, down.point);

        loop {
            println!("   -- m: {:?}  o: {:?}", m.point, o.point);
            // walk edges from top to bottom, alternating between the left and
            // right chains. The chain we are currently iterating over is the
            // main chain (m) and the other one the opposite chain (o).
            // p is the previous iteration, regardless of which chain it is on.
            if is_below(vertex(m), vertex(o)) || m == down {
                swap(&mut m, &mut o);
            }

            if i < 2 {
                vertex_stack.push(m);
            } else {
                if vertex_stack.len() > 0 && m.direction != vertex_stack[vertex_stack.len()-1].direction {
                    for i in 0..vertex_stack.len() - 1 {
                        let id_1 = polygon.vertex(vertex_stack[i].point);
                        let id_2 = polygon.vertex(vertex_stack[i+1].point);
                        let id_opp = polygon.vertex(m.point);

                        output.push_indices(id_opp.handle, id_1.handle, id_2.handle);
                        triangle_count += 1;
                    }

                    vertex_stack.clear();

                    vertex_stack.push(p);
                    vertex_stack.push(m);

                } else {

                    let mut last_popped = vertex_stack.pop();

                    loop {
                        if vertex_stack.len() < 1 {
                            break;
                        }
                        let mut id_1 = polygon.vertex(vertex_stack[vertex_stack.len()-1].point);
                        let id_2 = polygon.vertex(last_popped.unwrap().point);
                        let mut id_3 = polygon.vertex(m.point);

                        if m.direction == Direction::Backward {
                            swap(&mut id_1, &mut id_3);
                        }

                        let v1 = vertex_positions[id_1].position();
                        let v2 = vertex_positions[id_2].position();
                        let v3 = vertex_positions[id_3].position();
                        if directed_angle(vec2_sub(v1, v2), vec2_sub(v3, v2)) > PI {
                            output.push_indices(id_1.handle, id_2.handle, id_3.handle);
                            triangle_count += 1;

                            last_popped = vertex_stack.pop();

                        } else {
                            break;
                        }
                    } // loop 2

                    if let Some(item) = last_popped {
                        vertex_stack.push(item);
                    }
                    vertex_stack.push(m);

                }
            }

            if m.point == down.point {
                if o.point == down.point {
                    break;
                }
            }

            i += 1;
            p = m;
            m = next(m);
            if vertex(m).y() < vertex(p).y() {
                println!("   !!! m: {:?}  o: {:?}", m.point, o.point);
            }
            debug_assert!(!is_below(vertex(p), vertex(m)));
        }
        debug_assert_eq!(triangle_count, polygon.num_vertices() as usize - 2);

        // Keep the buffer to avoid reallocating it next time, if possible.
        self.vertex_stack_storage = vec::recycle(vertex_stack);
        return Ok(());
    }
}

#[cfg(test)]
pub fn triangulate_faces<T:Position2D, Output: VertexBufferBuilder<Vec2>>(
    kernel: &mut ConnectivityKernel,
    faces: &[FaceId],
    vertices: &[T],
    output: &mut Output
) {
    let mut new_faces: Vec<FaceId> = vec![];
    for &f in faces {
        new_faces.push(f);
    }
    let vertex_positions = IdSlice::new(vertices);
    let mut ctx = DecompositionContext::new();
    for f in faces {
        let res = ctx.y_monotone_decomposition(kernel, *f, vertex_positions, &mut new_faces);
        assert_eq!(res, Ok(()));
    }

    let mut triangulator = TriangulationContext::new();
    for f in new_faces {
        assert!(is_y_monotone(kernel, vertex_positions, f));
        let res = triangulator.y_monotone_triangulation(
            kernel, f,
            vertex_positions,
            output
        );
        assert_eq!(res, Ok(()));
    }
}

#[cfg(test)]
fn test_shape_with_holes(vertices: &[Vec2], separators: &[u16], angle: f32) {
    use tesselation::vertex_builder::{
        VertexBuffers, simple_vertex_builder,
    };


    use std::iter::FromIterator;
    let mut transformed_vertices: Vec<Vec2> = Vec::from_iter(vertices.iter().map(|v|{*v}));
    for ref mut v in &mut transformed_vertices[..] {
        // rotate all points around (0, 0).
        let cos = angle.cos();
        let sin = angle.sin();
        v[0] = v.x()*cos - v.y()*sin;
        v[1] = v.x()*sin + v.y()*cos;
    }

    println!("transformed_vertices: {:?}", transformed_vertices);

    let mut polygon = ComplexPolygon {
        main: Polygon::from_vertices(vertex_id_range(0, separators[0])),
        holes: Vec::new(),
    };

    let mut vertex_count = separators[0] as u16;
    for i in 1..separators.len() {
        let from = vertex_count as u16;
        let to = vertex_count + separators[i] as u16;
        polygon.holes.push(Polygon::from_vertices(ReverseIdRange::new(vertex_id_range(from, to))));
        vertex_count = to;
    }

    let vertex_positions = IdSlice::new(vertices);
    let mut ctx = DecompositionContext2::new();
    let mut diagonals = Diagonals::new();
    let res = ctx.y_monotone_polygon_decomposition(&polygon, vertex_positions, &mut diagonals);
    assert_eq!(res, Ok(()));

    let mut y_monotone_polygons = Vec::new();
    partition_polygon(&polygon, vertex_positions, &mut diagonals, &mut y_monotone_polygons);

    let mut triangulator = TriangulationContext2::new();
    let mut buffers: VertexBuffers<Vec2> = VertexBuffers::new();

    println!("\n\n -- There are {:?} monotone polygons", y_monotone_polygons.len());
    for poly in y_monotone_polygons {
        println!("\n\n -- Triangulating polygon with vertices {:?}", poly.vertices);
        let mut i = 0;
        for &p in &poly.vertices {
            println!("     -> point {} vertex {:?} position {:?}", i, p, vertex_positions[p].position());
            i += 1;
        }
        assert!(is_y_monotone_polygon(poly.view(), vertex_positions));
        let res = triangulator.y_monotone_triangulation(
            poly.view(),
            vertex_positions,
            &mut simple_vertex_builder(&mut buffers)
        );
        assert_eq!(res, Ok(()));
    }

    for n in 0 .. buffers.indices.len()/3 {
        println!(" ===> {} {} {}", buffers.indices[n*3], buffers.indices[n*3+1], buffers.indices[n*3+2]);
    }
}

#[cfg(test)]
fn test_shape(vertices: &[Vec2], angle: f32) {
    test_shape_with_holes(vertices, &[vertices.len() as u16], angle);
}

#[test]
fn test_triangulate() {
    let vertex_positions : &[&[Vec2]] = &[
        &[
            [-10.0, 5.0],
            [0.0, -5.0],
            [10.0, 5.0],
        ],
        &[
            [1.0, 2.0],
            [1.5, 3.0],
            [0.0, 4.0],
        ],
        &[
            [1.0, 2.0],
            [1.5, 3.0],
            [0.0, 4.0],
            [-1.0, 1.0],
        ],
        &[
            [0.0, 0.0],
            [3.0, 0.0],
            [2.0, 1.0],
            [3.0, 2.0],
            [2.0, 3.0],
            [0.0, 2.0],
            [1.0, 1.0],
        ],
        &[
            [0.0, 0.0],
            [1.0, 1.0],// <
            [2.0, 0.0],//  |
            [2.0, 4.0],//  |
            [1.0, 3.0],// <
            [0.0, 4.0],
        ],
        &[
            [0.0, 2.0],
            [1.0, 2.0],
            [0.0, 1.0],
            [2.0, 0.0],
            [3.0, 1.0],// 4
            [4.0, 0.0],
            [3.0, 2.0],
            [2.0, 1.0],// 7
            [3.0, 3.0],
            [2.0, 4.0]
        ],
        &[
            [0.0, 0.0],
            [1.0, 0.0],
            [2.0, 0.0],
            [3.0, 0.0],
            [3.0, 1.0],
            [3.0, 2.0],
            [3.0, 3.0],
            [2.0, 3.0],
            [1.0, 3.0],
            [0.0, 3.0],
            [0.0, 2.0],
            [0.0, 1.0],
        ],
    ];

    let mut angle = 0.0;
    while angle < 2.0*PI {
        for i in 0 .. vertex_positions.len() {
            println!("\n\n\n   -- shape {} angle {:?}", i, angle);
            test_shape(&vertex_positions[i][..], angle);
        }
        angle += 0.005;
    }
}

#[test]
fn test_triangulate_holes() {
    let vertex_positions : &[(&[Vec2], &[u16])] = &[
        (
            &[
                // outer
                [-11.0, 5.0],
                [0.0, -5.0],
                [10.0, 5.0],
                // hole
                [-5.0, 2.0],
                [0.0, -2.0],
                [4.0, 2.0],
            ],
            &[ 3, 3 ]
        ),
        (
            &[
                // outer
                [-10.0, -10.0],
                [ 10.0, -10.0],
                [ 10.0,  10.0],
                [-10.0,  10.0],
                // hole
                [-4.0, 2.0],
                [0.0, -2.0],
                [4.0, 2.0],
            ],
            &[ 4, 3 ]
        ),
        (
            &[
                // outer
                [-10.0, -10.0],
                [ 10.0, -10.0],
                [ 10.0,  10.0],
                [-10.0,  10.0],
                // hole 1
                [-8.0, -8.0],
                [-4.0, -8.0],
                [4.0, 8.0],
                [-8.0, 8.0],
                // hole 2
                [8.0, -8.0],
                [6.0, 7.0],
                [-2.0, -8.0],
            ],
            &[ 4, 4, 3 ]
        ),
        (
            &[
                // outer
                [0.0, 0.0],
                [1.0, 1.0],
                [2.0, 1.0],
                [3.0, 0.0],
                [4.0, 0.0],
                [5.0, 0.0],
                [3.0, 4.0],
                [1.0, 4.0],
                // hole 1
                [2.0, 2.0],
                [3.0, 2.0],
                [2.5, 3.0],
            ],
            &[8, 3]
        ),
    ];

    let mut angle = 0.0;
    while angle < 2.0*PI {
        for i in 0 .. vertex_positions.len() {
        let &(vertices, separators) = &vertex_positions[i];
            println!("\n\n\n   -- shape {} angle {:?}", i, angle);
            test_shape_with_holes(vertices, separators, angle);
        }
        angle += 0.005;
    }
}

#[test]
fn test_triangulate_degenerate() {
    let vertex_positions : &[&[Vec2]] = &[
        &[  // duplicate point
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 0.0],
        ],
        &[  // duplicate point
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
        ],
        &[  // duplicate point
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
        ],
        &[  // points in the same line
            [0.0, 0.0],
            [0.0, 1.0],
            [0.0, 2.0],
        ],
        &[  // points in the same line
            [0.0, 0.0],
            [0.0, 2.0],
            [0.0, 1.0],
        ],
        &[  // all points at the same place
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
        ],
        &[  // all points at the same place
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
        ],
        &[  // geometry comes back along a line on the x axis (zero-aera triangles)
            [0.0, 0.0],
            [2.0, 0.0],
            [1.0, 0.0],
            [0.0, 1.0],
        ],
        &[  // geometry comes back along a line on the y axis (zero-aera triangles)
            [0.0, 0.0],
            [0.0, 2.0],
            [0.0, 1.0],
            [-1.0, 0.0],
        ],
        &[  // a mix of the previous 2 cases
            [0.0, 0.0],
            [2.0, 0.0],
            [1.0, 0.0],
            [1.0, 2.0],
            [1.0, 1.0],
            [-1.0, 1.0],
            [0.0, 1.0],
            [0.0, -1.0],
        ],
    ];

    let mut angle = 0.0;
    while angle < 2.0*PI {
        for i in 0 .. vertex_positions.len() {
            println!("\n\n\n   -- shape {} angle {:?}", i, angle);
            test_shape(&vertex_positions[i][..], angle);
        }
        angle += 0.005;
    }
}

#[test]
#[ignore]
fn test_triangulate_failures() {
    // Test cases that are known to fail but we want to make work eventually.
    let vertex_positions : &[(&[Vec2], &[u16])] = &[
        // This path goes somthing like A,B,A,...
        (
            &[
                // outer
                [0.0, 0.0],
                [1.0, 1.0], // <--
                [2.0, 1.0],
                [3.0, 0.0],
                [4.0, 0.0],
                [5.0, 0.0],
                [3.0, 4.0],
                [1.0, 4.0],
                [1.0, 1.0], // <--
                // hole 1
                [2.0, 2.0],
                [3.0, 2.0],
                [2.5, 3.0],
            ],
            &[9, 3]
        ),
        // zero-area geometry shaped like a cross going back to the same position at the center
        (
            &[
                [1.0, 1.0],
                [2.0, 1.0],
                [1.0, 1.0],
                [2.0, 1.0],
                [1.0, 1.0],
                [0.0, 1.0],
                [1.0, 1.0],
                [1.0, 0.0],
            ],
            &[8]
        ),
        // Self-intersection
        (
            &[
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 1.0],
                [0.0, 1.0],
                [3.0, 0.0],
                [3.0, 1.0],
            ],
            &[6]
        ),
    ];

    for i in 0 .. vertex_positions.len() {
        println!("\n\n -- shape {:?}", i);
        let &(vertices, separators) = &vertex_positions[i];
        test_shape_with_holes(vertices, separators, 0.0);
    }
}
