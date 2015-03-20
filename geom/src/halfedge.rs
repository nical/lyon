use std::ops;
use std::u16;

use id_vector::IdVector;
use traits::*;
use vodk_math::vector::{ Vector2D, Vector3D, Vector4D };

use iterators::{
    FaceEdgeIterator, ReverseFaceEdgeIterator, DirectedEdgeCirculator,
    IdRangeIterator, Direction
};

pub type Index = u16;
use std::marker::PhantomData;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Id<T> {
    handle: Index,
    _marker: PhantomData<T>
}
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Vertex_;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Edge_;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Face_;

pub type VertexId = Id<Vertex_>;
pub type EdgeId = Id<Edge_>;
pub type FaceId = Id<Face_>;

impl<T> Id<T> {
    /// Returns true if the id is not the special value.
    #[inline]
    pub fn is_valid(self) -> bool { self.handle != u16::MAX }

    /// Returns an usize for use with standard arrays.
    #[inline]
    pub fn as_index(self) -> usize { self.handle as usize }

    /// Create an id from an usize index.
    #[inline]
    pub fn from_usize(idx: usize) -> Id<T> { Id { handle: idx as u16, _marker: PhantomData } }
}

/// Id with a special value, not referencing any edge.

#[inline]
pub fn no_edge() -> EdgeId { edge_id(u16::MAX) }

/// Id with a special value, not referencing any face.
#[inline]
pub fn no_face() -> FaceId { face_id(u16::MAX) }

/// Id with a special value, not referencing any vertex.
#[inline]
pub fn no_vertex() -> VertexId { vertex_id(u16::MAX) }

/// Create an EdgeId from an index (the offset in the ConnectivityKernel's half edge vector)
#[inline]
pub fn edge_id(index: Index) -> EdgeId { EdgeId { handle: index, _marker: PhantomData } }

/// Create a FaceId from an index (the offset in the ConnectivityKernel's face vector)
#[inline]
pub fn face_id(index: Index) -> FaceId { FaceId { handle: index, _marker: PhantomData } }

/// Create a VertexId from an index (the offset in the ConnectivityKernel's vertex vector)
#[inline]
pub fn vertex_id(index: Index) -> VertexId { VertexId { handle: index, _marker: PhantomData } }

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct IdRange<T> {
    pub first: Id<T>,
    pub count: Index,
}

impl<T: Copy> IdRange<T> {
    pub fn iter(self) -> IdRangeIterator<T> {
        return IdRangeIterator::new(self);
    }

    pub fn get(self, i: u16) -> Id<T> {
        debug_assert!(i < self.count);
        return Id { handle: self.first.handle + i, _marker: PhantomData };
    }
}

/// A range of Id pointing to contiguous vertices.
pub type VertexIdRange = IdRange<Vertex_>;

/// A range of Id pointing to contiguous half edges.
pub type EdgeIdRange = IdRange<Edge_>;

/// A range of Id pointing to contiguous faces.
pub type FaceIdRange = IdRange<Face_>;

/// The structure holding the data specific to each half edge.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HalfEdgeData {
    pub next: EdgeId, // next half edge around the face
    pub prev: EdgeId, // previous half edge around the face
    pub vertex: VertexId, // vertex this edge origins from
    pub opposite: EdgeId,
    pub face: FaceId,
}

/// The structure holding the data specific to each face.
#[derive(Clone, Debug, PartialEq)]
pub struct FaceData {
    pub first_edge: EdgeId,
    pub inner_edges: Vec<EdgeId>,
}

/// The structure holding the data specific to each vertex.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct VertexData {
    pub first_edge: EdgeId,
}

/// The data structure that contains a mesh's connectivity information
///
/// It does not contain other attributes such as positions. Use IdVector for that.
pub struct ConnectivityKernel {
    edges: Vec<HalfEdgeData>,
    vertices: Vec<VertexData>,
    faces: Vec<FaceData>
}

impl ConnectivityKernel {

    /// VertexData getter. You can also use the indexing operator.
    pub fn vertex(&self, id: VertexId) -> &VertexData {
        debug_assert!(id.is_valid());
        &self.vertices[id.handle as usize]
    }

    /// VertexData mutable getter. You can also use the indexing operator.
    fn vertex_mut(&mut self, id: VertexId) -> &mut VertexData {
        debug_assert!(id.is_valid());
        &mut self.vertices[id.handle as usize]
    }

    /// Face getter. You can also use the indexing operator.
    pub fn face(&self, id: FaceId) -> &FaceData {
        debug_assert!(id.is_valid());
        &self.faces[id.handle as usize]
    }

    /// Face mutable getter. You can also use the indexing operator.
    fn face_mut(&mut self, id: FaceId) -> &mut FaceData {
        debug_assert!(id.is_valid());
        &mut self.faces[id.handle as usize]
    }

    /// Half edge getter. You can also use the indexing operator.
    pub fn edge(&self, id: EdgeId) -> &HalfEdgeData {
        debug_assert!(id.is_valid());
        &self.edges[id.handle as usize]
    }

    /// Half edge mutable getter. You can also use the indexing operator.
    fn edge_mut(&mut self, id: EdgeId) -> &mut HalfEdgeData {
        debug_assert!(id.is_valid());
        &mut self.edges[id.handle as usize]
    }

    pub fn edges(&self) -> &[HalfEdgeData] { &self.edges[..] }

    pub fn faces(&self) -> &[FaceData] { &self.faces[..] }

    pub fn vertices(&self) -> &[VertexData] { &self.vertices[..] }

    pub fn first_edge(&self) -> EdgeId { edge_id(0) }

    pub fn first_face(&self) -> FaceId { face_id(0) }

    pub fn first_vertex(&self) -> VertexId { vertex_id(0) }

    pub fn contains_egde(&self, id: EdgeId) -> bool { id.as_index() < self.edges.len() }

    pub fn contains_face(&self, id: FaceId) -> bool { id.as_index() < self.faces.len() }

    pub fn contains_vertex(&self, id: VertexId) -> bool { id.as_index() < self.vertices.len() }

    pub fn walk_edges_around_face<'l>(&'l self, id: FaceId) -> FaceEdgeIterator<'l> {
        let edge = self.face(id).first_edge;
        let prev = self.edge(edge).prev;
        FaceEdgeIterator::new(self, edge, prev)
    }

    pub fn walk_edges<'l>(&'l self, first: EdgeId) -> FaceEdgeIterator<'l> {
        FaceEdgeIterator::new(self, first, self.edge(first).prev)
    }

    pub fn walk_edges_around_face_reverse<'l>(&'l self, id: FaceId) -> ReverseFaceEdgeIterator<'l> {
        let edge = self.face(id).first_edge;
        ReverseFaceEdgeIterator::new(self, edge, self.edge(edge).next)
    }

    pub fn next_edge_around_vertex(&self, id: EdgeId) -> EdgeId {
        return self.edge(self.edge(id).opposite).next;
    }

    pub fn debug_assert_edge_invariants(&self, id: EdgeId) {
        debug_assert_eq!(self.edge(self.edge(id).opposite).opposite, id);
        debug_assert_eq!(self.edge(self.edge(id).next).prev, id);
        debug_assert_eq!(self.edge(self.edge(id).prev).next, id);
        debug_assert_eq!(
            self.edge(id).vertex,
            self.edge(self.edge(self.edge(id).opposite).next).vertex
        );
        debug_assert_eq!(self.edge(id).face, self.edge(self.edge(id).next).face);
        debug_assert_eq!(self.edge(id).face, self.edge(self.edge(id).prev).face);
    }

    pub fn debug_assert_face_invariants(&self, face: FaceId) {
        for e in self.walk_edges_around_face(face) {
            self.debug_assert_edge_invariants(e);
        }
    }

    /// Insert a vertex on this edge and return the id of the new vertex
    pub fn split_edge(&mut self, id: EdgeId) -> VertexId {
        // from:
        //     a ---[id]----------------------------------------> b
        //     a <----------------------------------[opposite]--- b
        // to:
        //     a ---[id]------------> new_vertex ---[new_edge]--> b
        //     a <--[new_opposite]--- new_vertex <--[opposite]--- b

        let new_vertex = vertex_id(self.vertices.len() as Index);
        let new_edge = edge_id(self.vertices.len() as Index);
        let new_opposite = edge_id(self.vertices.len() as Index + 1);

        self.vertices.push(VertexData {
            first_edge: new_edge,
        });

        // new_edge
        let edge = *self.edge(id);
        self.edges.push(HalfEdgeData {
            vertex: edge.vertex,
            opposite: edge.opposite,
            face: edge.face,
            next: edge.next,
            prev: id,
        });

        // new_opposite
        let opposite = *self.edge(edge.opposite);
        self.edges.push(HalfEdgeData {
            vertex: opposite.vertex,
            opposite: id,
            face: opposite.face,
            next: opposite.next,
            prev: edge.opposite,
        });

        // patch up existing edges
        self.edge_mut(id).vertex = new_vertex;
        self.edge_mut(id).next = new_edge;
        self.edge_mut(edge.opposite).vertex = new_vertex;
        self.edge_mut(edge.opposite).next = new_opposite;

        return new_vertex;
    }

    /// Split a face in two along the vertices that a_prev and b_prev point to
    pub fn split_face(&mut self, a_next: EdgeId, b_next: EdgeId) -> Option<FaceId> {
        //
        // a_prev--> va -a_next->
        //          | ^
        //   f1     n |
        //          | |
        //          | o     f2
        //          v |
        // <-b_next- vb <--b_prev
        // ______________________
        //
        // f1: original_face
        // f2: new_face
        // n: new_edge
        // o: new_opposite_edge

        let original_face = self.edge(a_next).face;

        let a_opposite_face = self.edge(self.edge(a_next).opposite).face;
        let b_opposite_face = self.edge(self.edge(b_next).opposite).face;
        let mut add_face = true;

        for i in 0 .. self.face(original_face).inner_edges.len() {
            let opposite = self.edge(self.edge(self.face(original_face).inner_edges[i]).opposite).face;
            if opposite == a_opposite_face || opposite == b_opposite_face {
                add_face = false;
                // remove the hole from this face
                self.face_mut(original_face).inner_edges.remove(i);
                break;
            }
        }

        let a_prev = self.edge(a_next).prev;
        let b_prev = self.edge(b_next).prev;

        self.debug_assert_face_invariants(original_face);

        debug_assert_eq!(original_face, self.edge(b_prev).face);
        debug_assert!(self.edge(a_next).next != b_next);
        debug_assert!(a_prev != b_next);

        let va = self.edge(a_next).vertex;
        let vb = self.edge(b_next).vertex;
        let new_edge = edge_id(self.edges.len() as Index); // va -> vb
        let new_opposite_edge = edge_id(self.edges.len() as Index + 1); // vb -> va

        self.faces.push(FaceData {
            first_edge: new_opposite_edge,
            inner_edges: vec![],
        });

        let opposite_face = if add_face { face_id(self.faces.len() as Index - 1) }
                            else { original_face };

        // new_edge
        self.edges.push(HalfEdgeData {
            next: b_next,
            prev: a_prev,
            opposite: new_opposite_edge,
            face: original_face,
            vertex: va,
        });

        // new_opposite_edge
        self.edges.push(HalfEdgeData {
            next: a_next,
            prev: b_prev,
            opposite: new_edge,
            face: opposite_face,
            vertex: vb,
        });

        self.edge_mut(a_prev).next = new_edge;
        self.edge_mut(a_next).prev = new_opposite_edge;
        self.edge_mut(b_prev).next = new_opposite_edge;
        self.edge_mut(b_next).prev = new_edge;
        self.face_mut(original_face).first_edge = new_edge;

        let mut it = new_opposite_edge;
        loop {
            let edge = &mut self.edge_mut(it);
            edge.face = opposite_face;
            it = edge.next;
            if it == new_opposite_edge { break; }
        }

        self.debug_assert_face_invariants(original_face);
        self.debug_assert_face_invariants(opposite_face);

        return if add_face {
            Some(opposite_face)
        } else {
            None
        };
    }

    /// Not implemented yet.
    pub fn join_vertices(&mut self, _: VertexId, _: VertexId) {
        panic!("not implemented");
    }

    /// Not implemented yet.
    pub fn merge_vertices(&mut self, _: VertexId, _: VertexId) {
        panic!("not implemented");
    }

    /// Not implemented yet.
    pub fn extrude_edge(&mut self, _: EdgeId) {
        panic!("not implemented");
    }

    /// Not implemented yet.
    pub fn extrude_face(&mut self, _: FaceId, _face_per_edge: bool) {
        panic!("not implemented");
    }

    /// Returns the number of edges around the face in O(n).
    pub fn count_edges_around_face(&self, face: FaceId) -> u32 {
        let face = self.face(face);
        let stop = self.edge(face.first_edge).prev;
        let mut it = face.first_edge;
        let mut count: u32 = 1;
        loop {
            if it == stop { break; }
            count += 1;
            it = self.edge(it).next;
        }
        return count;
    }

    /// Insert a Face in the kernel
    pub fn add_edge(&mut self) -> EdgeId {
        let id = edge_id(self.edges().len() as Index);
        self.edges.push(HalfEdgeData {
            next: no_edge(),
            prev: no_edge(),
            opposite: no_edge(),
            face: no_face(),
            vertex: no_vertex(),
        });
        return id;
    }

    /// Insert a Face in the kernel
    pub fn add_face(&mut self) -> FaceId {
        let id = face_id(self.faces().len() as Index);
        self.faces.push(FaceData {
            first_edge: no_edge(),
            inner_edges: vec![],
        });
        return id;
    }

    /// Insert a Face in the kernel
    ///
    /// Note that this does not set the HalfEdgeData.face member of the edges in the loop
    /// that face.first_edge points to. This step has to be done manually if needed.
    pub fn add_face_with_connections(&mut self, face: FaceData) -> FaceId {
        let id = face_id(self.faces().len() as Index);
        self.faces.push(face);
        return id;
    }

    /// Insert a Vertex in the kernel
    ///
    /// The vertex is not connected to any edge.
    pub fn add_vertex(&mut self) -> VertexId {
        let id = vertex_id(self.vertices().len() as Index);
        self.vertices.push(VertexData { first_edge: no_edge() });
        return id;
    }

    pub fn add_loop_with_vertices(
        &mut self,
        vertices: &[VertexId], // TODO, use a generic iteartor instead?
        face1: FaceId, // inner face
        face2: FaceId  // outer face
    ) -> (
        EdgeId, // first inner edge
        EdgeId  // first outer edge
    ) {
        // TODO[nical] factor the code with add_loop
        assert!(face1 != face2);
        let edge_offset = self.edges.len() as Index;
        let first_inner_edge = edge_id(self.edges.len() as Index);
        let n_vertices = vertices.len() as Index;

        for i in (0 .. n_vertices) {
            self.vertex_mut(vertices[i as usize]).first_edge = edge_id(edge_offset + i);
            self.edges.push(HalfEdgeData {
                vertex: vertices[i as usize],
                opposite: edge_id(edge_offset + n_vertices * 2 - i - 1),
                face: face1,
                next: edge_id(edge_offset + modulo(i as i32 + 1, n_vertices as i32) as Index),
                prev: edge_id(edge_offset + modulo(i as i32 - 1, n_vertices as i32) as Index),
            });
        }

        let first_outer_edge = edge_id(self.edges.len() as Index);
        for i in (0 .. n_vertices) {
            let inv_i = n_vertices - i - 1;
            self.edges.push(HalfEdgeData {
                vertex: vertices[((inv_i + 1)%n_vertices) as usize],
                opposite: edge_id(edge_offset + inv_i),
                face: face2,
                next: edge_id(edge_offset + n_vertices + modulo(i as i32 + 1, n_vertices as i32) as Index),
                prev: edge_id(edge_offset + n_vertices + modulo(i as i32 - 1, n_vertices as i32) as Index),
            });
        }

        return (first_inner_edge, first_outer_edge);
    }

    // Add a loop of vertices and edges
    pub fn add_loop(
        &mut self,
        n_vertices: Index,
        face1: FaceId, // inner face
        face2: FaceId  // outer face
    ) -> (
        EdgeId, // first inner edge
        EdgeId  // first outer edge
    ) {
        assert!(face1 != face2);
        let edge_offset = self.edges.len() as Index;
        let vertex_offset = self.vertices.len() as Index;
        let first_inner_edge = edge_id(self.edges.len() as Index);
        for i in (0 .. n_vertices) {
            self.vertices.push(VertexData { first_edge: edge_id(edge_offset + i) });
            self.edges.push(HalfEdgeData {
                vertex: vertex_id(vertex_offset + i),
                opposite: edge_id(edge_offset + n_vertices * 2 - i - 1),
                face: face1,
                next: edge_id(edge_offset + modulo(i as i32 + 1, n_vertices as i32) as Index),
                prev: edge_id(edge_offset + modulo(i as i32 - 1, n_vertices as i32) as Index),
            });
        }

        let first_outer_edge = edge_id(self.edges.len() as Index);
        for i in (0 .. n_vertices) {
            let inv_i = n_vertices - i - 1;
            self.edges.push(HalfEdgeData {
                vertex: vertex_id(vertex_offset + (inv_i + 1)%n_vertices),
                opposite: edge_id(edge_offset + inv_i),
                face: face2,
                next: edge_id(edge_offset + n_vertices + modulo(i as i32 + 1, n_vertices as i32) as Index),
                prev: edge_id(edge_offset + n_vertices + modulo(i as i32 - 1, n_vertices as i32) as Index),
            });
        }

        return (first_inner_edge, first_outer_edge);
    }

    /// Create a ConnectivityKernel initialized with a loop
    pub fn from_loop(n_vertices: Index) -> ConnectivityKernel {
        assert!(n_vertices >= 3);
        let main_face = face_id(0);
        let back_face = face_id(1);

        let mut kernel = ConnectivityKernel {
            faces: vec![],
            vertices: vec![],
            edges: vec![],
        };

        let (first_inner_edge, first_outer_edge) = kernel.add_loop(n_vertices, main_face, back_face);

        kernel.faces = vec![
            FaceData {
                first_edge: first_inner_edge,
                inner_edges: vec![],
            },
            FaceData {
                first_edge: first_outer_edge,
                inner_edges: vec![],
            }
        ];

        kernel.debug_assert_face_invariants(main_face);
        kernel.debug_assert_face_invariants(back_face);

        return kernel;
    }

    pub fn new() -> ConnectivityKernel {
        ConnectivityKernel {
            edges: Vec::new(),
            vertices: Vec::new(),
            faces: Vec::new(),
        }
    }

    pub fn with_capacitites(v: u16, e: u16, f: u16) -> ConnectivityKernel {
        ConnectivityKernel {
            edges: Vec::with_capacity(v as usize),
            vertices: Vec::with_capacity(e as usize),
            faces: Vec::with_capacity(f as usize),
        }
    }

    /// Add a loop of vertices and edges creating a hole in an existing face.
    pub fn add_hole(&mut self, face: FaceId, n_vertices: Index) -> FaceId {
        let new_face = face_id(self.faces.len() as Index);

        let (exterior, _) = self.add_loop(n_vertices, face, new_face);

        self.faces.push(FaceData {
            first_edge: exterior,
            inner_edges: vec![],
        });

        self.face_mut(face).inner_edges.push(exterior);

        return new_face;
    }
}

impl ops::Index<EdgeId> for ConnectivityKernel {
    type Output = HalfEdgeData;
    fn index<'l>(&'l self, id: &EdgeId) -> &'l HalfEdgeData { self.edge(*id) }
}

impl ops::IndexMut<EdgeId> for ConnectivityKernel {
    fn index_mut<'l>(&'l mut self, id: &EdgeId) -> &'l mut HalfEdgeData { self.edge_mut(*id) }
}

impl ops::Index<VertexId> for ConnectivityKernel {
    type Output = VertexData;
    fn index<'l>(&'l self, id: &VertexId) -> &'l VertexData { self.vertex(*id) }
}

impl ops::IndexMut<VertexId> for ConnectivityKernel {
    fn index_mut<'l>(&'l mut self, id: &VertexId) -> &'l mut VertexData { self.vertex_mut(*id) }
}

impl ops::Index<FaceId> for ConnectivityKernel {
    type Output = FaceData;
    fn index<'l>(&'l self, id: &FaceId) -> &'l FaceData { self.face(*id) }
}

impl ops::IndexMut<FaceId> for ConnectivityKernel {
    fn index_mut<'l>(&'l mut self, id: &FaceId) -> &'l mut FaceData { self.face_mut(*id) }
}

/// A modulo that behaves properly with negative values.
fn modulo(v: i32, m: i32) -> i32 { (v%m+m)%m }

/// Convenience class that wraps a mesh's connectivity kernel and attribute data
pub struct Mesh<VertexAttribute, EdgeAttribute, FaceAttribute> {
    kernel: ConnectivityKernel,
    vertex_attributes: IdVector<VertexAttribute, Vertex_>,
    edge_attributes: IdVector<EdgeAttribute, Edge_>,
    face_attributes: IdVector<FaceAttribute, Face_>,
}

impl<V, E, F> Mesh<V, E, F> {

    pub fn new() -> Mesh<V, E, F> {
        Mesh {
            kernel: ConnectivityKernel::new(),
            vertex_attributes: IdVector::new(),
            edge_attributes: IdVector::new(),
            face_attributes: IdVector::new(),
        }
    }

    pub fn with_capacitites(v: u16, e: u16, f: u16) -> Mesh<V, E, F> {
        Mesh {
            kernel: ConnectivityKernel::with_capacitites(v, e, f),
            vertex_attributes: IdVector::with_capacity(v),
            edge_attributes: IdVector::with_capacity(e),
            face_attributes: IdVector::with_capacity(f),
        }
    }

    pub fn connectivity_kernel(&self) -> &ConnectivityKernel { &self.kernel }

    pub fn vertex(&self, id: VertexId) -> &V { &self.vertex_attributes[id] }

    pub fn vertex_mut(&mut self, id: VertexId) -> &mut V { &mut self.vertex_attributes[id] }

    pub fn egde(&self, id: EdgeId) -> &E { &self.edge_attributes[id] }

    pub fn egde_mut(&mut self, id: EdgeId) -> &mut E { &mut self.edge_attributes[id] }

    pub fn face(&self, id: FaceId) -> &F { &self.face_attributes[id] }

    pub fn face_mut(&mut self, id: FaceId) -> &mut F { &mut self.face_attributes[id] }

    pub fn add_edge(&mut self, data: E) -> EdgeId {
        let id = self.kernel.add_edge();
        self.edge_attributes[id] = data;
        return id;
    }

    pub fn add_vertex(&mut self, data: V) -> VertexId {
        let id = self.kernel.add_vertex();
        self.vertex_attributes[id] = data;
        return id;
    }

    pub fn add_face(&mut self, data: F) -> FaceId {
        let id = self.kernel.add_face();
        self.face_attributes[id] = data;
        return id;
    }
}

impl<U:Copy, V:Position2D<Unit = U>, E, F> Mesh<V, E, F> {
    pub fn position2d(&self, id: VertexId) -> &Vector2D<U> { self.vertex(id).position() }
    pub fn position2d_mut(&mut self, id: VertexId) -> &mut Vector2D<U> { self.vertex_mut(id).position_mut() }
}

impl<U:Copy, V:Position3D<Unit = U>, E, F> Mesh<V, E, F> {
    pub fn position3d(&self, id: VertexId) -> &Vector3D<U> { self.vertex(id).position() }
    pub fn position3d_mut(&mut self, id: VertexId) -> &mut Vector3D<U> { self.vertex_mut(id).position_mut() }
}

impl<U:Copy, V:Position4D<Unit = U>, E, F> Mesh<V, E, F> {
    pub fn position3d(&self, id: VertexId) -> &Vector4D<U> { self.vertex(id).position() }
    pub fn position3d_mut(&mut self, id: VertexId) -> &mut Vector4D<U> { self.vertex_mut(id).position_mut() }
}

impl<U:Copy, V:Normal2D<Unit = U>, E, F> Mesh<V, E, F> {
    pub fn normal2d(&self, id: VertexId) -> &Vector2D<U> { self.vertex(id).normal() }
    pub fn normal2d_mut(&mut self, id: VertexId) -> &mut Vector2D<U> { self.vertex_mut(id).normal_mut() }
}

impl<U:Copy, V:Normal3D<Unit = U>, E, F> Mesh<V, E, F> {
    pub fn normal3d(&self, id: VertexId) -> &Vector3D<U> { self.vertex(id).normal() }
    pub fn normal3d_mut(&mut self, id: VertexId) -> &mut Vector3D<U> { self.vertex_mut(id).normal_mut() }
}

impl<U:Copy, V:Normal4D<Unit = U>, E, F> Mesh<V, E, F> {
    pub fn normal3d(&self, id: VertexId) -> &Vector4D<U> { self.vertex(id).normal() }
    pub fn normal3d_mut(&mut self, id: VertexId) -> &mut Vector4D<U> { self.vertex_mut(id).normal_mut() }
}

impl<U:Copy, V:TextureCoordinates<Unit = U>, E, F> Mesh<V, E, F> {
    pub fn uv(&self, id: VertexId) -> &Vector2D<U> { self.vertex(id).uv() }
    pub fn uv_mut(&mut self, id: VertexId) -> &mut Vector2D<U> { self.vertex_mut(id).uv_mut() }
}


#[test]
fn test_from_loop() {
    for n in 3 .. 10 {
        println!(" --- test {} ", n);
        let kernel = ConnectivityKernel::from_loop(n);
        let face = kernel.first_face();

        assert_eq!(kernel.count_edges_around_face(face) as Index, n);

        let mut i = 0;
        for e in kernel.walk_edges_around_face(face) {
            assert!((e.as_index()) < kernel.edges.len());
            assert_eq!(
                kernel.edge(e).vertex,
                kernel.edge(kernel.edge(kernel.edge(e).opposite).next).vertex
            );
            i += 1;
        }
        assert_eq!(i, n);

        for i in  0 .. (kernel.edges.len() as u16) {
            let e = edge_id(i);
            assert_eq!(kernel.edge(kernel.edge(e).opposite).opposite, e);
            assert_eq!(kernel.edge(kernel.edge(e).next).prev, e);
            assert_eq!(kernel.edge(kernel.edge(e).prev).next, e);
        }

        for e in kernel.walk_edges_around_face_reverse(face) {
            assert!((e.as_index()) < kernel.edges.len());
            assert_eq!(kernel.edge(e).face, face);
        }

        let face2 = kernel.edge(kernel.edge(kernel.face(face).first_edge).opposite).face;
        let mut i = 0;
        for e in kernel.walk_edges_around_face_reverse(face2) {
            assert!((e.as_index()) < kernel.edges.len());
            assert_eq!(kernel.edge(e).face, face2);
            i += 1;
        }

        assert!(face2 != face);
        assert_eq!(i, n);
    }
}

#[test]
fn test_hole() {
    let mut kernel = ConnectivityKernel::from_loop(5);
    let f1 = kernel.first_face();
    kernel.add_hole(f1, 3);

    assert_eq!(kernel.edge(edge_id(0)).face, f1);
    assert_eq!(kernel.edge(edge_id(1)).face, f1);
    assert_eq!(kernel.edge(edge_id(2)).face, f1);
    assert_eq!(kernel.edge(edge_id(3)).face, f1);
    assert_eq!(kernel.edge(edge_id(4)).face, f1);

    assert!(kernel.edge(edge_id(5)).face != f1);
    assert!(kernel.edge(edge_id(6)).face != f1);
    assert!(kernel.edge(edge_id(7)).face != f1);
    assert!(kernel.edge(edge_id(8)).face != f1);
    assert!(kernel.edge(edge_id(9)).face != f1);

    assert_eq!(kernel.edge(edge_id(10)).face, f1);
    assert_eq!(kernel.edge(edge_id(11)).face, f1);
    assert_eq!(kernel.edge(edge_id(12)).face, f1);

    assert!(kernel.edge(edge_id(13)).face != f1);
    assert!(kernel.edge(edge_id(14)).face != f1);
    assert!(kernel.edge(edge_id(15)).face != f1);

    assert_eq!(kernel.edge(kernel.edge(edge_id(13)).opposite).face, f1);
}

#[test]
fn test_split_face_1() {
    let mut kernel = ConnectivityKernel::from_loop(4);
    let f1 = kernel.first_face();
    let e1 = kernel.face(f1).first_edge;
    let e2 = kernel.edge(e1).next;
    let e3 = kernel.edge(e2).next;
    let e4 = kernel.edge(e3).next;
    assert_eq!(kernel.edge(e4).next, e1);
    assert_eq!(kernel.count_edges_around_face(f1), 4);

    // x---e1---->x
    // ^          |
    // |          |
    // |          e2
    // e4   f1    |
    // |          |
    // |          v
    // x<-----e3--x

    let f2 = kernel.split_face(e3, e1).unwrap();

    // x---e1---->x
    // ^ \ ^   f1 |
    // | e5 \     |
    // |   \ \    e2
    // e4   \ \   |
    // |     \ e6 |
    // | f2   v \ v
    // x<-----e3--x

    assert!(f1 != f2);
    assert!(kernel.face(f1).first_edge != kernel.face(f2).first_edge);

    assert_eq!(kernel.edge(kernel[f1].first_edge).face, f1);
    assert_eq!(kernel.edge(kernel[f2].first_edge).face, f2);

    let e5 = kernel[e4].next;
    let e6 = kernel[e2].next;

    assert_eq!(kernel[e6].next, e1);
    assert_eq!(kernel[e1].prev, e6);
    assert_eq!(kernel[e5].next, e3);
    assert_eq!(kernel[e3].prev, e5);
    assert_eq!(kernel[e6].prev, e2);
    assert_eq!(kernel[e2].next, e6);
    assert_eq!(kernel[e5].prev, e4);
    assert_eq!(kernel[e4].next, e5);

    assert_eq!(kernel[e1].face, f1);
    assert_eq!(kernel[e2].face, f1);
    assert_eq!(kernel[e6].face, f1);
    assert_eq!(kernel[e3].face, f2);
    assert_eq!(kernel[e4].face, f2);
    assert_eq!(kernel[e5].face, f2);

    assert_eq!(kernel.count_edges_around_face(f1), 3);
    assert_eq!(kernel.count_edges_around_face(f2), 3);
}

#[test]
fn test_split_face_2() {
    let mut kernel = ConnectivityKernel::from_loop(10);
    let f1 = kernel.first_face();

    let e1 = kernel[f1].first_edge;
    let e2 = kernel[e1].next;
    let e3 = kernel[e2].next;
    let e4 = kernel[e3].next;

    let f2 = kernel.split_face(e4, e2).unwrap();

    for e in kernel.walk_edges_around_face(f2) {
        assert_eq!(kernel[e].face, f2);
    }

    for e in kernel.walk_edges_around_face(f1) {
        assert_eq!(kernel[e].face, f1);
    }

    for dir in [Direction::Forward, Direction::Backward].iter() {
        for face in [f1, f2].iter() {
            let mut it = DirectedEdgeCirculator::new(&kernel, kernel.face(*face).first_edge, *dir);
            let stop = it.prev();
            loop {
                assert_eq!(it.face_id(), *face);
                if it == stop {
                    break;
                }
                it = it.next();
            }
        }
    }
}
