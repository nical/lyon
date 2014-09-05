
use math::vector;
use math::units::world;
use math::units::texels;
use gfx2d::color::Rgba;
use gfx2d::shapes;
use data;
use std::num;

static PI: f32 = 3.1415;

fn abs<T: num::Signed>(a:T) -> T { a.abs() }

pub type TesselationFlags = u32;
pub static VERTEX_ANTIALIASING: TesselationFlags = 1;
pub static CONVEX_SHAPE: TesselationFlags = 2;

pub struct FlattenedPath {
    points: Vec<world::Vec2>,
    is_closed: bool,
}

#[deriving(Show)]
pub struct Pos2DNormal2DColorExtrusion {
    pub pos: world::Vec2,
    pub normal: world::Vec2,
    pub color: Rgba<f32>,
    pub extrusion : f32,
}

static vec2_vec2_vec4_f_f_data_type : &'static[data::Type] = &[
    data::VEC2, data::VEC2, data::VEC4, data::F32, data::F32
];

impl data::StructDataType for Pos2DNormal2DColorExtrusion {
    fn data_type() -> data::StructTypeResult<Pos2DNormal2DColorExtrusion> {
        data::StructTypeResult { data_type: vec2_vec2_vec4_f_f_data_type }
    }
}

fn normal(v: world::Vec2) -> world::Vec2 {
    let l = v.length();
    return world::vec2(-v.y / l, v.x / l);
}

pub enum PointType {
    BorderPoint,
    InteriorPoint,
    AntialiasPoint,
}

fn line_intersection<U>(
    a1: vector::Vector2D<U>,
    a2: vector::Vector2D<U>,
    b1: vector::Vector2D<U>,
    b2: vector::Vector2D<U>
) -> Option<vector::Vector2D<U>> {
    let det = (a1.x - a2.x) * (b1.y - b2.y) - (a1.y - a2.y) * (b1.x - b2.x);
    if det*det < 0.00001 {
        // The lines are very close to parallel
        return None;
    }
    let inv_det = 1.0 / det;
    let a = a1.x * a2.y - a1.y * a2.x;
    let b = b1.x * b2.y - b1.y * b2.x;
    return Some(vector::Vector2D {
        x: (a * (b1.x - b2.x) - b * (a1.x - a2.x)) * inv_det,
        y: (a * (b1.y - b2.y) - b * (a1.y - a2.y)) * inv_det
    });
}

// vertex layout:
//
// |  Pos   |    rgba         |  normal  | aa extrusion |
//  f32 f32   f32 f32 f32 f32   f32  f32       f32
// XXX - use 32bit rgba instead!


pub fn path_to_line_vbo(
    path: &[world::Vec2],
    is_closed: bool,
    flags: TesselationFlags,
    line_width_fn: |uint| -> f32,
    color_fn: |uint, PointType| -> Rgba<f32>,
    transform: world::Mat3,
    vbo: &mut [Pos2DNormal2DColorExtrusion],
) {
    let vertex_antialiasing = (flags & VERTEX_ANTIALIASING) != 0;
    if (path.len() < 2) {
        fail!("invalid path");
    }

    let stride = if vertex_antialiasing { 4 } else { 2 };

    // P1------>PX-------->P2
    let mut p1 = if is_closed { path[path.len() - 1] }
                  else { path[0] + path[0] - path[1] };
    let mut px = path[0];
    let mut n1 = normal(px - p1);
    // With the line equation y = a * x + b

    for i in range(0, path.len()) {
        let mut pos = transform.transform_2d(&path[i]);

        let color = color_fn(i, BorderPoint);
        let color_aa = color_fn(i, AntialiasPoint);

        // Compute the normal at the intersection point px
        let mut p2 = if i < path.len() - 1 { path[i + 1] }
                      else if is_closed { path[0] }
                      else { path[i] + path[i] - path[i - 1] };
        let mut n2 = normal(p2 - px);
        // Segment P1-->PX
        let pn1  = p1 + n1; // p1 extruded along the normal n1
        let pn1x = px + n1; // px extruded along the normal n1
        // Segment PX-->P2
        let pn2  = p2 + n2;
        let pn2x = px + n2;

        let inter = match line_intersection(pn1, pn1x, pn2x, pn2) {
            Some(v) => { v }
            None => {
                if (n1 - n2).square_length() < 0.00001 {
                    px + n1
                } else {
                    // TODO: the angle is very narrow, use rounded corner instead
                    // Arbitrarily, just take a normal that is almost zero but not quite
                    // to avoid running into issues if we divide by its length.
                    // This is wrong but it will do until rounded corners are implemented.
                    //world::vec2(0.0, 0.1);
                    fail!("Not implemented yet");
                }
            }
        };
        let normal = transform.transform_2d(&(inter - px));

        // Shift towards the next point; some values don't need to be recomputed
        // since the segment 1 is the segment 2 of the previous iteration.
        // TODO: more stuff could be cached in line_intersection.
        p1 = px;
        px = p2;
        n1 = n2;

        let line_width = line_width_fn(i);
        let aa_width = 1.0;
        let extrusion_ws = normal.times(line_width);

        vbo[i * stride].pos = pos + extrusion_ws;
        vbo[i * stride].normal = normal;
        vbo[i * stride].color = color;
        vbo[i * stride].extrusion = -aa_width;

        vbo[i * stride + 1].pos = pos - extrusion_ws;
        vbo[i * stride + 1].normal = normal;
        vbo[i * stride + 1].color = color;
        vbo[i * stride + 1].extrusion = aa_width;

        if (vertex_antialiasing) {
            vbo[i * stride + 2].pos = pos + extrusion_ws;
            vbo[i * stride + 2].normal = normal;
            vbo[i * stride + 2].color = color_aa;
            vbo[i * stride + 2].extrusion = aa_width;

            vbo[i * stride + 3].pos = pos - extrusion_ws;
            vbo[i * stride + 3].normal = normal;
            vbo[i * stride + 3].color = color_aa;
            vbo[i * stride + 3].extrusion = -aa_width;
        }
    }
}

pub fn path_to_line_ibo(
    num_points: u32,
    is_closed: bool,
    flags: TesselationFlags,
    base_vertex: u16,
    ibo: &mut [u16],
) {
    // 6--4----5--7
    // | /|  / | /|
    // |/ | /  |/ |
    // 2--0----1--3
    //
    // 0 1 5, 0 5 4, 2 0 4, 2 4 6, 1 3 7, 1 7 5 

    let vertex_antialiasing = (flags & VERTEX_ANTIALIASING) != 0;
    let vertex_stride : u16 = if vertex_antialiasing { 4 } else { 2 };
    let index_stride = 6 * (vertex_stride as uint - 1);
    for i in range(0, num_points as uint - 1) {
        let idx = i as u16;
        ibo[i * index_stride    ] = base_vertex + idx * vertex_stride;
        ibo[i * index_stride + 1] = base_vertex + idx * vertex_stride + 1;
        ibo[i * index_stride + 2] = base_vertex + (idx + 1) * vertex_stride + 1;

        ibo[i * index_stride + 3] = base_vertex + idx * vertex_stride;
        ibo[i * index_stride + 4] = base_vertex + (idx + 1) * vertex_stride + 1;
        ibo[i * index_stride + 5] = base_vertex + (idx + 1) * vertex_stride;

        if (vertex_antialiasing) {
            ibo[i * index_stride + 6] = base_vertex + idx * vertex_stride + 2;
            ibo[i * index_stride + 7] = base_vertex + idx * vertex_stride + 0;
            ibo[i * index_stride + 8] = base_vertex + (idx + 1) * vertex_stride;

            ibo[i * index_stride + 9 ] = base_vertex + idx * vertex_stride + 2;
            ibo[i * index_stride + 10] = base_vertex + (idx + 1) * vertex_stride;
            ibo[i * index_stride + 11] = base_vertex + (idx + 1) * vertex_stride + 2;

            ibo[i * index_stride + 12] = base_vertex + idx * vertex_stride + 1;
            ibo[i * index_stride + 13] = base_vertex + idx * vertex_stride + 3;
            ibo[i * index_stride + 14] = base_vertex + (idx + 1) * vertex_stride + 3;

            ibo[i * index_stride + 15] = base_vertex + idx * vertex_stride + 1;
            ibo[i * index_stride + 16] = base_vertex + (idx + 1) * vertex_stride + 3;
            ibo[i * index_stride + 17] = base_vertex + (idx + 1) * vertex_stride + 1;
        }
    }
    if is_closed {
        let i = num_points as uint - 1;
        let idx = i as u16;
        ibo[i * index_stride    ] = base_vertex + idx * vertex_stride + 0;
        ibo[i * index_stride + 1] = base_vertex + idx * vertex_stride + 1;
        ibo[i * index_stride + 2] = base_vertex + 1;

        ibo[i * index_stride + 3] = base_vertex + idx * vertex_stride + 0;
        ibo[i * index_stride + 4] = base_vertex + 1;
        ibo[i * index_stride + 5] = base_vertex + 0;

        if (vertex_antialiasing) {
            ibo[i * index_stride + 6] = base_vertex + idx * vertex_stride + 2;
            ibo[i * index_stride + 7] = base_vertex + idx * vertex_stride + 0;
            ibo[i * index_stride + 8] = base_vertex + 0;

            ibo[i * index_stride + 9 ] = base_vertex + idx * vertex_stride + 2;
            ibo[i * index_stride + 10] = base_vertex + 0;
            ibo[i * index_stride + 11] = base_vertex + 2;

            ibo[i * index_stride + 12] = base_vertex + idx * vertex_stride + 1;
            ibo[i * index_stride + 13] = base_vertex + idx * vertex_stride + 3;
            ibo[i * index_stride + 14] = base_vertex + 3;

            ibo[i * index_stride + 15] = base_vertex + idx * vertex_stride + 1;
            ibo[i * index_stride + 16] = base_vertex + 3;
            ibo[i * index_stride + 17] = base_vertex + 1;
        }
    }
}

pub struct VertexStream<'l, T: 'l> {
    pub vertices: &'l mut[T],
    pub indices: &'l mut[u16],
    pub vertex_cursor: uint,
    pub index_cursor: uint,
    pub base_vertex: u16,
}

pub struct Range {
    first_vertex: u16,
    vertex_count: u16,
    first_index: u16,
    index_count: u16,
}

impl<'l, T: Copy> VertexStream<'l, T> {
    pub fn push_vertex(&mut self, vertex: &T) {
        self.vertices[self.vertex_cursor] = *vertex;
        self.vertex_cursor += 1;
    }

    pub fn push_index(&mut self, idx: u16) {
        self.indices[self.index_cursor] = idx + self.base_vertex;
        self.index_cursor += 1;
    }

    pub fn push_quad(&mut self, a: &T, b: &T, c: &T, d: &T) {
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

    pub fn push_triangle(&mut self, a: &T, b: &T, c: &T) {
        let cursor = self.vertex_cursor as u16;
        self.push_vertex(a);
        self.push_vertex(b);
        self.push_vertex(c);
        self.push_index(cursor);
        self.push_index(cursor + 1);
        self.push_index(cursor + 2);
    }
}

pub fn fill_rectangle<'l, T: VertexType2D>(
    stream: &mut VertexStream<'l, T>,
    rectangle: &world::Rectangle,
    transform: &world::Mat3,
    fill: FillStyle<'l>,
) -> Range {
    let first_vertex = stream.vertex_cursor as u16;
    let first_index = stream.vertex_cursor as u16;
    let uv_rect = texels::rect(0.0, 0.0, 1.0, 1.0);

    let mut a: T = VertexType2D::from_pos(&transform.transform_2d(&rectangle.top_left()));
    let mut b: T = VertexType2D::from_pos(&transform.transform_2d(&rectangle.top_right()));
    let mut c: T = VertexType2D::from_pos(&transform.transform_2d(&rectangle.bottom_right()));
    let mut d: T = VertexType2D::from_pos(&transform.transform_2d(&rectangle.bottom_left()));

    match fill {
        NoFill => {}
        FillColor(color) => {
            a.set_color(color);
            b.set_color(color);
            c.set_color(color);
            d.set_color(color);
        }
        FillTexture(uv_transform) => {
            a.set_uv(&uv_transform.transform_2d(&uv_rect.top_left()));
            b.set_uv(&uv_transform.transform_2d(&uv_rect.top_right()));
            c.set_uv(&uv_transform.transform_2d(&uv_rect.bottom_right()));
            d.set_uv(&uv_transform.transform_2d(&uv_rect.bottom_left()));
        }
    }
    stream.push_quad(&a, &b, &c, &d);
    return Range {
        first_vertex: first_vertex,
        vertex_count: stream.vertex_cursor as u16 - first_vertex,
        first_index: first_index,
        index_count: stream.index_cursor as u16 - first_index,
    };
}

pub fn fill_circle<'l, T: VertexType2D>(
    stream: &mut VertexStream<'l, T>,
    circle: &shapes::Circle,
    num_points: u32,
    transform: &world::Mat3,
    fill: FillStyle<'l>,
) -> Range {
    let first_vertex = stream.vertex_cursor as u16;
    let first_index = stream.index_cursor as u16;
    let pos = transform.transform_2d(&world::vec2(
        circle.center.x,
        circle.center.y
    ));

    stream.push_vertex(
        &match fill {
            NoFill => VertexType2D::from_pos(&pos),
            FillColor(color) => VertexType2D::from_pos_color(&pos, color),
            FillTexture(uv_transform) => {
                VertexType2D::from_pos_uv(&pos,
                    &uv_transform.transform_2d(&texels::vec2(0.5, 0.5))
                )
            }
        }
    );

    for i in range(0, num_points+1) {
        let dx = (i as f32 / num_points as f32 * 2.0 * PI).cos();
        let dy = (i as f32 / num_points as f32 * 2.0 * PI).sin();

        let pos = transform.transform_2d(&world::vec2(
            circle.center.x + circle.radius * dx,
            circle.center.y + circle.radius * dy
        ));

        stream.push_vertex(
            &match fill {
                NoFill => VertexType2D::from_pos(&pos),
                FillColor(color) => VertexType2D::from_pos_color(&pos, color),
                FillTexture(uv_transform) => {
                    VertexType2D::from_pos_uv(&pos,
                        &uv_transform.transform_2d(&texels::vec2(
                            0.5 + dx * 0.5,
                            0.5 + dy * 0.5
                        ))
                    )
                }
            }
        );

        stream.push_index(first_vertex);
        stream.push_index(first_vertex + i as u16);
        stream.push_index(first_vertex + i as u16 + 1);
    }
    return Range {
        first_vertex: first_vertex,
        vertex_count: stream.vertex_cursor as u16 - first_vertex,
        first_index: first_index,
        index_count: stream.index_cursor as u16 - first_index,
    };
}

pub fn fill_grid<'l, T: VertexType2D>(
    stream: &mut VertexStream<'l, T>,
    columns: &[f32],
    lines: &[f32],
    transform: &world::Mat3,
    fill: FillStyle<'l>,
    uv_grid: Option<(&'l[f32], &'l[f32])>
) -> Range {
    assert!(columns.len() >= 2)
    assert!(lines.len() >= 2)

    let first_vertex = stream.vertex_cursor as u16;
    let first_index = stream.index_cursor as u16;

    for j in range(0, lines.len()) {
        for i in range(0, columns.len()) {
            let pos = transform.transform_2d(&world::vec2(columns[i],lines[j]));
            stream.push_vertex(&match fill {
                NoFill => VertexType2D::from_pos(&pos),
                FillColor(color) => VertexType2D::from_pos_color(&pos, color),
                FillTexture(uv_transform) => {
                    let uv = match uv_grid {
                        Some((uv_lines, uv_columns)) => {
                            texels::vec2(uv_columns[i], uv_lines[j])
                        }
                        None => {
                            texels::vec2(
                                (columns[i] - columns[0]) / (columns[columns.len()-1] - columns[0]),
                                (lines[i] - lines[0]) / (lines[lines.len()-1] - lines[0])
                            )
                        }
                    };
                    VertexType2D::from_pos_uv(&pos, &uv_transform.transform_2d(&uv))
                }
            });
        }
    }

    let stride = lines.len() as u16;
    for j in range(0, lines.len() as u16 - 1) {
        for i in range(0, columns.len() as u16 - 1) {
            stream.push_index(first_index + j * stride + i);
            stream.push_index(first_index + j * stride + (i+1));
            stream.push_index(first_index + (j+1) * stride + (i+1));

            stream.push_index(first_index + j * stride + i);
            stream.push_index(first_index + (j+1) * stride + (i+1));
            stream.push_index(first_index + (j+1) * stride + i);
        }
    }


    return Range {
        first_vertex: first_vertex,
        vertex_count: stream.vertex_cursor as u16 - first_vertex,
        first_index: first_index,
        index_count: stream.index_cursor as u16 - first_index,
    };
}

pub trait VertexType2D: Copy {
    fn from_pos(pos: &world::Vec2) -> Self;
    fn from_pos_uv(pos: &world::Vec2, uv: &texels::Vec2) -> Self;
    fn from_pos_color(pos: &world::Vec2, uv: &Rgba<u8>) -> Self;
    fn new() -> Self;
    fn set_pos(&mut self, &world::Vec2);
    fn set_uv(&mut self, &texels::Vec2);
    fn set_color(&mut self, &Rgba<u8>);
}

pub enum FillStyle<'l> {
    FillTexture(&'l texels::Mat3),
    FillColor(&'l Rgba<u8>),
    NoFill,
}

enum StrokeStyle<'l> {
    StrokeTexture(f32, &'l texels::Mat3),
    StrokeColor(f32, &'l Rgba<u8>),
    NoStroke,
}
