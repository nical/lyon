// Implementation inspired by Computational Geometry, Algorithms And Applications 3rd edition.
//
// Note that a lot of the code/comments/names in this module assume a coordinate
// system where y pointing downwards

use halfedge::*;
use id_vector::*;
use iterators::{Direction, DirectedEdgeCirculator};
use math::vector::*;
use std::num::Float;
use std::cmp::{Ordering, PartialOrd};
use std::collections::HashMap;
use std::mem::swap;
use std::fmt::Debug;
use super::mem::VecStorage;

#[cfg(test)]
use math::units::world;

#[derive(Debug, Copy, Clone)]
enum VertexType {
    Start,
    End,
    Split,
    Merge,
    Left,
    Right,
}

#[derive(Copy, Debug, PartialEq)]
pub enum DecompositionError {
    OpenPath,
    InvertedWindingOrder,
    MissingFace,
}

#[derive(Copy, Debug, PartialEq)]
pub enum TriangulationError {
    NotMonotone,
    InvalidPath,
    MissingFace,
}

/// Angle between v1 and v2 (oriented clockwise with y pointing downward)
/// (equivalent to counter-clockwise if y points upward)
/// ex: directed_angle([0,1], [1,0]) = 3/2 Pi rad = 270 deg
///     x       __
///   0-->     /  \
///  y|       |  x--> v2
///   v        \ |v1   
///              v
pub fn directed_angle<T>(v1: Vector2D<T>, v2: Vector2D<T>) -> f32 {
    let a = (v2.y).atan2(v2.x) - (v1.y).atan2(v1.x);
    return if a < 0.0 { a + 2.0 * PI } else { a };
}

fn get_vertex_type<T: Copy>(prev: Vector2D<T>, current: Vector2D<T>, next: Vector2D<T>) -> VertexType {
    // assuming clockwise vertex_positions winding order
    let interrior_angle = directed_angle(prev - current, next - current);

    if current.y > prev.y && current.y >= next.y {
        if interrior_angle <= PI {
            return VertexType::Merge;
        } else {
            return VertexType::End;
        }
    }

    if current.y < prev.y && current.y <= next.y {
        if interrior_angle <= PI {
            return VertexType::Split;
        } else {
            return VertexType::Start;
        }
    }

    return if prev.y < next.y { VertexType::Right } else { VertexType::Left };
}


fn sweep_status_push<'l, T:Copy>(
    kernel: &'l ConnectivityKernel,
    vertex_positions: IdSlice<'l, Vector2D<T>, Vertex_>,
    sweep: &'l mut Vec<EdgeId>,
    e: EdgeId
) {
    sweep.push(e);
    sweep.sort_by(|a, b| {
        vertex_positions[kernel[*a].vertex].y.partial_cmp(&vertex_positions[kernel[*b].vertex].y).unwrap().reverse()
    });    
}

fn split_face(
    kernel: &mut ConnectivityKernel,
    mut a: EdgeId,
    mut b: EdgeId,
    new_faces: &mut Vec<FaceId>
) {
    let first_a = a;
    let first_b = b;
    let mut ok = false;
    loop {
        loop {
            if kernel[a].face == kernel[b].face  {
                ok = true;
                break;
            }
            a = kernel.next_edge_around_vertex(a);
            if a == first_a { break; }
        }
        if ok { break; }
        b = kernel.next_edge_around_vertex(b);
        debug_assert!(b != first_b);
    }

    if let Some(new_face) = kernel.split_face(a, b) {
        new_faces.push(new_face);
    }
}


pub struct DecompositionContext {
    sorted_edges_storage: Option<VecStorage>,
    // list of edges that intercept the sweep line, sorted by increasing x coordinate
    sweep_status_storage: Option<VecStorage>,
    helper: HashMap<usize, (EdgeId, VertexType)>,
}

impl DecompositionContext {
    pub fn new() -> DecompositionContext {
        DecompositionContext {
            sorted_edges_storage: None,
            sweep_status_storage: None,
            helper: HashMap::new(),
        }
    }

    pub fn with_capacity(edges: usize, sweep: usize, helpers: usize) -> DecompositionContext {
        let edges_vec: Vec<EdgeId> = Vec::with_capacity(edges);
        let sweep_vec: Vec<EdgeId> = Vec::with_capacity(sweep);
        DecompositionContext {
            sorted_edges_storage: Some(VecStorage::from_vec(edges_vec)),
            sweep_status_storage: Some(VecStorage::from_vec(sweep_vec)),
            helper: HashMap::with_capacity(helpers),
        }
    }

    pub fn y_monotone_decomposition<'l, T: Copy+Debug>(
        &mut self,
        kernel: &'l mut ConnectivityKernel,
        face_id: FaceId,
        vertex_positions: IdSlice<'l, Vector2D<T>, Vertex_>,
        new_faces: &'l mut Vec<FaceId>
    ) -> Result<(), DecompositionError> {
        self.helper.clear();

        if !kernel.contain_face(face_id) {
            return Err(DecompositionError::MissingFace);
        }

        let mut temp_edges_storage = None;
        swap(&mut self.sorted_edges_storage, &mut temp_edges_storage);
        let mut sorted_edges: Vec<EdgeId> = match temp_edges_storage {
            Some(storage) => { storage.into_vec::<EdgeId>() }
            None => { Vec::with_capacity(64) }
        };
        sorted_edges.extend(kernel.walk_edges_around_face(face_id));

        let mut temp_sweep_storage = None;
        swap(&mut self.sweep_status_storage, &mut temp_sweep_storage);
        let mut sweep_status = match temp_sweep_storage {
            Some(storage) => { storage.into_vec::<EdgeId>() }
            None => { Vec::with_capacity(16) }
        };

        // also add holes in the sorted edge list
        for inner in &kernel.face(face_id).inner_edges {
            for e in kernel.walk_edges(*inner) {
                sorted_edges.push(e);
            }
        }

        // sort indices by increasing y coordinate of the corresponding vertex
        sorted_edges.sort_by(|a, b| {
            if vertex_positions[kernel[*a].vertex].y > vertex_positions[kernel[*b].vertex].y {
                return Ordering::Greater;
            }
            if vertex_positions[kernel[*a].vertex].y < vertex_positions[kernel[*b].vertex].y {
                return Ordering::Less;
            }
            if vertex_positions[kernel[*a].vertex].x < vertex_positions[kernel[*b].vertex].x {
                return Ordering::Greater;
            }
            if vertex_positions[kernel[*a].vertex].x > vertex_positions[kernel[*b].vertex].x {
                return Ordering::Less;
            }
            return Ordering::Equal;
        });

        for &e in &sorted_edges {
            let edge = kernel[e];
            let current_vertex = vertex_positions[edge.vertex];
            let previous_vertex = vertex_positions[kernel[edge.prev].vertex];
            let next_vertex = vertex_positions[kernel[edge.next].vertex];
            let vertex_type = get_vertex_type(previous_vertex, current_vertex, next_vertex);

            match vertex_type {
                VertexType::Start => {
                    sweep_status_push(kernel, vertex_positions, &mut sweep_status, e);
                    self.helper.insert(e.as_index(), (e, VertexType::Start));
                }
                VertexType::End => {
                    if let Some(&(h, VertexType::Merge)) = self.helper.get(&edge.prev.as_index()) {
                        split_face(kernel, edge.prev, h, new_faces);
                    } 
                    sweep_status.retain(|item|{ *item != edge.prev });
                }
                VertexType::Split => {
                    for i in 0 .. sweep_status.len() {
                        if vertex_positions[kernel[sweep_status[i]].vertex].x >= current_vertex.x {
                            if let Some(&(helper_edge,_)) = self.helper.get(&sweep_status[i].as_index()) {
                                split_face(kernel, e, helper_edge, new_faces);
                            }
                            self.helper.insert(sweep_status[i].as_index(), (e, VertexType::Split));
                            break;
                        }
                    }
                    sweep_status_push(kernel, vertex_positions, &mut sweep_status, e);
                    self.helper.insert(e.as_index(), (e, VertexType::Split));
                }
                VertexType::Merge => {
                    if let Some((h, VertexType::Merge)) = self.helper.remove(&edge.prev.as_index()) {
                        split_face(kernel, e, h, new_faces);
                    }
                    for i in 0 .. sweep_status.len() {
                        if vertex_positions[kernel[sweep_status[i]].vertex].x > current_vertex.x {
                            if let Some((prev_helper, VertexType::Merge)) = self.helper.insert(
                                sweep_status[i].as_index(),
                                (e, VertexType::Merge)
                            ) {
                                split_face(kernel, prev_helper, e, new_faces);
                            }
                            break;
                        }
                    }
                }
                VertexType::Left => {
                    for i in 0 .. sweep_status.len() {
                        if vertex_positions[kernel[sweep_status[i]].vertex].x > current_vertex.x {
                            if let Some((prev_helper, VertexType::Merge)) = self.helper.insert(
                                sweep_status[i].as_index(), (e, VertexType::Right)
                            ) {
                                split_face(kernel, prev_helper, e, new_faces);
                            }
                            break;
                        }
                    }
                }
                VertexType::Right => {
                    if let Some((h, VertexType::Merge)) = self.helper.remove(&edge.prev.as_index()) {
                        split_face(kernel, e, h, new_faces);
                    }
                    sweep_status.retain(|item|{ *item != edge.prev });
                    sweep_status_push(kernel, vertex_positions, &mut sweep_status, e);
                    self.helper.insert(e.as_index(), (e, VertexType::Left));
                }
            }
        }

        // Keep the buffers to avoid reallocating it next time, if possible.
        self.sweep_status_storage = Some(VecStorage::from_vec(sweep_status));
        self.sorted_edges_storage = Some(VecStorage::from_vec(sorted_edges));

        return Ok(());
    }
}


pub fn is_y_monotone<'l, T:Copy+Debug>(
    kernel: &'l ConnectivityKernel,
    vertex_positions: IdSlice<'l, Vector2D<T>, Vertex_>,
    face: FaceId
) -> bool {
    for e in kernel.walk_edges_around_face(face) {
        let edge = kernel[e];
        let current_vertex = vertex_positions[edge.vertex];
        let previous_vertex = vertex_positions[kernel[edge.prev].vertex];
        let next_vertex = vertex_positions[kernel[edge.next].vertex];
        match get_vertex_type(previous_vertex, current_vertex, next_vertex) {
            VertexType::Split | VertexType::Merge => {
                //println!("not y monotone because of vertices {} {} {} edge {} {} {}",
                //    kernel[edge.prev].vertex.as_index(), edge.vertex.as_index(), kernel[edge.next].vertex.as_index(), 
                //    edge.prev.as_index(), e.as_index(), edge.next.as_index());
                return false;
            }
            _ => {}
        }
    }
    return true;
}

// TODO[nical] there's probably a generic Writer thingy in std
pub trait TriangleStream {
    fn write(&mut self, a: VertexId, b: VertexId, c: VertexId);
    fn count(&self) -> usize;
}

pub struct SliceTriangleStream<'l> {
    indices: &'l mut[u16],
    offset: usize,
}

impl<'l> TriangleStream for SliceTriangleStream<'l> {
    fn write(&mut self, a: VertexId, b: VertexId, c: VertexId) {
        debug_assert!(a != b);
        debug_assert!(b != c);
        debug_assert!(c != a);
        self.indices[self.offset] = a.as_index() as u16;
        self.indices[self.offset+1] = b.as_index() as u16;
        self.indices[self.offset+2] = c.as_index() as u16;
        self.offset += 3;
    }

    fn count(&self) -> usize { self.offset as usize / 3 }
}

impl<'l> SliceTriangleStream<'l> {
    pub fn new(buffer: &'l mut[u16]) -> SliceTriangleStream {
        SliceTriangleStream {
            indices: buffer,
            offset: 0,
        }
    }
}

struct TriangulationContext {
    vertex_stack_storage: Option<VecStorage>,
}

impl TriangulationContext {
    pub fn new() -> TriangulationContext { TriangulationContext { vertex_stack_storage: None } }

    // Returns the number of indices added
    pub fn y_monotone_triangulation<'l, T: Copy+Debug, Triangles: TriangleStream>(
        &mut self,
        kernel: &'l ConnectivityKernel,
        face_id: FaceId,
        vertex_positions: IdSlice<'l, Vector2D<T>, Vertex_>,
        triangles: &'l mut Triangles,
    ) -> Result<(), TriangulationError> {
        if !kernel.contain_face(face_id) {
            return Err(TriangulationError::MissingFace);
        }

        let mut up = DirectedEdgeCirculator::new(kernel, kernel[face_id].first_edge, Direction::Forward);
        let mut down = up.clone();
        loop {
            down = down.next();
            if vertex_positions[up.vertex_id()].y != vertex_positions[down.vertex_id()].y {
                break;
            }
        }

        if vertex_positions[up.vertex_id()].y < vertex_positions[down.vertex_id()].y {
            up.set_direction(Direction::Backward);
        } else {
            down.set_direction(Direction::Backward);
        }

        // find the bottom-most vertex (with the highest y value)
        let mut big_y = vertex_positions[down.vertex_id()].y;
        loop {
            debug_assert_eq!(down.face_id(), face_id);
            down = down.next();
            let new_y = vertex_positions[down.vertex_id()].y;
            if new_y < big_y {
                down = down.prev();
                break;
            }
            big_y = new_y;
        }

        // find the top-most vertex (with the smallest y value)
        let mut small_y = vertex_positions[up.vertex_id()].y;
        loop {
            debug_assert_eq!(up.face_id(), face_id);
            up = up.next();
            let new_y = vertex_positions[up.vertex_id()].y;
            if new_y > small_y {
                up = up.prev();
                break;
            }
            small_y = new_y;
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

        if vertex_positions[m.vertex_id()].y > vertex_positions[o.vertex_id()].y {
            swap(&mut m, &mut o);
        }

        m = m.prev();
        // previous
        let mut p = m;

        // vertices already visited, waiting to be connected
        let mut temp_storage = None;
        swap(&mut temp_storage, &mut self.vertex_stack_storage);
        let mut vertex_stack: Vec<DirectedEdgeCirculator> = match temp_storage {
            Some(storage) => { storage.into_vec::<DirectedEdgeCirculator>() }
            None => { Vec::with_capacity(16) }
        };

        let initial_triangle_count = triangles.count();
        let mut i: i32 = 0;
        loop {
            // walk edges from top to bottom, alternating between the left and 
            // right chains. The chain we are currently iterating over is the
            // main chain (m) and the other one the opposite chain (o).
            // p is the previous iteration, regardless of whcih chain it is on.
            if vertex_positions[m.vertex_id()].y > vertex_positions[o.vertex_id()].y || m == down {
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

                        triangles.write(id_opp, id_1, id_2);
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

                        let v1 = vertex_positions[id_1];
                        let v2 = vertex_positions[id_2];
                        let v3 = vertex_positions[id_3];
                        if directed_angle(v1 - v2, v3 - v2) > PI {
                            triangles.write(id_1, id_2, id_3);

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
            debug_assert!(vertex_positions[m.vertex_id()].y >= vertex_positions[p.vertex_id()].y);
        }
        let num_triangles = triangles.count() - initial_triangle_count;
        debug_assert_eq!(num_triangles, kernel.count_edges_around_face(face_id) as usize - 2);

        // Keep the buffer to avoid reallocating it next time, if possible.
        self.vertex_stack_storage = Some(VecStorage::from_vec(vertex_stack));
        return Ok(());
    }
}

pub fn triangulate_faces<T:Copy+Debug>(
    kernel: &mut ConnectivityKernel,
    faces: &[FaceId],
    vertices: &[Vector2D<T>],
    indices: &mut[u16]
) -> usize {
    let mut new_faces: Vec<FaceId> = vec![];
    for &f in faces {
        new_faces.push(f);
    }
    let vertices_attr = IdSlice::new(vertices);
    let mut ctx = DecompositionContext::new();
    for f in faces {
        let res = ctx.y_monotone_decomposition(kernel, *f, vertices_attr, &mut new_faces);
        assert_eq!(res, Ok(()));
    }

    let mut triangles = SliceTriangleStream::new(&mut indices[..]);
    let mut triangulator = TriangulationContext::new();
    for f in new_faces {
        debug_assert!(is_y_monotone(kernel, vertices_attr, f));
        let res = triangulator.y_monotone_triangulation(
            kernel, f,
            vertices_attr,
            &mut triangles
        );
        assert_eq!(res, Ok(()));
    }

    return triangles.count() * 3;
}

#[test]
fn test_triangulate() {
    let vertex_positionss : &[&[world::Vec2]] = &[
        &[
            world::vec2(-10.0, 5.0),
            world::vec2(0.0, -5.0),
            world::vec2(10.0, 5.0),
        ],
        &[
            world::vec2(1.0, 2.0),
            world::vec2(1.5, 3.0),
            world::vec2(0.0, 4.0),
        ],
        &[
            world::vec2(1.0, 2.0),
            world::vec2(1.5, 3.0),
            world::vec2(0.0, 4.0),
            world::vec2(-1.0, 1.0),
        ],
        &[
            world::vec2(0.0, 0.0),
            world::vec2(3.0, 0.0),
            world::vec2(2.0, 1.0),
            world::vec2(3.0, 2.0),
            world::vec2(2.0, 3.0),
            world::vec2(0.0, 2.0),
            world::vec2(1.0, 1.0),
        ],
        &[
            world::vec2(0.0, 0.0),
            world::vec2(1.0, 1.0),// <
            world::vec2(2.0, 0.0),//  |
            world::vec2(2.0, 4.0),//  |
            world::vec2(1.0, 3.0),// <
            world::vec2(0.0, 4.0),
        ],
        &[
            world::vec2(0.0, 2.0),
            world::vec2(1.0, 2.0),
            world::vec2(0.0, 1.0),
            world::vec2(2.0, 0.0),
            world::vec2(3.0, 1.0),// 4
            world::vec2(4.0, 0.0),
            world::vec2(3.0, 2.0),
            world::vec2(2.0, 1.0),// 7
            world::vec2(3.0, 3.0),
            world::vec2(2.0, 4.0)
        ],
        &[
            world::vec2(0.0, 0.0),
            world::vec2(1.0, 0.0),
            world::vec2(2.0, 0.0),
            world::vec2(3.0, 0.0),
            world::vec2(3.0, 1.0),
            world::vec2(3.0, 2.0),
            world::vec2(3.0, 3.0),
            world::vec2(2.0, 3.0),
            world::vec2(1.0, 3.0),
            world::vec2(0.0, 3.0),
            world::vec2(0.0, 2.0),
            world::vec2(0.0, 1.0),
        ],
    ];

    let indices = &mut [0 as u16; 1024];
    for i in 0 .. vertex_positionss.len() {
        println!("\n\n -- vertex_positions {:?}", i);
        let mut kernel = ConnectivityKernel::from_loop(vertex_positionss[i].len() as u16);
        let main_face = kernel.first_face();
        let n_indices = triangulate_faces(&mut kernel, &[main_face], &vertex_positionss[i][..], indices);
        for n in 0 .. n_indices/3 {
            println!(" ===> {} {} {}", indices[n*3], indices[n*3+1], indices[n*3+2] );
        }
    }
}

#[test]
fn test_triangulate_holes() {
    let vertex_positionss : &[(&[world::Vec2], &[u16])] = &[
        (
            &[
                // outer
                world::vec2(-11.0, 5.0),
                world::vec2(0.0, -5.0),
                world::vec2(10.0, 5.0),
                // hole
                world::vec2(4.0, 2.0),
                world::vec2(0.0, -2.0),
                world::vec2(-5.0, 2.0),
            ],
            &[ 3, 3 ]
        ),
        (
            &[
                // outer
                world::vec2(-10.0, -10.0),
                world::vec2( 10.0, -10.0),
                world::vec2( 10.0,  10.0),
                world::vec2(-10.0,  10.0),
                // hole
                world::vec2(4.0, 2.0),
                world::vec2(0.0, -2.0),
                world::vec2(-4.0, 2.0),
            ],
            &[ 4, 3 ]
        ),
        (
            &[
                // outer
                world::vec2(-10.0, -10.0),
                world::vec2( 10.0, -10.0),
                world::vec2( 10.0,  10.0),
                world::vec2(-10.0,  10.0),
                // hole 1
                world::vec2(-8.0, 8.0),
                world::vec2(4.0, 8.0),
                world::vec2(-4.0, -8.0),
                world::vec2(-8.0, -8.0),
                // hole 2
                world::vec2(-2.0, -8.0),
                world::vec2(6.0, 7.0),
                world::vec2(8.0, -8.0),
            ],
            &[ 4, 4, 3 ]
        ),
    ];

    let indices = &mut [0 as u16; 1024];
    for i in 0 .. vertex_positionss.len() {
        println!("\n\n -- vertex_positions {:?}", i);
        let &(vertices, separators) = &vertex_positionss[i];

        let mut kernel = ConnectivityKernel::from_loop(separators[0]);
        let main_face = kernel.first_face();
        for i in 1 .. separators.len() {
            kernel.add_hole(main_face, separators[i]);
        }

        let n_indices = triangulate_faces(&mut kernel, &[main_face], vertices, indices);
        for n in 0 .. n_indices/3 {
            println!(" ===> {} {} {}", indices[n*3], indices[n*3+1], indices[n*3+2] );
        }
    }
}