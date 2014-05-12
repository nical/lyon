
pub static TEXT_VERTEX_SHADER: &'static str = &"
attribute vec2 a_position;
attribute vec2 a_tex_coords;
varying vec2 v_tex_coords;
void main() {
  gl_Position = vec4(a_position, 0.0, 1.0);
  v_tex_coords = a_tex_coords;
}
";

pub static TEXT_FRAGMENT_SHADER : &'static str = &"
uniform vec4 u_color;
uniform sampler2D u_texture_0;
varying vec2 v_tex_coords;
void main() {
    gl_FragColor = u_color * texture2D(u_texture_0, v_tex_coords).r;
    //gl_FragColor = vec4(v_tex_coords, 0.0, 1.0);
    //gl_FragColor = u_color;
}
";

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
    //gl_FragColor = vec4(0.0, v_tex_coords, 1.0);
}
";

pub static BASIC_VERTEX_SHADER : &'static str = &"
attribute vec2 a_position;
varying vec2 v_tex_coords;
void main() {
  gl_Position = vec4(a_position, 0.0, 1.0);
  v_tex_coords = vec2(a_position.x, 1.0 - a_position.y);
}
";
