struct Globals {
    resolution: vec2<f32>,
    scroll_offset: vec2<f32>,
    bg_color1: vec4<f32>,
    bg_color2: vec4<f32>,
    zoom: f32,
};

struct Primitive {
    color: vec4<f32>,
    translate: vec2<f32>,
    z_index: i32,
    width: f32,
};

struct Primitives {
    primitives: array<Primitive, 64>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<uniform> u_primitives: Primitives;

struct VertexOutput {
    @location(0) v_color: vec4<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn main(
    @location(0) a_position: vec2<f32>,
    @location(1) a_normal: vec2<f32>,
    @location(2) a_prim_id: u32,
    @builtin(instance_index) instance_idx: u32
) -> VertexOutput {
    var prim: Primitive = u_primitives.primitives[a_prim_id + instance_idx];

    var invert_y = vec2<f32>(1.0, -1.0);

    var local_pos = a_position + a_normal * prim.width;
    var world_pos = local_pos - globals.scroll_offset + prim.translate + 5.0 * vec2<f32>(f32(instance_idx), 0.0);
    var transformed_pos = world_pos * globals.zoom / (vec2<f32>(0.5, 0.5) * globals.resolution) * invert_y;

    var z = f32(prim.z_index) / 4096.0;
    var position = vec4<f32>(transformed_pos, z, 1.0);

    return VertexOutput(prim.color, position);
}
