use std::cmp::{max, min};
use std::ops::Add;

use euclid;
use image::MutableImageSlice;
use math::*;
use euclid::vec2;

type IntVector = euclid::default::Vector2D<i32>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct IntVec4 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub w: i32,
}

impl Add for IntVec4 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        IntVec4 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
            w: self.w + rhs.w,
        }
    }
}

/// A software triangle rasterizer intended for ref testing and to help debugging
/// the output of the various tessellation routines.
///
/// The triangles are defined by sequences of 3 indices in the input buffers.
/// For example, the first triangle is:
/// { vertices[indices[0]], vertices[indices[1]], vertices[indices[2]] }
/// the second triangle is:
/// { vertices[indices[3]], vertices[indices[4]], vertices[indices[5]] }
/// etc.
///
/// The rasterizer processes pixels by block of 4 and hands blocks containing at
/// least one affected pixel to the ShadingStage.
pub fn rasterize_triangles<Constants, Vertex: VertexData, Target>(
    vertices: &[Vertex],
    indices: &[u16],
    constants: &Constants,
    target: &mut Target,
) where
    Target: ShadingStage<Vertex, Constants>,
{
    // This is a naive implementation of the algorithm described in this blog post:
    // https://fgiesen.wordpress.com/2013/02/08/triangle-rasterization-in-practice/
    let target_size = target.get_size();
    let viewport_min_x = 0;
    let viewport_min_y = 0;
    let viewport_max_x = target_size.0 as i32;
    let viewport_max_y = target_size.1 as i32;

    let mut i = 0;
    while i < indices.len() {
        let v0 = vertices[indices[i] as usize].position().round().to_i32();
        let v1 = vertices[indices[i + 1] as usize]
            .position()
            .round()
            .to_i32();
        let v2 = vertices[indices[i + 2] as usize]
            .position()
            .round()
            .to_i32();

        let min_x = max(viewport_min_x, min(v0.x, min(v1.x, v2.x)));
        let max_x = min(viewport_max_x, max(v0.x, max(v1.x, v2.x)));
        let min_y = max(viewport_min_y, min(v0.y, min(v1.y, v2.y)));
        let max_y = min(viewport_max_y, max(v0.y, max(v1.y, v2.y)));

        let p = int_vector(min_x, min_y);
        let (e12_step_x, e12_step_y, mut w0_row) = init_edge(&v1, &v2, &p);
        let (e20_step_x, e20_step_y, mut w1_row) = init_edge(&v2, &v0, &p);
        let (e01_step_x, e01_step_y, mut w2_row) = init_edge(&v0, &v1, &p);

        for y in min_y..max_y {
            // Barycentric coordinates at the start of each row.
            let mut w0 = w0_row;
            let mut w1 = w1_row;
            let mut w2 = w2_row;

            let mut x = min_x;
            while x < max_x {
                // TODO: should implement | operator for IntVec4.
                let mask = bvec4(
                    (w0.x | w1.x | w2.x) >= 0,
                    (w0.y | w1.y | w2.y) >= 0,
                    (w0.z | w1.z | w2.z) >= 0,
                    (w0.w | w1.w | w2.w) >= 0,
                );

                if mask.any() {
                    target.process_block(
                        x,
                        y,
                        mask,
                        // TODO: interpolate the vertices
                        &vertices[indices[i] as usize],
                        constants,
                    );
                }

                w0 = w0 + e12_step_x;
                w1 = w1 + e20_step_x;
                w2 = w2 + e01_step_x;

                x += 4; // we process pixels by groups of 4.
            }

            w0_row = w0_row + e12_step_y;
            w1_row = w1_row + e20_step_y;
            w2_row = w2_row + e01_step_y;
        }

        i += 3;
    }
}

const PX_GROUP_X: i32 = 4;
const PX_GROUP_Y: i32 = 1;

fn init_edge(v0: &IntVector, v1: &IntVector, origin: &IntVector) -> (IntVec4, IntVec4, IntVec4) {
    let a = v0.y - v1.y;
    let b = v1.x - v0.x;
    let c = v0.x * v1.y - v0.y * v1.x;

    let sx = a * PX_GROUP_X;
    let sy = b * PX_GROUP_Y;
    let step_x = IntVec4 {
        x: sx,
        y: sx,
        z: sx,
        w: sx,
    };
    let step_y = IntVec4 {
        x: sy,
        y: sy,
        z: sy,
        w: sy,
    };

    let swizzling_x = IntVec4 {
        x: 0,
        y: 1,
        z: 2,
        w: 3,
    };
    let swizzling_y = IntVec4 {
        x: 0,
        y: 0,
        z: 0,
        w: 0,
    };
    let dx = IntVec4 {
        x: origin.x,
        y: origin.x,
        z: origin.x,
        w: origin.x,
    } + swizzling_x;
    let dy = IntVec4 {
        x: origin.y,
        y: origin.y,
        z: origin.y,
        w: origin.y,
    } + swizzling_y;
    let row = IntVec4 {
        x: a * dx.x,
        y: a * dx.y,
        z: a * dx.z,
        w: a * dx.w,
    } + IntVec4 {
        x: b * dy.x,
        y: b * dy.y,
        z: b * dy.w,
        w: b * dy.w,
    } + IntVec4 {
        x: c,
        y: c,
        z: c,
        w: c,
    };

    return (step_x, step_y, row);
}

pub trait ShadingStage<Vertex, Constants> {
    fn process_block(
        &mut self,
        x: i32,
        y: i32,
        mask: BoolVec4,
        vertex: &Vertex,
        constants: &Constants,
    );
    fn get_size(&self) -> (usize, usize);
}

/// An operation that is applied to each rasterized pixel
pub trait PixelShader<Pixel, Vertex, Constants> {
    fn shade(pixel: Pixel, vertex_pixels: &Vertex, constants: &Constants) -> Pixel;
}

// Vertices must implement this trait
pub trait VertexData {
    fn interpolate(v1: &Self, v2: &Self, v3: &Self, w1: f32, w2: f32, w3: f32) -> Self;
    fn position(&self) -> Vector;
}

/// A simple shader that returns the interpolated vertex color.
struct FillVertexColor;

/// A simple shader that returns the constant color.
struct FillConstantColor;

/// Implemented vertices and constants that can return a color.
pub trait GetColor<Pixel> {
    fn get_color(&self) -> Pixel;
}

impl<Pixel, Vertex: GetColor<Pixel>, Constants> PixelShader<Pixel, Vertex, Constants>
    for FillVertexColor
{
    fn shade(_: Pixel, vertex_pixels: &Vertex, _: &Constants) -> Pixel {
        vertex_pixels.get_color()
    }
}

impl<Pixel, Vertex, Constants: GetColor<Pixel>> PixelShader<Pixel, Vertex, Constants>
    for FillConstantColor
{
    fn shade(_: Pixel, _: &Vertex, constants: &Constants) -> Pixel {
        constants.get_color()
    }
}

impl VertexData for Vector {
    fn interpolate(a: &Vector, b: &Vector, c: &Vector, wa: f32, wb: f32, wc: f32) -> Vector {
        let inv_w = 1.0 / (wa + wb + wc);
        return (*a * wa + *b * wb + *c * wc) * inv_w;
    }

    fn position(&self) -> Vector {
        *self
    }
}

pub struct ColorTarget<'a, 'b: 'a, Pixel: Copy + 'static, Shader> {
    target: &'a mut MutableImageSlice<'b, Pixel>,
    shader: Shader,
}

impl<'l, 'm, Pixel, Vertex, Constants, Shader> ShadingStage<Vertex, Constants>
    for ColorTarget<'l, 'm, Pixel, Shader>
where
    Pixel: Copy + 'static,
    Shader: PixelShader<Pixel, Vertex, Constants>,
{
    fn process_block(
        &mut self,
        x: i32,
        y: i32,
        mask: BoolVec4,
        vertex: &Vertex,
        constants: &Constants,
    ) {
        // This is pretty slow, the shader should process blocks instead of pixels, etc.
        if mask.x {
            let i0 = self.target.pixel_offset(x as usize, y as usize);
            let p = self.target.pixels[i0];
            self.target.pixels[i0] = Shader::shade(p, vertex, constants);
        }
        if mask.y {
            let i1 = self.target.pixel_offset(x as usize + 1, y as usize);
            let p = self.target.pixels[i1];
            self.target.pixels[i1] = Shader::shade(p, vertex, constants);
        }
        if mask.z {
            let i2 = self.target.pixel_offset(x as usize + 2, y as usize);
            let p = self.target.pixels[i2];
            self.target.pixels[i2] = Shader::shade(p, vertex, constants);
        }
        if mask.w {
            let i3 = self.target.pixel_offset(x as usize + 3, y as usize);
            let p = self.target.pixels[i3];
            self.target.pixels[i3] = Shader::shade(p, vertex, constants);
        }
    }

    fn get_size(&self) -> (usize, usize) {
        (self.target.width, self.target.height)
    }
}

#[test]
fn test_rasterizer_simple() {
    // This test rasterizes two triangles which should produce a square of origin
    // (10, 10) and size (80, 80).

    struct Constants {
        color: u8,
    }
    impl GetColor<u8> for Constants {
        fn get_color(&self) -> u8 {
            self.color
        }
    }

    let mut buffer = Box::new([0; 256]);
    let mut surface = MutableImageSlice::new(16, 16, &mut *buffer);

    {
        let vertices = &[
            vector(0.0, 0.0),
            vector(8.0, 0.0),
            vector(0.0, 8.0),
            vector(15.0, 15.0),
            vector(1.0, 15.0),
        ];

        let indices = &[
            0, 1, 2, //0, 2, 3,
            1, 3, 4,
        ];

        // Fill the two triangles with the constant 1.
        rasterize_triangles(
            vertices,
            indices,
            &Constants { color: 1 },
            &mut ColorTarget {
                target: &mut surface,
                shader: FillConstantColor,
            },
        );
    }

    for i in 0..(surface.width * surface.height) {
        if i % surface.width == 0 {
            println!(" |");
            print!("          |");
        }
        let pix = surface.pixels[i];
        if pix == 0 {
            print!(" .");
        } else {
            print!(" {}", pix);
        }
    }
}

#[inline]
pub fn int_vector(x: i32, y: i32) -> IntVector {
    vec2(x, y)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BoolVec4 {
    pub x: bool,
    pub y: bool,
    pub z: bool,
    pub w: bool,
}

pub fn bvec4(x: bool, y: bool, z: bool, w: bool) -> BoolVec4 {
    BoolVec4 {
        x: x,
        y: y,
        z: z,
        w: w,
    }
}

impl BoolVec4 {
    #[inline]
    pub fn new(x: bool, y: bool, z: bool, w: bool) -> BoolVec4 {
        bvec4(x, y, z, w)
    }

    #[inline]
    pub fn any(self) -> bool {
        self.x || self.y || self.z || self.w
    }

    #[inline]
    pub fn all(self) -> bool {
        self.x && self.y && self.z && self.w
    }

    #[inline]
    pub fn and(self, other: BoolVec4) -> BoolVec4 {
        bvec4(
            self.x && other.x,
            self.y && other.y,
            self.z && other.z,
            self.w && other.w,
        )
    }

    #[inline]
    pub fn or(self, other: BoolVec4) -> BoolVec4 {
        bvec4(
            self.x || other.x,
            self.y || other.y,
            self.z || other.z,
            self.w || other.w,
        )
    }

    #[inline]
    pub fn tuple(&self) -> (bool, bool, bool, bool) {
        (self.x, self.y, self.z, self.w)
    }

    #[inline]
    pub fn array(&self) -> [bool; 4] {
        [self.x, self.y, self.z, self.w]
    }
}
