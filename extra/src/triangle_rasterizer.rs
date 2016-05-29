use vodk_math::{ Vec2 };
use image::MutableImageSlice;

#[cfg(test)]
use vodk_math::{ vec2 };

/// A software triangle rasterizer intended for ref testing and to help debugging
/// the output of the various tesselation routines.
///
/// The triangles are defined by sequences of 3 indices in the input buffers.
/// For example, the first triangle is:
/// { vertices[indices[0]], vertices[indices[1]], vertices[indices[2]] }
/// the second triangle is:
/// { vertices[indices[3]], vertices[indices[4]], vertices[indices[5]] }
/// etc.
///
/// For each triangle, the shading function is called on each covered pixel.
pub fn rasterize_triangles<Pixel, Constants, Vertex: VertexData, Shader: PixelShader<Pixel, Vertex, Constants>>(
    // geometry
    vertices: &[Vertex],
    indices: &[u16],
    // constant parameters passed to the shader for each pixel
    constants: &Constants,
    // render target
    output: &mut MutableImageSlice<Pixel>,
) {
    unimplemented!(); // TODO
}


/// An operation that is applied to each rasterized pixel
pub trait PixelShader<Pixel, Vertex, Constants> {
    fn shade(pixel: Pixel, vertex_data: &Vertex, constants: &Constants) -> Pixel;
}

// Vertices must implement this trait
pub trait VertexData {
    fn interpolate(v1: &Self, v2: &Self, v3: &Self, w1: f32, w2: f32, w3: f32) -> Self;
    fn position(&self) -> Vec2;
}

/// A simple shader that returns the interpolated vertex color.
struct FillVertexColor;

/// A simple shader that returns the constant color.
struct FillConstantColor;

/// Implemented vertices and constants that can return a color.
pub trait GetColor<Pixel> { fn get_color(&self) -> Pixel; }



impl<Pixel, Vertex: GetColor<Pixel>, Constants>
PixelShader<Pixel, Vertex, Constants>
for FillVertexColor {
    fn shade(_: Pixel, vertex_data: &Vertex, _: &Constants) -> Pixel {
        vertex_data.get_color()
    }
}

impl<Pixel, Vertex, Constants: GetColor<Pixel>>
PixelShader<Pixel, Vertex, Constants>
for FillConstantColor {
    fn shade(_: Pixel, _: &Vertex, constants: &Constants) -> Pixel {
        constants.get_color()
    }
}

impl VertexData for Vec2 {
    fn interpolate(a: &Vec2, b: &Vec2, c: &Vec2, wa: f32, wb: f32, wc: f32) -> Vec2 {
        let inv_w = 1.0 / (wa + wb + wc);
        return (*a * wa + *b * wb + *c * wc) * inv_w;
    }

    fn position(&self) -> Vec2 { *self }
}


#[test]
#[ignore]
fn test_rasterizer_simple() {
    // This test rasterizes two triangles which should produce a square of origin
    // (10, 10) and size (80, 80).

    struct Constants { color: u8, }
    impl GetColor<u8> for Constants { fn get_color(&self) -> u8 { self.color } }


    // Allocate the memory for the surface we are rendering to.
    let mut buffer: Vec<u8> = Vec::with_capacity(10000);
    for _ in 0..10000 {
        buffer.push(0);
    }

    let mut surface = MutableImageSlice::new(100, 100, &mut buffer[..]);


    // Describe the geometry

    let vertices = &[
        vec2(10.0, 10.0),
        vec2(90.0, 10.0),
        vec2(90.0, 90.0),
        vec2(10.0, 90.0),
    ];

    let indices = &[
        0, 1, 2, // first triangle
        0, 2, 3, // second triangle
    ];


    // Fill the two triangles with the constant 1.
    rasterize_triangles::<u8, Constants, Vec2, FillConstantColor>(
        vertices,
        indices,
        &Constants { color: 1 },
        &mut surface
    );

    let (w, h) = surface.get_size();
    for y in 0..h {
        for x in 0..w {
            let offset = surface.pixel_offset(x, y);
            let expected =  if x > 10 && x < 90 && y > 10 && y < 90 { 1 } else { 0 };
            let actual = surface.get_data()[offset];
            assert_eq!(actual, expected);
        }
    }
}
