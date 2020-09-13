#ifndef TRANSFORM_GLSL
#define TRANSFORM_GLSL

struct Transform {
    vec4 data0;
    vec4 data1;
};

mat3 unpack_transform(Transform t) {
    return mat3(
        t.data0.x, t.data0.y, 0.0,
        t.data0.z, t.data0.w, 0.0,
        t.data1.x, t.data1.y, 1.0
    );
}

layout(std140, binding = TRANSFORMS) uniform u_transforms { Transform transforms[512]; };

#endif
