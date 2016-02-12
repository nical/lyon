
pub mod monotone;
pub mod convex;
pub mod path;
pub mod polygon;
pub mod connection;
pub mod intersection;
pub mod bezier;
pub mod vertex_builder;
pub mod vectors;
pub mod tesselation;
pub mod path_to_polygon;

pub use vodk_math::vec2::Vec2;

pub fn error<Err, S>(err: Err) -> Result<S, Err> { Err(err) }

use vodk_id::{Id, IdRange};

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
