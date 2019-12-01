use crate::math::Point;
use crate::geometry_builder::{GeometryBuilder, VertexId};
use crate::{FillVertex, Side};

/// Helper class that generates a triangulation from a sequence of vertices describing a monotone
/// polygon (used internally by the `FillTessellator`).
pub(crate) struct MonotoneTessellator {
    stack: Vec<MonotoneVertex>,
    previous: MonotoneVertex,
    triangles: Vec<(VertexId, VertexId, VertexId)>,
}

#[derive(Copy, Clone, Debug)]
struct MonotoneVertex {
    pos: Point,
    id: VertexId,
    side: Side,
}

impl MonotoneTessellator {
    pub fn new() -> Self {
        MonotoneTessellator {
            stack: Vec::with_capacity(16),
            triangles: Vec::with_capacity(128),
            // Some placeholder value that will be replaced right away.
            previous: MonotoneVertex {
                pos: Point::new(0.0, 0.0),
                id: VertexId(0),
                side: Side::Left,
            },
        }
    }

    pub fn begin(mut self, pos: Point, id: VertexId) -> MonotoneTessellator {
        debug_assert!(id != VertexId::INVALID);
        let first = MonotoneVertex {
            pos,
            id,
            side: Side::Left,
        };
        self.previous = first;

        self.triangles.clear();
        self.stack.clear();
        self.stack.push(first);

        self
    }

    pub fn vertex(&mut self, pos: Point, id: VertexId, side: Side) {
        let current = MonotoneVertex { pos, id, side };
        debug_assert!(id != VertexId::INVALID);
        // cf. test_fixed_to_f32_precision
        // TODO: investigate whether we could do the conversion without this
        // precision issue. Otherwise we could also make MonotoneTessellator
        // manipulate fixed-point values instead of f32 to preverve the assertion.
        //
        //debug_assert!(is_after(current.pos, self.previous.pos));
        debug_assert!(!self.stack.is_empty());

        let changed_side = current.side != self.previous.side;

        if changed_side {
            for i in 0..(self.stack.len() - 1) {
                let mut a = self.stack[i];
                let mut b = self.stack[i + 1];

                let winding = (a.pos - b.pos).cross(current.pos - b.pos) >= 0.0;

                if !winding {
                    std::mem::swap(&mut a, &mut b);
                }

                self.push_triangle(&a, &b, &current);
            }
            self.stack.clear();
            self.stack.push(self.previous);
        } else {
            let mut last_popped = self.stack.pop();
            while !self.stack.is_empty() {
                let mut a = last_popped.unwrap();
                let mut b = *self.stack.last().unwrap();

                if current.side.is_right() {
                    std::mem::swap(&mut a, &mut b);
                }

                let cross = (current.pos - b.pos).cross(a.pos - b.pos);
                if cross >= 0.0 {
                    self.push_triangle(&b, &a, &current);
                    last_popped = self.stack.pop();
                } else {
                    break;
                }
            }
            if let Some(item) = last_popped {
                self.stack.push(item);
            }
        }

        self.stack.push(current);
        self.previous = current;

        return;
    }

    pub fn end(&mut self, pos: Point, id: VertexId) {
        let side = self.previous.side.opposite();
        self.vertex(pos, id, side);
        self.stack.clear();
    }

    fn push_triangle(&mut self, a: &MonotoneVertex, b: &MonotoneVertex, c: &MonotoneVertex) {
        debug_assert!(a.id != b.id);
        debug_assert!(b.id != c.id);
        debug_assert!(a.id != c.id);
        debug_assert!(a.id != VertexId::INVALID);
        debug_assert!(b.id != VertexId::INVALID);
        debug_assert!(c.id != VertexId::INVALID);

        let threshold = -0.0625; // Floating point errors stroke again :(
        debug_assert!((a.pos - b.pos).cross(c.pos - b.pos) >= threshold);
        self.triangles.push((a.id, b.id, c.id));
    }

    pub fn flush(&mut self, output: &mut dyn GeometryBuilder<FillVertex>) {
        for &(a, b, c) in &self.triangles {
            output.add_triangle(a, b, c);
        }
        self.triangles.clear();
    }

    pub fn flush_experimental(&mut self, output: &mut dyn GeometryBuilder<Point>) {
        for &(a, b, c) in &self.triangles {
            output.add_triangle(a, b, c);
        }
        self.triangles.clear();
    }
}

#[cfg(test)]
use crate::math::point;

#[test]
fn test_monotone_tess() {
    println!(" ------------ ");
    {
        let mut tess = MonotoneTessellator::new().begin(point(0.0, 0.0), VertexId(0));
        tess.vertex(point(-1.0, 1.0), VertexId(1), Side::Left);
        tess.end(point(1.0, 2.0), VertexId(2));
        assert_eq!(tess.triangles.len(), 1);
    }
    println!(" ------------ ");
    {
        let mut tess = MonotoneTessellator::new().begin(point(0.0, 0.0), VertexId(0));
        tess.vertex(point(1.0, 1.0), VertexId(1), Side::Right);
        tess.vertex(point(-1.5, 2.0), VertexId(2), Side::Left);
        tess.vertex(point(-1.0, 3.0), VertexId(3), Side::Left);
        tess.vertex(point(1.0, 4.0), VertexId(4), Side::Right);
        tess.end(point(0.0, 5.0), VertexId(5));
        assert_eq!(tess.triangles.len(), 4);
    }
    println!(" ------------ ");
    {
        let mut tess = MonotoneTessellator::new().begin(point(0.0, 0.0), VertexId(0));
        tess.vertex(point(1.0, 1.0), VertexId(1), Side::Right);
        tess.vertex(point(3.0, 2.0), VertexId(2), Side::Right);
        tess.vertex(point(1.0, 3.0), VertexId(3), Side::Right);
        tess.vertex(point(1.0, 4.0), VertexId(4), Side::Right);
        tess.vertex(point(4.0, 5.0), VertexId(5), Side::Right);
        tess.end(point(0.0, 6.0), VertexId(6));
        assert_eq!(tess.triangles.len(), 5);
    }
    println!(" ------------ ");
    {
        let mut tess = MonotoneTessellator::new().begin(point(0.0, 0.0), VertexId(0));
        tess.vertex(point(-1.0, 1.0), VertexId(1), Side::Left);
        tess.vertex(point(-3.0, 2.0), VertexId(2), Side::Left);
        tess.vertex(point(-1.0, 3.0), VertexId(3), Side::Left);
        tess.vertex(point(-1.0, 4.0), VertexId(4), Side::Left);
        tess.vertex(point(-4.0, 5.0), VertexId(5), Side::Left);
        tess.end(point(0.0, 6.0), VertexId(6));
        assert_eq!(tess.triangles.len(), 5);
    }
    println!(" ------------ ");
}
