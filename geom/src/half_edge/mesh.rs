use half_edge::kernel::*;
use half_edge::traits::*;
use vodk_math::vec2::Vector2D;
use vodk_math::vec3::Vector3D;
use vodk_math::vec4::Vector4D;
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
