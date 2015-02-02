#[derive(Copy, Clone, Show, PartialEq, Eq)]
pub struct EdgeId {
    pub index: u32,
}

#[derive(Copy, Clone, Show, PartialEq, Eq)]
pub struct FaceId {
    pub index: u32,
}

#[derive(Copy, Clone, Show, PartialEq, Eq)]
pub struct VertexId {
    pub index: u32,
}

impl EdgeId { pub fn is_valid(self) -> bool { self.index != 0 } }
impl FaceId { pub fn is_valid(self) -> bool { self.index != 0 } }
impl VertexId { pub fn is_valid(self) -> bool { self.index != 0 } }

#[derive(Copy, Clone, Show, PartialEq)]
pub struct HalfEdge {
    pub next: EdgeId, // next HalfEdge around the face
    pub prev: EdgeId, // previous HalfEdge around the face
    pub vertex: VertexId, // vertex this edge points to
    pub opposite: EdgeId,
    pub face: FaceId,
}

#[derive(Copy, Clone, Show, PartialEq)]
pub struct Face {
    pub first_edge: EdgeId,
}

#[derive(Copy, Clone, Show, PartialEq)]
pub struct Vertex {
    pub first_edge: EdgeId,
}

pub struct ConnectivityKernel {
    edges: Vec<HalfEdge>,
    vertices: Vec<Vertex>,
    faces: Vec<Face>
}


impl ConnectivityKernel {
    pub fn get_vertex(&self, id: VertexId) -> &Vertex { &self.vertices[id.index as usize - 1] }
    fn get_mut_vertex(&mut self, id: VertexId) -> &mut Vertex { &mut self.vertices[id.index as usize - 1] }
    
    pub fn get_face(&self, id: FaceId) -> &Face { &self.faces[id.index as usize - 1] }
    fn get_mut_face(&mut self, id: FaceId) -> &mut Face { &mut self.faces[id.index as usize - 1] }
    
    pub fn get_edge(&self, id: EdgeId) -> &HalfEdge { &self.edges[id.index as usize - 1] }
    fn get_mut_edge(&mut self, id: EdgeId) -> &mut HalfEdge { &mut self.edges[id.index as usize - 1] }

    pub fn edges(&self) -> &[HalfEdge] { &self.edges[] }
    pub fn faces(&self) -> &[Face] { &self.faces[] }
    pub fn vertices(&self) -> &[Vertex] { &self.vertices[] }

    pub fn walk_edges_around_face<'l>(&'l self, id: FaceId) -> FaceEdgeIterator<'l> {
        let edge = self.get_face(id).first_edge;
        return FaceEdgeIterator {
            data: self,
            current_edge: edge,
            last_edge: self.get_edge(edge).prev,
        }
    }

    pub fn walk_edges_around_face_reverse<'l>(&'l self, id: FaceId) -> ReverseFaceEdgeIterator<'l> {
        let edge = self.get_face(id).first_edge;
        return ReverseFaceEdgeIterator {
            data: self,
            current_edge: edge,
            last_edge: self.get_edge(edge).next,
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

        let new_vertex = VertexId { index: self.vertices.len() as u32 + 1 };
        let new_edge = EdgeId { index: self.vertices.len() as u32 + 1 };
        let new_opposite = EdgeId { index: self.vertices.len() as u32 + 2 };

        self.vertices.push(Vertex {
            first_edge: new_edge,
        });

        // new_edge
        let edge = *self.get_edge(id);
        self.edges.push(HalfEdge {
            vertex: edge.vertex,
            opposite: edge.opposite,
            face: edge.face,
            next: edge.next,
            prev: id,
        });

        // new_opposite
        let opposite = *self.get_edge(edge.opposite);
        self.edges.push(HalfEdge {
            vertex: opposite.vertex,
            opposite: id,
            face: opposite.face,
            next: opposite.next,
            prev: edge.opposite,
        });

        // patch up existing edges
        self.get_mut_edge(id).vertex = new_vertex;
        self.get_mut_edge(id).next = new_edge;
        self.get_mut_edge(edge.opposite).vertex = new_vertex;
        self.get_mut_edge(edge.opposite).next = new_opposite;

        return new_vertex;
    }

    /// Split a face in two along the given vertices
    pub fn split_face(&mut self, face: FaceId, a: VertexId, b: VertexId) -> FaceId {
        panic!("not implemented");
    }

    pub fn join_vertices(&mut self, a: VertexId, b: VertexId) {
        panic!("not implemented");
    }

    /// Merge b into a (remving b)
    pub fn merge_vertices(&mut self, a: VertexId, b: VertexId) {
        panic!("not implemented");
    }

    pub fn extrude_edge(&mut self, id: EdgeId) {
        panic!("not implemented");
    }

    pub fn extrude_face(&mut self, id: FaceId, face_per_edge: bool) {
        panic!("not implemented");
    }

    /// constructor
    pub fn from_loop(n_vertices: u32) -> ConnectivityKernel {
        assert!(n_vertices >= 3);
        let mut vertices = Vec::new();
        let mut edges = Vec::new();
        for i in range(0, n_vertices) {
            vertices.push(Vertex { first_edge: EdgeId { index:(i%n_vertices) + 1 } });
            edges.push(HalfEdge {
                vertex: VertexId { index: ((i + 1) % n_vertices) + 1 },
                opposite: EdgeId { index: 0 },
                face: FaceId { index: 1 },
                next: EdgeId { index: ((i + 1) % n_vertices) + 1 },
                prev: EdgeId { index: modulo(i as i32 - 1, n_vertices as i32) as u32 + 1 },
            });
        }
        return ConnectivityKernel {
            faces: vec!(Face { first_edge: EdgeId { index: 1 } }),
            vertices: vertices,
            edges: edges,
        };
    }
}

/// Iterates over the half edges around a face.
pub struct FaceEdgeIterator<'l> {
    data: &'l ConnectivityKernel,
    current_edge: EdgeId,
    last_edge: EdgeId,
}

impl<'l> Iterator for FaceEdgeIterator<'l> {
    type Item = EdgeId;

    fn next(&mut self) -> Option<EdgeId> {
        if self.current_edge == self.last_edge {
            return None;
        }
        self.current_edge = self.data.get_edge(self.current_edge).next;
        return Some(self.current_edge);
    }
}

/// Iterates over the half edges around a face in reverse order.
pub struct ReverseFaceEdgeIterator<'l> {
    data: &'l ConnectivityKernel,
    current_edge: EdgeId,
    last_edge: EdgeId,
}

impl<'l> Iterator for ReverseFaceEdgeIterator<'l> {
    type Item = EdgeId;

    fn next(&mut self) -> Option<EdgeId> {
        if self.current_edge == self.last_edge {
            return None;
        }
        self.current_edge = self.data.get_edge(self.current_edge).prev;
        return Some(self.current_edge);
    }
}

/// Iterates over the half edges around a vertex.
pub struct VertexEdgeIterator<'l> {
    data: &'l ConnectivityKernel,
    current_edge: EdgeId,
    first_edge: EdgeId,
}

impl<'l> Iterator for VertexEdgeIterator<'l> {
    type Item = EdgeId;

    fn next(&mut self) -> Option<EdgeId> {
        panic!("TODO");
    }
}

#[test]
fn test_from_loop() {
    for n in range(3, 10) {
        println!(" -- testing a loop with {} vertices", n);
        let kernel = ConnectivityKernel::from_loop(n);
        assert_eq!(kernel.faces().to_vec(), vec!(Face {first_edge: EdgeId { index: 1 }}));
        for e in kernel.edges.iter() {
            assert_eq!(e.face, FaceId { index: 1 });
            assert_eq!(e.opposite, EdgeId { index: 0 });
        }

        let mut i = 1;
        for e in kernel.walk_edges_around_face(FaceId { index: 1}) {
            assert!((e.index as usize - 1) < kernel.edges.len());
            i += 1;
        }
        assert_eq!(i, n);

        let mut i = 1;
        for e in kernel.walk_edges_around_face_reverse(FaceId { index: 1}) {
            assert!((e.index as usize - 1) < kernel.edges.len());
            i += 1;
        }
        assert_eq!(i, n);
    }
}

/// A modulo that behaves properly with negative values.
fn modulo(v: i32, m: i32) -> i32 { (v%m+m)%m }
