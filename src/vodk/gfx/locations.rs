use gfx::renderer;

pub static a_position:   renderer::VertexAttributeLocation = 0;
pub static a_normal:    renderer::VertexAttributeLocation = 1;
pub static a_tex_coords: renderer::VertexAttributeLocation = 2;
pub static a_color:      renderer::VertexAttributeLocation = 3;
// for antialiased shape rendering
pub static a_extrusion: renderer::VertexAttributeLocation = 4;

#[deriving(Show)]
pub struct UniformLayout {
    pub u_resolution: renderer::ShaderInputLocation,
    pub u_color: renderer::ShaderInputLocation,
    pub u_texture_0: renderer::ShaderInputLocation,
    pub u_texture_1: renderer::ShaderInputLocation,
    pub u_texture_2: renderer::ShaderInputLocation,
    pub u_texture_3: renderer::ShaderInputLocation,
    pub u_model_mat: renderer::ShaderInputLocation,
    pub u_view_mat: renderer::ShaderInputLocation,
    pub u_proj_mat: renderer::ShaderInputLocation,
}

impl UniformLayout {
    pub fn new(ctx: &mut renderer::RenderingContext, p: renderer::ShaderProgram) -> UniformLayout{
        return UniformLayout {
            u_resolution: ctx.get_shader_input_location(p, "u_resolution"),
            u_texture_0: ctx.get_shader_input_location(p, "u_texture_0"),
            u_texture_1: ctx.get_shader_input_location(p, "u_texture_1"),
            u_texture_2: ctx.get_shader_input_location(p, "u_texture_2"),
            u_texture_3: ctx.get_shader_input_location(p, "u_texture_3"),
            u_model_mat: ctx.get_shader_input_location(p, "u_model_mat"),
            u_view_mat: ctx.get_shader_input_location(p, "u_view_mat"),
            u_proj_mat: ctx.get_shader_input_location(p, "u_proj_mat"),
            u_color: ctx.get_shader_input_location(p, "u_color"),
        }
    }
}
