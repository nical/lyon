#version 450

layout(location = 0) in vec2 a_position;
layout(location = 0) out vec2 v_position;

void main() {
    gl_Position = vec4(a_position, 0.0000001, 1.0);
    v_position = a_position;
}
