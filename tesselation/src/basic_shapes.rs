#![allow(dead_code)]

//! Tesselation routines for simple shapes.

use vertex_builder::VertexBufferBuilder;
use super::{ Index };

use math::*;

use std::f32::consts::PI;

pub struct RoundedRect {
    rect: Rect,
    top_left_radius: f32,
    top_right_radius: f32,
    bottom_left_radius: f32,
    bottom_right_radius: f32,
}

pub fn tesselate_triangle<Input, Output: VertexBufferBuilder<Input>>(
    v1: Input,
    v2: Input,
    v3: Input,
    output: &mut Output,
) {
    output.begin_geometry();
    let a = output.push_vertex(v1);
    let b = output.push_vertex(v2);
    let c = output.push_vertex(v3);
    output.push_indices(a, b, c);
}

pub fn tesselate_quad<Input, Output: VertexBufferBuilder<Input>>(
    v1: Input,
    v2: Input,
    v3: Input,
    v4: Input,
    output: &mut Output,
) {
    output.begin_geometry();
    let a = output.push_vertex(v1);
    let b = output.push_vertex(v2);
    let c = output.push_vertex(v3);
    let d = output.push_vertex(v4);
    output.push_indices(a, b, c);
    output.push_indices(a, c, d);
}

pub fn tesselate_rectangle<Output: VertexBufferBuilder<Vec2>>(
    rect: &Rect,
    output: &mut Output,
) {
    tesselate_quad(
        rect.origin,
        rect.top_right(),
        rect.bottom_right(),
        rect.bottom_left(),
        output
    );
}

pub fn tesselate_rectangle_with_uv<A, Output: VertexBufferBuilder<(Vec2, Vec2)>>(
    rect: &Rect,
    uv: &Rect,
    output: &mut Output,
) {
    tesselate_quad(
        (rect.origin, uv.origin),
        (rect.top_right(), uv.top_right()),
        (rect.bottom_right(), uv.bottom_right()),
        (rect.bottom_left(), uv.bottom_left()),
        output
    );
}

pub fn tesselate_rounded_rectangle<Output: VertexBufferBuilder<Vec2>>(
    _rect: &RoundedRect,
    output: &mut Output
) {
    output.begin_geometry();
    unimplemented!()
}

pub fn tesselate_ellipsis<Output: VertexBufferBuilder<Vec2>>(
    center: Vec2,
    radius: Vec2,
    num_vertices: u32,
    output: &mut Output
) {
    output.begin_geometry();
    output.push_vertex(center);
    for i in 0..num_vertices {
        let angle = i as f32 * 2.0 * PI / ((num_vertices-1) as f32);
        output.push_vertex(center + vec2(radius.x*angle.cos(), radius.y*angle.sin()));
    }
    for i in 1..((num_vertices) as Index) {
        output.push_indices(0, i, (i-1)%num_vertices as Index+2);
    }
}
