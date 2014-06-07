
use gfx::renderer;
use gfx::shaders;
use gfx::text;
use math::vector::Vec2;
use math::vector;
use math::units::pixels;
use math::units::texels;
use gfx::locations::*;

static PI: f32 = 3.1415;

pub struct WidgetID {
    handle: u32,
}

pub enum WidgetType {
    ContainerWidget,
    SplitContainerWidget,
    GridContainerWidget,
    SliderWidget,
    TextWidget,
    ButtonWidget,
    ToggleWidget,
}

pub struct Widget {
    pub rect: pixels::Rect,
    pub first_child: WidgetID,
    pub next: WidgetID,
    pub widget_type: WidgetType,
}

pub struct WidgetGfxComponent {
    ranges: Vec<(renderer::IndexRange, renderer::VertexRange)>,
}

pub struct WidgetTree {
    widgets: Vec<Widget>,
    gfx_components: Vec<WidgetGfxComponent>,
}

pub struct IndexedBatch<'l> {
    pub vertices: &'l mut[f32],
    pub indices: &'l mut[u16],
    pub vertex_cursor: uint, // number of vertices added (!= num of floats)
    pub index_cursor: uint,
    pub vertex_stride: uint, // num
    pub attributes: &'l [renderer::VertexAttribute],
    pub base_vertex: u16,
}

impl<'l> IndexedBatch<'l> {
    pub fn new(
        vertices: &'l mut [f32], indices: &'l mut[u16],
        base_vertex: u16, vertex_stride: uint,
        attrib: &'l [renderer::VertexAttribute]
    ) -> IndexedBatch<'l> {
        IndexedBatch {
            vertices: vertices,
            indices: indices,
            vertex_cursor: 0,
            index_cursor: 0,
            base_vertex: 0,
            vertex_stride: vertex_stride,
            attributes: attrib,
        }
    }

    pub fn push_vertex(&mut self, vertex: &[f32]) {
        assert!(vertex.len() == self.vertex_stride);
        let mut i = 0;
        for val in vertex.iter() {
            self.vertices[(self.vertex_cursor) * self.vertex_stride + i] = *val;
            i += 1;
        }
        self.vertex_cursor += 1;
    }

    pub fn push_index(&mut self, idx: u16) {
        self.indices[self.index_cursor] = idx + self.base_vertex;
        self.index_cursor += 1;
    }

    pub fn push_triangle(&mut self, a: &[f32], b: &[f32], c: &[f32]) {
        let cursor = self.vertex_cursor as u16;
        self.push_vertex(a);
        self.push_vertex(b);
        self.push_vertex(c);
        self.push_index(cursor);
        self.push_index(cursor + 1);
        self.push_index(cursor + 2);
    }

    pub fn push_quad(&mut self, a: &[f32], b: &[f32], c: &[f32], d: &[f32]) {
        let cursor = self.vertex_cursor as u16;
        self.push_vertex(a);
        self.push_vertex(b);
        self.push_vertex(c);
        self.push_vertex(d);
        self.push_index(cursor);
        self.push_index(cursor + 1);
        self.push_index(cursor + 2);
        self.push_index(cursor);
        self.push_index(cursor + 2);
        self.push_index(cursor + 3);
    }
}

trait Shape {
    fn num_indices(&self) -> u32;
    fn num_vertices(&self) -> u32;
    fn aabb(&self) -> (f32, f32, f32, f32);
}

struct Circle {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub n_points: u32,
}

pub struct BezierSegment {
    pub p0: Vec2,
    pub p1: Vec2,
    pub p2: Vec2,
    pub p3: Vec2,
    pub line_width: f32,
}

pub struct Line {
    pub p0: Vec2,
    pub p1: Vec2,
    pub line_width: f32,
}

pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32
}

impl Shape for Circle {
    fn num_indices(&self) -> u32 { self.n_points * 3 }
    fn num_vertices(&self) -> u32 { self.n_points + 1 }
    fn aabb(&self) -> (f32, f32, f32, f32) {(
        self.x - self.r * 0.5,
        self.y - self.r * 0.5,
        self.r * 2.0, self.r * 2.0
    )}
}

impl Shape for pixels::Rect {
    fn num_indices(&self) -> u32 { 6 }
    fn num_vertices(&self) -> u32 { 4 }
    fn aabb(&self) -> (f32, f32, f32, f32) { (self.x, self.y, self.w, self.h) }
}

pub enum ItemShape {
    CircleShape(Circle),
    RectShape(pixels::Rect),
}

pub struct Item {
    pub shape: ItemShape,
    pub tex: Option<texels::Rect>,
    pub color: Option<Color>,
    pub first_index: uint,
    pub first_vertex: uint,
}

pub fn push_rect(
    batch: &mut IndexedBatch,
    rect: pixels::Rect,
    tex: Option<texels::Rect>,
    color: Option<Color>
) -> Item {
    let mut v0: Vec<f32> = Vec::from_fn(batch.vertex_stride, |_|{0.0 as f32});
    let mut v1: Vec<f32> = Vec::from_fn(batch.vertex_stride, |_|{0.0 as f32});
    let mut v2: Vec<f32> = Vec::from_fn(batch.vertex_stride, |_|{0.0 as f32});
    let mut v3: Vec<f32> = Vec::from_fn(batch.vertex_stride, |_|{0.0 as f32});

    let first_index = batch.index_cursor;
    let first_vertex = batch.vertex_cursor;

    for attrib in batch.attributes.iter() {
        let i = attrib.offset as uint / 4;
        match attrib.location {
            a_position => {
                *v0.get_mut(i  ) = rect.x;
                *v0.get_mut(i+1) = rect.y;
                *v1.get_mut(i  ) = rect.x;
                *v1.get_mut(i+1) = rect.y + rect.h;
                *v2.get_mut(i  ) = rect.x + rect.w;
                *v2.get_mut(i+1) = rect.y + rect.h;
                *v3.get_mut(i  ) = rect.x + rect.w;
                *v3.get_mut(i+1) = rect.y;
            }
            a_tex_coords => {
                match tex {
                    Some(tc) => {
                        *v0.get_mut(i)   = tc.x;
                        *v0.get_mut(i+1) = tc.y;
                        *v1.get_mut(i)   = tc.x;
                        *v1.get_mut(i+1) = tc.y + tc.h;
                        *v2.get_mut(i)   = tc.x + tc.w;
                        *v2.get_mut(i+1) = tc.y + tc.h;
                        *v3.get_mut(i)   = tc.x + tc.w;
                        *v3.get_mut(i+1) = tc.y;
                    }
                    _ => {}
                }
            }
            a_color => {
                match color {
                    Some(c) => {
                        *v0.get_mut(i  ) = c.r;
                        *v0.get_mut(i+1) = c.g;
                        *v0.get_mut(i+2) = c.b;
                        *v0.get_mut(i+3) = c.a;
                        *v1.get_mut(i  ) = c.r;
                        *v1.get_mut(i+1) = c.g;
                        *v1.get_mut(i+2) = c.b;
                        *v1.get_mut(i+3) = c.a;
                        *v2.get_mut(i  ) = c.r;
                        *v2.get_mut(i+1) = c.g;
                        *v2.get_mut(i+2) = c.b;
                        *v2.get_mut(i+3) = c.a;
                        *v3.get_mut(i  ) = c.r;
                        *v3.get_mut(i+1) = c.g;
                        *v3.get_mut(i+2) = c.b;
                        *v3.get_mut(i+3) = c.a;
                    }
                    _ => {
                        fail!();
                    }
                }
            }
            _ => {}
        }
    }

    batch.push_quad(
        v0.as_slice(),
        v1.as_slice(),
        v2.as_slice(),
        v3.as_slice()
    );

    return Item {
        shape: RectShape(rect),
        tex: tex,
        color: color,
        first_index: first_index,
        first_vertex: first_vertex,
    };
}

fn set_circle_coordinates(
    batch: &mut IndexedBatch,
    x: f32, y: f32, r: f32, n_points: u32,
    stride: uint, offset: uint
) {
    let step = 2.0 * PI / (n_points-2) as f32;
    batch.vertices[offset] = x;
    batch.vertices[offset + 1] = y;
    for i in range(1, n_points as uint) {
        batch.vertices[offset + i * stride] = x + (step * i as f32).cos() * r;
        batch.vertices[offset + i * stride + 1] = y + (step * i as f32).sin() * r;
    }
}

pub fn push_circle(
    batch: &mut IndexedBatch,
    x: f32, y: f32, r: f32, n_points: u32,
    tex: Option<texels::Rect>,
    color: Option<Color>
) {
    let first_index = batch.index_cursor;
    let stride = batch.vertex_stride;
    let first_vertex = batch.vertex_cursor * stride;
    println!("first index: {}, first vertex: {}, stride: {}", first_index, first_vertex, stride);

    for attrib in batch.attributes.iter() {
        let offset = first_vertex + attrib.offset as uint / 4;
        match attrib.location {
            a_position => {
                set_circle_coordinates(batch, x, y, r, n_points, stride, offset);
            }
            a_tex_coords => {
                match tex {
                    Some(tex_rect) => {
                        set_circle_coordinates(batch,
                            tex_rect.x + tex_rect.w / 2.0,
                            tex_rect.y + tex_rect.h / 2.0,
                            tex_rect.w / 2.0,
                            n_points, stride, offset
                        );
                    }
                    _ => { fail!(); }
                }
            }
            a_color => {
                match color {
                    Some(c) => {
                        for i in range(0, (n_points+1) as uint) {
                            batch.vertices[offset + i * stride] = c.r;
                            batch.vertices[offset + i * stride + 1] = c.g;
                            batch.vertices[offset + i * stride + 2] = c.b;
                            batch.vertices[offset + i * stride + 3] = c.a;
                        }
                    }
                    _ => { fail!(); }
                }
            }
            _ => {}
        }
    }
    for i in range(1, (n_points-1) as u16) {
        batch.push_index(batch.vertex_cursor as u16);
        batch.push_index(batch.vertex_cursor as u16 + i);
        batch.push_index(batch.vertex_cursor as u16 + i + 1);
    }
    batch.vertex_cursor += n_points as uint;
}
