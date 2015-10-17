use std::f32::consts::PI;
use std::mem::{ swap, transmute };
use std::fmt::Debug;
use half_edge::kernel::{ ConnectivityKernel, vertex_id, EdgeId, VertexId, FaceId };
use half_edge::kernel;
use vodk_id::id_vector::IdVector;

use vodk_math::vec2::*;
use monotone::directed_angle;
use mem::{ Allocation, pre_allocate };

use std::slice;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum WindingOrder {
    Clockwise,
    CounterClockwise,
    Unknown,
}

#[repr(u16)]
#[derive(Copy, Clone, PartialEq, Debug)]
enum OpType {
    MoveTo,
    LineTo,
    CubicBezierTo,
    BezierTo,
    Close,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum PathOperation {
    MoveTo(VertexId),
    LineTo(VertexId),
    CubicBezierTo(VertexId, VertexId),
    BezierTo(VertexId, VertexId, VertexId),
    Close,
}

pub use self::PathOperation::*;

impl PathOperation {
    fn op_type(&self) -> OpType {
        return match self {
            &MoveTo(_) => OpType::MoveTo,
            &LineTo(_) => OpType::LineTo,
            &CubicBezierTo(_, _) => OpType::CubicBezierTo,
            &BezierTo(_, _, _) => OpType::BezierTo,
            &Close => OpType::Close,
        };
    }

    pub fn params<'l>(&'l self) -> &'l[VertexId] {
        return unsafe { match self {
            &MoveTo(ref to) => { slice::from_raw_parts(to, 1) }
            &LineTo(ref to) => { slice::from_raw_parts(to, 1) }
            &CubicBezierTo(ref to, _) => { slice::from_raw_parts(to, 2) }
            &BezierTo(ref to, _, _) => { slice::from_raw_parts(to, 3) }
            &Close => { &[] }
        }}
    }
}

pub struct PathBuilder {
    path: Vec<u16>,
}

pub struct Path {
    data: Vec<u16>,
    winding: WindingOrder,
}

/// A slightly more compact representation of a sequence of immutable path operations
/// than just a simple vector, with the winding order pre-computed
impl Path {
    pub fn iter<'l>(&'l self) -> PathIter<'l> { PathIter { path: &self.data[..] } }

    pub fn winding_order(&self) -> WindingOrder { self.winding }

    pub fn recycle(self) -> Allocation { Allocation::from_vec(self.data) }

    pub fn apply_to_kernel(
        &self,
        kernel: &mut ConnectivityKernel,
        edge_attributes: &mut EdgeAttributeVector,
        mut face_in: FaceId,
        mut face_out: FaceId
    ) -> kernel::EdgeId {
        if self.winding == WindingOrder::CounterClockwise {
            swap(&mut face_in, &mut face_out);
        }

        let mut current_edge = kernel::NO_EDGE;
        let mut first_edge = kernel::NO_EDGE;
        let mut prev_vertex = kernel::NO_VERTEX;
        for op in self.iter() {
            match op {
                MoveTo(to) => {
                    prev_vertex = to;
                }
                LineTo(to)
                | CubicBezierTo(to, _)
                | BezierTo(to, _, _) => {
                    if current_edge != kernel::NO_EDGE {
                        current_edge = kernel.extrude_vertex(current_edge, to);
                    } else {
                        debug_assert!(prev_vertex != kernel::NO_VERTEX);
                        current_edge = kernel.add_segment(prev_vertex, to, face_in);
                    }
                }
                Close => {
                    kernel.connect_edges(current_edge, first_edge, Some(face_out));
                }
            }
            if first_edge == kernel::NO_EDGE {
                first_edge = current_edge;
            }
        }

        return first_edge;
    }

}

pub type VertexAttributeVector<T> = IdVector<VertexId, Vector2D<T>>;
pub type EdgeAttributeVector = IdVector<EdgeId, EdgeType>;

enum EdgeType {
    Line,
    CubicBezier(VertexId),
    Bezier(VertexId, VertexId),
}

impl PathBuilder {
    pub fn with_alloc(alloc: Allocation, begin: VertexId) -> PathBuilder {
        let mut builder = PathBuilder { path: alloc.into_vec() };
        debug_assert!(builder.path.is_empty());
        builder.push_op(OpType::MoveTo);
        builder.push_vertex(begin);
        return builder;
    }

    pub fn begin(at: VertexId) -> PathBuilder {
        let mut builder = PathBuilder { path: Vec::with_capacity(32) };
        debug_assert!(builder.path.is_empty());
        builder.push_op(OpType::MoveTo);
        builder.push_vertex(at);
        return builder;
    }

    pub fn line_to(mut self, to: VertexId) -> PathBuilder {
        self.push_op(OpType::LineTo);
        self.push_vertex(to);
        return self;
    }

    pub fn bezier_to(mut self, cp1: VertexId, cp2: VertexId, to: VertexId) -> PathBuilder {
        self.push_op(OpType::BezierTo);
        self.push_vertex(cp1);
        self.push_vertex(cp2);
        self.push_vertex(to);
        return self;
    }

    pub fn cubic_bezier_to(mut self, cp: VertexId, to: VertexId) -> PathBuilder {
        self.push_op(OpType::CubicBezierTo);
        self.push_vertex(cp);
        self.push_vertex(to);
        return self;
    }

    pub fn close(mut self) -> PathBuilder {
        self.push_op(OpType::Close);
        return self;
    }

    pub fn clear(&mut self) {
        self.path.clear();
    }

    pub fn iter<'l>(&'l self) -> PathIter<'l> {
        PathIter { path: &self.path[..] }
    }

    pub fn into_path<T: Copy>(self, vertices: &VertexAttributeVector<T>) -> Path {
        let winding = compute_winding_order(self.iter(), vertices);
        Path {
            data: self.path,
            winding: winding,
        }
    }

    fn push_vertex(&mut self, id: VertexId) {
        self.path.push(
            unsafe { transmute(id) }
        );
    }

    fn push_op(&mut self, op: OpType) {
        self.path.push(
            unsafe { transmute(op) }
        );
    }
}

pub fn compute_winding_order<'l, IT:Iterator<Item = PathOperation>, T:Copy>(
    path: IT,
    vertices: &'l VertexAttributeVector<T>
) -> WindingOrder {
    let mut accum_angle = 0.0;
    let mut vertex_count = 0;
    let mut prev = Vector2D::new(0.0, 0.0);
    let mut prev_prev = Vector2D::new(0.0, 0.0);
    let mut first = Vector2D::new(0.0, 0.0);
    let mut second = Vector2D::new(0.0, 0.0);
    let mut is_closed = false;
    for op in path {
        for &p in op.params() {
            let vertex = vertices[p];
            if vertex_count >= 2 {
                accum_angle += directed_angle(prev_prev - prev, vertex - prev);
                prev_prev = prev;
                prev = vertex
            } else if vertex_count == 0 {
                prev_prev = vertex;
                first = vertex;
            } else if vertex_count == 1 {
                prev = vertex;
                second = vertex;
            }
            vertex_count += 1;
        }
        if op == Close {
            if first != prev {
                accum_angle += directed_angle(prev_prev - prev, first - prev);
                vertex_count += 1;
            }
            accum_angle += directed_angle(prev - first, second - first);
            is_closed = true;
        }
    }

    if !is_closed || vertex_count < 3 {
        return WindingOrder::Unknown;
    }

    //println!("accum: {} vertex_count: {}", accum_angle, vertex_count);
    return if accum_angle > ((vertex_count-1) as f32) * PI { WindingOrder::Clockwise }
           else { WindingOrder::CounterClockwise };
}

pub struct PathIter<'l> {
    path: &'l[u16]
}

impl<'l> Iterator for PathIter<'l> {
    type Item = PathOperation;
    fn next(&mut self) -> Option<PathOperation> {
        if self.path.len() == 0 { return None; }
        let op = unsafe {
            transmute(self.path[0])
        };

        let result;
        let advance;
        match op {
            OpType::Close => {
                result = Close;
                advance = 1;
            }
            OpType::MoveTo => {
                result = MoveTo(vertex_id(self.path[1]));
                advance = 2;
            }
            OpType::LineTo => {
                result = LineTo(vertex_id(self.path[1]));
                advance = 2;
            }
            OpType::CubicBezierTo => {
                result = CubicBezierTo(
                    vertex_id(self.path[1]),
                    vertex_id(self.path[2])
                );
                advance = 3;
            }
            OpType::BezierTo => {
                result = BezierTo(
                    vertex_id(self.path[1]),
                    vertex_id(self.path[2]),
                    vertex_id(self.path[3])
                );
                advance = 4;
            }
        }
        self.path = &self.path[advance..];
        return Some(result);
    }
}


#[cfg(test)]
fn assert_path_ops(path: &Path, expected: &[PathOperation]) {
    let mut i = 0;
    for op in path.iter() {
        assert_eq!(op, expected[i]);
        i += 1;
    }
    assert_eq!(i, expected.len());
}

#[test]
fn test_path_op() {
    let a = vertex_id(0);
    let b = vertex_id(1);
    let c = vertex_id(2);

    let move_to = MoveTo(a);
    let line_to = LineTo(a);
    let cubic_to = CubicBezierTo(a, b);
    let bezier_to = BezierTo(a, b, c);
    let close = Close;

    assert_eq!(move_to.params(), &[a]);
    assert_eq!(line_to.params(), &[a]);
    assert_eq!(cubic_to.params(), &[a, b]);
    assert_eq!(bezier_to.params(), &[a, b, c]);
    assert_eq!(close.params(), &[]);

    assert_eq!(move_to.op_type(), OpType::MoveTo);
    assert_eq!(line_to.op_type(), OpType::LineTo);
    assert_eq!(cubic_to.op_type(), OpType::CubicBezierTo);
    assert_eq!(bezier_to.op_type(), OpType::BezierTo);
    assert_eq!(close.op_type(), OpType::Close);
}

#[test]
fn test_simple_paths_winding() {
    use vodk_math::units::world;

    //let mut kernel = ConnectivityKernel::new();
    let mut vertices = VertexAttributeVector::new();
    let a = vertices.push(world::vec2(0.0, 0.0));
    let b = vertices.push(world::vec2(1.0, 0.0));
    let c = vertices.push(world::vec2(1.0, 1.0));
    let d = vertices.push(world::vec2(0.0, 1.0));

    // Simple closed triangle path.
    let path = PathBuilder::begin(a).line_to(b).line_to(c).close().into_path(&vertices);
    assert_path_ops(&path, &[MoveTo(a), LineTo(b), LineTo(c), Close]);
    assert_eq!(path.winding_order(), WindingOrder::Clockwise);


    // Closed path with finishes with the first point explicitly.
    let path = PathBuilder::begin(a)
        .line_to(b).line_to(c).line_to(d).line_to(a).close()
        .into_path(&vertices);
    assert_path_ops(&path, &[MoveTo(a), LineTo(b), LineTo(c), LineTo(d), LineTo(a), Close]);
    assert_eq!(path.winding_order(), WindingOrder::Clockwise);


    // Same as previous path but does not LinTo back to a (so closing the path
    // has to account for an extra line).
    let path = PathBuilder::begin(a).line_to(b).line_to(c).line_to(d).close().into_path(&vertices);
    assert_path_ops(&path, &[MoveTo(a), LineTo(b), LineTo(c), LineTo(d), Close]);
    assert_eq!(path.winding_order(), WindingOrder::Clockwise);


    // Same path with reverse winding order.
    let path = PathBuilder::begin(d).line_to(c).line_to(b).line_to(a).close().into_path(&vertices);
    assert_path_ops(&path, &[MoveTo(d), LineTo(c), LineTo(b), LineTo(a), Close]);
    assert_eq!(path.winding_order(), WindingOrder::CounterClockwise);


    // Non-closed path.
    let path = PathBuilder::begin(a).line_to(b).line_to(c).into_path(&vertices);
    assert_path_ops(&path, &[MoveTo(a), LineTo(b), LineTo(c)]);
    assert_eq!(path.winding_order(), WindingOrder::Unknown);


    // Just one segment with a close operation at the end. Can't actually close the
    // path because you need at least 3 points.
    let path = PathBuilder::begin(a).line_to(b).close().into_path(&vertices);
    assert_path_ops(&path, &[MoveTo(a), LineTo(b), Close]);
    assert_eq!(path.winding_order(), WindingOrder::Unknown);


    // Simple Cubic bezier
    let path = PathBuilder::begin(a).cubic_bezier_to(b, c).close().into_path(&vertices);
    assert_path_ops(&path, &[MoveTo(a), CubicBezierTo(b, c), Close]);
    assert_eq!(path.winding_order(), WindingOrder::Clockwise);


    // Simple bezier
    let path = PathBuilder::begin(a).bezier_to(b, c, d).close().into_path(&vertices);
    assert_path_ops(&path, &[MoveTo(a), BezierTo(b, c, d), Close]);
    assert_eq!(path.winding_order(), WindingOrder::Clockwise);


    // Simple bezier
    let path = PathBuilder::begin(a).bezier_to(b, c, a).close().into_path(&vertices);
    assert_path_ops(&path, &[MoveTo(a), BezierTo(b, c, a), Close]);
    assert_eq!(path.winding_order(), WindingOrder::Clockwise);
}

#[test]
fn test_simple_paths_kernel() {
    use vodk_math::units::world;

    let mut kernel = ConnectivityKernel::new();
    let mut vertices = VertexAttributeVector::new();
    let mut edges = EdgeAttributeVector::new();

    let a = vertices.push(world::vec2(0.0, 0.0));
    let b = vertices.push(world::vec2(1.0, 0.0));
    let c = vertices.push(world::vec2(1.0, 1.0));
    let d = vertices.push(world::vec2(0.0, 1.0));

    let face = kernel.add_face();

    // Simple closed triangle path.
    let path = PathBuilder::begin(a).line_to(b).line_to(c).close().into_path(&vertices);
    let edge = path.apply_to_kernel(&mut kernel, &mut edges, face, kernel::NO_FACE);
    assert_eq!(kernel.walk_edge_ids(edge).count(), 3);
}

#[test]
fn test_path_recycle() {
    use vodk_math::units::world;

    //let mut kernel = ConnectivityKernel::new();
    let mut vertices = VertexAttributeVector::new();
    let a = vertices.push(world::vec2(0.0, 0.0));
    let b = vertices.push(world::vec2(1.0, 0.0));
    let c = vertices.push(world::vec2(1.0, 1.0));
    let d = vertices.push(world::vec2(0.0, 1.0));

    let alloc = pre_allocate(128);

    // Simple closed triangle path.
    let alloc = PathBuilder::with_alloc(alloc, a).line_to(b).line_to(c).close().into_path(&vertices).recycle();
    let alloc = PathBuilder::with_alloc(alloc, a).line_to(b).line_to(c).close().into_path(&vertices).recycle();
    let alloc = PathBuilder::with_alloc(alloc, a).line_to(b).line_to(c).close().into_path(&vertices).recycle();
    assert_eq!(alloc.capacity(), 128);
}