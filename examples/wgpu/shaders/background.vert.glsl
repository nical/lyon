#version 450

layout(std140, binding = 0) uniform Globals {
    vec2 u_resolution;
    vec2 u_scroll_offset;
    float u_zoom;
};

layout(location = 0) in vec2 a_position;
layout(location = 0) out vec2 v_position;

layout(location = 1) flat out vec2 v_resolution;
layout(location = 2) flat out vec2 v_scroll_offset;
layout(location = 3) flat out float v_zoom;

void main() {
    gl_Position = vec4(a_position, 0.0000001, 1.0);
    v_position = a_position;
    v_resolution = u_resolution;
    v_scroll_offset = u_scroll_offset;
    v_zoom = u_zoom;
}
