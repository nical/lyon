#![allow(dead_code)]

use tesselation::vertex_builder::VertexBufferBuilder;
use tesselation::{ Index };

use vodk_math::{ Vector2D, Rectangle };
use vodk_math::units::{ Texels };

use std::f32::consts::PI;

pub struct RoundedRectangle<U> {
    rect: Rectangle<U>,
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

pub fn tesselate_rectangle<U, Output: VertexBufferBuilder<Vector2D<U>>>(
    rect: &Rectangle<U>,
    output: &mut Output,
) {
    tesselate_quad(
        rect.top_left(),
        rect.top_right(),
        rect.bottom_right(),
        rect.bottom_left(),
        output
    );
}

pub trait Vertex2dUv {
    fn new(Vector2D) -> Self;
}

pub fn tesselate_rectangle_with_uv<A, Output: VertexBufferBuilder<(Vector2D<A>, Vector2D<Texels>)>>(
    rect: &Rectangle<A>,
    uv: &Rectangle<Texels>,
    output: &mut Output,
) {
    tesselate_quad(
        (rect.top_left(), uv.top_left()),
        (rect.top_right(), uv.top_right()),
        (rect.bottom_right(), uv.bottom_right()),
        (rect.bottom_left(), uv.bottom_left()),
        output
    );
}

pub fn tesselate_rounded_rectangle<U, Output: VertexBufferBuilder<Vector2D<U>>>(
    _rect: &RoundedRectangle<U>,
    output: &mut Output
) {
    output.begin_geometry();
    panic!("TODO!");
}

pub fn tesselate_ellipsis<U, Output: VertexBufferBuilder<Vector2D<U>>>(
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

/*
tesselate_rect_with_uv(text[i].rect(), cache.uv_for(text[i].key), output);

let glyph_rect = text[i].rect.top_left();
let uv_rect = cache.uv_for(text[i].key);
tesselate_quad(
    Vertex::new(transform * glyph_rect.top_left(), uv_rect.top_left()),
    Vertex::new(transform * glyph_rect.top_right(), uv_rect.top_right()),
    Vertex::new(transform * glyph_rect.bottom_right(), uv_rect.bottom_right()),
    Vertex::new(transform * glyph_rect.bottom_left(), uv_rect.bottom_left()),
    output
);

*/