#ifndef TRANSFORM_GLSL
#define TRANSFORM_GLSL

#define NO_TRANSFORM 0

struct Transform {
    vec4 data0;
    vec4 data1;
};

layout(std140, set = COMMON_SET, binding = TRANSFORMS) buffer u_transforms { Transform transforms[]; };

mat3 unpack_transform(Transform t) {
    return mat3(
        t.data0.x, t.data0.y, 0.0,
        t.data0.z, t.data0.w, 0.0,
        t.data1.x, t.data1.y, 1.0
    );
}

vec2 apply_transform(vec2 position, uint transform_id) {
    if (transform_id == NO_TRANSFORM) {
        return position;
    }

    mat3 transform = unpack_transform(transforms[transform_id]);
    return (transform * vec3(position, 1.0)).xy;
}

#endif
