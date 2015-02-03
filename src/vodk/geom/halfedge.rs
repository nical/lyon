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

    pub fn get_vertex(&self, id: VertexId) -> &Vertex {
        assert!(id.is_valid());
        &self.vertices[id.index as usize - 1]
    }

    fn get_mut_vertex(&mut self, id: VertexId) -> &mut Vertex {
        assert!(id.is_valid());
        &mut self.vertices[id.index as usize - 1]
    }
    
    pub fn get_face(&self, id: FaceId) -> &Face {
        assert!(id.is_valid());
        &self.faces[id.index as usize - 1]
    }

    fn get_mut_face(&mut self, id: FaceId) -> &mut Face {
        assert!(id.is_valid());
        &mut self.faces[id.index as usize - 1]
    }
    
    pub fn get_edge(&self, id: EdgeId) -> &HalfEdge {
        assert!(id.is_valid());
        &self.edges[id.index as usize - 1]
    }

    fn get_mut_edge(&mut self, id: EdgeId) -> &mut HalfEdge {
        assert!(id.is_valid());
        &mut self.edges[id.index as usize - 1]
    }

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

        let original_face = self.get_edge(a).face;
        assert_eq!(original_face, self.get_edge(b).face);
        assert!(self.get_edge(a).next != b);
        assert!(self.get_edge(a).prev != b);

        let va = self.get_edge(self.get_edge(a).prev).vertex;
        let vb = self.get_edge(self.get_edge(b).prev).vertex;
        let new_edge = EdgeId { index: self.edges.len() as u32 + 1 }; // va -> vb
        let new_opposite_edge = EdgeId { index: self.edges.len() as u32 + 2 }; // vb -> va

        let new_face = FaceId { index: self.faces.len() as u32 };
        self.faces.push(Face { first_edge: a });

        let mut it = a;
        loop {
            if it == b { break; }
            let edge = &mut self.get_mut_edge(it);
            edge.face = new_face;
            it = edge.next;
        }

        let a_prev = self.get_edge(a).prev;
        let b_prev = self.get_edge(b).prev;

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

        self.get_mut_edge(a_prev).next = new_edge;
        self.get_mut_edge(a).prev = new_opposite_edge;
        self.get_mut_edge(b_prev).next = new_opposite_edge;
        self.get_mut_edge(b).prev = new_edge;


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
        let face = self.get_face(face);
        let stop = self.get_edge(face.first_edge).prev;
        let mut it = face.first_edge;
        let mut count: u32 = 1;
        loop {
            if it == stop { break; }
            count += 1;
            it = self.get_edge(it).next;
            if count > 10 { panic!(); }
        }
        return count;
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

/// A modulo that behaves properly with negative values.
fn modulo(v: i32, m: i32) -> i32 { (v%m+m)%m }

#[test]
fn test_from_loop() {
    for n in range(3, 10) {
        println!(" -- testing a loop with {} vertices", n);
        let kernel = ConnectivityKernel::from_loop(n);
        let face = FaceId { index: 1};

        assert_eq!(kernel.count_edges_around_face(face), n);

        for e in kernel.edges.iter() {
            assert_eq!(e.face, FaceId { index: 1 });
            assert_eq!(e.opposite, EdgeId { index: 0 });
        }

        let mut i = 1;
        for e in kernel.walk_edges_around_face(face) {
            assert!((e.index as usize - 1) < kernel.edges.len());
            i += 1;
        }
        assert_eq!(i, n);

        let mut i = 1;
        for e in kernel.walk_edges_around_face_reverse(face) {
            assert!((e.index as usize - 1) < kernel.edges.len());
            i += 1;
        }
        assert_eq!(i, n);
    }
}

#[test]
fn test_split_face() {
    let mut kernel = ConnectivityKernel::from_loop(4);
    let f1 = FaceId { index: 1 };
    let e1 = kernel.get_face(f1).first_edge;
    let e2 = kernel.get_edge(e1).next;
    let e3 = kernel.get_edge(e2).next;
    let e4 = kernel.get_edge(e3).next;
    assert_eq!(kernel.get_edge(e4).next, e1);
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

    let e5 = kernel.get_edge(e4).next;
    assert_eq!(e5.index, 5);
    let e6 = kernel.get_edge(e2).next;
    assert_eq!(e6.index, 6);

    assert_eq!(kernel.get_edge(e6).next, e1);
    assert_eq!(kernel.get_edge(e5).next, e3);
    assert_eq!(kernel.get_edge(e6).prev, e2);
    assert_eq!(kernel.get_edge(e5).prev, e4);

    assert_eq!(kernel.count_edges_around_face(f1), 3);
    assert_eq!(kernel.count_edges_around_face(f2), 3);
}
