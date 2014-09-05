use containers::item_vector::ItemVector;
use containers::id::Id;

pub struct GeomId<T> {
    handle: u16,
}

impl<T> Id for GeomId<T> {
    fn to_index(&self) -> uint { self.handle as uint }
    fn from_index(idx: uint) -> GeomId<T> { GeomId { handle: idx as u16 } }
}

struct VertexHandle;
struct FaceHandle;
struct HalfEdgeHandle;

pub type VertexId = GeomId<VertexHandle>;
pub type FaceId = GeomId<FaceHandle>;
pub type HalfEdgeId = GeomId<HalfEdgeHandle>;

pub struct HalfEdge {
    next: HalfEdgeId,
    previous: HalfEdgeId,
    opposite: HalfEdgeId,
    vertex: VertexId,
    face: FaceId,
}

pub struct Face {
    first_edge: HalfEdgeId,
}

pub struct Vertex {
    edge: HalfEdgeId,
}

pub struct Manifold {
    kernel: ConnectivityKernel,
}

impl Manifold {
    pub fn new() -> Manifold {
        Manifold: {
            kernel {
                half_edges: ItemVector::new(),
                vertices: ItemVector::new(),
                faces: ItemVector::new(),
            }
        }
    }
}

struct ConnectivityKernel {
    half_edges: ItemVector<HalfEdge, HalfEdgeId>,
    vertices: ItemVector<Vertex, VertexId>,
    faces: ItemVector<Face, FaceId>,    
}

impl ConnectivityKernel {
    fn set_out(&mut self, current: VertexId, out: HalfEdgeId) {
        self.vertices.get_mut(id).edge = out;
    }

    fn set_opposite(&mut self, id: HalfEdgeId, opposite: HalfEdgeId) {
        self.half_edges.get_mut(id).edge = out;
    }

    fn set_face(&mut self, id: HalfEdgeId, face: FaceId) {
        self.half_edges.get_mut(id).face = face;
    }

    fn set_next(&mut self, id: HalfEdgeId, next: HalfEdgeId) {
        self.half_edges.get_mut(id).next = next;
    }
}