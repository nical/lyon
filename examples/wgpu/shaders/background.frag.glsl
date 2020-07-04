#version 450

layout(location = 0) in vec2 v_position;
layout(location = 1) flat in vec2 v_resolution;
layout(location = 2) flat in vec2 v_scroll_offset;
layout(location = 3) flat in float v_zoom;

layout(location = 0) out vec4 out_color;


void main() {
    vec2 invert_y = vec2(1.0, -1.0);
    vec2 px_position = v_position * v_resolution * 0.5 * invert_y;

    // #005fa4
    float vignette = clamp(0.7 * length(v_position), 0.0, 1.0);
    out_color = mix(
        vec4(0.0, 0.47, 0.9, 1.0),
        vec4(0.0, 0.1, 0.64, 1.0),
        vignette
    );

    // TODO: properly adapt the grid while zooming in and out.
    float grid_scale = 5.0;
    if (v_zoom < 2.5) {
        grid_scale = 1.0;
    }

    vec2 pos = px_position + v_scroll_offset * v_zoom;

    if (mod(pos.x, 20.0 / grid_scale * v_zoom) <= 1.0 ||
        mod(pos.y, 20.0 / grid_scale * v_zoom) <= 1.0) {
        out_color *= 1.2;
    }

    if (mod(pos.x, 100.0 / grid_scale * v_zoom) <= 2.0 ||
        mod(pos.y, 100.0 / grid_scale * v_zoom) <= 2.0) {
        out_color *= 1.2;
    }
}
