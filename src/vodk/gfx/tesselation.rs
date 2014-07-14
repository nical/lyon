
use math::vector;
use math::units::world;
use math::units::texels;
use gfx::locations::*;
use gfx::renderer;
use gfx::color::Rgba;
use data;

// in:
//  * Iterator<(Pos: Lerp, Attrib: Lerp)>
// out:
//  * vbo: |Pos, Attrib|
//  * ibo: |u16|

// vertex layout:
//
// |  Pos   |   rgba         | extrude_vec | extrude_ws | extrude_ss
//  f32 f32   f32 f32 f32 f32    f32  f32       f32          f32
//
// 2-0---1-3  0 1 5, 0 5 4, 2 0 4, 2 4 6, 1 3 7, 1 7 5
// | |   | |
// | |   | |
// | |   | |
// | |   | |
// 6-4---5-7 

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
    pub extrude_ws : f32,
    pub extrude_ss : f32,
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

    let mut n1 = if is_closed { normal(path[0] - path[path.len() - 1]) }
                 else { normal(path[1] - path[0]) };

    let stride = if vertex_antialiasing { 4 } else { 2 };

    for i in range(0, path.len()) {
        let mut pos = transform.transform_2d(&path[i]);

        let color = color_fn(i, BorderPoint);
        let color_aa = color_fn(i, AntialiasPoint);

        let n2 = if i < path.len() - 1 {
            normal(path[i + 1] - path[i])
        } else {
            if is_closed { normal(path[0] - path[i]) }
            else { normal(path[i] - path[i - 1]) }
        };
        let mut normal = n1 + n2;
        normal.x *= 0.5;
        normal.y *= 0.5;
        println!(" ---- n1: {} n2: {}", n1, n2);
        normal = transform.transform_2d(&normal);
        n1 = n2;
        let line_width = line_width_fn(i);

        vbo[i * stride].pos = pos;
        vbo[i * stride].normal = normal;
        vbo[i * stride].color = color;
        vbo[i * stride].extrude_ws = line_width;
        vbo[i * stride].extrude_ss = -0.5;

        vbo[i * stride + 1].pos = pos;
        vbo[i * stride + 1].normal = normal;
        vbo[i * stride + 1].color = color;
        vbo[i * stride + 1].extrude_ws = -line_width;
        vbo[i * stride + 1].extrude_ss = 0.5;

        if (vertex_antialiasing) {
            vbo[i * stride + 2].pos = pos;
            vbo[i * stride + 2].normal = normal;
            vbo[i * stride + 2].color = color_aa;
            vbo[i * stride + 2].extrude_ws = line_width;
            vbo[i * stride + 2].extrude_ss = 0.5;

            vbo[i * stride + 3].pos = pos;
            vbo[i * stride + 3].normal = normal;
            vbo[i * stride + 3].color = color_aa;
            vbo[i * stride + 3].extrude_ws = -line_width;
            vbo[i * stride + 3].extrude_ss = -0.5;
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

pub fn test_path() {
    let path : Vec<world::Vec2> = vec!(
            world::vec2(0.0, 0.0),
            world::vec2(1.0, 0.0),
            world::vec2(1.0, 1.0),
            world::vec2(0.0, 1.0)
    );
    let is_closed = true;

    let mut vbo : Vec<Pos2DNormal2DColorExtrusion> = Vec::from_fn(path.len()*4, |_|{
        Pos2DNormal2DColorExtrusion {
            pos: world::vec2(0.0, 0.0),
            normal: world::vec2(0.0, 0.0),
            color: Rgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
            extrude_ss: 0.0,
            extrude_ws: 0.0,
        }
    });

    path_to_line_vbo(
        path.as_slice(),
        is_closed,
        VERTEX_ANTIALIASING|CONVEX_SHAPE,
        |_| { 10.0 },
        |_, ptype| { match ptype {
            AntialiasPoint => Rgba { r: 0.5, g: 0.5, b: 0.5, a: 0.0 },
            _ => Rgba { r: 0.5, g: 0.5, b: 0.5, a: 1.0 },
        }},
        world::Mat3::identity(),
        vbo.as_mut_slice()
    );

    for l in vbo.iter() {
        println!(" {}", l.normal);
    }
}