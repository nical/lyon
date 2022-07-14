struct Globals {
    u_zoom: vec2<f32>,
    u_pan: vec2<f32>,
    u_aspect_ratio: f32,
};

struct Primitive {
    transform: u32,
    color: u32,
    pad: vec2<u32>,
};

struct Transform {
    data0: vec4<f32>,
    data1: vec4<f32>,
};

struct Primitives {
    primitives: array<Primitive>,
};

struct Transforms {
    transforms: array<Transform>,
};

@group(0) @binding(0) var<uniform> global: Globals;
@group(0) @binding(1) var<storage, read> u_primitives: Primitives;
@group(0) @binding(2) var<storage, read> u_transforms: Transforms;

struct VertexOutput {
    @location(0) v_color: vec4<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn main(
    @location(0) a_position: vec2<f32>,
    @location(1) a_prim_id: u32
) -> VertexOutput {
    var prim: Primitive = u_primitives.primitives[a_prim_id];

    var t: Transform = u_transforms.transforms[prim.transform];
    var transform = mat3x3<f32>(
        vec3<f32>(t.data0.x, t.data0.y, 0.0),
        vec3<f32>(t.data0.z, t.data0.w, 0.0),
        vec3<f32>(t.data1.x, t.data1.y, 1.0)
    );
    
    var invert_y = vec2<f32>(1.0, -1.0);

    var pos: vec2<f32> = (transform * vec3<f32>(a_position, 1.0)).xy;
    var position: vec4<f32> = vec4<f32>((((pos.xy + global.u_pan) * global.u_zoom) * invert_y), 0.0, 1.0);
    position.x = (position.x / global.u_aspect_ratio);

    var mask: u32 = 255u;
    var color = vec4<f32>(
        f32(((prim.color >> u32(24)) & mask)),
        f32(((prim.color >> u32(16)) & mask)),
        f32(((prim.color >> u32(8)) & mask)),
        f32((prim.color & mask))
    ) / vec4<f32>(255.0);

    return VertexOutput(color, position);
}
