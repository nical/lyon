pub mod basic_shapes;
pub mod bezier;
pub mod math_utils;
pub mod path;
pub mod path_builder;
pub mod path_tesselator;
pub mod vertex_builder;
pub mod rust_logo;

pub fn error<Err, S>(err: Err) -> Result<S, Err> { Err(err) }
pub fn crash() -> ! { panic!() }

use sid::{ Id, IdRange, };
use sid_vec::{ IdSlice, MutIdSlice, };

pub type Index = u16;

#[derive(Debug)]
pub struct Vertex_;
pub type VertexId = Id<Vertex_, Index>;

/// Create a VertexId from an index (the offset in the ConnectivityKernel's vertex vector)
#[inline]
pub fn vertex_id(index: Index) -> VertexId { VertexId::new(index) }

/// A range of Id pointing to contiguous vertices.
pub type VertexIdRange = IdRange<Vertex_, Index>;

pub fn vertex_id_range(from: u16, to: u16) -> VertexIdRange {
    IdRange {
        first: Id::new(from),
        count: to - from,
    }
}

pub type VertexSlice<'l, V> = IdSlice<'l, VertexId, V>;
pub type MutVertexSlice<'l, V> = MutIdSlice<'l, VertexId, V>;
