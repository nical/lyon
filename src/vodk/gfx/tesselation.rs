
use math::vector;
use math::units::world;
use math::units::texels;
use gfx::locations::*;
use gfx::renderer;
use gfx::color::Rgba;
use data;
use std::num;

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
    a1: vector::Vector2D<f32, U>,
    a2: vector::Vector2D<f32, U>,
    b1: vector::Vector2D<f32, U>,
    b2: vector::Vector2D<f32, U>
) -> Option<vector::Vector2D<f32 ,U>> {
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
