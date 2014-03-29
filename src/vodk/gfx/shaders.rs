
pub static SOLID_COLOR_FRAGMENT_SHADER : &'static str = &"
uniform vec4 u_color;
void main() {
    gl_FragColor = u_color;
}
";

pub static TEX_COORDS_FRAGMENT_SHADER : &'static str = &"
varying vec2 v_tex_coords;
void main() {
    gl_FragColor = vec4(v_tex_coords, 0.0, 1.0);
}
";

pub static BASIC_VERTEX_SHADER : &'static str = &"
attribute vec2 a_position;
varying vec2 v_tex_coords;
void main() {
  gl_Position = vec4(a_position, 0.0, 1.0);
  v_tex_coords = (vec2(1.0, 1.0) + a_position) / 2.0;
}
";
