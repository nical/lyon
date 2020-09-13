#ifndef CLIP_GLSL
#define CLIP_GLSL

#ifdef VERTEX_SHADER

layout(location = V_MASK_UV) out vec2 v_mask_uv;

void write_clip_mask_data(uint clip_src_id, vec2 vertex_pos) {
    if (clip_src_id == NO_IMAGE_SOURCE) {
        v_mask_uv = vec2(0.0, 0.0);
    } else {
        ImageSource src = image_sources[clip_src_id];
        v_mask_uv = rect_sample(src.rect, vertex_pos);
    }    
}

#endif // VERTEX_SHADER


#ifdef FRAGMENT_SHADER

layout(set = INPUT_SAMPLERS_SET, binding = U8_MASK) uniform texture2D u_mask;

layout(location = V_MASK_UV) in vec2 v_mask_uv;

float get_clip_mask_alpha() {
    float mask = textureLod(sampler2D(u_mask, u_default_sampler), v_mask_uv, 0).r;

    return mask;
}

#endif // FRAGMENT_SHADER


#endif
