use std::ops;
use std::u16;

pub use id_internals::Index;

use id_internals::is_valid;
use traits::*;
use vodk_math::vector::{ Vector2D, Vector3D, Vector4D };
use vodk_id::*;
use vodk_id::id_vector::IdVector;

use iterators::{
    EdgeIdLoop, ReverseEdgeIdLoop, MutEdgeLoop,
};

#[cfg(test)]
use iterators::{ DirectedEdgeCirculator, Direction };

use std::marker::PhantomData;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Vertex_;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Edge_;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Face_;

pub type VertexId = Id<Vertex_, Index>;
pub type EdgeId = Id<Edge_, Index>;
pub type FaceId = Id<Face_, Index>;

pub const NO_EDGE: EdgeId = EdgeId { handle: u16::MAX, _marker: PhantomData };
pub const NO_FACE: FaceId = FaceId { handle: u16::MAX, _marker: PhantomData };
pub const NO_VERTEX: VertexId = VertexId { handle: u16::MAX, _marker: PhantomData };

/// Create an EdgeId from an index (the offset in the ConnectivityKernel's half edge vector)
#[inline]
pub fn edge_id(index: Index) -> EdgeId { EdgeId::new(index) }

/// Create a FaceId from an index (the offset in the ConnectivityKernel's face vector)
#[inline]
pub fn face_id(index: Index) -> FaceId { FaceId::new(index) }

/// Create a VertexId from an index (the offset in the ConnectivityKernel's vertex vector)
#[inline]
pub fn vertex_id(index: Index) -> VertexId { VertexId::new(index) }

/// A range of Id pointing to contiguous vertices.
pub type VertexIdRange = IdRange<Vertex_, Index>;

/// A range of Id pointing to contiguous half edges.
pub type EdgeIdRange = IdRange<Edge_, Index>;

/// A range of Id pointing to contiguous faces.
pub type FaceIdRange = IdRange<Face_, Index>;

/// The structure holding the data specific to each half edge.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HalfEdge {
    pub next: EdgeId, // next half edge around the face
    pub prev: EdgeId, // previous half edge around the face
    pub vertex: VertexId, // vertex this edge origins from
    pub opposite: EdgeId,
    pub face: FaceId,
}

const EMPTY_EDGE: HalfEdge = HalfEdge {
    next: NO_EDGE,
    prev: NO_EDGE,
    opposite: NO_EDGE,
    face: NO_FACE,
    vertex: NO_VERTEX,
};

/// The structure holding the data specific to each face.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub inner_edges: Vec<EdgeId>,
    pub first_edge: EdgeId,
}

/// The structure holding the data specific to each vertex.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vertex {
    pub first_edge: EdgeId,
}

const EMPTY_VERTEX: Vertex = Vertex {
    first_edge: NO_EDGE,
};

struct Wrapper<T, ID> {
    data: T,
    list_next: ID, // intrusive linked list
    list_prev: ID, // intrusive linked list
}

type FaceData = Wrapper<Face, FaceId>;
type HalfEdgeData = Wrapper<HalfEdge, EdgeId>;

/// The data structure that contains a mesh's connectivity information
///
/// It does not contain other attributes such as positions. Use IdVector for that.
pub struct ConnectivityKernel {
    edges: Vec<HalfEdgeData>,
    vertices: Vec<Vertex>,
    faces: Vec<FaceData>,

    first_edge: EdgeId,
    edge_freelist: EdgeId,

    first_vertex: VertexId,
    vertex_freelist: VertexId,

    first_face: FaceId,
    face_freelist: FaceId,
}

impl ConnectivityKernel {

    /// Create an empty kernel.
    pub fn new() -> ConnectivityKernel {
        ConnectivityKernel {
            edges: Vec::new(),
            vertices: Vec::new(),
            faces: Vec::new(),
            first_edge: NO_EDGE,
            edge_freelist: NO_EDGE,
            first_vertex: NO_VERTEX,
            vertex_freelist: NO_VERTEX,
            first_face: NO_FACE,
            face_freelist: NO_FACE,
        }
    }

    /// Create an empty kernel and preallocate memory for vertices, edges and faces.
    pub fn with_capacitites(v: u16, e: u16, f: u16) -> ConnectivityKernel {
        ConnectivityKernel {
            edges: Vec::with_capacity(v as usize),
            vertices: Vec::with_capacity(e as usize),
            faces: Vec::with_capacity(f as usize),
            first_edge: NO_EDGE,
            edge_freelist: NO_EDGE,
            first_vertex: NO_VERTEX,
            vertex_freelist: NO_VERTEX,
            first_face: NO_FACE,
            face_freelist: NO_FACE,
        }
    }

    /// Create a ConnectivityKernel initialized with a loop
    pub fn from_loop(n_vertices: Index) -> ConnectivityKernel {
        assert!(n_vertices >= 3);
        let mut kernel = ConnectivityKernel::with_capacitites(n_vertices, n_vertices*2, 2);

        let back_face = kernel.add_face();
        let main_face = kernel.add_face();

        kernel.add_loop(n_vertices, main_face, back_face);

        kernel.debug_assert_face_invariants(main_face);
        kernel.debug_assert_face_invariants(back_face);

        return kernel;
    }

    /// Vertex getter. You can also use the indexing operator.
    pub fn vertex(&self, id: VertexId) -> &Vertex {
        debug_assert!(is_valid(id));
        &self.vertices[id.handle as usize]
    }

    /// Vertex mutable getter. You can also use the indexing operator.
    fn vertex_mut(&mut self, id: VertexId) -> &mut Vertex {
        debug_assert!(is_valid(id));
        &mut self.vertices[id.handle as usize]
    }

    /// Face getter. You can also use the indexing operator.
    pub fn face(&self, id: FaceId) -> &Face {
        debug_assert!(is_valid(id));
        &self.faces[id.handle as usize].data
    }

    /// Face mutable getter. You can also use the indexing operator.
    fn face_mut(&mut self, id: FaceId) -> &mut Face {
        debug_assert!(is_valid(id));
        &mut self.faces[id.handle as usize].data
    }

    fn face_internal(&self, id: FaceId) -> &Wrapper<Face, FaceId> {
        debug_assert!(is_valid(id));
        &self.faces[id.handle as usize]
    }

    fn face_internal_mut(&mut self, id: FaceId) -> &mut Wrapper<Face, FaceId> {
        debug_assert!(is_valid(id));
        &mut self.faces[id.handle as usize]
    }

    /// Half edge getter. You can also use the indexing operator.
    pub fn edge(&self, id: EdgeId) -> &HalfEdge {
        debug_assert!(is_valid(id));
        &self.edges[id.handle as usize].data
    }

    /// Half edge mutable getter. You can also use the indexing operator.
    pub fn edge_mut(&mut self, id: EdgeId) -> &mut HalfEdge {
        debug_assert!(is_valid(id));
        &mut self.edges[id.handle as usize].data
    }

    pub fn first_edge(&self) -> EdgeId { self.first_edge }

    pub fn first_face(&self) -> FaceId { self.first_face }

    pub fn first_vertex(&self) -> VertexId { self.first_vertex }

    pub fn contains_egde(&self, id: EdgeId) -> bool { id.to_index() < self.edges.len() }

    pub fn contains_face(&self, id: FaceId) -> bool { id.to_index() < self.faces.len() }

    pub fn contains_vertex(&self, id: VertexId) -> bool { id.to_index() < self.vertices.len() }

    pub fn walk_edge_ids_around_face<'l>(&'l self, id: FaceId) -> EdgeIdLoop<'l> {
        let edge = self.face(id).first_edge;
        let prev = if is_valid(edge) { self.edge(edge).prev } else { NO_EDGE };
        EdgeIdLoop::new(self, edge, prev)
    }

    /// Iterate over halfedge ids around a loop
    pub fn walk_edge_ids<'l>(&'l self, first: EdgeId) -> EdgeIdLoop<'l> {
        EdgeIdLoop::new(self, first, self.edge(first).prev)
    }

    /// Iterate over halfedges around a loop
    pub fn walk_edges_mut<'l>(&'l mut self, first: EdgeId) -> MutEdgeLoop<'l> {
        let stop = self.edge(first).prev;
        return MutEdgeLoop::new(self, first, stop);
    }

    /// Shorthand for walk_edge_ids for a given face's loop
    pub fn walk_edge_ids_around_face_reverse<'l>(&'l self, id: FaceId) -> ReverseEdgeIdLoop<'l> {
        let edge = self.face(id).first_edge;
        ReverseEdgeIdLoop::new(self, edge, self.edge(edge).next)
    }

    /// Return the next edge id when circulating around a vertex.
    pub fn next_edge_id_around_vertex(&self, id: EdgeId) -> EdgeId {
        return self.edge(self.edge(id).opposite).next;
    }

    /// Run a few debug-only assertions to check the state of a given edge.
    pub fn debug_assert_edge_invariants(&self, id: EdgeId) {
        debug_assert_eq!(self[self[id].opposite].opposite, id);
        debug_assert_eq!(self[self[id].next].prev, id);
        debug_assert_eq!(self[self[id].prev].next, id);
        debug_assert_eq!(
            self[id].vertex,
            self[self[self[id].opposite].next].vertex
        );
        debug_assert_eq!(self[id].face, self[self[id].next].face);
        debug_assert_eq!(self[id].face, self[self[id].prev].face);
    }

    /// Run a few debug-only assertions to check the state of a given face,
    /// and the edges in its loop.
    pub fn debug_assert_face_invariants(&self, face: FaceId) {
        if !is_valid(face) {
            return;
        }
        for e in self.walk_edge_ids_around_face(face) {
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
        //let new_edge = edge_id(self.vertices.len() as Index);
        //let new_opposite = edge_id(self.vertices.len() as Index + 1);

        // new_edge
        let edge = *self.edge(id);
        let new_edge = self.add_edge(HalfEdge {
            vertex: edge.vertex,
            opposite: edge.opposite,
            face: edge.face,
            next: edge.next,
            prev: id,
        });

        // new_opposite
        let opposite = *self.edge(edge.opposite);
        let new_opposite = self.add_edge(HalfEdge {
            vertex: opposite.vertex,
            opposite: id,
            face: opposite.face,
            next: opposite.next,
            prev: edge.opposite,
        });

        self.vertices.push(Vertex {
            first_edge: new_edge,

        });

        // patch up existing edges
        self.edge_mut(id).vertex = new_vertex;
        self.edge_mut(id).next = new_edge;
        self.edge_mut(edge.opposite).vertex = new_vertex;
        self.edge_mut(edge.opposite).next = new_opposite;

        return new_vertex;
    }

    /// Connect edges e1 and e2 such that e1->[new edge]->e2.
    ///
    /// This operation may add a new face. If so, the face's id is returned.
    /// If a face id is provided as parameter, and a face must be added, the
    /// provided face will be used instead of creating a new one.
    pub fn connect_edges(
        &mut self,
        e1: EdgeId,
        e2: EdgeId,
        maybe_new_face: Option<FaceId>
    ) -> Option<FaceId> {
        //
        //   -e1--> v1 --e1_next->
        //          |^
        //          n|
        //          ||   new_face
        //          |o
        //          v|
        //   <--e2- v2 <--e2_prev-
        // ______________________
        //
        // n: new_edge (returned)
        // o: new_opposite_edge

        let mut add_face = true;
        let original_face = self.edge(e1).face;

        // Check whether we are connecting to a hole in the face, in which case
        // we should not add a face.
        for i in 0 .. self.face(original_face).inner_edges.len() {
            for e in self.walk_edge_ids(self.face(original_face).inner_edges[i]) {
                if e == e1 || e == e2 {
                    // connecting to one of the inner loops
                    add_face = false;
                    // remove the hole from this face
                    break;
                }
            }
            if !add_face {
                self.face_mut(original_face).inner_edges.remove(i);
                break;
            }
        }

        let e1_next = self.edge(e1).next;
        let e2_prev = self.edge(e2).prev;
        let v1 = self.edge(e1_next).vertex;
        let v2 = self.edge(e2).vertex;

        let new_edge = self.add_edge(HalfEdge {
            next: e2,
            prev: e1,
            opposite: NO_EDGE,
            face: original_face,
            vertex: v1
        });
        let new_opposite_edge = self.add_edge(HalfEdge {
            next: e1_next,
            prev: e2_prev,
            opposite: new_edge,
            face: original_face, // may become opposite_face
            vertex: v2
        });
        self.edge_mut(new_edge).opposite = new_opposite_edge;

        self.edge_mut(e1).next = new_edge;
        self.edge_mut(e2).prev = new_edge;
        self.edge_mut(e1_next).prev = new_opposite_edge;
        self.edge_mut(e2_prev).next = new_opposite_edge;
        self.face_mut(original_face).first_edge = new_edge;

        self.debug_assert_face_invariants(original_face);

        if add_face {
            let opposite_face = match maybe_new_face {
                Some(face) => { face }
                None => { self.add_face_with_edge(e1_next) }
            };
            let mut it = new_opposite_edge;
            loop {
                let edge = &mut self.edge_mut(it);
                edge.face = opposite_face;
                it = edge.next;
                if it == new_opposite_edge { break; }
            }
            self.debug_assert_face_invariants(opposite_face);
            return Some(opposite_face);
        }

        return None;
    }

    /// Insert a half edge in the kernel
    pub fn add_empty_edge(&mut self) -> EdgeId {
        self.add_edge(HalfEdge {
            next: NO_EDGE,
            prev: NO_EDGE,
            opposite: NO_EDGE,
            face: NO_FACE,
            vertex: NO_VERTEX,
        })
    }

    fn add_edge(&mut self, data: HalfEdge) -> EdgeId {
        let first_edge = self.first_edge;
        let new_id = if self.edge_freelist != NO_EDGE {
            let id = self.edge_freelist;
            let freelist_next = self.edges[id.to_index()].list_next;
            self.edges[id.to_index()] = Wrapper {
                data: data,
                list_next: first_edge,
                list_prev: NO_EDGE
            };
            if is_valid(freelist_next) {
                self.edges[freelist_next.to_index()].list_prev = NO_EDGE;
            }
            self.edge_freelist = freelist_next;

            id
        } else {
            let id = edge_id(self.edges.len() as Index);
            self.edges.push(Wrapper {
                data: data,
                list_next: first_edge,
                list_prev: NO_EDGE,
            });

            id
        };
        if first_edge != NO_EDGE {
            self.edges[first_edge.to_index()].list_prev = new_id;
        }
        self.first_edge = new_id;
        return new_id;
    }

    fn remove_edge(&mut self, id: EdgeId) {
        let prev = self.edges[id.to_index()].list_prev;
        if is_valid(prev) {
            self.edges[prev.to_index()].list_next = self.edges[id.to_index()].list_next;
        } else {
            self.first_edge = NO_EDGE;
        }
        self.edges[id.to_index()] = Wrapper {
            data: EMPTY_EDGE,
            list_next: self.edge_freelist,
            list_prev: NO_EDGE,
        };
        self.edge_freelist = id;
    }

    /// Insert a Face in the kernel.
    pub fn add_face(&mut self) -> FaceId { self.add_face_with_edge(NO_EDGE) }

    /// Insert a Face in the kernel.
    pub fn add_face_with_edge(&mut self, first_edge: EdgeId) -> FaceId {
        let first_face = self.first_face;
        let new_id = if self.face_freelist != NO_FACE {
            let id = self.face_freelist;
            let freelist_next = self.face_internal(id).list_next;
            self.faces[id.to_index()] = Wrapper {
                data: Face {
                    first_edge: first_edge,
                    inner_edges: vec![],
                },
                list_next: self.first_face,
                list_prev: NO_FACE,
            };
            if is_valid(freelist_next) {
                self.face_internal_mut(freelist_next).list_prev = NO_FACE;
            }
            self.face_freelist = freelist_next;

            id
        } else {
            let id = face_id(self.faces.len() as Index);
            self.faces.push(Wrapper {
                data: Face {
                    first_edge: first_edge,
                    inner_edges: vec![],
                },
                list_next: self.first_face,
                list_prev: NO_FACE,
            });

            id
        };

        if first_face != NO_FACE {
            self.face_internal_mut(first_face).list_prev = new_id;
        }
        self.first_face = new_id;
        return new_id;
    }


    /// Remove a face, without removing the half edges in its loop.
    pub fn remove_face(&mut self, id: FaceId) {
        let prev = self.face_internal_mut(id).list_prev;
        if is_valid(prev) {
            self.face_internal_mut(prev).list_next = self.face_internal(id).list_next;
        } else {
            self.first_face = NO_FACE;
        }
        *self.face_internal_mut(id) = Wrapper {
            data: Face {
                first_edge: NO_EDGE,
                inner_edges: vec![],
            },
            list_next: self.face_freelist,
            list_prev: NO_FACE,
        };
        self.face_freelist = id;
    }

    /// Insert a Vertex in the kernel
    ///
    /// The vertex is not connected to any edge.
    pub fn add_vertex(&mut self) -> VertexId {
        let id = vertex_id(self.vertices.len() as Index);
        self.vertices.push(EMPTY_VERTEX);
        return id;
    }

    /// Add several several vertices with contiguous offsets.
    pub fn add_vertices(&mut self, number: Index) -> VertexIdRange {
        let first = self.vertices.len() as Index;
        for _ in 0..number {
            self.vertices.push(EMPTY_VERTEX);
        }
        return VertexIdRange {
            first: vertex_id(first),
            count: number
        };
    }

    /// Try to add several several vertices with contiguous offsets, expecting a
    /// given value for the first offset.
    ///
    /// Useful when the caller expects the created vertices to be at certain offsets, for
    /// example when building a kernel against a pre-existing set of vertices without
    /// duplicating the vertices.
    /// Returns an error without adding any vertex if the size of the vertex
    /// array is not equal to the first offset.
    pub fn add_vertices_with_offsets(&mut self, first: Index, number: Index) -> Result<VertexIdRange, ()> {
        if first != self.vertices.len() as Index {
            return Err(());
        }
        for _ in 0..number {
            self.vertices.push(EMPTY_VERTEX);
        }
        return Ok(VertexIdRange {
            first: vertex_id(first),
            count: number
        });
    }

    /// Extrude the vertex that the edge passed as parameter points to, adding a vertex and
    /// two edges to the kernel.
    pub fn extrude_vertex(&mut self, id: EdgeId, vertex: VertexId) -> EdgeId {
        let edge_data = *self.edge(id);
        let opposite_data = *self.edge(edge_data.opposite);
        let v1 = opposite_data.vertex;

        let new_edge = self.add_edge(HalfEdge {
            next: NO_EDGE,
            prev: id,
            opposite: NO_EDGE,
            face: edge_data.face,
            vertex: v1,
        });
        let new_opposite = self.add_edge(HalfEdge {
            next: edge_data.next,
            prev: new_edge,
            opposite: new_edge,
            face: edge_data.face,
            vertex: vertex,
        });
        {
            let edge = self.edge_mut(new_edge);
            edge.opposite = new_opposite;
            edge.next = new_opposite;
        }
        self.vertex_mut(vertex).first_edge = new_opposite;

        self.edge_mut(edge_data.next).prev = new_opposite;
        self.edge_mut(id).next = new_edge;

        return new_edge;
    }

    /// Connect two vertices.
    ///
    /// Only use this on isolated vertices.
    pub fn add_segment(&mut self, v1: VertexId, v2: VertexId, face: FaceId) -> EdgeId {
        let e12 = self.add_edge(HalfEdge{
            next: NO_EDGE,
            prev: NO_EDGE,
            opposite: NO_EDGE,
            vertex: v1,
            face: face,
        });
        let e21 = self.add_edge(HalfEdge{
            next: e12,
            prev: e12,
            opposite: e12,
            vertex: v2,
            face: face,
        });
        {
            let edge12 = self.edge_mut(e12);
            edge12.next = e21;
            edge12.prev = e21;
            edge12.opposite = e21;
        }
        self.vertex_mut(v1).first_edge = e12;
        self.vertex_mut(v2).first_edge = e21;
        return e12;
    }

    // Add a loop of edges, using existing vertices.
    pub fn add_loop_with_vertices<IT:Iterator<Item=VertexId>>(
        &mut self,
        mut vertices: IT,
        f1: FaceId,
        f2: FaceId
    ) -> EdgeId {
        let v1 = vertices.next().unwrap();
        let v2 = vertices.next().unwrap();
        let first_edge = self.add_segment(v1, v2, f1);
        let mut edge = first_edge;
        for vertex in vertices {
            edge = self.extrude_vertex(edge, vertex);
        }
        // close the loop
        self.connect_edges(edge, first_edge, Some(f2));

        if is_valid(f1) {
            self.face_mut(f1).first_edge = first_edge;
        }
        if is_valid(f2) {
            self.face_mut(f2).first_edge = self.edge(first_edge).opposite;
        }

        return first_edge;
    }

    // Add a loop of vertices and edges
    pub fn add_loop(
        &mut self,
        n_vertices: Index,
        f1: FaceId, // inner face
        f2: FaceId  // outer face
    ) -> EdgeId {
        let v1 = self.add_vertex();
        let v2 = self.add_vertex();
        let first_edge = self.add_segment(v1, v2, f1);
        let mut edge = first_edge;
        for _ in 2..n_vertices {
            let vertex = self.add_vertex();
            edge = self.extrude_vertex(edge, vertex);
        }

        // close the loop
        self.connect_edges(edge, first_edge, Some(f2));

        self.face_mut(f1).first_edge = first_edge;
        self.face_mut(f2).first_edge = self.edge(first_edge).opposite;

        return first_edge;
    }

    /// Add a loop of vertices and edges creating a hole in an existing face.
    pub fn add_hole(&mut self, outer_face: FaceId, n_vertices: Index) -> FaceId {
        let hole_face = self.add_face();
        let hole_vertices = self.add_vertices(n_vertices);
        let hole_loop = self.add_loop_with_vertices(hole_vertices.iter(), hole_face, NO_FACE);

        let opp = self[hole_loop].opposite;
        for edge in self.walk_edges_mut(opp) {
            edge.face = outer_face;
        }

        self.set_hole(outer_face, hole_loop);

        return hole_face;
    }

    pub fn set_hole(&mut self, outer_face: FaceId, hole_interior_loop: EdgeId) {
        let opp = self[hole_interior_loop].opposite;
        self[outer_face].inner_edges.push(opp);
    }
}

impl ops::Index<EdgeId> for ConnectivityKernel {
    type Output = HalfEdge;
    fn index<'l>(&'l self, id: EdgeId) -> &'l HalfEdge { self.edge(id) }
}

impl ops::IndexMut<EdgeId> for ConnectivityKernel {
    fn index_mut<'l>(&'l mut self, id: EdgeId) -> &'l mut HalfEdge { self.edge_mut(id) }
}

impl ops::Index<VertexId> for ConnectivityKernel {
    type Output = Vertex;
    fn index<'l>(&'l self, id: VertexId) -> &'l Vertex { self.vertex(id) }
}

impl ops::IndexMut<VertexId> for ConnectivityKernel {
    fn index_mut<'l>(&'l mut self, id: VertexId) -> &'l mut Vertex { self.vertex_mut(id) }
}

impl ops::Index<FaceId> for ConnectivityKernel {
    type Output = Face;
    fn index<'l>(&'l self, id: FaceId) -> &'l Face { self.face(id) }
}

impl ops::IndexMut<FaceId> for ConnectivityKernel {
    fn index_mut<'l>(&'l mut self, id: FaceId) -> &'l mut Face { self.face_mut(id) }
}

/// Convenience class that wraps a mesh's connectivity kernel and attribute data
pub struct Mesh<VertexAttribute, EdgeAttribute, FaceAttribute> {
    kernel: ConnectivityKernel,
    vertex_attributes: IdVector<VertexId, VertexAttribute>,
    edge_attributes: IdVector<EdgeId, EdgeAttribute>,
    face_attributes: IdVector<FaceId, FaceAttribute>,
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
        let id = self.kernel.add_empty_edge();
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
fn test_add_segment() {
    let mut kernel = ConnectivityKernel::new();
    for _ in 0..5 {
        let f1 = kernel.add_face();
        let v1 = kernel.add_vertex();
        let v2 = kernel.add_vertex();
        let e = kernel.add_segment(v1, v2, f1);
        let o = kernel[e].opposite;

        kernel.debug_assert_edge_invariants(e);
        kernel.debug_assert_edge_invariants(o);
        assert_eq!(kernel[e].face, f1);
        assert_eq!(kernel[e].next, o);
        assert_eq!(kernel[e].opposite, o);
        assert_eq!(kernel[e].prev, o);
        assert_eq!(kernel[o].next, e);
        assert_eq!(kernel[o].opposite, e);
        assert_eq!(kernel[o].prev, e);
        assert_eq!(kernel[o].face, f1);
        assert_eq!(kernel[v1].first_edge, e);
        assert_eq!(kernel[v2].first_edge, o);
    }
}

#[test]
fn test_extrude_vertex() {
    let mut kernel = ConnectivityKernel::new();
    for _ in 0..5 {
        let f1 = kernel.add_face();
        let v1 = kernel.add_vertex();
        let v2 = kernel.add_vertex();
        let v3 = kernel.add_vertex();
        let e1 = kernel.add_segment(v1, v2, f1);
        let o1 = kernel[e1].opposite;

        let e2 = kernel.extrude_vertex(e1, v3);
        let o2 = kernel[e2].opposite;

        assert_eq!(kernel[e1].next, e2);
        assert_eq!(kernel[e1].prev, o1);
        assert_eq!(kernel[e2].next, o2);
        assert_eq!(kernel[e2].prev, e1);
        assert_eq!(kernel[o2].next, o1);
        assert_eq!(kernel[o2].prev, e2);

        assert_eq!(kernel[e2].vertex, v2);
        assert_eq!(kernel[o2].vertex, v3);

        assert_eq!(kernel[e2].face, f1);
        assert_eq!(kernel[o2].face, f1);

        kernel.debug_assert_edge_invariants(e1);
        kernel.debug_assert_edge_invariants(e2);
        kernel.debug_assert_edge_invariants(o1);
        kernel.debug_assert_edge_invariants(o2);
    }
}

#[test]
fn test_make_loop() {
    let n_vertices = 4;
    let mut kernel = ConnectivityKernel::new();
    let f1 = kernel.add_face();
    let f2 = kernel.add_face();
    let v1 = kernel.add_vertex();
    let v2 = kernel.add_vertex();
    let first_edge = kernel.add_segment(v1, v2, f1);
    let mut edge = first_edge;
    for _ in 2..n_vertices {
        let vertex = kernel.add_vertex();
        edge = kernel.extrude_vertex(edge, vertex);
    }
    // close the loop
    kernel.connect_edges(edge, first_edge, Some(f2));

    kernel.face_mut(f1).first_edge = first_edge;
    kernel.face_mut(f2).first_edge = kernel.edge(first_edge).opposite;

    println!(" -- built loop, testing f1");
    kernel.debug_assert_face_invariants(f1);
    println!(" -- testing f1");
    kernel.debug_assert_face_invariants(f2);
}

#[test]
fn test_add_loop_with_vertices() {
    let mut kernel = ConnectivityKernel::new();
    for n_vertices in 3..10 {
        let vertex_ids = kernel.add_vertices(n_vertices);

        let f1 = kernel.add_face();
        let f2 = kernel.add_face();

        kernel.add_loop_with_vertices(vertex_ids.iter(), f1, f2);

        kernel.debug_assert_face_invariants(f1);
        kernel.debug_assert_face_invariants(f2);

        let mut edge = NO_EDGE;
        for e in kernel.walk_edge_ids_around_face(f1) {
            if kernel[e].vertex == vertex_ids.get(0) {
                edge = e;
                break;
            }
        }
        assert!(edge != NO_EDGE);

        // Check that the winding order is correct.
        for i in 0..n_vertices {
            assert_eq!(kernel[edge].vertex, vertex_ids.get(i));
            assert_eq!(kernel[edge].face, f1);
            edge = kernel[edge].next;
        }
    }
}

#[test]
fn test_from_loop() {
    for n in 3 .. 10 {
        println!(" --- test {} ", n);
        let kernel = ConnectivityKernel::from_loop(n);
        let face = kernel.first_face();

        assert_eq!(kernel.walk_edge_ids_around_face(face).count() as Index, n);

        let mut i = 0;
        for e in kernel.walk_edge_ids_around_face(face) {
            assert!((e.to_index()) < kernel.edges.len());
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

        for e in kernel.walk_edge_ids_around_face_reverse(face) {
            assert!((e.to_index()) < kernel.edges.len());
            assert_eq!(kernel.edge(e).face, face);
        }

        let face2 = kernel.edge(kernel.edge(kernel.face(face).first_edge).opposite).face;
        let mut i = 0;
        for e in kernel.walk_edge_ids_around_face_reverse(face2) {
            assert!((e.to_index()) < kernel.edges.len());
            assert_eq!(kernel.edge(e).face, face2);
            i += 1;
        }

        assert!(face2 != face);
        assert_eq!(i, n);
    }
}

#[test]
fn test_hole() {
    let mut kernel = ConnectivityKernel::from_loop(4);

    let f1 = kernel.first_face();
    kernel.add_hole(f1, 3);

    assert_eq!(kernel[f1].inner_edges.len(), 1);
    let inner1 = kernel[f1].inner_edges[0];
    for e in kernel.walk_edge_ids(inner1) {
        kernel.debug_assert_edge_invariants(e);
    }
    let inner_opp = kernel[inner1].opposite;
    for e in kernel.walk_edge_ids(inner_opp) {
        kernel.debug_assert_edge_invariants(e);
    }

    for e in kernel.walk_edge_ids(kernel[f1].first_edge) {
        kernel.debug_assert_edge_invariants(e);
    }
}

#[test]
fn test_connect_1() {
    let mut kernel = ConnectivityKernel::from_loop(4);
    let f1 = kernel.first_face();
    let e1 = kernel.face(f1).first_edge;
    let e2 = kernel.edge(e1).next;
    let e3 = kernel.edge(e2).next;
    let e4 = kernel.edge(e3).next;
    assert_eq!(kernel.edge(e4).next, e1);
    assert_eq!(kernel.walk_edge_ids_around_face(f1).count(), 4);

    // x---e1---->x
    // ^          |
    // |          |
    // |          e2
    // e4   f1    |
    // |          |
    // |          v
    // x<-----e3--x

    kernel.connect_edges(e2, e1, None);


    // x---e1---->x
    // ^ \ ^   f1 |
    // | e5 \     |
    // |   \ \    e2
    // e4   \ \   |
    // |     \ e6 |
    // | f2   v \ v
    // x<-----e3--x

    let f2 = kernel[e4].face;
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

    assert_eq!(kernel.walk_edge_ids_around_face(f1).count(), 3);
    assert_eq!(kernel.walk_edge_ids_around_face(f2).count(), 3);
}

#[test]
fn test_connect_2() {
    let mut kernel = ConnectivityKernel::from_loop(10);
    let f1 = kernel.first_face();

    let e1 = kernel[f1].first_edge;
    let e2 = kernel[e1].next;
    let e3 = kernel[e2].next;
    let e4 = kernel[e3].next;

    let f2 = kernel.connect_edges(e4, e2, None).unwrap();

    for e in kernel.walk_edge_ids_around_face(f2) {
        assert_eq!(kernel[e].face, f2);
    }

    for e in kernel.walk_edge_ids_around_face(f1) {
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

#[test]
fn test_face_list() {
    let mut kernel = ConnectivityKernel::new();

    assert_eq!(kernel.first_face(), NO_FACE);

    let f1 = kernel.add_face();
    let f2 = kernel.add_face();
    let f3 = kernel.add_face();
    kernel.remove_face(f1);
    kernel.remove_face(f2);
    kernel.remove_face(f3);

    assert_eq!(kernel.first_face(), NO_FACE);

    let f1 = kernel.add_face();
    let f2 = kernel.add_face();
    let f3 = kernel.add_face();
    kernel.remove_face(f3);
    kernel.remove_face(f2);
    kernel.remove_face(f1);

    assert_eq!(kernel.first_face(), NO_FACE);

    let f1 = kernel.add_face();
    let f2 = kernel.add_face();
    let f3 = kernel.add_face();
    kernel.remove_face(f2);
    let f4 = kernel.add_face();
    kernel.remove_face(f1);
    kernel.remove_face(f3);
    let f5 = kernel.add_face();
    let f6 = kernel.add_face();
    kernel.remove_face(f5);
    kernel.remove_face(f4);
    kernel.remove_face(f6);

    assert_eq!(kernel.first_face(), NO_FACE);
}
