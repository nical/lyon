
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

pub struct Error {
    pub code: ErrorCode,
    pub detail: Option<~str>,
}

pub struct ErrorCode(pub u32);
pub type Status = Result<(), ErrorCode>;

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

#[deriving(Eq, Clone, Show)]
pub struct DrawCommand<'l> {
    pub mode: DrawMode,
    pub target: RenderTarget,
    pub shader_program: ShaderProgram,
    pub shader_inputs: ~[ShaderInput<'l>],
    pub geometry: Geometry,
    pub first: u32,
    pub count: u32,
    pub use_indices: bool,
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
                           w:u32, h:u32) -> bool;
    /**
     * Specifies the texture's size and format
     * Does not need to be called if some data will be uploaded
     * through upload_texture_data.
     */
    fn allocate_texture(&mut self, dest: Texture,
                        format: PixelFormat,
                        w:u32, h:u32) -> bool;

    fn create_shader(&mut self, t: ShaderType) -> Shader;
    fn destroy_shader(&mut self, s: Shader);
    fn compile_shader(&mut self, shader: Shader, src: &str) -> Result<(), ~str>;

    fn create_shader_program(&mut self) -> ShaderProgram;
    fn destroy_shader_program(&mut self, s: ShaderProgram);
    fn link_shader_program(&mut self, p: ShaderProgram, shaders: &[Shader],
                           attrib_locations: Option<&[(&str, u32)]>) -> Result<(), ~str>;

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

    fn create_render_target(&mut self,
                            color_attachments: &[Texture],
                            depth: Option<Texture>,
                            stencil: Option<Texture>) -> Result<RenderTarget, ~str>;
    fn destroy_render_target(&mut self, fbo: RenderTarget);

    fn set_render_target(&mut self, target: RenderTarget);

    fn get_default_render_target(&mut self) -> RenderTarget;

    fn get_shader_input_location(&mut self, program: ShaderProgram,
                                 name: &str) -> i32;

    fn get_vertex_attribute_location(&mut self, program: ShaderProgram,
                                     name: &str) -> i32;

    fn set_shader(&mut self, program: ShaderProgram);

    fn set_shader_input_float(&mut self, location: i32, input: &[f32]);
    fn set_shader_input_int(&mut self, location: i32, input: &[i32]);
    fn set_shader_input_matrix(&mut self, location: i32, input: &[f32], dimension: u32, transpose: bool);
    fn set_shader_input_texture(&mut self, location: i32, texture_unit: u32, input: Texture);

    fn draw(&mut self, mode: DrawMode, geom: GeometryRange);
}
