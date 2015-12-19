use half_edge::kernel::*;
use half_edge::vectors::*;
use vodk_id::id_vector::IdVector;

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

    pub fn with_capacities(v: u16, e: u16, f: u16) -> Mesh<V, E, F> {
        Mesh {
            kernel: ConnectivityKernel::with_capacities(e, f),
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

    pub fn add_face(&mut self, data: F) -> FaceId {
        let id = self.kernel.add_face();
        self.face_attributes[id] = data;
        return id;
    }
}

impl<V:Position2D, E, F> Mesh<V, E, F> {
    pub fn position2d(&self, id: VertexId) -> Vec2 { self.vertex(id).position() }
}

impl<V:Position3D, E, F> Mesh<V, E, F> {
    pub fn position3d(&self, id: VertexId) -> Vec3 { self.vertex(id).position() }
}

impl<V:Position4D, E, F> Mesh<V, E, F> {
    pub fn position3d(&self, id: VertexId) -> Vec4 { self.vertex(id).position() }
}

impl<V:Normal2D, E, F> Mesh<V, E, F> {
    pub fn normal2d(&self, id: VertexId) -> Vec2 { self.vertex(id).normal() }
}

impl<V:Normal3D, E, F> Mesh<V, E, F> {
    pub fn normal3d(&self, id: VertexId) -> Vec3 { self.vertex(id).normal() }
}

impl<V:Normal4D, E, F> Mesh<V, E, F> {
    pub fn normal4d(&self, id: VertexId) -> Vec4 { self.vertex(id).normal() }
}

impl<V:TextureCoordinates, E, F> Mesh<V, E, F> {
    pub fn uv(&self, id: VertexId) -> Vec2 { self.vertex(id).uv() }
}
