
pub type TextureFlags = i32;
pub static TEXTURE_REPEAT_S          : TextureFlags = 1;
pub static TEXTURE_REPEAT_T          : TextureFlags = 2;
pub static TEXTURE_REPEAT            : TextureFlags = 3;
pub static TEXTURE_CLAMP_S           : TextureFlags = 4;
pub static TEXTURE_CLAMP_T           : TextureFlags = 8;
pub static TEXTURE_CLAMP             : TextureFlags = 12;
pub static TEXTURE_MIN_FILTER_LINEAR : TextureFlags = 16;
pub static TEXTURE_MAG_FILTER_LINEAR : TextureFlags = 32;
pub static TEXTURE_FILTER_LINEAR     : TextureFlags = TEXTURE_MIN_FILTER_LINEAR|TEXTURE_MAG_FILTER_LINEAR;
pub static TEXTURE_MIN_FILTER_NEAREST: TextureFlags = 64;
pub static TEXTURE_MAG_FILTER_NEAREST: TextureFlags = 128;
pub static TEXTURE_FILTER_NEAREST    : TextureFlags = TEXTURE_MIN_FILTER_NEAREST|TEXTURE_MAG_FILTER_NEAREST;
pub static TEXTURE_FLAGS_DEFAULT     : TextureFlags = TEXTURE_CLAMP|TEXTURE_FILTER_LINEAR;

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
pub enum BackendType {
    GL_BACKEND,
    INVALID_BACKEND,
}

#[deriving(Eq, Clone, Show)]
pub enum ShaderType {
    FRAGMENT_SHADER,
    VERTEX_SHADER,
    GEOMETRY_SHADER,
}

#[deriving(Eq, Clone, Show)]
pub enum Feature {
    FRAGMENT_SHADING,
    VERTEX_SHADING,
    GEOMETRY_SHADING,
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
    FORMAT_R8G8B8A8,
    FORMAT_R8G8B8X8,
    FORMAT_B8G8R8A8,
    FORMAT_B8G8R8X8,
    FORMAT_A8,
}

#[deriving(Eq, Clone, Show)]
pub enum UpdateHint {
    STATIC_UPDATE,
    STREAM_UPDATE,
    DYNAMIC_UPDATE
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

pub type ShaderInputLocation = i32;
#[deriving(Eq, Clone, Show)]
pub type ErrorCode = u32;

#[deriving(Clone, Show)]
pub struct VertexAttribute {
    pub buffer: VertexBuffer,
    pub attrib_type: AttributeType,
    pub location: i16,
    pub stride: u16,
    pub offset: u16,
    pub components: u8,
    pub normalize: bool,
}

type TextureUnit = u32;

#[deriving(Eq, Clone, Show)]
pub enum ShaderInputValue<'l> {
    INPUT_FLOATS(&'l [f32]),
    INPUT_MAT3(&'l [f32]),
    INPUT_MAT4(&'l [f32]),
    INPUT_TEXTURE(Texture, TextureUnit),
}

#[deriving(Eq, Clone, Show)]
pub struct ShaderInput<'l> {
    pub location: i32,
    pub value: ShaderInputValue<'l>,
}

pub enum ShaderConstant<'l> {
    FloatsInput(&'l [f32]),
    MatrixInput(&'l [f32], bool),
    TextureInput(Texture),
    IntInput(Texture),
}

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
                         x:u32, y:u32, w: u32, h: u32,
                         format: PixelFormat,
                         dest: &[u8]) -> RendererResult;

    fn create_shader(&mut self, t: ShaderType) -> Shader;
    fn destroy_shader(&mut self, s: Shader);
    fn compile_shader(&mut self, shader: Shader, src: &str) -> RendererResult;

    fn create_shader_program(&mut self) -> ShaderProgram;
    fn destroy_shader_program(&mut self, s: ShaderProgram);
    fn link_shader_program(&mut self, p: ShaderProgram, shaders: &[Shader],
                           attrib_locations: Option<&[(&str, u32)]>)  -> RendererResult;

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
                                     name: &str) -> ShaderInputLocation;

    fn create_render_target(&mut self,
                            color_attachments: &[Texture],
                            depth: Option<Texture>,
                            stencil: Option<Texture>) -> Result<RenderTarget, Error>;
    fn destroy_render_target(&mut self, fbo: RenderTarget);

    fn set_render_target(&mut self, target: RenderTarget);

    fn get_default_render_target(&mut self) -> RenderTarget;

    fn set_shader(&mut self, program: ShaderProgram) -> RendererResult;

    fn set_shader_input_float(&mut self, location: i32, input: &[f32]);
    fn set_shader_input_int(&mut self, location: i32, input: &[i32]);
    fn set_shader_input_matrix(&mut self, location: i32, input: &[f32], dimension: u32, transpose: bool);
    fn set_shader_input_texture(&mut self, location: i32, texture_unit: u32, input: Texture);

    fn draw(&mut self, geom: GeometryRange, flags: RenderFlags) -> RendererResult;
}
