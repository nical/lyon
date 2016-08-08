#![allow(dead_code)]

//! Tessellation routines for simple shapes.

use geometry_builder::{ GeometryBuilder, Count, VertexId };

use math::*;

use std::f32::consts::PI;

pub struct RoundedRect {
    rect: Rect,
    top_left_radius: f32,
    top_right_radius: f32,
    bottom_left_radius: f32,
    bottom_right_radius: f32,
}

/// Add a triangle to a geometry.
///
/// Does not call begin_geometry and end_geometry.
pub fn add_triangle<Output: GeometryBuilder<Point>>(
    v1: Point,
    v2: Point,
    v3: Point,
    output: &mut Output,
) {
    let a = output.add_vertex(v1);
    let b = output.add_vertex(v2);
    let c = output.add_vertex(v3);
    output.add_triangle(a, b, c);
}

/// Tessellate a simple triangle geometry.
pub fn tessellate_triangle<Output: GeometryBuilder<Point>>(
    v1: Point,
    v2: Point,
    v3: Point,
    output: &mut Output,
) -> Count {
    output.begin_geometry();
    add_triangle(v1, v2, v3, output);
    return output.end_geometry();
}

/// Add a quad to a geometry.
///
/// Does not call begin_geometry and end_geometry.
pub fn add_quad<Output: GeometryBuilder<Point>>(
    v1: Point,
    v2: Point,
    v3: Point,
    v4: Point,
    output: &mut Output,
) {
    let a = output.add_vertex(v1);
    let b = output.add_vertex(v2);
    let c = output.add_vertex(v3);
    let d = output.add_vertex(v4);
    output.add_triangle(a, b, c);
    output.add_triangle(a, c, d);
}

/// Tessellate a simple quad geometry.
pub fn tessellate_quad<Output: GeometryBuilder<Point>>(
    v1: Point,
    v2: Point,
    v3: Point,
    v4: Point,
    output: &mut Output,
) -> Count {
    output.begin_geometry();
    add_quad(v1, v2, v3, v4, output);
    return output.end_geometry()
}

/// Add a Rect to a geometry.
///
/// Does not call begin_geometry and end_geometry.
pub fn add_rectangle<Output: GeometryBuilder<Point>>(
    rect: &Rect,
    output: &mut Output,
) {
    add_quad(
        rect.origin,
        rect.top_right(),
        rect.bottom_right(),
        rect.bottom_left(),
        output
    );
}

/// Tessellate a simple Rect.
pub fn tessellate_rectangle<Output: GeometryBuilder<Point>>(
    rect: &Rect,
    output: &mut Output,
) -> Count {
    return tessellate_quad(
        rect.origin,
        rect.top_right(),
        rect.bottom_right(),
        rect.bottom_left(),
        output
    );
}

/// Add a rounded reactangle to a geometry.
///
/// Does not call begin_geometry and end_geometry.
pub fn add_rounded_rectangle<Output: GeometryBuilder<Point>>(
    _rect: &RoundedRect,
    _output: &mut Output
) -> Count {
    unimplemented!();
}

/// Tessellate a simple rounded rectangle.
pub fn tessellate_rounded_rectangle<Output: GeometryBuilder<Point>>(
    _rect: &RoundedRect,
    _output: &mut Output
) -> Count {
    unimplemented!();
}

/// Tessellate a simple ellipsis.
pub fn tessellate_ellipsis<Output: GeometryBuilder<Point>>(
    center: Point,
    radius: Vec2,
    num_vertices: u32, // TODO: use a tolerance instead?
    output: &mut Output
) -> Count {
    output.begin_geometry();
    let c = output.add_vertex(center);
    for i in 0..num_vertices {
        let angle = i as f32 * 2.0 * PI / ((num_vertices - 1) as f32);
        output.add_vertex(center + vec2(radius.x*angle.cos(), radius.y*angle.sin()));
    }
    for i in 1..((num_vertices) as u16) {
        output.add_triangle(c, VertexId(i), VertexId((i - 1)%num_vertices as u16 + 2));
    }
    return output.end_geometry()
}
