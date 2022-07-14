struct Globals {
    resolution: vec2<f32>,
    scroll_offset: vec2<f32>,
    zoom: f32,
};

@group(0) @binding(0) var<uniform> globals: Globals;

struct VertexOutput {
    @location(0) v_position: vec2<f32>,
    @location(1) @interpolate(flat) v_resolution: vec2<f32>,
    @location(2) @interpolate(flat) v_scroll_offset: vec2<f32>,
    @location(3) @interpolate(flat) v_zoom: f32,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn main(@location(0) a_position: vec2<f32>) -> VertexOutput {
    var position = vec4<f32>(a_position, 0.0000001, 1.0);
    return VertexOutput(
        a_position,
        globals.resolution,
        globals.scroll_offset,
        globals.zoom,
        position
    );
}
