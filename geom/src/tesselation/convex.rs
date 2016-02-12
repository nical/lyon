#![allow(dead_code)]

use tesselation::vertex_builder::VertexBufferBuilder;
use tesselation::{ Index };

use vodk_math::{ Vector2D, Rectangle };

use std::f32::consts::PI;

pub struct RoundedRectangle<U> {
    rect: Rectangle<U>,
    top_left_radius: f32,
    top_right_radius: f32,
    bottom_left_radius: f32,
    bottom_right_radius: f32,
}

pub fn emit_rectangle<U, Output: VertexBufferBuilder<Vector2D<U>>>(
    rect: &Rectangle<U>,
    output: &mut Output,
) {
    output.begin_geometry();
    let a = output.push_vertex(rect.top_left());
    let b = output.push_vertex(rect.top_right());
    let c = output.push_vertex(rect.bottom_right());
    let d = output.push_vertex(rect.bottom_left());
    output.push_indices(a, b, c);
    output.push_indices(a, c, d);
}

pub fn emit_rounded_rectangle<U, Output: VertexBufferBuilder<Vector2D<U>>>(
    _rect: &RoundedRectangle<U>,
    output: &mut Output
) {
    output.begin_geometry();
    panic!("TODO!");
}

pub fn emit_ellipsis<U, Output: VertexBufferBuilder<Vector2D<U>>>(
    center: Vector2D<U>,
    radius: Vector2D<U>,
    num_vertices: u32,
    output: &mut Output
) {
    output.begin_geometry();
    output.push_vertex(center);
    for i in 0..num_vertices {
        let angle = i as f32 * 2.0 * PI / ((num_vertices-1) as f32);
        output.push_vertex(center + Vector2D::new(radius.x*angle.cos(), radius.y*angle.sin()));
    }
    for i in 1..((num_vertices) as Index) {
        output.push_indices(0, i, (i-1)%num_vertices as Index+2);
    }
}
