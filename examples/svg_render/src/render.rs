use gfx;

use lyon::tessellation::geometry_builder::VertexConstructor;
use lyon::tessellation;
use usvg::Color;

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

gfx_defines!{
    vertex GpuFillVertex {
        position: [f32; 2] = "a_position",
        prim_id: u32 = "a_prim_id",
    }

    // a 2x3 matrix (last two members of data1 unused).
    constant Transform {
        data0: [f32; 4] = "data0",
        data1: [f32; 4] = "data1",
    }

    constant Primitive {
        transform: u32 = "transform",
        color: u32 = "color",
    }

    constant Globals {
        zoom: [f32; 2] = "u_zoom",
        pan: [f32; 2] = "u_pan",
        aspect_ratio: f32 = "u_aspect_ratio",
    }

    pipeline fill_pipeline {
        vbo: gfx::VertexBuffer<GpuFillVertex> = (),
        out_color: gfx::RenderTarget<ColorFormat> = "out_color",
        constants: gfx::ConstantBuffer<Globals> = "Globals",
        primitives: gfx::ConstantBuffer<Primitive> = "u_primitives",
        transforms: gfx::ConstantBuffer<Transform> = "u_transforms",
    }
}

// This struct carries the data for each vertex
pub struct VertexCtor {
    pub prim_id: u32,
}

// Handle conversions to the gfx vertex format
impl VertexConstructor<tessellation::FillVertex, GpuFillVertex> for VertexCtor {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> GpuFillVertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());

        GpuFillVertex {
            position: vertex.position.to_array(),
            prim_id: self.prim_id,
        }
    }
}

impl VertexConstructor<tessellation::StrokeVertex, GpuFillVertex> for VertexCtor {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex) -> GpuFillVertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());

        GpuFillVertex {
            position: vertex.position.to_array(),
            prim_id: self.prim_id,
        }
    }
}

// Default scene has all values set to zero
#[derive(Copy, Clone, Debug, Default)]
pub struct Scene {
    pub zoom: f32,
    pub pan: [f32; 2],
    pub aspect_ratio: f32,
    pub wireframe: bool,
}

impl Scene {
    pub fn new(zoom: f32, pan: [f32; 2], aspect_ratio: f32) -> Self {
        Self {
            zoom,
            pan,
            aspect_ratio,
            wireframe: false,
        }
    }
}

// Extract the relevant globals from the scene struct
impl From<Scene> for Globals {
    fn from(scene: Scene) -> Self {
        Globals {
            zoom: [scene.zoom, scene.zoom],
            pan: scene.pan,
            aspect_ratio: scene.aspect_ratio
        }
    }
}

impl Primitive {
    pub fn new(transform_idx: u32, color: Color, alpha: f32) -> Self {
        Primitive {
            transform: transform_idx,
            color: ((color.red as u32) << 24)
                + ((color.green as u32) << 16)
                + ((color.blue as u32) << 8)
                + (alpha * 255.0) as u32,
        }
    }
}

pub static MAX_PRIMITIVES: usize = 512;
pub static MAX_TRANSFORMS: usize = 512;

pub static VERTEX_SHADER: &'static str = "
    #version 150
    #line 118

    uniform Globals {
        vec2 u_zoom;
        vec2 u_pan;
        float u_aspect_ratio;
    };

    struct Primitive {
        uint transform;
        uint color;
    };

    struct Transform {
        vec4 data0;
        vec4 data1;
    };

    uniform u_primitives { Primitive primitives[512]; };
    uniform u_transforms { Transform transforms[512]; };

    in vec2 a_position;
    in uint a_prim_id;

    out vec4 v_color;

    void main() {
        Primitive prim = primitives[a_prim_id];

        Transform t = transforms[prim.transform];
        mat3 transform = mat3(
            t.data0.x, t.data0.y, 0.0,
            t.data0.z, t.data0.w, 0.0,
            t.data1.x, t.data1.y, 1.0
        );

        vec2 pos = (transform * vec3(a_position, 1.0)).xy;
        gl_Position = vec4((pos.xy + u_pan) * u_zoom, 0.0, 1.0);
        gl_Position.y *= -1.0;
        gl_Position.x /= u_aspect_ratio;

        uint mask = 0x000000FFu;
        uint color = prim.color;
        v_color = vec4(
            float((color >> 24) & mask),
            float((color >> 16) & mask),
            float((color >>  8) & mask),
            float(color & mask)
        ) / 255.0;
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
