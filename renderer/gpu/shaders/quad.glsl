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

#include "instance.glsl"
#include "transform.glsl"
#include "color.glsl"

layout(std140, binding = PRIMITIVE_RECTS) buffer u_prim_rects { Rect prim_rects[]; };

layout(location = A_POSITION) in vec2 a_position;

vec2 normalized_screen_position(vec2 position) {
    // Flip the Y axis because input y points down.
    return (position * vec2(2.0, -2.0)) / u_resolution + vec2(-1.0, 1.0);
}

void main() {
    Instance instance = unpack_instance();

    Rect local_rect = prim_rects[instance.rect_id];
    vec2 local_position = rect_sample(local_rect, a_position);

    vec2 transformed_position = apply_transform(local_position, instance.transform_id);

    write_image_data(instance.src_color_id, a_position);

    write_clip_mask_data(instance.src_mask_id, a_position);

    gl_Position = vec4(
        normalized_screen_position(transformed_position),
        instance.z,
        1.0
    );
}

#endif


#ifdef FRAGMENT_SHADER

layout(location = 0) out vec4 out_color;

void main() {
    out_color = get_image_color();

    float mask = get_clip_mask_alpha();
    out_color.a *= mask;
}


#endif
