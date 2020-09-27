#version 450

#include "bindings.glsl"

layout(std140, binding = GLOBALS)
uniform Globals {
    vec2 u_resolution;
};

layout(set = INPUT_SAMPLERS_SET, binding = DEFAULT_SAMPLER)  uniform sampler u_default_sampler;

#include "rect.glsl"
#include "image_source.glsl"
#include "image.glsl"
#include "mask.glsl"

#ifdef VERTEX_SHADER

#include "transform.glsl"
#include "color.glsl"

layout(location = A_INSTANCE) in uvec4 a_instance;

struct MeshInstance {
    uint sub_mesh_offset;
    uint transform_id;
    uint user_data;
    float z;
};

MeshInstance unpack_instance() {
    MeshInstance instance;
    instance.sub_mesh_offset = a_instance[0];
    instance.transform_id = a_instance[1];
    instance.user_data = a_instance[2];
    instance.z = float(a_instance[3]) / 16384.0;

    return instance;
}

struct SubMesh {
    Rect img_dst_rect;
    uint src_color_id;
    uint transform_id;
    uint user_data;
    float opacity;
};

layout(std140, set = COMMON_SET, binding = PRIMITIVE_RECTS) buffer u_prim_rects { Rect prim_rects[]; };
layout(std140, set = SPECIFIC_SET, binding = SUB_MESHES) buffer u_sub_meshes { SubMesh sub_meshes[]; };

layout(location = A_POSITION) in vec2 a_position;
layout(location = A_SUB_MESH) in vec2 a_sub_mesh_id;

vec2 normalized_screen_position(vec2 position) {
    // Flip the Y axis because input y points down.
    return (position * vec2(2.0, -2.0)) / u_resolution + vec2(-1.0, 1.0);
}

void main() {
    MeshInstance instance = unpack_instance();
    SubMesh sub_mesh = sub_meshes[instance.sub_mesh_offset];

    vec2 position = a_position;
    position = apply_transform(position, sub_mesh.transform_id);
    position = apply_transform(position, instance.transform_id);

    vec2 sample_pos = (a_position - sub_mesh.img_dst_rect.xy) / rect_size(sub_mesh.img_dst_rect);
    write_image_data(sub_mesh.src_color_id, sample_pos);

    gl_Position = vec4(
        normalized_screen_position(position),
        instance.z,
        1.0
    );
}

#endif


#ifdef FRAGMENT_SHADER

layout(location = 0) out vec4 out_color;

void main() {
    out_color = get_image_color();
}


#endif
