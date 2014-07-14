pub static TEXT_FRAGMENT_SHADER : &'static str = "
uniform vec4 u_color;
uniform sampler2D u_texture_0;
varying vec2 v_tex_coords;
void main() {
    gl_FragColor = u_color;
    gl_FragColor.a = texture2D(u_texture_0, v_tex_coords).r * 2.0;
}
";

pub static SOLID_COLOR_FRAGMENT_SHADER : &'static str = "
uniform vec4 u_color;
void main() {
    gl_FragColor = u_color;
}
";

pub static COLOR_FRAGMENT_SHADER : &'static str = "
varying vec4 v_color;
void main() {
    gl_FragColor = v_color;
}
";

pub static TEXTURED_FRAGMENT_SHADER : &'static str = "
uniform vec4 u_color;
uniform sampler2D u_texture_0;
varying vec2 v_tex_coords;
void main() {
    gl_FragColor = texture2D(u_texture_0, v_tex_coords);
}
";

pub static BASIC_VERTEX_SHADER_2D : &'static str = "
attribute vec2 a_position;
attribute vec2 a_tex_coords;
uniform vec2 u_resolution;
varying vec2 v_tex_coords;
void main() {
  vec2 pos = vec2(a_position.x - u_resolution.x, u_resolution.y - a_position.y) / u_resolution;
  gl_Position = vec4(pos, 0.0, 1.0);
  v_tex_coords = a_tex_coords;
}
";

pub static SHAPE_VERTEX_SHADER_2D: &'static str = "
attribute vec2 a_position;
attribute vec2 a_normal;
attribute vec4 a_color;
attribute float a_extrude_world_space;
attribute float a_extrude_screen_space;

uniform mat3 u_model_mat;
uniform mat3 u_view_mat;
uniform vec2 u_resolution;

varying vec4 v_color;

void main() {
  mat3 transform = u_view_mat * u_model_mat;
  float scale = length(transform * vec3(1.0,0.0,0.0));
  vec2 pos = (a_position + a_normal * a_extrude_world_space
                         + a_normal * a_extrude_screen_space / scale) / u_resolution;
  gl_Position = vec4(transform * vec3(pos, 0.0), 1.0);
  v_color = a_color;
}
";

pub static BASIC_VERTEX_SHADER_3D : &'static str = "
attribute vec3 a_position;
attribute vec3 a_normal;
attribute vec2 a_tex_coords;
uniform mat4 u_model_mat;
uniform mat4 u_view_mat;
uniform mat4 u_proj_mat;
varying vec3 v_normal;
varying vec2 v_tex_coords;
void main() {
    v_tex_coords = a_tex_coords;
    v_normal = a_normal;
    gl_Position = u_proj_mat * u_view_mat * u_model_mat * vec4(a_position, 1.0);
}
";

pub static NORMALS_FRAGMENT_SHADER : &'static str = "
varying vec3 v_normal;
varying vec2 v_tex_coords;
void main() {
    vec3 normals = v_normal * 0.5 + vec3(0.5, 0.5, 0.5);
    gl_FragColor = vec4(normals, 1.0);
}
";
