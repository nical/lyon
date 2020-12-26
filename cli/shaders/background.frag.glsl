#version 450

layout(location = 0) in vec2 v_position;

layout(std140, binding = 0) uniform Globals {
    vec2 u_resolution;
    vec2 u_scroll_offset;
    vec4 u_bg_color;
    vec4 u_vignette_color;
    float u_zoom;
};

layout(location = 0) out vec4 out_color;


void main() {
    vec2 invert_y = vec2(1.0, -1.0);
    vec2 px_position = v_position * u_resolution * 0.5 * invert_y;

    // #005fa4
    float vignette = clamp(0.7 * length(v_position), 0.0, 1.0);
    out_color = mix(
        u_bg_color,
        u_vignette_color,
        vignette
    );

    // TODO: properly adapt the grid while zooming in and out.
    float grid_scale = 5.0;
    if (u_zoom < 2.5) {
        grid_scale = 1.0;
    }

    vec2 pos = px_position + u_scroll_offset * u_zoom;

    if (mod(pos.x, 20.0 / grid_scale * u_zoom) <= 1.0 ||
        mod(pos.y, 20.0 / grid_scale * u_zoom) <= 1.0) {
        out_color *= 1.2;
    }

    if (mod(pos.x, 100.0 / grid_scale * u_zoom) <= 2.0 ||
        mod(pos.y, 100.0 / grid_scale * u_zoom) <= 2.0) {
        out_color *= 1.2;
    }
}
