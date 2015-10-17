use std::f32::consts::PI;
use std::mem::{ swap, transmute };
use std::fmt::Debug;
use half_edge::kernel::{ ConnectivityKernel, VertexId, FaceId };
use half_edge::kernel;
use vodk_id::id_vector::IdVector;

use vodk_math::vec2::*;
use monotone::directed_angle;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum WindingOrder {
    Clockwise,
    CounterClockwise,
    Unknown,
}


#[repr(u16)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum PathOp {
    MoveTo,
    LineTo,
    BezierTo,
    CubicBezierTo,
    Close,
}

pub struct PathBuilder {
    path: Vec<u16>,
}

pub struct PathView<'l> {
    path: &'l[u16]
}

pub struct PathObject {
    data: Vec<u16>,
    winding: WindingOrder,
}

impl PathObject {
    pub fn view<'l>(&'l self) -> PathView<'l> { PathView { path: &self.data[..] } }

    pub fn iter<'l>(&'l self) -> PathIter<'l> { PathIter { path: &self.data[..] } }

    pub fn winding_order(&self) -> WindingOrder { self.winding }

    pub fn apply_to_kernel(
        &self,
        kernel: &mut ConnectivityKernel,
        mut face_in: FaceId,
        mut face_out: FaceId
    ) -> kernel::EdgeId {
        if self.winding == WindingOrder::CounterClockwise {
            swap(&mut face_in, &mut face_out);
        }

        let mut current_edge = kernel::NO_EDGE;
        let mut first_edge = kernel::NO_EDGE;
        let mut prev_vertex = kernel::NO_VERTEX;
        for (op, params) in self.iter() {
            match op {
                PathOp::MoveTo => {
                    prev_vertex = params[0];
                }
                PathOp::LineTo | PathOp::CubicBezierTo | PathOp::BezierTo => {
                    if current_edge != kernel::NO_EDGE {
                        current_edge = kernel.extrude_vertex(current_edge, params[0]);
                    } else {
                        debug_assert!(prev_vertex != kernel::NO_VERTEX);
                        current_edge = kernel.add_segment(prev_vertex, params[0], face_in);
                    }
                }
                PathOp::Close => {
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

impl<'l> PathView<'l> {
    pub fn iter(&'l self) -> PathIter<'l> { PathIter { path: self.path } }
}

pub type VertexAttributeVector<T> = IdVector<VertexId, Vector2D<T>>;

impl PathBuilder {
    pub fn begin(at: VertexId) -> PathBuilder {
        let mut builder = PathBuilder { path: Vec::new() };
        debug_assert!(builder.path.is_empty());
        builder.push_op(PathOp::MoveTo);
        builder.push_vertex(at);
        return builder;
    }

    pub fn line_to(mut self, to: VertexId) -> PathBuilder {
        self.push_op(PathOp::LineTo);
        self.push_vertex(to);
        return self;
    }

    pub fn bezier_to(mut self, cp1: VertexId, cp2: VertexId, to: VertexId) -> PathBuilder {
        self.push_op(PathOp::BezierTo);
        self.push_vertex(cp1);
        self.push_vertex(cp2);
        self.push_vertex(to);
        return self;
    }

    pub fn cubic_bezier_to(mut self, cp: VertexId, to: VertexId) -> PathBuilder {
        self.push_op(PathOp::CubicBezierTo);
        self.push_vertex(cp);
        self.push_vertex(to);
        return self;
    }

    pub fn close(mut self) -> PathBuilder {
        self.push_op(PathOp::Close);
        return self;
    }

    pub fn clear(&mut self) {
        self.path.clear();
    }

    pub fn iter<'l>(&'l self) -> PathIter<'l> {
        PathIter { path: &self.path[..] }
    }

    pub fn view<'l>(&'l self) -> PathView<'l> {
        PathView { path: &self.path[..] }
    }

    pub fn into_path<T: Copy>(self, vertices: &VertexAttributeVector<T>) -> PathObject {
        let winding = winding_order(self.view(), vertices);
        PathObject {
            data: self.path,
            winding: winding,
        }
    }


    fn push_vertex(&mut self, id: VertexId) {
        self.path.push(
            unsafe { transmute(id) }
        );
    }

    fn push_op(&mut self, op: PathOp) {
        self.path.push(
            unsafe { transmute(op) }
        );
    }
}

pub fn winding_order<'l, T:Copy>(
    path: PathView<'l>,
    vertices: &'l VertexAttributeVector<T>
) -> WindingOrder {
    let mut accum_angle = 0.0;
    let mut vertex_count = 0;
    let mut prev = Vector2D::new(0.0, 0.0);
    let mut prev_prev = Vector2D::new(0.0, 0.0);
    let mut first = Vector2D::new(0.0, 0.0);
    let mut second = Vector2D::new(0.0, 0.0);
    let mut is_closed = false;
    for (op, params) in path.iter() {
        for &p in params {
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
        if op == PathOp::Close {
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
    type Item = (PathOp, &'l[VertexId]);
    fn next(&mut self) -> Option<(PathOp, &'l[VertexId])> {
        if self.path.len() == 0 { return None; }
        let op = unsafe {
            transmute(self.path[0])
        };

        let num_args = match op {
            PathOp::Close => { 0 }
            PathOp::MoveTo | PathOp::LineTo => { 1 }
            PathOp::CubicBezierTo => { 2 }
            PathOp::BezierTo => { 3 }
        };

        let result: &'l[VertexId] = unsafe {
            transmute(&self.path[1 .. 1+num_args])
        };

        self.path = &self.path[1+num_args ..];
        return Some((op, result));
    }
}


#[cfg(test)]
fn assert_path_ops(path: PathView, expected: &[(PathOp, &[VertexId])]) {
    let mut i = 0;
    for (op, args) in path.iter() {
        let &(expected_op, expected_args) = &expected[i];
        assert_eq!(op, expected_op);
        assert_eq!(args, expected_args);
        i += 1;
    }
    assert_eq!(i, expected.len());
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
    assert_path_ops(path.view(),&[
        (PathOp::MoveTo, &[a]),
        (PathOp::LineTo, &[b]),
        (PathOp::LineTo, &[c]),
        (PathOp::Close, &[])
    ]);
    assert_eq!(path.winding_order(), WindingOrder::Clockwise);


    // Closed path with finishes with the first point explicitly.
    let path = PathBuilder::begin(a).line_to(b).line_to(c).line_to(d).line_to(a).close().into_path(&vertices);
    assert_path_ops(path.view(),&[
        (PathOp::MoveTo, &[a]),
        (PathOp::LineTo, &[b]),
        (PathOp::LineTo, &[c]),
        (PathOp::LineTo, &[d]),
        (PathOp::LineTo, &[a]),
        (PathOp::Close, &[])
    ]);
    assert_eq!(path.winding_order(), WindingOrder::Clockwise);


    // Same as previous path but does not LinTo back to a (so closing the path
    // has to account for an extra line).
    let path = PathBuilder::begin(a).line_to(b).line_to(c).line_to(d).close().into_path(&vertices);
    assert_path_ops(path.view(),&[
        (PathOp::MoveTo, &[a]),
        (PathOp::LineTo, &[b]),
        (PathOp::LineTo, &[c]),
        (PathOp::LineTo, &[d]),
        (PathOp::Close, &[])
    ]);
    assert_eq!(path.winding_order(), WindingOrder::Clockwise);


    // Same path with reverse winding order.
    let path = PathBuilder::begin(d).line_to(c).line_to(b).line_to(a).close().into_path(&vertices);
    assert_path_ops(path.view(),&[
        (PathOp::MoveTo, &[d]),
        (PathOp::LineTo, &[c]),
        (PathOp::LineTo, &[b]),
        (PathOp::LineTo, &[a]),
        (PathOp::Close, &[])
    ]);
    assert_eq!(path.winding_order(), WindingOrder::CounterClockwise);


    // Non-closed path.
    let path = PathBuilder::begin(a).line_to(b).line_to(c).into_path(&vertices);
    assert_path_ops(path.view(),&[
        (PathOp::MoveTo, &[a]),
        (PathOp::LineTo, &[b]),
        (PathOp::LineTo, &[c]),
    ]);
    assert_eq!(path.winding_order(), WindingOrder::Unknown);


    // Just one segment with a close operation at the end. Can't actually close the
    // path because you need at least 3 points.
    let path = PathBuilder::begin(a).line_to(b).close().into_path(&vertices);
    assert_path_ops(path.view(),&[
        (PathOp::MoveTo, &[a]),
        (PathOp::LineTo, &[b]),
        (PathOp::Close, &[])
    ]);
    assert_eq!(path.winding_order(), WindingOrder::Unknown);


    // Simple Cubic bezier
    let path = PathBuilder::begin(a).cubic_bezier_to(b, c).close().into_path(&vertices);
    assert_path_ops(path.view(),&[
        (PathOp::MoveTo, &[a]),
        (PathOp::CubicBezierTo, &[b, c]),
        (PathOp::Close, &[])
    ]);
    assert_eq!(path.winding_order(), WindingOrder::Clockwise);


    // Simple bezier
    let path = PathBuilder::begin(a).bezier_to(b, c, d).close().into_path(&vertices);
    assert_path_ops(path.view(),&[
        (PathOp::MoveTo, &[a]),
        (PathOp::BezierTo, &[b, c, d]),
        (PathOp::Close, &[])
    ]);
    assert_eq!(path.winding_order(), WindingOrder::Clockwise);


    // Simple bezier
    let path = PathBuilder::begin(a).bezier_to(b, c, a).close().into_path(&vertices);
    assert_path_ops(path.view(),&[
        (PathOp::MoveTo, &[a]),
        (PathOp::BezierTo, &[b, c, a]),
        (PathOp::Close, &[])
    ]);
    assert_eq!(path.winding_order(), WindingOrder::Clockwise);
}

#[test]
fn test_simple_paths_kernel() {
    use vodk_math::units::world;

    let mut kernel = ConnectivityKernel::new();
    let mut vertices = VertexAttributeVector::new();

    let a = vertices.push(world::vec2(0.0, 0.0));
    let b = vertices.push(world::vec2(1.0, 0.0));
    let c = vertices.push(world::vec2(1.0, 1.0));
    let d = vertices.push(world::vec2(0.0, 1.0));

    let face = kernel.add_face();

    // Simple closed triangle path.
    let path = PathBuilder::begin(a).line_to(b).line_to(c).close().into_path(&vertices);
    let edge = path.apply_to_kernel(&mut kernel, face, kernel::NO_FACE);
    assert_eq!(kernel.walk_edge_ids(edge).count(), 3);
}