#version 450

layout(std140, binding = 0)
uniform Globals {
    vec2 u_zoom;
    vec2 u_pan;
    float u_aspect_ratio;
};

struct Primitive {
    uint transform;
    uint color;
    uvec2 _pad;
};

struct Transform {
    vec4 data0;
    vec4 data1;
};

layout(std140, binding = 1) uniform u_primitives { Primitive primitives[512]; };
layout(std140, binding = 2) uniform u_transforms { Transform transforms[512]; };

layout(location = 0) in vec2 a_position;
layout(location = 1) in uint a_prim_id;

layout(location = 0) out vec4 v_color;

void main() {
    Primitive prim = primitives[a_prim_id];

    Transform t = transforms[prim.transform];
    mat3 transform = mat3(
        t.data0.x, t.data0.y, 0.0,
        t.data0.z, t.data0.w, 0.0,
        t.data1.x, t.data1.y, 1.0
    );

    vec2 invert_y = vec2(1.0, -1.0);

    vec2 pos = (transform * vec3(a_position, 1.0)).xy;
    gl_Position = vec4((pos.xy + u_pan) * u_zoom * invert_y, 0.0, 1.0);
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

