
pub mod id_internals;
pub mod iterators;
pub mod kernel;

pub use half_edge::kernel::{
    ConnectivityKernel,
    EdgeId, VertexId, FaceId,
    EdgeIdRange, VertexIdRange, FaceIdRange,
    edge_id, vertex_id, face_id
};
pub use half_edge::iterators::*;
