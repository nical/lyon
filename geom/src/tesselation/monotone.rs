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
use half_edge::iterators::{Direction, DirectedEdgeCirculator};
use half_edge::vectors::{ Position2D, Vec2, X, Y, vec2_sub };

use tesselation::vertex_builder::{
    VertexBuffers, simple_vertex_builder, VertexBufferBuilder,
};

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

/// Angle between vectors v1 and v2 (oriented clockwise with y pointing downward).
///
/// (equivalent to counter-clockwise if y points upward)
///
/// ex: directed_angle([0,1], [1,0]) = 3/2 Pi rad = 270 deg
///     x       __
///   0-->     /  \
///  y|       |  x--> v2
///   v        \ |v1
///              v
pub fn directed_angle(v1: [f32; 2], v2: [f32; 2]) -> f32 {
    let a = (v2.y()).atan2(v2.x()) - (v1.y()).atan2(v1.x());
    return if a < 0.0 { a + 2.0 * PI } else { a };
}

fn get_vertex_type(prev: [f32; 2], current: [f32; 2], next: [f32; 2]) -> VertexType {
    // assuming clockwise vertex_positions winding order
    let interrior_angle = directed_angle(vec2_sub(prev, current), vec2_sub(next, current));

    // If the interrior angle is exactly 0 we'll have degenerate (invisible 0-area) triangles
    // which is yucks but we can live with it for the sake of being robust against degenerate
    // inputs. So special-case them so that they don't get considered as Merge ot Split vertices
    // otherwise there can be no monotone decomposition of a shape where all points are on the
    // same line.

    if current.y() > prev.y() && current.y() >= next.y() {
        if interrior_angle <= PI && interrior_angle != 0.0 {
            return VertexType::Merge;
        } else {
            return VertexType::End;
        }
    }

    if current.y() < prev.y() && current.y() <= next.y() {
        if interrior_angle <= PI && interrior_angle != 0.0 {
            return VertexType::Split;
        } else {
            return VertexType::Start;
        }
    }

    return if prev.y() < next.y() { VertexType::Right } else { VertexType::Left };
}


fn sweep_status_push<'l, Pos: Position2D>(
    kernel: &'l ConnectivityKernel,
    vertex_positions: IdSlice<'l, VertexId, Pos>,
    sweep: &'l mut Vec<EdgeId>,
    e: EdgeId
) {
    sweep.push(e);
    sweep.sort_by(|a, b| {
        vertex_positions[kernel[*a].vertex].y().partial_cmp(&vertex_positions[kernel[*b].vertex].y()).unwrap().reverse()
    });
}

fn connect<Faces: Write<FaceId>>(
    kernel: &mut ConnectivityKernel,
    mut a: EdgeId,
    mut b: EdgeId,
    new_faces: &mut Faces
) {
    let first_a = a;
    let first_b = b;
    debug_assert_eq!(kernel[a].face, kernel[b].face);

    // Look for a and b such that they share the same face.
    // TODO: Why would we need this already?
    //loop {
    //    let mut ok = false;
    //    loop {
    //        if kernel[a].face == kernel[b].face  {
    //            ok = true;
    //            break;
    //        }
    //        a = kernel.next_edge_id_around_vertex(a).unwrap();
    //        debug_assert_eq!(kernel[a].vertex, kernel[first_a].vertex);
    //        if a == first_a { break; }
    //    }
    //    if ok { break; }
    //    b = kernel.next_edge_id_around_vertex(b).unwrap();
    //    debug_assert_eq!(kernel[b].vertex, kernel[first_b].vertex);
    //    debug_assert!(b != first_b);
    //}

    let a_prev = kernel[a].prev;

    println!(" > connect {} {}",
        kernel[a_prev].vertex.to_index(),
        kernel[b].vertex.to_index()
    );
    if let Some(face) = kernel.connect_edges(a_prev, b) {
        new_faces.write(face);
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
    sweep_status_storage: Allocation,
    helper: HashMap<usize, (EdgeId, VertexType)>,
}

impl DecompositionContext {
    /// Constructor
    pub fn new() -> DecompositionContext {
        DecompositionContext {
            sorted_edges_storage: Allocation::empty(),
            sweep_status_storage: Allocation::empty(),
            helper: HashMap::new(),
        }
    }

    /// Constructor which pre-allocates memory
    pub fn with_capacity(edges: usize, sweep: usize, helpers: usize) -> DecompositionContext {
        let edges_vec: Vec<EdgeId> = Vec::with_capacity(edges);
        let sweep_vec: Vec<EdgeId> = Vec::with_capacity(sweep);
        DecompositionContext {
            sorted_edges_storage: vec::recycle(edges_vec),
            sweep_status_storage: vec::recycle(sweep_vec),
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
        swap(&mut self.sweep_status_storage, &mut storage);
        let mut sweep_status: Vec<EdgeId> = create_vec_from(storage);

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

        // sort indices by increasing y coordinate of the corresponding vertex
        sorted_edges.sort_by(|a, b| {
            let va = vertex_positions[kernel[*a].vertex].position();
            let vb = vertex_positions[kernel[*b].vertex].position();
            if va.y() > vb.y() { return Ordering::Greater; }
            if va.y() < vb.y() { return Ordering::Less; }
            if va.x() < vb.x() { return Ordering::Greater; }
            if va.x() > vb.x() { return Ordering::Less; }
            return Ordering::Equal;
        });

        for &e in &sorted_edges {
            let edge = kernel[e];
            let current_vertex = vertex_positions[edge.vertex].position();
            let previous_vertex = vertex_positions[kernel[edge.prev].vertex].position();
            let next_vertex = vertex_positions[kernel[edge.next].vertex].position();
            let vertex_type = get_vertex_type(previous_vertex, current_vertex, next_vertex);
            println!(" ** current vertex: {} edge {} pos {:?} type {:?}",
                edge.vertex.to_index(), e.to_index(), vertex_positions[edge.vertex].position(), vertex_type
            );
            match vertex_type {
                VertexType::Start => {
                    sweep_status_push(kernel, vertex_positions, &mut sweep_status, e);
                    self.helper.insert(e.to_index(), (e, VertexType::Start));
                }
                VertexType::End => {
                    if let Some(&(h, VertexType::Merge)) = self.helper.get(&edge.prev.to_index()) {
                        connect(kernel, e, h, new_faces);
                    }
                    sweep_status.retain(|item|{ *item != edge.prev });
                }
                VertexType::Split => {
                    for i in 0 .. sweep_status.len() {
                        if vertex_positions[kernel[sweep_status[i]].vertex].x() >= current_vertex.x() {
                            if let Some(&(helper_edge,_)) = self.helper.get(&sweep_status[i].to_index()) {
                                connect(kernel, e, helper_edge, new_faces);
                            }
                            self.helper.insert(sweep_status[i].to_index(), (e, VertexType::Split));
                            break;
                        }
                    }
                    sweep_status_push(kernel, vertex_positions, &mut sweep_status, e);
                    self.helper.insert(e.to_index(), (e, VertexType::Split));
                }
                VertexType::Merge => {
                    if let Some((h, VertexType::Merge)) = self.helper.remove(&edge.prev.to_index()) {
                        connect(kernel, e, h, new_faces);
                    }
                    for i in 0 .. sweep_status.len() {
                        if vertex_positions[kernel[sweep_status[i]].vertex].x() > current_vertex.x() {
                            if let Some((prev_helper, VertexType::Merge)) = self.helper.insert(
                                sweep_status[i].to_index(),
                                (e, VertexType::Merge)
                            ) {
                                connect(kernel, prev_helper, e, new_faces);
                            }
                            break;
                        }
                    }
                }
                VertexType::Left => {
                    for i in 0 .. sweep_status.len() {
                        if vertex_positions[kernel[sweep_status[i]].vertex].x() > current_vertex.x() {
                            if let Some((prev_helper, VertexType::Merge)) = self.helper.insert(
                                sweep_status[i].to_index(), (e, VertexType::Right)
                            ) {
                                connect(kernel, prev_helper, e, new_faces);
                            }
                            break;
                        }
                    }
                }
                VertexType::Right => {
                    if let Some((h, VertexType::Merge)) = self.helper.remove(&edge.prev.to_index()) {
                        connect(kernel, e, h, new_faces);
                    }
                    sweep_status.retain(|item|{ *item != edge.prev });
                    sweep_status_push(kernel, vertex_positions, &mut sweep_status, e);
                    self.helper.insert(e.to_index(), (e, VertexType::Left));
                }
            }
        }

        // Keep the buffers to avoid reallocating it next time, if possible.
        self.sweep_status_storage = vec::recycle(sweep_status);
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
                    println!(" vertex {}  edge {}", kernel[e].vertex.to_index(), e.to_index());
                }
                println!(" ---");
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
    use std::iter::FromIterator;
    let mut transformed_vertices: Vec<Vec2> = Vec::from_iter(vertices.iter().map(|v|{*v}));
    for ref mut v in &mut transformed_vertices[..] {
        // rotate all points around (0, 0).
        let cos = angle.cos();
        let sin = angle.sin();
        v[0] = v[0]*cos - v[1]*sin;
        v[1] = v[0]*sin + v[1]*cos;
    }

    let n_vertices = separators[0] as u16;

    let mut kernel = ConnectivityKernel::new();

    let f1 = kernel.add_face();

    kernel.add_loop(vertex_range(0, n_vertices), Some(f1), None);

    let mut vertex_count = n_vertices;
    for i in 1 .. separators.len() {
        kernel.add_hole(f1, vertex_range(vertex_count, separators[i]));
        vertex_count += separators[i];
    }

    let mut buffers: VertexBuffers<Vec2> = VertexBuffers::new();
    triangulate_faces(
        &mut kernel, &[f1], &transformed_vertices[..],
        &mut simple_vertex_builder(&mut buffers)
    );
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

    for i in 0 .. vertex_positions.len() {
        println!("\n\n -- shape {:?}", i);
        let mut angle = 0.0;
        while angle < 2.0*PI {
            println!("   -- angle {:?}", angle);
            test_shape(&vertex_positions[i][..], angle);
            angle += 0.005;
        }
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

    for i in 0 .. vertex_positions.len() {
        println!("\n\n -- shape {:?}", i);
        let &(vertices, separators) = &vertex_positions[i];

        let mut angle = 0.0;
        while angle < 2.0*PI {
            println!("   -- angle {:?}", angle);
            test_shape_with_holes(vertices, separators, angle);
            angle += 0.005;
        }
    }
}

#[test]
fn test_triangulate_degenerate() {
    let mut vertex_positions : &[&[Vec2]] = &[
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

    for i in 0 .. vertex_positions.len() {
        println!("\n\n -- shape {:?}", i);
        let mut angle = 0.0;
        while angle < 2.0*PI {
            println!("   -- angle {:?}", angle);
            test_shape(&vertex_positions[i][..], angle);
            angle += 0.005;
        }
    }
}

#[test]
#[ignore]
fn test_triangulate_failures() {
    // Test cases that are know to fail but we want to make work eventually.
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
