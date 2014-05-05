
pub static SOLID_COLOR_FRAGMENT_SHADER : &'static str = &"
uniform vec4 u_color;
void main() {
    gl_FragColor = u_color;
}
";

pub static TEXTURED_FRAGMENT_SHADER : &'static str = &"
//precision lowp float;
uniform vec4 u_color;
uniform sampler2D u_texture_0;
varying vec2 v_tex_coords;
void main() {
    gl_FragColor = texture2D(u_texture_0, v_tex_coords);
}
";

pub static BASIC_VERTEX_SHADER : &'static str = &"
attribute vec2 a_position;
varying vec2 v_tex_coords;
void main() {
  gl_Position = vec4(a_position, 0.0, 1.0);
  v_tex_coords = a_position;
}
";
