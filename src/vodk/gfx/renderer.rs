
pub type TextureFlags = i32;
pub static REPEAT_S          : TextureFlags = 1;
pub static REPEAT_T          : TextureFlags = 2;
pub static REPEAT            : TextureFlags = 3;
pub static CLAMP_S           : TextureFlags = 4;
pub static CLAMP_T           : TextureFlags = 8;
pub static CLAMP             : TextureFlags = 12;
pub static MIN_FILTER_LINEAR : TextureFlags = 16;
pub static MAG_FILTER_LINEAR : TextureFlags = 32;
pub static FILTER_LINEAR     : TextureFlags = MIN_FILTER_LINEAR|MAG_FILTER_LINEAR;
pub static MIN_FILTER_NEAREST: TextureFlags = 64;
pub static MAG_FILTER_NEAREST: TextureFlags = 128;
pub static FILTER_NEAREST    : TextureFlags = MIN_FILTER_NEAREST|MAG_FILTER_NEAREST;
pub static FLAGS_DEFAULT     : TextureFlags = CLAMP|FILTER_LINEAR;

pub type RenderFlags = u32;
pub static ENABLE_Z_TEST         : RenderFlags = 1 >> 0;
pub static ENABLE_STENCIL_TEST   : RenderFlags = 1 >> 1;
pub static LINES                 : RenderFlags = 1 >> 2;
pub static STRIP                 : RenderFlags = 1 >> 3;
pub static LOOP                  : RenderFlags = 1 >> 4;
pub static INDEXED               : RenderFlags = 1 >> 5;
pub static TRIANGLE_STRIP        : RenderFlags = STRIP;
pub static LINE_STRIP            : RenderFlags = LINES | STRIP;
pub static LINE_LOOP             : RenderFlags = LINES | LOOP;
pub static RENDER_DEFAULT        : RenderFlags = 0;

#[deriving(Eq, Clone, Show)]
pub enum ShaderType {
    FRAGMENT_SHADER,
    VERTEX_SHADER,
    GEOMETRY_SHADER,
    COMPUTE_SHADER,
}

#[deriving(Eq, Clone, Show)]
pub enum Feature {
    FRAGMENT_SHADING,
    VERTEX_SHADING,
    GEOMETRY_SHADING,
    COMPUTE,
    DEPTH_TEXTURE,
    RENDER_TO_TEXTURE,
    MULTIPLE_RENDER_TARGETS,
    INSTANCED_RENDERING,
}

#[deriving(Eq, Clone, Show)]
pub enum AttributeType {
    F32,
    F64,
    I32,
    U32,
}

#[deriving(Eq, Clone, Show)]
pub enum PixelFormat {
    R8G8B8A8,
    R8G8B8X8,
    B8G8R8A8,
    B8G8R8X8,
    A8,
}

#[deriving(Eq, Clone, Show)]
pub enum UpdateHint {
    STATIC,
    STREAM,
    DYNAMIC
}

pub type Handle = u32;

#[deriving(Eq, Clone, Show)]
pub struct Shader { pub handle: Handle }

#[deriving(Eq, Clone, Show)]
pub struct ShaderProgram { pub handle: Handle }

#[deriving(Eq, Clone, Show)]
pub struct Texture { pub handle: Handle }

/// Equivalent of a Buffer object in OpenGL
#[deriving(Eq, Clone, Show)]
pub struct VertexBuffer { pub handle: Handle }

/// Equivalent of a VAO in OpenGL
#[deriving(Eq, Clone, Show)]
pub struct Geometry { pub handle: Handle }

pub struct GeometryRange {
    pub geometry: Geometry,
    pub from: u32,
    pub to: u32,
    pub indexed: bool,
}

/// Equivalent of a FBO in OpenGL
#[deriving(Eq, Clone, Show)]
pub struct RenderTarget { pub handle: Handle }

#[deriving(Eq, Clone, Show)]
pub struct Error {
    pub code: ErrorCode,
    pub detail: Option<~str>,
}

pub type RendererResult = Result<(), Error>;

pub type ShaderInputLocation = i16;
pub type VertexAttributeLocation = i16;

#[deriving(Eq, Clone, Show)]
pub type ErrorCode = u32;

#[deriving(Clone, Show)]
pub struct VertexAttribute {
    pub buffer: VertexBuffer,
    pub attrib_type: AttributeType,
    pub location: VertexAttributeLocation,
    pub stride: u16,
    pub offset: u16,
    pub components: u8,
    pub normalize: bool,
}

type TextureUnit = u32;

pub trait RenderingContext {
    fn is_supported(&mut self, f: Feature) -> bool;
    fn flush(&mut self);
    fn set_viewport(&mut self, x:i32, y:i32, w:i32, h:i32);
    fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32);
    fn clear(&mut self);

    fn reset_state(&mut self);

    fn make_current(&mut self) -> bool;
    fn check_error(&mut self) -> Option<~str>;
    fn get_error_str(&mut self, err: ErrorCode) -> &'static str;

    fn create_texture(&mut self, flags: TextureFlags) -> Texture;
    fn destroy_texture(&mut self, tex: Texture);
    fn set_texture_flags(&mut self, tex: Texture, flags: TextureFlags);
    fn upload_texture_data(&mut self, dest: Texture, data: &[u8],
                           w:u32, h:u32, format: PixelFormat) -> RendererResult;
    /**
     * Specifies the texture's size and format
     * Does not need to be called if some data will be uploaded
     * through upload_texture_data.
     */
    fn allocate_texture(&mut self, dest: Texture,
                        w:u32, h:u32, format: PixelFormat) -> RendererResult;

    fn read_back_texture(&mut self, tex: Texture,
                         format: PixelFormat,
                         dest: &mut [u8]) -> RendererResult;

    fn create_shader(&mut self, t: ShaderType) -> Shader;
    fn destroy_shader(&mut self, s: Shader);
    fn compile_shader(&mut self, shader: Shader, src: &str) -> RendererResult;

    fn create_shader_program(&mut self) -> ShaderProgram;
    fn destroy_shader_program(&mut self, s: ShaderProgram);
    fn link_shader_program(&mut self, p: ShaderProgram, shaders: &[Shader],
                           attrib_locations: Option<&[(&str, VertexAttributeLocation)]>)  -> RendererResult;

    fn create_vertex_buffer(&mut self) -> VertexBuffer;
    fn destroy_vertex_buffer(&mut self, buffer: VertexBuffer);
    fn upload_vertex_data(&mut self, buffer: VertexBuffer,
                          data: &[f32], update: UpdateHint) -> RendererResult;
    fn allocate_vertex_buffer(&mut self, dest: VertexBuffer,
                              size: u32, update: UpdateHint) -> RendererResult;

    fn create_geometry(&mut self) -> Geometry;
    fn destroy_geometry(&mut self, geom: Geometry);
    fn define_geometry(&mut self, geom: Geometry,
                       attributes: &[VertexAttribute],
                       indices: Option<VertexBuffer>) -> RendererResult;

    fn get_shader_input_location(&mut self, program: ShaderProgram,
                                 name: &str) -> ShaderInputLocation;
    fn get_vertex_attribute_location(&mut self, program: ShaderProgram,
                                     name: &str) -> VertexAttributeLocation;

    fn create_render_target(&mut self,
                            color_attachments: &[Texture],
                            depth: Option<Texture>,
                            stencil: Option<Texture>) -> Result<RenderTarget, Error>;
    fn destroy_render_target(&mut self, fbo: RenderTarget);

    fn set_render_target(&mut self, target: RenderTarget);

    fn get_default_render_target(&mut self) -> RenderTarget;

    fn set_shader(&mut self, program: ShaderProgram) -> RendererResult;

    fn set_shader_input_float(&mut self, location: ShaderInputLocation, input: &[f32]);
    fn set_shader_input_int(&mut self, location: ShaderInputLocation, input: &[i32]);
    fn set_shader_input_matrix(&mut self, location: ShaderInputLocation, input: &[f32], dimension: u32, transpose: bool);
    fn set_shader_input_texture(&mut self, location: ShaderInputLocation, texture_unit: u32, input: Texture);

    fn draw(&mut self, geom: GeometryRange, flags: RenderFlags) -> RendererResult;
}
