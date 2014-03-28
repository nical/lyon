
pub type TextureFlags = i32;
pub static TEXTURE_REPEAT_S          : TextureFlags = 1;
pub static TEXTURE_REPEAT_T          : TextureFlags = 2;
pub static TEXTURE_REPEAT            : TextureFlags = 3;
pub static TEXTURE_CLAMP_S           : TextureFlags = 4;
pub static TEXTURE_CLAMP_T           : TextureFlags = 8;
pub static TEXTURE_CLAMP             : TextureFlags = 12;
pub static TEXTURE_MIN_FILTER_LINEAR : TextureFlags = 16;
pub static TEXTURE_MAG_FILTER_LINEAR : TextureFlags = 32;
pub static TEXTURE_FILTER_LINEAR     : TextureFlags = 48;
pub static TEXTURE_MIN_FILTER_NEAREST: TextureFlags = 64;
pub static TEXTURE_MAG_FILTER_NEAREST: TextureFlags = 128;
pub static TEXTURE_SAMPLE_NEAREST    : TextureFlags = 192;
pub static TEXTURE_FLAGS_DEFAULT     : TextureFlags = TEXTURE_CLAMP|TEXTURE_FILTER_LINEAR;

#[deriving(Eq, Clone, Show)]
pub enum BackendType {
    GL_BACKEND,
    INVALID_NACKEND,
}

#[deriving(Eq, Clone, Show)]
pub enum ShaderType {
    FRAGMENT_SHADER,
    VERTEX_SHADER,
    GEOMETRY_SHADER,
}

#[deriving(Eq, Clone, Show)]
pub enum DrawMode {
    LINES,
    LINE_LOOP,
    LINE_STRIP,
    TRIANGLES,
    TRIANGLE_STRIP
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

type Handle = u32;

#[deriving(Eq, Clone, Show)]
pub struct Shader { handle: Handle }

#[deriving(Eq, Clone, Show)]
pub struct ShaderProgram { handle: Handle }

#[deriving(Eq, Clone, Show)]
pub struct Texture { handle: Handle }

/// Equivalent of a Buffer object in OpenGL
#[deriving(Eq, Clone, Show)]
pub struct VertexBuffer { handle: Handle }

/// Equivalent of a VAO in OpenGL
#[deriving(Eq, Clone, Show)]
pub struct Geometry { handle: Handle }

/// Equivalent of a FBO in OpenGL
#[deriving(Eq, Clone, Show)]
pub struct RenderTarget { handle: Handle }

pub struct Error {
    code: ErrorCode,
    detail: Option<~str>,
}

pub struct ErrorCode(u32);
pub type Status = Result<(), ErrorCode>;

#[deriving(Clone, Show)]
pub struct VertexAttribute {
    buffer: VertexBuffer,
    attrib_type: AttributeType,
    location: u16,
    stride: u16,
    offset: u16,
    components: u8,
    normalize: bool,
}

#[deriving(Eq, Clone, Show)]
pub enum ShaderInputValue {
    INPUT_FLOATS(~[f32]),
    INPUT_TEXTURE(Texture),
}

#[deriving(Eq, Clone, Show)]
pub struct ShaderInput {
    location: i32,
    value: ShaderInputValue,
}

#[deriving(Eq, Clone, Show)]
pub struct DrawCommand {
    mode: DrawMode,
    target: RenderTarget,
    shader_program: ShaderProgram,
    shader_inputs: ~[ShaderInput],
    geometry: Geometry,
    first: u32,
    count: u32,
    use_indices: bool,
}

pub enum RenderingCommand {
    OpDraw(DrawCommand),
    OpFlush,
    OpClear,
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

    fn create_texture(&mut self) -> Texture;
    fn destroy_texture(&mut self, tex: Texture);
    fn set_texture_flags(&mut self, tex: Texture, flags: TextureFlags);
    fn upload_texture_data(&mut self, dest: Texture,
                           data: &[u8], format: PixelFormat,
                           w:u32, h:u32, stride: u32) -> bool;
    /**
     * Specifies the texture's size and format
     * Does not need to be called if some data will be uploaded
     * through upload_texture_data.
     */
    fn allocate_texture(&mut self, dest: Texture,
                        format: PixelFormat,
                        w:u32, h:u32, stride: u32) -> bool;

    fn create_shader(&mut self, t: ShaderType) -> Shader;
    fn destroy_shader(&mut self, s: Shader);
    fn compile_shader(&mut self, shader: Shader, src: &str) -> Result<(), ~str>;

    fn create_shader_program(&mut self) -> ShaderProgram;
    fn destroy_shader_program(&mut self, s: ShaderProgram);
    fn link_shader_program(&mut self, p: ShaderProgram, shaders: &[Shader]) -> Result<(), ~str>;

    fn create_vertex_buffer(&mut self) -> VertexBuffer;
    fn destroy_vertex_buffer(&mut self, buffer: VertexBuffer);
    fn upload_vertex_data(&mut self, buffer: VertexBuffer,
                          data: &[f32], update: UpdateHint);
    fn allocate_vertex_buffer(&mut self, dest: VertexBuffer,
                              size: u32, update: UpdateHint) -> Status;

    fn create_geometry(&mut self) -> Geometry;
    fn destroy_geometry(&mut self, geom: Geometry);
    fn define_geometry(&mut self, geom: Geometry,
                       attributes: &[VertexAttribute],
                       indices: Option<VertexBuffer>) -> Status;

    fn get_shader_input_location(&mut self, program: ShaderProgram,
                                 name: &str) -> i32;
    fn get_vertex_attribute_location(&mut self, program: ShaderProgram,
                                     name: &str) -> i32;

    fn create_render_target(&mut self,
                            color_attachments: &[Texture],
                            depth: Option<Texture>,
                            stencil: Option<Texture>) -> Result<RenderTarget, ~str>;
    fn destroy_render_target(&mut self, fbo: RenderTarget);

    fn get_default_render_target(&mut self) -> RenderTarget;

    fn render(&mut self, commands: &[RenderingCommand]);
}

impl RenderTarget {
    fn default() -> RenderTarget { RenderTarget { handle: 0 } }
}
