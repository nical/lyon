#[derive(Copy, Clone, Show, PartialEq, Eq)]
pub struct EdgeId {
    pub handle: u16,
}

#[derive(Copy, Clone, Show, PartialEq, Eq)]
pub struct FaceId {
    pub handle: u16,
}

#[derive(Copy, Clone, Show, PartialEq, Eq)]
pub struct VertexId {
    pub handle: u16,
}

impl EdgeId { pub fn is_valid(self) -> bool { self.handle != 0 } }
impl FaceId { pub fn is_valid(self) -> bool { self.handle != 0 } }
impl VertexId { pub fn is_valid(self) -> bool { self.handle != 0 } }

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

    pub fn vertex(&self, id: VertexId) -> &Vertex {
        assert!(id.is_valid());
        &self.vertices[id.handle as usize - 1]
    }

    fn vertex_mut(&mut self, id: VertexId) -> &mut Vertex {
        assert!(id.is_valid());
        &mut self.vertices[id.handle as usize - 1]
    }
    
    pub fn face(&self, id: FaceId) -> &Face {
        assert!(id.is_valid());
        &self.faces[id.handle as usize - 1]
    }

    fn face_mut(&mut self, id: FaceId) -> &mut Face {
        assert!(id.is_valid());
        &mut self.faces[id.handle as usize - 1]
    }
    
    pub fn edge(&self, id: EdgeId) -> &HalfEdge {
        assert!(id.is_valid());
        &self.edges[id.handle as usize - 1]
    }

    fn edge_mut(&mut self, id: EdgeId) -> &mut HalfEdge {
        assert!(id.is_valid());
        &mut self.edges[id.handle as usize - 1]
    }

    pub fn edges(&self) -> &[HalfEdge] { &self.edges[] }

    pub fn faces(&self) -> &[Face] { &self.faces[] }

    pub fn vertices(&self) -> &[Vertex] { &self.vertices[] }

    pub fn first_edge(&self) -> EdgeId { EdgeId { handle: 1 } }

    pub fn first_face(&self) -> FaceId { FaceId { handle: 1 } }

    pub fn first_vertex(&self) -> VertexId { VertexId { handle: 1 } }

    pub fn walk_edges_around_face<'l>(&'l self, id: FaceId) -> FaceEdgeIterator<'l> {
        let edge = self.face(id).first_edge;
        return FaceEdgeIterator {
            data: self,
            current_edge: edge,
            last_edge: self.edge(edge).prev,
        }
    }

    pub fn walk_edges_around_face_reverse<'l>(&'l self, id: FaceId) -> ReverseFaceEdgeIterator<'l> {
        let edge = self.face(id).first_edge;
        return ReverseFaceEdgeIterator {
            data: self,
            current_edge: edge,
            last_edge: self.edge(edge).next,
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

        let new_vertex = VertexId { handle: self.vertices.len() as u16 + 1 };
        let new_edge = EdgeId { handle: self.vertices.len() as u16 + 1 };
        let new_opposite = EdgeId { handle: self.vertices.len() as u16 + 2 };

        self.vertices.push(Vertex {
            first_edge: new_edge,
        });

        // new_edge
        let edge = *self.edge(id);
        self.edges.push(HalfEdge {
            vertex: edge.vertex,
            opposite: edge.opposite,
            face: edge.face,
            next: edge.next,
            prev: id,
        });

        // new_opposite
        let opposite = *self.edge(edge.opposite);
        self.edges.push(HalfEdge {
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

    /// Split a face in two along the vertices vertices
    pub fn split_face(&mut self, a: EdgeId, b: EdgeId) -> FaceId {
        //
        // a_prev--> va -a------>
        //          | ^
        //   f1     n |
        //          | |
        //          | o     f2
        //          v |
        // <------b- vb <--b_prev
        // ______________________
        //
        // f1: original_face
        // f2: new_face
        // n: new_edge
        // o: new_opposite_edge

        let original_face = self.edge(a).face;
        assert_eq!(original_face, self.edge(b).face);
        assert!(self.edge(a).next != b);
        assert!(self.edge(a).prev != b);

        let va = self.edge(self.edge(a).prev).vertex;
        let vb = self.edge(self.edge(b).prev).vertex;
        let new_edge = EdgeId { handle: self.edges.len() as u16 + 1 }; // va -> vb
        let new_opposite_edge = EdgeId { handle: self.edges.len() as u16 + 2 }; // vb -> va

        let new_face = FaceId { handle: self.faces.len() as u16 };
        self.faces.push(Face { first_edge: a });

        let mut it = a;
        loop {
            if it == b { break; }
            let edge = &mut self.edge_mut(it);
            edge.face = new_face;
            it = edge.next;
        }

        let a_prev = self.edge(a).prev;
        let b_prev = self.edge(b).prev;

        // new_edge
        self.edges.push(HalfEdge {
            next: b,
            prev: a_prev,
            opposite: new_opposite_edge,
            face: original_face,
            vertex: vb,
        });

        // new_opposite_edge
        self.edges.push(HalfEdge {
            next: a,
            prev: b_prev,
            opposite: new_edge,
            face: new_face,
            vertex: va,
        });

        self.edge_mut(a_prev).next = new_edge;
        self.edge_mut(a).prev = new_opposite_edge;
        self.edge_mut(b_prev).next = new_opposite_edge;
        self.edge_mut(b).prev = new_edge;


        return new_face;
    }

    pub fn join_vertices(&mut self, a: VertexId, b: VertexId) {
        panic!("not implemented");
    }

    /// Merge b into a (removing b)
    pub fn merge_vertices(&mut self, a: VertexId, b: VertexId) {
        panic!("not implemented");
    }

    pub fn extrude_edge(&mut self, id: EdgeId) {
        panic!("not implemented");
    }

    pub fn extrude_face(&mut self, id: FaceId, face_per_edge: bool) {
        panic!("not implemented");
    }

    pub fn count_edges_around_face(&self, face: FaceId) -> u32 {
        let face = self.face(face);
        let stop = self.edge(face.first_edge).prev;
        let mut it = face.first_edge;
        let mut count: u32 = 1;
        loop {
            if it == stop { break; }
            count += 1;
            it = self.edge(it).next;
            if count > 10 { panic!(); }
        }
        return count;
    }

    /// constructor
    pub fn from_loop(n_vertices: u16) -> ConnectivityKernel {
        assert!(n_vertices >= 3);
        let mut vertices = Vec::new();
        let mut edges = Vec::new();
        for i in range(0, n_vertices) {
            vertices.push(Vertex { first_edge: EdgeId { handle:(i%n_vertices) + 1 } });
            edges.push(HalfEdge {
                vertex: VertexId { handle: ((i + 1) % n_vertices) + 1 },
                opposite: EdgeId { handle: 0 },
                face: FaceId { handle: 1 },
                next: EdgeId { handle: ((i + 1) % n_vertices) + 1 },
                prev: EdgeId { handle: modulo(i as i32 - 1, n_vertices as i32) as u16 + 1 },
            });
        }
        return ConnectivityKernel {
            faces: vec!(Face { first_edge: EdgeId { handle: 1 } }),
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
        self.current_edge = self.data.edge(self.current_edge).next;
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
        self.current_edge = self.data.edge(self.current_edge).prev;
        return Some(self.current_edge);
    }
}

/// Iterates over the half edges that point to a vertex.
pub struct VertexEdgeIterator<'l> {
    data: &'l ConnectivityKernel,
    current_edge: EdgeId,
    first_edge: EdgeId,
}

impl<'l> Iterator for VertexEdgeIterator<'l> {
    type Item = EdgeId;

    fn next(&mut self) -> Option<EdgeId> {
        if !self.current_edge.is_valid() {
            return None;
        }
        let temp = self.current_edge;
        self.current_edge = self.data.edge(self.data.edge(self.current_edge).next).opposite;
        if self.current_edge == self.first_edge {
            self.current_edge = EdgeId { handle: 0 };
        }
        return Some(temp);
    }
}

/// A modulo that behaves properly with negative values.
fn modulo(v: i32, m: i32) -> i32 { (v%m+m)%m }

#[test]
fn test_from_loop() {
    for n in range(3, 10) {
        println!(" -- testing a loop with {} vertices", n);
        let kernel = ConnectivityKernel::from_loop(n);
        let face = kernel.first_face();

        assert_eq!(kernel.count_edges_around_face(face) as u16, n);

        for e in kernel.edges.iter() {
            assert_eq!(e.face, face);
            assert!(!e.opposite.is_valid());
        }

        let mut i = 1;
        for e in kernel.walk_edges_around_face(face) {
            assert!((e.handle as usize - 1) < kernel.edges.len());
            i += 1;
        }
        assert_eq!(i, n);

        let mut i = 1;
        for e in kernel.walk_edges_around_face_reverse(face) {
            assert!((e.handle as usize - 1) < kernel.edges.len());
            i += 1;
        }
        assert_eq!(i, n);
    }
}

#[test]
fn test_split_face() {
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

    let f2 = kernel.split_face(e1, e3);

    // x---e1---->x
    // ^ \ ^   f2 |
    // | e5 \     |
    // |   \ \    e2
    // e4   \ \   |
    // |     \ e6 |
    // | f1   v \ v
    // x<-----e3--x

    let e5 = kernel.edge(e4).next;
    assert_eq!(e5.handle, 5);
    let e6 = kernel.edge(e2).next;
    assert_eq!(e6.handle, 6);

    assert_eq!(kernel.edge(e6).next, e1);
    assert_eq!(kernel.edge(e5).next, e3);
    assert_eq!(kernel.edge(e6).prev, e2);
    assert_eq!(kernel.edge(e5).prev, e4);

    assert_eq!(kernel.count_edges_around_face(f1), 3);
    assert_eq!(kernel.count_edges_around_face(f2), 3);
}
