#version 450

layout(std140, binding = 0)
uniform Globals {
    vec2 u_resolution;
};

layout(location = 0) in vec2 a_min;
layout(location = 1) in vec2 a_max;
layout(location = 2) in uint a_z_index;
layout(location = 3) in uint a_color;

layout(location = 0) out vec4 v_color;

const vec2 quad[4] = vec2[](
  vec2(0.0, 0.0),
  vec2(1.0, 0.0),
  vec2(1.0, 1.0),
  vec2(0.0, 1.0)
);

void main() {
    vec2 local_pos = a_min + quad[gl_VertexIndex] * (a_max - a_min);
    local_pos = local_pos / u_resolution;
    float z = float(a_z_index) / 4096.0;

    gl_Position = vec4(local_pos, z, 1.0);

    uint mask = 0x000000FFu;
    v_color = vec4(
        float(a_color & mask),
        float((a_color >>  8) & mask),
        float((a_color >> 16) & mask),
        float((a_color >> 24) & mask)
    ) / 255.0;
}
