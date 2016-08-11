extern crate sid;
extern crate sid_vec;
extern crate euclid;

pub mod math;
pub mod path;
pub mod path_builder;
pub mod path_iterator;
pub mod math_utils;
pub mod bezier;
pub mod flatten_cubic;
pub mod arc;

use sid::{ Id, IdRange, };
use sid_vec::{ IdSlice, MutIdSlice, };

/// The integer type to index a vertex in a vertex buffer or path.
pub type Index = u16;

/// Phantom type marker for VertexId.
#[derive(Debug)]
pub struct Vertex_;

/// An Id that represents a vertex in a contiguous vertex buffer.
pub type VertexId = Id<Vertex_, Index>;

/// Create a VertexId from an u16 index.
#[inline]
pub fn vertex_id(index: Index) -> VertexId { VertexId::new(index) }

/// A range of VertexIds pointing to contiguous vertices.
pub type VertexIdRange = IdRange<Vertex_, Index>;

/// Create a VertexIdRange.
pub fn vertex_id_range(from: u16, to: u16) -> VertexIdRange {
    IdRange {
        first: Id::new(from),
        count: to - from,
    }
}

/// A slice of vertices indexed with VertexIds rather than usize offset.
pub type VertexSlice<'l, V> = IdSlice<'l, VertexId, V>;

/// A slice of mutable vertices indexed with VertexIds rather than usize offset.
pub type MutVertexSlice<'l, V> = MutIdSlice<'l, VertexId, V>;
