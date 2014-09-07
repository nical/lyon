
use data;

pub type BufferData<'l> = data::DynamicallyTypedSlice<'l>;

pub type TextureFlags = i32;
pub static REPEAT_S          : TextureFlags = 1 << 0;
pub static REPEAT_T          : TextureFlags = 1 << 1;
pub static REPEAT            : TextureFlags = 1 << (REPEAT_S | REPEAT_T) as uint;
pub static CLAMP_S           : TextureFlags = 1 << 2;
pub static CLAMP_T           : TextureFlags = 1 << 3;
pub static CLAMP             : TextureFlags = 1 << (CLAMP_S | CLAMP_T) as uint;
pub static MIN_FILTER_LINEAR : TextureFlags = 1 << 4;
pub static MAG_FILTER_LINEAR : TextureFlags = 1 << 5;
pub static FILTER_LINEAR     : TextureFlags = MIN_FILTER_LINEAR | MAG_FILTER_LINEAR;
pub static MIN_FILTER_NEAREST: TextureFlags = 1 << 6;
pub static MAG_FILTER_NEAREST: TextureFlags = 1 << 7;
pub static FILTER_NEAREST    : TextureFlags = MIN_FILTER_NEAREST | MAG_FILTER_NEAREST;
pub static FLAGS_DEFAULT     : TextureFlags = CLAMP | FILTER_LINEAR;

// TODO this mixes flags that are about the geometry and flags that are about
// pipeline features. Should proably sperate it.
pub type GeometryFlags = u32;
pub static TRIANGLES             : GeometryFlags = 1 << 3;
pub static LINES                 : GeometryFlags = 1 << 4;
pub static STRIP                 : GeometryFlags = 1 << 5;
pub static LOOP                  : GeometryFlags = 1 << 5;
pub static TRIANGLE_STRIP        : GeometryFlags = TRIANGLES | STRIP;
pub static LINE_STRIP            : GeometryFlags = LINES | STRIP;
pub static LINE_LOOP             : GeometryFlags = LINES | LOOP;

pub type TargetTypes = u32;
pub static COLOR  : TargetTypes = 1 << 0;
pub static DEPTH  : TargetTypes = 1 << 1;
pub static STENCIL: TargetTypes = 1 << 2;

#[deriving(PartialEq, Clone, Show)]
pub enum ShaderType {
    FRAGMENT_SHADER,
    VERTEX_SHADER,
    GEOMETRY_SHADER,
    COMPUTE_SHADER,
}

#[deriving(PartialEq, Clone, Show)]
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

pub type AttributeType = data::Type;

#[deriving(PartialEq, Clone, Show)]
pub enum PixelFormat {
    R8G8B8A8,
    R8G8B8X8,
    B8G8R8A8,
    B8G8R8X8,
    A8,
    A_F32,
}

#[deriving(PartialEq, Clone, Show)]
pub enum UpdateHint {
    STATIC,
    STREAM,
    DYNAMIC
}

#[deriving(PartialEq, Clone, Show)]
pub enum BufferType {
    VERTEX_BUFFER,
    INDEX_BUFFER,
    UNIFORM_BUFFER,
    DRAW_INDIRECT_BUFFER,
    TRANSFORM_FEEDBACK_BUFFER,
}

#[deriving(PartialEq, Clone, Show)]
pub enum BlendMode {
    NO_BLENDING,
    ALPHA_BLENDING,
    ADD_BLENDING,
    SUB_BLENDING,
    MUL_BLENDING,
}

#[deriving(PartialEq, Clone, Show)]
pub struct VertexRange {
    pub first: u32,
    pub count: u32,
}

#[deriving(PartialEq, Clone, Show)]
pub struct IndexRange {
    pub first: u32,
    pub count: u32,
}

pub type Handle = u32;
pub static INVALID_HANDLE: Handle = 0;

#[deriving(PartialEq, Clone, Show)]
pub struct ShaderStage { pub handle: Handle }

#[deriving(PartialEq, Clone, Show)]
pub struct Shader { pub handle: Handle }

#[deriving(PartialEq, Clone, Show)]
pub struct Texture { pub handle: Handle }

/// Equivalent of a Buffer object in OpenGL
#[deriving(PartialEq, Clone, Show)]
pub struct Buffer {
    pub handle: Handle,
    pub buffer_type: BufferType
}

/// Equivalent of a VAO in OpenGL
#[deriving(PartialEq, Clone, Show)]
pub struct Geometry {
    pub handle: Handle,
    // To work around some drivers not storing the index buffer
    // binding in the VAO state
    pub ibo: Handle
}

/// Equivalent of a FBO in OpenGL
#[deriving(PartialEq, Clone, Show)]
pub struct RenderTarget { pub handle: Handle }

impl Texture { pub fn invalid_handle() -> Texture { Texture { handle: INVALID_HANDLE } } }
impl Shader { pub fn invalid_handle() -> Shader { Shader { handle: INVALID_HANDLE } } }
impl Buffer { pub fn invalid_handle() -> Buffer { Buffer { handle: INVALID_HANDLE, buffer_type: VERTEX_BUFFER } } }
impl Geometry { pub fn invalid_handle() -> Geometry { Geometry { handle: INVALID_HANDLE, ibo: INVALID_HANDLE } } }
impl ShaderStage { pub fn invalid_handle() -> ShaderStage { ShaderStage { handle: INVALID_HANDLE } } }

pub enum Range {
    VertexRange(u16, u16),
    IndexRange(u16, u16),
}

#[deriving(PartialEq, Clone, Show)]
pub struct Error {
    pub code: String,
    pub detail: Option<String>,
}

pub type RendererResult = Result<(), Error>;

pub type ShaderInputLocation = i16;
pub type VertexAttributeLocation = i16;

#[deriving(PartialEq, Clone, Show)]
pub type ErrorCode = u32;

#[deriving(Clone, Show)]
pub struct VertexAttribute {
    pub buffer: Buffer, // TODO: move out of this struct
    pub attrib_type: AttributeType,
    pub location: VertexAttributeLocation,
    pub stride: u16,
    pub offset: u16,
    pub normalize: bool,
}

pub trait RenderingContext {
    fn is_supported(&mut self, f: Feature) -> bool;
    fn flush(&mut self);
    fn set_viewport(&mut self, x:i32, y:i32, w:i32, h:i32);
    fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32);
    fn clear(&mut self, targets: TargetTypes);

    fn reset_state(&mut self);

    //fn get_error_str(&mut self, err: ErrorCode) -> &'static str;

    fn create_texture(&mut self, flags: TextureFlags) -> Texture;
    fn destroy_texture(&mut self, tex: Texture);
    fn set_texture_flags(&mut self, tex: Texture, flags: TextureFlags);
    fn upload_texture_data(&mut self,
        dest: Texture,
        data: &BufferData,
        w:u32, h:u32,
        format: PixelFormat
    ) -> RendererResult;
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

    fn create_shader_stage(&mut self, t: ShaderType) -> ShaderStage;
    fn destroy_shader_stage(&mut self, s: ShaderStage);
    fn compile_shader_stage(&mut self, shader: ShaderStage, src: &[&str]) -> RendererResult;

    fn create_shader(&mut self) -> Shader;
    fn destroy_shader(&mut self, s: Shader);
    fn link_shader(&mut self, p: Shader, stages: &[ShaderStage],
                   attrib_locations: &[(&str, VertexAttributeLocation)])  -> RendererResult;

    fn create_buffer(&mut self, buffer_type: BufferType) -> Buffer;
    fn destroy_buffer(&mut self, buffer: Buffer);
    fn upload_buffer(&mut self,
        buffer: Buffer,
        buf_type: BufferType,
        update: UpdateHint,
        data: &BufferData
    ) -> RendererResult;
    fn allocate_buffer(&mut self, dest: Buffer, buf_type: BufferType,
                       update: UpdateHint, size: u32) -> RendererResult;

    fn destroy_geometry(&mut self, geom: Geometry);
    fn create_geometry(&mut self,
                       attributes: &[VertexAttribute],
                       indices: Option<Buffer>) -> Result<Geometry, Error>;

    fn get_shader_input_location(&mut self, program: Shader,
                                 name: &str) -> ShaderInputLocation;
    fn get_vertex_attribute_location(&mut self, program: Shader,
                                     name: &str) -> VertexAttributeLocation;

    fn create_render_target(&mut self,
                            color_attachments: &[Texture],
                            depth: Option<Texture>,
                            stencil: Option<Texture>) -> Result<RenderTarget, Error>;
    fn destroy_render_target(&mut self, fbo: RenderTarget);

    fn set_render_target(&mut self, target: RenderTarget);

    fn get_default_render_target(&mut self) -> RenderTarget;

    fn set_shader(&mut self, program: Shader) -> RendererResult;

    fn set_shader_input_float(&mut self, location: ShaderInputLocation, input: &[f32]);
    fn set_shader_input_int(&mut self, location: ShaderInputLocation, input: &[i32]);
    fn set_shader_input_matrix(&mut self, location: ShaderInputLocation, input: &[f32], dimension: u32, transpose: bool);
    fn set_shader_input_texture(&mut self, location: ShaderInputLocation, texture_unit: u32, input: Texture);

    fn draw(&mut self,
        geom: Geometry,
        range: Range,
        flags: GeometryFlags,
        blend: BlendMode,
        targets: TargetTypes
    ) -> RendererResult;

    fn multi_draw(&mut self,
        geom: Geometry,
        indirect_buffer: Buffer,
        flags: GeometryFlags,
        targets: TargetTypes,
        commands: &[DrawCommand]
    ) -> RendererResult;
}

pub struct DrawCommand {
    primitive_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: u32,
    base_instance: u32,
}
