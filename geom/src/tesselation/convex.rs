
use std::f32::consts::PI;

use tesselation::vertex_builder::VertexBufferBuilder;
use tesselation::vectors::{ vec2_add, vec2_mul, Vec2};

pub type Index = u16;

pub struct Rectangle {
    pos: [f32; 2],
    size: [f32; 2],
}

pub struct RoundedRectangle {
    rect: Rectangle,
    top_left_radius: f32,
    top_right_radius: f32,
    bottom_left_radius: f32,
    bottom_right_radius: f32,
}

pub fn emit_rectangle<Output: VertexBufferBuilder<[f32; 2]>>(
    rect: &Rectangle,
    output: &mut Output,
) {
    output.begin_geometry();
    let a = output.push_vertex(rect.pos);
    let b = output.push_vertex(vec2_add(rect.pos, [rect.size[0], 0.0]));
    let c = output.push_vertex(vec2_add(rect.pos, rect.size));
    let d = output.push_vertex(vec2_add(rect.pos, [0.0, rect.size[0]]));
    output.push_indices(a, b, c);
    output.push_indices(a, c, d);
}

pub fn emit_rounded_rectangle<Output: VertexBufferBuilder<[f32; 2]>>(
    rect: &RoundedRectangle,
    output: &mut Output
) {
    output.begin_geometry();
    panic!("TODO!");
}

pub fn emit_ellipsis<Output: VertexBufferBuilder<[f32; 2]>>(
    center: [f32; 2],
    readius: [f32; 2],
    num_vertices: u32,
    output: &mut Output
) {
    output.begin_geometry();
    output.push_vertex(center);
    for i in 0..num_vertices {
        let angle = i as f32 * 2.0 * PI / ((num_vertices-1) as f32);
        output.push_vertex(vec2_add(center, [readius[0]*angle.cos(), readius[1]*angle.sin()]));
    }
    for i in 1..((num_vertices) as Index) {
        output.push_indices(0, i, (i-1)%num_vertices as Index+2);
    }
}
