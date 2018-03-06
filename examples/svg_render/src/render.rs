use cgmath;
use gfx;

use lyon::tessellation::geometry_builder::VertexConstructor;
use lyon::tessellation;
use resvg::tree::Color;

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

gfx_defines!{
    vertex GpuFillVertex {
        position: [f32; 2] = "a_position",
        color: [f32; 4] = "a_color",
    }

    constant Globals {
        zoom: [f32; 2] = "u_zoom",
        pan: [f32; 2] = "u_pan",
        proj: [[f32;4];4] = "u_proj",
    }

    pipeline fill_pipeline {
        vbo: gfx::VertexBuffer<GpuFillVertex> = (),
        out_color: gfx::RenderTarget<ColorFormat> = "out_color",
        constants: gfx::ConstantBuffer<Globals> = "Globals",
    }
}

fn gpu_color(color: Color, opacity: f32) -> [f32; 4] {
    [
        f32::from(color.red) / 255.0,
        f32::from(color.green) / 255.0,
        f32::from(color.blue) / 255.0,
        opacity,
    ]
}

// This struct carries the data for each vertex
pub struct VertexCtor {
    fill: Color,
    opacity: f32,
}

impl VertexCtor {
    pub fn new(c: Color, o: f64) -> Self {
        Self {
            fill: c,
            opacity: o as f32,
        }
    }
}

// handle conversions to the gfx vertex format
impl VertexConstructor<tessellation::FillVertex, GpuFillVertex> for VertexCtor {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> GpuFillVertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());

        GpuFillVertex {
            position: vertex.position.to_array(),
            color: gpu_color(self.fill, self.opacity),
        }
    }
}

impl VertexConstructor<tessellation::StrokeVertex, GpuFillVertex> for VertexCtor {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex) -> GpuFillVertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());

        GpuFillVertex {
            position: vertex.position.to_array(),
            color: gpu_color(self.fill, self.opacity),
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
// default scene has all values set to zero
pub struct Scene {
    pub zoom: f32,
    pub pan: [f32; 2],
    pub proj: [[f32; 4]; 4],
}

impl Scene {
    pub fn new(zoom: f32, pan: [f32; 2], proj: cgmath::Matrix4<f32>) -> Self {
        let proj = Self::convert_mat4(proj);
        Self {
            zoom: zoom,
            pan: pan,
            proj: proj,
        }
    }
    pub fn update_proj(&mut self, proj: cgmath::Matrix4<f32>) {
        self.proj = Self::convert_mat4(proj);
    }
    fn convert_mat4(m: cgmath::Matrix4<f32>) -> [[f32; 4]; 4] {
        [
            [m.x.x, m.x.y, m.x.z, m.x.w],
            [m.y.x, m.y.y, m.y.z, m.y.w],
            [m.z.x, m.z.y, m.z.z, m.z.w],
            [m.w.x, m.w.y, m.w.z, m.w.w],
        ]
    }
}

// extract the relevant globals from the scene struct
impl From<Scene> for Globals {
    fn from(scene: Scene) -> Self {
        Globals {
            zoom: [scene.zoom, scene.zoom],
            pan: scene.pan,
            proj: scene.proj,
        }
    }
}

pub static VERTEX_SHADER: &'static str = "
    #version 150
    #line 266

    uniform Globals {
        vec2 u_zoom;
        vec2 u_pan;
        mat4 u_proj;
    };

    in vec2 a_position;
    in vec4 a_color;

    out vec4 v_color;

    void main() {
        gl_Position = u_proj * vec4((a_position + u_pan) * u_zoom, 0.0, 1.0);
        gl_Position.y *= -1.0;
        v_color = a_color;
    }
";

// The fragment shader is dead simple. It just applies the color computed in the vertex shader.
// A more advanced renderer would probably compute texture coordinates in the vertex shader and
// sample the color from a texture here.
pub static FRAGMENT_SHADER: &'static str = "
    #version 150
    in vec4 v_color;
    out vec4 out_color;

    void main() {
        out_color = v_color;
    }
";
