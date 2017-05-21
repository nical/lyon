#![allow(dead_code)]

//! Tessellation routines for simple shapes.

use core::FlattenedEvent;
use geometry_builder::{GeometryBuilder, Count, VertexId};
use path_stroke::{StrokeOptions, StrokeTessellator};
use math_utils::compute_normal;
use math::*;
use {FillVertex, StrokeVertex, Side};

use std::f32::consts::PI;

/// Tessellate a triangle.
pub fn fill_triangle<Output: GeometryBuilder<FillVertex>>(
    v1: Point,
    v2: Point,
    v3: Point,
    output: &mut Output,
) -> Count {
    output.begin_geometry();

    let a = output.add_vertex(
        FillVertex {
            position: v1,
            normal: compute_normal(v1 - v3, v2 - v1),
        }
    );
    let b = output.add_vertex(
        FillVertex {
            position: v2,
            normal: compute_normal(v2 - v1, v3 - v2),
        }
    );
    let c = output.add_vertex(
        FillVertex {
            position: v3,
            normal: compute_normal(v3 - v2, v1 - v3),
        }
    );

    output.add_triangle(a, b, c);

    return output.end_geometry();
}

/// Tessellate the stroke of a triangle.
pub fn stroke_triangle<Output: GeometryBuilder<StrokeVertex>>(
    v1: Point,
    v2: Point,
    v3: Point,
    output: &mut Output,
) -> Count {
    output.begin_geometry();

    let na = compute_normal(v1 - v3, v2 - v1);
    let nb = compute_normal(v2 - v1, v3 - v2);
    let nc = compute_normal(v3 - v2, v1 - v3);

    let a1 = output.add_vertex(
        StrokeVertex {
            position: v1,
            normal: -na,
            side: Side::Right,
        }
    );
    let a2 = output.add_vertex(
        StrokeVertex {
            position: v1,
            normal: na,
            side: Side::Left,
        }
    );

    let b1 = output.add_vertex(
        StrokeVertex {
            position: v2,
            normal: -nb,
            side: Side::Right,
        }
    );
    let b2 = output.add_vertex(
        StrokeVertex {
            position: v2,
            normal: nb,
            side: Side::Left,
        }
    );

    let c1 = output.add_vertex(
        StrokeVertex {
            position: v3,
            normal: -nc,
            side: Side::Right,
        }
    );
    let c2 = output.add_vertex(
        StrokeVertex {
            position: v3,
            normal: nc,
            side: Side::Left,
        }
    );

    output.add_triangle(a1, a2, b2);
    output.add_triangle(a2, b2, b1);
    output.add_triangle(b1, b2, c1);
    output.add_triangle(b2, c2, c1);
    output.add_triangle(c1, c2, a1);
    output.add_triangle(c2, a2, a1);

    return output.end_geometry();
}


/// Tessellate a quad.
pub fn fill_quad<Output: GeometryBuilder<FillVertex>>(
    v1: Point,
    v2: Point,
    v3: Point,
    v4: Point,
    output: &mut Output,
) -> Count {
    output.begin_geometry();

    let a = output.add_vertex(
        FillVertex {
            position: v1,
            normal: compute_normal(v1 - v4, v2 - v1),
        }
    );
    let b = output.add_vertex(
        FillVertex {
            position: v2,
            normal: compute_normal(v2 - v1, v3 - v2),
        }
    );
    let c = output.add_vertex(
        FillVertex {
            position: v3,
            normal: compute_normal(v3 - v2, v4 - v3),
        }
    );
    let d = output.add_vertex(
        FillVertex {
            position: v4,
            normal: compute_normal(v4 - v3, v1 - v4),
        }
    );
    output.add_triangle(a, b, c);
    output.add_triangle(a, c, d);

    return output.end_geometry();
}

/// Tessellate the stroke of a quad.
pub fn stroke_quad<Output: GeometryBuilder<StrokeVertex>>(
    v1: Point,
    v2: Point,
    v3: Point,
    v4: Point,
    output: &mut Output,
) -> Count {
    output.begin_geometry();

    let na = compute_normal(v1 - v4, v2 - v1);
    let nb = compute_normal(v2 - v1, v3 - v2);
    let nc = compute_normal(v3 - v2, v4 - v3);
    let nd = compute_normal(v4 - v3, v1 - v4);

    let a1 = output.add_vertex(
        StrokeVertex {
            position: v1,
            normal: -na,
            side: Side::Right,
        }
    );
    let a2 = output.add_vertex(
        StrokeVertex {
            position: v1,
            normal: na,
            side: Side::Left,
        }
    );

    let b1 = output.add_vertex(
        StrokeVertex {
            position: v2,
            normal: -nb,
            side: Side::Right,
        }
    );
    let b2 = output.add_vertex(
        StrokeVertex {
            position: v2,
            normal: nb,
            side: Side::Left,
        }
    );

    let c1 = output.add_vertex(
        StrokeVertex {
            position: v3,
            normal: -nc,
            side: Side::Right,
        }
    );
    let c2 = output.add_vertex(
        StrokeVertex {
            position: v3,
            normal: nc,
            side: Side::Left,
        }
    );

    let d1 = output.add_vertex(
        StrokeVertex {
            position: v4,
            normal: -nc,
            side: Side::Right,
        }
    );
    let d2 = output.add_vertex(
        StrokeVertex {
            position: v4,
            normal: nd,
            side: Side::Left,
        }
    );

    output.add_triangle(a1, a2, b2);
    output.add_triangle(a2, b2, b1);
    output.add_triangle(b1, b2, c1);
    output.add_triangle(b2, c2, c1);
    output.add_triangle(c1, c2, d1);
    output.add_triangle(c2, d2, d1);
    output.add_triangle(d1, d2, a1);
    output.add_triangle(d2, a2, a1);

    return output.end_geometry();
}

/// Tessellate an axis-aligned rectangle.
pub fn fill_rectangle<Output: GeometryBuilder<FillVertex>>(
    rect: &Rect,
    output: &mut Output,
) -> Count {
    output.begin_geometry();

    let a = output.add_vertex(
        FillVertex {
            position: rect.origin,
            normal: vec2(-1.0, -1.0),
        }
    );
    let b = output.add_vertex(
        FillVertex {
            position: rect.top_right(),
            normal: vec2(1.0, -1.0),
        }
    );
    let c = output.add_vertex(
        FillVertex {
            position: rect.bottom_right(),
            normal: vec2(1.0, 1.0),
        }
    );
    let d = output.add_vertex(
        FillVertex {
            position: rect.bottom_left(),
            normal: vec2(-1.0, 1.0),
        }
    );
    output.add_triangle(a, b, c);
    output.add_triangle(a, c, d);

    return output.end_geometry();
}

/// Tessellate the stroke for an axis-aligne rectangle.
pub fn stroke_rectangle<Output: GeometryBuilder<StrokeVertex>>(
    rect: &Rect,
    output: &mut Output,
) -> Count {
    output.begin_geometry();

    let a1 = output.add_vertex(
        StrokeVertex {
            position: rect.origin,
            normal: -vec2(-1.0, -1.0),
            side: Side::Right,
        }
    );
    let a2 = output.add_vertex(
        StrokeVertex {
            position: rect.origin,
            normal: vec2(-1.0, -1.0),
            side: Side::Left,
        }
    );

    let b1 = output.add_vertex(
        StrokeVertex {
            position: rect.top_right(),
            normal: -vec2(1.0, -1.0),
            side: Side::Right,
        }
    );
    let b2 = output.add_vertex(
        StrokeVertex {
            position: rect.top_right(),
            normal: vec2(1.0, -1.0),
            side: Side::Left,
        }
    );

    let c1 = output.add_vertex(
        StrokeVertex {
            position: rect.bottom_right(),
            normal: -vec2(1.0, 1.0),
            side: Side::Right,
        }
    );
    let c2 = output.add_vertex(
        StrokeVertex {
            position: rect.bottom_right(),
            normal: vec2(1.0, 1.0),
            side: Side::Left,
        }
    );

    let d1 = output.add_vertex(
        StrokeVertex {
            position: rect.bottom_left(),
            normal: -vec2(1.0, 0.0),
            side: Side::Right,
        }
    );
    let d2 = output.add_vertex(
        StrokeVertex {
            position: rect.bottom_left(),
            normal: vec2(1.0, 0.0),
            side: Side::Left,
        }
    );

    output.add_triangle(a1, a2, b2);
    output.add_triangle(a2, b2, b1);
    output.add_triangle(b1, b2, c1);
    output.add_triangle(b2, c2, c1);
    output.add_triangle(c1, c2, d1);
    output.add_triangle(c2, d2, d1);
    output.add_triangle(d1, d2, a1);
    output.add_triangle(d2, a2, a1);

    return output.end_geometry();
}

/// An axis-aligned rounded rectangle.
pub struct RoundedRect {
    pub rect: Rect,
    pub top_left_radius: f32,
    pub top_right_radius: f32,
    pub bottom_left_radius: f32,
    pub bottom_right_radius: f32,
}

/// Tessellate an axis-aligned rounded rectangle.
pub fn fill_rounded_rectangle<Output: GeometryBuilder<FillVertex>>(
    _rect: &RoundedRect,
    _output: &mut Output,
) -> Count {
    unimplemented!();
}

/// Tessellate the stroke of an axis-aligned rounded rectangle.
pub fn stroke_rounded_rectangle<Output: GeometryBuilder<StrokeVertex>>(
    _rect: &RoundedRect,
    _output: &mut Output,
) -> Count {
    unimplemented!();
}

/// Tessellate an ellipsis.
pub fn fill_ellipse<Output: GeometryBuilder<FillVertex>>(
    center: Point,
    radius: Vec2,
    num_vertices: u32, // TODO: use a tolerance instead?
    output: &mut Output,
) -> Count {
    output.begin_geometry();
    let c = output.add_vertex(
        FillVertex {
            position: center,
            normal: vec2(0.0, 0.0),
        }
    );
    for i in 0..num_vertices {
        let angle = i as f32 * 2.0 * PI / ((num_vertices - 1) as f32);
        output.add_vertex(
            FillVertex {
                position: center + vec2(radius.x * angle.cos(), radius.y * angle.sin()),
                normal: vec2(0.0, 0.0), // TODO
            }
        );
    }
    for i in 1..((num_vertices) as u16) {
        output.add_triangle(c, VertexId(i), VertexId((i - 1) % num_vertices as u16 + 2));
    }
    return output.end_geometry();
}

/// Tessellate a convex polyline.
///
/// TODO: normals are not implemented yet.
pub fn fill_convex_polyline<Iter, Output>(mut it: Iter, output: &mut Output) -> Count
where
    Iter: Iterator<Item = Point>,
    Output: GeometryBuilder<FillVertex>,
{
    output.begin_geometry();
    if let (Some(first), Some(second)) = (it.next(), it.next()) {
        let mut a = output.add_vertex(
            FillVertex {
                position: first,
                normal: vec2(0.0, 0.0), // TODO
            }
        );
        let mut b = output.add_vertex(
            FillVertex {
                position: second,
                normal: vec2(0.0, 0.0), // TODO
            }
        );

        for point in it {
            let c = output.add_vertex(
                FillVertex {
                    position: point,
                    normal: vec2(0.0, 0.0), // TODO
                }
            );

            output.add_triangle(a, b, c);
            a = b;
            b = c;
        }
    }
    return output.end_geometry();
}

pub fn stroke_polyline<Iter, Output>(it: Iter, is_closed: bool, output: &mut Output) -> Count
where
    Iter: Iterator<Item = Point>,
    Output: GeometryBuilder<StrokeVertex>,
{
    let options = StrokeOptions::default();
    let mut tess = StrokeTessellator::new();

    return tess.tessellate(PolylineEvents::new(is_closed, it), &options, output).unwrap();
}

// TODO: This should be in path_iterator but it creates a dependency.

/// An iterator that consumes an iterator of points and produces FlattenedEvents.
pub struct PolylineEvents<Iter> {
    iter: Iter,
    first: bool,
    done: bool,
    close: bool,
}

impl<Iter: Iterator<Item = Point>> PolylineEvents<Iter> {
    pub fn new(closed: bool, iter: Iter) -> Self {
        PolylineEvents {
            iter: iter,
            first: true,
            done: false,
            close: closed,
        }
    }

    pub fn closed(iter: Iter) -> Self { PolylineEvents::new(true, iter) }

    pub fn open(iter: Iter) -> Self { PolylineEvents::new(false, iter) }
}

impl<Iter> Iterator for PolylineEvents<Iter>
where
    Iter: Iterator<Item = Point>,
{
    type Item = FlattenedEvent;

    fn next(&mut self) -> Option<FlattenedEvent> {
        if self.done {
            return None;
        }

        if let Some(next) = self.iter.next() {
            return Some(
                if self.first {
                    self.first = false;
                    FlattenedEvent::MoveTo(next)
                } else {
                    FlattenedEvent::LineTo(next)
                }
            );
        }

        self.done = true;
        if self.close {
            return Some(FlattenedEvent::Close);
        }

        return None;
    }
}

// Returns the maximum length of individual line segments when approximating a
// circle.
//
// From pythagora's theorem:
// r² = (d/2)² + (r - t)²
// r² = d²/4 + r² + t² - 2 * e * r
// d² = 4 * (2 * t * r - t²)
// d = 2 * sqrt(2 * t * r - t²)
//
// With:
//  r: the radius
//  t: the tolerance threshold
//  d: the line segment length
fn circle_flattening_step(radius:f32, tolerance: f32) -> f32 {
    2.0 * (2.0 * tolerance * radius - tolerance * tolerance).sqrt()
}

#[test]
fn test_polyline_events_open() {
    let points = &[
        point(1.0, 1.0),
        point(3.0, 1.0),
        point(4.0, 5.0),
        point(5.0, 2.0),
    ];

    let mut evts = PolylineEvents::open(points.iter().cloned());

    assert_eq!(evts.next(), Some(FlattenedEvent::MoveTo(point(1.0, 1.0))));
    assert_eq!(evts.next(), Some(FlattenedEvent::LineTo(point(3.0, 1.0))));
    assert_eq!(evts.next(), Some(FlattenedEvent::LineTo(point(4.0, 5.0))));
    assert_eq!(evts.next(), Some(FlattenedEvent::LineTo(point(5.0, 2.0))));
    assert_eq!(evts.next(), None);
}

#[test]
fn test_polyline_events_closed() {
    let points = &[
        point(1.0, 1.0),
        point(3.0, 1.0),
        point(4.0, 5.0),
        point(5.0, 2.0),
    ];

    let mut evts = PolylineEvents::closed(points.iter().cloned());

    assert_eq!(evts.next(), Some(FlattenedEvent::MoveTo(point(1.0, 1.0))));
    assert_eq!(evts.next(), Some(FlattenedEvent::LineTo(point(3.0, 1.0))));
    assert_eq!(evts.next(), Some(FlattenedEvent::LineTo(point(4.0, 5.0))));
    assert_eq!(evts.next(), Some(FlattenedEvent::LineTo(point(5.0, 2.0))));
    assert_eq!(evts.next(), Some(FlattenedEvent::Close));
}
