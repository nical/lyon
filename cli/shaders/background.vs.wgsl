struct VertexOutput {
    @location(0) v_position: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn main(@location(0) a_position: vec2<f32>) -> VertexOutput {
    var position = vec4<f32>(a_position, 0.0000001, 1.0);
    return VertexOutput(a_position, position);
}
