
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


pub struct FlattenedPath {
    points: Vec<world::Vec2>,
    is_closed: bool,
}

pub struct VertexDescriptor {
    pub attrib_type: data::Type,
    pub location: renderer::VertexAttributeLocation,
    pub offset: u16,
}

pub enum PointType {
    InteriorPoint,
    ExteriorPoint,
    AntialiasPoint,
}


enum TexCoordsDescriptor<'l> {
    PointTexCoords(&'l[texels::Vec2]),
    RectTexCoords(texels::Rect),
    NoTexCoords,
}

enum AntialiasingDescriptor {
    VertexAntiAliasing,
    NoAntialiasing,
}

enum ColorDescriptor<'l> {
    PointColor(&'l[Rgba<f32>]),
    ConstColor(Rgba<f32>),
    NoColor,
}

enum LineWidthDescriptor<'l> {
    PointWidth(&'l[f32]),
    ConstWidth(f32),
}

/*
build_stroke_vbo(
    &path,
    &mut vbo,
    ConstWidth(5.0),
    ConstColor(Rgba {r: 0.0, g: 0.0, b: 0.0, 1.0}),
    VertexAntiAliasing,
    NoTexCoords,
    0,
)
*/


fn path_to_vbo(
    path: &FlattenedPath,
    vbo: &mut [f32],
    //ibo: &mut [u16],
    attributes: &[VertexDescriptor],
    vertex_stride: uint,
) {
    if (path.points.len() < 2) {
        return;
    }
    let mut v0 = if path.is_closed {
        *path.points.get(path.points.len() - 1) - *path.points.get(0)
    } else {
        *path.points.get(1) - *path.points.get(0)
    };
    let mut vx = if path.is_closed {
        *path.points.get(path.points.len() - 1) - *path.points.get(0)
    } else {
        *path.points.get(path.points.len() - 1) - *path.points.get(path.points.len() - 2)
    };

    for i in range(0, path.points.len()) {
        let point_offset: uint = (4 / 4 * i * vertex_stride) as uint;
        for attrib in attributes.iter() {
            let attrib_offset : uint = attrib.offset as uint / 4;
            match attrib.location {
                a_position => {
                    let (x, y) = (path.points.get(i).x, path.points.get(i).y);
                    println!(" ---- pos: ({}, {})\n", x, y);
                    vbo[point_offset + attrib_offset]     = 1337.0;
                    vbo[point_offset + attrib_offset + 1] = 42.0;
                    vbo[point_offset + attrib_offset + vertex_stride/4]     = x;
                    vbo[point_offset + attrib_offset + vertex_stride/4 + 1] = y;
                    vbo[point_offset + attrib_offset + 2*vertex_stride/4]     = x;
                    vbo[point_offset + attrib_offset + 2*vertex_stride/4 + 1] = y;
                    vbo[point_offset + attrib_offset + 3*vertex_stride/4]     = x;
                    vbo[point_offset + attrib_offset + 3*vertex_stride/4 + 1] = y;
                }
                a_normals => {
                    let v1 = if i == path.points.len() -1 {
                        vx
                    } else {
                        *path.points.get(i) - *path.points.get(i+1)
                    };
                    let mut normal = v1 - v0;
                    println!(" ---- v0: {} v1: {}", v0, v1);
                    let len = normal.length();
                    normal.x /= len;
                    normal.y /= len;
                    println!(" ---- normal: ({}, {})\n", normal.x, normal.y);
                    vbo[point_offset + attrib_offset]     = normal.x;
                    vbo[point_offset + attrib_offset + 1] = normal.y;
                    vbo[point_offset + attrib_offset + vertex_stride/4]       = normal.x;
                    vbo[point_offset + attrib_offset + vertex_stride/4 + 1]   = normal.y;
                    vbo[point_offset + attrib_offset + 2*vertex_stride/4]     = normal.x;
                    vbo[point_offset + attrib_offset + 2*vertex_stride/4 + 1] = normal.y;
                    vbo[point_offset + attrib_offset + 3*vertex_stride/4]     = normal.x;
                    vbo[point_offset + attrib_offset + 3*vertex_stride/4 + 1] = normal.y;
                    v0 = v1;
                }
                _ => {}
            }
        }

    }
}

pub fn test_path() {
    let path = FlattenedPath {
        points: vec!(
            world::vec2(0.0, 0.0),
            world::vec2(1.0, 0.0),
            world::vec2(1.0, 1.0),
            world::vec2(0.0, 1.0)
        ),
        is_closed: false,
    };

    let mut vbo : Vec<f32> = Vec::from_fn(path.points.len()*4*10, |_|{ 0.0 });

    let attributes = &[
        VertexDescriptor{
            location: a_position,
            attrib_type: data::VEC2,
            offset: 0,
        },
        VertexDescriptor{
            location: a_normals,
            attrib_type: data::VEC2,
            offset: 8,
        },
        VertexDescriptor{
            location: a_color,
            attrib_type: data::VEC4,
            offset: 16,
        },
        VertexDescriptor{
            location: a_extrude_world_space,
            attrib_type: data::F32,
            offset: 32,
        },
        VertexDescriptor{
            location: a_extrude_screen_space,
            attrib_type: data::F32,
            offset: 36,
        },
    ];
    path_to_vbo(&path, vbo.as_mut_slice(), attributes, 40);

    println!(" -- FlattenedPath vbo: {}", vbo.as_slice());
}