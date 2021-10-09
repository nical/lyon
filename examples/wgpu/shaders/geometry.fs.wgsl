struct Output {
    [[location(0)]] out_color: vec4<f32>;
};

[[stage(fragment)]]
fn main([[location(0)]] v_color: vec4<f32>) -> Output {
    return Output(v_color);
}
