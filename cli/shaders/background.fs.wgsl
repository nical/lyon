
struct Globals {
    resolution: vec2<f32>,
    scroll_offset: vec2<f32>,
    bg_color: vec4<f32>,
    vignette_color: vec4<f32>,
    zoom: f32,
};

@group(0) @binding(0) var<uniform> globals: Globals;
struct Output {
    @location(0) color: vec4<f32>,
};

@fragment
fn main(@location(0) v_position: vec2<f32>) -> Output {

    var invert_y = vec2<f32>(1.0, -1.0);
    var px_position = v_position * invert_y * globals.resolution * 0.5;

    var vignette = clamp(0.7 * length(v_position), 0.0, 1.0);
    var color = mix(globals.bg_color, globals.vignette_color, vignette);

    // TODO: properly adapt the grid while zooming in and out.
    var grid_scale: f32 = 5.0;
    if (globals.zoom < 2.5) {
        grid_scale = 1.0;
    }

    var pos = px_position + globals.scroll_offset * globals.zoom;

    var small_cell = 20.0 / grid_scale * globals.zoom;
    if (abs(pos.x) % small_cell <= 1.0 || abs(pos.y) % small_cell <= 1.0) {
        color = color * 1.2;
    }

    var large_cell = 100.0 / grid_scale * globals.zoom;
    if (abs(pos.x) % large_cell <= 2.0 || abs(pos.y) % large_cell <= 2.0) {
        color = color * 1.2;
    }

    return Output(color);
}
