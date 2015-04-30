use halfedge::{
    ConnectivityKernel, HalfEdge,
    EdgeId, FaceId, VertexId,
    NO_EDGE,
};

use id_internals::is_valid;

use std::cmp::PartialEq;
use std::mem::transmute;

/// Iterates over the half edges around a face.
pub struct EdgeIdLoop<'l> {
    kernel: &'l ConnectivityKernel,
    current_edge: EdgeId,
    last_edge: EdgeId,
    done: bool,
}

impl<'l> Iterator for EdgeIdLoop<'l> {
    type Item = EdgeId;

    fn next(&mut self) -> Option<EdgeId> {
        let res = self.current_edge;
        if self.done {
            return None;
        }
        if self.current_edge == self.last_edge {
            self.done = true;
        }
        if self.current_edge == NO_EDGE {
            return None;
        }
        self.current_edge = self.kernel[self.current_edge].next;
        return Some(res);
    }
}

impl<'l> EdgeIdLoop<'l> {
    pub fn new(
        kernel: &'l ConnectivityKernel,
        first: EdgeId,
        last: EdgeId,
    ) -> EdgeIdLoop<'l> {
        EdgeIdLoop {
            kernel: kernel,
            current_edge: first,
            last_edge: last,
            done: false,
        }
    }
}

/// Iterates over the half edges around a face.
pub struct MutEdgeLoop<'l> {
    kernel: &'l mut ConnectivityKernel,
    current_edge: EdgeId,
    last_edge: EdgeId,
    done: bool,
}

impl<'l> Iterator for MutEdgeLoop<'l> {
    type Item = &'l mut HalfEdge;

    fn next(&mut self) -> Option<&'l mut HalfEdge> {
        let res = self.current_edge;
        if self.done {
            return None;
        }
        if self.current_edge == self.last_edge {
            self.done = true;
        }
        if self.current_edge == NO_EDGE {
            return None;
        }
        self.current_edge = self.kernel[self.current_edge].next;
        return unsafe { Some(transmute(&mut self.kernel[res])) }; // TODO could remove transmute
    }
}

impl<'l> MutEdgeLoop<'l> {
    pub fn new(
        kernel: &'l mut ConnectivityKernel,
        first: EdgeId,
        last: EdgeId,
    ) -> MutEdgeLoop<'l> {
        MutEdgeLoop {
            kernel: kernel,
            current_edge: first,
            last_edge: last,
            done: false,
        }
    }
}

/// Iterates over the half edges around a face in reverse order.
pub struct ReverseEdgeIdLoop<'l> {
    kernel: &'l ConnectivityKernel,
    current_edge: EdgeId,
    last_edge: EdgeId,
    done: bool,
}

impl<'l> Iterator for ReverseEdgeIdLoop<'l> {
    type Item = EdgeId;

    fn next(&mut self) -> Option<EdgeId> {
        let res = self.current_edge;
        if self.done {
            return None;
        }
        if self.current_edge == self.last_edge {
            self.done = true;
        }
        self.current_edge = self.kernel[self.current_edge].prev;
        return Some(res);
    }
}

impl<'l> ReverseEdgeIdLoop<'l> {
    pub fn new(
        kernel: &'l ConnectivityKernel,
        first: EdgeId,
        last: EdgeId,
    ) -> ReverseEdgeIdLoop<'l> {
        ReverseEdgeIdLoop {
            kernel: kernel,
            current_edge: first,
            last_edge: last,
            done: false,
        }
    }
}

/// Iterates over the half edges that point to a vertex.
pub struct VertexEdgeIterator<'l> {
    kernel: &'l ConnectivityKernel,
    current_edge: EdgeId,
    first_edge: EdgeId,
}

impl<'l> Iterator for VertexEdgeIterator<'l> {
    type Item = EdgeId;

    fn next(&mut self) -> Option<EdgeId> {
        if !is_valid(self.current_edge) {
            return None;
        }
        let temp = self.current_edge;
        self.current_edge = self.kernel[self.kernel[self.current_edge].next].opposite;
        if self.current_edge == self.first_edge {
            self.current_edge = NO_EDGE;
        }
        return Some(temp);
    }
}

//pub struct VertexIdIterator {
//    current: Index,
//    stop: Index,
//}
//
//impl<'l> Iterator for VertexIdIterator {
//    type Item = VertexId;
//
//    fn next(&mut self) -> Option<VertexId> {
//        if self.current == self.stop { return None; }
//        let res = self.current;
//        self.current += 1;
//        return Some(vertex_id(res));
//    }
//}
//
//pub struct EdgeIdIterator {
//    current: Index,
//    stop: Index,
//}
//
//impl<'l> Iterator for EdgeIdIterator {
//    type Item = EdgeId;
//
//    fn next(&mut self) -> Option<EdgeId> {
//        if self.current == self.stop { return None; }
//        let res = self.current;
//        self.current += 1;
//        return Some(edge_id(res));
//    }
//}
//
//pub struct FaceIdIterator {
//    current: Index,
//    stop: Index,
//}
//
//impl<'l> Iterator for FaceIdIterator {
//    type Item = FaceId;
//
//    fn next(&mut self) -> Option<FaceId> {
//        if self.current == self.stop { return None; }
//        let res = self.current;
//        self.current += 1;
//        return Some(face_id(res));
//    }
//}

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

#[derive(Copy, Clone)]
pub struct EdgeCirculator<'l> {
    kernel: &'l ConnectivityKernel,
    edge: EdgeId,
}

impl<'l> EdgeCirculator<'l> {
    pub fn new(kernel: &'l ConnectivityKernel, edge: EdgeId) -> EdgeCirculator{
        EdgeCirculator {
            kernel: kernel,
            edge: edge,
        }
    }

    pub fn edge(&'l self) -> &'l HalfEdge { &self.kernel[self.edge] }

    pub fn next(self) -> EdgeCirculator<'l> {
        EdgeCirculator {
            kernel: self.kernel,
            edge: self.edge().next,
        }
    }

    pub fn prev(self) -> EdgeCirculator<'l> {
        EdgeCirculator {
            kernel: self.kernel,
            edge: self.edge().prev,
        }
    }

    pub fn advance(self, direction: Direction) -> EdgeCirculator<'l> {
        match direction {
            Direction::Forward => self.next(),
            Direction::Backward => self.prev(),
        }
    }

    pub fn edge_id(&self) -> EdgeId { self.edge }

    pub fn vertex_id(&self) -> VertexId { self.edge().vertex }

    pub fn face_id(&self) -> FaceId { self.edge().face }
}

impl<'l> PartialEq<EdgeCirculator<'l>> for EdgeCirculator<'l> {
    fn eq(&self, other: &EdgeCirculator) -> bool {
        return self.edge.eq(&other.edge);
    }
    fn ne(&self, other: &EdgeCirculator) -> bool {
        return self.edge.ne(&other.edge);
    }
}

#[derive(Copy, Clone)]
pub struct DirectedEdgeCirculator<'l> {
    circulator: EdgeCirculator<'l>,
    direction: Direction,
}

impl<'l> DirectedEdgeCirculator<'l> {
    pub fn new(kernel: &'l ConnectivityKernel, edge: EdgeId, direction: Direction) -> DirectedEdgeCirculator {
        DirectedEdgeCirculator {
            circulator: EdgeCirculator::new(kernel, edge),
            direction: direction,
        }
    }

    pub fn edge(&'l self) -> &'l HalfEdge { self.circulator.edge() }

    pub fn next(self) -> DirectedEdgeCirculator<'l> {
        DirectedEdgeCirculator {
            circulator: self.circulator.advance(self.direction),
            direction: self.direction,
        }
    }

    pub fn prev(self) -> DirectedEdgeCirculator<'l> {
        DirectedEdgeCirculator {
            circulator: self.circulator.advance(self.direction.reverse()),
            direction: self.direction,
        }
    }

    pub fn advance(self, direction: Direction) -> DirectedEdgeCirculator<'l> {
        match self.direction == direction {
            true => self.next(),
            false => self.prev(),
        }
    }

    pub fn edge_id(&self) -> EdgeId { self.circulator.edge }

    pub fn vertex_id(&self) -> VertexId { self.circulator.vertex_id() }

    pub fn face_id(&self) -> FaceId { self.circulator.face_id() }

    pub fn direction(&self) -> Direction { self.direction }

    pub fn set_direction(&mut self, direction: Direction) { self.direction = direction; }
}

impl<'l> PartialEq<DirectedEdgeCirculator<'l>> for DirectedEdgeCirculator<'l> {
    fn eq(&self, other: &DirectedEdgeCirculator) -> bool {
        return self.circulator.edge.eq(&other.circulator.edge);
    }
    fn ne(&self, other: &DirectedEdgeCirculator) -> bool {
        return self.circulator.edge.ne(&other.circulator.edge);
    }
}

