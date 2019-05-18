#version 450

layout(std140, binding = 0)
uniform Globals {
    vec2 u_resolution;
};

struct Primitive {
    uint z_index;
    uint color;
};

// Must match the constant in mesh2d.rs.
#define PRIM_BUFFER_LEN 1024

layout(std140, binding = 1)
uniform u_primitives { Primitive primitives[PRIM_BUFFER_LEN]; };

layout(location = 0) in vec2 a_position;
layout(location = 1) in uint a_prim_id;

layout(location = 2) in uint a_transform_id;
layout(location = 3) in uint a_prim_offset;
layout(location = 4) in uint a_layer_id;
layout(location = 5) in uint a_instance_z_index;

layout(location = 0) flat out vec4 v_color;

void main() {
    Primitive prim = primitives[a_prim_offset + a_prim_id];

    vec2 transformed_pos = a_position / u_resolution;

    float z = float(a_instance_z_index + prim.z_index) / 4096.0;
    gl_Position = vec4(transformed_pos, z, 1.0);

    uint mask = 0x000000FFu;
    v_color = vec4(
        float(prim.color & mask),
        float((prim.color >>  8) & mask),
        float((prim.color >> 16) & mask),
        float((prim.color >> 24) & mask)
    ) / 255.0;
}
