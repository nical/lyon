
pub mod monotone;
pub mod convex;
pub mod path;
pub mod polygon;
pub mod connection;
pub mod bezier;
pub mod vertex_builder;
pub mod vectors;
pub mod tesselation;
pub mod path_to_polygon;
pub mod sweep_line;
pub mod bentley_ottmann;
pub mod experimental;

pub fn error<Err, S>(err: Err) -> Result<S, Err> { Err(err) }

use vodk_id::{Id, IdSlice, MutIdSlice, IdRange};

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

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum WindingOrder {
    Clockwise,
    CounterClockwise,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Direction {
    Forward,
    Backward,
}

impl Direction {
    pub fn reverse(self) -> Direction {
        match self {
            Direction::Forward => Direction::Backward,
            Direction::Backward => Direction::Forward,
        }
    }
}
