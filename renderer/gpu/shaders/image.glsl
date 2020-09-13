#ifndef IMAGE_GLSL
#define IMAGE_GLSL

#ifdef VERTEX_SHADER

layout(location = V_IMAGE_UV) out vec2 v_image_uv;

#ifdef FEATURE_REPETITION
    layout(location = V_IMAGE_UV_BOUNDS) flat out Rect v_image_uv_bounds;
#endif

void write_image_data(uint img_src_id, vec2 vertex_pos) {
    ImageSource src = image_sources[img_src_id];
    
    vec2 uv = rect_sample(src.rect, vertex_pos);

    #ifdef FEATURE_REPETITION
        vec2 repeat = src.parameters.xy;
        vec2 offset = src.parameters.zw;
        uv = uv - rect_min(src.rect) * repeat - offset;

        v_image_uv_bounds = src.rect;
    #endif

    v_image_uv = uv;
}

#endif // VERTEX_SHADER


#ifdef FRAGMENT_SHADER

layout(set = INPUT_SAMPLERS_SET, binding = INPUT_COLOR_0) uniform texture2D u_input_color_0;

layout(location = V_IMAGE_UV) in vec2 v_image_uv;

vec4 get_image_color() {
    vec2 uv = v_image_uv;

    #ifdef FEATURE_REPETITION
        vec2 uv_size = v_image_uv_bounds.zw - v_image_uv_bounds.xy;
        uv = mod(uv - v_image_uv_bounds.xy, uv_size) + v_image_uv_bounds.xy;
    #endif

    // If we inflate primitives for vertex-aa we'll need to clamp the uvs
    // uv = rect_clamp(v_image_uv_bounds, -half_px);

    vec4 color = textureLod(sampler2D(u_input_color_0, u_default_sampler), uv, 0);

    return color;
}

#endif // FRAGMENT_SHADER


#endif
