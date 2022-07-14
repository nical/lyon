struct Output {
    @location(0) color: vec4<f32>,
};

@fragment
fn main(
    @location(0) v_position: vec2<f32>,
    @location(1) @interpolate(flat) v_resolution: vec2<f32>,
    @location(2) @interpolate(flat) v_scroll_offset: vec2<f32>,
    @location(3) @interpolate(flat) v_zoom: f32
) -> Output {

    var invert_y = vec2<f32>(1.0, -1.0);
    var px_position = v_position * invert_y * v_resolution * 0.5;

    var vignette = clamp(0.7 * length(v_position), 0.0, 1.0);
    var color = mix(
        vec4<f32>(0.0, 0.47, 0.9, 1.0),
        vec4<f32>(0.0, 0.1, 0.64, 1.0),
        vignette
    );

    var grid_scale: f32 = 5.0;
    if (v_zoom < 2.5) {
        grid_scale = 1.0;
    }

    var pos = px_position + v_scroll_offset * v_zoom;

    var small_cell = 20.0 / grid_scale * v_zoom;
    if (abs(pos.x) % small_cell <= 1.0 || abs(pos.y) % small_cell <= 1.0) {
        color = color * 1.2;
    }

    var large_cell = 100.0 / grid_scale * v_zoom;
    if (abs(pos.x) % large_cell <= 2.0 || abs(pos.y) % large_cell <= 2.0) {
        color = color * 1.2;
    }

    return Output(color);
}
